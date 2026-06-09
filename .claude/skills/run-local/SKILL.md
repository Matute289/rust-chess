---
name: run-local
description: Build WASM and serve the chess game locally. Use whenever Rust, JS, CSS, or HTML changed and you want to see the result in the browser.
---

# Run Local — rust-chess

Kills any existing server, rebuilds WASM, starts a fresh server, opens the game in the browser.

## Steps

**1. Kill any existing server on port 8091**
```bash
lsof -ti :8091 | xargs kill -9 2>/dev/null; true
```

**2. Build WASM** (run from project root)
```bash
wasm-pack build --target web --out-dir web/pkg
```
Expected: ends with `✨  Done in Xs`. Warnings about `Color::rgb` are pre-existing, not errors.

**3. Ensure assets symlink exists**
```bash
ls web/assets 2>/dev/null || ln -s ../assets web/assets
```

**4. Start server from `web/`**
```bash
python3 -m http.server 8091 &>/tmp/chess-server.log &
echo "Server PID: $!"
```

**5. Verify assets accessible**
```bash
sleep 1 && curl -s -o /dev/null -w "%{http_code}" http://localhost:8091/assets/models/chess_kit/pieces.glb
```
Expected: `200`. If `404`: the symlink failed — check `web/assets -> ../assets`.

**6. Open browser**
```bash
open http://localhost:8091
```

## Success Criteria
- Loading overlay shows "♟ AJEDREZ / Cargando..."
- Overlay disappears after ~3–5 seconds
- 3D chess board visible, no console errors about missing assets
- Pieces respond to clicks

## Known Quirks
- `wasm-bindgen` throws "Using exceptions for control flow, don't mind me. This isn't actually an error!" — normal, caught in app.js try/catch
- AudioContext warning on first load — normal, requires user gesture
- SSAO and DoF warnings — expected on WebGL 2.0, no action needed
