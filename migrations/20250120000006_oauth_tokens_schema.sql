-- ABOUTME: User OAuth tokens schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates table for per-user, per-tenant OAuth credential storage with multi-tenant isolation

-- User OAuth Tokens Table
CREATE TABLE IF NOT EXISTS user_oauth_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    access_token TEXT NOT NULL,
    refresh_token TEXT,
    token_type TEXT NOT NULL DEFAULT 'bearer',
    expires_at TEXT,
    scope TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, tenant_id, provider)
);

-- Indexes for User OAuth Tokens
CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_user ON user_oauth_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_tenant_provider ON user_oauth_tokens(tenant_id, provider);
