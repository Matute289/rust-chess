---
name: ai-search
description: Implementing and debugging the AI search stack — negamax, alpha-beta, iterative deepening, transposition tables, move ordering, evaluation. Use when working on chess-engine/src/search.rs or chess-engine/src/eval.rs.
---

# AI Search — rust-chess

Search lives in `chess-engine/src/search.rs`. Evaluation in `chess-engine/src/eval.rs`.

## Layer Stack — Build and Test in This Order

Each layer must be correct before adding the next. Test after each step.

```
1. Negamax (no pruning)          → correct but slow, use for reference
2. + Alpha-Beta                  → same result as 1, much faster
3. + Iterative Deepening         → wraps 2, always returns a move
4. + Transposition Table         → same result as 3, faster
5. + Move Ordering               → same result, dramatically faster
6. + Null Move Pruning           → may differ slightly (acceptable)
7. + Late Move Reduction         → may differ slightly (acceptable)
8. + Quiescence Search           → fixes horizon effect, results improve
```

Layers 1–5: results must be **identical** at same depth. If they differ, there's a bug.
Layers 6–8: results may legitimately differ (pruning can miss some lines), but quality should improve.

## Search Struct

```rust
pub struct Search {
    tt: TranspositionTable,
    killers: [[Move; 2]; MAX_PLY],   // two killer moves per ply
    history: [[i32; 64]; 64],        // history[from][to] score
    nodes: u64,                      // node counter for debugging
    stop: bool,                      // set by time manager or UCI "stop"
}

impl Search {
    pub fn best_move(&mut self, pos: &Position, config: &DifficultyConfig) -> SearchResult;
    fn negamax(&mut self, pos: &Position, depth: i32, alpha: i32, beta: i32) -> i32;
    fn quiescence(&mut self, pos: &Position, alpha: i32, beta: i32) -> i32;
}
```

## Alpha-Beta — Common Bugs

**Bug: score sign flip**
Negamax always evaluates from the current side's perspective. After `make_move`, negate the score:
```rust
let score = -self.negamax(&child_pos, depth - 1, -beta, -alpha);
//          ^ THIS NEGATION IS MANDATORY
```

**Bug: checkmate score wrong depth**
Checkmate should be scored as `MATE - ply` (slightly less than MATE for farther mates):
```rust
const MATE: i32 = 30_000;
// In negamax, when no legal moves and in check:
return -(MATE - ply as i32);
// NOT just -MATE, or the engine won't prefer faster mates
```

**Bug: stalemate not handled**
When `legal_moves()` is empty AND NOT in check → return 0 (draw), not -MATE.

## Transposition Table — Common Bugs

**Bug: hash collision not handled**
Always verify `entry.hash == pos.hash` before using a TT hit. Different positions can hash to the same index.

**Bug: using TT score as exact when it's a bound**
```rust
match entry.flag {
    TTFlag::Exact => return entry.score,          // exact — use directly
    TTFlag::LowerBound => alpha = alpha.max(entry.score),  // we know score >= this
    TTFlag::UpperBound => beta = beta.min(entry.score),    // we know score <= this
}
if alpha >= beta { return entry.score; }           // cutoff
```

**Bug: TT stores wrong score at root**
At the root, don't use TT to skip the move — use it only to reorder (TT best move first).

**Bug: stale TT entries at wrong depth**
Only use a TT entry if `entry.depth >= remaining_depth`. Shallower entries can cause incorrect pruning.

## Move Ordering — Verify With Node Count

Good move ordering should cut node count by 4–10× compared to no ordering. Test:

```bash
cargo test --package chess-engine search::node_count -- --nocapture
```

Order: `TT move → captures (MVV-LVA) → killers → history → quiet moves`

MVV-LVA score: `victim_value * 10 - attacker_value` (take queen with pawn scores highest).

## Iterative Deepening + Time Management

```rust
pub fn best_move(&mut self, pos: &Position, config: &DifficultyConfig) -> SearchResult {
    let deadline = now() + config.time_limit_ms;
    let mut best = None;

    for depth in 1..=config.max_depth {
        let score = self.negamax(pos, depth, -INFINITY, INFINITY);
        if self.stop { break; }        // time ran out mid-search
        best = Some((self.root_best_move, score));
    }

    best.map(|(m, s)| SearchResult::EngineMove(m, s))
        .unwrap_or_else(|| SearchResult::EngineMove(legal_moves[0], 0))
}
```

Check `self.stop` every ~2048 nodes (not every node — the check itself is expensive):
```rust
if self.nodes & 2047 == 0 {
    self.stop = now() >= self.deadline;
}
```

## Evaluation — Tuning Order

Start simple, verify correctness, add components one by one:

1. **Material only** — engine shouldn't hang pieces
2. **+ PST** — engine should develop pieces, prefer center
3. **+ Mobility** — engine should avoid cramped positions
4. **+ Pawn structure** — engine should avoid doubled/isolated pawns
5. **+ King safety** — engine should castle and protect king
6. **+ Center control** — engine should contest center

Test each level informally by playing against it. Regression test: known tactical positions should be solved at their expected depth.

## Difficulty Random Factor

```rust
// After getting the sorted move list, for random_factor > 0:
if config.random_factor > 0.0 {
    let n_candidates = (move_list.len() as f32 * config.random_factor).ceil() as usize;
    let n_candidates = n_candidates.max(1).min(move_list.len());
    return move_list[rng.gen_range(0..n_candidates)];
}
```

Use a seeded RNG (not `rand::thread_rng`) so that games are reproducible for debugging.

## Debugging a Bad Move

When the engine plays an obviously bad move:

1. Get the FEN of the position before the bad move
2. Run `cargo run --bin uci` → `position fen <FEN>` → `go depth 8`
3. Check the engine's best move and score
4. If wrong: reduce depth to 1 and check. If depth 1 is fine but depth 2 is not → `make_move`/`unmake_move` bug
5. If depth 1 is wrong → evaluation bug

## WASM Async (Bevy Integration)

The search must NOT block the Bevy main thread. In `bevy-frontend/src/game/ai.rs`:

```rust
fn spawn_ai_task(
    mut commands: Commands,
    pool: Res<AsyncComputeTaskPool>,
    game_state: Res<GameState>,
) {
    let pos = game_state.position.clone();
    let config = game_state.difficulty_config.clone();
    let task = pool.spawn(async move {
        let mut search = Search::new();
        search.best_move(&pos, &config)
    });
    commands.spawn(AiTask(task));
}

fn poll_ai_task(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut AiTask)>,
    mut move_events: EventWriter<AIMoveReady>,
) {
    for (entity, mut task) in &mut tasks {
        if let Some(result) = block_on(future::poll_once(&mut task.0)) {
            move_events.send(AIMoveReady(result));
            commands.entity(entity).despawn();
        }
    }
}
```
