# rust-chess — Full Platform Skills Design
Date: 2026-06-09

## Context

rust-chess is a Bevy 0.14 chess game compiled to WASM via wasm-pack, served as a static site behind nginx, deployed to chess.greenmountain.dev via Docker CI on the `browser` branch. The project is entering a phase of major changes: multi-device responsive support (desktop, mobile, tablet, iPad), bug fixes, and eventually AI move generation. This skill suite covers the full development lifecycle up to (but not including) the AI phase.

## Skill Inventory — 18 skills

### Category 1: Build & Run

**`run-local`**
- Runs `wasm-pack build --target web --out-dir web/pkg`, starts `python3 -m http.server` from `web/`, opens browser at the local URL.
- Trigger: any time code changes and dev wants to see the result in the browser.
- Success: browser opens, chess board renders, no console errors.

**`build-wasm`**
- Runs `wasm-pack build --target web --out-dir web/pkg` only. No server.
- Trigger: validate compilation after a Rust change before serving or deploying.
- Success: `web/pkg/` updated, no compiler errors.

**`serve-local`**
- Starts the local HTTP server from `web/` without rebuilding.
- Trigger: when only JS/CSS/HTML changed and WASM is already current.
- Success: server running, assets accessible (200 on `/assets/models/chess_kit/pieces.glb`).

### Category 2: Deploy

**`deploy`**
- Stages changed files, creates a commit, pushes to `browser` branch.
- Checks CI status after push (GitHub Actions).
- Trigger: feature tested locally and ready to validate on mobile / production.
- Success: push accepted, CI green, chess.greenmountain.dev serving new version.

**`deploy-status`**
- Curls chess.greenmountain.dev, checks HTTP status and last-modified headers.
- Optionally checks GitHub Actions run status via `gh run list`.
- Trigger: after a deploy, to confirm it reached production without waiting manually.
- Success: site returns 200, response matches expected build.

### Category 3: Code Review by Subsystem

**`review-board`**
- Reviews `src/board.rs`: square spawn logic, selection system, highlight correctness, ECS component design.
- Bevy-aware: checks for system ordering issues, missing run conditions, component leaks.

**`review-pieces`**
- Reviews `src/pieces.rs`: piece spawn, legal move generation, capture logic, rule validation.
- Chess-aware: checks for edge cases (castling, en passant, promotion placeholders).

**`review-ui`**
- Reviews `src/ui.rs`: Bevy UI nodes, text components, layout correctness, reactivity.
- Checks for hardcoded pixel sizes that would break on different resolutions.

**`review-web`**
- Reviews `web/index.html`, `web/style.css`, `web/app.js`.
- Checks: canvas scaling correctness, touch-action coverage, portrait hint logic, loading overlay lifecycle, MIME types, asset paths.

**`review-infra`**
- Reviews `deploy/Dockerfile.vps`, `deploy/nginx.conf`, `deploy/docker-compose.vps.yml`.
- Checks: build cache efficiency, asset copy correctness, nginx cache headers, MIME types, gzip, security headers.

### Category 4: Architecture

**`bevy-architecture`**
- Reviews or advises on Bevy ECS patterns: plugin boundaries, system ordering, resource vs component choice, event vs query patterns, startup vs runtime systems.
- Anchored to Bevy 0.14 API (not 0.15+).

**`responsive-architecture`**
- Reviews or advises on the multi-device strategy: canvas scaling approach, CSS viewport rules, touch event handling, orientation detection, breakpoints for tablet/iPad.
- Reference: the responsive-mobile spec (2026-06-09-responsive-mobile-design.md).

### Category 5: Performance

**`wasm-perf`**
- Measures: WASM bundle size (before/after wasm-opt), `wasm-pack build` time, JS heap at steady state.
- Reports size breakdown, flags regressions vs previous build if git history available.
- Trigger: before any deploy that adds dependencies or significant code.

**`mobile-perf`**
- Checks frame rate, touch latency, and scaling quality on mobile viewport sizes.
- Uses browser DevTools emulation (via Playwright or manual instructions).
- Metrics: target 60fps, touch response < 100ms, canvas fits viewport without overflow.

### Category 6: Debugging

**`bevy-debug`**
- Systematic debugging for WASM/Bevy console errors: asset 404s, WASM panics, system scheduling issues, wasm-bindgen control-flow exceptions.
- Reads browser console output provided by user, maps to Bevy source, proposes targeted fixes.

**`mobile-debug`**
- Debugging for mobile-specific failures: touch events not firing, portrait hint not toggling, canvas scaling wrong on specific devices, iOS Safari quirks (AudioContext, viewport units).
- Guides user through DevTools remote debugging if needed.

### Category 7: Setup & Scaffolding

**`init-claude`**
- Generates `CLAUDE.md` at project root with: architecture overview, file map, build commands, deploy process, known quirks (wasm-bindgen control flow exception, assets symlink, port 8091).
- Trigger: once, or after major structural changes.

**`new-feature`**
- Scaffolds a new Bevy plugin following project conventions: creates `src/<name>.rs`, adds `Plugin` impl, registers in `lib.rs`, follows existing system naming patterns.
- Trigger: adding a new subsystem (e.g., AI engine, multiplayer, settings screen).

---

## Deferred (Post-AI Phase)

The following will be designed after responsive + fixes are complete:
- `ai-architecture` — design for move generation engine (minimax, MCTS, or external engine via JS bridge)
- `ai-review` — review of AI plugin code
- `ai-perf` — AI think time, WASM thread constraints

---

## Location

All skills are created under `.claude/skills/<skill-name>/SKILL.md` in the project root, following the superpowers skill format.

## Build & Test Process

Each skill is tested by invoking it in a real scenario immediately after creation, per `superpowers:writing-skills` protocol.
