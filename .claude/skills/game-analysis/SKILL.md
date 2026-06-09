---
name: game-analysis
description: Working on the game analyzer — move classification, accuracy calculation, brilliant detection, GameReport generation. Use when working on chess-engine/src/analysis.rs or the post-game UI.
---

# Game Analysis — rust-chess

Analyzer lives in `chess-engine/src/analysis.rs`. Reuses the `Search` engine from `search.rs` directly — no separate search instance needed.

## Core Invariant

The analyzer evaluates every position **twice at the same depth** — once before the move (to get best_move + best_score) and once after (to get score_after). The difference is `eval_loss`. Consistency requires using the same depth for both evaluations.

## Analysis Depths

| Context | Depth | Rationale |
|---|---|---|
| Client-side (WASM, post-game) | 10 | Fast, ~seconds per move |
| Backend (deep, async) | 18 | Quality, ~minutes per game |

The client analysis runs immediately and shows results. The backend analysis runs as a background job and updates the UI when complete (Phase 2).

## eval_loss Calculation

```rust
// IMPORTANT: signs
// score_before: from side_to_move's perspective (positive = good for them)
// After make_move, it's the opponent's turn
// score_after must be negated to compare in the same frame of reference

let score_before = search.negamax(&pos, depth, -INFINITY, INFINITY);
let best_move = search.root_best_move;
let best_score = score_before;  // what the best move would have scored

pos.make_move_mut(played_move);
let score_after_opponent = search.negamax(&pos, depth, -INFINITY, INFINITY);
pos.unmake_move_mut(played_move, saved);

let score_after = -score_after_opponent;  // ← NEGATION REQUIRED
let eval_loss = best_score - score_after; // always >= 0
```

If `eval_loss` is negative, there's a sign bug. `eval_loss` should never be negative (the best move by definition can't score less than the played move if search is consistent).

## MoveClass Thresholds

```rust
pub fn classify(eval_loss: i32, played: Move, best: Move, pos: &Position) -> MoveClass {
    if played == best {
        if is_brilliant(played, pos) { return MoveClass::Brilliant; }
        return MoveClass::Excellent;
    }
    match eval_loss {
        0..=19   => MoveClass::Good,
        20..=99  => MoveClass::Inaccuracy,
        100..=299 => MoveClass::Mistake,
        _        => MoveClass::Blunder,
    }
}
```

Note: `played == best` means same from+to+flags. Two promotions to different pieces are different moves.

## Brilliant Detection

A Brilliant move must satisfy ALL three conditions:
1. `eval_loss == 0` (or ≤ 5cp — allow for search noise at deeper depths)
2. The piece that moved lands on a square attacked by an opponent piece: `pos.attackers_to(to_square, opponent_color).any()`
3. The full evaluation of the resulting position is significantly better than material suggests: `full_eval - material_eval > 50cp`

Condition 3 catches sacrifices — giving up material for positional compensation that the engine sees as winning. Without it, any equal recapture would be "Brilliant".

## Accuracy Formula

Chess.com-style accuracy (0–100):

```rust
pub fn accuracy(move_analyses: &[MoveAnalysis]) -> f32 {
    // Win% before and after each move
    // win_pct(cp) = 50 + 50 * (2/(1 + exp(-0.00368 * cp)) - 1)
    let total_loss: f32 = move_analyses.iter()
        .map(|m| win_pct(m.best_score) - win_pct(m.score_after))
        .sum();
    let max_possible_loss = move_analyses.len() as f32 * 100.0;
    100.0 - (total_loss / max_possible_loss * 100.0)
}

fn win_pct(cp: i32) -> f32 {
    50.0 + 50.0 * (2.0 / (1.0 + (-0.00368 * cp as f32).exp()) - 1.0)
}
```

## Critical Positions

A position is "critical" if `eval_loss > 100cp` (Mistake or Blunder). Store the FEN before the move. These FENs feed into the Learning Mode as learning events.

```rust
let critical: Vec<String> = analyses.iter()
    .filter(|m| m.eval_loss > 100)
    .map(|m| m.fen_before.clone())
    .collect();
```

## Testing the Analyzer

Use famous blunders to verify classification:

```rust
#[test]
fn test_fischer_blunder() {
    // Fischer vs Spassky 1972, Game 1 — Fischer's 29. Bxh2 is a famous blunder
    let record = GameRecord::from_pgn("...").unwrap();
    let report = analyze(&record, 10);
    let move_29 = &report.move_analyses[28];
    assert_eq!(move_29.classification, MoveClass::Blunder);
    assert!(move_29.eval_loss > 300);
}
```

Collect 5–10 well-known tactical positions with verified best moves to regression-test the analyzer.

## Bevy Integration — Post-Game UI

After `GameStatus` transitions to `Checkmate | Stalemate | Draw`, `game/ui.rs` triggers analysis:

```rust
fn on_game_end(
    mut game_end: EventReader<GameEnded>,
    game_state: Res<GameState>,
    pool: Res<AsyncComputeTaskPool>,
    mut commands: Commands,
) {
    for _ in game_end.read() {
        let record = game_state.to_game_record();
        let task = pool.spawn(async move {
            chess_engine::analysis::analyze(&record, ANALYSIS_DEPTH_CLIENT)
        });
        commands.spawn(AnalysisTask(task));
    }
}
```

The analysis panel shows:
- Move list with colored badges (Brilliant=cyan, Excellent=green, Good=white, Inaccuracy=yellow, Mistake=orange, Blunder=red)
- Accuracy % for each player
- Summary counts (blunders, mistakes, inaccuracies)
- "Deep analysis in progress..." spinner until backend result arrives (Phase 2)

## Backend Analysis Endpoint (Phase 2 foundation)

The `GameReport` struct is serializable (`serde::Serialize`). In Phase 2, the backend endpoint `POST /api/games/{id}/analyze` runs depth-18 analysis and stores `MoveAnalysis` records in the `learning_events` table. The `analysis.rs` module doesn't need to change — the backend just calls the same `analyze()` function with a higher depth.
