#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Automated Claude Desktop testing setup script
# ABOUTME: Prepares server, tokens, and config for testing feature/automatic-oauth-reauth branch
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

set -e

# Parse command line arguments
AUTOMATIC_OAUTH=false
if [ "$1" = "--automatic-oauth" ]; then
    AUTOMATIC_OAUTH=true
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MAIN_WORKTREE="$PROJECT_ROOT"
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

# Step 1-5: Use unified setup-and-start.sh script
# This handles: fresh-start, build, admin user creation, server start, health check
echo "Step 1-5: Running setup-and-start.sh (unified setup)..."
cd "$PROJECT_ROOT"

# Run the unified setup script with workflow tests
./bin/setup-and-start.sh --run-tests

echo "✅ Setup and workflow complete"
echo ""

# Get the server PID for later reference
SERVER_PID=$(pgrep -f "pierre-mcp-server" | head -1)
echo "Server PID: $SERVER_PID"
echo ""

# Re-source .workflow_test_env to get FRESH tokens from complete-user-workflow.sh
echo "Step 6: Loading fresh tokens from workflow..."
if [ -f "$PROJECT_ROOT/.workflow_test_env" ]; then
    source "$PROJECT_ROOT/.workflow_test_env"
    echo "✅ Fresh tokens loaded"
else
    echo "⚠️  Warning: $PROJECT_ROOT/.workflow_test_env not found"
    exit 1
fi
echo ""

# Step 7: Handle token file based on mode
if [ "$AUTOMATIC_OAUTH" = true ]; then
    echo "Step 7: Updating $TOKEN_FILE with fresh token (automatic OAuth mode)..."
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
    echo "Step 7: Removing $TOKEN_FILE to force OAuth flow (manual OAuth mode)..."
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

# Step 8: Update Claude Desktop config to point to this worktree
echo "Step 8: Updating Claude Desktop config..."
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
      "env": {
        "PIERRE_ALLOW_INTERACTIVE_OAUTH": "true"
      }
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

# Step 9: Restart Claude Desktop to pick up new config
echo "Step 9: Restarting Claude Desktop..."
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
    echo "✅ All 45 tools visible immediately (no connect_to_pierre needed first)"
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
    echo "✅ All 45 tools visible immediately (proactive connection caches tools)"
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
