#!/bin/bash
# ABOUTME: Pierre MCP Server stop script
# ABOUTME: Gracefully stops any running Pierre server instances

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo -e "${YELLOW}=== Pierre MCP Server Shutdown ===${NC}"

# Find and stop pierre-mcp-server processes
if pgrep -f "pierre-mcp-server" > /dev/null; then
    echo -e "Stopping Pierre MCP Server..."
    pkill -f "pierre-mcp-server" 2>/dev/null || true

    # Wait for graceful shutdown
    sleep 2

    # Force kill if still running
    if pgrep -f "pierre-mcp-server" > /dev/null; then
        echo -e "${YELLOW}Force killing remaining processes...${NC}"
        pkill -9 -f "pierre-mcp-server" 2>/dev/null || true
        sleep 1
    fi

    echo -e "${GREEN}Pierre MCP Server stopped successfully${NC}"
else
    echo -e "${YELLOW}No running Pierre MCP Server instances found${NC}"
fi

# Also clean up any cargo run processes for the server
if pgrep -f "cargo.*pierre-mcp-server" > /dev/null; then
    echo -e "Stopping cargo processes..."
    pkill -f "cargo.*pierre-mcp-server" 2>/dev/null || true
fi

echo -e "${GREEN}Shutdown complete${NC}"
