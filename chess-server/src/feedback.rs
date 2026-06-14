use axum::{extract::State, Json};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
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

    let ctx = body.context.as_deref().unwrap_or("app");
    if let Err(e) = send_email(&state, &msg, ctx).await {
        tracing::warn!("email send failed: {e:#}");
    }

    Ok(())
}

async fn send_email(state: &AppState, message: &str, context: &str) -> anyhow::Result<()> {
    let cfg = &state.config;

    let email = Message::builder()
        .from(format!("RustChess <no-reply@greenmountain.dev>").parse()?)
        .to("mgrinberg@greenmountain.dev".parse()?)
        .subject(format!("RustChess Feedback [{}]", context))
        .body(message.to_string())?;

    let transport: AsyncSmtpTransport<Tokio1Executor> =
        match (&cfg.smtp_user, &cfg.smtp_pass) {
            (Some(user), Some(pass)) => {
                let creds = Credentials::new(user.clone(), pass.clone());
                AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.smtp_host)?
                    .port(cfg.smtp_port)
                    .credentials(creds)
                    .build()
            }
            _ => {
                AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&cfg.smtp_host)
                    .port(cfg.smtp_port)
                    .build()
            }
        };

    transport.send(email).await?;
    Ok(())
}
