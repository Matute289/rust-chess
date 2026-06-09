---
name: review-web
description: Code review for the web layer — web/index.html, web/style.css, web/app.js. Use before committing frontend changes or after any responsive/mobile work.
---

# Review: Web Layer — rust-chess

## Files to Review
- `web/index.html`
- `web/style.css`
- `web/app.js`

## HTML Checklist
- [ ] `<meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">` present
- [ ] `<canvas id="canvas">` present — Bevy targets this via `WindowPlugin.canvas = Some("#canvas")`
- [ ] Loading overlay (`#loading-overlay`) and portrait hint (`#portrait-hint`) both present
- [ ] `<script type="module" src="app.js">` — must be `type="module"` for ES imports
- [ ] No inline styles — all styling in `style.css`

## CSS Checklist
- [ ] `html, body` have `overflow: hidden` and `touch-action: none` — prevents scroll on mobile
- [ ] `#canvas` is `position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%)` — centered
- [ ] `#portrait-hint` has `display: none` default and `display: flex` when shown via JS
- [ ] No `min-height: 100vh` on body — breaks iOS Safari (use `height: 100%` instead)
- [ ] `user-select: none` and `-webkit-user-select: none` on body — prevents text selection on tap

## JS Checklist
- [ ] `await init()` wrapped in try/catch that swallows the wasm-bindgen control-flow exception
- [ ] `fitCanvas()` called on init AND on every `resize` event
- [ ] `updatePortraitHint()` called on init AND on every `resize` event
- [ ] Desktop upscale capped at `1.0` — `Math.min(factor, 1.0)` when `!isTouchDevice`
- [ ] Portrait hint shows when `isTouchDevice && !isLandscape`
- [ ] Import path: `import init from './pkg/bevy_chess.js'` — must match wasm-pack output name

## Asset Path
- [ ] `web/assets` symlink points to `../assets` — verify with `ls -la web/assets`
- [ ] No hardcoded `/assets/` absolute paths in JS (use relative)
