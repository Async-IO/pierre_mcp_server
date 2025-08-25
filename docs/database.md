# Database Guide

Complete database configuration, architecture, and management guide for Pierre MCP Server.

## Quick Setup

### Development (SQLite - Default)

```bash
# SQLite database auto-created on first startup
cargo run --bin pierre-mcp-server
# Database created at: ./data/users.db
```

### Multi-user Deployment (PostgreSQL)

```bash
# 1. Install PostgreSQL
brew install postgresql  # macOS
sudo apt-get install postgresql postgresql-contrib  # Ubuntu

# 2. Create database
createdb pierre_mcp_server

# 3. Configure connection
export DATABASE_URL="postgresql://user:password@localhost/pierre_mcp_server"

# 4. Run server (auto-migrates on startup)
cargo run --bin pierre-mcp-server
```

## Database Architecture

### Plugin System

Pierre uses a plugin-based architecture supporting multiple database backends:

```
┌─────────────────────────────────────────┐
│         Application Layer               │
├─────────────────────────────────────────┤
│     DatabaseProvider Trait             │
│     (40+ async methods)                 │
├─────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  │
│  │Database │  │ SQLite  │  │ PostgreSQL │
│  │Factory  │  │Provider │  │Provider   │  │
│  └─────────┘  └─────────┘  └─────────┘  │
└─────────────────────────────────────────┘
```

### Auto-Detection

Database type automatically detected from connection string:

```rust
// SQLite
let db = Database::new("sqlite:./data/users.db", encryption_key).await?;

// PostgreSQL  
let db = Database::new("postgresql://user:pass@host/db", encryption_key).await?;
```

## Core Schema

### Users Table

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    display_name TEXT,
    password_hash TEXT NOT NULL,
    tier TEXT NOT NULL DEFAULT 'starter',
    tenant_id TEXT,
    
    -- OAuth tokens (encrypted)
    strava_access_token TEXT,
    strava_refresh_token TEXT,
    strava_expires_at DATETIME,
    strava_scope TEXT,
    strava_nonce TEXT,
    
    fitbit_access_token TEXT,
    fitbit_refresh_token TEXT,
    fitbit_expires_at DATETIME,
    fitbit_scope TEXT,
    fitbit_nonce TEXT,
    
    -- Admin workflow
    is_active BOOLEAN NOT NULL DEFAULT 1,
    user_status TEXT NOT NULL DEFAULT 'pending' 
        CHECK (user_status IN ('pending', 'active', 'suspended')),
    is_admin BOOLEAN NOT NULL DEFAULT 0,
    approved_by TEXT REFERENCES users(id),
    approved_at DATETIME,
    
    -- Timestamps
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_active DATETIME
);
```

### API Keys Table

```sql
CREATE TABLE api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    
    -- Security
    key_hash TEXT NOT NULL,
    key_prefix TEXT NOT NULL,
    
    -- Access control
    tier TEXT NOT NULL DEFAULT 'starter',
    rate_limit_requests INTEGER NOT NULL DEFAULT 1000,
    rate_limit_period TEXT NOT NULL DEFAULT 'month',
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT 1,
    expires_at DATETIME,
    last_used_at DATETIME,
    usage_count INTEGER DEFAULT 0,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Admin Tokens Table

