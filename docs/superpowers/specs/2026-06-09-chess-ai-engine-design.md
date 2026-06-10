# Chess AI Engine — Design Spec
**Phases 1 + 1.5 | Date: 2026-06-09**

---

## Implementation Progress

> **Convention:** When a plan is fully complete (tested locally + on mobile, committed and pushed to `browser`), update this section before doing `/clear`. This keeps the spec as the persistent context across sessions — plans are ephemeral, the spec is the record.

| Sub-project | Plan file | Status | Completed | Notes |
|---|---|---|---|---|
| 1 — Chess Engine | `plans/2026-06-09-chess-engine-sub1.md` | ✅ Done | 2026-06-09 | All tasks: types, bitboard, magic tables, position, movegen, perft. WASM-compatible. |
| 2 — AI Engine | `plans/2026-06-09-chess-ai-engine-sub2.md` | ✅ Done | 2026-06-09 | Eval (material+PST+pawn structure+mobility+king safety), negamax+alpha-beta+ID+TT+move ordering+quiescence+null-move+LMR. |
| 3 — AI Integration (Bevy) | `plans/2026-06-09-chess-ai-integration-sub3.md` | ✅ Done | 2026-06-09 | FEN builder, AIPlugin, CastlingState, difficulty resource, Tab cycling. Post-plan fixes: check detection, board system ordering, promotion color. |
| 4 — Frontend Overhaul | `plans/2026-06-09-frontend-overhaul-sub4.md` | ✅ Done | 2026-06-09 | AppState (Home/Playing), HomePlugin (PvP/PvC/PvL + difficulty select + OAuth stub), CapturedPlugin, check banner, game-over overlay, pawn promotion mesh, AI timing, touch/mobile fixes. |
| 3 (Phase 1.5) — Game Analyzer | `plans/2026-06-10-game-analyzer.md` | ✅ Done | 2026-06-10 | `chess-engine/src/analysis.rs`: `analyze_game`, `MoveClass`, `GameReport`, `classify`, `compute_accuracy`. Bevy `AnalysisPlugin` + `GameHistory`. Depth-6 / 200k nodes (not depth-10). Accuracy + error counts + critical moments shown in game-over overlay. |
| 5 — Legal Move Validation | `plans/2026-06-10-legal-move-validation.md` | ✅ Done | 2026-06-10 | Replaced `is_move_valid + would_leave_king_in_check` with engine `legal_moves()`. `legal_squares_for` + `engine_valid_squares` in board.rs. Pinned pieces and king-in-check moves correctly blocked. 3 unit tests. |
| 6 — Human Castling | `plans/2026-06-10-human-castling.md` | ✅ Done | 2026-06-10 | `move_piece` saves `eng_mv_flag` from history block; teleports rook on `KingSideCastle`/`QueenSideCastle` before check detection. `drop`+re-borrow pattern for borrow safety. 1 unit test. |

### Deviations from original spec
- **Sub-project 3 naming**: The plan covers Bevy AI integration (originally listed as sub-project 3 in the roadmap), NOT the Game Analyzer. The Game Analyzer is Phase 1.5 and comes after.
- **WASM threading**: Search runs synchronously (node-count bounded) rather than `AsyncComputeTask` for WASM compatibility. The async approach is deferred to native builds.
- **En passant**: FEN always passes `-` for en passant target — engine won't generate en passant moves. Acceptable for v1.
- **Tablebases**: Backend not yet implemented. The `GET /api/tablebase` endpoint is out of scope until Phase 1.5.
- **Human move validation**: ~~Resolved in Sub-project 5~~ — engine `legal_moves()` now enforces all rules including pins and king safety.

---

## Scope

This spec covers sub-projects 1 and 2 of the overall roadmap:

- **Sub-project 1 — Chess Engine**: legal move generation, game state, FEN, bitboards
- **Sub-project 2 — AI Engine**: minimax search, evaluation, difficulty levels, opening book, tablebases
- **Sub-project 3 — Game Analyzer (Phase 1.5)**: post-game move classification, reports

It does NOT cover: user accounts, learning mode, multiplayer, or generative AI (separate specs).

---

## Architecture Decision: Option A

- **chess-engine** runs client-side in WASM (zero latency, offline-capable for levels 1–4)
- **backend** (Axum) serves only Syzygy tablebase queries in Phase 1
- **bevy-frontend** retains Classic mode untouched; new "Jugar" mode consumes chess-engine

