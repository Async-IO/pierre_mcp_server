-- ABOUTME: Migration to add missing foreign key constraints for referential integrity
-- ABOUTME: Fixes a2a_clients.user_id and user_configurations.user_id to properly reference users table

-- SQLite requires table recreation to add FK constraints
-- Using PRAGMA foreign_keys is handled by SQLx at runtime

-- ============================================================================
-- Fix 1: a2a_clients.user_id should reference users(id)
-- ============================================================================

-- Create new table with proper FK constraint
CREATE TABLE IF NOT EXISTS a2a_clients_new (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
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

-- Copy existing data (only rows with valid user references)
INSERT INTO a2a_clients_new
SELECT ac.* FROM a2a_clients ac
WHERE EXISTS (SELECT 1 FROM users u WHERE u.id = ac.user_id);

-- Drop old table
DROP TABLE IF EXISTS a2a_clients;

-- Rename new table
ALTER TABLE a2a_clients_new RENAME TO a2a_clients;

-- Recreate indexes for a2a_clients
CREATE INDEX IF NOT EXISTS idx_a2a_clients_user_id ON a2a_clients(user_id);

-- ============================================================================
-- Fix 2: user_configurations.user_id should reference users(id)
-- ============================================================================

-- Create new table with proper FK constraint
CREATE TABLE IF NOT EXISTS user_configurations_new (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    config_data TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Copy existing data (only rows with valid user references)
INSERT INTO user_configurations_new
SELECT uc.* FROM user_configurations uc
WHERE EXISTS (SELECT 1 FROM users u WHERE u.id = uc.user_id);

-- Drop old table
DROP TABLE IF EXISTS user_configurations;

-- Rename new table
ALTER TABLE user_configurations_new RENAME TO user_configurations;

-- Index on user_id is implicit since it's the PRIMARY KEY
