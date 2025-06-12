-- API Key Management Schema
-- This migration adds support for B2B API key authentication and usage tracking

-- API Keys table
CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    
    -- Key information
    name TEXT NOT NULL,
    key_prefix TEXT NOT NULL, -- First 8 chars for identification (pk_live_)
    key_hash TEXT NOT NULL, -- SHA-256 hash of full key
    
    -- Metadata
    description TEXT,
    tier TEXT NOT NULL DEFAULT 'starter' CHECK (tier IN ('starter', 'professional', 'enterprise')),
    
    -- Rate limiting
    rate_limit_requests INTEGER NOT NULL DEFAULT 10000, -- Requests per month
    rate_limit_window INTEGER NOT NULL DEFAULT 2592000, -- 30 days in seconds
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_used_at TIMESTAMP,
    expires_at TIMESTAMP,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Ensure unique key names per user
    UNIQUE(user_id, name)
);

-- Index for fast key lookup
CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_active ON api_keys(is_active);

-- API Key Usage table
CREATE TABLE IF NOT EXISTS api_key_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    
    -- Usage metrics
    timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    tool_name TEXT NOT NULL,
    response_time_ms INTEGER,
    status_code INTEGER NOT NULL,
    error_message TEXT,
    
    -- Request metadata
    request_size_bytes INTEGER,
    response_size_bytes INTEGER,
    ip_address TEXT,
    user_agent TEXT,
    
    -- Indexes for analytics
    INDEX idx_usage_api_key_id (api_key_id),
    INDEX idx_usage_timestamp (timestamp),
    INDEX idx_usage_tool_name (tool_name)
);

-- Aggregated usage stats (for performance)
CREATE TABLE IF NOT EXISTS api_key_usage_stats (
    api_key_id TEXT NOT NULL REFERENCES api_keys(id) ON DELETE CASCADE,
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,
    
    -- Aggregated metrics
    total_requests INTEGER NOT NULL DEFAULT 0,
    successful_requests INTEGER NOT NULL DEFAULT 0,
    failed_requests INTEGER NOT NULL DEFAULT 0,
    total_response_time_ms INTEGER NOT NULL DEFAULT 0,
    
    -- Per-tool breakdown (JSON)
    tool_usage TEXT NOT NULL DEFAULT '{}', -- JSON object with tool counts
    
    PRIMARY KEY (api_key_id, period_start),
    INDEX idx_stats_period (period_start, period_end)
);

-- Rate limit tracking (sliding window)
CREATE TABLE IF NOT EXISTS api_key_rate_limits (
    api_key_id TEXT PRIMARY KEY REFERENCES api_keys(id) ON DELETE CASCADE,
    
    -- Current window
    window_start TIMESTAMP NOT NULL,
    request_count INTEGER NOT NULL DEFAULT 0,
    
    -- Quick lookup
    is_rate_limited BOOLEAN NOT NULL DEFAULT false,
    rate_limit_reset_at TIMESTAMP,
    
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Trigger to update timestamps
CREATE TRIGGER update_api_keys_timestamp 
AFTER UPDATE ON api_keys
BEGIN
    UPDATE api_keys SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;