-- ABOUTME: Tenant management schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates tables for multi-tenant architecture including tenants, OAuth credentials, OAuth apps, key versions, audit events, tenant users, and user configurations
--
-- FOLLOW-UP REQUIRED (ASY-176): The legacy `users.tenant_id` column should be dropped
-- in a future migration. The `tenant_users` junction table is now the source of truth
-- for user-tenant relationships. See tenant_users table below for the new model.

-- Tenants Table
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL,
    domain TEXT UNIQUE,
    plan TEXT NOT NULL DEFAULT 'starter' CHECK (plan IN ('starter', 'professional', 'enterprise')),
    owner_user_id TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Tenant OAuth Credentials Table
CREATE TABLE IF NOT EXISTS tenant_oauth_credentials (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('strava', 'garmin', 'fitbit', 'whoop', 'terra')),
    client_id TEXT NOT NULL,
    client_secret_encrypted TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL, -- JSON array
    rate_limit_per_day INTEGER DEFAULT 1000,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(tenant_id, provider)
);

-- OAuth Apps Table
CREATE TABLE IF NOT EXISTS oauth_apps (
    id TEXT PRIMARY KEY,
    client_id TEXT UNIQUE NOT NULL,
    client_secret_hash TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    redirect_uris TEXT NOT NULL, -- JSON array
    scopes TEXT NOT NULL, -- JSON array
    app_type TEXT NOT NULL DEFAULT 'public' CHECK (app_type IN ('public', 'confidential')),
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Key Versions Table
CREATE TABLE IF NOT EXISTS key_versions (
    id TEXT PRIMARY KEY,
    tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
    version INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM',
    UNIQUE(tenant_id, version)
);

-- Audit Events Table
CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    severity TEXT NOT NULL,
    message TEXT NOT NULL,
    source TEXT NOT NULL,
    result TEXT NOT NULL,
    tenant_id TEXT REFERENCES tenants(id) ON DELETE CASCADE,
    user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    ip_address TEXT,
    user_agent TEXT,
    metadata TEXT, -- JSON
    timestamp TEXT NOT NULL
);

-- Tenant Users Table
CREATE TABLE IF NOT EXISTS tenant_users (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'admin', 'member', 'viewer')),
    permissions TEXT, -- JSON array of specific permissions
    invited_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    invited_at TEXT NOT NULL,
    joined_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    UNIQUE(tenant_id, user_id)
);

-- User Configurations Table
CREATE TABLE IF NOT EXISTS user_configurations (
    user_id TEXT PRIMARY KEY,
    config_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Indexes for Tenant Management Tables
CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
CREATE INDEX IF NOT EXISTS idx_tenants_owner ON tenants(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_tenant ON tenant_oauth_credentials(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_provider ON tenant_oauth_credentials(provider);
CREATE INDEX IF NOT EXISTS idx_key_versions_tenant ON key_versions(tenant_id);
CREATE INDEX IF NOT EXISTS idx_key_versions_active ON key_versions(tenant_id, is_active);
CREATE INDEX IF NOT EXISTS idx_audit_events_tenant ON audit_events(tenant_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp ON audit_events(timestamp);
CREATE INDEX IF NOT EXISTS idx_tenant_users_tenant ON tenant_users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_users_user ON tenant_users(user_id);
CREATE INDEX IF NOT EXISTS idx_oauth_apps_client_id ON oauth_apps(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_apps_owner ON oauth_apps(owner_user_id);
