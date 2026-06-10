# Game Analyzer (Phase 1.5) — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Record every chess game played and, after it ends, run a depth-6 engine analysis producing per-move classifications (Brilliant/Excellent/Good/Inaccuracy/Mistake/Blunder) and accuracy percentages, displayed in the game-over overlay.

**Architecture:** Two layers. `chess-engine/src/analysis.rs` is pure Rust (WASM-safe): it takes a `GameRecord` (initial FEN + move list) and returns a `GameReport`. The Bevy layer (`src/board.rs`, `src/ai.rs`, new `src/analysis.rs`) records moves during play, triggers analysis synchronously when the game ends, and shows results in the existing game-over overlay.

**Tech Stack:** Rust 2021, `chess-engine` (internal), Bevy 0.14, `wasm32-unknown-unknown` compatible (no `std::time`, analysis runs synchronously in WASM).

---

## Coordinate reminder

- Bevy `Piece.x` = rank (0=rank1 … 7=rank8), `Piece.y` = file (0=a … 7=h)
- `chess_engine::Square(n)`: `rank() = n/8`, `file() = n%8`
- Conversion: `chess_engine::Square(bevy_x * 8 + bevy_y)`

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `chess-engine/src/analysis.rs` | Create | `GameResult`, `MoveClass`, `MoveAnalysis`, `GameSummary`, `GameReport`, `GameRecord`, `analyze_game()` |
| `chess-engine/src/lib.rs` | Modify | Export analysis types |
| `src/board.rs` | Modify | Add `GameHistory` resource; record each human move as a `chess_engine::Move` |
| `src/ai.rs` | Modify | Record AI moves in `GameHistory`; add `GameHistory` param to `ai_apply_move` |
| `src/analysis.rs` | Create | `AnalysisReport` resource; `AnalysisPlugin`; `run_post_game_analysis` system |
| `src/ui.rs` | Modify | Read `AnalysisReport` in `spawn_game_over_overlay`; add summary block |
| `src/lib.rs` | Modify | Register `AnalysisPlugin`; order analysis before ui event handling |

---

## Task 1: chess-engine/src/analysis.rs — types

**Files:**
- Create: `chess-engine/src/analysis.rs`
- Modify: `chess-engine/src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `chess-engine/src/analysis.rs` with only tests (no implementation yet):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_class_thresholds() {
        assert_eq!(classify(0),   MoveClass::Excellent);
        assert_eq!(classify(15),  MoveClass::Good);
        assert_eq!(classify(50),  MoveClass::Inaccuracy);
        assert_eq!(classify(150), MoveClass::Mistake);
        assert_eq!(classify(400), MoveClass::Blunder);
    }

    #[test]
    fn accuracy_perfect_game() {
        // All Excellent moves → 100% accuracy
        let analyses = vec![
            MoveAnalysis { eval_loss: 0, classification: MoveClass::Excellent,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_before: 0, score_after: 0, best_score: 0 },
            MoveAnalysis { eval_loss: 0, classification: MoveClass::Excellent,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_before: 0, score_after: 0, best_score: 0 },
        ];
        let acc = compute_accuracy(&analyses);
        assert!((acc - 100.0).abs() < 0.01, "expected 100.0 got {}", acc);
    }

    #[test]
    fn accuracy_all_blunders() {
        let analyses = vec![
            MoveAnalysis { eval_loss: 500, classification: MoveClass::Blunder,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_before: 0, score_after: -500, best_score: 0 },
        ];
        let acc = compute_accuracy(&analyses);
        assert!(acc < 50.0, "all-blunder accuracy should be low, got {}", acc);
    }
}
```

- [ ] **Step 2: Run to verify fail**

```bash
cargo test --package chess-engine -- analysis::tests 2>&1 | head -5
```

Expected: compile error (types not defined yet).

- [ ] **Step 3: Implement types and classification**

Replace the entire file with:

