CREATE TABLE IF NOT EXISTS usage_requests (
    id TEXT PRIMARY KEY,
    visitor_id TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    model TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL,
    prompt_tokens BIGINT NOT NULL DEFAULT 0,
    completion_tokens BIGINT NOT NULL DEFAULT 0,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS usage_requests_created_at
    ON usage_requests (created_at DESC);

CREATE INDEX IF NOT EXISTS usage_requests_model
    ON usage_requests (model, created_at DESC);
