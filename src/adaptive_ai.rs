use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use crate::{
    ai::build_fen_ep,
    auth::UserSession,
    board::{CastlingState, EnPassantTarget, GameHistory, GameStatus, GameStatusEvent, PlayerTurn},
    pieces::{Piece, PieceColor},
    state::{AppState, GameConfig, GameMode, PvLMode},
};

const API: &str = "https://rustchess.greenmountain.dev/api/ai_profile";
const MAX_BIASES: usize = 60;
const BLUNDER_PENALTY: i32 = -1_000_000;
const BLUNDER_THRESHOLD_CP: i32 = 200;

// ─── Resource ────────────────────────────────────────────────────────────────

#[derive(Resource, Default, Clone)]
pub struct AdaptiveAiProfile {
    pub elo_estimate: i32,
    pub loss_count:   i32,
    pub biases:       Vec<(u8, u8, i32)>,  // (from_sq_idx, to_sq_idx, delta)
    pub loaded:       bool,
}

impl AdaptiveAiProfile {
    pub fn elo_to_depth(&self) -> u8 {
        match self.elo_estimate {
            ..=899  => 2,
            900..=1099 => 3,
            1100..=1399 => 5,
            1400..=1699 => 7,
            _ => 9,
        }
    }
}

#[derive(Resource, Default, Clone)]
pub struct AdaptiveFetchState(Arc<Mutex<Option<ProfileResult>>>);

#[derive(Clone)]
struct ProfileResult {
    elo_estimate: i32,
    loss_count:   i32,
    biases:       Vec<(u8, u8, i32)>,
}

// ─── Banner ──────────────────────────────────────────────────────────────────

#[derive(Component)] pub struct LearningBanner;
#[derive(Resource, Default)] struct LearningBannerTimer(f32);

// ─── Systems ─────────────────────────────────────────────────────────────────

fn reset_profile_on_enter(mut profile: ResMut<AdaptiveAiProfile>) {
    // Preserve loaded profile data across games — only mark not-yet-loaded for
    // the case where we haven't fetched yet.
    if !profile.loaded {
        *profile = AdaptiveAiProfile::default();
    }
}

fn load_profile_on_enter(
    config:  Res<GameConfig>,
    session: Res<UserSession>,
    state:   Res<AdaptiveFetchState>,
) {
    if config.mode != GameMode::PvL || config.pvl_mode != PvLMode::Adaptativa { return; }
    let Some(jwt) = session.jwt.clone() else { return };
    let arc = state.0.clone();

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        let Ok(resp) = gloo_net::http::Request::get(API)
            .header("Authorization", &format!("Bearer {}", jwt))
            .send().await else { return };
        let Ok(p) = resp.json::<ApiProfile>().await else { return };
        *arc.lock().unwrap() = Some(api_profile_to_result(p));
    });

    #[cfg(not(target_arch = "wasm32"))]
    let _ = (arc, jwt);
}

fn poll_profile_load(
    state:       Res<AdaptiveFetchState>,
    mut profile: ResMut<AdaptiveAiProfile>,
) {
    if let Ok(mut guard) = state.0.try_lock() {
        if let Some(result) = guard.take() {
            profile.elo_estimate = result.elo_estimate;
            profile.loss_count   = result.loss_count;
            profile.biases       = result.biases;
            profile.loaded       = true;
        }
    }
}

fn on_game_end(
    mut events:  EventReader<GameStatusEvent>,
    config:      Res<GameConfig>,
    session:     Res<UserSession>,
    history:     Res<GameHistory>,
    turn:        Res<PlayerTurn>,
    castling:    Res<CastlingState>,
    ep:          Res<EnPassantTarget>,
    pieces_q:    Query<&Piece>,
    mut profile: ResMut<AdaptiveAiProfile>,
    mut timer:   ResMut<LearningBannerTimer>,
    state:       Res<AdaptiveFetchState>,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if config.mode != GameMode::PvL || config.pvl_mode != PvLMode::Adaptativa { return; }

    for ev in events.read() {
        let GameStatus::Checkmate { winner } = &ev.0 else { continue };
        let ai_side = if config.player_side == PieceColor::White { PieceColor::Black } else { PieceColor::White };
        if *winner == ai_side { continue; }  // AI won — no learning

        // AI lost — compute blunders, update ELO
        let blunder_biases = compute_blunder_biases(&history);

        let new_loss_count = profile.loss_count + 1;
        let elo_gain = (25i32).saturating_sub(new_loss_count / 3).max(5);
        let new_elo = (profile.elo_estimate + elo_gain).min(2500);

        let mut new_biases = profile.biases.clone();
        for b in &blunder_biases {
            new_biases.push(*b);
        }
        if new_biases.len() > MAX_BIASES {
            let drain = new_biases.len() - MAX_BIASES;
            new_biases.drain(0..drain);
        }

        profile.elo_estimate = new_elo;
        profile.loss_count   = new_loss_count;
        profile.biases       = new_biases.clone();

        // Spawn learning banner
        spawn_learning_banner(&mut commands, &asset_server);
        timer.0 = 3.5;

        // Save to backend
        if let Some(jwt) = session.jwt.clone() {
            let arc = state.0.clone();
            save_profile_async(arc, jwt, new_elo, new_loss_count, new_biases);
        }
    }
}

