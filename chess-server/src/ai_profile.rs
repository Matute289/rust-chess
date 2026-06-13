use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::{AppState, error::AppError, middleware::AuthUser};

#[derive(Debug, sqlx::FromRow)]
pub struct AiProfileRow {
    pub elo_estimate:   i32,
    pub loss_count:     i32,
    pub learned_biases: serde_json::Value,
}

#[derive(Serialize)]
pub struct AiProfileResponse {
    pub elo_estimate:   i32,
    pub loss_count:     i32,
    pub learned_biases: serde_json::Value,
}

#[derive(Deserialize)]
pub struct PatchAiProfilePayload {
    pub elo_estimate:   i32,
    pub loss_count:     i32,
    pub learned_biases: serde_json::Value,
}

pub async fn get_ai_profile(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
) -> Result<Json<AiProfileResponse>, AppError> {
    let row = crate::db::get_or_create_ai_profile(&state.pool, user_id).await?;
    Ok(Json(AiProfileResponse {
        elo_estimate:   row.elo_estimate,
        loss_count:     row.loss_count,
        learned_biases: row.learned_biases,
    }))
}

pub async fn patch_ai_profile(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
    Json(p): Json<PatchAiProfilePayload>,
) -> Result<Json<AiProfileResponse>, AppError> {
    let row = crate::db::update_ai_profile(
        &state.pool, user_id, p.elo_estimate, p.loss_count, p.learned_biases,
    ).await?;
    Ok(Json(AiProfileResponse {
        elo_estimate:   row.elo_estimate,
        loss_count:     row.loss_count,
        learned_biases: row.learned_biases,
    }))
}
