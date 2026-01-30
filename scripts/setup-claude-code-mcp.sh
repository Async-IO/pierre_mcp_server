#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Automated setup script for Claude Code sessions with Pierre MCP Server
# ABOUTME: Validates/refreshes JWT token, starts server if needed, updates .envrc

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[0;33m'
NC='\033[0m'

# Find project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo -e "${BLUE}=== Pierre MCP Server - Claude Code Session Setup ===${NC}"
echo -e "Project root: ${PROJECT_ROOT}"
echo ""

# Load current .envrc
ENVRC_PATH="$PROJECT_ROOT/.envrc"
if [ -f "$ENVRC_PATH" ]; then
    set -a
    source "$ENVRC_PATH"
    set +a
else
    echo -e "${RED}ERROR: .envrc not found at ${ENVRC_PATH}${NC}"
    exit 1
fi

# Step 1: Check if server is running
echo -e "${BLUE}Step 1: Checking server status...${NC}"
if curl -s http://localhost:8081/health > /dev/null 2>&1; then
    echo -e "${GREEN}  Server is running${NC}"
    SERVER_RUNNING=true
else
    echo -e "${YELLOW}  Server not running${NC}"
    SERVER_RUNNING=false
fi

# Step 2: Validate current token
echo -e "${BLUE}Step 2: Validating JWT token...${NC}"
TOKEN_VALID=false

if [ -n "$PIERRE_JWT_TOKEN" ]; then
    # Check token expiration locally first
    EXPIRY=$(echo "$PIERRE_JWT_TOKEN" | cut -d'.' -f2 | tr '_-' '/+' | base64 -d 2>/dev/null | python3 -c "
import json,sys
from datetime import datetime
try:
    # Handle padding
    import base64
    data = sys.stdin.read()
    # Add padding if needed
    padding = 4 - len(data) % 4
    if padding != 4:
        data += '=' * padding
    payload = json.loads(base64.b64decode(data.replace('-','+').replace('_','/')))
    exp = payload.get('exp', 0)
    now = datetime.now().timestamp()
    remaining = exp - now
    if remaining > 3600:  # More than 1 hour remaining
        print(f'VALID:{int(remaining/3600)}h')
    elif remaining > 0:
        print(f'EXPIRING:{int(remaining/60)}m')
    else:
        print('EXPIRED')
except Exception as e:
    print(f'ERROR:{e}')
" 2>/dev/null)

    case "$EXPIRY" in
        VALID:*)
            HOURS="${EXPIRY#VALID:}"
            echo -e "${GREEN}  Token valid (${HOURS} remaining)${NC}"
            TOKEN_VALID=true
            ;;
        EXPIRING:*)
            MINS="${EXPIRY#EXPIRING:}"
            echo -e "${YELLOW}  Token expiring soon (${MINS} remaining) - will refresh${NC}"
            ;;
        EXPIRED)
            echo -e "${YELLOW}  Token expired - will refresh${NC}"
            ;;
        *)
            echo -e "${YELLOW}  Could not validate token - will refresh${NC}"
            ;;
    esac
else
    echo -e "${YELLOW}  No token found - will generate${NC}"
fi

# Step 3: Start server if needed
if [ "$SERVER_RUNNING" = false ]; then
    echo -e "${BLUE}Step 3: Starting Pierre MCP server...${NC}"

    # Ensure data directory exists
    mkdir -p "$PROJECT_ROOT/data"

    # Set defaults
    export RUST_LOG="${RUST_LOG:-info}"
    export HTTP_PORT="${HTTP_PORT:-8081}"

    # Kill any zombie processes
    pkill -f "pierre-mcp-server" 2>/dev/null || true
    sleep 1

    # Start server in background
    echo -e "  Starting server on port ${HTTP_PORT}..."
    nohup cargo run --quiet --bin pierre-mcp-server > "$PROJECT_ROOT/server.log" 2>&1 &

    # Wait for server to be ready
    echo -e "  Waiting for server to be ready..."
    for i in {1..30}; do
        if curl -s http://localhost:8081/health > /dev/null 2>&1; then
            echo -e "${GREEN}  Server started successfully${NC}"
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            echo -e "${RED}  Server failed to start - check server.log${NC}"
            exit 1
        fi
    done
else
    echo -e "${BLUE}Step 3: Server already running - skipped${NC}"
fi

