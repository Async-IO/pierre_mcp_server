# Database Schema Documentation

Pierre MCP Server database schema reference for SQLite and PostgreSQL.

## Overview

Pierre uses a single database with 20+ tables organized into functional modules:
- User management and authentication
- OAuth 2.0 token storage (fitness providers)
- OAuth 2.0 Authorization Server (MCP clients)
- API key management
- Tenant multi-tenancy
- Analytics and usage tracking
- A2A (Agent-to-Agent) authentication
- Fitness configurations
- System administration

All migrations are in src/database/mod.rs:72-103.

## Entity Relationship Diagram

```
┌─────────┐       ┌──────────────────┐       ┌───────────────┐
│  users  │──────<│ user_oauth_tokens│       │ oauth2_clients│
└────┬────┘       └──────────────────┘       └───────┬───────┘
     │                                                 │
     │            ┌──────────────┐                    │
     ├───────────<│  api_keys    │                    │
     │            └──────────────┘                    │
     │                                                 │
     │            ┌──────────────┐                    │
     ├───────────<│  insights    │                    │
     │            └──────────────┘                    │
     │                                                 │
     │            ┌──────────────┐                    │
     ├───────────<│    goals     │                    │
     │            └──────────────┘                    │
     │                                            oauth2_auth_codes
     │            ┌──────────────┐                    │
     └───────────<│   tenants    │                    │
                  └──────────────┘                    │
```

## Core Tables

### users

User accounts with authentication and status tracking (src/database/users.rs).

```sql
CREATE TABLE users (
    user_id TEXT PRIMARY KEY,              -- UUID
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    display_name TEXT,
    user_status TEXT NOT NULL,             -- active, pending, suspended
    created_at DATETIME NOT NULL,
    last_active DATETIME,
    tenant_id TEXT,                        -- References tenants(tenant_id)
    is_admin BOOLEAN DEFAULT 0,
    profile_data TEXT                      -- JSON
);

CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_tenant_id ON users(tenant_id);
CREATE INDEX idx_users_status ON users(user_status);
```

**Fields**:
- `user_id` - UUID primary key
- `email` - Unique email address for login
- `password_hash` - bcrypt hash (never store plaintext)
- `user_status` - Enum: `active`, `pending`, `suspended`
- `tenant_id` - Multi-tenancy support (NULL = default tenant)
- `is_admin` - Admin user flag for elevated permissions
- `profile_data` - JSON blob for athlete profile from providers

**User Status Flow**:
```
pending → active (admin approval required)
active → suspended (admin action)
suspended → active (admin reinstatement)
```

### user_oauth_tokens

OAuth tokens for fitness providers (Strava, Garmin, Fitbit) - src/database/user_oauth_tokens.rs.

```sql
CREATE TABLE user_oauth_tokens (
    id TEXT PRIMARY KEY,                   -- UUID
    user_id TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,                -- strava, fitbit
    access_token TEXT NOT NULL,            -- Encrypted
    refresh_token TEXT,                    -- Encrypted (optional)
    expires_at DATETIME,
    scope TEXT,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    UNIQUE(user_id, tenant_id, provider)
);

CREATE INDEX idx_user_oauth_tokens_user_provider ON user_oauth_tokens(user_id, provider);
CREATE INDEX idx_user_oauth_tokens_expires ON user_oauth_tokens(expires_at);
```

**Encryption**: `access_token` and `refresh_token` are encrypted using `PIERRE_MASTER_ENCRYPTION_KEY`.

**Token Refresh**: Tokens auto-refresh when expired (expires_at < now) before API calls.

### api_keys

API keys for programmatic access - src/database/api_keys.rs.

```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,                   -- key_UUID
    user_id TEXT NOT NULL,
    key_hash TEXT NOT NULL,                -- SHA-256 hash
    key_prefix TEXT NOT NULL,              -- First 13 chars (pk_live_abc12)
    name TEXT,
    description TEXT,
    tier TEXT NOT NULL,                    -- trial, starter, professional, enterprise
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL,
    last_used_at DATETIME,
    expires_at DATETIME,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
CREATE INDEX idx_api_keys_prefix ON api_keys(key_prefix);
```

**Key Format**: `pk_{tier}_{random_50_chars}`
- `pk_trial_abc123def456...`
- `pk_live_abc123def456...`

**Security**: Only hash stored in database. Actual key shown once at creation.

**Tiers and Rate Limits** (src/constants/api_tier_limits.rs):
- Trial: 10,000 requests/month
- Starter: 50,000 requests/month
- Professional: 500,000 requests/month
- Enterprise: Unlimited

