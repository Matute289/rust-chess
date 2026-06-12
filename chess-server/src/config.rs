use anyhow::Context;

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
            database_url:          std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?,
            jwt_secret:            std::env::var("JWT_SECRET").context("JWT_SECRET must be set")?,
            google_client_id:      std::env::var("GOOGLE_CLIENT_ID").context("GOOGLE_CLIENT_ID must be set")?,
            google_client_secret:  std::env::var("GOOGLE_CLIENT_SECRET").context("GOOGLE_CLIENT_SECRET must be set")?,
            github_client_id:      std::env::var("GITHUB_CLIENT_ID").context("GITHUB_CLIENT_ID must be set")?,
            github_client_secret:  std::env::var("GITHUB_CLIENT_SECRET").context("GITHUB_CLIENT_SECRET must be set")?,
            discord_client_id:     std::env::var("DISCORD_CLIENT_ID").context("DISCORD_CLIENT_ID must be set")?,
            discord_client_secret: std::env::var("DISCORD_CLIENT_SECRET").context("DISCORD_CLIENT_SECRET must be set")?,
            frontend_url:          std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:8090".into()),
            port:                  std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8005),
        })
    }
}
