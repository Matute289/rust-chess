mod ai_profile;
mod auth;
mod config;
mod db;
mod error;
mod feedback;
mod games;
mod lesson_progress;
mod middleware;
mod models;
mod stats;

use std::sync::Arc;
use axum::{routing::{get, post}, Router};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub pool:   PgPool,
    pub config: Arc<config::Config>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = Arc::new(config::Config::from_env()?);
    let pool = db::connect(&cfg.database_url).await?;
    db::migrate(&pool).await?;

    let state = AppState { pool, config: cfg.clone() };

    let cors = CorsLayer::new()
        .allow_origin(cfg.frontend_url.parse::<axum::http::HeaderValue>()?)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/health",                   get(|| async { "ok" }))
        .route("/api/auth/google/login",         get(auth::google_login))
        .route("/api/auth/google/callback",      get(auth::google_callback))
        .route("/api/auth/github/login",         get(auth::github_login))
        .route("/api/auth/github/callback",      get(auth::github_callback))
        .route("/api/auth/discord/login",        get(auth::discord_login))
        .route("/api/auth/discord/callback",     get(auth::discord_callback))
        .route("/api/me",                        get(auth::me))
        .route("/api/games",                     post(games::post_game))
        .route("/api/stats",                     get(stats::get_stats))
        .route("/api/ai_profile",                get(ai_profile::get_ai_profile))
        .route("/api/ai_profile",                axum::routing::patch(ai_profile::patch_ai_profile))
        .route("/api/lesson_progress",           get(lesson_progress::get_lesson_progress))
        .route("/api/lesson_progress",           post(lesson_progress::post_lesson_progress))
        .route("/api/feedback",                  post(feedback::post_feedback))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("chess-server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
