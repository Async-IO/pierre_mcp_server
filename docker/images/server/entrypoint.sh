#!/bin/bash
# Docker entrypoint script for Pierre MCP Server
set -e

# If .envrc exists, source it for environment variables
if [ -f "/app/.envrc" ]; then
    echo "Loading environment variables from .envrc..."
    # Remove 'export ' prefix and source the variables
    while IFS= read -r line; do
        # Skip empty lines and comments
        [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]] && continue
        
        if [[ $line =~ ^export[[:space:]]+([^=]+)=(.+)$ ]]; then
            var_name="${BASH_REMATCH[1]}"
            var_value="${BASH_REMATCH[2]}"
            # Remove quotes if present
            var_value=$(echo "$var_value" | sed 's/^"//;s/"$//')
            export "$var_name"="$var_value"
        fi
    done < /app/.envrc
    echo "Environment variables loaded successfully"
fi

# Create data directory if it doesn't exist
mkdir -p /app/data

echo "Starting Pierre MCP Server..."
echo "🚀 Multi-tenant MCP server starting on ports $MCP_PORT (MCP) and $HTTP_PORT (HTTP)"

# Debug information
echo "Binary path: $1"
echo "Binary exists: $(test -f "$1" && echo "YES" || echo "NO")"
echo "Binary executable: $(test -x "$1" && echo "YES" || echo "NO")"
echo "Binary file info: $(file "$1" 2>/dev/null || echo "file command failed")"
echo "Current user: $(whoami)"
echo "Current directory: $(pwd)"
echo "Environment variables set:"
env | grep -E "(RUST_LOG|MCP_PORT|HTTP_PORT|DATABASE_URL|ENCRYPTION_KEY_PATH|JWT_SECRET_PATH|JWT_EXPIRY_HOURS|STRAVA_|OPENWEATHER_)" | sort

echo "Starting Pierre MCP Server binary..."

# Execute the server directly
exec "$@"