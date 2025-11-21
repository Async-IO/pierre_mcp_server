-- ABOUTME: API keys schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates api_keys and api_key_usage tables with indexes

CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    key_hash TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    tier TEXT NOT NULL CHECK (tier IN ('trial', 'starter', 'professional', 'enterprise')),
    rate_limit_requests INTEGER,
    rate_limit_window_seconds INTEGER,
    is_active INTEGER NOT NULL DEFAULT 1,
    expires_at TEXT,
    last_used_at TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(key_hash),
    UNIQUE(key_prefix)
);

CREATE TABLE IF NOT EXISTS api_key_usage (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    error_message TEXT,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT
);

-- Indexes for API keys
CREATE INDEX IF NOT EXISTS idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX IF NOT EXISTS idx_api_keys_key_prefix ON api_keys(key_prefix);
CREATE INDEX IF NOT EXISTS idx_api_key_usage_key_id ON api_key_usage(api_key_id);
CREATE INDEX IF NOT EXISTS idx_api_key_usage_timestamp ON api_key_usage(timestamp);
