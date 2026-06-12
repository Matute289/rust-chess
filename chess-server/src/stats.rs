use axum::{extract::State, Json};
use serde::Serialize;
use crate::{AppState, error::AppError, middleware::AuthUser};

#[derive(Serialize, sqlx::FromRow)]
pub struct RecentGame {
    pub id:             uuid::Uuid,
    pub result:         String,
    pub accuracy_white: Option<f32>,
    pub accuracy_black: Option<f32>,
    pub blunders:       i32,
    pub mistakes:       i32,
    pub inaccuracies:   i32,
    pub summary:        Option<String>,
}

#[derive(Serialize)]
pub struct UserStats {
    pub wins:               i64,
    pub losses:             i64,
    pub draws:              i64,
    pub accuracy_avg:       Option<f32>,
    pub blunders_total:     i64,
    pub mistakes_total:     i64,
    pub inaccuracies_total: i64,
    pub recent_games:       Vec<RecentGame>,
}

// Internal row type — lets sqlx handle nullable SUM/AVG naturally.
#[derive(sqlx::FromRow)]
struct RawStats {
    wins:               i64,
    losses:             i64,
    draws:              i64,
    accuracy_avg:       Option<f64>,   // AVG(REAL+REAL) → double precision, null when no rows
    blunders_total:     Option<i64>,   // SUM → null when no rows
    mistakes_total:     Option<i64>,
    inaccuracies_total: Option<i64>,
}

pub async fn get_stats(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
) -> Result<Json<UserStats>, AppError> {
    let raw = sqlx::query_as::<_, RawStats>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE result = 'win')  AS wins,
            COUNT(*) FILTER (WHERE result = 'loss') AS losses,
            COUNT(*) FILTER (WHERE result = 'draw') AS draws,
            AVG((accuracy_white + accuracy_black) / 2.0) AS accuracy_avg,
            SUM(blunders_white::INTEGER     + blunders_black::INTEGER)     AS blunders_total,
            SUM(mistakes_white::INTEGER     + mistakes_black::INTEGER)     AS mistakes_total,
            SUM(inaccuracies_white::INTEGER + inaccuracies_black::INTEGER) AS inaccuracies_total
        FROM games
        WHERE user_id = $1 AND mode = 'pvl'
        "#,
    )
    .bind(user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;

    let recent_games = sqlx::query_as::<_, RecentGame>(
        r#"
        SELECT
            id,
            result,
            accuracy_white,
            accuracy_black,
            (COALESCE(blunders_white,     0)::INTEGER + COALESCE(blunders_black,     0)::INTEGER) AS blunders,
            (COALESCE(mistakes_white,     0)::INTEGER + COALESCE(mistakes_black,     0)::INTEGER) AS mistakes,
            (COALESCE(inaccuracies_white, 0)::INTEGER + COALESCE(inaccuracies_black, 0)::INTEGER) AS inaccuracies,
            summary
        FROM games
        WHERE user_id = $1 AND mode = 'pvl'
        ORDER BY played_at DESC
        LIMIT 5
        "#,
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;

    Ok(Json(UserStats {
        wins:               raw.wins,
        losses:             raw.losses,
        draws:              raw.draws,
        accuracy_avg:       raw.accuracy_avg.map(|v| v as f32),
        blunders_total:     raw.blunders_total.unwrap_or(0),
        mistakes_total:     raw.mistakes_total.unwrap_or(0),
        inaccuracies_total: raw.inaccuracies_total.unwrap_or(0),
        recent_games,
    }))
}
