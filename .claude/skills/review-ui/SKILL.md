---
name: review-ui
description: Code review for src/ui.rs — Bevy UI nodes, text display, layout. Use before committing UI changes.
---

# Review: UI — rust-chess

## File to Review
`src/ui.rs`

## Checklist

### Bevy UI (0.14 API)
- [ ] Text uses `TextBundle` with `TextSection` — not raw `Text2dBundle` (different coordinate space)
- [ ] UI nodes use `Val::Px` or `Val::Percent` — no hardcoded pixel values for font sizes
- [ ] Font loaded via `asset_server.load("fonts/FiraSans-Bold.ttf")` — path relative to `assets/`
- [ ] Root UI node has `Style { width: Val::Percent(100.), height: Val::Percent(100.), ..default() }`

### Reactivity
- [ ] Text updates use `Query<&mut Text>` in an Update system triggered by state change
- [ ] No text re-creation every frame — only when turn/state actually changes
- [ ] Change detection: system only runs when relevant resource or component changes (`.run_if` or `Changed<>` filter)

### Responsive Concerns
- [ ] Font sizes specified in `px` will not scale with canvas — flag any that should adapt
- [ ] UI overlay elements positioned relative to viewport, not to world coordinates
- [ ] No fixed-pixel margins that assume 1200×1000 resolution

### Content
- [ ] "Next move" text accurately reflects whose turn it is
- [ ] Check/checkmate state surfaced to player with clear message
