# Rust Chess — Responsive & Mobile Design
Date: 2026-06-09

## Goal

Make the chess game fully playable across all devices and orientations: desktop, tablet, phone portrait, phone landscape. Same treatment as simple-flappy-dragon v1.0.0, adapted for a Bevy-based 3D chess game.

## Reference

This spec mirrors `simple-flappy-dragon/docs/superpowers/specs/2026-06-08-responsive-mobile-design.md`.
The flappy dragon implementation (branch `browser`) is the canonical reference for code patterns.

---

## Key Technical Differences vs Flappy Dragon

| | Flappy Dragon | Rust Chess |
|---|---|---|
| Engine | bracket-lib 0.8 | Bevy 0.14 |
| Canvas created by | Rust (BTermBuilder) | Bevy (WindowPlugin) |
| Canvas size | 664×400 (83×50 cells) | 1200×1000 |
| Game loop in JS | `await init()` returns, `start_game()` starts loop | `await init()` returns after Bevy setup; loop runs via rAF |
| WASM exports needed | `start_game`, `get_player_y`, `flap`, `begin_play`, `pause_game`, `resume_game` | **None** — no new exports needed |
| Touch picking | Manual touchstart → `flap()` | bevy_mod_picking (raycast backend) handles pointer events including touch |
| Game modes | classic / new | **Single mode only** |
| Pause feature | Yes (action game) | **No** — turn-based, no pause needed |
| Ready-to-start screen | Yes | **No** — board shows immediately |
| Pause button | Yes (mobile) | **No** |
| Dragon SVG overlay | Yes (new mode) | **No** |

---

## Scope

| Feature | Files |
|---|---|
| Canvas scaling | `web/app.js` (new), `web/style.css` (new), `web/index.html` |
| Touch reliability | `web/style.css`, `web/index.html` |
| Canvas selector wiring | `src/lib.rs` |
| Loading overlay | `web/index.html`, `web/style.css` |
| Rotate-device hint (portrait) | `web/index.html`, `web/style.css`, `web/app.js` |

---

## 1. Rust Change — Canvas Selector

Bevy's `WindowPlugin` needs to know which HTML canvas element to use. Add `canvas` field:

```rust
// src/lib.rs — inside run_app(), WindowPlugin setup:
primary_window: Some(Window {
    title: "Ajedrez".to_string(),
    resolution: (1200., 1000.).into(),
    canvas: Some("#canvas".to_string()), // ← add this
    ..default()
}),
```

This tells Bevy to render into `<canvas id="canvas">` which we create in the HTML.

---

## 2. HTML — `web/index.html`

Replace the current inline everything with a proper structure:

```html
<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">
  <meta name="apple-mobile-web-app-capable" content="yes">
  <title>Ajedrez</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <canvas id="canvas"></canvas>

  <!-- Loading overlay — shown until Bevy renders first frame -->
  <div id="loading-overlay">
    <div class="loading-box">
      <div class="loading-title">♟ AJEDREZ</div>
      <div class="loading-hint">Cargando...</div>
    </div>
  </div>

  <!-- Portrait hint — shown when phone is in portrait mode -->
  <div id="portrait-hint" style="display:none">
    <div class="portrait-box">
      <div class="portrait-icon">🔄</div>
      <div class="portrait-text">Rotá el dispositivo<br>para jugar</div>
    </div>
  </div>

  <script type="module" src="app.js"></script>
</body>
</html>
```

---

## 3. CSS — `web/style.css` (new file)

