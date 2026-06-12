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
    pool:         &PgPool,
    email:        &str,
    display_name: &str,
    avatar_url:   Option<&str>,
    provider:     &str,
    provider_id:  &str,
) -> anyhow::Result<User> {
    let user = sqlx::query_as::<_, User>(
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
    )
    .bind(email)
    .bind(display_name)
    .bind(avatar_url)
    .bind(provider)
    .bind(provider_id)
    .fetch_one(pool)
    .await?;
    Ok(user)
}

pub async fn find_user_by_id(pool: &PgPool, id: Uuid) -> anyhow::Result<Option<User>> {
    Ok(
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn insert_game_and_update_elo(
    pool:    &PgPool,
    user_id: Uuid,
    payload: &crate::games::GamePayload,
    new_elo: i32,
) -> anyhow::Result<Uuid> {
    let mut tx = pool.begin().await?;

    let game_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO games (
            user_id, mode, result, opponent_elo,
            accuracy_white, accuracy_black,
            blunders_white, blunders_black,
            mistakes_white, mistakes_black,
            inaccuracies_white, inaccuracies_black,
            moves_uci
        ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .bind(&payload.mode)
    .bind(&payload.result)
    .bind(payload.opponent_elo)
    .bind(payload.accuracy_white)
    .bind(payload.accuracy_black)
    .bind(payload.blunders[0] as i16)
    .bind(payload.blunders[1] as i16)
    .bind(payload.mistakes[0] as i16)
    .bind(payload.mistakes[1] as i16)
    .bind(payload.inaccuracies[0] as i16)
    .bind(payload.inaccuracies[1] as i16)
    .bind(&payload.moves_uci)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("UPDATE users SET elo = $1, updated_at = NOW() WHERE id = $2")
        .bind(new_elo)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(game_id)
}

pub async fn store_oauth_state(pool: &PgPool, state: &str, provider: &str) -> anyhow::Result<()> {
    sqlx::query("INSERT INTO oauth_states (state, provider) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(state)
        .bind(provider)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM oauth_states WHERE created_at < NOW() - INTERVAL '10 minutes'")
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn consume_oauth_state(pool: &PgPool, state: &str, provider: &str) -> anyhow::Result<bool> {
    let result = sqlx::query(
        "DELETE FROM oauth_states WHERE state = $1 AND provider = $2
         AND created_at > NOW() - INTERVAL '10 minutes'",
    )
    .bind(state)
    .bind(provider)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}
