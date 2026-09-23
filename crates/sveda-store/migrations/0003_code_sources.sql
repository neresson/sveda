CREATE TABLE IF NOT EXISTS code_sources (
    id BIGINT PRIMARY KEY,
    document JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
