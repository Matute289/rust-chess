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
    pub sub: String,
    pub exp: usize,
}
