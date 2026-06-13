use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use crate::{AppState, error::AppError, middleware::AuthUser};

#[derive(Serialize, sqlx::FromRow)]
pub struct LessonProgressRow {
    pub lesson_idx: i32,
    pub stars:      i16,
}

pub async fn get_lesson_progress(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
) -> Result<Json<Vec<LessonProgressRow>>, AppError> {
    let rows = sqlx::query_as::<_, LessonProgressRow>(
        "SELECT lesson_idx, stars FROM lesson_progress WHERE user_id = $1 ORDER BY lesson_idx",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
pub struct PostProgressBody {
    pub lesson_idx: i32,
    pub mode:       String,
}

pub async fn post_lesson_progress(
    State(state): State<AppState>,
    AuthUser { user_id }: AuthUser,
    Json(body): Json<PostProgressBody>,
) -> Result<(), AppError> {
    if body.lesson_idx < 0 || body.lesson_idx > 4 {
        return Err(AppError::BadRequest("invalid lesson_idx".into()));
    }
    let new_stars: i16 = if body.mode == "interactive" { 2 } else { 1 };
    sqlx::query(
        r#"INSERT INTO lesson_progress (user_id, lesson_idx, stars)
           VALUES ($1, $2, $3)
           ON CONFLICT (user_id, lesson_idx)
           DO UPDATE SET stars = GREATEST(lesson_progress.stars, EXCLUDED.stars),
                         updated_at = NOW()"#,
    )
    .bind(user_id)
    .bind(body.lesson_idx)
    .bind(new_stars)
    .execute(&state.pool)
    .await
    .map_err(anyhow::Error::from)?;
    Ok(())
}
