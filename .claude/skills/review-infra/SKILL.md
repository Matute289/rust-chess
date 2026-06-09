---
name: review-infra
description: Code review for deploy/Dockerfile.vps, deploy/nginx.conf, deploy/docker-compose.vps.yml. Use before merging infra changes or after adding new asset types.
---

# Review: Infrastructure — rust-chess

## Files to Review
- `deploy/Dockerfile.vps`
- `deploy/nginx.conf`
- `deploy/docker-compose.vps.yml`

## Dockerfile Checklist
- [ ] Multi-stage build: `builder` stage (rust:1.88-slim + wasm-pack) + `nginx:alpine` stage
- [ ] `assets/` copied to `/usr/share/nginx/html/assets/` in final stage — not symlinked
- [ ] `web/` copied to `/usr/share/nginx/html/` AFTER `wasm-pack build` in builder stage
- [ ] `.cargo/` cache directory copied before `Cargo.toml`/`src/` for layer caching
- [ ] No secrets or credentials in any layer
- [ ] Builder image version pinned (`rust:1.88-slim`, not `rust:latest`)

### Build Order (must be in this sequence for cache efficiency)
1. Copy `.cargo/`
2. Copy `Cargo.toml`, `Cargo.lock`
3. Copy `src/`
4. Copy `assets/`
5. Run `wasm-pack build`
6. Copy `web/`

## nginx.conf Checklist
- [ ] WASM served with correct MIME type: `application/wasm` for `.wasm` files
- [ ] `.js` served as `application/javascript` (not `text/plain`)
- [ ] GLB/assets served with appropriate `Cache-Control` headers (long TTL ok for assets)
- [ ] HTML served with `Cache-Control: no-cache` — ensures fresh on deploy
- [ ] Gzip enabled for JS, WASM, HTML, CSS
- [ ] No directory listing (`autoindex off`)

## Security Headers (nice-to-have, flag if missing)
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: SAMEORIGIN`
- `Content-Security-Policy` scoped to allow WASM execution
