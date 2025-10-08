#!/bin/bash

# ABOUTME: Automated Claude Desktop testing setup script
# ABOUTME: Prepares server, tokens, and config for testing feature/automatic-oauth-reauth branch

set -e

# Parse command line arguments
AUTOMATIC_OAUTH=false
if [ "$1" = "--automatic-oauth" ]; then
    AUTOMATIC_OAUTH=true
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MAIN_WORKTREE="/Users/jeanfrancoisarcand/workspace/strava_ai/pierre_mcp_server"
CLAUDE_CONFIG="$HOME/Library/Application Support/Claude/claude_desktop_config.json"
TOKEN_FILE="$HOME/.pierre-claude-tokens.json"

echo "=========================================="
echo "Claude Desktop Testing Automation"
echo "Branch: feature/automatic-oauth-reauth"
if [ "$AUTOMATIC_OAUTH" = true ]; then
    echo "Mode: Automatic OAuth (skip browser flow)"
else
    echo "Mode: Manual OAuth (test fresh install flow)"
fi
echo "=========================================="
echo ""

# Step 1: Fresh start (clean database, etc.)
echo "Step 1: Running fresh-start.sh..."
cd "$PROJECT_ROOT"
./scripts/fresh-start.sh
echo "✅ Fresh start complete"
echo ""

# Step 2: Build SDK (install dependencies only if needed)
echo "Step 2: Building SDK..."
cd "$PROJECT_ROOT/sdk"

# Only run npm install if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo "Installing dependencies..."
    npm install
    if [ $? -ne 0 ]; then
        echo "❌ npm install failed"
        exit 1
    fi
    echo "✅ Dependencies installed"
else
    echo "✅ Dependencies already installed (node_modules exists)"
fi

npm run build
if [ $? -ne 0 ]; then
    echo "❌ SDK build failed"
    exit 1
fi
echo "✅ SDK built successfully"
echo ""

# Step 3: Kill any server running on port 8081
echo "Step 3: Killing any server on port 8081..."
if lsof -i :8081 -t > /dev/null 2>&1; then
    lsof -i :8081 -t | xargs kill -9 2>/dev/null || true
    sleep 2
    echo "✅ Server on port 8081 killed"
else
    echo "ℹ️  No server running on port 8081"
fi
echo ""

# Step 4: Create data directory for database
echo "Step 4: Creating data directory..."
cd "$PROJECT_ROOT"
mkdir -p data
echo "✅ Data directory created"
echo ""

# Step 5: Start server with environment from main + workflow_test_env
echo "Step 5: Starting Pierre MCP Server with test environment..."
echo "Loading environment from:"
echo "  - $MAIN_WORKTREE/.envrc"
echo "  - $PROJECT_ROOT/.workflow_test_env"
echo ""

# Source main .envrc first
if [ -f "$MAIN_WORKTREE/.envrc" ]; then
    source "$MAIN_WORKTREE/.envrc"
else
    echo "⚠️  Warning: $MAIN_WORKTREE/.envrc not found"
fi

# Source workflow test env (overwrites with test tokens)
# Note: This file is created by complete-user-workflow.sh, so it's OK if it doesn't exist yet
if [ -f "$PROJECT_ROOT/.workflow_test_env" ]; then
    source "$PROJECT_ROOT/.workflow_test_env"
    echo "✅ Loaded existing workflow test environment"
fi

# Start server in background with trace logging
echo "Starting server with RUST_LOG=trace on port ${HTTP_PORT:-8081}..."
echo "Server logs: $PROJECT_ROOT/server.log"
echo ""

# Start cargo in background and capture output
RUST_LOG=trace cargo run --bin pierre-mcp-server > "$PROJECT_ROOT/server.log" 2>&1 &
SERVER_PID=$!

# Show compilation output in real-time
echo "Showing compilation/startup output..."
echo "----------------------------------------"

# Tail the log file while waiting for server to start
MAX_WAIT=180  # 3 minutes for compilation + startup
WAITED=0
LAST_LINE=0

while [ $WAITED -lt $MAX_WAIT ]; do
    # Check if server process is still alive
    if ! kill -0 $SERVER_PID 2>/dev/null; then
        echo ""
        echo "❌ Server process died unexpectedly"
        echo "Last 30 lines of server.log:"
        tail -30 "$PROJECT_ROOT/server.log"
        exit 1
    fi

    # Check if server is responding
    if curl -s "http://localhost:${HTTP_PORT:-8081}/health" > /dev/null 2>&1; then
        echo ""
        echo "✅ Server started successfully after ${WAITED}s (PID: $SERVER_PID)"
        break
    fi

    # Show new lines from log file
    if [ -f "$PROJECT_ROOT/server.log" ]; then
        CURRENT_LINES=$(wc -l < "$PROJECT_ROOT/server.log" 2>/dev/null || echo "0")
        if [ "$CURRENT_LINES" -gt "$LAST_LINE" ]; then
            tail -n +$((LAST_LINE + 1)) "$PROJECT_ROOT/server.log" 2>/dev/null || true
            LAST_LINE=$CURRENT_LINES
        fi
    fi

    sleep 1
    WAITED=$((WAITED + 1))
done

# Final check after timeout
if [ $WAITED -ge $MAX_WAIT ]; then
    echo ""
    echo "❌ Server failed to start within ${MAX_WAIT} seconds"
    echo "Last 40 lines of server.log:"
    tail -40 "$PROJECT_ROOT/server.log"
    kill $SERVER_PID 2>/dev/null || true
    exit 1
fi

echo "----------------------------------------"
echo ""

