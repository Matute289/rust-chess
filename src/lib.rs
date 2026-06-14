mod adaptive_ai;
mod ai;
mod analysis;
mod auth;
mod board;
mod captured;
mod feedback_ui;
mod home;
mod lessons;
mod persistence;
mod pieces;
mod pvl_hub;
mod state;
mod suggestion;
mod ui;

use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
use bevy_mod_picking::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use adaptive_ai::AdaptiveAiPlugin;
use ai::AIPlugin;
use analysis::AnalysisPlugin;
use auth::AuthPlugin;
use board::BoardPlugin;
use captured::CapturedPlugin;
use feedback_ui::FeedbackUiPlugin;
use home::HomePlugin;
use lessons::LessonsPlugin;
use persistence::PersistencePlugin;
use pieces::PiecesPlugin;
use pvl_hub::PvLHubPlugin;
use state::{AppState, GameConfig, GameMode, LessonSetup};
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
    #[wasm_bindgen(js_namespace = window)]
    fn set_pvl_mode(is_pvl: bool);
}

// JS can set these to request a state transition on the next Bevy frame
static NAV_TO_HUB:  AtomicBool = AtomicBool::new(false);
static NAV_TO_HOME: AtomicBool = AtomicBool::new(false);

// Touch scroll delta for the lesson list (screen-to-canvas-space px, set by JS)
pub(crate) static LESSON_SCROLL_DELTA: AtomicI32 = AtomicI32::new(0);

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn go_to_pvl_hub() {
    NAV_TO_HUB.store(true, Ordering::SeqCst);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn go_to_home() {
    NAV_TO_HOME.store(true, Ordering::SeqCst);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn bevy_lesson_scroll(delta: f32) {
    LESSON_SCROLL_DELTA.fetch_add(delta as i32, Ordering::Relaxed);
}

fn on_enter_playing(config: Res<GameConfig>) {
    #[cfg(target_arch = "wasm32")]
    {
        show_game_controls();
        set_pvl_mode(config.mode == GameMode::PvL || config.mode == GameMode::Lesson);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = config;
}

fn on_enter_home() {
    #[cfg(target_arch = "wasm32")]
    hide_game_controls();
}

fn on_enter_pvl_hub() {
    #[cfg(target_arch = "wasm32")]
    hide_game_controls();
}

fn on_enter_lessons() {
    #[cfg(target_arch = "wasm32")]
    hide_game_controls();
}

fn on_enter_lesson_retry() {
    // Bounce state — LessonsPlugin::lesson_retry_enter immediately sets Playing.
}

fn poll_nav_requests(mut next_state: ResMut<NextState<AppState>>) {
    if NAV_TO_HUB.swap(false, Ordering::SeqCst) {
        next_state.set(AppState::PvLHub);
    } else if NAV_TO_HOME.swap(false, Ordering::SeqCst) {
        next_state.set(AppState::Home);
    }
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
        .init_resource::<LessonSetup>()
        .add_plugins(AuthPlugin)
        .add_plugins((HomePlugin, PvLHubPlugin, BoardPlugin, PiecesPlugin, CapturedPlugin, UIPlugin, AIPlugin, AnalysisPlugin, PersistencePlugin, SuggestionPlugin, AdaptiveAiPlugin, LessonsPlugin, FeedbackUiPlugin))
        .add_systems(OnEnter(AppState::Playing), on_enter_playing)
        .add_systems(OnEnter(AppState::Home),    on_enter_home)
        .add_systems(OnEnter(AppState::PvLHub),  on_enter_pvl_hub)
        .add_systems(OnEnter(AppState::Lessons),     on_enter_lessons)
        .add_systems(OnEnter(AppState::LessonRetry), on_enter_lesson_retry)
        .add_systems(Update, poll_nav_requests)
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
