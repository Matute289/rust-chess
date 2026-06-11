use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::Deserialize;
use uuid::Uuid;

use crate::{db, error::AppError, middleware::AuthUser, models::{JwtClaims, UserPublic}, AppState};

fn issue_jwt(user_id: Uuid, secret: &str) -> Result<String, AppError> {
    let exp = chrono::Utc::now().timestamp() as u64 + 30 * 24 * 3600;
    let claims = JwtClaims { sub: user_id.to_string(), exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::Internal(anyhow::anyhow!("JWT encode error: {e}")))
}

fn new_state() -> String {
    Uuid::new_v4().to_string()
}

// ─── Google ───────────────────────────────────────────────────────────────────

pub async fn google_login(State(app): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let state = new_state();
    db::store_oauth_state(&app.pool, &state, "google").await?;
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?client_id={}&redirect_uri={}/api/auth/google/callback\
         &response_type=code&scope=openid%20email%20profile&state={}",
        app.config.google_client_id, app.config.frontend_url, state
    );
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code:  String,
    pub state: String,
}

#[derive(Deserialize)]
struct GoogleToken {
    access_token: String,
}

#[derive(Deserialize)]
struct GoogleUser {
    sub:     String,
    email:   String,
    name:    String,
    picture: Option<String>,
}

pub async fn google_callback(
    Query(params): Query<OAuthCallback>,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !db::consume_oauth_state(&app.pool, &params.state, "google").await? {
        return Err(AppError::BadRequest("Invalid or expired state".into()));
    }
    let http = reqwest::Client::new();
    let token: GoogleToken = http
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code",          params.code.as_str()),
            ("client_id",     app.config.google_client_id.as_str()),
            ("client_secret", app.config.google_client_secret.as_str()),
            ("redirect_uri",  &format!("{}/api/auth/google/callback", app.config.frontend_url)),
            ("grant_type",    "authorization_code"),
        ])
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Google token request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Google token parse: {e}")))?;
    let profile: GoogleUser = http
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token.access_token)
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Google userinfo request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Google userinfo parse: {e}")))?;
    let user = db::upsert_user(
        &app.pool, &profile.email, &profile.name,
        profile.picture.as_deref(), "google", &profile.sub,
    ).await?;
    let jwt = issue_jwt(user.id, &app.config.jwt_secret)?;
    Ok(Redirect::temporary(&format!("{}/?jwt={}", app.config.frontend_url, jwt)))
}

// ─── GitHub ───────────────────────────────────────────────────────────────────

pub async fn github_login(State(app): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let state = new_state();
    db::store_oauth_state(&app.pool, &state, "github").await?;
    let url = format!(
        "https://github.com/login/oauth/authorize\
         ?client_id={}&redirect_uri={}/api/auth/github/callback\
         &scope=read:user%20user:email&state={}",
        app.config.github_client_id, app.config.frontend_url, state
    );
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct GithubToken {
    access_token: String,
}

#[derive(Deserialize)]
struct GithubUser {
    id:         i64,
    login:      String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmail {
    email:    String,
    primary:  bool,
    verified: bool,
}

pub async fn github_callback(
    Query(params): Query<OAuthCallback>,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !db::consume_oauth_state(&app.pool, &params.state, "github").await? {
        return Err(AppError::BadRequest("Invalid or expired state".into()));
    }
    let http = reqwest::Client::new();
    let token: GithubToken = http
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("code",          params.code.as_str()),
            ("client_id",     app.config.github_client_id.as_str()),
            ("client_secret", app.config.github_client_secret.as_str()),
            ("redirect_uri",  &format!("{}/api/auth/github/callback", app.config.frontend_url)),
        ])
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub token request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub token parse: {e}")))?;
    let profile: GithubUser = http
        .get("https://api.github.com/user")
        .bearer_auth(&token.access_token)
        .header("User-Agent", "chess-server/1.0")
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub user request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub user parse: {e}")))?;
    let emails: Vec<GithubEmail> = http
        .get("https://api.github.com/user/emails")
        .bearer_auth(&token.access_token)
        .header("User-Agent", "chess-server/1.0")
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub emails request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("GitHub emails parse: {e}")))?;
    let email = emails.iter()
        .find(|e| e.primary && e.verified)
        .map(|e| e.email.clone())
        .unwrap_or_else(|| format!("{}@github.invalid", profile.login));
    let user = db::upsert_user(
        &app.pool, &email, &profile.login,
        profile.avatar_url.as_deref(), "github", &profile.id.to_string(),
    ).await?;
    let jwt = issue_jwt(user.id, &app.config.jwt_secret)?;
    Ok(Redirect::temporary(&format!("{}/?jwt={}", app.config.frontend_url, jwt)))
}

// ─── Discord ──────────────────────────────────────────────────────────────────

pub async fn discord_login(State(app): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let state = new_state();
    db::store_oauth_state(&app.pool, &state, "discord").await?;
    let url = format!(
        "https://discord.com/api/oauth2/authorize\
         ?client_id={}&redirect_uri={}/api/auth/discord/callback\
         &response_type=code&scope=identify%20email&state={}",
        app.config.discord_client_id, app.config.frontend_url, state
    );
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct DiscordToken {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id:       String,
    username: String,
    email:    Option<String>,
    avatar:   Option<String>,
}

pub async fn discord_callback(
    Query(params): Query<OAuthCallback>,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !db::consume_oauth_state(&app.pool, &params.state, "discord").await? {
        return Err(AppError::BadRequest("Invalid or expired state".into()));
    }
    let http = reqwest::Client::new();
    let token: DiscordToken = http
        .post("https://discord.com/api/oauth2/token")
        .form(&[
            ("code",          params.code.as_str()),
            ("client_id",     app.config.discord_client_id.as_str()),
            ("client_secret", app.config.discord_client_secret.as_str()),
            ("redirect_uri",  &format!("{}/api/auth/discord/callback", app.config.frontend_url)),
            ("grant_type",    "authorization_code"),
        ])
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord token request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord token parse: {e}")))?;
    let profile: DiscordUser = http
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token.access_token)
        .send().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord user request: {e}")))?
        .json().await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("Discord user parse: {e}")))?;
    let avatar_url = profile.avatar.as_ref().map(|a|
        format!("https://cdn.discordapp.com/avatars/{}/{}.png", profile.id, a)
    );
    let email = profile.email
        .unwrap_or_else(|| format!("{}@discord.invalid", profile.id));
    let user = db::upsert_user(
        &app.pool, &email, &profile.username,
        avatar_url.as_deref(), "discord", &profile.id,
    ).await?;
    let jwt = issue_jwt(user.id, &app.config.jwt_secret)?;
    Ok(Redirect::temporary(&format!("{}/?jwt={}", app.config.frontend_url, jwt)))
}

// ─── /api/me ──────────────────────────────────────────────────────────────────

pub async fn me(
    auth_user: AuthUser,
    State(app): State<AppState>,
) -> Result<Json<UserPublic>, AppError> {
    let user = db::find_user_by_id(&app.pool, auth_user.user_id)
        .await?
        .ok_or(AppError::Unauthorized)?;
    Ok(Json(user.into()))
}