```rust
use crate::moves::Move;
use crate::position::Position;
use crate::search::{DifficultyConfig, Search, SearchResult, MATE_SCORE};
use crate::movegen::MoveGen;
use crate::types::{Color, Square};

// ── Analysis constants ────────────────────────────────────────────────────────

pub const ANALYSIS_DEPTH: u8 = 6;
pub const ANALYSIS_NODES: u64 = 200_000;

// ── Public types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GameResult {
    WhiteWins,
    BlackWins,
    Draw,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MoveClass {
    Brilliant,
    Excellent,
    Good,
    Inaccuracy,
    Mistake,
    Blunder,
}

#[derive(Debug, Clone)]
pub struct MoveAnalysis {
    pub fen_before:     String,
    pub played_move:    String,   // UCI string e.g. "e2e4"
    pub best_move:      String,   // UCI string
    pub score_before:   i32,      // centipawns from mover's POV before the move
    pub score_after:    i32,      // centipawns from mover's POV after the move (negated from opponent)
    pub best_score:     i32,      // engine's best score for this position
    pub eval_loss:      i32,      // best_score - score_after (≥ 0)
    pub classification: MoveClass,
}

#[derive(Debug, Clone)]
pub struct GameSummary {
    pub accuracy_white:       f32,
    pub accuracy_black:       f32,
    pub blunders:             [u8; 2],   // [white, black]
    pub mistakes:             [u8; 2],
    pub inaccuracies:         [u8; 2],
    pub critical_move_indices: Vec<usize>, // indices into move_analyses where eval_loss > 100cp
}

#[derive(Debug, Clone)]
pub struct GameReport {
    pub move_analyses: Vec<MoveAnalysis>,
    pub summary:       GameSummary,
}

pub struct GameRecord {
    pub initial_fen: String,
    pub moves:       Vec<Move>,
    pub result:      GameResult,
}

// ── Classification ────────────────────────────────────────────────────────────

pub fn classify(eval_loss: i32) -> MoveClass {
    match eval_loss {
        0          => MoveClass::Excellent,
        1..=19     => MoveClass::Good,
        20..=99    => MoveClass::Inaccuracy,
        100..=299  => MoveClass::Mistake,
        _          => MoveClass::Blunder,
    }
}

// ── Accuracy formula ─────────────────────────────────────────────────────────
// Inspired by chess.com / Lichess formula: sigmoid-like curve where each blunder
// costs more than inaccuracies. Simple version: start at 100, subtract per category.

pub fn compute_accuracy(analyses: &[MoveAnalysis]) -> f32 {
    if analyses.is_empty() { return 100.0; }

    let total: f32 = analyses.iter().map(|a| {
        match a.classification {
            MoveClass::Brilliant | MoveClass::Excellent => 0.0,
            MoveClass::Good       => 2.0,
            MoveClass::Inaccuracy => 8.0,
            MoveClass::Mistake    => 20.0,
            MoveClass::Blunder    => 40.0,
        }
    }).sum();

    let max_penalty = analyses.len() as f32 * 40.0;
    let raw = 1.0 - total / max_penalty;
    (raw * 100.0).max(0.0).min(100.0)
}

// ── Main analysis ─────────────────────────────────────────────────────────────

/// Analyze a full game. Returns one MoveAnalysis per move in record.moves.
pub fn analyze_game(record: &GameRecord) -> GameReport {
    let config = DifficultyConfig {
        max_depth: ANALYSIS_DEPTH,
        max_nodes: ANALYSIS_NODES,
        random_factor: 0.0,
    };

    let mut pos = match Position::from_fen(&record.initial_fen) {
        Ok(p) => p,
        Err(_) => return empty_report(),
    };

    let mut all_analyses: Vec<MoveAnalysis> = Vec::new();

    for played_move in &record.moves {
        let fen_before = pos.to_fen();
        let mover = pos.side_to_move;

        // Step 1: search from this position — gives us best_score and best_move
        let result = Search::new().best_move(&pos, &config);
        let (best_move_m, best_score) = match result {
            SearchResult::EngineMove(m, s) => (m, s),
        };

        // Step 2: apply played move (SavedState discarded — analysis only walks forward)
        let _ = pos.make_move_mut(*played_move);

        // Step 3: evaluate resulting position from opponent's POV, negate for mover
        let result_after = Search::new().best_move(&pos, &config);
        let score_after = match result_after {
            SearchResult::EngineMove(_, s) => -s, // negate: opponent's score → mover's score
        };

        // Step 4: eval_loss = best_score - score_after (how much worse than best?)
        // Cap at 0 to handle search noise
        let eval_loss = (best_score - score_after).max(0);

        // Step 5: classify, check for Brilliant
        let mut classification = classify(eval_loss);
        if classification == MoveClass::Excellent || classification == MoveClass::Good {
            // Brilliant: near-best move AND destination was under opponent attack (sacrifice-like)
            let to_sq = played_move.to_sq();
            let opponent = mover.flip();
            // Re-parse pre-move position to check attack
            if let Ok(pre_pos) = Position::from_fen(&fen_before) {
                let dest_attacked = MoveGen::is_attacked(&pre_pos, to_sq, opponent);
                if eval_loss == 0 && dest_attacked {
                    classification = MoveClass::Brilliant;
                }
            }
        }

        all_analyses.push(MoveAnalysis {
            fen_before,
            played_move:    played_move.to_uci(),
            best_move:      best_move_m.to_uci(),
            score_before:   best_score,
            score_after,
            best_score,
            eval_loss,
            classification,
        });
    }

    build_report(all_analyses, &record.result)
}

fn build_report(analyses: Vec<MoveAnalysis>, result: &GameResult) -> GameReport {
    // Split into white and black move lists (white = even indices, black = odd)
    let white_moves: Vec<&MoveAnalysis> = analyses.iter().enumerate()
        .filter(|(i, _)| i % 2 == 0).map(|(_, a)| a).collect();
    let black_moves: Vec<&MoveAnalysis> = analyses.iter().enumerate()
        .filter(|(i, _)| i % 2 == 1).map(|(_, a)| a).collect();

    let accuracy_white = compute_accuracy(&white_moves.iter().map(|a| (*a).clone()).collect::<Vec<_>>());
    let accuracy_black = compute_accuracy(&black_moves.iter().map(|a| (*a).clone()).collect::<Vec<_>>());

    let mut blunders     = [0u8; 2];
    let mut mistakes     = [0u8; 2];
    let mut inaccuracies = [0u8; 2];
    let mut critical     = Vec::new();

    for (i, a) in analyses.iter().enumerate() {
        let side = i % 2; // 0=white, 1=black
        match a.classification {
            MoveClass::Blunder    => blunders[side] = blunders[side].saturating_add(1),
            MoveClass::Mistake    => mistakes[side]  = mistakes[side].saturating_add(1),
            MoveClass::Inaccuracy => inaccuracies[side] = inaccuracies[side].saturating_add(1),
            _ => {}
        }
        if a.eval_loss > 100 { critical.push(i); }
    }

    GameReport {
        move_analyses: analyses,
        summary: GameSummary {
            accuracy_white,
            accuracy_black,
            blunders,
            mistakes,
            inaccuracies,
            critical_move_indices: critical,
        },
    }
}

fn empty_report() -> GameReport {
    GameReport {
        move_analyses: Vec::new(),
        summary: GameSummary {
            accuracy_white: 0.0,
            accuracy_black: 0.0,
            blunders:     [0; 2],
            mistakes:     [0; 2],
            inaccuracies: [0; 2],
            critical_move_indices: Vec::new(),
        },
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_class_thresholds() {
        assert_eq!(classify(0),   MoveClass::Excellent);
        assert_eq!(classify(15),  MoveClass::Good);
        assert_eq!(classify(50),  MoveClass::Inaccuracy);
        assert_eq!(classify(150), MoveClass::Mistake);
        assert_eq!(classify(400), MoveClass::Blunder);
    }

    #[test]
    fn accuracy_perfect_game() {
        let analyses = vec![
            MoveAnalysis { eval_loss: 0,  classification: MoveClass::Excellent,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_before: 0, score_after: 0, best_score: 0 },
            MoveAnalysis { eval_loss: 0,  classification: MoveClass::Excellent,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_before: 0, score_after: 0, best_score: 0 },
        ];
        let acc = compute_accuracy(&analyses);
        assert!((acc - 100.0).abs() < 0.01, "expected 100.0 got {}", acc);
    }

    #[test]
    fn accuracy_all_blunders() {
        let analyses = vec![
            MoveAnalysis { eval_loss: 500, classification: MoveClass::Blunder,
                fen_before: String::new(), played_move: String::new(),
                best_move: String::new(), score_before: 0, score_after: -500, best_score: 0 },
        ];
        let acc = compute_accuracy(&analyses);
        assert!(acc < 50.0, "all-blunder accuracy should be low, got {}", acc);
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --package chess-engine -- analysis::tests
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```bash
git add chess-engine/src/analysis.rs
git commit -m "feat(chess-engine): analysis types — MoveClass, MoveAnalysis, GameReport, classify, compute_accuracy"
```

---

## Task 2: Export analysis from chess-engine lib

**Files:**
- Modify: `chess-engine/src/lib.rs`

- [ ] **Step 1: Add mod and pub-use declarations**

Edit `chess-engine/src/lib.rs` — replace the entire file:

```rust
mod bitboard;
pub mod eval;
mod moves;
mod movegen;
mod position;
pub mod search;
mod tables;
mod types;
pub mod analysis;