### api_key_usage

Usage tracking for API keys - src/database/api_keys.rs.

```sql
CREATE TABLE api_key_usage (
    id TEXT PRIMARY KEY,
    api_key_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    success BOOLEAN NOT NULL,
    response_time_ms INTEGER,
    error_message TEXT,
    FOREIGN KEY (api_key_id) REFERENCES api_keys(id) ON DELETE CASCADE
);

CREATE INDEX idx_api_key_usage_key_id ON api_key_usage(api_key_id);
CREATE INDEX idx_api_key_usage_timestamp ON api_key_usage(timestamp);
CREATE INDEX idx_api_key_usage_tool_name ON api_key_usage(tool_name);
```

**Retention**: Consider cleaning old usage data (>90 days) periodically.

**Query Examples**:
```sql
-- Usage in last 30 days
SELECT COUNT(*) FROM api_key_usage
WHERE api_key_id = 'key_xyz'
AND timestamp > datetime('now', '-30 days');

-- Most used tools
SELECT tool_name, COUNT(*) as count
FROM api_key_usage
WHERE api_key_id = 'key_xyz'
GROUP BY tool_name
ORDER BY count DESC LIMIT 10;
```

## OAuth 2.0 Authorization Server Tables

Tables for RFC 7591 dynamic client registration - src/database/mod.rs:111-180.

### oauth2_clients

Registered OAuth 2.0 clients (MCP clients, mobile apps).

```sql
CREATE TABLE oauth2_clients (
    id TEXT PRIMARY KEY,                   -- UUID
    client_id TEXT UNIQUE NOT NULL,        -- oauth2_client_UUID
    client_secret_hash TEXT NOT NULL,      -- SHA-256 hash
    redirect_uris TEXT NOT NULL,           -- JSON array
    grant_types TEXT NOT NULL,             -- JSON array (authorization_code, client_credentials, refresh_token)
    response_types TEXT NOT NULL,          -- JSON array (code)
    client_name TEXT,
    client_uri TEXT,
    scope TEXT,                            -- Space-separated scopes
    created_at DATETIME NOT NULL,
    expires_at DATETIME
);

CREATE INDEX idx_oauth2_clients_client_id ON oauth2_clients(client_id);
```

**Example Row**:
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "client_id": "oauth2_client_550e8400e29b41d4a716446655440000",
  "client_secret_hash": "sha256_hash_here",
  "redirect_uris": "[\"http://localhost:3000/callback\"]",
  "grant_types": "[\"authorization_code\",\"refresh_token\"]",
  "response_types": "[\"code\"]",
  "client_name": "Claude Desktop",
  "client_uri": "https://claude.ai",
  "scope": "fitness:read activities:read profile:read",
  "created_at": "2024-01-15 10:00:00",
  "expires_at": "2025-01-15 10:00:00"
}
```

### oauth2_auth_codes

Authorization codes for OAuth 2.0 flow (10-minute TTL).

```sql
CREATE TABLE oauth2_auth_codes (
    code TEXT PRIMARY KEY,                 -- Random 32-byte base64
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT,
    expires_at DATETIME NOT NULL,
    used BOOLEAN NOT NULL DEFAULT 0,
    code_challenge TEXT,                   -- PKCE support
    code_challenge_method TEXT,            -- S256 or plain
    FOREIGN KEY (client_id) REFERENCES oauth2_clients(client_id) ON DELETE CASCADE
);

CREATE INDEX idx_oauth2_auth_codes_code ON oauth2_auth_codes(code);
CREATE INDEX idx_oauth2_auth_codes_expires ON oauth2_auth_codes(expires_at);
```

**Lifecycle**:
1. Created during authorization (expires_at = now + 10 minutes)
2. Exchanged for access token (used = 1)
3. Cleaned up after expiry or use

**Cleanup Query**:
```sql
DELETE FROM oauth2_auth_codes WHERE expires_at < datetime('now') OR used = 1;
```

## Multi-Tenancy Tables

### tenants

Organization/workspace isolation - src/database/mod.rs:181-199.

```sql
CREATE TABLE tenants (
    tenant_id TEXT PRIMARY KEY,            -- tenant_UUID
    name TEXT NOT NULL,
    plan_type TEXT NOT NULL,               -- free, starter, professional, enterprise
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1
);

