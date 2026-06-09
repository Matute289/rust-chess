---
name: build-wasm
description: Compile rust-chess to WASM without starting a server. Use to validate Rust changes compile correctly before serving or deploying.
---

# Build WASM — rust-chess

## Command (run from project root)

```bash
wasm-pack build --target web --out-dir web/pkg
```

## Expected Output

```
[INFO]: 🎯  Compiling to Wasm...
[INFO]: ⬇️  Installing wasm-bindgen...
[INFO]: Optimizing wasm binaries with `wasm-opt`...
[INFO]: ✨   Done in Xs
[INFO]: 📦   Your wasm pkg is ready to publish at .../web/pkg.
```

Pre-existing warnings (not errors, ignore them):
- `Color::rgb` deprecation warnings in Bevy 0.14
- `Optional fields missing from Cargo.toml` — not blocking

## Failure Modes

| Error | Cause | Fix |
|---|---|---|
| `error[E0432]` missing import | Bevy API mismatch | Check Bevy 0.14 docs, not 0.15 |
| `wasm-pack not found` | Tool not installed | `cargo install wasm-pack` |
| `error: linking with cc failed` | Missing wasm target | `rustup target add wasm32-unknown-unknown` |

## Output Location

`web/pkg/` — contains `bevy_chess_bg.wasm`, `bevy_chess.js`, and supporting files.
