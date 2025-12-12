#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Unified setup script that initializes database, creates admin user, and starts server
# ABOUTME: Handles the complete bootstrap sequence in the correct order for a fresh environment

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default configuration
ADMIN_EMAIL="${ADMIN_EMAIL:-admin@pierre.mcp}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-adminpass123}"
SKIP_FRESH_START="${SKIP_FRESH_START:-false}"
RUN_WORKFLOW_TESTS="${RUN_WORKFLOW_TESTS:-false}"
WAIT_TIMEOUT=60

# Get script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Load environment
if [ -f .envrc ]; then
    source .envrc
fi

HTTP_PORT=${HTTP_PORT:-8081}

print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --skip-fresh-start    Skip the fresh-start.sh cleanup step"
    echo "  --run-tests           Run complete-user-workflow.sh after startup"
    echo "  --admin-email EMAIL   Admin email (default: admin@pierre.mcp)"
    echo "  --admin-password PWD  Admin password (default: adminpass123)"
    echo "  --help                Show this help message"
    echo ""
    echo "Environment variables:"
    echo "  ADMIN_EMAIL           Admin email address"
    echo "  ADMIN_PASSWORD        Admin password"
    echo "  SKIP_FRESH_START      Set to 'true' to skip cleanup"
    echo "  RUN_WORKFLOW_TESTS    Set to 'true' to run tests after startup"
    echo "  HTTP_PORT             Server port (default: 8081)"
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --skip-fresh-start)
            SKIP_FRESH_START=true
            shift
            ;;
        --run-tests)
            RUN_WORKFLOW_TESTS=true
            shift
            ;;
        --admin-email)
            ADMIN_EMAIL="$2"
            shift 2
            ;;
        --admin-password)
            ADMIN_PASSWORD="$2"
            shift 2
            ;;
        --help)
            print_usage
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $1${NC}"
            print_usage
            exit 1
            ;;
    esac
done

echo -e "${BLUE}=== Pierre MCP Server Setup and Start ===${NC}"
echo -e "${BLUE}Project root: $PROJECT_ROOT${NC}"
echo -e "${BLUE}Server port: $HTTP_PORT${NC}"

# Step 1: Fresh start (optional)
if [ "$SKIP_FRESH_START" = "true" ]; then
    echo -e "\n${YELLOW}Skipping fresh-start.sh (--skip-fresh-start)${NC}"
else
    echo -e "\n${BLUE}=== Step 1: Running fresh-start.sh ===${NC}"
    if [ -x "$PROJECT_ROOT/scripts/fresh-start.sh" ]; then
        "$PROJECT_ROOT/scripts/fresh-start.sh"
        echo -e "${GREEN}Fresh start completed${NC}"
    else
        echo -e "${RED}fresh-start.sh not found or not executable${NC}"
        exit 1
    fi
fi

# Step 2: Build binaries
echo -e "\n${BLUE}=== Step 2: Building server binaries ===${NC}"
cargo build --bin admin-setup --bin pierre-mcp-server 2>&1 | tail -5
echo -e "${GREEN}Build completed${NC}"

# Step 3: Create admin user (before server starts)
echo -e "\n${BLUE}=== Step 3: Creating admin user ===${NC}"
echo -e "Email: $ADMIN_EMAIL"

# Run admin-setup to create the admin user
RUST_LOG=warn cargo run --bin admin-setup -- create-admin-user \
    --email "$ADMIN_EMAIL" \
    --password "$ADMIN_PASSWORD" 2>&1 | grep -E "(Success|Error|✅|❌|Admin|Email|Password)" || true

if [ $? -eq 0 ]; then
    echo -e "${GREEN}Admin user created successfully${NC}"
else
    echo -e "${YELLOW}Admin user may already exist (continuing...)${NC}"
fi

# Step 4: Stop any existing server
echo -e "\n${BLUE}=== Step 4: Stopping any existing server ===${NC}"
pkill -f "pierre-mcp-server" 2>/dev/null || true
sleep 1
pkill -9 -f "pierre-mcp-server" 2>/dev/null || true
echo -e "${GREEN}Existing server processes stopped${NC}"

# Step 5: Start server
echo -e "\n${BLUE}=== Step 5: Starting Pierre MCP Server ===${NC}"
if [ -x "$PROJECT_ROOT/bin/start-server.sh" ]; then
    "$PROJECT_ROOT/bin/start-server.sh" &
    SERVER_PID=$!
    echo -e "Server starting in background (PID: $SERVER_PID)"
else
    echo -e "${YELLOW}bin/start-server.sh not found, starting directly...${NC}"
    RUST_LOG=info cargo run --bin pierre-mcp-server &
    SERVER_PID=$!
fi

# Step 6: Wait for server to be ready
echo -e "\n${BLUE}=== Step 6: Waiting for server health check ===${NC}"
ELAPSED=0
while [ $ELAPSED -lt $WAIT_TIMEOUT ]; do
    if curl -s -f "http://localhost:$HTTP_PORT/health" > /dev/null 2>&1; then
        echo -e "${GREEN}Server is healthy and ready!${NC}"
        break
    fi
    sleep 1
    ELAPSED=$((ELAPSED + 1))
    if [ $((ELAPSED % 5)) -eq 0 ]; then
        echo "  Waiting... ($ELAPSED seconds)"
    fi
done

if [ $ELAPSED -ge $WAIT_TIMEOUT ]; then
    echo -e "${RED}Server failed to start within $WAIT_TIMEOUT seconds${NC}"
    exit 1
fi

# Display server info
echo -e "\n${GREEN}=== Server Started Successfully ===${NC}"
echo -e "Health endpoint: http://localhost:$HTTP_PORT/health"
echo -e "Login page:      http://localhost:$HTTP_PORT/oauth2/login"
echo -e "Admin email:     $ADMIN_EMAIL"
echo -e "Admin password:  $ADMIN_PASSWORD"

# Step 7: Run workflow tests (optional)
if [ "$RUN_WORKFLOW_TESTS" = "true" ]; then
    echo -e "\n${BLUE}=== Step 7: Running workflow tests ===${NC}"
    if [ -x "$PROJECT_ROOT/scripts/complete-user-workflow.sh" ]; then
        "$PROJECT_ROOT/scripts/complete-user-workflow.sh"
    else
        echo -e "${YELLOW}complete-user-workflow.sh not found${NC}"
    fi
fi

echo -e "\n${GREEN}=== Setup Complete ===${NC}"
echo -e "Server is running on http://localhost:$HTTP_PORT"
echo -e "To stop the server: pkill -f pierre-mcp-server"