pub use analysis::{
    GameRecord, GameReport, GameResult, GameSummary,
    MoveAnalysis, MoveClass,
    analyze_game, classify, compute_accuracy,
    ANALYSIS_DEPTH, ANALYSIS_NODES,
};
pub use bitboard::Bitboard;
pub use moves::{Move, MoveFlag, SavedState};
pub use position::Position;
pub use search::{DifficultyConfig, Search, SearchResult, MATE_SCORE};
pub use types::{CastlingRights, Color, PieceType, Square};
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check --package chess-engine
cargo check --package chess-engine --target wasm32-unknown-unknown
```

Expected: both clean.

- [ ] **Step 3: Commit**

```bash
git add chess-engine/src/lib.rs
git commit -m "feat(chess-engine): export analysis types from crate root"
```

---

## Task 3: analyze_game integration test

**Files:**
- Create: `chess-engine/tests/analysis_integration.rs`

- [ ] **Step 1: Write integration test**

```rust
use chess_engine::{
    analyze_game, GameRecord, GameResult, MoveClass, Position,
};

/// Parse UCI move string against a position's legal moves
fn uci_to_move(pos: &Position, uci: &str) -> chess_engine::Move {
    pos.legal_moves()
        .into_iter()
        .find(|m| m.to_uci() == uci)
        .unwrap_or_else(|| panic!("move {} not found in legal moves for {}", uci, pos.to_fen()))
}

