-- Provider connections table: single source of truth for "is this provider connected for this user"
-- Unifies OAuth, synthetic, and future non-OAuth provider connection tracking.
-- Previously, connection status was scattered across user_oauth_tokens and synthetic_activities queries.

CREATE TABLE IF NOT EXISTS provider_connections (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    connection_type TEXT NOT NULL,  -- 'oauth', 'synthetic', 'manual'
    connected_at TEXT NOT NULL,
    metadata TEXT,  -- JSON: e.g. {"source": "seed-synthetic-activities"}
    UNIQUE(user_id, tenant_id, provider)
);

CREATE INDEX IF NOT EXISTS idx_provider_connections_user ON provider_connections(user_id);
CREATE INDEX IF NOT EXISTS idx_provider_connections_tenant ON provider_connections(tenant_id, provider);

-- Backfill: register provider connections for existing OAuth tokens
INSERT OR IGNORE INTO provider_connections (id, user_id, tenant_id, provider, connection_type, connected_at)
SELECT id, user_id, tenant_id, provider, 'oauth', created_at
FROM user_oauth_tokens;

-- Backfill: register provider connections for users with existing synthetic activities
INSERT OR IGNORE INTO provider_connections (id, user_id, tenant_id, provider, connection_type, connected_at, metadata)
SELECT
    lower(hex(randomblob(4)) || '-' || hex(randomblob(2)) || '-4' || substr(hex(randomblob(2)),2) || '-' || substr('89ab', abs(random()) % 4 + 1, 1) || substr(hex(randomblob(2)),2) || '-' || hex(randomblob(6))) as id,
    sa.user_id,
    sa.tenant_id,
    'synthetic',
    'synthetic',
    MIN(sa.created_at),
    '{"source": "migration-backfill"}'
FROM synthetic_activities sa
GROUP BY sa.user_id, sa.tenant_id;
