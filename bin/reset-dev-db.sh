#!/bin/bash
# ABOUTME: Development database reset script for fixing migration checksum mismatches
# ABOUTME: Backs up, deletes, recreates database and runs seeders - DEVELOPMENT ONLY

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Find project root (where Cargo.toml is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}=== Pierre Development Database Reset ===${NC}"
echo -e "Project root: ${PROJECT_ROOT}"
echo ""

cd "$PROJECT_ROOT"

# Load .envrc
ENVRC_PATH="$PROJECT_ROOT/.envrc"
if [ -f "$ENVRC_PATH" ]; then
    echo -e "${GREEN}Loading environment from: ${ENVRC_PATH}${NC}"
    set -a
    source "$ENVRC_PATH"
    set +a
else
    echo -e "${RED}ERROR: .envrc not found at ${ENVRC_PATH}${NC}"
    exit 1
fi

# Safety check: Refuse to run against production databases
if [[ "$DATABASE_URL" == *"rds.amazonaws.com"* ]] || \
   [[ "$DATABASE_URL" == *"postgres://"* ]] || \
   [[ "$DATABASE_URL" == *"mysql://"* ]] || \
   [[ "$DATABASE_URL" != sqlite:* ]]; then
    echo -e "${RED}ERROR: This script only works with local SQLite databases${NC}"
    echo -e "${RED}DATABASE_URL appears to point to a non-local database${NC}"
    echo -e "${RED}Current DATABASE_URL: ${DATABASE_URL}${NC}"
    exit 1
fi

# Extract database path from URL
DB_PATH="${DATABASE_URL#sqlite:}"
DB_PATH="${DB_PATH#./}"
DB_FULL_PATH="$PROJECT_ROOT/$DB_PATH"

echo -e "Database path: ${DB_FULL_PATH}"
echo ""

# Confirmation prompt
echo -e "${YELLOW}WARNING: This will DELETE your local development database!${NC}"
echo -e "${YELLOW}All user data, OAuth tokens, and usage analytics will be lost.${NC}"
echo ""
read -p "Are you sure you want to continue? (type 'yes' to confirm): " confirm

if [ "$confirm" != "yes" ]; then
    echo -e "${YELLOW}Aborted.${NC}"
    exit 0
fi

# Stop the server if running
if pgrep -f "pierre-mcp-server" > /dev/null; then
    echo -e "${BLUE}Stopping running server...${NC}"
    pkill -f "pierre-mcp-server" 2>/dev/null || true
    sleep 2
fi

# Create backup if database exists
if [ -f "$DB_FULL_PATH" ]; then
    BACKUP_DIR="$PROJECT_ROOT/data/backups"
    mkdir -p "$BACKUP_DIR"
    BACKUP_FILE="$BACKUP_DIR/users_$(date +%Y%m%d_%H%M%S).db"
    echo -e "${BLUE}Backing up database to: ${BACKUP_FILE}${NC}"
    cp "$DB_FULL_PATH" "$BACKUP_FILE"
    echo -e "${GREEN}Backup created successfully${NC}"

    # Delete the database
    echo -e "${BLUE}Deleting database: ${DB_FULL_PATH}${NC}"
    rm -f "$DB_FULL_PATH"
    rm -f "${DB_FULL_PATH}-shm" 2>/dev/null || true
    rm -f "${DB_FULL_PATH}-wal" 2>/dev/null || true
    echo -e "${GREEN}Database deleted${NC}"
else
    echo -e "${YELLOW}No existing database found at ${DB_FULL_PATH}${NC}"
fi

# Ensure data directory exists
mkdir -p "$PROJECT_ROOT/data"

# Run migrations by starting the server briefly (it auto-migrates on startup)
echo -e "${BLUE}Running database migrations...${NC}"
# We use cargo build first to ensure binary is up to date
cargo build --bin pierre-mcp-server --quiet

# Start server in background to run migrations
export RUST_LOG=warn
cargo run --bin pierre-mcp-server &
SERVER_PID=$!

# Wait for migrations to complete (check for health endpoint)
echo -e "Waiting for migrations to complete..."
for i in {1..30}; do
    if curl -s http://localhost:${HTTP_PORT:-8081}/health > /dev/null 2>&1; then
        echo -e "${GREEN}Migrations completed successfully${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}Server startup timed out${NC}"
        kill $SERVER_PID 2>/dev/null || true
        exit 1
    fi
    sleep 1
done

# Stop the server
kill $SERVER_PID 2>/dev/null || true
sleep 1

echo ""
echo -e "${BLUE}Running seeders...${NC}"

# Create admin user
echo -e "${BLUE}Step 1: Creating admin user...${NC}"
RUST_LOG=warn cargo run --bin admin-setup -- create-admin-user \
    --email admin@example.com \
    --password AdminPassword123 || true

# Seed coaches
echo -e "${BLUE}Step 2: Seeding coaches...${NC}"
RUST_LOG=warn cargo run --bin seed-coaches || true

# Seed demo data
echo -e "${BLUE}Step 3: Seeding demo data...${NC}"
RUST_LOG=warn cargo run --bin seed-demo-data -- --days 30 || true

# Seed social data
echo -e "${BLUE}Step 4: Seeding social data...${NC}"
RUST_LOG=warn cargo run --bin seed-social || true

# Seed mobility data
echo -e "${BLUE}Step 5: Seeding mobility data...${NC}"
RUST_LOG=warn cargo run --bin seed-mobility || true

echo ""
echo -e "${GREEN}=== Database Reset Complete ===${NC}"
echo ""
echo -e "Default admin credentials:"
echo -e "  Email: ${BLUE}admin@example.com${NC}"
echo -e "  Password: ${BLUE}AdminPassword123${NC}"
echo ""
echo -e "Start the server with: ${BLUE}./bin/start-server.sh${NC}"
