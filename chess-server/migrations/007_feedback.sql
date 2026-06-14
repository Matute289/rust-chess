CREATE TABLE feedback (
    id         SERIAL PRIMARY KEY,
    message    TEXT NOT NULL,
    context    TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