#[test]
fn fools_mate_is_blunder() {
    // 1.f3?? e5  2.g4?? Qh4#  — white's f3 and g4 are blunders
    let start = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let mut pos = Position::from_fen(start).unwrap();

    let mut moves = Vec::new();
    for uci in ["f2f3", "e7e5", "g2g4", "d8h4"] {
        let m = uci_to_move(&pos, uci);
        moves.push(m);
        let _ = pos.make_move_mut(m);
    }

    let record = GameRecord {
        initial_fen: start.to_string(),
        moves,
        result: GameResult::BlackWins,
    };

    let report = analyze_game(&record);
    assert_eq!(report.move_analyses.len(), 4);

    // f3 (move 0, white) should be classified as a blunder or mistake
    let f3_class = &report.move_analyses[0].classification;
    assert!(
        matches!(f3_class, MoveClass::Blunder | MoveClass::Mistake),
        "f3 should be Blunder/Mistake, got {:?}", f3_class
    );

    // g4 (move 2, white) should also be a blunder
    let g4_class = &report.move_analyses[2].classification;
    assert!(
        matches!(g4_class, MoveClass::Blunder | MoveClass::Mistake),
        "g4 should be Blunder/Mistake, got {:?}", g4_class
    );

    // Accuracy stats: white's accuracy should be significantly below 100
    assert!(
        report.summary.accuracy_white < 80.0,
        "white accuracy should be low after two blunders, got {}",
        report.summary.accuracy_white
    );
}

#[test]
fn mate_in_one_is_excellent() {
    // White to play, Ra7-a8#. Should be classified as Excellent or Brilliant.
    let fen = "k7/R7/1K6/8/8/8/8/8 w - - 0 1";
    let pos = Position::from_fen(fen).unwrap();
    let m = uci_to_move(&pos, "a7a8");

    let record = GameRecord {
        initial_fen: fen.to_string(),
        moves: vec![m],
        result: GameResult::WhiteWins,
    };

    let report = analyze_game(&record);
    assert_eq!(report.move_analyses.len(), 1);

    let cls = &report.move_analyses[0].classification;
    assert!(
        matches!(cls, MoveClass::Excellent | MoveClass::Brilliant | MoveClass::Good),
        "Ra8# should be Excellent/Brilliant, got {:?}", cls
    );
}
```

- [ ] **Step 2: Run test**

```bash
cargo test --package chess-engine --test analysis_integration 2>&1 | tail -12
```

Expected: both tests pass. (May take ~10–30s due to search.)

- [ ] **Step 3: Commit**

```bash
git add chess-engine/tests/analysis_integration.rs
git commit -m "test(chess-engine): integration tests for analyze_game — blunder detection, mate classification"
```

---

## Task 4: Add GameHistory to board.rs

Track every move played during the game so the analyzer can replay it.

**Files:**
- Modify: `src/board.rs`

- [ ] **Step 1: Add imports at top of board.rs**

Add these lines to the existing imports at the top of `src/board.rs`:

```rust
use chess_engine::{Move as EngineMove, Square as EngineSquare};
```

- [ ] **Step 2: Add GameHistory resource and helper function**

After the `BadMoveFlash` component definition (around line 46), add:

```rust
#[derive(Resource, Default)]
pub struct GameHistory {
    pub initial_fen: String,
    pub moves: Vec<EngineMove>,
}

