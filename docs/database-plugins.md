# Database Plugin Architecture

This document describes the database plugin architecture that enables Pierre MCP Server to support both SQLite (local development) and PostgreSQL (cloud deployment).

## Overview

The database plugin architecture provides a trait-based abstraction layer that allows easy switching between different database backends without changing application code.

### Supported Backends

- **SQLite**: Default backend for local development and testing
- **PostgreSQL**: Cloud-ready backend for production deployments

## Architecture

### Core Components

1. **DatabaseProvider Trait** (`src/database_plugins/mod.rs`)
   - Defines the interface that all database backends must implement
   - 40+ methods covering users, OAuth tokens, API keys, A2A protocol, analytics
   - Async trait with full Send + Sync + Clone support

2. **Database Factory** (`src/database_plugins/factory.rs`)
   - Auto-detects database type from connection string
   - Creates appropriate database instance
   - Provides unified enum wrapper for different backends

3. **SQLite Implementation** (`src/database_plugins/sqlite.rs`)
   - Wraps existing SQLite database implementation
   - Maintains backward compatibility
   - Default choice for local development

4. **PostgreSQL Implementation** (`src/database_plugins/postgres.rs`)
   - Full PostgreSQL implementation with native types
   - Optimized for cloud deployment
   - Supports advanced features like JSONB, arrays

## Usage

### Basic Usage

```rust
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};

// Auto-detection based on connection string
let db = Database::new("sqlite:./data/pierre.db", encryption_key).await?;
// or
let db = Database::new("postgresql://user:pass@host/db", encryption_key).await?;

// All methods work the same regardless of backend
let user = db.get_user(user_id).await?;
let token = db.get_strava_token(user_id).await?;
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

## Testing

### SQLite Tests (Default)

```bash
cargo test database_plugins_test
```

### PostgreSQL Tests with Docker

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

### Automated Testing

```bash
# Run complete PostgreSQL test suite
./scripts/test-postgres.sh

# Keep PostgreSQL running for manual testing
./scripts/test-postgres.sh --keep-running
```

## Database Schema

### SQLite Schema
- Uses existing schema from `src/database.rs`
- Backward compatible with current installations
- Supports all current features

### PostgreSQL Schema
- Optimized native PostgreSQL types
- JSONB for flexible data storage
- UUID primary keys with `gen_random_uuid()`
- Proper foreign key constraints
- Performance indexes

Key differences:
- PostgreSQL uses `TIMESTAMPTZ` instead of `TEXT` for timestamps
- PostgreSQL uses `JSONB` instead of `TEXT` for JSON data
- PostgreSQL uses proper `UUID` type instead of `TEXT`
- PostgreSQL uses arrays (`TEXT[]`) for lists

## Environment Configuration

### SQLite (.env)
```env
DATABASE_URL=sqlite:./data/pierre.db
ENCRYPTION_KEY=your_32_byte_base64_encoded_key
```

### PostgreSQL (.env.postgres)
```env
DATABASE_URL=postgresql://user:password@host:5432/database
ENCRYPTION_KEY=your_32_byte_base64_encoded_key
```

## Migration Strategy

### Development to Production
1. Develop with SQLite locally
2. Test with PostgreSQL using Docker
3. Deploy with PostgreSQL in production

### Data Migration
Currently, no automatic migration tools are provided. For production migration:

1. Export data from SQLite
2. Transform data format if needed
3. Import into PostgreSQL
4. Verify data integrity

## Performance Considerations

### SQLite
- Fast for local development
- Single file storage
- No network overhead
- Limited concurrent access

### PostgreSQL
- Optimized for concurrent access
- Network overhead
- Better for multiple users
- Advanced query capabilities
- JSONB performance benefits

## Security

Both backends support:
- AES-256-GCM encryption for OAuth tokens
- Secure password hashing with bcrypt
- SQL injection prevention via parameterized queries
- Connection pooling with timeout protection

## Future Enhancements

Potential additions:
- Redis support for caching
- MongoDB support for document storage  
- Automatic migration tools
- Connection health monitoring
- Read/write splitting
- Database clustering support