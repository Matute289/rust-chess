---
name: serve-local
description: Start the local HTTP server without rebuilding WASM. Use when only JS, CSS, or HTML changed and the WASM build is already current.
---

# Serve Local — rust-chess

Faster than run-local when Rust code hasn't changed.

## Steps

**1. Kill any existing server on port 8091**
```bash
lsof -ti :8091 | xargs kill -9 2>/dev/null; true
```

**2. Ensure assets symlink exists**
```bash
ls web/assets 2>/dev/null || ln -s ../assets web/assets
```

**3. Start server from `web/`**
```bash
python3 -m http.server 8091 &>/tmp/chess-server.log &
sleep 1 && curl -s -o /dev/null -w "HTTP %{http_code}\n" http://localhost:8091/
```

**4. Open browser**
```bash
open http://localhost:8091
```

## When to use run-local instead

Use `run-local` (which rebuilds) if:
- Any `.rs` file changed
- `Cargo.toml` or `Cargo.lock` changed
- `web/pkg/` is missing or stale
