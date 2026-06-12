CREATE TABLE games (
    id                  UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id             UUID NOT NULL REFERENCES users(id),
    mode                TEXT NOT NULL,
    result              TEXT NOT NULL,
    opponent_elo        INTEGER,
    accuracy_white      REAL,
    accuracy_black      REAL,
    blunders_white      SMALLINT NOT NULL DEFAULT 0,
    blunders_black      SMALLINT NOT NULL DEFAULT 0,
    mistakes_white      SMALLINT NOT NULL DEFAULT 0,
    mistakes_black      SMALLINT NOT NULL DEFAULT 0,
    inaccuracies_white  SMALLINT NOT NULL DEFAULT 0,
    inaccuracies_black  SMALLINT NOT NULL DEFAULT 0,
    moves_uci           TEXT,
    played_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_games_user_id   ON games(user_id);
CREATE INDEX idx_games_played_at ON games(played_at);
