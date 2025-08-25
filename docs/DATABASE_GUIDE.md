# Database Guide

Complete guide for database configuration, plugin architecture, and data management in Pierre MCP Server.

## Table of Contents

1. [Database Plugin Architecture](#database-plugin-architecture)
2. [Supported Backends](#supported-backends)
3. [Configuration](#configuration)
4. [Testing](#testing)
5. [Migration Strategy](#migration-strategy)

## Database Plugin Architecture

Pierre MCP Server uses a plugin-based database architecture that provides a trait-based abstraction layer for easy switching between different database backends without changing application code.

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                       │
├─────────────────────────────────────────────────────────────┤
│                DatabaseProvider Trait                      │
│              (40+ async methods)                           │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  Database   │  │   SQLite    │  │ PostgreSQL  │          │
│  │   Factory   │  │ Implementation│ Implementation│          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
├─────────────────────────────────────────────────────────────┤
│        ┌─────────────┐            ┌─────────────┐            │
│        │   SQLite    │            │ PostgreSQL  │            │
│        │  Database   │            │  Database   │            │
│        └─────────────┘            └─────────────┘            │
└─────────────────────────────────────────────────────────────┘
```

### Core Components

#### 1. DatabaseProvider Trait (`src/database_plugins/mod.rs`)
- Defines the interface that all database backends must implement
- 40+ methods covering users, OAuth tokens, API keys, A2A protocol, analytics
- Async trait with full Send + Sync + Clone support

#### 2. Database Factory (`src/database_plugins/factory.rs`)
- Auto-detects database type from connection string
- Creates appropriate database instance
- Provides unified enum wrapper for different backends

#### 3. SQLite Implementation (`src/database_plugins/sqlite.rs`)
- Wraps existing SQLite database implementation
- Maintains backward compatibility
- Default choice for local development

#### 4. PostgreSQL Implementation (`src/database_plugins/postgres.rs`)
- Full PostgreSQL implementation with native types
- Optimized for cloud deployment
- Supports advanced features like JSONB, arrays

## Supported Backends

### SQLite (Default)
- **Use Case**: Local development, testing, single-user deployments
- **Benefits**: 
  - Fast for local development
  - Single file storage
  - No network overhead
  - Easy backup and migration
- **Limitations**: 
  - Limited concurrent access
  - Not suitable for high-traffic production

### PostgreSQL
- **Use Case**: Production deployments, multi-user environments
- **Benefits**:
  - Optimized for concurrent access
  - Better for multiple users
  - Advanced query capabilities
  - JSONB performance benefits
  - Enterprise-ready features
- **Considerations**:
  - Network overhead
  - Requires separate database server

## Configuration

### Environment Variables

#### SQLite Configuration
```env
DATABASE_URL=sqlite:./data/pierre.db
ENCRYPTION_KEY=your_32_byte_base64_encoded_key
```

#### PostgreSQL Configuration
```env
DATABASE_URL=postgresql://user:password@host:5432/database
ENCRYPTION_KEY=your_32_byte_base64_encoded_key
```

### Feature Flags

Add PostgreSQL support to your `Cargo.toml`:

```toml
[features]
default = ["sqlite"]
sqlite = []
postgresql = ["sqlx/postgres"]
```

Enable PostgreSQL:
```bash
cargo build --features postgresql
```

### Basic Usage

```rust
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use pierre_mcp_server::mcp::multitenant::ServerResources;

// ✅ CORRECT: Database created once at startup in main binary
let database = Database::new("sqlite:./data/pierre.db", encryption_key).await?;

// Create ServerResources with shared database
let resources = Arc::new(ServerResources {
    database: Arc::new(database),
    // ... other resources
});

// ✅ CORRECT: Components receive ServerResources, access database through shared reference
let server = MultiTenantMcpServer::new(resources.clone());

// All methods work the same regardless of backend
let user = resources.database.get_user(user_id).await?;
let token = resources.database.get_strava_token(user_id).await?;

// ❌ ANTI-PATTERN: Never create database instances in components
// let db = Database::new(...).await?; // Don't do this!
```

## Testing

### SQLite Tests (Default)

```bash
cargo test database_plugins_test
```

### PostgreSQL Tests with Docker

#### Manual Setup

1. Start PostgreSQL:
```bash
./scripts/test-postgres.sh --keep-running
```

2. Run tests:
```bash
export DATABASE_URL="postgresql://pierre:pierre_dev_password@localhost:5432/pierre_mcp_server"
export ENCRYPTION_KEY="$(openssl rand -base64 32)"
cargo test --features postgresql database_plugins_test
```

3. Clean up:
```bash
docker-compose -f docker-compose.postgres.yml down --volumes
```

#### Automated Testing

```bash
# Run complete PostgreSQL test suite
./scripts/test-postgres.sh

# Keep PostgreSQL running for manual testing
./scripts/test-postgres.sh --keep-running
```

### Test Coverage

The database plugin tests cover:
- User management (creation, retrieval, updates)
- OAuth token storage and encryption
- API key management with rate limiting
- A2A protocol authentication
- Analytics and usage tracking
- Error handling and edge cases

## Database Schema

### SQLite Schema
- Uses existing schema from `src/database.rs`
- Backward compatible with current installations
- Supports all current features
- Text-based storage for JSON and timestamps

### PostgreSQL Schema
- Optimized native PostgreSQL types
- JSONB for flexible data storage
- UUID primary keys with `gen_random_uuid()`
- Proper foreign key constraints
- Performance indexes

#### Key Schema Differences

| Feature | SQLite | PostgreSQL |
|---------|---------|-------------|
| Timestamps | TEXT (ISO 8601) | TIMESTAMPTZ |
| JSON Data | TEXT | JSONB |
| Primary Keys | TEXT (UUID) | UUID |
| Arrays | TEXT (JSON) | TEXT[] |
| Indexes | Basic | Advanced (GIN, partial) |

#### Example Table Definitions

**SQLite**:
```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY,
    email TEXT UNIQUE NOT NULL,
    created_at TEXT NOT NULL
);
```

**PostgreSQL**:
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

## Migration Strategy

### Development to Production

1. **Local Development**: Use SQLite for fast iteration
2. **Testing**: Test with PostgreSQL using Docker
3. **Production**: Deploy with PostgreSQL for scalability

### Data Migration

Currently, no automatic migration tools are provided. For production migration:

1. **Export Data**: Extract data from SQLite database
2. **Transform Format**: Convert data types if needed (timestamps, JSON)
3. **Import Data**: Load data into PostgreSQL
4. **Verify Integrity**: Confirm all data migrated correctly

#### Migration Script Example

```bash
#!/bin/bash
# Example migration approach

# Export SQLite data
sqlite3 ./data/pierre.db ".mode csv" ".output users.csv" "SELECT * FROM users;"

# Transform and import to PostgreSQL
psql $DATABASE_URL -c "COPY users FROM './users.csv' WITH CSV;"
```

## Performance Considerations

### SQLite Performance
- **Optimal for**: Single-user, development, small datasets
- **Read Performance**: Excellent for local access
- **Write Performance**: Good for moderate write loads
- **Concurrency**: Limited to single writer
- **Memory Usage**: Minimal overhead

### PostgreSQL Performance
- **Optimal for**: Multi-user, production, large datasets
- **Read Performance**: Excellent with proper indexing
- **Write Performance**: High concurrent write capability
- **Concurrency**: Full ACID compliance with high concurrency
- **Memory Usage**: Configurable, can be optimized for workload

### Performance Tuning

#### PostgreSQL Optimization
```sql
-- Example performance settings
SET shared_buffers = '256MB';
SET effective_cache_size = '1GB';
SET random_page_cost = 1.1;

-- Useful indexes
CREATE INDEX CONCURRENTLY idx_users_email ON users(email);
CREATE INDEX CONCURRENTLY idx_oauth_tokens_user_id ON oauth_tokens(user_id);
CREATE INDEX CONCURRENTLY idx_api_keys_user_id ON api_keys(user_id);
```

## Security

Both database backends implement comprehensive security measures:

### Encryption
- **OAuth Tokens**: AES-256-GCM encryption at rest
- **API Keys**: SHA-256 hashing with salt
- **Connection**: TLS encryption for PostgreSQL connections

### Access Control
- **SQL Injection**: Prevention via parameterized queries
- **Connection Pooling**: Timeout protection and connection limits
- **Authentication**: Secure password hashing with bcrypt

### Security Best Practices

```rust
// Example secure token storage
let encrypted_token = encrypt_token(&oauth_token, &encryption_key)?;
db.store_oauth_token(user_id, provider, encrypted_token).await?;

// Secure API key validation
let key_hash = hash_api_key(&api_key);
let stored_key = db.get_api_key_by_hash(&key_hash).await?;
```

## Monitoring and Maintenance

### Database Health Checks

```rust
// Built-in health check
let health = db.health_check().await?;
println!("Database status: {}", health.status);
```

### Maintenance Tasks

#### SQLite Maintenance
```bash
# Vacuum database to reclaim space
sqlite3 ./data/pierre.db "VACUUM;"

# Analyze for query optimization
sqlite3 ./data/pierre.db "ANALYZE;"
```

#### PostgreSQL Maintenance
```sql
-- Regular maintenance
VACUUM ANALYZE;

-- Check database size
SELECT pg_size_pretty(pg_database_size('pierre_mcp_server'));

-- Monitor active connections
SELECT count(*) FROM pg_stat_activity;
```

## Troubleshooting

### Common Issues

#### Connection Problems
**SQLite**: Ensure file permissions and directory exists
**PostgreSQL**: Verify network connectivity and credentials

#### Performance Issues
**SQLite**: Check for file system performance, consider WAL mode
**PostgreSQL**: Review connection pooling, indexing, and query plans

#### Migration Issues
**Data Type Mismatches**: Ensure proper type conversion between backends
**Encoding**: Verify UTF-8 encoding for text data

### Diagnostic Commands

```bash
# Test database connection
cargo run --bin test-database-connection

# Validate schema
cargo run --bin validate-database-schema

# Performance benchmark
cargo run --bin benchmark-database-operations
```

## Future Enhancements

Planned database improvements:
- **Redis Integration**: Caching layer for improved performance
- **MongoDB Support**: Document storage for flexible data models
- **Migration Tools**: Automated data migration between backends
- **Health Monitoring**: Real-time database health metrics
- **Read/Write Splitting**: Improved scalability for high-traffic applications
- **Backup Automation**: Scheduled backup and recovery tools

This database guide provides comprehensive information for selecting, configuring, and maintaining database backends in Pierre MCP Server.