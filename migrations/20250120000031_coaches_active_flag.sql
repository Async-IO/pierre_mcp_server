-- ABOUTME: Migration to add active coach functionality
-- ABOUTME: Allows one coach per user to be marked as active for current session

-- Add is_active column to coaches table
-- Only one coach per user can be active at a time
ALTER TABLE coaches ADD COLUMN is_active INTEGER NOT NULL DEFAULT 0;

-- Index for finding active coach by user (most common query)
CREATE INDEX IF NOT EXISTS idx_coaches_active ON coaches(user_id, is_active) WHERE is_active = 1;
