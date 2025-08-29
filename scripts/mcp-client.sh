#!/bin/bash
# ABOUTME: MCP client wrapper script for Claude Desktop integration
# ABOUTME: Forwards stdin/stdout to Pierre MCP Server HTTP endpoint

# Set environment variables if not already set
PIERRE_SERVER_URL="${PIERRE_SERVER_URL:-http://127.0.0.1:8080/mcp}"
PIERRE_JWT_TOKEN="${PIERRE_JWT_TOKEN:-}"

if [ -z "$PIERRE_JWT_TOKEN" ]; then
    echo '{"jsonrpc":"2.0","error":{"code":-32000,"message":"PIERRE_JWT_TOKEN environment variable not set"},"id":null}' >&2
    exit 1
fi

# Forward stdin to server and return response
while IFS= read -r line; do
    response=$(curl -s -X POST "$PIERRE_SERVER_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $PIERRE_JWT_TOKEN" \
        -d "$line" 2>/dev/null)
    
    if [ $? -eq 0 ]; then
        echo "$response"
    else
        echo '{"jsonrpc":"2.0","error":{"code":-32001,"message":"Failed to connect to Pierre MCP Server"},"id":null}' >&2
    fi
done