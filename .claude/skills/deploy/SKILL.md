---
name: deploy
description: Stage, commit, and push to the browser branch to trigger CI/CD deploy to chess.greenmountain.dev. Use when a feature is tested locally and ready for production.
---

# Deploy — rust-chess

Pushes to `browser` branch → GitHub Actions builds Docker image → deploys to chess.greenmountain.dev.

## Pre-deploy Checklist

Before running any git commands:
- [ ] Local build passes: run `build-wasm` skill, no errors
- [ ] Game works in browser: run `run-local` skill, board renders
- [ ] No unintended files staged (check `git status`)

## Steps

**1. Review what will be committed**
```bash
git status
git diff --stat
```

**2. Stage files** (be explicit — never `git add .`)
```bash
git add src/ web/index.html web/style.css web/app.js
# Add other changed files as needed
```

Do NOT stage:
- `web/pkg/` — built artifacts, in .gitignore
- `web/assets` — symlink, not needed (assets/ is committed separately)
- `.env` or secrets

**3. Commit**
```bash
git commit -m "feat/fix: <description>"
```

**4. Push to browser branch**
```bash
git push origin browser
```

**5. Monitor CI**
```bash
gh run list --branch browser --limit 3
```
Wait for status to show `completed` / `success`. Usually takes 3–5 minutes.

## Failure Modes

| Symptom | Cause | Fix |
|---|---|---|
| Push rejected | Branch protected or diverged | `git pull --rebase origin browser` then push |
| CI fails at `wasm-pack build` | Rust compile error in CI | Run `build-wasm` locally first |
| CI fails at Docker build | Dockerfile issue | Check `deploy/Dockerfile.vps` |
| Site not updated after CI green | nginx cache | Hard refresh (Ctrl+Shift+R) or check deploy-status |

## Branch Convention

Always push to `browser` branch. Do NOT push directly to `main` or `develop` for production deploys.
