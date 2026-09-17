CREATE TABLE IF NOT EXISTS histories (
    visitor_id TEXT NOT NULL,
    chat_id TEXT NOT NULL,
    title TEXT NOT NULL,
    preview TEXT NOT NULL,
    messages JSONB NOT NULL,
    conversation_history JSONB NOT NULL,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    summary TEXT,
    PRIMARY KEY (visitor_id, chat_id)
);

CREATE INDEX IF NOT EXISTS histories_visitor_updated
    ON histories (visitor_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS settings (
    id SMALLINT PRIMARY KEY CHECK (id = 1),
    document JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
