---
name: backend-auth
description: Implementing authentication in the Axum backend — OAuth2 (Google/Facebook/Apple), JWT access tokens, refresh tokens, session management, rate limiting. Use when working on backend/src/routes/auth.rs or any auth middleware.
---

# Backend Auth — rust-chess

Auth lives in `backend/src/`. Stack: Axum 0.7, SQLx, PostgreSQL, `oauth2` crate, `jsonwebtoken` crate.

## Token Architecture

```
Client                          Backend
  │                               │
  ├─ POST /auth/oauth/google ─────► exchange code → Google token → get user info
  │                               │ upsert user in DB
  │  ◄─── { access_token, refresh_token } ──────────────────────────────┤
  │                               │
  ├─ GET /api/* + Bearer <access_token> ──────────────────────────────►  │
  │                               │ JWT middleware extracts user_id
  │                               │ access_token TTL: 15 minutes
  │                               │
  ├─ POST /auth/refresh + refresh_token ──────────────────────────────►  │
  │  ◄─── { new_access_token, new_refresh_token } ───────────────────────┤
  │                               │ refresh_token TTL: 30 days
  │                               │ rotation: old refresh_token invalidated on use
```

## Database Schema (Auth-Related)

```sql
CREATE TABLE users (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email       TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    provider    TEXT NOT NULL,   -- 'google' | 'facebook' | 'apple' | 'email'
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE refresh_tokens (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id     UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token_hash  TEXT NOT NULL UNIQUE,   -- bcrypt hash of the token
    expires_at  TIMESTAMPTZ NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    used_at     TIMESTAMPTZ             -- set on rotation, then deleted
);
```

Never store the raw refresh token — store a hash. Compare with `bcrypt::verify`.

## JWT Claims

```rust
#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,          // user_id
    pub exp: i64,           // unix timestamp (15 min from now)
    pub iat: i64,           // issued at
}
```

Sign with `HS256` using `JWT_SECRET` env var (min 32 bytes, random). Verify on every protected request via Axum middleware.

## Axum Middleware Pattern

```rust
// Extractor — use in any handler that needs auth
pub struct AuthUser(pub Uuid);  // user_id

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, AppError> {
        let bearer = parts.headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let claims = verify_jwt(bearer)?;
        Ok(AuthUser(claims.sub))
    }
}

// Usage in any protected handler:
async fn get_profile(AuthUser(user_id): AuthUser, State(db): State<Pool>) -> impl IntoResponse {
    // user_id is verified
}
```

## OAuth2 Flow (Google Example)

```
1. Client requests: GET /auth/oauth/google/url
   → Backend returns Google authorization URL with state parameter (CSRF)

2. User authenticates with Google
   → Google redirects to: /auth/oauth/google/callback?code=...&state=...

3. Backend: POST https://oauth2.googleapis.com/token (exchange code)
   → Gets id_token + access_token from Google

4. Backend: decode id_token (JWT, verify with Google's public keys)
   → Extract email, name, sub (Google user ID)

5. Backend: upsert user in DB (email = unique key)
   → Generate our own JWT + refresh_token
   → Return to client
```

Use the `oauth2` crate for the code exchange. Verify `state` parameter to prevent CSRF.

Required env vars:
```
GOOGLE_CLIENT_ID=...
GOOGLE_CLIENT_SECRET=...
GOOGLE_REDIRECT_URI=https://chess.greenmountain.dev/auth/oauth/google/callback
JWT_SECRET=<32+ random bytes, base64>
```

## Refresh Token Rotation

```rust
async fn refresh(
    State(db): State<Pool>,
    Json(body): Json<RefreshRequest>,
) -> Result<Json<TokenResponse>, AppError> {
    // 1. Find token in DB by hash
    let stored = sqlx::query!(...)
        .fetch_optional(&db).await?
        .ok_or(AppError::Unauthorized)?;   // token not found = invalid

    // 2. Check not expired
    if stored.expires_at < Utc::now() {
        return Err(AppError::Unauthorized);
    }

    // 3. Verify hash
    bcrypt::verify(&body.refresh_token, &stored.token_hash)?;

    // 4. Delete old token (rotation)
    sqlx::query!("DELETE FROM refresh_tokens WHERE id = $1", stored.id)
        .execute(&db).await?;

    // 5. Issue new pair
    let (access, refresh) = generate_tokens(stored.user_id, &db).await?;
    Ok(Json(TokenResponse { access_token: access, refresh_token: refresh }))
}
```

## Rate Limiting

Use `tower_governor` (wraps `governor` crate) on auth endpoints:

```rust
let governor = GovernorConfigBuilder::default()
    .per_second(2)          // 2 requests per second
    .burst_size(5)          // allow bursts of 5
    .use_headers()
    .finish().unwrap();

Router::new()
    .route("/auth/oauth/google/callback", post(oauth_callback))
    .layer(GovernorLayer { config: Arc::new(governor) })
```

Rate limit all `/auth/*` routes. For general API: 60 req/min per user (use `AuthUser` as the key).

## Anti-Cheat (Move Validation)

All moves received by the backend must be validated server-side:

```rust
async fn submit_move(
    AuthUser(user_id): AuthUser,
    State(db): State<Pool>,
    Path(game_id): Path<Uuid>,
    Json(body): Json<MoveRequest>,
) -> Result<Json<GameState>, AppError> {
    let game = load_game(&db, game_id, user_id).await?;

    // Validate using chess-engine
    let pos = Position::from_fen(&game.fen)?;
    let legal = pos.legal_moves();
    let m = Move::from_uci(&body.uci)?;

    if !legal.contains(&m) {
        return Err(AppError::IllegalMove);
    }

    // Apply and save
    let new_pos = pos.make_move(m);
    update_game(&db, game_id, &new_pos.to_fen(), m).await?;
    Ok(Json(new_pos.into()))
}
```

Never trust the client's FEN. Always load from DB and recompute.

## Security Checklist

Before shipping auth:
- [ ] JWT secret is at least 32 random bytes, stored in env var (not in code)
- [ ] Refresh tokens stored as bcrypt hashes (never plaintext)
- [ ] OAuth `state` parameter verified (CSRF protection)
- [ ] Rate limiting on all `/auth/*` endpoints
- [ ] HTTPS only (nginx terminates TLS)
- [ ] `SameSite=Strict` if using cookies (WASM uses Authorization header instead)
- [ ] Move validation is server-side in all game endpoints
- [ ] `email` uniqueness constraint in DB