```sql
CREATE TABLE admin_tokens (
    id TEXT PRIMARY KEY,
    service_name TEXT NOT NULL,
    service_description TEXT,
    
    -- Security  
    token_hash TEXT NOT NULL,
    token_prefix TEXT NOT NULL,
    jwt_secret_hash TEXT NOT NULL,
    
    -- Permissions
    permissions TEXT NOT NULL, -- JSON array
    is_super_admin BOOLEAN NOT NULL DEFAULT 0,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT 1,
    expires_at DATETIME,
    last_used_at DATETIME,
    last_used_ip TEXT,
    usage_count INTEGER DEFAULT 0,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### A2A Clients Table

```sql
CREATE TABLE a2a_clients (
    client_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    description TEXT,
    
    -- Security
    client_secret_hash TEXT NOT NULL,
    
    -- Configuration
    capabilities TEXT NOT NULL, -- JSON array
    redirect_uris TEXT, -- JSON array
    contact_email TEXT,
    agent_version TEXT,
    documentation_url TEXT,
    
    -- Status
    is_active BOOLEAN NOT NULL DEFAULT 1,
    last_used_at DATETIME,
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

## Data Encryption

### Two-Tier Key Management

Pierre implements a two-tier encryption system:

1. **Master Encryption Key (MEK)**: From environment variable
2. **Database Encryption Key (DEK)**: Stored encrypted in database

```rust
// Encryption example
let encrypted_token = encrypt_with_dek(&oauth_token, &dek)?;
database.store_encrypted_token(user_id, encrypted_token).await?;
```

### Encrypted Fields

- OAuth access/refresh tokens
- API key values  
- Admin JWT secrets
- A2A client secrets

## Tenant Isolation

### Multi-Tenant Architecture

```sql
-- Tenant-specific tables
CREATE TABLE tenant_oauth_configs (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    client_id TEXT NOT NULL,
    client_secret_encrypted TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scopes TEXT NOT NULL, -- JSON array
    
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    
    UNIQUE(tenant_id, provider)
);
```

### Data Access Patterns

```rust
// Always include tenant_id in queries
async fn get_user_activities(
    &self, 
    tenant_id: Uuid, 
    user_id: Uuid, 
    limit: i32
) -> Result<Vec<Activity>> {
    // Tenant isolation enforced at query level
    sqlx::query_as!(Activity,
        "SELECT * FROM activities WHERE tenant_id = ? AND user_id = ? LIMIT ?",
        tenant_id, user_id, limit
    ).fetch_all(&self.pool).await.map_err(Into::into)
}
```

## Database Operations

### Migrations

Migrations run automatically on startup:

```rust
impl Database {
    pub async fn new(url: &str, encryption_key: Vec<u8>) -> Result<Self> {
        let provider = create_provider(url, encryption_key).await?;
        provider.run_migrations().await?; // Auto-migrate
        Ok(Self { provider })
    }
}
```

### Connection Pooling

```rust
// SQLite: Single connection with WAL mode
let pool = SqlitePool::connect_with(
    SqliteConnectOptions::from_str(database_url)?
        .journal_mode(SqliteJournalMode::Wal)
        .create_if_missing(true)
).await?;

// PostgreSQL: Connection pool
let pool = PgPool::connect_with(
    PgConnectOptions::from_str(database_url)?
        .application_name("pierre-mcp-server")
).await?;
```

## Management Tasks

### Clean Restart

```bash
# Remove all data and start fresh
./scripts/fresh-start.sh
```

### Database Inspection

```bash
# SQLite browser
sqlite3 ./data/users.db
.tables
SELECT * FROM users;

# PostgreSQL
psql -d pierre_mcp_server
\dt
SELECT * FROM users;
```

### Backup and Restore

```bash
# SQLite backup
cp ./data/users.db ./backup/users-$(date +%Y%m%d).db

# PostgreSQL backup
pg_dump pierre_mcp_server > backup-$(date +%Y%m%d).sql
```

### User Management

```bash
# List users
sqlite3 ./data/users.db "SELECT email, user_status, is_admin, created_at FROM users;"

# Approve pending user
sqlite3 ./data/users.db "UPDATE users SET user_status = 'active', approved_at = datetime('now') WHERE email = 'user@example.com';"

# Make user admin
sqlite3 ./data/users.db "UPDATE users SET is_admin = 1 WHERE email = 'admin@example.com';"
```

## Performance Considerations

### SQLite Optimizations

- **WAL Mode**: Enabled for better concurrency
- **Foreign Keys**: Enforced for data integrity  
- **Indexes**: On frequently queried fields

### PostgreSQL Optimizations

- **Connection Pooling**: Configured for concurrent access
- **Prepared Statements**: Used via sqlx
- **Indexes**: B-tree indexes on primary lookups

### Query Patterns

```rust
// Efficient user lookup with caching
async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
    sqlx::query_as!(User,
        "SELECT * FROM users WHERE email = ? AND is_active = 1",
        email
    ).fetch_optional(&self.pool).await.map_err(Into::into)
}
```

## Troubleshooting

### Common Issues

**Database locked (SQLite):**
```bash
# Check for other processes
lsof ./data/users.db
# Or restart with fresh database
rm ./data/users.db
```

**Connection refused (PostgreSQL):**
```bash
# Check PostgreSQL is running
pg_ctl status
# Check connection string
psql $DATABASE_URL
```

**Migration failures:**
```bash
# Check database permissions
ls -la ./data/
# Check migration logs in server output
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

### Debug Queries

```bash
# Enable query logging
RUST_LOG=sqlx=debug cargo run --bin pierre-mcp-server
```

## Advanced Configuration

### Custom Connection Options

```bash
# SQLite with custom options
DATABASE_URL="sqlite:./data/users.db?mode=rwc&cache=shared&_fk=1"

# PostgreSQL with SSL
DATABASE_URL="postgresql://user:pass@host/db?sslmode=require"
```

### Environment Variables

```bash
# Database configuration
DATABASE_URL="sqlite:./data/users.db"           # Connection string
DATABASE_MAX_CONNECTIONS=10                     # Pool size (PostgreSQL)
DATABASE_ENCRYPTION_KEY_PATH="/secrets/db.key" # Key file path
```

### Health Checks

```bash
# Database health endpoint
curl http://localhost:8081/api/health
```

Returns database status and connection info.