impl GameHistory {
    pub fn reset(&mut self) {
        self.initial_fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1".to_string();
        self.moves.clear();
    }
}
```

- [ ] **Step 3: Add find_engine_move helper function**

Add this free function anywhere before `impl Plugin for BoardPlugin`:

```rust
/// Given a board position and from/to squares, find the matching legal engine move.
/// Prefers queen promotion when multiple matches exist (e.g. different promotion pieces).
fn find_engine_move(pos: &chess_engine::Position, from: EngineSquare, to: EngineSquare) -> Option<EngineMove> {
    pos.legal_moves()
        .into_iter()
        .filter(|m| m.from_sq() == from && m.to_sq() == to)
        .max_by_key(|m| if m.is_promotion() { 1 } else { 0 })
}
```

- [ ] **Step 4: Add reset_game_history system and register it**

Add the system function:

```rust
fn reset_game_history(mut history: ResMut<GameHistory>) {
    history.reset();
}
```

In `BoardPlugin::build()`, add to `OnEnter(AppState::Playing)`:

```rust
.add_systems(OnEnter(AppState::Playing), (reset_board_state, reset_game_history))
```

And register the resource:

```rust
.init_resource::<GameHistory>()
```

- [ ] **Step 5: Record human moves in move_piece**

In the `move_piece` function, add `mut history: ResMut<GameHistory>` to its parameters and add recording logic BEFORE `piece.x = square_x; piece.y = square_y;`:

The full updated parameter list for `move_piece`:

```rust
fn move_piece(
    mut commands:       Commands,
    mut selected_square: ResMut<SelectedSquare>,
    selected_piece:     Res<SelectedPiece>,
    mut turn:           ResMut<PlayerTurn>,
    mut castling_state: ResMut<CastlingState>,
    mut captured:       ResMut<CapturedPieces>,
    mut promotion:      ResMut<PromotionPending>,
    mut valid_moves:    ResMut<ValidMoveSquares>,
    mut history:        ResMut<GameHistory>,
    squares_query:      Query<(Entity, &Square)>,
    mut pieces_query:   Query<(Entity, &mut Piece)>,
    mut reset_event:    EventWriter<ResetSelectedEvent>,
    mut status_event:   EventWriter<GameStatusEvent>,
)
```

Add this recording block INSIDE the `if piece.is_move_valid(...)` branch, right before `piece.x = square_x; piece.y = square_y;` (i.e., before modifying the piece position):

```rust
            // Record this move in game history (before position changes)
            {
                let from_eng = EngineSquare(piece.x * 8 + piece.y);
                let to_eng   = EngineSquare(square_x * 8 + square_y);
                let pre_fen  = build_fen(&pieces_vec, turn.0, &castling_state);
                if let Ok(pre_pos) = chess_engine::Position::from_fen(&pre_fen) {
                    if let Some(eng_mv) = find_engine_move(&pre_pos, from_eng, to_eng) {
                        history.moves.push(eng_mv);
                    }
                }
            }
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/board.rs
git commit -m "feat(board): add GameHistory resource; record human moves as chess_engine::Move"
```

---

## Task 5: Record AI moves in ai.rs

**Files:**
- Modify: `src/ai.rs`

- [ ] **Step 1: Add GameHistory import and parameter**

At the top of `src/ai.rs`, add `GameHistory` to the existing `crate::board` import:

```rust
use crate::board::{CastlingState, GameHistory, GameStatus, GameStatusEvent, PlayerTurn, Taken};
```

Also add `Move as EngineMove` to the chess_engine import:

```rust
use chess_engine::{
    DifficultyConfig, Move as EngineMove, MoveFlag, Position, Search, SearchResult,
};
```

- [ ] **Step 2: Add history recording in ai_apply_move**

Add `mut history: ResMut<GameHistory>` to `ai_apply_move`'s parameter list.

Find the line `let mv = match phase.move_to_apply()` (or wherever `mv: chess_engine::Move` is extracted) and add recording right after the move is confirmed:

Find the existing block in `ai_apply_move` that starts with extracting the move from `AiPhase::Ready`. After extracting `mv`, add:

```rust
    // Record AI move in game history
    history.moves.push(mv);