# Step 4: Generate new token if needed
if [ "$TOKEN_VALID" = false ]; then
    echo -e "${BLUE}Step 4: Generating new JWT token...${NC}"

    # Generate 7-day token (with timeout to prevent hanging)
    TOKEN_OUTPUT=$(timeout 60 cargo run --bin admin-setup -- generate-token --service claude_code --expires-days 7 2>&1) || true

    # Extract token from output
    NEW_TOKEN=$(echo "$TOKEN_OUTPUT" | grep -o 'eyJ[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*\.[A-Za-z0-9_-]*' | head -1)

    if [ -z "$NEW_TOKEN" ]; then
        echo -e "${YELLOW}  Token generation failed - check for database migration issues${NC}"
        echo -e "${YELLOW}  Error output:${NC}"
        echo "$TOKEN_OUTPUT" | grep -i "error\|failed" | head -3
        echo ""
        echo -e "${YELLOW}  Manual workaround:${NC}"
        echo -e "${YELLOW}  1. Generate token manually: cargo run --bin admin-setup -- generate-token --service claude_code --expires-days 7${NC}"
        echo -e "${YELLOW}  2. Update .envrc with the token${NC}"
        echo -e "${YELLOW}  3. Run: direnv allow${NC}"
        echo ""

        # Check if current token still works despite expiry (server might have cached it)
        if [ -n "$PIERRE_JWT_TOKEN" ]; then
            FALLBACK_CHECK=$(curl -s -w "%{http_code}" -o /dev/null \
                -H "Authorization: Bearer $PIERRE_JWT_TOKEN" \
                -H "Content-Type: application/json" \
                http://localhost:8081/mcp -X POST \
                -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}' 2>/dev/null)

            if [ "$FALLBACK_CHECK" = "200" ]; then
                echo -e "${GREEN}  Current token still works - continuing with existing token${NC}"
                echo -e "${BLUE}Step 5: .envrc update - skipped (using existing token)${NC}"
                # Skip to verification
                NEW_TOKEN=""
            else
                echo -e "${RED}  Current token invalid and cannot generate new one${NC}"
                exit 1
            fi
        else
            exit 1
        fi
    else
        echo -e "${GREEN}  Token generated (expires in 7 days)${NC}"
    fi

    # Update .envrc only if we have a new token
    if [ -n "$NEW_TOKEN" ]; then
    echo -e "${BLUE}Step 5: Updating .envrc...${NC}"

    if grep -q "^export PIERRE_JWT_TOKEN=" "$ENVRC_PATH"; then
        # Update existing line
        sed -i '' "s|^export PIERRE_JWT_TOKEN=.*|export PIERRE_JWT_TOKEN=\"$NEW_TOKEN\"|" "$ENVRC_PATH"
        echo -e "${GREEN}  Updated existing PIERRE_JWT_TOKEN in .envrc${NC}"
    else
        # Add new line
        echo "" >> "$ENVRC_PATH"
        echo "# JWT token for MCP authentication (auto-generated)" >> "$ENVRC_PATH"
        echo "export PIERRE_JWT_TOKEN=\"$NEW_TOKEN\"" >> "$ENVRC_PATH"
        echo -e "${GREEN}  Added PIERRE_JWT_TOKEN to .envrc${NC}"
    fi

    # Export for current session
    export PIERRE_JWT_TOKEN="$NEW_TOKEN"

    # Reload direnv if available
    if command -v direnv &> /dev/null; then
        direnv allow "$PROJECT_ROOT" 2>/dev/null || true
        echo -e "${GREEN}  direnv reloaded${NC}"
    fi
    fi
else
    echo -e "${BLUE}Step 4: Token valid - skipped generation${NC}"
    echo -e "${BLUE}Step 5: .envrc update - skipped${NC}"
fi

# Step 6: Verify everything works
echo -e "${BLUE}Step 6: Verifying setup...${NC}"

VERIFY_RESULT=$(curl -s -w "%{http_code}" -o /tmp/mcp_verify.json \
    -H "Authorization: Bearer $PIERRE_JWT_TOKEN" \
    -H "Content-Type: application/json" \
    http://localhost:8081/mcp -X POST \
    -d '{"jsonrpc":"2.0","method":"tools/list","params":{},"id":1}')

if [ "$VERIFY_RESULT" = "200" ]; then
    TOOL_COUNT=$(python3 -c "import json; print(len(json.load(open('/tmp/mcp_verify.json')).get('result',{}).get('tools',[])))" 2>/dev/null || echo "?")
    echo -e "${GREEN}  MCP endpoint responding - ${TOOL_COUNT} tools available${NC}"
else
    echo -e "${RED}  MCP endpoint returned HTTP ${VERIFY_RESULT}${NC}"
    echo -e "${RED}  Check server.log for details${NC}"
    exit 1
fi

rm -f /tmp/mcp_verify.json

# Step 7: Refresh Stitch MCP token (if gcloud is available)
echo -e "${BLUE}Step 7: Refreshing Stitch MCP token...${NC}"
if command -v gcloud &> /dev/null; then
    STITCH_TOKEN=$(gcloud auth application-default print-access-token 2>/dev/null) || true
    if [ -n "$STITCH_TOKEN" ]; then
        claude mcp remove stitch -s user 2>/dev/null || true
        claude mcp add stitch \
            --transport http https://stitch.googleapis.com/mcp \
            --header "Authorization: Bearer ${STITCH_TOKEN}" \
            --header "X-Goog-User-Project: pierre-fitness-intelligence" \
            -s user 2>/dev/null
        echo -e "${GREEN}  Stitch MCP token refreshed (expires in ~1 hour)${NC}"
    else
        echo -e "${YELLOW}  Could not get Stitch token - run: gcloud auth application-default login${NC}"
    fi
else
    echo -e "${YELLOW}  gcloud not found - Stitch MCP skipped${NC}"
fi

echo ""
echo -e "${GREEN}=== Setup Complete ===${NC}"
echo -e "Server: http://localhost:8081"
echo -e "MCP endpoint: http://localhost:8081/mcp"
echo -e "Pierre Token: Valid for 7 days"
echo -e "Stitch Token: Valid for ~1 hour"
echo ""
echo -e "${YELLOW}NOTE: If using Claude Code built-in MCP, restart the session${NC}"
echo -e "${YELLOW}      to pick up the new PIERRE_JWT_TOKEN environment variable.${NC}"
