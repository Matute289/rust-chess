---
name: deploy-status
description: Check whether the latest deploy reached chess.greenmountain.dev. Use after pushing to the browser branch to confirm production is updated.
---

# Deploy Status — rust-chess

## Quick Check

```bash
curl -sI https://chess.greenmountain.dev | grep -E "HTTP|last-modified|content-type"
```

Expected: `HTTP/2 200`, `content-type: text/html`

## CI Status

```bash
gh run list --branch browser --limit 5
```

Look for the most recent run. States:
- `completed` + `success` → deploy succeeded
- `completed` + `failure` → check logs: `gh run view <run-id> --log-failed`
- `in_progress` → wait 1–2 minutes and re-check

## Full Deploy Log

```bash
gh run view $(gh run list --branch browser --limit 1 --json databaseId -q '.[0].databaseId') --log
```

## If Site Returns 502/503

The nginx container or the Docker service may have crashed:
1. Use `sysadmin-linux` skill for server diagnosis
2. Check: `docker ps` on the VPS — is the chess container running?
