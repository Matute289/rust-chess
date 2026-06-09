---
name: responsive-architecture
description: Review or advise on multi-device responsive strategy — canvas scaling, viewport, touch, orientation, tablet/iPad breakpoints. Reference for all responsive work.
---

# Responsive Architecture — rust-chess

Reference for the multi-device strategy. See also: `docs/superpowers/specs/2026-06-09-responsive-mobile-design.md`.

## Canvas Dimensions

The Bevy canvas is fixed at **1200×1000px** (set in `WindowPlugin`). The web layer scales it to fit the viewport without changing the Bevy resolution.

## Scaling Strategy

```
viewport size → fitCanvas() → CSS transform scale
```

```js
function fitCanvas() {
  const scaleX = window.innerWidth  / canvas.offsetWidth;   // canvas.offsetWidth = 1200
  const scaleY = window.innerHeight / canvas.offsetHeight;  // canvas.offsetHeight = 1000
  let factor = Math.min(scaleX, scaleY);                    // fit, preserve aspect ratio
  if (!isTouchDevice) factor = Math.min(factor, 1.0);       // desktop: no upscale
  canvas.style.transform = `translate(-50%, -50%) scale(${factor})`;
}
```

**Desktop cap:** 1.0x (1200px canvas looks good at 1:1, no upscale needed)
**Mobile cap:** none (downscale only — touch devices get full viewport fill)

## Orientation Strategy

| Device | Portrait | Landscape |
|---|---|---|
| Desktop | N/A (always landscape) | Game renders at scale ≤ 1.0 |
| Phone (touch) | Show rotate-device overlay | Game renders, scaled down |
| Tablet/iPad | Game may be playable at small scale | Game renders comfortably |

**Portrait threshold:** `window.innerWidth <= window.innerHeight`

For tablet: evaluate whether the overlay threshold should be narrower (tablets can play in portrait at >600px width). Current implementation uses the same touch+portrait check for all touch devices.

## Touch Handling

Bevy's `bevy_mod_picking` with raycast backend handles pointer events including touch — no manual touch→click conversion needed in JS.

**CSS requirements:**
```css
html, body, #canvas { touch-action: none; }  /* prevent browser scroll/zoom on touch */
body { user-select: none; -webkit-user-select: none; }  /* prevent text selection on tap */
```

## Viewport Meta

```html
<meta name="viewport" content="width=device-width, initial-scale=1, user-scalable=no">
```

`user-scalable=no` prevents pinch-zoom which would break the scaling model.

## iOS Safari Quirks

- `100vh` includes the browser chrome bar → use `height: 100%` on html/body, not `min-height: 100vh`
- `AudioContext` requires user gesture → expected warning on load, no fix needed
- Safe area insets: if canvas gets clipped on notched phones, add `padding: env(safe-area-inset-*)` to body

## Device Breakpoints for Future Features

| Device class | Width range | Expected scale factor |
|---|---|---|
| Phone portrait | 375–430px | ~0.31–0.36x (show overlay) |
| Phone landscape | 667–932px | ~0.55–0.78x |
| Tablet portrait | 768–834px | ~0.64–0.69x |
| Tablet landscape | 1024–1366px | ~0.85–1.0x |
| Desktop | 1200px+ | 1.0x (capped) |
