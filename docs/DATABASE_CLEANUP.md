# Database Cleanup Guide

This guide provides instructions for cleaning up Pierre MCP Server databases when you need to start fresh, troubleshoot issues, or switch between different configurations.

## When to Clean Your Database

You should consider cleaning your database when:
- Starting fresh development after configuration changes
- Troubleshooting authentication or permission issues
- Switching between single-tenant and multi-tenant modes
- Migrating between SQLite and PostgreSQL
- Resolving database corruption or migration errors
- Testing with a clean state

## SQLite Cleanup (Local Development)

### Complete Cleanup

Remove all SQLite database files:

```bash
# Remove database files from data directory
rm -f ./data/pierre.db
rm -f ./data/users.db

# Remove any database files in root directory
rm -f *.db
rm -f *.sqlite
```

### Verify Cleanup

```bash
# Check for any remaining database files
find . -name "*.db" -o -name "*.sqlite" | grep -v target | grep -v node_modules
```

### Preserve Secrets (Optional)

If you want to keep your encryption keys and JWT secrets:

```bash
# Backup secrets
cp ./data/encryption.key ./encryption.key.backup
cp ./data/jwt.secret ./jwt.secret.backup

# Clean databases only
rm -f ./data/*.db

# Restore secrets
mv ./encryption.key.backup ./data/encryption.key
mv ./jwt.secret.backup ./data/jwt.secret
```

## PostgreSQL Cleanup (Docker/Production)

### Using Docker Compose

Complete cleanup including volumes:

```bash
# Stop all containers and remove volumes
docker-compose down --volumes --remove-orphans

# Verify volumes are removed
docker volume ls | grep pierre
```

### Manual PostgreSQL Cleanup

If you need to clean the database without removing the container:

```bash
# Connect to PostgreSQL and drop/recreate database
docker exec -it pierre-postgres psql -U postgres -c "DROP DATABASE IF EXISTS pierre_mcp_server;"
docker exec -it pierre-postgres psql -U postgres -c "CREATE DATABASE pierre_mcp_server;"

# Or using DATABASE_URL
psql $DATABASE_URL -c "DROP DATABASE IF EXISTS pierre_mcp_server WITH (FORCE);"
psql $DATABASE_URL -c "CREATE DATABASE pierre_mcp_server;"
```

### Clean Specific Tables

To clean specific tables while preserving others:

```bash
# Clean only user-related tables
docker exec -it pierre-postgres psql -U postgres -d pierre_mcp_server -c "
TRUNCATE TABLE users CASCADE;
TRUNCATE TABLE api_keys CASCADE;
TRUNCATE TABLE oauth_tokens CASCADE;
"

# Clean only activity data
docker exec -it pierre-postgres psql -U postgres -d pierre_mcp_server -c "
TRUNCATE TABLE activities CASCADE;
TRUNCATE TABLE activity_locations CASCADE;
"
```

## Development Workflow

### Quick Fresh Start Script

Create a `scripts/fresh-start.sh` script:

```bash
#!/bin/bash
# Fresh start script for Pierre MCP Server

echo "🧹 Cleaning Pierre MCP Server databases..."

# Stop any running containers
docker-compose down 2>/dev/null || true

# Remove SQLite databases
rm -f ./data/*.db
rm -f *.db

# Remove Docker volumes
docker volume rm pierre_mcp_server_postgres_data 2>/dev/null || true
docker volume rm pierre-data 2>/dev/null || true

echo "✅ Database cleanup complete!"
echo "📝 Run 'cargo run --bin pierre-mcp-server' to start fresh"
```

Make it executable:

```bash
chmod +x scripts/fresh-start.sh
```

### Switching Between Database Types

When switching from SQLite to PostgreSQL:

```bash
# 1. Export data if needed (optional)
# Data export functionality moved to server API endpoints
curl -X GET http://localhost:8081/admin/export-data \
  -H "Authorization: Bearer <ADMIN_TOKEN>" > backup.json

# 2. Clean SQLite
rm -f ./data/*.db

# 3. Start PostgreSQL
docker-compose up -d postgres

# 4. Update configuration
export DATABASE_URL="postgresql://postgres:password@localhost:5432/pierre_mcp_server"

# 5. Import data if needed (optional)
# Data import functionality moved to server API endpoints
curl -X POST http://localhost:8081/admin/import-data \
  -H "Authorization: Bearer <ADMIN_TOKEN>" \
  -H "Content-Type: application/json" \
  --data-binary @backup.json
```

## Troubleshooting

### Permission Issues After Cleanup

If you encounter permission issues after cleanup:

```bash
# Ensure data directory exists with correct permissions
mkdir -p ./data
chmod 755 ./data

# For Docker volumes
docker-compose down -v
docker system prune -f
docker-compose up -d
```

### Database Locked Errors

If you get "database is locked" errors:

```bash
# Find and kill processes using the database
lsof | grep pierre.db
# Kill the process: kill -9 <PID>

# Or simply restart your machine
```

### Migration Errors

If migrations fail after cleanup:

```bash
# Force re-run all migrations
rm -f ./data/*.db
cargo run --bin pierre-mcp-server -- --migrate-only

# For PostgreSQL
docker exec -it pierre-postgres psql -U postgres -d pierre_mcp_server -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
```

## Best Practices

1. **Always backup important data** before cleaning databases
2. **Stop all services** before cleaning to avoid corruption
3. **Verify cleanup** completed successfully before starting services
4. **Document your configuration** when switching between database types
5. **Use version control** for configuration files but never for database files

## Integration with Getting Started

After cleaning your database, follow the standard setup process:

1. Clean databases (as described above)
2. Start the server: `cargo run --bin pierre-mcp-server`
3. Create admin user: `curl -X POST http://localhost:8081/admin/setup`
4. Use admin token from setup response for API access
5. Configure OAuth providers as needed

This ensures you always start from a known, clean state.