CREATE INDEX idx_tenants_name ON tenants(name);
```

**Multi-Tenancy Model**: Each user belongs to one tenant. Tenant-level OAuth credentials and settings.

### tenant_oauth_credentials

Per-tenant OAuth app credentials (custom Strava/Garmin/Fitbit apps).

```sql
CREATE TABLE tenant_oauth_credentials (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,                -- strava, fitbit
    client_id TEXT NOT NULL,               -- Encrypted
    client_secret TEXT NOT NULL,           -- Encrypted
    redirect_uri TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    UNIQUE(tenant_id, provider)
);
```

**Use Case**: Tenant brings their own Strava app credentials for white-labeling.

### tenant_users

User-tenant association (supports users in multiple tenants).

```sql
CREATE TABLE tenant_users (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    role TEXT NOT NULL,                    -- owner, admin, member
    joined_at DATETIME NOT NULL,
    FOREIGN KEY (tenant_id) REFERENCES tenants(tenant_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    UNIQUE(tenant_id, user_id)
);

CREATE INDEX idx_tenant_users_tenant ON tenant_users(tenant_id);
CREATE INDEX idx_tenant_users_user ON tenant_users(user_id);
```

## Admin Tables

### admin_tokens

Admin JWT secrets for authentication - src/database/mod.rs:377-400.

```sql
CREATE TABLE admin_tokens (
    id TEXT PRIMARY KEY,
    admin_user_id TEXT NOT NULL,
    jwt_secret TEXT NOT NULL,              -- Base64-encoded secret
    created_at DATETIME NOT NULL,
    expires_at DATETIME,
    is_revoked BOOLEAN NOT NULL DEFAULT 0,
    FOREIGN KEY (admin_user_id) REFERENCES users(user_id) ON DELETE CASCADE
);
```

**Note**: JWT secrets are stored per-admin for rotation capability.

### admin_provisioned_keys

Pre-provisioned API keys for system services.

```sql
CREATE TABLE admin_provisioned_keys (
    id TEXT PRIMARY KEY,
    key_hash TEXT NOT NULL,
    service_name TEXT NOT NULL,
    is_super_admin BOOLEAN NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL,
    expires_at DATETIME,
    created_by TEXT NOT NULL
);

CREATE INDEX idx_admin_provisioned_keys_hash ON admin_provisioned_keys(key_hash);
```

**Super Admin Keys**: Bypass rate limits and user authentication for internal services.

## A2A (Agent-to-Agent) Tables

A2A protocol support - src/database/a2a.rs.

### a2a_clients

Registered A2A agents.

```sql
CREATE TABLE a2a_clients (
    id TEXT PRIMARY KEY,
    client_id TEXT UNIQUE NOT NULL,        -- a2a_client_UUID
    client_name TEXT NOT NULL,
    public_key TEXT NOT NULL,              -- RSA public key (PEM)
    created_at DATETIME NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 1
);
```

### a2a_sessions

Active A2A sessions.

```sql
CREATE TABLE a2a_sessions (
    session_id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    expires_at DATETIME NOT NULL,
    created_at DATETIME NOT NULL,
    FOREIGN KEY (client_id) REFERENCES a2a_clients(client_id) ON DELETE CASCADE
);

CREATE INDEX idx_a2a_sessions_expires ON a2a_sessions(expires_at);
```

### a2a_usage

A2A agent usage tracking.

```sql
CREATE TABLE a2a_usage (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    capability TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    success BOOLEAN NOT NULL,
    FOREIGN KEY (client_id) REFERENCES a2a_clients(client_id) ON DELETE CASCADE
);

CREATE INDEX idx_a2a_usage_client ON a2a_usage(client_id);
CREATE INDEX idx_a2a_usage_timestamp ON a2a_usage(timestamp);
```

## Analytics Tables

### jwt_usage

JWT token usage tracking - src/database/analytics.rs.

```sql
CREATE TABLE jwt_usage (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    timestamp DATETIME NOT NULL,
    action TEXT NOT NULL,
    success BOOLEAN NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_jwt_usage_user ON jwt_usage(user_id);
CREATE INDEX idx_jwt_usage_timestamp ON jwt_usage(timestamp);
```

## User Data Tables

### goals

User fitness goals.

```sql
CREATE TABLE goals (
    goal_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    goal_data TEXT NOT NULL,               -- JSON blob
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_goals_user_id ON goals(user_id);
```

**Goal Data Structure** (JSON):
```json
{
  "goal_type": "distance",
  "target_value": 100,
  "current_value": 45,
  "timeframe": "monthly",
  "sport_type": "Run",
  "start_date": "2024-01-01",
  "end_date": "2024-01-31"
}
```

### insights

AI-generated activity insights.

```sql
CREATE TABLE insights (
    insight_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    insight_data TEXT NOT NULL,            -- JSON blob
    created_at DATETIME NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE
);

CREATE INDEX idx_insights_user_id ON insights(user_id);
CREATE INDEX idx_insights_created_at ON insights(created_at);
```

**Insight Data Structure** (JSON):
```json
{
  "type": "pace_improvement",
  "activity_id": "12345",
  "title": "Pace Improved 12%",
  "description": "Your pace was 12% faster than your 30-day average",
  "confidence": 0.85,
  "generated_at": "2024-01-15T10:00:00Z"
}
```

### fitness_configurations

User-specific fitness zone configurations - src/database/fitness_configurations.rs.

```sql
CREATE TABLE fitness_configurations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    config_name TEXT NOT NULL,
    config_data TEXT NOT NULL,             -- JSON blob
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(user_id) ON DELETE CASCADE,
    UNIQUE(user_id, config_name)
);

CREATE INDEX idx_fitness_config_user ON fitness_configurations(user_id);
```

**Configuration Structure** (JSON):
```json
{
  "hr_zones": {
    "zone1_max": 130,
    "zone2_max": 150,
    "zone3_max": 170,
    "zone4_max": 185,
    "zone5_max": 200
  },
  "power_zones": {
    "ftp": 250,
    "zone1_percent": 55,
    "zone2_percent": 75
  }
}
```

## System Tables

### system_secrets

System-wide secrets and encryption keys.

```sql
CREATE TABLE system_secrets (
    key_name TEXT PRIMARY KEY,
    encrypted_value TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);
```

**Stored Secrets**:
- `admin_jwt_secret` - Admin JWT signing key
- `oauth_state_secret` - OAuth CSRF state secret

### audit_events

Security audit log.

```sql
CREATE TABLE audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    user_id TEXT,
    tenant_id TEXT,
    metadata TEXT,                         -- JSON blob
    created_at DATETIME NOT NULL
);

CREATE INDEX idx_audit_events_type ON audit_events(event_type);
CREATE INDEX idx_audit_events_user ON audit_events(user_id);
CREATE INDEX idx_audit_events_timestamp ON audit_events(created_at);
```

**Event Types**:
- `user_login`, `user_logout`
- `user_registered`, `user_approved`, `user_suspended`
- `api_key_created`, `api_key_revoked`
- `oauth_connected`, `oauth_disconnected`
- `admin_action`

## Database Migrations

Migrations run automatically on server startup (src/database/mod.rs:72-103).

### Migration Order

1. `migrate_users()` - User tables
2. `migrate_api_keys()` - API key tables
3. `migrate_analytics()` - Analytics tables
4. `migrate_a2a()` - A2A tables
5. `migrate_admin()` - Admin tables
6. `migrate_user_oauth_tokens()` - OAuth token tables
7. `migrate_oauth_notifications()` - Notification tables
8. `migrate_oauth2()` - OAuth 2.0 server tables
9. `migrate_tenant_management()` - Tenant tables
10. `migrate_fitness_configurations()` - Fitness config tables

### Running Migrations

Migrations are idempotent (`CREATE TABLE IF NOT EXISTS`):

```bash
# SQLite (automatic on startup)
cargo run --bin pierre-mcp-server

# PostgreSQL (automatic on startup with DATABASE_URL set)
DATABASE_URL=postgresql://user:pass@localhost/pierre cargo run --bin pierre-mcp-server
```

### Manual Migration

```rust
use pierre_mcp_server::database::Database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let encryption_key = vec![0u8; 32]; // Load from environment
    let db = Database::new("sqlite:./data/pierre.db", encryption_key).await?;
    db.migrate().await?;
    Ok(())
}
```

## SQLite vs PostgreSQL Differences

| Feature | SQLite | PostgreSQL |
|---------|--------|------------|
| Data Types | TEXT, INTEGER, REAL, BLOB | VARCHAR, INT, TIMESTAMP, JSONB |
| DATETIME | TEXT with ISO 8601 | TIMESTAMP WITH TIME ZONE |
| JSON | TEXT (manual parse) | JSONB (native support) |
| Arrays | TEXT with JSON | ARRAY type |
| Connection Pool | Single writer | Multi-writer (100+ connections) |
| Full-Text Search | FTS5 extension | Native GIN index |
| Max DB Size | ~140TB (theoretical) | Unlimited |

## Query Optimization

### Essential Indexes

Already created in migrations:

```sql
-- User lookups
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_users_tenant_id ON users(tenant_id);

