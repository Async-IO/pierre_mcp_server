-- ABOUTME: A2A (Agent-to-Agent) schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for A2A client registration, sessions, tasks, usage tracking, and API key associations

-- A2A Clients Table
CREATE TABLE IF NOT EXISTS a2a_clients (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    public_key TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    permissions TEXT NOT NULL,
    capabilities TEXT NOT NULL DEFAULT '[]',
    redirect_uris TEXT NOT NULL DEFAULT '[]',
    rate_limit_requests INTEGER NOT NULL DEFAULT 1000,
    rate_limit_window_seconds INTEGER NOT NULL DEFAULT 3600,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(public_key)
);

-- A2A Sessions Table
CREATE TABLE IF NOT EXISTS a2a_sessions (
    session_token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    granted_scopes TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_activity TEXT NOT NULL,
    created_at TEXT NOT NULL,
    requests_count INTEGER NOT NULL DEFAULT 0
);

-- A2A Tasks Table
CREATE TABLE IF NOT EXISTS a2a_tasks (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
    task_type TEXT NOT NULL,
    input_data TEXT NOT NULL,
    output_data TEXT,
    status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'cancelled')),
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT
);

-- A2A Usage Table
CREATE TABLE IF NOT EXISTS a2a_usage (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
    session_token TEXT REFERENCES a2a_sessions(session_token) ON DELETE SET NULL,
    timestamp TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code INTEGER NOT NULL,
    error_message TEXT,
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT,
    protocol_version TEXT NOT NULL DEFAULT '1.0',
    client_capabilities TEXT NOT NULL DEFAULT '[]',
    granted_scopes TEXT NOT NULL DEFAULT '[]'
);

-- A2A Client API Keys Junction Table
CREATE TABLE IF NOT EXISTS a2a_client_api_keys (
    client_id TEXT NOT NULL REFERENCES a2a_clients(id) ON DELETE CASCADE,
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    PRIMARY KEY (client_id, api_key_id)
);

-- Indexes for A2A Tables
CREATE INDEX IF NOT EXISTS idx_a2a_sessions_client_id ON a2a_sessions(client_id);
CREATE INDEX IF NOT EXISTS idx_a2a_sessions_expires_at ON a2a_sessions(expires_at);
CREATE INDEX IF NOT EXISTS idx_a2a_tasks_client_id ON a2a_tasks(client_id);
CREATE INDEX IF NOT EXISTS idx_a2a_tasks_status ON a2a_tasks(status);
CREATE INDEX IF NOT EXISTS idx_a2a_usage_client_id ON a2a_usage(client_id);
CREATE INDEX IF NOT EXISTS idx_a2a_usage_timestamp ON a2a_usage(timestamp);
CREATE INDEX IF NOT EXISTS idx_a2a_client_api_keys_api_key_id ON a2a_client_api_keys(api_key_id);
