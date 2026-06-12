# Phase 2 Sub-project 1 — Backend (Axum) + OAuth Auth

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Axum REST backend with OAuth login (Google, GitHub, Discord) that issues JWTs, backed by a dedicated PostgreSQL database. The WASM frontend reads the JWT from localStorage on startup and shows the user's display name when logged in.

**Architecture:**
- `chess-server/` — new Rust crate in the workspace (native binary, Axum 0.7 + sqlx 0.8 + PostgreSQL)
- OAuth Authorization Code Flow: WASM opens `/api/auth/{provider}/login` → provider → `/api/auth/{provider}/callback` → backend stores user, issues JWT, redirects to `rustchess.greenmountain.dev/?jwt=<token>`
- WASM reads `?jwt=` on load, stores in `localStorage`, populates `UserSession` Bevy resource
- `deploy/docker-compose.vps.yml` gains `chess-api` (port 8006) + `chess-db` (PostgreSQL 16)
- Nginx adds `/api/` proxy block to the existing `rustchess.greenmountain.dev` server block

**Tech Stack:** Rust 2021, Axum 0.7, sqlx 0.8 (PostgreSQL), jsonwebtoken 9, reqwest 0.12, Docker Compose, Nginx on Vultr VPS Ubuntu 24.04

---

## Pre-requisites (done by operator before Task 1)

Register OAuth apps at each provider and collect credentials:

| Provider | Where | Callback URL to register |
|---|---|---|
| Google | console.cloud.google.com → APIs & Services → Credentials → OAuth 2.0 Client | `https://rustchess.greenmountain.dev/api/auth/google/callback` |
| GitHub | github.com/settings/developers → OAuth Apps | `https://rustchess.greenmountain.dev/api/auth/github/callback` |
| Discord | discord.com/developers/applications → OAuth2 | `https://rustchess.greenmountain.dev/api/auth/discord/callback` |

Store the following in `chess-server/.env` (never committed):
```
DATABASE_URL=postgres://chess:chess_pass@chess-db:5432/chess
JWT_SECRET=<random 64-char string>
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
DISCORD_CLIENT_ID=...
DISCORD_CLIENT_SECRET=...
FRONTEND_URL=https://rustchess.greenmountain.dev
```

---

## File Map

| File | Action | Responsibility |
|---|---|---|
| `chess-server/Cargo.toml` | Create | Server crate manifest |
| `chess-server/src/main.rs` | Create | Axum app bootstrap, router |
| `chess-server/src/config.rs` | Create | Load env vars into `Config` struct |
| `chess-server/src/db.rs` | Create | PgPool setup, user upsert, session queries |
| `chess-server/src/auth.rs` | Create | OAuth login/callback handlers, JWT issuance |
| `chess-server/src/middleware.rs` | Create | JWT extraction + validation middleware |
| `chess-server/src/models.rs` | Create | `User`, `OAuthState` structs |
| `chess-server/src/error.rs` | Create | Unified `AppError` → Axum response |
| `chess-server/migrations/001_initial.sql` | Create | `users` + `oauth_states` tables |
| `chess-server/Dockerfile` | Create | Multi-stage Rust build |
| `chess-server/.env.example` | Create | Template (committed) |
| `chess-server/.gitignore` | Create | Ignore `.env` |
| `Cargo.toml` (root) | Modify | Add `chess-server` to workspace members |
| `deploy/docker-compose.vps.yml` | Modify | Add `chess-api` + `chess-db` services |
| `~/MyServerVPS/nginx/sites-available/subdomains.greenmountain.dev` | Modify | Add `/api/` proxy block |
| `src/auth.rs` (frontend) | Create | `AuthPlugin`, `UserSession`, localStorage read |
| `src/home.rs` (frontend) | Modify | Wire `BtnOAuth` to real redirect |
| `src/lib.rs` (frontend) | Modify | Register `AuthPlugin` |

---

## Task 1: chess-server crate skeleton

**Files:**
- Create: `chess-server/Cargo.toml`
- Create: `chess-server/src/main.rs`
- Create: `chess-server/src/error.rs`
- Create: `chess-server/src/config.rs`
- Modify: `Cargo.toml` (root)

- [ ] **Step 1: Add chess-server to workspace**

Edit root `Cargo.toml`:
```toml
[workspace]
members = [".", "chess-engine", "chess-server"]
resolver = "2"
```

