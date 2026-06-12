use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{AppState, error::AppError, middleware::AuthUser};

#[derive(Deserialize)]
pub struct GamePayload {
    pub mode:           String,
    pub result:         String,
    pub opponent_elo:   Option<i32>,
    pub accuracy_white: Option<f32>,
    pub accuracy_black: Option<f32>,
    pub blunders:       [u8; 2],
    pub mistakes:       [u8; 2],
    pub inaccuracies:   [u8; 2],
    pub moves_uci:      Option<String>,
    pub summary:        Option<String>,
}

#[derive(Serialize)]
pub struct GameResponse {
    pub game_id:   Uuid,
    pub new_elo:   i32,
    pub elo_delta: i32,
}

pub async fn post_game(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
    Json(payload): Json<GamePayload>,
) -> Result<Json<GameResponse>, AppError> {
    let user = crate::db::find_user_by_id(&state.pool, user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let (new_elo, elo_delta) = if let Some(opp_elo) = payload.opponent_elo {
        let score: f64 = match payload.result.as_str() {
            "win"  => 1.0,
            "loss" => 0.0,
            _      => 0.5,
        };
        let expected = 1.0 / (1.0 + 10f64.powf((opp_elo as f64 - user.elo as f64) / 400.0));
        let delta = (32.0 * (score - expected)).round() as i32;
        let new_elo = (user.elo + delta).max(100);
        (new_elo, delta)
    } else {
        (user.elo, 0)
    };

    let game_id = crate::db::insert_game_and_update_elo(
        &state.pool, user_id, &payload, new_elo,
    ).await?;

    Ok(Json(GameResponse { game_id, new_elo, elo_delta }))
}
