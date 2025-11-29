-- ABOUTME: Migration to expand OAuth provider CHECK constraints to support all providers
-- ABOUTME: Adds garmin, whoop, terra to user_oauth_app_credentials and tenant_oauth_credentials

-- For SQLite: Recreate tables with expanded CHECK constraints
-- For PostgreSQL: This also works (CREATE TABLE IF NOT EXISTS is idempotent for existing tables)

-- Step 1: Expand user_oauth_app_credentials provider constraint
-- SQLite doesn't support ALTER TABLE to modify CHECK constraints, so we recreate

-- Create new table with expanded constraint
CREATE TABLE IF NOT EXISTS user_oauth_app_credentials_new (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('strava', 'fitbit', 'garmin', 'whoop', 'terra')),
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, provider)
);

-- Copy existing data (if any)
INSERT OR IGNORE INTO user_oauth_app_credentials_new
SELECT id, user_id, provider, client_id, client_secret, redirect_uri, created_at, updated_at
FROM user_oauth_app_credentials;

-- Drop old table
DROP TABLE IF EXISTS user_oauth_app_credentials;

-- Rename new table
ALTER TABLE user_oauth_app_credentials_new RENAME TO user_oauth_app_credentials;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_user_oauth_apps_user ON user_oauth_app_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_apps_provider ON user_oauth_app_credentials(provider);
CREATE INDEX IF NOT EXISTS idx_user_oauth_apps_user_provider ON user_oauth_app_credentials(user_id, provider);


-- Step 2: Expand tenant_oauth_credentials provider constraint

-- Create new table with expanded constraint
CREATE TABLE IF NOT EXISTS tenant_oauth_credentials_new (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('strava', 'fitbit', 'garmin', 'whoop', 'terra')),
    client_id TEXT NOT NULL,
    client_secret_encrypted TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL,
    rate_limit_per_day INTEGER DEFAULT 1000,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(tenant_id, provider)
);

-- Copy existing data (if any)
INSERT OR IGNORE INTO tenant_oauth_credentials_new
SELECT id, tenant_id, provider, client_id, client_secret_encrypted, redirect_uri, scopes, rate_limit_per_day, is_active, created_at, updated_at
FROM tenant_oauth_credentials;

-- Drop old table
DROP TABLE IF EXISTS tenant_oauth_credentials;

-- Rename new table
ALTER TABLE tenant_oauth_credentials_new RENAME TO tenant_oauth_credentials;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_tenant ON tenant_oauth_credentials(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_provider ON tenant_oauth_credentials(provider);
