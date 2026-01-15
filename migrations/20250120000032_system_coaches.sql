-- ABOUTME: Schema migration for system coaches (admin-created, tenant-wide visibility).
-- ABOUTME: Adds is_system/visibility columns and tables for assignments and user preferences.

-- Add columns to coaches table for system coach support
ALTER TABLE coaches ADD COLUMN is_system INTEGER NOT NULL DEFAULT 0;
ALTER TABLE coaches ADD COLUMN visibility TEXT NOT NULL DEFAULT 'private';
-- visibility values: 'private' (user only), 'tenant' (all tenant users), 'global' (all tenants)

-- Index for efficient system coach queries
CREATE INDEX IF NOT EXISTS idx_coaches_system ON coaches(tenant_id, is_system, visibility)
    WHERE is_system = 1;

-- Coach assignments table: admin assigns specific coaches to specific users
CREATE TABLE IF NOT EXISTS coach_assignments (
    id TEXT PRIMARY KEY,
    coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    assigned_by TEXT REFERENCES users(id) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    UNIQUE(coach_id, user_id)
);

-- Indexes for coach assignments
CREATE INDEX IF NOT EXISTS idx_coach_assignments_coach ON coach_assignments(coach_id);
CREATE INDEX IF NOT EXISTS idx_coach_assignments_user ON coach_assignments(user_id);

-- User coach preferences table: allows users to hide system/assigned coaches
CREATE TABLE IF NOT EXISTS user_coach_preferences (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    coach_id TEXT NOT NULL REFERENCES coaches(id) ON DELETE CASCADE,
    is_hidden INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(user_id, coach_id)
);

-- Index for user preferences lookup
CREATE INDEX IF NOT EXISTS idx_user_coach_prefs_user ON user_coach_preferences(user_id);
CREATE INDEX IF NOT EXISTS idx_user_coach_prefs_hidden ON user_coach_preferences(user_id, is_hidden)
    WHERE is_hidden = 1;
