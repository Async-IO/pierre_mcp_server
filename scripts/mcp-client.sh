#!/bin/bash
# ABOUTME: MCP client wrapper script for Claude Desktop integration
# ABOUTME: Forwards stdin/stdout to Pierre MCP Server HTTP endpoint

# Set environment variables if not already set
PIERRE_SERVER_URL="${PIERRE_SERVER_URL:-http://127.0.0.1:8080/mcp}"
PIERRE_JWT_TOKEN="${PIERRE_JWT_TOKEN:-}"

if [ -z "$PIERRE_JWT_TOKEN" ]; then
    echo '{"jsonrpc":"2.0","error":{"code":-32000,"message":"PIERRE_JWT_TOKEN environment variable not set"},"id":0}'
    exit 1
fi

# Forward stdin to server and return response
while IFS= read -r line; do
    # Extract ID from the request for proper error responses
    request_id=$(echo "$line" | grep -o '"id":[^,}]*' | cut -d: -f2 | tr -d ' ')
    if [ -z "$request_id" ]; then
        request_id="0"
    fi
    
    response=$(curl -s -X POST "$PIERRE_SERVER_URL" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $PIERRE_JWT_TOKEN" \
        -d "$line" 2>/dev/null)
    
    curl_exit_code=$?
    
    if [ $curl_exit_code -eq 0 ] && [ -n "$response" ]; then
        # Check if response is valid JSON by trying to parse it
        if echo "$response" | jq . >/dev/null 2>&1; then
            echo "$response"
        else
            echo "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32002,\"message\":\"Invalid JSON response from server\"},\"id\":$request_id}"
        fi
    else
        echo "{\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32001,\"message\":\"Failed to connect to Pierre MCP Server\"},\"id\":$request_id}"
    fi
done