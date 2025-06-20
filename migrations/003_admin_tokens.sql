-- Admin Token Management Schema
-- This migration adds support for admin service authentication and API key provisioning

-- Admin Tokens table
CREATE TABLE IF NOT EXISTS admin_tokens (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    
    -- Service identification
    service_name TEXT NOT NULL, -- "pierre_admin_service", "custom_admin_tool"
    service_description TEXT,
    
    -- Token security
    token_hash TEXT NOT NULL, -- bcrypt hash of full JWT token
    token_prefix TEXT NOT NULL, -- First 12 chars for identification (admin_jwt_)
    jwt_secret_hash TEXT NOT NULL, -- Hash of JWT signing secret
    
    -- Permissions (JSON array of permission strings)
    permissions TEXT NOT NULL DEFAULT '["provision_keys"]',
    
    -- Super admin capability
    is_super_admin BOOLEAN NOT NULL DEFAULT false,
    
    -- Token lifecycle
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP, -- NULL for super admin tokens (never expire)
    last_used_at TIMESTAMP,
    
    -- Security tracking
    last_used_ip TEXT,
    usage_count INTEGER NOT NULL DEFAULT 0,
    
    -- Unique constraints
    UNIQUE(service_name, is_active), -- Only one active token per service
    UNIQUE(token_prefix)
);

-- Indexes for performance and security
CREATE INDEX idx_admin_tokens_prefix ON admin_tokens(token_prefix);
CREATE INDEX idx_admin_tokens_service ON admin_tokens(service_name);
CREATE INDEX idx_admin_tokens_active ON admin_tokens(is_active);
CREATE INDEX idx_admin_tokens_expires ON admin_tokens(expires_at);
CREATE INDEX idx_admin_tokens_super_admin ON admin_tokens(is_super_admin);

-- Admin Token Usage Audit Log
CREATE TABLE IF NOT EXISTS admin_token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
    
    -- Usage details
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    action TEXT NOT NULL, -- "provision_key", "revoke_key", "list_keys", etc.
    target_resource TEXT, -- User ID, API key ID, etc.
    
    -- Request context
    ip_address TEXT,
    user_agent TEXT,
    request_size_bytes INTEGER,
    
    -- Result
    success BOOLEAN NOT NULL,
    error_message TEXT,
    response_time_ms INTEGER,
    
    -- Indexes for auditing
    INDEX idx_admin_usage_token_id (admin_token_id),
    INDEX idx_admin_usage_timestamp (timestamp),
    INDEX idx_admin_usage_action (action),
    INDEX idx_admin_usage_success (success)
);

-- Admin API Key Provisioning Log
CREATE TABLE IF NOT EXISTS admin_provisioned_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    admin_token_id TEXT NOT NULL REFERENCES admin_tokens(id) ON DELETE CASCADE,
    
    -- Provisioned key details
    api_key_id TEXT NOT NULL, -- Links to api_keys table
    user_email TEXT NOT NULL,
    requested_tier TEXT NOT NULL,
    
    -- Provisioning details
    provisioned_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    provisioned_by_service TEXT NOT NULL, -- Service name that provisioned
    
    -- Rate limit details at provisioning time
    rate_limit_requests INTEGER NOT NULL,
    rate_limit_period TEXT NOT NULL, -- "hour", "day", "month"
    
    -- Key lifecycle
    key_status TEXT NOT NULL DEFAULT 'active', -- "active", "revoked", "expired"
    revoked_at TIMESTAMP,
    revoked_reason TEXT,
    
    -- Indexes for tracking
    INDEX idx_provisioned_admin_token (admin_token_id),
    INDEX idx_provisioned_api_key (api_key_id),
    INDEX idx_provisioned_user_email (user_email),
    INDEX idx_provisioned_timestamp (provisioned_at),
    INDEX idx_provisioned_status (key_status)
);

-- Trigger to update admin token usage count and last_used_at
CREATE TRIGGER update_admin_token_usage 
AFTER INSERT ON admin_token_usage
BEGIN
    UPDATE admin_tokens 
    SET 
        usage_count = usage_count + 1,
        last_used_at = NEW.timestamp,
        last_used_ip = NEW.ip_address
    WHERE id = NEW.admin_token_id;
END;

-- Trigger to update admin token timestamps
CREATE TRIGGER update_admin_tokens_timestamp 
AFTER UPDATE ON admin_tokens
BEGIN
    UPDATE admin_tokens SET last_used_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;