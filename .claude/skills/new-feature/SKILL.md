---
name: new-feature
description: Scaffold a new Bevy plugin for rust-chess following project conventions. Use when adding a new subsystem (AI engine, settings screen, multiplayer, animation system, etc.).
---

# New Feature — rust-chess

Scaffolds a new Bevy plugin following the patterns in `src/board.rs`, `src/pieces.rs`, `src/ui.rs`.

## Steps

**1. Determine the plugin name**

Use snake_case for the filename and PascalCase for the Plugin struct. Example: feature `ai_engine` → file `src/ai_engine.rs` → struct `AIEnginePlugin`.

**2. Create `src/<feature>.rs`**

```rust
use bevy::prelude::*;

pub struct <FeatureName>Plugin;

impl Plugin for <FeatureName>Plugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, setup_<feature>)
            .add_systems(Update, update_<feature>);
    }
}

fn setup_<feature>(mut commands: Commands) {
    // One-time initialization: spawn entities, insert resources
}

fn update_<feature>(
    // Query parameters here
) {
    // Per-frame logic
}
```

Replace:
- `<FeatureName>` with PascalCase name (e.g., `AIEngine`)
- `<feature>` with snake_case name (e.g., `ai_engine`)

**3. Register in `src/lib.rs`**

Add the module declaration and plugin to `run_app()`:

```rust
// At the top of lib.rs, with other mod declarations:
mod <feature>;

// Inside run_app(), with other plugins:
use <feature>::<FeatureName>Plugin;

App::new()
    // ... existing plugins ...
    .add_plugins(<FeatureName>Plugin)
    .run();
```

**4. Define Components and Resources**

If the feature needs data:

```rust
// Components: per-entity data
#[derive(Component)]
pub struct <FeatureName>State {
    pub field: Type,
}

// Resources: global/singleton data
#[derive(Resource)]
pub struct <FeatureName>Config {
    pub setting: Type,
}

// In build():
app.insert_resource(<FeatureName>Config { setting: default_value });
```

**5. Define Events (if needed)**

```rust
#[derive(Event)]
pub struct <FeatureName>Event {
    pub data: Type,
}

// In build():
app.add_event::<FeatureName>Event>();
```

**6. Verify compilation**

```bash
cargo check --target wasm32-unknown-unknown
```

No errors → run `run-local` skill to test in browser.

## Conventions to Follow

- Systems named `verb_noun` (e.g., `handle_selection`, `spawn_pieces`, `update_turn_text`)
- Startup systems for one-time entity spawning
- Update systems for per-frame logic
- Use `Changed<T>` filter to avoid running every frame when data hasn't changed
- Public types that other plugins need: mark `pub` and import in consumers
- Private implementation: no `pub` on internal functions
