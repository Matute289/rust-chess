use axum::{extract::State, Json};
use serde::Deserialize;
use crate::{AppState, error::AppError};

#[derive(Deserialize)]
pub struct FeedbackBody {
    pub message: String,
    pub context: Option<String>,
}

pub async fn post_feedback(
    State(state): State<AppState>,
    Json(body): Json<FeedbackBody>,
) -> Result<(), AppError> {
    let msg = body.message.trim().to_string();
    if msg.is_empty() {
        return Err(AppError::BadRequest("mensaje vacío".into()));
    }
    if msg.len() > 2000 {
        return Err(AppError::BadRequest("mensaje demasiado largo".into()));
    }

    sqlx::query("INSERT INTO feedback (message, context) VALUES ($1, $2)")
        .bind(&msg)
        .bind(body.context.as_deref())
        .execute(&state.pool)
        .await
        .map_err(anyhow::Error::from)?;

    if let Some(ref key) = state.config.resend_api_key {
        let ctx = body.context.as_deref().unwrap_or("app");
        if let Err(e) = send_email(key, &msg, ctx).await {
            tracing::warn!("email send failed: {e:#}");
        }
    }

    Ok(())
}

async fn send_email(api_key: &str, message: &str, context: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "from":    "RustChess <onboarding@resend.dev>",
        "to":      ["maticgrinberg@gmail.com"],
        "subject": format!("RustChess Feedback [{}]", context),
        "text":    message,
    });
    client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await?;
    Ok(())
}
