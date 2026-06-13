# Phase 2 — Accounts, Stats & PvL Learning Mode

## Vision

Phase 1 built a fully functional chess game with a strong AI.  
Phase 2 adds **identity** (who is playing) and **growth** (both the player and the AI learn over time).

The two pillars:

1. **Auth + Accounts** — users log in via OAuth, and every game they play is persisted. Stats, ELO, and an error profile accumulate across sessions.
2. **PvL (Player vs Learning)** — a mode focused on learning, with three selectable sub-modes:
   - **vs IA Adaptativa** — the AI stores its own loss history and permanently improves. It never regresses.
   - **Con Sugerencias** — the player can request a move hint at any time, with an explanation of why the suggested move is better.
   - **Currículo de Lecciones** — structured exercises (openings, tactics, endgames) with progress tracked per user.

---

## Implementation Progress

| Sub-project | Status | Notes |
|---|---|---|
| 1 — Backend + Auth | ✅ Done | chess-server crate, OAuth Google/GitHub/Discord, JWT, chess-db. /api/me called post-login to populate display_name/elo. User menu chip (top-right, persiste en PvLHub) con logout. PNG icons en assets/icons/. |
| 2 — Game Persistence + ELO | ✅ Done | persist_on_game_end en persistence.rs. POST /api/games con payload completo (mode, result, accuracy, blunders/mistakes/inaccuracies, moves_uci, summary). ELO calculado con K=32 en el backend. |
| 3 — User Dashboard (stats screen) | ✅ Done (en PvLHub) | Stats integradas en el PvL Hub: ELO con tooltip, win/loss/draw totals, accuracy promedio, blunders/errores/inexactos totales, tabla de últimas 5 partidas con scrolling. Narrativa coloquial en español guardada por partida y visible con botón "ver". |
| 4 — PvL: vs IA Adaptativa | 🔲 Pending (3°) | |
| 5 — PvL: Con Sugerencias | 🔲 Pending (1° — siguiente) | |
| 6 — PvL: Currículo de Lecciones | 🔲 Pending (2°) | |

---

## Architecture Decisions

### Backend choice

**Servidor propio — Rust (Axum + PostgreSQL)**

El usuario dispone de servidor propio con almacenamiento y capacidad de usuarios sin límites externos.

- **Framework**: Axum (async, ergonómico, mismo ecosistema Rust que el frontend)
- **DB**: PostgreSQL (robusto, relacional, soporta JSONB para `learned_positions`)
- **Auth**: OAuth Authorization Code Flow — el backend actúa como intermediario:
  1. Frontend redirige al provider (Google/GitHub/Discord)
  2. Provider redirige de vuelta al backend con un `code`
  3. Backend intercambia `code` por tokens, crea/actualiza el usuario, genera un JWT propio
  4. Frontend recibe el JWT y lo guarda en `localStorage`
- **API**: REST JSON sobre HTTPS
- **Ventajas**: control total, sin límites de usuarios ni storage, lógica custom (IA adaptativa) en el mismo codebase Rust, sin vendor lock-in
- **Apple Sign In**: diferido — pendiente confirmación Apple Developer account ($99/año). Una vez aprobada, agregar como Sub-project 1b.

### WASM ↔ Backend communication

The WASM binary calls the backend via `wasm-bindgen-futures` + `web-sys`/`reqwest` (WASM target).  
All API calls are async. Bevy uses a channel pattern: spawn a JS Future, resolve into a Bevy event.

### AI learning persistence

The adaptive AI's "learned state" is a small JSON blob stored per AI profile in the backend (or locally in `localStorage` as a fallback). It is never reset between games.

---

## Data Model

### `User`
```
id            UUID PK
email         TEXT
display_name  TEXT
elo           INTEGER  (default 800)
created_at    TIMESTAMP
```

### `Game`
```
id            UUID PK
user_id       UUID FK → User
mode          TEXT  ('pvp' | 'pvc' | 'pvl')
result        TEXT  ('win' | 'loss' | 'draw')
played_at     TIMESTAMP
pgn           TEXT  (full move list)
accuracy_white  FLOAT
accuracy_black  FLOAT
blunders        INTEGER[]  (index 0=white, 1=black)
mistakes        INTEGER[]
inaccuracies    INTEGER[]
```

### `ErrorProfile`
```
user_id           UUID FK → User (1:1)
tactic_errors     INTEGER   (blunders/mistakes in tactical positions)
endgame_errors    INTEGER
opening_errors    INTEGER
time_pressure_errors INTEGER  (moves made with < 10s on timer)
last_updated      TIMESTAMP
```

### `AiProfile`  (for PvL adaptive AI)
```
user_id           UUID FK → User (1:1)
elo_estimate      INTEGER   (AI's current estimated level — never decreases)
loss_count        INTEGER   (total losses since creation)
learned_positions JSONB     (compact table: position_key → eval adjustment)
last_updated      TIMESTAMP
```

