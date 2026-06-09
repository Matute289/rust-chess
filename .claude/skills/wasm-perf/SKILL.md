---
name: wasm-perf
description: Measure WASM bundle size, build time, and JS heap. Use before any deploy that adds dependencies or significant Rust code.
---

# WASM Performance — rust-chess

## Bundle Size Check

```bash
ls -lh web/pkg/bevy_chess_bg.wasm
du -sh web/pkg/
```

**Baseline (Bevy 0.14 + bevy_mod_picking, release):**
- `.wasm` file: ~15–25 MB before wasm-opt, ~8–15 MB after
- Total `pkg/`: ~20–30 MB

If size increases by >20% with a change, investigate with:

```bash
# Re-build with size info
wasm-pack build --target web --out-dir web/pkg -- --release
```

## Build Time

```bash
time wasm-pack build --target web --out-dir web/pkg
```

- Cold build (no cache): ~3–8 minutes
- Incremental (only changed files): ~30–90 seconds

## JS Heap at Steady State

Open browser DevTools → Memory tab → Take Heap Snapshot after board loads.

Typical usage: ~50–150 MB JS heap for a Bevy WASM app at idle.

Red flags:
- Heap grows continuously → memory leak (likely orphaned Bevy entities or event queues not draining)
- Heap > 500 MB → investigate large asset loading or texture duplication

## wasm-opt Results

wasm-pack runs `wasm-opt -Os` automatically. To check optimization level:
```bash
grep "wasm-opt" $(which wasm-pack) 2>/dev/null || wasm-pack --version
```

## Load Time Budget

| Phase | Target | How to measure |
|---|---|---|
| WASM fetch | < 3s on WiFi | Network tab: `bevy_chess_bg.wasm` request |
| WASM instantiate | < 2s | Console: time between fetch end and first Bevy log |
| Asset loading (GLB) | < 2s | Network tab: `pieces.glb` request |
| Total to board visible | < 8s on WiFi | Loading overlay visible duration |

## Regression Check

```bash
git stash
wasm-pack build --target web --out-dir web/pkg
ls -lh web/pkg/bevy_chess_bg.wasm  # baseline size
git stash pop
wasm-pack build --target web --out-dir web/pkg
ls -lh web/pkg/bevy_chess_bg.wasm  # new size
```
