---
name: mobile-debug
description: Debug mobile-specific failures — touch not working, portrait hint broken, canvas scaling wrong, iOS Safari quirks. Use when the game works on desktop but fails on mobile.
---

# Mobile Debug — rust-chess

## Triage Tree

```
Works on desktop, broken on mobile
│
├─ Touch events not registering (clicking pieces does nothing)
│   ├─ Is touch-action: none on canvas? → DevTools → Computed styles on #canvas
│   ├─ Is the canvas z-index blocking the picking backend? (should be z-index: 0)
│   └─ bevy_mod_picking raycast backend handles touch natively — check it's in DefaultPickingPlugins
│
├─ Canvas not scaling to viewport
│   ├─ Open DevTools console → run fitCanvas() manually
│   │   window.dispatchEvent(new Event('resize'))
│   ├─ Check canvas.offsetWidth (should be 1200) — if 0, canvas not yet rendered
│   └─ Check transform: document.getElementById('canvas').style.transform
│
├─ Portrait hint not showing / hiding
│   ├─ Check isTouchDevice: 'ontouchstart' in window || navigator.maxTouchPoints > 0
│   │   (DevTools device emulation sets maxTouchPoints > 0)
│   ├─ Check isLandscape: window.innerWidth > window.innerHeight
│   └─ Manually test: document.getElementById('portrait-hint').style.display = 'flex'
│
├─ Canvas overflows viewport (scroll appears)
│   ├─ Check: html, body have overflow: hidden
│   ├─ Check: body height is 100% not min-height: 100vh (iOS Safari bug)
│   └─ Check: #canvas is position: fixed (not relative/absolute)
│
└─ iOS Safari specific
    ├─ White flash on load → background color of html/body not set (#1a1a2e)
    ├─ Viewport height wrong (includes browser chrome) → use height: 100%, not 100vh
    └─ WebGL context lost after backgrounding app → user needs to reload (known limitation)
```

## Remote Debug on Real Device

### Android (Chrome)
1. Enable USB debugging on phone
2. Connect via USB
3. Chrome on desktop: `chrome://inspect` → find device
4. Inspect → Console shows device logs in real time

### iOS (Safari)
1. iPhone: Settings → Safari → Advanced → Web Inspector → ON
2. Mac: Safari → Develop → [device name] → [page]
3. Console and Elements available remotely

## Quick Console Tests (paste in DevTools)

```js
// Check scaling state
const c = document.getElementById('canvas');
console.log('Canvas transform:', c.style.transform);
console.log('Canvas offset size:', c.offsetWidth, '×', c.offsetHeight);
console.log('Window size:', window.innerWidth, '×', window.innerHeight);
console.log('Is touch device:', 'ontouchstart' in window || navigator.maxTouchPoints > 0);
console.log('Is landscape:', window.innerWidth > window.innerHeight);
console.log('Portrait hint display:', document.getElementById('portrait-hint').style.display);
```

## Emulating Specific Devices Locally

In Chrome DevTools device toolbar:
- Set exact dimensions for the target device
- Toggle "Touch" to simulate touch events
- Rotate: click the rotate icon to swap width/height
- Network: throttle to "Slow 4G" to test load time on mobile networks
