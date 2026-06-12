use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use crate::{
    analysis::{AnalysisReport, GameNarrative},
    auth::UserSession,
    board::{GameHistory, GameStatus, GameStatusEvent},
    pieces::PieceColor,
    state::{AppState, GameConfig, GameMode},
};

pub struct PersistResult {
    pub new_elo: i32,
}

#[derive(Resource, Clone)]
pub struct PersistFetchState(Arc<Mutex<Option<PersistResult>>>);

impl Default for PersistFetchState {
    fn default() -> Self { Self(Arc::new(Mutex::new(None))) }
}

pub struct PersistencePlugin;

impl Plugin for PersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PersistFetchState>()
           .add_systems(Update, (
               persist_on_game_end,
               poll_persist_result,
           ).run_if(in_state(AppState::Playing)));
    }
}

fn persist_on_game_end(
    mut events: EventReader<GameStatusEvent>,
    session:    Res<UserSession>,
    config:     Res<GameConfig>,
    history:    Res<GameHistory>,
    report:     Res<AnalysisReport>,
    narrative:  Res<GameNarrative>,
    state:      Res<PersistFetchState>,
) {
    for ev in events.read() {
        let result_str: &'static str = match &ev.0 {
            GameStatus::Checkmate { winner } => {
                let my_side = if config.mode == GameMode::PvP { PieceColor::White }
                              else { config.player_side };
                if *winner == my_side { "win" } else { "loss" }
            }
            GameStatus::Stalemate => "draw",
            _ => continue,
        };

        if config.mode != GameMode::PvL { continue; }
        if !session.is_logged_in() || history.moves.is_empty() { continue; }

        let jwt     = session.jwt.clone().unwrap();
        let moves   = history.moves.iter().map(|m| m.to_uci()).collect::<Vec<_>>().join(" ");
        let opp_elo = Some(config.difficulty.elo_estimate());
        let summary = narrative.0.clone();
        let (aw, ab, bl, mi, ina) = report.0.as_ref().map(|r| {
            let s = &r.summary;
            (Some(s.accuracy_white), Some(s.accuracy_black),
             [s.blunders[0], s.blunders[1]],
             [s.mistakes[0], s.mistakes[1]],
             [s.inaccuracies[0], s.inaccuracies[1]])
        }).unwrap_or((None, None, [0u8; 2], [0u8; 2], [0u8; 2]));

        dispatch_persist(
            state.0.clone(),
            jwt, result_str, opp_elo,
            aw, ab, bl, mi, ina, moves, summary,
        );
    }
}

fn dispatch_persist(
    arc:            Arc<Mutex<Option<PersistResult>>>,
    jwt:            String,
    result:         &'static str,
    opponent_elo:   Option<i32>,
    accuracy_white: Option<f32>,
    accuracy_black: Option<f32>,
    blunders:       [u8; 2],
    mistakes:       [u8; 2],
    inaccuracies:   [u8; 2],
    moves_uci:      String,
    summary:        Option<String>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        #[derive(serde::Serialize)]
        struct P {
            mode:           &'static str,
            result:         &'static str,
            opponent_elo:   Option<i32>,
            accuracy_white: Option<f32>,
            accuracy_black: Option<f32>,
            blunders:       [u8; 2],
            mistakes:       [u8; 2],
            inaccuracies:   [u8; 2],
            moves_uci:      String,
            summary:        Option<String>,
        }

        let payload = P {
            mode: "pvl", result, opponent_elo,
            accuracy_white, accuracy_black,
            blunders, mistakes, inaccuracies,
            moves_uci, summary,
        };

        wasm_bindgen_futures::spawn_local(async move {
            let req = gloo_net::http::Request::post(
                "https://rustchess.greenmountain.dev/api/games",
            )
            .header("Authorization", &format!("Bearer {}", jwt))
            .json(&payload);

            if let Ok(req) = req {
                if let Ok(resp) = req.send().await {
                    #[derive(serde::Deserialize)]
                    struct R { new_elo: i32 }
                    if let Ok(r) = resp.json::<R>().await {
                        *arc.lock().unwrap() = Some(PersistResult { new_elo: r.new_elo });
                    }
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = (arc, jwt, result, opponent_elo, accuracy_white, accuracy_black,
             blunders, mistakes, inaccuracies, moves_uci, summary);
}

fn poll_persist_result(
    state:       Res<PersistFetchState>,
    mut session: ResMut<UserSession>,
) {
    if let Ok(mut guard) = state.0.try_lock() {
        if let Some(r) = guard.take() {
            session.elo = Some(r.new_elo);
        }
    }
}
