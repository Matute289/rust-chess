---
name: review-board
description: Code review for src/board.rs — square spawning, selection system, highlighting, ECS component design. Use before committing board logic changes.
---

# Review: Board — rust-chess

## File to Review
`src/board.rs`

## Checklist

### ECS Design
- [ ] Components are data-only structs with no logic — logic lives in systems
- [ ] Each component has exactly one clear responsibility
- [ ] No `World` access outside of systems — no `.get_resource_unchecked`

### System Ordering
- [ ] Selection system runs before move-validation system (check `.before()`/`.after()` chains)
- [ ] No two systems mutate the same component without explicit ordering
- [ ] Startup systems (board spawn) are in `Startup` schedule, not `Update`

### Square Selection
- [ ] Selected square state uses a `Resource` or marker `Component`, not a global `static`
- [ ] Deselection happens correctly when clicking empty square or invalid move
- [ ] Highlight entities are despawned when selection changes (no ghost highlights)

### Common Bevy 0.14 Pitfalls
- [ ] No use of deprecated `Color::rgb()` → should be `Color::srgb()` or `Color::linear_rgb()`
- [ ] PbrBundle / StandardMaterial handles are cloned via `assets.add()`, not stored as raw handles
- [ ] No `.unwrap()` on `Query::get()` results — use `if let Ok(...)`

### Performance
- [ ] Board squares spawned once at startup, not re-spawned every frame
- [ ] No unbounded entity spawning in Update systems
