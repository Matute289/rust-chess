---
name: learning-mode
description: Working on the Learning Mode system — AI profiles, tactical/strategic memory, playstyle profiling, learning events, experience progression, AI versioning. Use when working on backend/src/services/learning.rs or the learning layer.
---

# Learning Mode — rust-chess

The learning layer sits BETWEEN the game analyzer and the AI search engine. It never modifies `chess-engine` directly — it influences move selection through a `LearningEngine` trait.

## Architecture

```
GameAnalyzer (chess-engine)
    │ produces GameReport
    ▼
LearningService (backend)
    │ extracts LearningEvents, updates AIProfile
    ▼
AIProfile (PostgreSQL)
    │ loaded at game start
    ▼
LearningEngine trait
    │ implemented by MemoryLearningEngine (Phase 2)
    │                 NNUELearningEngine  (Phase 4)
    ▼
Search (chess-engine)
    │ receives position + memory context → avoids known errors
```

## LearningEngine Trait (Phase 4 prep)

Define this in `chess-engine/src/learning.rs`:

```rust
pub trait LearningEngine: Send + Sync {
    /// Given a position, return a bias score adjustment (-100..+100 cp)
    /// that nudges the search away from known bad moves.
    fn position_bias(&self, pos: &Position, m: Move) -> i32;

    /// Return the preferred opening move for this position, if any.
    fn opening_preference(&self, pos: &Position) -> Option<Move>;
}

pub struct NoLearning;
impl LearningEngine for NoLearning {
    fn position_bias(&self, _: &Position, _: &Move) -> i32 { 0 }
    fn opening_preference(&self, _: &Position) -> Option<Move> { None }
}
```

`Search` takes `&dyn LearningEngine`. Default is `NoLearning` — backward compatible.

## Database Schema

