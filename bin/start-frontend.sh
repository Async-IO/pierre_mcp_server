#!/bin/bash
# ABOUTME: Pierre Frontend startup script with proper environment loading
# ABOUTME: Loads .envrc for VITE_BACKEND_URL and starts Vite dev server

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Find project root (where Cargo.toml is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_DIR="$PROJECT_ROOT/frontend"

echo -e "${BLUE}=== Pierre Frontend Startup ===${NC}"
echo -e "Project root: ${PROJECT_ROOT}"
echo -e "Frontend dir: ${FRONTEND_DIR}"

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

# Validate critical environment variables for frontend
if [ -z "$VITE_BACKEND_URL" ]; then
    echo -e "${RED}WARNING: VITE_BACKEND_URL not set, using default: http://localhost:8081${NC}"
fi

# Check frontend directory exists
if [ ! -d "$FRONTEND_DIR" ]; then
    echo -e "${RED}ERROR: Frontend directory not found at ${FRONTEND_DIR}${NC}"
    exit 1
fi

cd "$FRONTEND_DIR"

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo -e "${BLUE}Installing dependencies...${NC}"
    bun install
fi

# Set default VITE_BACKEND_URL if not set
export VITE_BACKEND_URL="${VITE_BACKEND_URL:-http://localhost:8081}"

echo -e "${BLUE}Starting Vite dev server...${NC}"
echo -e "Backend URL: ${VITE_BACKEND_URL}"
echo ""

bun run dev
