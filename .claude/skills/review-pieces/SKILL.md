---
name: review-pieces
description: Code review for src/pieces.rs — piece spawning, movement logic, capture, rule validation. Use before committing piece or movement changes.
---

# Review: Pieces — rust-chess

## File to Review
`src/pieces.rs`

## Checklist

### Piece Spawning
- [ ] Each piece type loads its mesh once and clones the handle — not re-loading per piece
- [ ] GLB mesh paths follow `models/chess_kit/pieces.glb#MeshN/Primitive0` format
- [ ] White and black pieces use different materials loaded via `asset_server`

### Movement Validation
- [ ] Each piece type has its own movement function returning `Vec<(u8, u8)>` of valid targets
- [ ] Movement functions do not access `World` directly — they receive board state as parameters
- [ ] Bounds checking: all coordinates clamped to 0–7 before returning
- [ ] Blocking pieces: sliding pieces (rook, bishop, queen) stop at first occupied square

### Chess Rules — Edge Cases to Verify
- [ ] **Castling**: is it implemented? If not, is it a known placeholder?
- [ ] **En passant**: is it implemented? If not, is it a known placeholder?
- [ ] **Promotion**: what happens when a pawn reaches rank 8? Auto-queen or placeholder?
- [ ] **Check detection**: does the game prevent moves that leave own king in check?
- [ ] **Checkmate**: is end-of-game detected and surfaced to the player?

### ECS Patterns
- [ ] Capture: captured piece entity is despawned, not just hidden
- [ ] No raw `Transform` mutation for movement — uses the established move-piece mechanism
- [ ] Turn tracking uses a `Resource`, not component state on pieces

### Common Bevy 0.14 Pitfalls
- [ ] `asset_server.load()` paths are relative to `assets/` directory — no leading `/`
- [ ] Mesh handles stored in a `Resource` after first load, not re-loaded per frame