---

## Workspace Structure

```
rust-chess/
├── Cargo.toml                  ← workspace root [chess-engine, bevy-frontend, backend]
├── chess-engine/
│   ├── Cargo.toml              ← no Bevy, no Axum; compiles wasm32 + native
│   └── src/
│       ├── lib.rs
│       ├── bitboard.rs         ← Bitboard(u64), masks, ops
│       ├── position.rs         ← Position struct, Zobrist hash
│       ├── moves.rs            ← Move(u32) type, flags
│       ├── movegen.rs          ← magic bitboards, legal move generation
│       ├── eval.rs             ← evaluation function
│       ├── search.rs           ← negamax, alpha-beta, iterative deepening, TT
│       ├── opening_book.rs     ← Polyglot .bin reader
│       ├── analysis.rs         ← GameAnalyzer, MoveClass, GameReport
│       └── uci.rs              ← UCI protocol (testing + future backend integration)
├── bevy-frontend/
│   ├── Cargo.toml              ← depends on chess-engine
│   └── src/
│       ├── lib.rs
│       ├── menu.rs             ← AppState, main menu (Jugar / Clásico)
│       ├── classic/            ← existing code, untouched
│       │   ├── board.rs
│       │   ├── pieces.rs
│       │   └── ui.rs
│       └── game/               ← new mode, chess-engine powered
│           ├── mod.rs
│           ├── board.rs
│           ├── pieces.rs
│           ├── input.rs
│           ├── ai.rs
│           └── ui.rs
└── backend/
    ├── Cargo.toml              ← depends on chess-engine (native only)
    └── src/
        ├── main.rs
        ├── config.rs
        ├── error.rs
        └── routes/
            └── tablebase.rs
```

---

## Sub-project 1: Chess Engine

### Board Representation

12 bitboards — one per (piece type × color):

```rust
pub struct Position {
    pieces: [[Bitboard; 6]; 2],   // [Color][PieceType]
    side_to_move: Color,
    castling_rights: CastlingRights,  // 4 bits: KQkq
    en_passant: Option<Square>,
    halfmove_clock: u8,
    fullmove_number: u16,
    hash: u64,                        // Zobrist hash, updated incrementally
}
```

`Bitboard` is a newtype over `u64`. Bit `i` = square `i` (A1=0 … H8=63).

### Move Type

```rust
// packed u32:
// bits  0-5:  from square
// bits  6-11: to square
// bits 12-15: flags (quiet, capture, ep, castling, promotion piece)
// bits 16-31: ordering score (set by move ordering, not part of move identity)
pub struct Move(u32);
```

### Move Generation

- **Leaper pieces** (knight, king): precomputed attack tables `[Square; 64]` → `Bitboard`
- **Slider pieces** (bishop, rook, queen): **Magic Bitboards** — `magic[square][blockers_masked * magic >> shift]` → attack `Bitboard`. Tables computed at startup.
- **Pawns**: shifts + rank masks for single push, double push, captures, en passant
- Generation pipeline: pseudo-legal moves → filter leaving king in check → legal moves

### Position API

```rust
impl Position {
    pub fn from_fen(fen: &str) -> Result<Position, FenError>;
    pub fn to_fen(&self) -> String;
    pub fn legal_moves(&self) -> Vec<Move>;
    pub fn make_move(&self, m: Move) -> Position;       // immutable for search tree
    pub fn make_move_mut(&mut self, m: Move);           // mutable for analysis
    pub fn unmake_move_mut(&mut self, m: Move, saved: SavedState);
    pub fn is_check(&self) -> bool;
    pub fn is_checkmate(&self) -> bool;
    pub fn is_stalemate(&self) -> bool;
    pub fn is_draw(&self) -> bool;                      // 50-move, repetition, insufficient material
    pub fn perft(&self, depth: u8) -> u64;              // correctness verification
}
```

### Correctness Verification

`perft` tests against published reference values (e.g. `perft(5)` from startpos = 4,865,609). All standard positions (Kiwipete, CPW positions) must pass before AI work begins.

---

## Sub-project 2: AI Engine

### Search (`search.rs`)

**Algorithm stack** (each layer builds on the previous):

