use axum::{extract::State, Json};
use serde::Serialize;
use crate::{AppState, error::AppError, middleware::AuthUser};

#[derive(Serialize, sqlx::FromRow)]
pub struct UserStats {
    pub wins:               i64,
    pub losses:             i64,
    pub draws:              i64,
    pub accuracy_avg:       Option<f32>,
    pub blunders_total:     i64,
    pub mistakes_total:     i64,
    pub inaccuracies_total: i64,
}

pub async fn get_stats(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
) -> Result<Json<UserStats>, AppError> {
    let stats = sqlx::query_as::<_, UserStats>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE result = 'win')  AS wins,
            COUNT(*) FILTER (WHERE result = 'loss') AS losses,
            COUNT(*) FILTER (WHERE result = 'draw') AS draws,
            AVG(
                (COALESCE(accuracy_white, 0.0) + COALESCE(accuracy_black, 0.0)) / 2.0
            )::REAL AS accuracy_avg,
            COALESCE(SUM(blunders_white::BIGINT     + blunders_black::BIGINT),     0::BIGINT) AS blunders_total,
            COALESCE(SUM(mistakes_white::BIGINT     + mistakes_black::BIGINT),     0::BIGINT) AS mistakes_total,
            COALESCE(SUM(inaccuracies_white::BIGINT + inaccuracies_black::BIGINT), 0::BIGINT) AS inaccuracies_total
        FROM games
        WHERE user_id = $1 AND mode = 'pvl'
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;

    Ok(Json(stats))
}