```

Place this BEFORE any board mutations, immediately after `mv` is extracted from `phase`.

The relevant section of `ai_apply_move` looks like this in the current code:

```rust
fn ai_apply_move(
    ...
) {
    ...
    // Extract move from phase
    let mv = match &*phase {
        AiPhase::Ready(m) => *m,
        _ => return,
    };
    *phase = AiPhase::Idle;
    
    // ADD HERE:
    history.moves.push(mv);
    
    // ... rest of the function (apply move to board entities) ...
```

The full updated parameter list:

```rust
fn ai_apply_move(
    mut commands:    Commands,
    mut phase:       ResMut<AiPhase>,
    mut turn:        ResMut<PlayerTurn>,
    mut castling:    ResMut<CastlingState>,
    mut captured:    ResMut<CapturedPieces>,
    mut history:     ResMut<GameHistory>,
    mut pieces_q:    Query<(Entity, &mut Piece)>,
    mut status_ev:   EventWriter<GameStatusEvent>,
    all_pieces_q:    Query<&Piece>,
    game_config:     Res<GameConfig>,
)
```

(Match the exact signature that exists in your ai.rs — add `mut history: ResMut<GameHistory>` to whatever the current parameter list is.)

- [ ] **Step 3: Verify compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/ai.rs
git commit -m "feat(ai): record AI moves in GameHistory"
```

---

## Task 6: Create src/analysis.rs — Bevy AnalysisPlugin

**Files:**
- Create: `src/analysis.rs`

- [ ] **Step 1: Write src/analysis.rs**

```rust
use bevy::prelude::*;
use chess_engine::{GameRecord, GameResult as EngineGameResult, GameReport, analyze_game};
use crate::board::{GameHistory, GameStatus, GameStatusEvent};
use crate::pieces::PieceColor;
use crate::state::AppState;

// ─── Resources ───────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct AnalysisReport(pub Option<GameReport>);

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Runs synchronously when the game ends (checkmate or stalemate).
/// Reads GameHistory, calls analyze_game, stores result in AnalysisReport.
/// This frame will stutter by ~1–4s; that's acceptable since the game just ended.
pub fn run_post_game_analysis(
    mut events:  EventReader<GameStatusEvent>,
    history:     Res<GameHistory>,
    mut report:  ResMut<AnalysisReport>,
) {
    for ev in events.read() {
        let engine_result = match &ev.0 {
            GameStatus::Checkmate { winner } => match winner {
                PieceColor::White => EngineGameResult::WhiteWins,
                PieceColor::Black => EngineGameResult::BlackWins,
            },
            GameStatus::Stalemate => EngineGameResult::Draw,
            _ => continue,  // Check or Ok — game isn't over, skip
        };

        if history.moves.is_empty() { continue; }

        let record = GameRecord {
            initial_fen: history.initial_fen.clone(),
            moves:       history.moves.clone(),
            result:      engine_result,
        };

        report.0 = Some(analyze_game(&record));
    }
}

fn reset_analysis(mut report: ResMut<AnalysisReport>) {
    report.0 = None;
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct AnalysisPlugin;

impl Plugin for AnalysisPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AnalysisReport>()
            .add_systems(OnEnter(AppState::Playing), reset_analysis)
            .add_systems(Update, run_post_game_analysis.run_if(in_state(AppState::Playing)));
    }
}
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/analysis.rs
git commit -m "feat(analysis): AnalysisPlugin — run_post_game_analysis on game end"
```

---

## Task 7: Register AnalysisPlugin in src/lib.rs

**Files:**
- Modify: `src/lib.rs`

- [ ] **Step 1: Add mod and plugin registration**

Add `mod analysis;` to the module list at the top.

Add `use analysis::AnalysisPlugin;` to the use list.

Add `AnalysisPlugin` to the plugin list in `run_app()`.

**The run_post_game_analysis system must run BEFORE handle_status_events in ui.rs** so that `AnalysisReport` is populated when the overlay spawns. Enforce this with system ordering in `run_app()`:

```rust
.add_plugins((HomePlugin, BoardPlugin, PiecesPlugin, CapturedPlugin, UIPlugin, AIPlugin, AnalysisPlugin))
```

Then add explicit ordering after the plugins line:

```rust
.add_systems(Update,
    analysis::run_post_game_analysis
        .before(ui_handle_status_stub)  // see below
        .run_if(in_state(AppState::Playing))
)
```

Actually the cleanest way without cross-module function references is to order by plugin insertion order within the same set. Since Bevy processes `EventReader`s per-system (each reader has its own cursor), both `run_post_game_analysis` (in AnalysisPlugin) and `handle_status_events` (in UIPlugin) read the same `GameStatusEvent` independently — no consumption conflict.

The ordering issue: `run_post_game_analysis` needs to run BEFORE `spawn_game_over_overlay` which is called from `handle_status_events`.

Enforce ordering: in `src/lib.rs`, after adding plugins, add:

```rust
.add_systems(Update,
    analysis::run_post_game_analysis
        .run_if(in_state(AppState::Playing))
        .before(ui::handle_status_events_label)  // not possible without label
)
```

**Simpler approach**: make `run_post_game_analysis` a `First` schedule system (runs before `Update`). Since `handle_status_events` is in `Update`, this guarantees ordering.

Update `AnalysisPlugin::build()` to use the `First` schedule:

```rust
impl Plugin for AnalysisPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AnalysisReport>()
            .add_systems(OnEnter(AppState::Playing), reset_analysis)
            .add_systems(First,
                run_post_game_analysis.run_if(in_state(AppState::Playing))
            );
    }
}
```

This guarantees `AnalysisReport` is written before any `Update` system reads it.

The `lib.rs` change is just registering the plugin. Edit `src/lib.rs`:

```rust
mod analysis;
mod ai;
mod board;
mod captured;
mod home;
mod pieces;
mod state;
mod ui;

// ... (imports) ...
use analysis::AnalysisPlugin;

// in run_app():
.add_plugins((HomePlugin, BoardPlugin, PiecesPlugin, CapturedPlugin, UIPlugin, AIPlugin, AnalysisPlugin))
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs src/analysis.rs
git commit -m "feat: register AnalysisPlugin; schedule analysis in First to precede UI update"
```

---

## Task 8: Show analysis summary in game-over overlay

**Files:**
- Modify: `src/ui.rs`

Add the `AnalysisReport` resource to the `handle_status_events` system and include a summary block in `spawn_game_over_overlay`.

- [ ] **Step 1: Add import**

In `src/ui.rs`, add:

```rust
use crate::analysis::AnalysisReport;
```

- [ ] **Step 2: Pass AnalysisReport to handle_status_events**

Update `handle_status_events` signature to include the report:

```rust
fn handle_status_events(
    mut events:      EventReader<GameStatusEvent>,
    mut commands:    Commands,
    asset_server:    Res<AssetServer>,
    analysis_report: Res<AnalysisReport>,
    overlay_q:       Query<Entity, With<GameOverOverlay>>,
    banner_q:        Query<Entity, With<CheckBanner>>,
) {
```

In the `Checkmate` branch, replace:

```rust
spawn_game_over_overlay(&mut commands, &asset_server, winner_str, false);
```

with:

```rust
spawn_game_over_overlay(&mut commands, &asset_server, winner_str, false, analysis_report.0.as_ref());
```

In the `Stalemate` branch:

```rust
spawn_game_over_overlay(&mut commands, &asset_server, "¡Empate por ahogado!", true, analysis_report.0.as_ref());
```

- [ ] **Step 3: Update spawn_game_over_overlay signature and body**

Replace the entire `spawn_game_over_overlay` function:

```rust
fn spawn_game_over_overlay(
    commands:  &mut Commands,
    asset_server: &AssetServer,
    title: &str,
    is_draw: bool,
    report: Option<&chess_engine::GameReport>,
) {
    let font: Handle<Font> = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(16.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::rgba(0.0, 0.0, 0.0, 0.85)),
                ..default()
            },
            GameOverOverlay,
        ))
        .with_children(|root| {
            // Title (e.g., "¡Blancas ganan!")
            root.spawn(TextBundle::from_section(
                title,
                TextStyle { font: font.clone(), font_size: 52.0, color: Color::rgb(1.0, 0.9, 0.2) },
            ));

            // Analysis summary block (if available)
            if let Some(r) = report {
                let s = &r.summary;

                // Accuracy line
                root.spawn(TextBundle::from_section(
                    format!(
                        "Precisión  —  Blancas: {:.0}%   Negras: {:.0}%",
                        s.accuracy_white, s.accuracy_black
                    ),
                    TextStyle { font: font.clone(), font_size: 26.0, color: Color::rgb(0.8, 0.9, 1.0) },
                ));

                // Error counts
                root.spawn(TextBundle::from_section(
                    format!(
                        "Blancas: ??{}  ?{}  ⚠{}      Negras: ??{}  ?{}  ⚠{}",
                        s.blunders[0], s.mistakes[0], s.inaccuracies[0],
                        s.blunders[1], s.mistakes[1], s.inaccuracies[1],
                    ),
                    TextStyle { font: font.clone(), font_size: 22.0, color: Color::rgb(0.7, 0.7, 0.7) },
                ));

                // Critical moments (top 3 blunders)
                let critical: Vec<String> = r.summary.critical_move_indices
                    .iter()
                    .take(3)
                    .filter_map(|&i| r.move_analyses.get(i).map(|a| (i, a)))
                    .map(|(i, a)| {
                        let side = if i % 2 == 0 { "B" } else { "N" };
                        let move_num = i / 2 + 1;
                        format!("Mov {}: {} {}", move_num, side, a.played_move)
                    })
                    .collect();

                if !critical.is_empty() {
                    root.spawn(TextBundle::from_section(
                        format!("Momentos clave: {}", critical.join("  |  ")),
                        TextStyle { font: font.clone(), font_size: 20.0, color: Color::rgb(1.0, 0.5, 0.2) },
                    ));
                }
            }

            // Retry button (only for non-draw results)
            if !is_draw {
                root.spawn((
                    ButtonBundle {
                        style: Style {
                            width: Val::Px(240.0), height: Val::Px(56.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::rgba(0.1, 0.4, 0.1, 0.9)),
                        ..default()
                    },
                    BtnRetry,
                ))
                .with_children(|p| {
                    p.spawn(TextBundle::from_section(
                        "Reintentar",
                        TextStyle { font: font.clone(), font_size: 28.0, color: Color::rgb(0.9, 0.9, 0.9) },
                    ));
                });
            }

            // Home button
            root.spawn((
                ButtonBundle {
                    style: Style {
                        width: Val::Px(240.0), height: Val::Px(56.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: BackgroundColor(Color::rgba(0.3, 0.1, 0.1, 0.9)),
                    ..default()
                },
                BtnHome,
            ))
            .with_children(|p| {
                p.spawn(TextBundle::from_section(
                    "Menú principal",
                    TextStyle { font, font_size: 28.0, color: Color::rgb(0.9, 0.9, 0.9) },
                ));
            });
        });
}
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/ui.rs
git commit -m "feat(ui): show accuracy and error counts in game-over overlay"
```

---

## Task 9: Integration test — local run

**Files:** None (testing only)

- [ ] **Step 1: Run all tests**

```bash
cargo test --workspace 2>&1 | tail -15
```

Expected: all pass (including `analysis_integration` and `chess-engine` unit tests).

- [ ] **Step 2: Build WASM and run locally**

```bash
wasm-pack build --target web --out-dir web/pkg
```

Then use the `run-local` skill to serve and open in browser.

- [ ] **Step 3: Golden path verification**

Play a quick game to checkmate (e.g. Fool's Mate: 1.f3 e5 2.g4 Qh4#):
1. Home screen → "Player VS Player" → board appears
2. Play 1.f3 e5 2.g4 Qh4# (Black wins by Fool's Mate)
3. Game-over overlay appears showing:
   - "¡Jaque Mate! ¡Negras ganan!"
   - Accuracy line: "Precisión — Blancas: XX%  Negras: XX%"
   - Error counts line
   - Possibly "Momentos clave" if blunders detected
4. Verify numbers are plausible (white should show poor accuracy after f3/g4)
5. Overlay appears within ~5 seconds of the final move (analysis may stutter)

- [ ] **Step 4: Test PvC game as well**

Play a PvC game to completion (let the AI checkmate you by playing badly):
1. Home → Player VS Computer → Medio → play until game ends
2. Verify analysis overlay shows for both sides

- [ ] **Step 5: Fix and commit any issues found**

```bash
git add -p
git commit -m "fix: <describe issue>"
```

---

## Success Criteria

Before marking this plan complete:

- [ ] `cargo test --workspace` passes (all analysis tests + integration test)
- [ ] `cargo check --target wasm32-unknown-unknown` clean
- [ ] After Fool's Mate: game-over overlay shows accuracy percentages for both sides
- [ ] After any game: analysis runs and completes (may take a few seconds)
- [ ] Analysis doesn't crash or panic on short games (1–2 moves)
- [ ] "Nueva Partida" resets history correctly (next game produces fresh analysis)
- [ ] Classic mode (if any) is unaffected

---

## Known Limitations (v1)

- Analysis depth 6 / 200k nodes — strong enough for blunder detection, not grandmaster-level. Backend deep analysis (depth 18) is Phase 2.
- The `critical_move_indices` pointer trick using `std::ptr::eq` in `spawn_game_over_overlay` is fragile. If the iterator produces wrong indices, use `enumerate()` tracking instead.
- Castling and en-passant moves played by the human player are not recorded (the human UI doesn't support them). AI castling IS recorded since it already produces `chess_engine::Move`.
- Analysis result is not persisted after leaving the Playing state — Phase 2 will store it in a database.