### `LessonProgress`
```
user_id       UUID FK → User
lesson_id     TEXT
completed_at  TIMESTAMP
score         INTEGER  (0-100)
```

---

## Sub-project 1: Backend + Auth

**Goal:** Real OAuth login replacing the stub. Users can log in and get a session token used by all subsequent API calls.

### Scope

**Backend (nuevo repo o workspace crate `chess-server`)**
- Axum HTTP server con rutas de auth y API
- PostgreSQL schema: migraciones con `sqlx migrate`
- OAuth Authorization Code Flow para Google, GitHub, Discord
- Endpoint `GET /auth/{provider}/login` → redirige al provider
- Endpoint `GET /auth/{provider}/callback` → intercambia code, crea usuario, devuelve JWT propio
- JWT firmado con clave privada (RS256 o HS256), expiración 30 días
- Middleware de autenticación para rutas protegidas (`/api/*`)
- **Apple Sign In**: diferido — pendiente confirmación Apple Developer account ($99/año). Agregar como Sub-project 1b.

**Frontend (WASM)**
- `UserSession { user_id: Option<Uuid>, jwt: Option<String>, display_name: Option<String> }` — Bevy resource
- OAuth flow: Frontend abre `{backend}/auth/google/login` en nueva pestaña; el callback del backend redirige de vuelta a la app con `?jwt=...`; JS intercepta el parámetro y lo pasa al WASM via `wasm_bindgen`
- JWT guardado en `localStorage` para persistir entre sesiones
- Home screen: "Iniciar sesión" botón — PvL requiere auth; PvP/PvC opcionales

### Files
- New: `chess-server/` — crate Axum con `main.rs`, `auth.rs`, `api.rs`, `db.rs`, `migrations/`
- New: `src/auth.rs` — `AuthPlugin`, `UserSession`, helpers de redirect y JWT parse
- Modify: `src/home.rs` — `BtnOAuth` lanza redirect real en vez del stub
- Modify: `src/lib.rs` — add `AuthPlugin`

---

## Sub-project 2: Game Persistence + ELO

**Goal:** After every completed game (checkmate, stalemate, timeout), persist the result to the backend and update ELO.

### Scope
- On `GameStatus::Checkmate` or `Stalemate`, if user is authenticated, POST to `/games`
- Payload: mode, result, PGN (built from `GameHistory`), `GameReport` accuracy/error data
- ELO update: simple Elo formula, K=32, opponent ELO = `difficulty.elo_estimate()`
  - Win: +K × (1 − expected); Loss: +K × (0 − expected)
  - Update user ELO in Supabase; store in `UserSession`
- Error profile: after each game, increment the relevant counters based on `GameReport`
  - Tactic errors: blunders/mistakes where engine eval before was ≥ +1.0 (winning position blown)
  - Endgame errors: blunders in positions with ≤ 6 pieces
  - Opening errors: blunders in first 15 moves

### Files
- New: `src/persistence.rs` — `PersistencePlugin`, `persist_game` system (fires on `GameStatusEvent`)
- New: `src/pgn.rs` — build PGN string from `GameHistory` moves
- Modify: `src/board.rs` — expose `GameHistory` as public

### ELO estimates by difficulty
| Difficulty | ELO estimate |
|---|---|
| Principiante | 400 |
| Fácil | 800 |
| Medio | 1200 |
| Difícil | 1600 |
| Pro | 2000 |

---

## Sub-project 3: User Dashboard

**Goal:** A new screen (accessible from the Home menu) showing the authenticated user's stats.

### Scope
- New `AppState::Dashboard` state
- Stats page shows:
  - ELO actual + sparkline of last 10 ELO changes
  - Win / Loss / Draw totals + % across all modes
  - Accuracy promedio (últimas 10 partidas)
  - Errores: blunders / mistakes / inaccuracies totals
  - Error profile radar: Táctica / Finales / Aperturas / Presión de tiempo
  - Last 5 games table (date, mode, result, accuracy)
- Home screen: "Mi perfil" button (only shown when logged in)

### Files
- New: `src/dashboard.rs` — `DashboardPlugin`, fetch + display stats
- Modify: `src/home.rs` — add "Mi perfil" button, navigate to Dashboard
- Modify: `src/state.rs` — add `AppState::Dashboard`

---

## Sub-project 4: PvL — vs IA Adaptativa

**Goal:** An AI opponent that permanently learns from each loss. Its ELO estimate only moves up, never down. Its learned knowledge is persisted per user in the backend.