- [ ] **Step 2: Create `chess-server/Cargo.toml`**

```toml
[package]
name = "chess-server"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "chess-server"
path = "src/main.rs"

[dependencies]
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
jsonwebtoken = "9"
reqwest = { version = "0.12", features = ["json"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tower = "0.5"
tower-http = { version = "0.6", features = ["cors", "trace"] }
dotenvy = "0.15"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

- [ ] **Step 3: Create `chess-server/src/config.rs`**

```rust
#[derive(Clone)]
pub struct Config {
    pub database_url:          String,
    pub jwt_secret:            String,
    pub google_client_id:      String,
    pub google_client_secret:  String,
    pub github_client_id:      String,
    pub github_client_secret:  String,
    pub discord_client_id:     String,
    pub discord_client_secret: String,
    pub frontend_url:          String,
    pub port:                  u16,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();
        Ok(Self {
            database_url:          std::env::var("DATABASE_URL")?,
            jwt_secret:            std::env::var("JWT_SECRET")?,
            google_client_id:      std::env::var("GOOGLE_CLIENT_ID")?,
            google_client_secret:  std::env::var("GOOGLE_CLIENT_SECRET")?,
            github_client_id:      std::env::var("GITHUB_CLIENT_ID")?,
            github_client_secret:  std::env::var("GITHUB_CLIENT_SECRET")?,
            discord_client_id:     std::env::var("DISCORD_CLIENT_ID")?,
            discord_client_secret: std::env::var("DISCORD_CLIENT_SECRET")?,
            frontend_url:          std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8090".into()),
            port:                  std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8006),
        })
    }
}
```

- [ ] **Step 4: Create `chess-server/src/error.rs`**

```rust
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    Unauthorized,
    BadRequest(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            AppError::Unauthorized        => (StatusCode::UNAUTHORIZED,  "Unauthorized".into()),
            AppError::BadRequest(m)       => (StatusCode::BAD_REQUEST,   m),
            AppError::Internal(e)         => {
                tracing::error!("internal error: {e:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".into())
            }
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}

impl<E: Into<anyhow::Error>> From<E> for AppError {
    fn from(e: E) -> Self { AppError::Internal(e.into()) }
}
```

- [ ] **Step 5: Create `chess-server/src/main.rs`**

```rust
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
        .route("/api/health",                    get(|| async { "ok" }))
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
```

- [ ] **Step 6: Verify it compiles (stubs for missing modules)**

Create empty stubs `chess-server/src/{auth,db,middleware,models}.rs`:
```rust
// Each file: just `pub fn placeholder() {}`
```

Run:
```bash
cargo build -p chess-server 2>&1 | tail -5
```
Expected: compiles successfully.

- [ ] **Step 7: Create `chess-server/.gitignore`**
```
.env
target/
```

- [ ] **Step 8: Create `chess-server/.env.example`**
```
DATABASE_URL=postgres://chess:chess_pass@chess-db:5432/chess
JWT_SECRET=replace_with_64_char_random_string
GOOGLE_CLIENT_ID=
GOOGLE_CLIENT_SECRET=
GITHUB_CLIENT_ID=
GITHUB_CLIENT_SECRET=
DISCORD_CLIENT_ID=
DISCORD_CLIENT_SECRET=
FRONTEND_URL=https://rustchess.greenmountain.dev
PORT=8006
```

- [ ] **Step 9: Commit**
```bash
git add chess-server/ Cargo.toml
git commit -m "feat(chess-server): add Axum server crate skeleton"
```

---

## Task 2: Database schema + models

**Files:**
- Create: `chess-server/migrations/001_initial.sql`
- Create: `chess-server/src/models.rs`
- Create: `chess-server/src/db.rs`

- [ ] **Step 1: Create migration**

`chess-server/migrations/001_initial.sql`:
```sql
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE TABLE users (
    id           UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    email        TEXT UNIQUE NOT NULL,
    display_name TEXT NOT NULL,
    avatar_url   TEXT,
    provider     TEXT NOT NULL,  -- 'google' | 'github' | 'discord'
    provider_id  TEXT NOT NULL,
    elo          INTEGER NOT NULL DEFAULT 800,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(provider, provider_id)
);

