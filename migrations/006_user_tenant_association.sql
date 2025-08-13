-- Migration 006: User-Tenant Association and OAuth Token Storage
-- Adds tenant association for users and user-specific OAuth token storage

-- Add tenant_id to users table for multi-tenant association
ALTER TABLE users ADD COLUMN tenant_id TEXT;

-- Add foreign key constraint to tenants table (if it doesn't already exist)
-- This ensures users belong to valid tenants
-- Note: We don't make it NOT NULL initially to allow existing users

-- Create user_oauth_tokens table for storing user's OAuth tokens per tenant-provider
CREATE TABLE IF NOT EXISTS user_oauth_tokens (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL, -- 'strava', 'fitbit', etc.
    access_token TEXT NOT NULL, -- encrypted OAuth access token
    refresh_token TEXT, -- encrypted OAuth refresh token (nullable for some providers)
    token_type TEXT NOT NULL DEFAULT 'Bearer',
    expires_at DATETIME, -- when access token expires
    scope TEXT, -- granted OAuth scopes
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Ensure one token per user-tenant-provider combination
    UNIQUE(user_id, tenant_id, provider),
    
    -- Foreign key constraints
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE CASCADE
);

-- Create indexes for efficient lookups
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_user_tenant ON user_oauth_tokens(user_id, tenant_id);
CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_tenant_provider ON user_oauth_tokens(tenant_id, provider);
CREATE INDEX IF NOT EXISTS idx_user_oauth_tokens_expires_at ON user_oauth_tokens(expires_at);

-- Create trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_user_oauth_tokens_timestamp 
    AFTER UPDATE ON user_oauth_tokens
    FOR EACH ROW 
    BEGIN
        UPDATE user_oauth_tokens SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
    END;

-- Add default tenant for existing users (if any)
-- This allows existing installations to work without breaking
UPDATE users 
SET tenant_id = 'default-tenant' 
WHERE tenant_id IS NULL;

-- Insert default tenant if it doesn't exist
INSERT OR IGNORE INTO tenants (id, name, contact_email, created_at) 
VALUES ('default-tenant', 'Default Tenant', 'admin@pierre.mcp', CURRENT_TIMESTAMP);

-- Now make tenant_id NOT NULL with a default
-- This is done after populating existing rows
-- Note: SQLite doesn't support ALTER COLUMN directly, so we'll handle this in the application