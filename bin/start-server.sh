#!/bin/bash
# ABOUTME: Pierre MCP Server startup script with proper environment loading
# ABOUTME: Loads .envrc, creates data directory, and starts server with logging

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Find project root (where Cargo.toml is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo -e "${BLUE}=== Pierre MCP Server Startup ===${NC}"
echo -e "Project root: ${PROJECT_ROOT}"

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
    echo -e "${RED}Please create .envrc with required environment variables${NC}"
    echo -e "${RED}Run: cp .envrc.example .envrc${NC}"
    exit 1
fi

# Validate critical environment variables
MISSING_VARS=()
[ -z "$DATABASE_URL" ] && MISSING_VARS+=("DATABASE_URL")
[ -z "$PIERRE_MASTER_ENCRYPTION_KEY" ] && MISSING_VARS+=("PIERRE_MASTER_ENCRYPTION_KEY")

if [ ${#MISSING_VARS[@]} -ne 0 ]; then
    echo -e "${RED}ERROR: Missing required environment variables:${NC}"
    for var in "${MISSING_VARS[@]}"; do
        echo -e "${RED}  - $var${NC}"
    done
    echo -e "${RED}Please check your .envrc file${NC}"
    exit 1
fi

echo -e "${GREEN}Environment validated successfully${NC}"

# Ensure data directory exists
mkdir -p "$PROJECT_ROOT/data"

# Set sensible defaults
export RUST_LOG="${RUST_LOG:-info}"
export HTTP_PORT="${HTTP_PORT:-8081}"

# Kill any existing server
if pgrep -f "pierre-mcp-server" > /dev/null; then
    echo -e "Stopping existing server..."
    pkill -f "pierre-mcp-server" 2>/dev/null || true
    sleep 2
fi

echo -e "${BLUE}Starting Pierre MCP Server on port ${HTTP_PORT}...${NC}"
echo -e "Log level: ${RUST_LOG}"
echo ""

# Start server - Rust code handles env var validation
cargo run --bin pierre-mcp-server 2>&1 | tee "$PROJECT_ROOT/server.log"