```css
*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

html {
  height: 100%;
  overflow: hidden;
  touch-action: none;
}

body {
  background: #1a1a2e;
  overflow: hidden;
  height: 100%;
  touch-action: none;
  user-select: none;
  -webkit-user-select: none;
}

#canvas {
  display: block;
  position: fixed;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  z-index: 0;
  will-change: transform;
  touch-action: none;
}

/* Loading overlay */
#loading-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #1a1a2e;
}
.loading-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  font-family: monospace;
  color: #eee;
}
.loading-title {
  font-size: clamp(24px, 6vw, 40px);
  font-weight: 900;
  color: #ffd700;
  letter-spacing: 4px;
}
.loading-hint {
  font-size: 14px;
  color: #888;
  letter-spacing: 2px;
  animation: pulse 1.5s ease-in-out infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 0.6; }
  50%       { opacity: 1; }
}

/* Portrait orientation hint */
#portrait-hint {
  position: fixed;
  inset: 0;
  z-index: 200;
  display: none;
  align-items: center;
  justify-content: center;
  background: #1a1a2e;
  flex-direction: column;
}
.portrait-box {
  text-align: center;
  font-family: monospace;
  color: #eee;
}
.portrait-icon {
  font-size: 48px;
  margin-bottom: 16px;
  animation: spin 2s ease-in-out infinite;
}
.portrait-text {
  font-size: clamp(16px, 4vw, 22px);
  color: #aaa;
  line-height: 1.6;
  letter-spacing: 1px;
}
@keyframes spin {
  0%   { transform: rotate(0deg); }
  40%  { transform: rotate(90deg); }
  100% { transform: rotate(90deg); }
}
```

---

## 4. JS — `web/app.js` (new file)

```js
import init from './pkg/bevy_chess.js';

const isTouchDevice = 'ontouchstart' in window || navigator.maxTouchPoints > 0;
let isLandscape = window.innerWidth > window.innerHeight;

function fitCanvas() {
  const canvas = document.getElementById('canvas');
  if (!canvas.offsetWidth) return;
  const scaleX = window.innerWidth  / canvas.offsetWidth;
  const scaleY = window.innerHeight / canvas.offsetHeight;
  let factor = Math.min(scaleX, scaleY);
  if (!isTouchDevice) factor = Math.min(factor, 1.0); // desktop: don't upscale
  canvas.style.transform = `translate(-50%, -50%) scale(${factor})`;
}

function updatePortraitHint() {
  const nowLandscape = window.innerWidth > window.innerHeight;
  const hint = document.getElementById('portrait-hint');
  if (isTouchDevice && !nowLandscape) {
    hint.style.display = 'flex';
  } else {
    hint.style.display = 'none';
  }
}

window.addEventListener('resize', () => {
  isLandscape = window.innerWidth > window.innerHeight;
  fitCanvas();
  updatePortraitHint();
});

// Boot
await init();
document.getElementById('loading-overlay').style.display = 'none';
fitCanvas();
updatePortraitHint();
```

**Note on desktop upscale cap:** Flappy dragon caps at 1.5x. For chess, cap at 1.0x (no upscale on desktop — 1200px canvas already looks good at 1:1).

---

## 5. Portrait Mode Decision

The chess board is 1200×1000. On iPhone 16 Pro Max portrait (393px wide):
- scale = 393/1200 = 0.328
- Canvas appears 393×328px — very small, pieces hard to tap

**Decision:** Show a "rotate device" overlay when `isTouchDevice && portrait`. The canvas still scales underneath it. When user rotates to landscape, the hint disappears and the game is playable.

---

## Out of Scope

- Ready-to-start screen (not needed for chess)
- Pause / resume (turn-based, no action to pause)
- Pause button (not needed)
- Multiplayer or online mode
- Piece drag-and-drop (existing click-to-select-then-click-to-move stays as-is)

---

## Build & Test Process

Same as flappy dragon (from CLAUDE.md):

```bash
# Build WASM
wasm-pack build --target web --out-dir web/pkg

# Serve locally
cd web && python3 -m http.server 8090

# Test at http://localhost:8090
# Then commit + push to browser branch
```

## Deploy

Push to `browser` branch → GitHub Actions CI → Docker build → deploys to `chess.greenmountain.dev`.
