mod auth;
mod config;
mod db;
mod error;
mod middleware;
mod models;

use std::sync::Arc;
use axum::{routing::get, Router};
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
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("chess-server listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