# Step 5: Run complete-user-workflow.sh to create test user and tokens
echo "Step 5: Running complete-user-workflow.sh..."
cd "$PROJECT_ROOT"
./scripts/complete-user-workflow.sh
echo "✅ User workflow complete"
echo ""

# Re-source .workflow_test_env to get FRESH tokens from complete-user-workflow.sh
echo "Step 5: Loading fresh tokens from workflow..."
if [ -f "$PROJECT_ROOT/.workflow_test_env" ]; then
    source "$PROJECT_ROOT/.workflow_test_env"
    echo "✅ Fresh tokens loaded"
else
    echo "⚠️  Warning: $PROJECT_ROOT/.workflow_test_env not found"
    exit 1
fi
echo ""

# Step 6: Handle token file based on mode
if [ "$AUTOMATIC_OAUTH" = true ]; then
    echo "Step 6: Updating $TOKEN_FILE with fresh token (automatic OAuth mode)..."
    if [ -n "$JWT_TOKEN" ]; then
        cat > "$TOKEN_FILE" <<EOF
{
  "pierre": {
    "access_token": "$JWT_TOKEN",
    "token_type": "Bearer",
    "expires_in": 86400,
    "scope": "read:fitness write:fitness",
    "saved_at": $(date +%s)
  },
  "providers": {}
}
EOF
        echo "✅ Token file updated with JWT from workflow"
        echo "   User ID: $USER_ID"
        echo "   Token expires: $(date -r $(($(date +%s) + 86400)) '+%Y-%m-%d %H:%M:%S')"

        # Validate token format (basic check)
        echo ""
        echo "Validating token format..."
        if echo "$JWT_TOKEN" | grep -qE "^eyJ[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+$"; then
            echo "✅ Token format valid (JWT structure correct)"
            echo "   Full validation will happen when Claude Desktop connects"
        else
            echo "❌ Token format invalid (not a valid JWT)"
            echo "This token will cause 'Internal Server Error' in OAuth flows"
            exit 1
        fi
    else
        echo "❌ Error: JWT_TOKEN not set after workflow, token file not updated"
        exit 1
    fi
else
    echo "Step 6: Removing $TOKEN_FILE to force OAuth flow (manual OAuth mode)..."
    if [ -f "$TOKEN_FILE" ]; then
        rm "$TOKEN_FILE"
        echo "✅ Token file removed - OAuth flow will be triggered on first connect"
    else
        echo "ℹ️  Token file doesn't exist - OAuth flow will be triggered on first connect"
    fi
    echo ""
    echo "When you connect in Claude Desktop:"
    echo "1. Call 'connect_to_pierre' - browser will open for Pierre OAuth"
    echo "2. Authenticate in browser - token will be saved automatically"
    echo "3. Call 'connect_provider' for Strava - second OAuth flow will happen"
fi
echo ""

# Step 7: Update Claude Desktop config to point to this worktree
echo "Step 7: Updating Claude Desktop config..."
CLAUDE_CONFIG_DIR="$(dirname "$CLAUDE_CONFIG")"
mkdir -p "$CLAUDE_CONFIG_DIR"

cat > "$CLAUDE_CONFIG" <<EOF
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": [
        "$PROJECT_ROOT/sdk/dist/cli.js",
        "--server",
        "http://localhost:${HTTP_PORT:-8081}"
      ],
      "env": {}
    }
  }
}
EOF
echo "✅ Claude Desktop config updated to point to: $PROJECT_ROOT/sdk/dist/cli.js"
echo ""

echo "=========================================="
echo "Setup Complete!"
echo "=========================================="
echo ""
echo "Server is running with PID: $SERVER_PID"
echo "Server logs: $PROJECT_ROOT/server.log"
echo ""

# Step 8: Restart Claude Desktop to pick up new config
echo "Step 8: Restarting Claude Desktop..."
echo "Stopping Claude Desktop..."
osascript -e 'quit app "Claude"' 2>/dev/null || true
sleep 2

# Force quit if still running
pkill -9 "Claude" 2>/dev/null || true
sleep 1

echo "Starting Claude Desktop..."
open -a "Claude"
sleep 3
echo "✅ Claude Desktop restarted"
echo ""

echo "=========================================="
echo "Testing Ready!"
echo "=========================================="
echo ""

if [ "$AUTOMATIC_OAUTH" = true ]; then
    echo "Mode: Automatic OAuth (with pre-generated token)"
    echo ""
    echo "Claude Desktop should now show:"
    echo "✅ All 35 tools visible immediately (no connect_to_pierre needed first)"
    echo "✅ You can call any tool right away"
    echo ""
    echo "To verify:"
    echo "1. Check Claude Desktop - should see all tools"
    echo "2. Try: 'What tools do you have available?'"
    echo "3. Try: 'Check my Strava connection status'"
else
    echo "Mode: Manual OAuth (fresh install flow)"
    echo ""
    echo "Claude Desktop should now show:"
    echo "✅ All 35 tools visible immediately (proactive connection caches tools)"
    echo "⚠️  Tools require authentication - you'll need to connect first"
    echo ""
    echo "To test OAuth flow:"
    echo "1. Try: 'Connect to Pierre' - browser will open for OAuth"
    echo "2. Complete authentication in browser"
    echo "3. Try: 'Connect to Strava' - second OAuth flow"
    echo "4. Verify: 'Check my Strava connection status'"
fi

echo ""
echo "Server PID: $SERVER_PID (kill $SERVER_PID to stop)"
echo "Token file: $TOKEN_FILE"
echo "Claude config: $CLAUDE_CONFIG"
echo ""
