-- ABOUTME: Admin schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for admin token management, usage tracking, provisioned keys, system secrets, and RSA keypairs

-- Admin Tokens Table
CREATE TABLE IF NOT EXISTS admin_tokens (
    id TEXT PRIMARY KEY,
    service_name TEXT NOT NULL,
    service_description TEXT,
    token_hash TEXT NOT NULL,
    token_prefix TEXT NOT NULL,
    jwt_secret_hash TEXT NOT NULL,
    permissions TEXT NOT NULL DEFAULT '["provision_keys"]',
    is_super_admin INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    expires_at TEXT,
    last_used_at TEXT,
    last_used_ip TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0
);

-- Admin Token Usage Table
CREATE TABLE IF NOT EXISTS admin_token_usage (
    id TEXT PRIMARY KEY,
    admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
    timestamp TEXT NOT NULL,
    action TEXT NOT NULL,
    target_resource TEXT,
    ip_address TEXT,
    user_agent TEXT,
    request_size_bytes INTEGER,
    success INTEGER NOT NULL,
    error_message TEXT,
    response_time_ms INTEGER
);

-- Admin Provisioned Keys Table
CREATE TABLE IF NOT EXISTS admin_provisioned_keys (
    id TEXT PRIMARY KEY,
    admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
    api_key_id TEXT NOT NULL,
    user_email TEXT NOT NULL,
    requested_tier TEXT NOT NULL,
    provisioned_at TEXT NOT NULL,
    provisioned_by_service TEXT NOT NULL,
    rate_limit_requests INTEGER NOT NULL,
    rate_limit_period TEXT NOT NULL,
    key_status TEXT NOT NULL DEFAULT 'active',
    revoked_at TEXT,
    revoked_reason TEXT
);

-- System Secrets Table
CREATE TABLE IF NOT EXISTS system_secrets (
    secret_type TEXT PRIMARY KEY,
    secret_value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- RSA Keypairs Table
CREATE TABLE IF NOT EXISTS rsa_keypairs (
    kid TEXT PRIMARY KEY,
    private_key_pem TEXT NOT NULL,
    public_key_pem TEXT NOT NULL,
    created_at TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 0,
    key_size_bits INTEGER NOT NULL
);

-- Indexes for Admin Tables
CREATE INDEX IF NOT EXISTS idx_admin_tokens_service ON admin_tokens(service_name);
CREATE INDEX IF NOT EXISTS idx_admin_tokens_prefix ON admin_tokens(token_prefix);
CREATE INDEX IF NOT EXISTS idx_admin_usage_token_id ON admin_token_usage(admin_token_id);
CREATE INDEX IF NOT EXISTS idx_admin_usage_timestamp ON admin_token_usage(timestamp);
CREATE INDEX IF NOT EXISTS idx_admin_provisioned_token ON admin_provisioned_keys(admin_token_id);
CREATE INDEX IF NOT EXISTS idx_system_secrets_type ON system_secrets(secret_type);
CREATE INDEX IF NOT EXISTS idx_rsa_keypairs_active ON rsa_keypairs(is_active);
