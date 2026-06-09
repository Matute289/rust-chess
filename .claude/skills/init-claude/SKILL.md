---
name: init-claude
description: Generate CLAUDE.md at the project root with architecture overview, file map, build commands, deploy process, and known quirks. Use once at project setup or after major structural changes.
---

# Init CLAUDE.md — rust-chess

Generate a `CLAUDE.md` at the project root using the template below. Read the current state of key files before filling in any values that may have changed.

## Steps

**1. Read current project state**
```bash
cat Cargo.toml
ls src/
ls web/
ls deploy/
git log --oneline -5
```

**2. Write CLAUDE.md** using this template, filling in current values:

```markdown
# rust-chess

Bevy 0.14 chess game compiled to WASM, served as a static site.
Production: https://chess.greenmountain.dev (deployed via `browser` branch → GitHub Actions → Docker → nginx)

## Architecture

| File | Responsibility |
|---|---|
| `src/lib.rs` | WASM entry point, Bevy App setup, Camera + Light |
| `src/board.rs` | BoardPlugin — square entities, selection, highlighting |
| `src/pieces.rs` | PiecesPlugin — piece entities, movement, capture |
| `src/ui.rs` | UIPlugin — text overlays, turn display |
| `web/index.html` | HTML shell — canvas, loading overlay, portrait hint |
| `web/style.css` | Responsive layout — canvas centering, overlays |
| `web/app.js` | Canvas scaling, portrait detection, WASM boot |
| `deploy/Dockerfile.vps` | Multi-stage build: rust:1.88 → nginx:alpine |
| `deploy/nginx.conf` | nginx config — MIME types, cache headers |

## Build Commands

```bash
# Build WASM
wasm-pack build --target web --out-dir web/pkg

# Serve locally (port 8091)
python3 -m http.server 8091   # run from web/ directory

# Ensure assets symlink (first time only)
ln -s ../assets web/assets
```

## Local Dev Loop

1. Make changes to `src/` or `web/`
2. Run `run-local` skill (rebuilds + serves + opens browser)
3. Test in browser
4. Run `deploy` skill to push to production

## Deploy

Push to `browser` branch → CI builds Docker image → deploys to chess.greenmountain.dev.

```bash
git add <files>
git commit -m "feat/fix: description"
git push origin browser
gh run list --branch browser --limit 3  # monitor CI
```

## Known Quirks

- **wasm-bindgen control-flow exception**: `await init()` throws "Using exceptions for control flow, don't mind me." — this is normal. Caught in `app.js` try/catch. Not a bug.
- **Assets symlink**: `web/assets` must be a symlink to `../assets`. Run `ln -s ../assets web/assets` if assets return 404.
- **Port 8091**: project uses 8091 (8090 conflicts with simple-flappy-dragon).
- **Color::rgb deprecation**: pre-existing warnings in Bevy 0.14, not errors.
- **WebGL 2.0 limitations**: SSAO and DoF disabled on WASM — expected Bevy WARN logs.

## Skills

This project has skills in `.claude/skills/`. Key ones:
- `run-local` — full dev loop (build + serve + open)
- `deploy` — push to production
- `bevy-debug` — debug WASM/Bevy errors
- `mobile-debug` — debug mobile-specific issues
- `review-board/pieces/ui/web/infra` — subsystem code reviews
```

**3. Commit**
```bash
git add CLAUDE.md
git commit -m "docs: add CLAUDE.md with project architecture and dev guide"
```