### Learning algorithm
- After each loss, the AI runs a deeper analysis (depth+2) on the game's critical positions (positions where it made the losing mistake).
- For each critical position, store `(position_key → eval_bias)` in `AiProfile.learned_positions`. The bias nudges the AI away from moves it evaluated incorrectly.
- On each turn, before searching, apply stored biases to adjust move ordering for known-bad positions.
- ELO estimate: increases by `10 × (1 − expected)` after a loss (same formula as player ELO, inverted).

### In-game UI
- Profile card shown during PvL: "IA Nivel X" where X = `ai_profile.elo_estimate`
- After each AI loss: brief banner "¡La IA aprendió de esta derrota!"

### Files
- New: `src/adaptive_ai.rs` — `AdaptiveAiProfile`, bias application, post-game learning
- Modify: `src/ai.rs` — accept optional `AdaptiveAiProfile` to adjust move ordering
- Modify: `src/persistence.rs` — save updated AI profile after PvL games

---

## Sub-project 5: PvL — Con Sugerencias

**Goal:** During a PvL game, the player can tap a "Sugerir" button to see the engine's top move with a brief explanation.

### Scope
- "Sugerir" button visible during the player's turn in PvL mode
- On tap: run engine at depth 10 (same depth as analysis), return top move + classification
- Highlight the suggested move's origin and destination squares in blue
- Show explanation text below the board:
  - Move name (algebraic)
  - Classification: "Excelente", "Bueno", "Interesante"
  - Why: eval delta vs the player's last move (e.g. "Ganas +1.2 de ventaja")
  - If it's a tactic: "Fork", "Pin", "Skewer", "Discovered check", etc.
- Player can accept (play the suggested move) or dismiss and play their own
- Suggestions count tracked in `Game.suggestions_used` (stored, influences "assisted" flag on the game)

### Files
- New: `src/suggestion.rs` — `SuggestionPlugin`, `SuggestionState` resource, compute + display
- Modify: `src/ui.rs` — "Sugerir" button (only in PvL + player's turn)
- Modify: `src/board.rs` — highlight suggested squares

---

## Sub-project 6: PvL — Currículo de Lecciones

**Goal:** A structured set of exercises grouped into tracks (Aperturas, Táctica, Finales). Progress is saved per user.

### Tracks and content

**Aperturas (Opening track)**
- Lesson 1: Control del centro — e4, d4
- Lesson 2: Desarrollo de piezas — knight before bishop
- Lesson 3: Enroque temprano
- Lesson 4: Gambito de Rey — recognized positions

**Táctica (Tactics track)**
- Lesson 1: Jaque mate en 1
- Lesson 2: Horquilla de caballo
- Lesson 3: Clavada
- Lesson 4: Ensartada
- Lesson 5: Ataque descubierto

**Finales (Endgame track)**
- Lesson 1: Rey y Peón vs Rey
- Lesson 2: Rey y Torre vs Rey
- Lesson 3: Rey y Dama vs Rey
- Lesson 4: Oposición de reyes

### Lesson structure
Each lesson = 1 intro text (markdown) + 3-5 exercises (FEN position + correct move sequence).  
Exercises stored as JSON assets bundled with the WASM binary.

### Progress tracking
- `LessonProgress` rows written after each lesson completion
- Home screen for lessons shows track completion %
- Stars (1-3) based on score (correct moves / total moves)

### Files
- New: `src/lessons.rs` — `LessonsPlugin`, lesson state machine, exercise runner
- New: `assets/lessons/*.json` — lesson definitions (FEN + moves + explanations)
- Modify: `src/home.rs` — "Lecciones" entry point in PvL flow
- Modify: `src/state.rs` — add `AppState::Lessons`

---

## Phase 2 Dependency Order

```
Sub-project 1 (Auth)
    ↓
Sub-project 2 (Persistence + ELO)       ← requires Auth
    ↓
Sub-project 3 (Dashboard)               ← requires Persistence
Sub-project 4 (IA Adaptativa)           ← requires Persistence (AI profile)
Sub-project 5 (Con Sugerencias)         ← requires Auth (to log suggestion usage)
Sub-project 6 (Currículo)               ← requires Auth (to track progress)
```

Sub-projects 3–6 can be built in parallel once Sub-project 2 is done.

---

## Out of Scope for Phase 2

- Multiplayer, tournaments, spectators (Phase 3)
- NNUE / RL replacement for learning engine (Phase 4)
- Mobile push notifications
- Social features (friends, leaderboards) — Phase 3

---

## Success Criteria

- [ ] User can log in with Google, GitHub, or Discord
- [ ] Every completed game is persisted with full analysis data
- [ ] ELO updates correctly after each PvC/PvL game
- [ ] Stats dashboard shows accurate historical data
- [ ] PvL adaptive AI's ELO estimate increases after each loss and never resets
- [ ] "Sugerir" shows a move + explanation within 2 seconds
- [ ] At least one full lesson track (Táctica) is playable with progress saved
- [ ] All Phase 1 functionality unchanged (no regressions)
