-- ABOUTME: OAuth2 server schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for RFC 7591 OAuth 2.0 server implementation including client registration, auth codes, refresh tokens, and state management

-- OAuth2 Clients Table
CREATE TABLE IF NOT EXISTS oauth2_clients (
    id TEXT PRIMARY KEY,
    client_id TEXT UNIQUE NOT NULL,
    client_secret_hash TEXT NOT NULL,
    redirect_uris TEXT NOT NULL, -- JSON array
    grant_types TEXT NOT NULL,   -- JSON array
    response_types TEXT NOT NULL, -- JSON array
    client_name TEXT,
    client_uri TEXT,
    scope TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT
);

-- OAuth2 Authorization Codes Table
CREATE TABLE IF NOT EXISTS oauth2_auth_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT,
    expires_at TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0,
    state TEXT,
    code_challenge TEXT,
    code_challenge_method TEXT,
    FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
);

-- OAuth2 Refresh Tokens Table
CREATE TABLE IF NOT EXISTS oauth2_refresh_tokens (
    token TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    scope TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
);

-- OAuth2 States Table (CSRF Protection)
CREATE TABLE IF NOT EXISTS oauth2_states (
    state TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT,
    tenant_id TEXT,
    redirect_uri TEXT NOT NULL,
    scope TEXT,
    code_challenge TEXT,
    code_challenge_method TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    used INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
);

-- Indexes for OAuth2 Tables
CREATE INDEX IF NOT EXISTS idx_oauth2_clients_client_id ON oauth2_clients(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth2_auth_codes_code ON oauth2_auth_codes(code);
CREATE INDEX IF NOT EXISTS idx_oauth2_auth_codes_expires_at ON oauth2_auth_codes(expires_at);
CREATE INDEX IF NOT EXISTS idx_oauth2_auth_codes_tenant_user ON oauth2_auth_codes(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_oauth2_refresh_tokens_token ON oauth2_refresh_tokens(token);
CREATE INDEX IF NOT EXISTS idx_oauth2_refresh_tokens_tenant_user ON oauth2_refresh_tokens(tenant_id, user_id);
CREATE INDEX IF NOT EXISTS idx_oauth2_refresh_tokens_user_id ON oauth2_refresh_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_oauth2_states_state ON oauth2_states(state);
CREATE INDEX IF NOT EXISTS idx_oauth2_states_expires_at ON oauth2_states(expires_at);