-- OAuth tokens
CREATE INDEX idx_user_oauth_tokens_user_provider ON user_oauth_tokens(user_id, provider);
CREATE INDEX idx_user_oauth_tokens_expires ON user_oauth_tokens(expires_at);

-- API keys
CREATE INDEX idx_api_keys_key_hash ON api_keys(key_hash);
CREATE INDEX idx_api_keys_user_id ON api_keys(user_id);
```

### Performance Tips

**SQLite**:
```sql
-- Enable WAL mode for better concurrency
PRAGMA journal_mode=WAL;

-- Increase cache size
PRAGMA cache_size=-64000;  -- 64MB cache

-- Analyze query plans
EXPLAIN QUERY PLAN SELECT * FROM users WHERE email = ?;
```

**PostgreSQL**:
```sql
-- Analyze table statistics
ANALYZE users;

-- Check index usage
SELECT schemaname, tablename, indexname, idx_scan
FROM pg_stat_user_indexes
ORDER BY idx_scan;

-- Find missing indexes
SELECT schemaname, tablename, attname
FROM pg_stats
WHERE correlation < 0.5
AND schemaname = 'public';
```

## Backup and Restore

### SQLite Backup

```bash
# Hot backup (server running)
sqlite3 data/pierre.db ".backup data/pierre_backup.db"

