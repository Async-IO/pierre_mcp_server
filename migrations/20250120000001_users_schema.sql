-- ABOUTME: Users schema migration for SQLite and PostgreSQL
-- ABOUTME: Creates users, user_profiles, and user_oauth_app_credentials tables with indexes

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    display_name TEXT,
    password_hash TEXT NOT NULL,
    tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
    tenant_id TEXT,
    strava_access_token TEXT,
    strava_refresh_token TEXT,
    strava_expires_at INTEGER,
    strava_scope TEXT,
    strava_last_sync TEXT,
    fitbit_access_token TEXT,
    fitbit_refresh_token TEXT,
    fitbit_expires_at INTEGER,
    fitbit_scope TEXT,
    fitbit_last_sync TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    user_status TEXT NOT NULL DEFAULT 'pending' CHECK (user_status IN ('pending', 'active', 'suspended')),
    is_admin INTEGER NOT NULL DEFAULT 0,
    approved_by TEXT,
    approved_at TEXT,
    created_at TEXT NOT NULL,
    last_active TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_profiles (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    profile_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS user_oauth_app_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider TEXT NOT NULL CHECK (provider IN ('strava', 'fitbit')),
    client_id TEXT NOT NULL,
    client_secret TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(user_id, provider)
);

-- Indexes for users table
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);
CREATE INDEX IF NOT EXISTS idx_users_status ON users(user_status);
CREATE INDEX IF NOT EXISTS idx_user_oauth_apps_user ON user_oauth_app_credentials(user_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_apps_provider ON user_oauth_app_credentials(provider);
CREATE INDEX IF NOT EXISTS idx_user_oauth_apps_user_provider ON user_oauth_app_credentials(user_id, provider);
