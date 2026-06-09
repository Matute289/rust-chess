---
name: bevy-debug
description: Systematic debugging for WASM/Bevy console errors — asset 404s, WASM panics, system scheduling issues, wasm-bindgen exceptions. Use when the game fails to load or behaves incorrectly.
---

# Bevy Debug — rust-chess

## Triage Tree

```
Game fails to load / board doesn't appear
│
├─ Loading overlay never disappears
│   ├─ Check console for "Uncaught Error" that is NOT the control-flow exception
│   │   → Real error: read stack trace, fix the underlying Rust panic or JS error
│   └─ Only control-flow exception ("Using exceptions for control flow...") → normal
│       → Check that app.js try/catch is in place (see review-web skill)
│
├─ Assets 404 (pieces.glb, FiraSans-Bold.ttf not found)
│   ├─ Is web/assets symlink present? → ls -la web/assets
│   │   → Missing: ln -s ../assets web/assets
│   └─ Is server serving from web/ directory?
│       → lsof -p $(lsof -ti :8091) | grep cwd
│
├─ Console shows "WASM instantiation failed" or "CompileError"
│   ├─ Rebuild: wasm-pack build --target web --out-dir web/pkg
│   └─ Check that MIME type for .wasm is application/wasm (nginx.conf or python server)
│
└─ Board loads but pieces missing / wrong behavior
    ├─ Check ERROR lines in console (Bevy logs errors with ERROR prefix)
    └─ Run build-wasm skill to see if there are Rust compiler errors
```

## Reading Bevy Console Logs

Bevy logs to browser console via `bevy_log`. Format:
```
INFO  bevy_render::renderer > AdapterInfo { ... }    ← GPU adapter chosen
WARN  bevy_pbr::ssao        > ScreenSpaceAmbientOcclusion...  ← expected on WebGL2
ERROR bevy_asset::server    > Path not found: assets/...      ← real error, fix it
```

**Expected warnings (not errors):**
- `SSAO not loaded. GPU lacks support` — WebGL 2.0 limitation
- `Disabling depth of field` — WebGL 2.0 limitation
- `AudioContext was not allowed to start` — requires user gesture, not a bug

**Real errors (fix these):**
- `ERROR bevy_asset::server > Path not found` — missing asset file
- `ERROR bevy_render` — GPU/rendering failure
- `panicked at` in console — Rust panic, check stack trace

## WASM Panic Stack Trace

If you see `panicked at 'message', src/file.rs:line`:
1. The Rust source path is relative to the project root
2. Open `src/file.rs` at the indicated line
3. Common causes: `.unwrap()` on `None`, array out of bounds, `query.single()` with 0 results

## System Not Running

If a Bevy system appears to not execute:
1. Check it's registered: `app.add_systems(Update, system_name)` in the plugin's `build()` method
2. Check run conditions: `.run_if(...)` may be blocking it
3. Check schedule: startup systems won't run after init

## Rebuilding After Fix

```bash
# Kill server
lsof -ti :8091 | xargs kill -9 2>/dev/null; true
# Rebuild
wasm-pack build --target web --out-dir web/pkg
# Restart
python3 -m http.server 8091 &>/tmp/chess-server.log &
open http://localhost:8091
```
