-- ABOUTME: Migration to remove legacy OAuth token columns from users table
-- ABOUTME: Adds last_sync to user_oauth_tokens and drops redundant columns from users

-- ============================================================================
-- Step 1: Add last_sync column to user_oauth_tokens for sync tracking
-- ============================================================================

ALTER TABLE user_oauth_tokens ADD COLUMN last_sync TEXT;

-- ============================================================================
-- Step 2: Recreate users table without legacy token columns
-- SQLite requires table recreation to remove columns
-- ============================================================================

-- Create new users table without legacy OAuth token columns
-- Note: Preserves role column added by migration 20250120000012_user_roles_permissions.sql
CREATE TABLE IF NOT EXISTS users_new (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    display_name TEXT,
    password_hash TEXT NOT NULL,
    tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
    tenant_id TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    user_status TEXT NOT NULL DEFAULT 'pending' CHECK (user_status IN ('pending', 'active', 'suspended')),
    is_admin INTEGER NOT NULL DEFAULT 0,
    role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('super_admin', 'admin', 'user')),
    approved_by TEXT,
    approved_at TEXT,
    created_at TEXT NOT NULL,
    last_active TEXT NOT NULL
);

-- Copy data from old table (excluding legacy token columns)
INSERT INTO users_new (
    id, email, display_name, password_hash, tier, tenant_id,
    is_active, user_status, is_admin, role, approved_by, approved_at,
    created_at, last_active
)
SELECT
    id, email, display_name, password_hash, tier, tenant_id,
    is_active, user_status, is_admin, role, approved_by, approved_at,
    created_at, last_active
FROM users;

-- Drop old table
DROP TABLE users;

-- Rename new table
ALTER TABLE users_new RENAME TO users;

-- Recreate indexes
CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);
CREATE INDEX IF NOT EXISTS idx_users_status ON users(user_status);
