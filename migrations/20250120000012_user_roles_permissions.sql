-- ABOUTME: Adds role-based permission system with super_admin, admin, user roles
-- ABOUTME: Includes impersonation audit logging and permission delegation tables
--
-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Copyright (c) 2025 Pierre Fitness Intelligence

-- Add role column to users table (replaces is_admin boolean)
-- role: 'super_admin' | 'admin' | 'user'
ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'user'
  CHECK (role IN ('super_admin', 'admin', 'user'));

-- Migrate existing is_admin flag to role
UPDATE users SET role = 'admin' WHERE is_admin = 1;

-- Impersonation audit log for super admin actions
CREATE TABLE IF NOT EXISTS impersonation_sessions (
    id TEXT PRIMARY KEY,
    impersonator_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_impersonation_active ON impersonation_sessions(impersonator_id, is_active);
CREATE INDEX IF NOT EXISTS idx_impersonation_target ON impersonation_sessions(target_user_id);

-- Permission delegations for session sharing between users
CREATE TABLE IF NOT EXISTS permission_delegations (
    id TEXT PRIMARY KEY,
    grantor_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    grantee_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    permissions INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    UNIQUE(grantor_id, grantee_id)
);

CREATE INDEX IF NOT EXISTS idx_delegation_grantee ON permission_delegations(grantee_id);
CREATE INDEX IF NOT EXISTS idx_delegation_grantor ON permission_delegations(grantor_id);

-- User MCP tokens for AI client connections (separate from admin tokens)
CREATE TABLE IF NOT EXISTS user_mcp_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    token_prefix TEXT NOT NULL,
    expires_at TEXT,
    last_used_at TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    is_revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_user_tokens_user ON user_mcp_tokens(user_id, is_revoked);
CREATE INDEX IF NOT EXISTS idx_user_tokens_prefix ON user_mcp_tokens(token_prefix);
