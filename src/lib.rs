mod board;
mod pieces;
mod ui;

use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use board::BoardPlugin;
use pieces::PiecesPlugin;
use ui::UIPlugin;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    run_app();
}

pub fn run_app() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ajedrez".to_string(),
                    resolution: (1200., 1000.).into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .add_plugins(DefaultPickingPlugins)
        .add_plugins((BoardPlugin, PiecesPlugin, UIPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_matrix(Mat4::from_rotation_translation(
            Quat::from_xyzw(-0.3, -0.5, -0.3, 0.5).normalize(),
            Vec3::new(-4.0, 12.0, 4.0),
        )),
        ..default()
    });

    commands.spawn(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
        ..default()
    });
}
