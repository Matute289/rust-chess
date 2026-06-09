---
name: chess-engine
description: Working on the chess-engine crate — bitboards, magic move generation, Position API, Zobrist hashing, perft testing. Use when adding or modifying any code in chess-engine/src/.
---

# Chess Engine — rust-chess

Crate lives at `chess-engine/`. Pure Rust, no Bevy, no Axum. Compiles to both `wasm32-unknown-unknown` and native.

## Crate Invariants

- **No std dependencies that break WASM**: avoid `std::time`, `std::thread`, `std::fs` in `chess-engine`. Use `cfg(not(target_arch = "wasm32"))` guards if needed.
- **Position is immutable by default**: `make_move(&self) -> Position` for search tree. `make_move_mut(&mut self)` only for analysis/perft.
- **Zobrist hash must stay consistent**: every `make_move` call must XOR the hash correctly or the TT will silently break. When adding new state (e.g. new piece type, new flag), add Zobrist keys immediately.

## File Map

| File | Responsibility |
|---|---|
| `bitboard.rs` | `Bitboard(u64)` newtype, bit ops, square iteration, masks (ranks, files, diagonals) |
| `position.rs` | `Position` struct, `Color`, `PieceType`, `Square`, `CastlingRights`, Zobrist hash table |
| `moves.rs` | `Move(u32)` packed type, `MoveFlag` enum, move constructors, `SavedState` for unmake |
| `movegen.rs` | Magic bitboard tables, `MoveGen` struct, `legal_moves()`, `pseudo_legal_moves()` |
| `eval.rs` | `evaluate(pos) -> i32` — centipawns from side-to-move perspective |
| `search.rs` | `Search` struct, `best_move()`, `SearchResult` enum |
| `opening_book.rs` | `OpeningBook`, Polyglot `.bin` reader |
| `analysis.rs` | `GameAnalyzer`, `GameRecord`, `GameReport`, `MoveAnalysis`, `MoveClass` |
| `uci.rs` | UCI protocol loop (binary entry point, not part of lib API) |

## Bitboard Conventions

```rust
// Square encoding: A1=0, B1=1, ..., H1=7, A2=8, ..., H8=63
// Bit i is set if square i is occupied

// Common patterns
let white_pawns: Bitboard = pos.pieces[Color::White][PieceType::Pawn];
let all_pieces: Bitboard = pos.occupied();        // union of all 12 BBs
let empty: Bitboard = !all_pieces;

// Iteration over set bits
for sq in white_pawns.squares() {                 // yields Square values
    // process square sq
}

// Attacks
let knight_attacks = KNIGHT_ATTACKS[sq];          // precomputed table
let bishop_attacks = bishop_attacks(sq, blockers); // magic lookup
```

## Move Encoding

```rust
// bits 0-5:   from square (0-63)
// bits 6-11:  to square (0-63)
// bits 12-15: flags
pub enum MoveFlag {
    Quiet         = 0,
    DoublePush    = 1,  // pawn double push — sets en passant square
    KingSideCastle = 2,
    QueenSideCastle = 3,
    Capture       = 4,
    EnPassant     = 5,
    // promotions 8-15
    PromoKnight   = 8,
    PromoBishop   = 9,
    PromoRook     = 10,
    PromoQueen    = 11,
    PromoKnightCapture = 12,
    // ...
}
```

## Magic Bitboards — How to Debug

Magic bitboards are precomputed at startup. If move generation produces wrong attacks for sliders:

1. `cargo test -- --nocapture movegen::bishop_attacks` — verify individual squares
2. Check that the magic number and shift for the failing square match the precomputed table
3. Use `perft(1)` on a custom position to count expected moves
4. If magic tables corrupt, re-run the magic finder (see `movegen.rs::find_magics`)

## Perft — Correctness Gating

**Run before any AI work.** All of these must pass:

```bash
cargo test --package chess-engine -- perft
```

| Position | Depth | Expected nodes |
|---|---|---|
| Startpos | 5 | 4,865,609 |
| Kiwipete (`r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq -`) | 4 | 4,085,603 |
| Position 3 (`8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - -`) | 5 | 674,624 |
| Position 5 (`rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8`) | 4 | 2,103,487 |

If perft fails:
1. Bisect: test depth 1 first. Count expected moves for the position manually or with a known-good engine.
2. If depth 1 passes but depth 2 fails: the bug is in `make_move` or `unmake_move` (state corruption).
3. If depth 1 fails: the bug is in move generation (missing or extra moves).
4. Use `perft_divide(depth)` to get per-move node counts — compare with Stockfish output to find the diverging branch.

## Adding a New Piece Rule

When modifying game rules (e.g., fixing castling, adding promotion variants):

1. Update `movegen.rs` for the new move generation logic
2. Update `position.rs::make_move` and `unmake_move` for the new state
3. Update `position.rs::hash` — add/XOR the right Zobrist keys
4. Update `uci.rs` if the move needs different string encoding
5. Run perft suite — **do not proceed until all perft values match**

## Building

```bash
# Check engine compiles for WASM
cargo check --package chess-engine --target wasm32-unknown-unknown

# Run engine tests (native)
cargo test --package chess-engine

# Run with output (for perft prints)
cargo test --package chess-engine -- --nocapture
```
