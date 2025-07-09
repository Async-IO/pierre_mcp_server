#!/bin/bash
# ABOUTME: Fresh start script for Pierre MCP Server database cleanup
# ABOUTME: Removes all database files and Docker volumes for a clean state

set -e

echo "🧹 Cleaning Pierre MCP Server databases..."

# Stop any running containers
echo "📦 Stopping Docker containers..."
docker-compose down 2>/dev/null || true

# Remove SQLite databases
echo "🗑️  Removing SQLite databases..."
rm -f ./data/*.db
rm -f *.db
rm -f *.sqlite

# Remove Docker volumes
echo "🐳 Removing Docker volumes..."
docker volume rm pierre_mcp_server_postgres_data 2>/dev/null || true
docker volume rm pierre-data 2>/dev/null || true

# Create data directory if it doesn't exist
mkdir -p ./data

echo "✅ Database cleanup complete!"
echo "📝 Next steps:"
echo "   - For single-tenant: cargo run --bin pierre-mcp-server -- --single-tenant"
echo "   - For multi-tenant: cargo run --bin pierre-mcp-server"
echo "   - For Docker: docker-compose up"