# Copy file (server stopped)
cp data/pierre.db data/pierre_backup_$(date +%Y%m%d).db
```

### PostgreSQL Backup

```bash
# Dump to file
pg_dump -h localhost -U pierre -d pierre_mcp_server > pierre_backup.sql

# Compressed dump
pg_dump -h localhost -U pierre -d pierre_mcp_server | gzip > pierre_backup.sql.gz

# Custom format (faster restore)
pg_dump -h localhost -U pierre -d pierre_mcp_server -F c > pierre_backup.dump
```

### Restore

```bash
# SQLite
cp pierre_backup.db data/pierre.db

# PostgreSQL
psql -h localhost -U pierre -d pierre_mcp_server < pierre_backup.sql

# Or custom format
pg_restore -h localhost -U pierre -d pierre_mcp_server pierre_backup.dump
```

## Maintenance Tasks

### Vacuum (SQLite)

```sql
-- Rebuild database, reclaim space
VACUUM;

-- Auto-vacuum (set once)
PRAGMA auto_vacuum = FULL;
```

### Vacuum (PostgreSQL)

```sql
-- Reclaim space, update statistics
VACUUM ANALYZE;

-- Full vacuum (requires downtime, rebuilds tables)
VACUUM FULL;
```

### Cleanup Old Data

```sql
-- Delete old API key usage (>90 days)
DELETE FROM api_key_usage WHERE timestamp < datetime('now', '-90 days');

-- Delete expired OAuth auth codes
DELETE FROM oauth2_auth_codes WHERE expires_at < datetime('now');

-- Delete old audit events (>1 year)
DELETE FROM audit_events WHERE created_at < datetime('now', '-365 days');
```

## Security Considerations

### Encryption

**Encrypted Fields**:
- `user_oauth_tokens.access_token`
- `user_oauth_tokens.refresh_token`
- `tenant_oauth_credentials.client_id`
- `tenant_oauth_credentials.client_secret`

Encrypted using `PIERRE_MASTER_ENCRYPTION_KEY` (AES-256).

### Password Hashing

User passwords hashed with bcrypt (cost factor 10):

```rust
use bcrypt::{hash, verify, DEFAULT_COST};

let hash = hash("password", DEFAULT_COST)?;
let valid = verify("password", &hash)?;
```

### API Key Hashing

API keys stored as SHA-256 hash:

```rust
use sha2::{Sha256, Digest};

let mut hasher = Sha256::new();
hasher.update(api_key.as_bytes());
let hash = format!("{:x}", hasher.finalize());
```

### SQL Injection Prevention

Use parameterized queries:

```rust
// SAFE
sqlx::query("SELECT * FROM users WHERE email = ?")
    .bind(email)
    .fetch_one(&pool)
    .await?;

// UNSAFE - Never do this
let query = format!("SELECT * FROM users WHERE email = '{}'", email);
sqlx::query(&query).fetch_one(&pool).await?;
```

## Related Documentation

- [Deployment Guide](../operations/deployment-guide.md) - Production database setup
- [Authentication](06-authentication.md) - JWT and OAuth authentication
- [API Reference](14-api-reference.md) - API endpoints using database
- [Security Guide](17-security-guide.md) - Security best practices
