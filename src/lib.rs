mod ai;
mod analysis;
mod auth;
mod board;
mod captured;
mod home;
mod persistence;
mod pieces;
mod pvl_hub;
mod state;
mod suggestion;
mod ui;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use ai::AIPlugin;
use analysis::AnalysisPlugin;
use auth::AuthPlugin;
use board::BoardPlugin;
use captured::CapturedPlugin;
use home::HomePlugin;
use persistence::PersistencePlugin;
use pieces::PiecesPlugin;
use pvl_hub::PvLHubPlugin;
use state::{AppState, GameConfig};
use suggestion::SuggestionPlugin;
use ui::UIPlugin;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn wasm_main() {
    run_app();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = window)]
    fn show_game_controls();
    #[wasm_bindgen(js_namespace = window)]
    fn hide_game_controls();
}

fn on_enter_playing() {
    #[cfg(target_arch = "wasm32")]
    show_game_controls();
}

fn on_enter_home() {
    #[cfg(target_arch = "wasm32")]
    hide_game_controls();
}

fn on_enter_pvl_hub() {
    #[cfg(target_arch = "wasm32")]
    hide_game_controls();
}

pub fn run_app() {
    App::new()
        .add_plugins(
            DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Ajedrez".to_string(),
                    resolution: (1200., 1000.).into(),
                    canvas: Some("#canvas".to_string()),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
        )
        .add_plugins(DefaultPickingPlugins)
        .init_state::<AppState>()
        .init_resource::<GameConfig>()
        .add_plugins(AuthPlugin)
        .add_plugins((HomePlugin, PvLHubPlugin, BoardPlugin, PiecesPlugin, CapturedPlugin, UIPlugin, AIPlugin, AnalysisPlugin, PersistencePlugin, SuggestionPlugin))
        .add_systems(OnEnter(AppState::Playing), on_enter_playing)
        .add_systems(OnEnter(AppState::Home),    on_enter_home)
        .add_systems(OnEnter(AppState::PvLHub),  on_enter_pvl_hub)
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