-- CSRF protection: short-lived state tokens for OAuth flow
CREATE TABLE oauth_states (
    state      TEXT PRIMARY KEY,
    provider   TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Auto-clean expired states (older than 10 minutes)
CREATE INDEX idx_oauth_states_created ON oauth_states(created_at);
```

- [ ] **Step 2: Create `chess-server/src/models.rs`**

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id:           Uuid,
    pub email:        String,
    pub display_name: String,
    pub avatar_url:   Option<String>,
    pub provider:     String,
    pub provider_id:  String,
    pub elo:          i32,
    pub created_at:   DateTime<Utc>,
    pub updated_at:   DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UserPublic {
    pub id:           Uuid,
    pub display_name: String,
    pub avatar_url:   Option<String>,
    pub elo:          i32,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        UserPublic { id: u.id, display_name: u.display_name, avatar_url: u.avatar_url, elo: u.elo }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,   // user UUID as string
    pub exp: usize,    // unix timestamp
}
```

- [ ] **Step 3: Create `chess-server/src/db.rs`**

```rust
use sqlx::PgPool;
use uuid::Uuid;
use crate::models::User;

pub async fn connect(url: &str) -> anyhow::Result<PgPool> {
    Ok(PgPool::connect(url).await?)
}

pub async fn migrate(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn upsert_user(
    pool:        &PgPool,
    email:       &str,
    display_name:&str,
    avatar_url:  Option<&str>,
    provider:    &str,
    provider_id: &str,
) -> anyhow::Result<User> {
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (email, display_name, avatar_url, provider, provider_id)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (provider, provider_id) DO UPDATE
            SET email        = EXCLUDED.email,
                display_name = EXCLUDED.display_name,
                avatar_url   = EXCLUDED.avatar_url,
                updated_at   = NOW()
        RETURNING *
        "#,
        email, display_name, avatar_url, provider, provider_id
    )
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    Ok(sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", id)
        .fetch_optional(pool)
        .await?)
}

pub async fn store_oauth_state(pool: &PgPool, state: &str, provider: &str) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT INTO oauth_states (state, provider) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        state, provider
    )
    .execute(pool)
    .await?;
    // Clean up states older than 10 minutes
    sqlx::query!("DELETE FROM oauth_states WHERE created_at < NOW() - INTERVAL '10 minutes'")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn consume_oauth_state(pool: &PgPool, state: &str, provider: &str) -> anyhow::Result<bool> {
    let result = sqlx::query!(
        "DELETE FROM oauth_states WHERE state = $1 AND provider = $2
         AND created_at > NOW() - INTERVAL '10 minutes'",
        state, provider
    )
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
```

- [ ] **Step 4: Compile check**
```bash
cargo build -p chess-server 2>&1 | tail -5
```
Expected: no errors.

- [ ] **Step 5: Commit**
```bash
git add chess-server/
git commit -m "feat(chess-server): add DB schema, models, connection pool"
```

---

## Task 3: OAuth login redirect endpoints

**Files:**
- Modify: `chess-server/src/auth.rs`

The login handlers generate a CSRF state token, store it in DB, then redirect to the provider's authorization URL.

- [ ] **Step 1: Implement login handlers in `chess-server/src/auth.rs`**

```rust
use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{db, error::AppError, middleware::AuthUser, models::{JwtClaims, UserPublic}, AppState};

// ─── JWT helpers ─────────────────────────────────────────────────────────────

fn issue_jwt(user_id: Uuid, secret: &str) -> Result<String, AppError> {
    let exp = (chrono::Utc::now() + chrono::Duration::days(30)).timestamp() as usize;
    let claims = JwtClaims { sub: user_id.to_string(), exp };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .map_err(|e| AppError::Internal(e.into()))
}

fn new_state() -> String {
    Uuid::new_v4().to_string()
}

// ─── Google ──────────────────────────────────────────────────────────────────

pub async fn google_login(State(app): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let state = new_state();
    db::store_oauth_state(&app.pool, &state, "google").await?;
    let url = format!(
        "https://accounts.google.com/o/oauth2/v2/auth\
         ?client_id={}&redirect_uri={}/api/auth/google/callback\
         &response_type=code&scope=openid%20email%20profile&state={}",
        app.config.google_client_id,
        app.config.frontend_url,
        state
    );
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
pub struct OAuthCallback { code: String, state: String }

#[derive(Deserialize)]
struct GoogleToken { access_token: String }

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

    let token: GoogleToken = http.post("https://oauth2.googleapis.com/token")
        .form(&[
            ("code",          params.code.as_str()),
            ("client_id",     &app.config.google_client_id),
            ("client_secret", &app.config.google_client_secret),
            ("redirect_uri",  &format!("{}/api/auth/google/callback", app.config.frontend_url)),
            ("grant_type",    "authorization_code"),
        ])
        .send().await?.json().await?;

    let profile: GoogleUser = http
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(&token.access_token)
        .send().await?.json().await?;

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
        app.config.github_client_id,
        app.config.frontend_url,
        state
    );
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct GithubToken { access_token: String }

#[derive(Deserialize)]
struct GithubUser {
    id:         i64,
    login:      String,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GithubEmail { email: String, primary: bool, verified: bool }

pub async fn github_callback(
    Query(params): Query<OAuthCallback>,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !db::consume_oauth_state(&app.pool, &params.state, "github").await? {
        return Err(AppError::BadRequest("Invalid or expired state".into()));
    }

    let http = reqwest::Client::new();

    let token: GithubToken = http.post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("code",          params.code.as_str()),
            ("client_id",     &app.config.github_client_id),
            ("client_secret", &app.config.github_client_secret),
            ("redirect_uri",  &format!("{}/api/auth/github/callback", app.config.frontend_url)),
        ])
        .send().await?.json().await?;

    let profile: GithubUser = http
        .get("https://api.github.com/user")
        .bearer_auth(&token.access_token)
        .header("User-Agent", "chess-server/1.0")
        .send().await?.json().await?;

    let emails: Vec<GithubEmail> = http
        .get("https://api.github.com/user/emails")
        .bearer_auth(&token.access_token)
        .header("User-Agent", "chess-server/1.0")
        .send().await?.json().await?;

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
        app.config.discord_client_id,
        app.config.frontend_url,
        state
    );
    Ok(Redirect::temporary(&url))
}

#[derive(Deserialize)]
struct DiscordToken { access_token: String }

#[derive(Deserialize)]
struct DiscordUser {
    id:           String,
    username:     String,
    email:        Option<String>,
    avatar:       Option<String>,
}

pub async fn discord_callback(
    Query(params): Query<OAuthCallback>,
    State(app): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    if !db::consume_oauth_state(&app.pool, &params.state, "discord").await? {
        return Err(AppError::BadRequest("Invalid or expired state".into()));
    }

    let http = reqwest::Client::new();

    let token: DiscordToken = http.post("https://discord.com/api/oauth2/token")
        .form(&[
            ("code",          params.code.as_str()),
            ("client_id",     &app.config.discord_client_id),
            ("client_secret", &app.config.discord_client_secret),
            ("redirect_uri",  &format!("{}/api/auth/discord/callback", app.config.frontend_url)),
            ("grant_type",    "authorization_code"),
        ])
        .send().await?.json().await?;

    let profile: DiscordUser = http
        .get("https://discord.com/api/users/@me")
        .bearer_auth(&token.access_token)
        .send().await?.json().await?;

    let avatar_url = profile.avatar.as_ref().map(|a|
        format!("https://cdn.discordapp.com/avatars/{}/{}.png", profile.id, a)
    );
    let email = profile.email.unwrap_or_else(|| format!("{}@discord.invalid", profile.id));

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
```

- [ ] **Step 2: Compile check**
```bash
cargo build -p chess-server 2>&1 | tail -5
```

- [ ] **Step 3: Commit**
```bash
git add chess-server/src/
git commit -m "feat(chess-server): OAuth login/callback handlers for Google, GitHub, Discord"
```

---

## Task 4: JWT middleware (`/api/me` auth)

**Files:**
- Create: `chess-server/src/middleware.rs`

- [ ] **Step 1: Implement `chess-server/src/middleware.rs`**

```rust
use axum::{
    async_trait,
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use uuid::Uuid;
use crate::{models::JwtClaims, AppState};

pub struct AuthUser { pub user_id: Uuid }

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let header = parts.headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(StatusCode::UNAUTHORIZED)?;

        let data = decode::<JwtClaims>(
            header,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let user_id = Uuid::parse_str(&data.claims.sub)
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        Ok(AuthUser { user_id })
    }
}
```

- [ ] **Step 2: Compile check**
```bash
cargo build -p chess-server 2>&1 | tail -5
```

- [ ] **Step 3: Run server locally (requires .env)**

Create `chess-server/.env` from `.env.example` (use `postgres://` URL pointing to a local or test DB).

```bash
cd chess-server && cargo run 2>&1 &
sleep 2
curl http://localhost:8006/api/health
```
Expected: `ok`

```bash
curl http://localhost:8006/api/me
```
Expected: 401 Unauthorized

- [ ] **Step 4: Commit**
```bash
git add chess-server/src/middleware.rs
git commit -m "feat(chess-server): JWT Bearer middleware for protected routes"
```

---

## Task 5: Docker setup for chess-server

**Files:**
- Create: `chess-server/Dockerfile`
- Modify: `deploy/docker-compose.vps.yml`

- [ ] **Step 1: Create `chess-server/Dockerfile`**

```dockerfile
FROM rust:1.82-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY chess-engine/ chess-engine/
COPY chess-server/ chess-server/
COPY src/ src/
RUN cargo build --release -p chess-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/chess-server /usr/local/bin/chess-server
COPY --from=builder /app/chess-server/migrations /migrations
EXPOSE 8006
CMD ["chess-server"]
```

- [ ] **Step 2: Update `deploy/docker-compose.vps.yml`**

```yaml
services:
  rust-chess:
    image: ghcr.io/matute289/rust-chess:latest
    container_name: rust-chess
    ports:
      - "127.0.0.1:8007:80"
    mem_limit: 32m
    restart: unless-stopped

  chess-api:
    image: ghcr.io/matute289/rust-chess-api:latest
    container_name: chess-api
    ports:
      - "127.0.0.1:8006:8006"
    mem_limit: 128m
    restart: unless-stopped
    env_file: /opt/rust-chess/.env
    depends_on:
      chess-db:
        condition: service_healthy

  chess-db:
    image: postgres:16-alpine
    container_name: chess-db
    volumes:
      - chess-db-data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB:       chess
      POSTGRES_USER:     chess
      POSTGRES_PASSWORD: chess_pass
    mem_limit: 128m
    restart: unless-stopped
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U chess"]
      interval: 5s
      timeout: 5s
      retries: 5

volumes:
  chess-db-data:
```

- [ ] **Step 3: Build Docker image locally to verify**

```bash
docker build -f chess-server/Dockerfile -t chess-api-test . 2>&1 | tail -10
```
Expected: successfully tagged.

- [ ] **Step 4: Commit**
```bash
git add chess-server/Dockerfile deploy/docker-compose.vps.yml
git commit -m "feat(chess-server): Dockerfile + docker-compose chess-api + chess-db services"
```

---

## Task 6: Nginx update (add /api/ proxy block)

**Files:**
- Modify: `~/MyServerVPS/nginx/sites-available/subdomains.greenmountain.dev`

- [ ] **Step 1: Add `/api/` location to the rustchess server block**

Find the existing `rustchess.greenmountain.dev` server block (the one with `listen 443 ssl`) and add the `/api/` location **before** the existing `location /` block:

```nginx
    # Chess API backend
    location /api/ {
        limit_req zone=zone_api burst=30 nodelay;
        proxy_pass         http://127.0.0.1:8006;
        proxy_http_version 1.1;
        proxy_set_header   Host              $host;
        proxy_set_header   X-Real-IP         $remote_addr;
        proxy_set_header   X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header   X-Forwarded-Proto $scheme;
    }
```

Note: `zone_api` already exists (15r/s) from the DDoS protection phase.

- [ ] **Step 2: Test nginx config locally (syntax check)**
```bash
nginx -t -c ~/MyServerVPS/nginx/nginx.conf 2>&1 || echo "test nginx config on VPS instead"
```

- [ ] **Step 3: Commit to MyServerVPS**
```bash
cd ~/MyServerVPS
git add nginx/sites-available/subdomains.greenmountain.dev
git commit -m "feat(nginx): add /api/ proxy to chess-api backend for rustchess"
```

- [ ] **Step 4: Apply on VPS**
```bash
ssh mgrinberg@216.238.126.97 "sudo cp /path/to/updated/subdomains.greenmountain.dev /etc/nginx/sites-available/ && sudo nginx -t && sudo systemctl reload nginx"
```

---

## Task 7: Frontend WASM auth integration

**Files:**
- Create: `src/auth.rs`
- Modify: `src/home.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create `src/auth.rs`**

```rust
use bevy::prelude::*;
use crate::state::AppState;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Resource, Default, Clone)]
pub struct UserSession {
    pub user_id:      Option<String>,
    pub jwt:          Option<String>,
    pub display_name: Option<String>,
    pub elo:          Option<i32>,
}

