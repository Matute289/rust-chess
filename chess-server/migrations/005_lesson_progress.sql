CREATE TABLE lesson_progress (
    user_id    UUID     NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    lesson_idx INT      NOT NULL CHECK (lesson_idx BETWEEN 0 AND 4),
    stars      SMALLINT NOT NULL DEFAULT 0 CHECK (stars BETWEEN 0 AND 2),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, lesson_idx)
);
