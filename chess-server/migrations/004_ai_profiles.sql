CREATE TABLE ai_profiles (
    user_id         UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    elo_estimate    INTEGER NOT NULL DEFAULT 800,
    loss_count      INTEGER NOT NULL DEFAULT 0,
    learned_biases  JSONB   NOT NULL DEFAULT '[]',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
