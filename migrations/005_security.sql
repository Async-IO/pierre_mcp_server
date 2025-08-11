-- Security and Key Rotation Tables Migration
-- Creates tables for encryption key management and audit logging

-- Key Versions Table
-- Tracks encryption key versions for rotation management
CREATE TABLE IF NOT EXISTS key_versions (
    tenant_id TEXT,                              -- NULL for global keys, tenant UUID for tenant-specific
    version INTEGER NOT NULL,                    -- Version number (incremental)
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    expires_at DATETIME NOT NULL,               -- When this key version expires
    is_active BOOLEAN NOT NULL DEFAULT FALSE,   -- Only one version should be active per tenant
    algorithm TEXT NOT NULL DEFAULT 'AES-256-GCM',
    
    -- Primary key is composite of tenant_id and version
    PRIMARY KEY (tenant_id, version),
    
    -- Ensure only one active version per tenant
    UNIQUE (tenant_id, is_active) WHERE is_active = true
);

-- Create index for efficient lookups
CREATE INDEX IF NOT EXISTS idx_key_versions_tenant_active 
ON key_versions(tenant_id, is_active, version DESC);

CREATE INDEX IF NOT EXISTS idx_key_versions_expires_at 
ON key_versions(expires_at);

-- Audit Events Table
-- Comprehensive audit logging for security events
CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,                         -- UUID for the audit event
    event_type TEXT NOT NULL,                   -- Type of event (login, key_rotation, etc)
    severity TEXT NOT NULL,                     -- Critical, Warning, Info, Debug
    message TEXT NOT NULL,                      -- Human-readable event description
    source TEXT NOT NULL,                       -- Source component/service
    result TEXT NOT NULL,                       -- success, failure, partial
    
    -- Context information
    tenant_id TEXT,                             -- Associated tenant (if applicable)
    user_id TEXT,                              -- Associated user (if applicable)
    ip_address TEXT,                           -- Client IP address
    user_agent TEXT,                           -- Client user agent
    
    -- Structured metadata as JSON
    metadata TEXT NOT NULL DEFAULT '{}',       -- Additional structured data
    
    -- Timestamp
    timestamp DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    
    -- Foreign key constraints (soft references due to multi-tenant nature)
    FOREIGN KEY (tenant_id) REFERENCES tenants(id) ON DELETE SET NULL,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE SET NULL
);

-- Create indexes for efficient audit queries
CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp 
ON audit_events(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_audit_events_tenant_timestamp 
ON audit_events(tenant_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_audit_events_user_timestamp 
ON audit_events(user_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_audit_events_type_timestamp 
ON audit_events(event_type, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_audit_events_severity 
ON audit_events(severity, timestamp DESC);

-- Create a trigger to automatically clean up old audit events (keep last 100,000)
CREATE TRIGGER IF NOT EXISTS audit_events_cleanup
    AFTER INSERT ON audit_events
    WHEN (SELECT COUNT(*) FROM audit_events) > 100000
BEGIN
    DELETE FROM audit_events 
    WHERE id IN (
        SELECT id FROM audit_events 
        ORDER BY timestamp ASC 
        LIMIT ((SELECT COUNT(*) FROM audit_events) - 100000)
    );
END;

-- Insert initial global key version (version 1)
INSERT OR IGNORE INTO key_versions (tenant_id, version, created_at, expires_at, is_active, algorithm)
VALUES (NULL, 1, CURRENT_TIMESTAMP, datetime('now', '+1 year'), true, 'AES-256-GCM');

-- Insert initial audit event for key rotation system initialization
INSERT OR IGNORE INTO audit_events (
    id, event_type, severity, message, source, result, 
    metadata, timestamp
) VALUES (
    hex(randomblob(16)), 
    'SystemInitialized', 
    'Info', 
    'Key rotation and audit system initialized',
    'security_migration',
    'success',
    '{"version": "1.0", "migration": "005_security"}',
    CURRENT_TIMESTAMP
);