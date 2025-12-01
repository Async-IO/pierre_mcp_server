#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Test script to verify JWT authentication after Claude Code restart
# ABOUTME: Checks config file JWT matches server's expected key ID

set -e

echo "==========================================
JWT Authentication Test
Verifies Claude Code MCP configuration
==========================================
"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if config file exists
if [ ! -f ~/.claude.json ]; then
    echo -e "${RED}❌ Config file not found: ~/.claude.json${NC}"
    exit 1
fi

# Extract JWT header from config file
echo "Step 1: Checking JWT token in ~/.claude.json..."
JWT_HEADER=$(grep "Authorization" ~/.claude.json | grep -o "eyJ[A-Za-z0-9_-]*" | head -1)

if [ -z "$JWT_HEADER" ]; then
    echo -e "${RED}❌ No JWT token found in config file${NC}"
    exit 1
fi

# Decode JWT header to get kid
CONFIG_KID=$(python3 -c "import base64, json; header = '$JWT_HEADER'; decoded = json.loads(base64.urlsafe_b64decode(header + '==').decode('utf-8')); print(decoded['kid'])" 2>/dev/null)

if [ -z "$CONFIG_KID" ]; then
    echo -e "${RED}❌ Failed to decode JWT header${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Config file JWT key ID: ${YELLOW}$CONFIG_KID${NC}"

# Check server logs for authentication attempts
echo ""
echo "Step 2: Checking server logs for recent authentication..."

if [ ! -f server.log ]; then
    echo -e "${YELLOW}⚠${NC}  Server log not found at ./server.log"
    echo "   Make sure the server is running and logging to this location"
    exit 0
fi

# Get most recent authentication attempt
RECENT_AUTH=$(grep "JWT authentication successful\|Key not found in JWKS" server.log | tail -1)

if [ -z "$RECENT_AUTH" ]; then
    echo -e "${YELLOW}⚠${NC}  No recent authentication attempts found in server.log"
    echo "   Server may not have received any MCP requests yet"
    exit 0
fi

# Check if authentication was successful
if echo "$RECENT_AUTH" | grep -q "JWT authentication successful"; then
    echo -e "${GREEN}✓${NC} Most recent authentication: SUCCESS"

    # Extract user ID if available
    USER_ID=$(echo "$RECENT_AUTH" | grep -o "user: [a-f0-9-]*" | cut -d' ' -f2)
    if [ -n "$USER_ID" ]; then
        echo "   User ID: $USER_ID"
    fi

    echo ""
    echo -e "${GREEN}✅ JWT authentication is working correctly!${NC}"
    exit 0
else
    # Authentication failed - check for key mismatch
    echo -e "${RED}✗${NC} Most recent authentication: FAILED"

    # Extract the kid from error message
    FAILED_KID=$(echo "$RECENT_AUTH" | grep -o "key_[0-9_]*" | head -1)

    if [ -n "$FAILED_KID" ]; then
        echo "   Server received JWT with key ID: ${RED}$FAILED_KID${NC}"
        echo "   Config file has key ID: ${YELLOW}$CONFIG_KID${NC}"

        if [ "$FAILED_KID" != "$CONFIG_KID" ]; then
            echo ""
            echo -e "${RED}❌ KEY MISMATCH DETECTED${NC}"
            echo ""
            echo "The server received a JWT with a different key ID than what's in your config file."
            echo "This means Claude Code is using a CACHED old JWT token."
            echo ""
            echo -e "${YELLOW}Solution:${NC}"
            echo "1. Quit Claude Code completely (Cmd+Q on macOS)"
            echo "2. Wait 2-3 seconds"
            echo "3. Relaunch Claude Code"
            echo "4. Run this test again"
            echo ""
            exit 1
        fi
    fi

    echo ""
    echo -e "${RED}❌ JWT authentication failed${NC}"
    echo ""
    echo "Server log shows:"
    echo "$RECENT_AUTH"
    echo ""
    exit 1
fi