| Layer | Technique | Effect |
|---|---|---|
| 1 | Negamax | base recursive search |
| 2 | Alpha-Beta pruning | cuts ~50% of tree |
| 3 | Iterative Deepening | always has a best move, enables time management |
| 4 | Transposition Table | avoids re-searching repeated positions |
| 5 | Move Ordering | maximizes alpha-beta cutoffs |
| 6 | Null Move Pruning | cuts passive branches |
| 7 | Late Move Reduction | reduces depth for low-ranked moves |
| 8 | Quiescence Search | resolves captures at leaf nodes, avoids horizon effect |

**Move ordering priority:**
1. TT best move (from previous iteration)
2. Captures ordered by MVV-LVA (Most Valuable Victim – Least Valuable Attacker)
3. Killer moves (two non-capture moves that caused beta cutoffs at same depth)
4. History heuristic (moves that improved alpha, accumulated over search)

**Transposition Table entry:**
```rust
struct TTEntry {
    hash: u64,
    depth: u8,
    score: i32,
    flag: TTFlag,    // Exact | LowerBound | UpperBound
    best_move: Move,
}
```

### Evaluation (`eval.rs`)

Returns centipawns from side-to-move perspective. Components:

| Component | Description |
|---|---|
| **Material** | P=100, N=320, B=330, R=500, Q=900 |
| **PST** | Per-piece tables of 64 values, interpolated between opening and endgame phase |
| **Mobility** | Count of legal moves (own − opponent) |
| **King safety** | Penalty for missing pawn shield, bonus for completed castling |
| **Center control** | Bonus for pieces/pawns attacking e4/d4/e5/d5 |
| **Pawn structure** | Penalty: doubled, isolated, backward. Bonus: passed pawns |
| **Game phase** | Linear interpolation by remaining material (opening ↔ endgame PST) |

### Difficulty Levels

```rust
pub struct DifficultyConfig {
    pub max_depth: u8,
    pub time_limit_ms: u64,
    pub random_factor: f32,    // 0.0 = perfect, >0 = pick randomly among top-N moves
}
```

| Level | Max Depth | Time Limit | Random Factor |
|---|---|---|---|
| Principiante | 2 | 500ms | 0.25 |
| Fácil | 3 | 1s | 0.10 |
| Medio | 5 | 3s | 0.00 |
| Difícil | 7 | 5s | 0.00 |
| Pro | iterative deepening | 10s | 0.00 |

### Opening Book (`opening_book.rs`)

Reads Polyglot `.bin` format. Consulted first on every turn. Returns `Option<Move>`. Book file bundled as WASM asset (~5–30 MB depending on book depth).

### Tablebases

Syzygy format, 3+4+5 piece (~1 GB total). Served by backend.

```rust
pub enum SearchResult {
    BookMove(Move),
    TablebaseMove(Move, WDL),   // perfect play, WDL = Win/Draw/Loss
    EngineMove(Move, i32),      // centipawn score
}
```

Query flow: if `position.piece_count() <= 5`, frontend calls `GET /api/tablebase?fen=...` before running minimax. Tablebase result takes priority.

### WASM Threading

Search runs as a Bevy `AsyncComputeTask` (via `AsyncComputeTaskPool`) to avoid blocking the render thread. Communication pattern:

```
input.rs: player move made → emit RequestAIMove event
ai.rs: spawns AsyncComputeTask(search)
ai.rs: polls task each frame → when done, emits AIMoveReady(Move)
board.rs: consumes AIMoveReady → applies move, updates visuals
```

---

## Sub-project 3: Game Analyzer (Phase 1.5)

Lives in `chess-engine/src/analysis.rs`. Reuses the search engine directly.

### Types

```rust
pub struct GameRecord {
    pub moves: Vec<Move>,
    pub initial_fen: String,
    pub result: GameResult,
}

pub struct MoveAnalysis {
    pub fen_before: String,
    pub played_move: Move,
    pub best_move: Move,
    pub score_before: i32,
    pub score_after: i32,
    pub best_score: i32,
    pub eval_loss: i32,           // best_score − score_after (always ≥ 0)
    pub classification: MoveClass,
}

pub enum MoveClass {
    Brilliant,    // eval_loss ≈ 0, material sacrifice with positional gain
    Excellent,    // eval_loss == 0
    Good,         // eval_loss < 20cp
    Inaccuracy,   // 20–100cp
    Mistake,      // 100–300cp
    Blunder,      // > 300cp
}

pub struct GameSummary {
    pub accuracy_white: f32,          // 0–100
    pub accuracy_black: f32,
    pub blunders:     [u8; 2],
    pub mistakes:     [u8; 2],
    pub inaccuracies: [u8; 2],
    pub critical_positions: Vec<String>,  // FENs of positions with eval_loss > 100cp
}

pub struct GameReport {
    pub move_analyses: Vec<MoveAnalysis>,
    pub summary: GameSummary,
}
```

