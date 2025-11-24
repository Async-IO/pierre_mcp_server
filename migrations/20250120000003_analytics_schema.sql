-- ABOUTME: Analytics schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates jwt_usage, goals, insights, and request_logs tables with indexes

CREATE TABLE IF NOT EXISTS jwt_usage (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    method TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT
);

CREATE TABLE IF NOT EXISTS goals (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    goal_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS insights (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    activity_id TEXT,
    insight_type TEXT NOT NULL,
    insight_data TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS request_logs (
    id TEXT PRIMARY KEY,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    api_key_id TEXT REFERENCES api_keys(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    method TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    error_message TEXT
);

-- Indexes for analytics tables
CREATE INDEX IF NOT EXISTS idx_jwt_usage_user_id ON jwt_usage(user_id);
CREATE INDEX IF NOT EXISTS idx_jwt_usage_timestamp ON jwt_usage(timestamp);
CREATE INDEX IF NOT EXISTS idx_goals_user_id ON goals(user_id);
CREATE INDEX IF NOT EXISTS idx_insights_user_id ON insights(user_id);
CREATE INDEX IF NOT EXISTS idx_insights_activity_id ON insights(activity_id);
CREATE INDEX IF NOT EXISTS idx_request_logs_timestamp ON request_logs(timestamp);