impl UserSession {
    pub fn is_logged_in(&self) -> bool { self.jwt.is_some() }
}

pub struct AuthPlugin;

impl Plugin for AuthPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UserSession>()
           .add_systems(Startup, load_session_from_storage);
    }
}

fn load_session_from_storage(mut session: ResMut<UserSession>) {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(jwt) = read_jwt_from_url_or_storage() {
            session.jwt = Some(jwt);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn read_jwt_from_url_or_storage() -> Option<String> {
    use web_sys::window;

    let win = window()?;

    // Check URL query param first (?jwt=...)
    let location = win.location();
    if let Ok(search) = location.search() {
        if let Some(jwt) = parse_jwt_from_query(&search) {
            // Store in localStorage and clean URL
            if let Ok(Some(storage)) = win.local_storage() {
                let _ = storage.set_item("chess_jwt", &jwt);
            }
            let _ = location.set_search("");
            return Some(jwt);
        }
    }

    // Fall back to localStorage
    if let Ok(Some(storage)) = win.local_storage() {
        if let Ok(Some(jwt)) = storage.get_item("chess_jwt") {
            return Some(jwt);
        }
    }

    None
}

#[cfg(target_arch = "wasm32")]
fn parse_jwt_from_query(search: &str) -> Option<String> {
    let query = search.trim_start_matches('?');
    for part in query.split('&') {
        if let Some(val) = part.strip_prefix("jwt=") {
            return Some(val.to_string());
        }
    }
    None
}
```

- [ ] **Step 2: Add `web-sys` features to `Cargo.toml`**

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
getrandom = { version = "0.3", features = ["wasm_js"] }
web-sys = { version = "0.3", features = [
    "Window", "Location", "Storage"
] }
```

- [ ] **Step 3: Register AuthPlugin in `src/lib.rs`**

```rust
// Add near the top imports:
mod auth;
use auth::AuthPlugin;

// In the App builder:
.add_plugins(AuthPlugin)
```

- [ ] **Step 4: Update BtnOAuth in `src/home.rs`**

Replace the stub `handle_oauth` function:

```rust
fn handle_oauth(
    q: Query<(&Interaction, &BtnOAuth), Changed<Interaction>>,
    mut home_screen: ResMut<HomeScreen>,
    mut commands: Commands,
    root_q: Query<Entity, With<HomeRoot>>,
    asset_server: Res<AssetServer>,
    timer_idx: Res<SelectedTimerIdx>,
) {
    for (i, btn) in &q {
        if *i == Interaction::Pressed {
            #[cfg(target_arch = "wasm32")]
            {
                let provider = btn.0.to_lowercase();
                let url = format!("https://rustchess.greenmountain.dev/api/auth/{}/login", provider);
                if let Some(win) = web_sys::window() {
                    let _ = win.location().set_href(&url);
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                // Desktop dev: just go to ColorSelect as before (stub)
                *home_screen = HomeScreen::ColorSelect;
                rebuild_home(&mut commands, &asset_server, &root_q, HomeScreen::ColorSelect, timer_idx.0);
            }
        }
    }
}
```

- [ ] **Step 5: Show username in HUD when logged in**

In `src/ui.rs`, update `spawn_hud` to conditionally show user display name:

```rust
fn spawn_hud(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    config: Res<GameConfig>,
    session: Res<crate::auth::UserSession>,  // add this param
) {
    // ... existing code ...
    // After mode_label spawn, add:
    if let Some(name) = &session.display_name {
        parent.spawn(TextBundle::from_section(
            format!("Jugador: {}", name),
            TextStyle { font: font.clone(), font_size: 18.0, color: Color::rgb(0.5, 0.8, 0.5) },
        ));
    }
}
```

- [ ] **Step 6: Compile check (WASM)**
```bash
cargo check --target wasm32-unknown-unknown 2>&1 | grep "^error" | head -10
```
Expected: no errors.

- [ ] **Step 7: Commit**
```bash
git add src/auth.rs src/home.rs src/lib.rs src/ui.rs Cargo.toml
git commit -m "feat(frontend): AuthPlugin loads JWT from URL/localStorage, shows username in HUD"
```

---

## Task 8: CI/CD — build and deploy chess-api

**Files:**
- Create: `.github/workflows/deploy-chess-api.yml`

- [ ] **Step 1: Create GitHub Actions workflow**

`.github/workflows/deploy-chess-api.yml`:
```yaml
name: Deploy chess-api

on:
  push:
    branches: [browser]
    paths:
      - 'chess-server/**'
      - 'deploy/docker-compose.vps.yml'
      - '.github/workflows/deploy-chess-api.yml'

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository_owner }}/rust-chess-api

jobs:
  build-and-deploy:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write

    steps:
      - uses: actions/checkout@v4

      - name: Log in to GHCR
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          file: chess-server/Dockerfile
          push: true
          tags: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:latest

      - name: Deploy to VPS
        uses: appleboy/ssh-action@v1
        with:
          host: ${{ secrets.VPS_HOST }}
          username: ${{ secrets.VPS_USER }}
          key: ${{ secrets.VPS_SSH_KEY }}
          script: |
            cd /opt/rust-chess
            docker compose pull chess-api
            docker compose up -d chess-api chess-db
            docker compose ps
```

- [ ] **Step 2: Create `/opt/rust-chess/.env` on VPS (with real credentials)**

```bash
ssh mgrinberg@216.238.126.97
mkdir -p /opt/rust-chess
cat > /opt/rust-chess/.env << 'EOF'
DATABASE_URL=postgres://chess:chess_pass@chess-db:5432/chess
JWT_SECRET=<generate with: openssl rand -hex 32>
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GITHUB_CLIENT_ID=...
GITHUB_CLIENT_SECRET=...
DISCORD_CLIENT_ID=...
DISCORD_CLIENT_SECRET=...
FRONTEND_URL=https://rustchess.greenmountain.dev
PORT=8006
EOF
chmod 600 /opt/rust-chess/.env
```

- [ ] **Step 3: Push and trigger deploy**

```bash
git add .github/workflows/deploy-chess-api.yml
git commit -m "ci: add GitHub Actions workflow to build and deploy chess-api"
git push
```

- [ ] **Step 4: Verify deploy on VPS**

```bash
ssh mgrinberg@216.238.126.97 "docker ps | grep chess"
curl https://rustchess.greenmountain.dev/api/health
```
Expected: `ok`

---

## Task 9: End-to-end OAuth test

- [ ] **Step 1: Test Google OAuth flow**
  1. Open `https://rustchess.greenmountain.dev`
  2. Click "Player VS Learning" → "Iniciar sesión" → "Continuar con Google"
  3. Complete Google login
  4. Verify redirect back to `rustchess.greenmountain.dev`
  5. Verify JWT in `localStorage` (DevTools → Application → Local Storage)
  6. Verify username appears in HUD when starting a game

- [ ] **Step 2: Test GitHub OAuth flow** — same steps with GitHub

- [ ] **Step 3: Test Discord OAuth flow** — same steps with Discord

- [ ] **Step 4: Test session persistence**
  1. Close browser, reopen `rustchess.greenmountain.dev`
  2. Verify still logged in (JWT in localStorage, no redirect)

- [ ] **Step 5: Test /api/me with JWT**

```bash
JWT=$(curl -s https://rustchess.greenmountain.dev/api/health)  # get a real JWT from DevTools
curl -H "Authorization: Bearer $JWT" https://rustchess.greenmountain.dev/api/me
```
Expected: JSON with `id`, `display_name`, `elo`.

---

## Success Criteria

- [ ] `cargo build -p chess-server` passes
- [ ] `cargo check --target wasm32-unknown-unknown` passes (frontend changes)
- [ ] `https://rustchess.greenmountain.dev/api/health` returns `ok`
- [ ] All three OAuth providers complete flow and return to the app with JWT
- [ ] JWT persists in localStorage across browser sessions
- [ ] Logged-in username appears in game HUD
- [ ] `/api/me` returns correct user data
- [ ] chess-db data persists across container restarts (volume mounted)
