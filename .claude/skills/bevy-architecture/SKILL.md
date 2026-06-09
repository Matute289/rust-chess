---
name: bevy-architecture
description: Review or advise on Bevy ECS architecture decisions — plugin boundaries, system ordering, resource vs component choice, event patterns. Anchored to Bevy 0.14 API.
---

# Bevy Architecture — rust-chess

Reference for ECS design decisions in Bevy 0.14. Use when designing a new system, reviewing plugin structure, or debugging scheduling issues.

## Plugin Boundaries

Each logical subsystem is a Plugin. Current plugins:
- `BoardPlugin` (`src/board.rs`) — board geometry, square selection
- `PiecesPlugin` (`src/pieces.rs`) — piece entities, movement, capture
- `UIPlugin` (`src/ui.rs`) — text overlays, status display

**Rule:** A plugin owns the components it defines. If Plugin A needs to read Plugin B's component, that's fine. If A needs to *write* B's component, consider whether the responsibility belongs in B.

## Resource vs Component

| Use `Resource` when | Use `Component` when |
|---|---|
| Exactly one instance exists globally | Data is per-entity |
| Represents game-wide state (whose turn, selected piece) | Marks entity type (Piece, Square, Selected) |
| Shared across plugins | Scoped to one plugin's entities |

Example: `Turn` (whose turn it is) → Resource. `Piece { color, kind }` → Component.

## System Scheduling (Bevy 0.14)

```rust
// Correct: explicit ordering
app.add_systems(Update, (
    handle_selection,
    validate_move.after(handle_selection),
    execute_move.after(validate_move),
));

// Wrong: implicit ordering (non-deterministic)
app.add_systems(Update, handle_selection);
app.add_systems(Update, validate_move);
```

**Startup vs Update:**
- Entity spawning → `Startup` schedule
- Per-frame logic → `Update` schedule
- One-shot after condition → `run_if(condition_once)` or events

## Events vs Queries

| Use Events when | Use Queries when |
|---|---|
| Something happened that multiple systems need to react to | Reading/writing entity state |
| Decoupling producer from consumer | The system owns the data it reads |
| Move executed, piece captured, game over | Iterating all pieces, updating positions |

```rust
// Event definition
#[derive(Event)]
struct PieceMoved { from: (u8, u8), to: (u8, u8) }

// Sender
fn execute_move(mut ev: EventWriter<PieceMoved>) {
    ev.send(PieceMoved { from, to });
}

// Receiver (can be in different plugin)
fn on_piece_moved(mut ev: EventReader<PieceMoved>) {
    for event in ev.read() { ... }
}
```

## Bevy 0.14 API Notes

- `Color::rgb()` is deprecated → use `Color::srgb()` or `Color::linear_rgb()`
- `Commands::spawn()` returns `EntityCommands` — chain `.insert()` or use tuple bundles
- `Query::get()` returns `Result` — always handle with `if let Ok(...)`, never `.unwrap()`
- Asset loading: `asset_server.load("path")` is relative to `assets/` directory
- `Transform::from_matrix()` is available for complex camera setup (as used in `setup()`)

## Anti-Patterns to Flag

- Mutating `World` directly outside of systems
- Using `static mut` for game state — use `Resource` instead
- Spawning entities in `Update` without a condition — unbounded growth
- `query.single()` when there might be 0 or 2+ entities — use `query.get_single()` and handle `Err`