fn tick_banner(
    time:         Res<Time>,
    mut timer:    ResMut<LearningBannerTimer>,
    mut commands: Commands,
    banner_q:     Query<Entity, With<LearningBanner>>,
) {
    if timer.0 <= 0.0 { return; }
    timer.0 -= time.delta_seconds();
    if timer.0 <= 0.0 {
        for e in &banner_q { commands.entity(e).despawn_recursive(); }
    }
}

fn despawn_banner(mut commands: Commands, banner_q: Query<Entity, With<LearningBanner>>) {
    for e in &banner_q { commands.entity(e).despawn_recursive(); }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn compute_blunder_biases(history: &GameHistory) -> Vec<(u8, u8, i32)> {
    use chess_engine::{DifficultyConfig, Search, SearchResult};

    let mut biases = Vec::new();
    let Ok(mut pos) = chess_engine::Position::from_fen(&history.initial_fen) else { return biases };

    let moves = &history.moves;
    let total  = moves.len();
    // Only analyze the last 12 half-moves (6 full moves)
    let start  = total.saturating_sub(12);

    for (i, &mv) in moves.iter().enumerate() {
        // We only care about AI moves (even indices = white moves, odd = black)
        // But we track whose move it was based on side_to_move at that position
        let ai_is_white = pos.side_to_move == chess_engine::Color::White;
        // Skip: we want moves where the AI actually moved (for the current pos side_to_move)
        // We'll analyze every other move starting from a rough approximation

        if i >= start {
            let cfg = DifficultyConfig { max_depth: 3, max_nodes: 50_000, random_factor: 0.0 };
            let SearchResult::EngineMove(best_mv, best_score) = Search::new().best_move(&pos, &cfg);

            if best_mv != mv {
                // Evaluate the actual move that was played
                let actual_pos = pos.make_move(mv);
                let actual_score = -chess_engine::eval::evaluate(&actual_pos);
                let diff = best_score - actual_score;
                if diff > BLUNDER_THRESHOLD_CP {
                    biases.push((mv.from_sq().0, mv.to_sq().0, BLUNDER_PENALTY));
                }
            }
        }
        pos = pos.make_move(mv);
    }
    biases
}

#[derive(Deserialize)]
struct ApiProfile {
    elo_estimate:   i32,
    loss_count:     i32,
    learned_biases: Vec<[i64; 3]>,
}

fn api_profile_to_result(p: ApiProfile) -> ProfileResult {
    let biases = p.learned_biases.iter()
        .map(|b| (b[0] as u8, b[1] as u8, b[2] as i32))
        .collect();
    ProfileResult { elo_estimate: p.elo_estimate, loss_count: p.loss_count, biases }
}

fn save_profile_async(
    arc:        Arc<Mutex<Option<ProfileResult>>>,
    jwt:        String,
    elo:        i32,
    loss_count: i32,
    biases:     Vec<(u8, u8, i32)>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let biases_json: Vec<[i64; 3]> = biases.iter()
            .map(|&(f, t, d)| [f as i64, t as i64, d as i64])
            .collect();

        #[derive(Serialize)]
        struct P { elo_estimate: i32, loss_count: i32, learned_biases: Vec<[i64; 3]> }

        let payload = P { elo_estimate: elo, loss_count, learned_biases: biases_json };
        wasm_bindgen_futures::spawn_local(async move {
            let Ok(req) = gloo_net::http::Request::patch(API)
                .header("Authorization", &format!("Bearer {}", jwt))
                .json(&payload) else { return };
            if let Ok(resp) = req.send().await {
                let Ok(p) = resp.json::<ApiProfile>().await else { return };
                *arc.lock().unwrap() = Some(api_profile_to_result(p));
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = (arc, jwt, elo, loss_count, biases);
}

fn spawn_learning_banner(commands: &mut Commands, asset_server: &AssetServer) {
    let font = asset_server.load("fonts/DejaVuSans-Bold.ttf");
    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(80.0),
                left: Val::Px(0.0),
                right: Val::Px(0.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            z_index: ZIndex::Global(30),
            ..default()
        },
        LearningBanner,
    ))
    .with_children(|root| {
        root.spawn(NodeBundle {
            style: Style {
                padding: UiRect { left: Val::Px(28.0), right: Val::Px(28.0), top: Val::Px(12.0), bottom: Val::Px(12.0) },
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::rgba(0.05, 0.22, 0.08, 0.93)),
            border_color: BorderColor(Color::rgba(0.25, 0.80, 0.35, 0.65)),
            ..default()
        })
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                "¡La IA aprendió de esta derrota!",
                TextStyle { font, font_size: 24.0, color: Color::rgb(0.60, 1.00, 0.65) },
            ));
        });
    });
}

// ─── Plugin ──────────────────────────────────────────────────────────────────

pub struct AdaptiveAiPlugin;

impl Plugin for AdaptiveAiPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<AdaptiveAiProfile>()
            .init_resource::<AdaptiveFetchState>()
            .init_resource::<LearningBannerTimer>()
            .add_systems(OnEnter(AppState::Playing), (reset_profile_on_enter, load_profile_on_enter).chain())
            .add_systems(OnExit(AppState::Playing), despawn_banner)
            .add_systems(Update, (
                poll_profile_load,
                on_game_end,
                tick_banner,
            ).run_if(in_state(AppState::Playing)));
    }
}
