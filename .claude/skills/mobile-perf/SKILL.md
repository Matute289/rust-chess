---
name: mobile-perf
description: Check frame rate, touch latency, and canvas scaling quality on mobile viewport sizes. Use after responsive changes or before mobile-targeted releases.
---

# Mobile Performance — rust-chess

## Browser DevTools Emulation

1. Open http://localhost:8091
2. DevTools (F12) → Toggle device toolbar (Ctrl+Shift+M)
3. Select a device preset or set custom dimensions:

| Device | Width × Height | Expected scale factor |
|---|---|---|
| iPhone 16 Pro Max | 430 × 932 | ~0.36x portrait, ~0.78x landscape |
| iPhone SE | 375 × 667 | ~0.31x portrait, ~0.56x landscape |
| iPad Air | 820 × 1180 | ~0.68x portrait, ~0.85x landscape |
| iPad Pro 12.9" | 1024 × 1366 | ~0.85x portrait, ~1.0x landscape |

## Frame Rate Check

1. DevTools → Performance tab → Record 10 seconds of gameplay
2. Check: is the frame timeline consistently at 60fps?
3. Red flag: frame drops below 30fps → likely a Bevy system running too frequently

Quick check via console:
```js
// Paste in console after board loads
let frames = 0, last = performance.now();
const id = setInterval(() => {
  const now = performance.now();
  console.log(`FPS: ${(frames / ((now - last) / 1000)).toFixed(1)}`);
  frames = 0; last = now;
}, 1000);
requestAnimationFrame(function count() { frames++; requestAnimationFrame(count); });
// Stop: clearInterval(id)
```
Target: 55–60 fps steady state.

## Canvas Scaling Quality

After resizing to mobile viewport:
- Canvas should be centered with no overflow
- Pieces should be visible (not too small to tap — minimum ~30px rendered size)
- Board grid should be sharp (CSS transform scale, not canvas resize — should remain crisp)

Check:
```js
const c = document.getElementById('canvas');
const t = c.style.transform;
console.log('Transform:', t);
console.log('Rendered size:', c.offsetWidth * parseFloat(t.match(/scale\(([\d.]+)\)/)[1]), 'px wide');
```

## Touch Latency

- Tap a piece → should highlight within 1 frame (~16ms)
- If highlight is delayed: check that `bevy_mod_picking` touch events are routing to Bevy
- DevTools → Performance → look for long tasks blocking the main thread after touch event

## Portrait Overlay

On portrait touch device:
- `#portrait-hint` should be `display: flex` (overlay visible)
- `#canvas` should still be present underneath (scaled)
- Rotating to landscape: overlay hides within one `resize` event

## Known Mobile Limitations

- WebGL 2.0 SSAO not available on most mobile GPUs → expected, logged as WARN by Bevy
- 3D shadows may render differently on mobile GPU — visual check required
- iOS Safari: test separately from Chrome emulation (Safari has different WebGL behavior)
