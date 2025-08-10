-- Tenant Management Schema
-- This migration adds support for multi-tenant architecture with per-tenant OAuth credentials

-- Tenants table - Organizations/customers using the MCP server
CREATE TABLE IF NOT EXISTS tenants (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    
    -- Basic tenant information
    name TEXT NOT NULL,
    slug TEXT UNIQUE NOT NULL, -- URL-friendly identifier
    domain TEXT, -- Optional custom domain
    plan TEXT NOT NULL DEFAULT 'starter' CHECK (plan IN ('starter', 'professional', 'enterprise')),
    
    -- Ownership
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    UNIQUE(owner_user_id, name) -- One tenant per name per owner
);

-- Indexes for tenant lookup
CREATE INDEX IF NOT EXISTS idx_tenants_slug ON tenants(slug);
CREATE INDEX IF NOT EXISTS idx_tenants_owner ON tenants(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_tenants_active ON tenants(is_active);

-- Tenant OAuth Credentials - Per-tenant provider credentials (Strava, Fitbit, etc.)
CREATE TABLE IF NOT EXISTS tenant_oauth_credentials (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    
    -- OAuth provider information
    provider TEXT NOT NULL, -- 'strava', 'fitbit', etc.
    client_id TEXT NOT NULL,
    client_secret_encrypted TEXT NOT NULL, -- Encrypted with server key
    redirect_uri TEXT NOT NULL,
    
    -- Scopes and limits
    scopes TEXT NOT NULL, -- JSON array of scopes
    rate_limit_per_day INTEGER NOT NULL DEFAULT 15000,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    UNIQUE(tenant_id, provider) -- One OAuth config per provider per tenant
);

-- Indexes for OAuth credential lookup
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_tenant_id ON tenant_oauth_credentials(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_provider ON tenant_oauth_credentials(provider);
CREATE INDEX IF NOT EXISTS idx_tenant_oauth_active ON tenant_oauth_credentials(is_active);

-- OAuth Applications - Claude Desktop, ChatGPT, etc. connecting to this server
CREATE TABLE IF NOT EXISTS oauth_apps (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    
    -- OAuth application details
    client_id TEXT UNIQUE NOT NULL,
    client_secret_hash TEXT NOT NULL, -- SHA-256 hash of client secret
    name TEXT NOT NULL,
    description TEXT,
    
    -- OAuth configuration
    redirect_uris TEXT NOT NULL, -- JSON array of valid redirect URIs
    scopes TEXT NOT NULL, -- JSON array of supported scopes
    app_type TEXT NOT NULL DEFAULT 'confidential' CHECK (app_type IN ('public', 'confidential')),
    
    -- Ownership
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Constraints
    UNIQUE(owner_user_id, name) -- One app per name per owner
);

-- Indexes for OAuth app lookup
CREATE INDEX IF NOT EXISTS idx_oauth_apps_client_id ON oauth_apps(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_apps_owner ON oauth_apps(owner_user_id);
CREATE INDEX IF NOT EXISTS idx_oauth_apps_active ON oauth_apps(is_active);

-- Authorization Codes - Temporary codes for OAuth flow
CREATE TABLE IF NOT EXISTS authorization_codes (
    code TEXT PRIMARY KEY,
    
    -- OAuth flow data
    client_id TEXT NOT NULL REFERENCES oauth_apps(client_id) ON DELETE CASCADE,
    redirect_uri TEXT NOT NULL,
    scope TEXT NOT NULL,
    
    -- Optional user association (for user-specific authorization)
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    
    -- Code lifecycle
    expires_at TIMESTAMP NOT NULL,
    is_used BOOLEAN NOT NULL DEFAULT false,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    used_at TIMESTAMP
);

-- Indexes for authorization code cleanup and lookup
CREATE INDEX IF NOT EXISTS idx_auth_codes_client_id ON authorization_codes(client_id);
CREATE INDEX IF NOT EXISTS idx_auth_codes_expires_at ON authorization_codes(expires_at);
CREATE INDEX IF NOT EXISTS idx_auth_codes_used ON authorization_codes(is_used);

-- OAuth Access Tokens - Long-lived tokens for API access
CREATE TABLE IF NOT EXISTS oauth_access_tokens (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    
    -- Token details
    token_hash TEXT UNIQUE NOT NULL, -- SHA-256 hash of actual token
    client_id TEXT NOT NULL REFERENCES oauth_apps(client_id) ON DELETE CASCADE,
    user_id TEXT REFERENCES users(id) ON DELETE CASCADE, -- Optional for client credentials
    
    -- Token metadata
    scope TEXT NOT NULL,
    token_type TEXT NOT NULL DEFAULT 'bearer',
    expires_at TIMESTAMP,
    
    -- Usage tracking
    last_used_at TIMESTAMP,
    usage_count INTEGER NOT NULL DEFAULT 0,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for token lookup and cleanup
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_hash ON oauth_access_tokens(token_hash);
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_client ON oauth_access_tokens(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_expires ON oauth_access_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_active ON oauth_access_tokens(is_active);

-- Tenant Usage Tracking - Per-tenant API usage metrics
CREATE TABLE IF NOT EXISTS tenant_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    
    -- Usage details
    provider TEXT NOT NULL, -- 'strava', 'fitbit', etc.
    endpoint TEXT NOT NULL, -- API endpoint used
    method TEXT NOT NULL, -- HTTP method
    
    -- Response details
    status_code INTEGER NOT NULL,
    response_time_ms INTEGER,
    error_message TEXT,
    
    -- Timestamps
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Tenant Usage Stats - Aggregated metrics for dashboard
CREATE TABLE IF NOT EXISTS tenant_usage_stats (
    tenant_id TEXT NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,
    
    -- Aggregated metrics
    total_requests INTEGER NOT NULL DEFAULT 0,
    successful_requests INTEGER NOT NULL DEFAULT 0,
    failed_requests INTEGER NOT NULL DEFAULT 0,
    total_response_time_ms INTEGER NOT NULL DEFAULT 0,
    
    -- Rate limiting
    daily_limit INTEGER NOT NULL,
    current_usage INTEGER NOT NULL DEFAULT 0,
    
    PRIMARY KEY (tenant_id, provider, period_start)
);

-- Indexes for tenant usage tracking
CREATE INDEX IF NOT EXISTS idx_tenant_usage_tenant_id ON tenant_usage(tenant_id);
CREATE INDEX IF NOT EXISTS idx_tenant_usage_timestamp ON tenant_usage(timestamp);
CREATE INDEX IF NOT EXISTS idx_tenant_usage_provider ON tenant_usage(provider);

-- Indexes for tenant usage stats
CREATE INDEX IF NOT EXISTS idx_tenant_stats_period ON tenant_usage_stats(period_start, period_end);

-- Triggers to update timestamps (drop and recreate for idempotency)
DROP TRIGGER IF EXISTS update_tenants_timestamp;
CREATE TRIGGER update_tenants_timestamp 
AFTER UPDATE ON tenants
BEGIN
    UPDATE tenants SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

DROP TRIGGER IF EXISTS update_tenant_oauth_timestamp;
CREATE TRIGGER update_tenant_oauth_timestamp 
AFTER UPDATE ON tenant_oauth_credentials
BEGIN
    UPDATE tenant_oauth_credentials SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

DROP TRIGGER IF EXISTS update_oauth_apps_timestamp;
CREATE TRIGGER update_oauth_apps_timestamp 
AFTER UPDATE ON oauth_apps
BEGIN
    UPDATE oauth_apps SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

DROP TRIGGER IF EXISTS update_oauth_tokens_timestamp;
CREATE TRIGGER update_oauth_tokens_timestamp 
AFTER UPDATE ON oauth_access_tokens
BEGIN
    UPDATE oauth_access_tokens SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- Cleanup trigger for expired authorization codes
DROP TRIGGER IF EXISTS cleanup_expired_auth_codes;
CREATE TRIGGER cleanup_expired_auth_codes
AFTER INSERT ON authorization_codes
BEGIN
    DELETE FROM authorization_codes 
    WHERE expires_at < CURRENT_TIMESTAMP 
    AND is_used = false;
END;