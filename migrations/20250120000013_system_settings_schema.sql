-- ABOUTME: System settings table for admin-configurable options
-- ABOUTME: Stores key-value pairs for system-wide configuration like auto-approval
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- System Settings Table for admin-configurable options
CREATE TABLE IF NOT EXISTS system_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Insert default settings
INSERT OR IGNORE INTO system_settings (key, value, description, created_at, updated_at)
VALUES (
    'auto_approval_enabled',
    'false',
    'When enabled, new user registrations are automatically approved without admin intervention',
    datetime('now'),
    datetime('now')
);

-- Index for fast lookups
CREATE INDEX IF NOT EXISTS idx_system_settings_key ON system_settings(key);