### Analysis Algorithm

For each move in the game:
1. `score_before` = `search(position, ANALYSIS_DEPTH)` (signed for side to move)
2. Apply played move
3. `score_after` = `-search(position, ANALYSIS_DEPTH)` (negated, now from opponent's POV)
4. `best_move`, `best_score` extracted from search at step 1
5. `eval_loss` = `best_score − score_after`
6. Classify by thresholds above

**Brilliant detection**: `eval_loss ≈ 0` AND the moved piece lands on a square attacked by opponent AND `full_eval − material_eval > 50cp` (the position is better than material suggests).

### Analysis Depth

- **Client-side (WASM)**: depth 10, runs immediately after game ends, shown to user within seconds
- **Backend**: depth 18, runs async in background, result stored in DB (Phase 2 foundation), updates UI when ready

---

## Backend (Phase 1 scope)

**Stack:** Axum 0.7, Tokio, `shakmaty-syzygy`, `tower-http` (CORS, tracing)

**Endpoint:**

```
GET /api/tablebase?fen={fen}

200: { "wdl": "Win"|"Draw"|"Loss"|"CursedWin"|"BlessedLoss", "dtz": 14, "best_move": "e2e4" }
400: invalid FEN or piece count > 5
503: tablebases not loaded
```

**Tablebases:** Syzygy 3+4+5 piece (~1 GB), mounted at `/tablebases` in Docker, loaded into memory at startup via `shakmaty-syzygy`.

**CORS:** accepts `chess.greenmountain.dev` and `localhost:8080`.

**Deploy:** new `backend` service in `docker-compose.vps.yml`. Nginx proxies `/api/*` → backend:3001.

---

## bevy-frontend Integration

### App States

```rust
#[derive(States, Default)]
enum AppState {
    #[default] MainMenu,
    PlayingClassic,   // existing code, zero changes
    PlayingGame,      // chess-engine powered
}
```

### Game Mode

```rust
pub enum GameMode {
    HumanVsAI { difficulty: DifficultyConfig, human_color: Color },
    HumanVsHuman,
}
```

### Turn Flow (HumanVsAI)

```
Click piece → query legal_moves() → highlight valid squares
Click destination → validate → make_move() → update piece transforms
  → if promotion: show piece selector
  → if AI turn: spawn AsyncComputeTask(search)
    → if piece_count ≤ 5: fetch /api/tablebase first
    → emit AIMoveReady(move) when done
  → check game status (checkmate / stalemate / draw)
```

### Visual Feedback

| State | Color |
|---|---|
| Selected piece | red |
| Legal move targets | translucent green |
| Last move (both squares) | yellow |
| King in check | pulsing red |

### Post-Game Analysis UI

After game ends, the analysis report (from client-side depth-10 analysis) is shown inline in the side panel — move list with colored classifications (Brilliant = cyan, Blunder = red, etc.). Deep backend analysis updates this view when available (Phase 2).

---

## Out of Scope for This Spec

- User accounts, auth, sessions (Phase 2 spec)
- Learning mode, AI profiles, error memory (Phase 2 spec)
- Multiplayer, tournaments, spectators (Phase 3 spec)
- Generative AI / NNUE / RL replacement for learning engine (Phase 4 spec)
- `LearningEngine` trait (designed in Phase 4 spec, stubbed in Phase 2)

---

## Success Criteria

- `perft(5)` from startpos = 4,865,609 (move generation correctness)
- All standard test positions (Kiwipete, etc.) pass perft
- Pro level responds in < 10s for any position
- Principiante makes intentional mistakes ~25% of the time
- Analyzer correctly classifies known blunders from famous games
- Classic mode unchanged and still functional
- Tablebase endpoint returns correct WDL for known K+Q vs K positions