```sql
CREATE TABLE ai_profiles (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    level        SMALLINT NOT NULL DEFAULT 1,    -- 1-5, controls search depth
    experience   INT NOT NULL DEFAULT 0,
    games_played INT NOT NULL DEFAULT 0,
    games_won    INT NOT NULL DEFAULT 0,
    games_lost   INT NOT NULL DEFAULT 0,
    version      INT NOT NULL DEFAULT 1,         -- for AI versioning
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE learning_events (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id           UUID NOT NULL REFERENCES ai_profiles(id) ON DELETE CASCADE,
    event_type           TEXT NOT NULL,      -- 'tactical_error' | 'strategic_pattern' | 'opening_pref'
    position_fen         TEXT NOT NULL,
    bad_move             TEXT,               -- UCI notation (null for strategic events)
    better_move          TEXT,               -- UCI notation
    evaluation_diff      INT,                -- centipawns
    classification       TEXT,               -- MoveClass: 'Blunder' | 'Mistake' etc.
    weight               FLOAT NOT NULL DEFAULT 1.0,  -- decreases with time/recency
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_learning_events_profile ON learning_events(profile_id);
CREATE INDEX idx_learning_events_fen ON learning_events(position_fen);

CREATE TABLE ai_profile_snapshots (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    profile_id   UUID NOT NULL REFERENCES ai_profiles(id) ON DELETE CASCADE,
    version      INT NOT NULL,
    snapshot     JSONB NOT NULL,    -- serialized AIProfile state at this version
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

## Learning Rules

**On defeat:**
1. Run deep analysis (depth 18) on the game
2. For each `MoveClass::Blunder` or `MoveClass::Mistake`: create `learning_event` (tactical_error)
3. Increment `experience` by `blunders * 10 + mistakes * 5`
4. If `experience >= threshold(level)`: increment `level`, snapshot current profile

**On victory:**
1. Record game statistics only (openings used, structures played, result)
2. No experience gain, no learning events
3. Update `games_won`, `games_played` counters

**On draw:**
1. Record statistics
2. No experience gain

```rust
pub const EXPERIENCE_THRESHOLDS: [i32; 5] = [100, 300, 600, 1000, 2000];
// Level 1→2: 100xp, Level 2→3: 300xp total, etc.
```

## Position Similarity

Exact FEN matching is too strict — the same tactical motif appears in different positions. Use **partial position hashing**:

```rust
pub fn tactical_similarity(stored_fen: &str, current_pos: &Position) -> f32 {
    let stored = Position::from_fen(stored_fen).unwrap();

    // Compare piece configurations relative to the error square
    // Pieces within 2 squares of the error are weighted heavily
    let piece_match = compare_local_structure(&stored, current_pos);
    let material_match = (stored.material_balance() - current_pos.material_balance()).abs() < 100;
    let phase_match = similar_game_phase(&stored, current_pos);

    let score = piece_match * 0.7 + material_match as u8 as f32 * 0.2 + phase_match as u8 as f32 * 0.1;
    score
}
```

In `MemoryLearningEngine::position_bias()`:
```rust
fn position_bias(&self, pos: &Position, m: Move) -> i32 {
    let current_fen = pos.to_fen();
    for event in &self.tactical_errors {
        let similarity = tactical_similarity(&event.position_fen, pos);
        if similarity > 0.8 && Move::from_uci(&event.bad_move) == Some(m) {
            // Penalize this move — the AI learned it's bad here
            return -(event.evaluation_diff as f32 * similarity * event.weight) as i32;
        }
    }
    0
}
```

## Tactical vs Strategic Memory

**Tactical memory** (`event_type = 'tactical_error'`):
- Specific bad moves in specific positions
- High precision, clear right/wrong
- Sources: Blunders and Mistakes from GameReport

**Strategic memory** (`event_type = 'strategic_pattern'`):
- Patterns that led to lost games: openings played, pawn structures entered, endgame types
- Less precise — winning/losing isn't always the AI's fault
- Sources: game result + opening classification + structural analysis

**Opening preference** (`event_type = 'opening_pref'`):
- Openings where the AI has won games → slight preference
- Openings where the AI consistently loses → slight avoidance
- Only affects first 8-10 moves (when opening book ends)

## Playstyle Profiling

Track user behavior over games:

```rust
pub struct PlaystyleProfile {
    pub favorite_openings: Vec<(String, u32)>,  // (ECO code, frequency)
    pub sacrifice_frequency: f32,                // 0-1
    pub aggression_score: f32,                   // 0-1 (early attacks, king hunt)
    pub tactical_tendency: f32,                  // 0-1 vs positional
    pub castles_frequency: f32,                  // how often they castle
    pub simplification_tendency: f32,            // trades off pieces in winning positions
}
```

The AI uses this to **specialize its defense** against the user's style:
- Aggressive user → AI reinforces king safety, doesn't open lines
- Tactical user → AI keeps position closed, avoids sharp tactics
- Positional user → AI contests space and outpost squares

## AI Versioning

On every level-up:
```rust
async fn level_up(profile: &mut AiProfile, db: &Pool) {
    // 1. Snapshot current state
    let snapshot = serde_json::to_value(&profile)?;
    sqlx::query!("INSERT INTO ai_profile_snapshots (profile_id, version, snapshot) VALUES ($1, $2, $3)",
        profile.id, profile.version, snapshot).execute(db).await?;

    // 2. Increment version and level
    profile.version += 1;
    profile.level = (profile.level + 1).min(5);
    profile.experience = 0;  // reset for next level
    sqlx::query!("UPDATE ai_profiles SET level=$1, version=$2, experience=0 WHERE id=$3",
        profile.level, profile.version, profile.id).execute(db).await?;
}
```

Users can restore a previous version:
```
GET /api/ai/versions             → list snapshots
POST /api/ai/restore/{version}   → restore from snapshot
```

## Depth by Level

| Level | Search Depth | Memory Access |
|---|---|---|
| 1 | 2 | None |
| 2 | 3 | Last 10 tactical errors |
| 3 | 4 | Last 25 tactical errors + strategic patterns |
| 4 | 5 | Last 50 events + playstyle adaptation |
| 5 | 6+ | Full memory + playstyle + opening preferences |

At higher levels, the AI loads more `LearningEvent` records from DB into the `MemoryLearningEngine` at game start (kept in memory for the duration of the game).

## Testing Learning Behavior

```rust
#[tokio::test]
async fn test_avoids_known_blunder() {
    // Set up a profile with a known blunder in memory
    let mut engine = MemoryLearningEngine::new();
    engine.add_tactical_error(BLUNDER_FEN, "e2e4", 350);  // AI blundered e2e4 here before

    let pos = Position::from_fen(BLUNDER_FEN).unwrap();
    let bias = engine.position_bias(&pos, Move::from_uci("e2e4").unwrap());

    assert!(bias < -100, "AI should strongly penalize known blunder move");
}
```
