CREATE TABLE IF NOT EXISTS content_reports (
    id TEXT PRIMARY KEY,
    visitor_id TEXT NOT NULL,
    reason TEXT NOT NULL,
    excerpt TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS content_reports_created_at
    ON content_reports (created_at DESC);
