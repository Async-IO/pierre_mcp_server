#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

# ABOUTME: Visual testing environment setup script for Chrome DevTools MCP and iOS Simulator MCP
# ABOUTME: Creates loginable test users, friend connections, and shared insights for testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Find project root (where Cargo.toml is)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

echo -e "${BLUE}╔══════════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                   PIERRE VISUAL TESTING ENVIRONMENT SETUP                   ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""

# Load .envrc
ENVRC_PATH="$PROJECT_ROOT/.envrc"
if [ -f "$ENVRC_PATH" ]; then
    echo -e "${GREEN}Loading environment from: ${ENVRC_PATH}${NC}"
    set -a
    source "$ENVRC_PATH"
    set +a
else
    echo -e "${RED}ERROR: .envrc not found at ${ENVRC_PATH}${NC}"
    exit 1
fi

HTTP_PORT=${HTTP_PORT:-8081}

# Test user credentials
ADMIN_EMAIL="admin@example.com"
ADMIN_PASSWORD="AdminPassword123"
WEB_TEST_EMAIL="webtest@pierre.dev"
WEB_TEST_PASSWORD="WebTest123!"
MOBILE_TEST_EMAIL="mobiletest@pierre.dev"
MOBILE_TEST_PASSWORD="MobileTest123!"

echo ""
echo -e "${BLUE}=== Step 1: Checking Server Status ===${NC}"

# Check if server is running
if curl -s -f "http://localhost:$HTTP_PORT/health" > /dev/null 2>&1; then
    echo -e "${GREEN}Server is running on port $HTTP_PORT${NC}"
else
    echo -e "${YELLOW}Server not running. Starting it...${NC}"
    if [ -x "$PROJECT_ROOT/bin/start-server.sh" ]; then
        "$PROJECT_ROOT/bin/start-server.sh" &
        # Wait for server to start
        for i in {1..30}; do
            if curl -s -f "http://localhost:$HTTP_PORT/health" > /dev/null 2>&1; then
                echo -e "${GREEN}Server started successfully${NC}"
                break
            fi
            if [ $i -eq 30 ]; then
                echo -e "${RED}Server failed to start${NC}"
                exit 1
            fi
            sleep 1
        done
    else
        echo -e "${RED}Cannot find bin/start-server.sh${NC}"
        exit 1
    fi
fi

echo ""
echo -e "${BLUE}=== Step 2: Getting Admin Token ===${NC}"

# Get admin token for API calls
ADMIN_LOGIN=$(curl -s -X POST "http://localhost:$HTTP_PORT/oauth/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password&username=$ADMIN_EMAIL&password=$ADMIN_PASSWORD")

ADMIN_TOKEN=$(echo "$ADMIN_LOGIN" | jq -r '.access_token // empty')

if [ -z "$ADMIN_TOKEN" ]; then
    echo -e "${YELLOW}Admin login failed. Running database reset with seeders...${NC}"

    # Run reset-dev-db.sh to create fresh database with all seeders
    if [ -x "$PROJECT_ROOT/bin/reset-dev-db.sh" ]; then
        echo -e "${YELLOW}This will reset the database. Continue? (y/n)${NC}"
        read -r confirm
        if [ "$confirm" = "y" ] || [ "$confirm" = "Y" ]; then
            # Temporarily disable the prompt in reset-dev-db.sh by piping 'yes'
            echo "yes" | "$PROJECT_ROOT/bin/reset-dev-db.sh"

            # Wait for server to restart
            sleep 3

            # Try admin login again
            ADMIN_LOGIN=$(curl -s -X POST "http://localhost:$HTTP_PORT/oauth/token" \
                -H "Content-Type: application/x-www-form-urlencoded" \
                -d "grant_type=password&username=$ADMIN_EMAIL&password=$ADMIN_PASSWORD")
            ADMIN_TOKEN=$(echo "$ADMIN_LOGIN" | jq -r '.access_token // empty')
        else
            echo -e "${RED}Aborted${NC}"
            exit 1
        fi
    fi
fi

if [ -z "$ADMIN_TOKEN" ]; then
    echo -e "${RED}Failed to get admin token${NC}"
    exit 1
fi

echo -e "${GREEN}Admin token acquired${NC}"

echo ""
echo -e "${BLUE}=== Step 3: Verifying Visual Test Users ===${NC}"

# Check if web test user exists and can login
WEB_LOGIN=$(curl -s -X POST "http://localhost:$HTTP_PORT/oauth/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password&username=$WEB_TEST_EMAIL&password=$WEB_TEST_PASSWORD")

WEB_TOKEN=$(echo "$WEB_LOGIN" | jq -r '.access_token // empty')
WEB_USER_ID=$(echo "$WEB_LOGIN" | jq -r '.user.user_id // empty')

if [ -n "$WEB_TOKEN" ]; then
    echo -e "${GREEN}Web test user verified: $WEB_TEST_EMAIL${NC}"
else
    echo -e "${YELLOW}Web test user not found or cannot login. Run reset-dev-db.sh first.${NC}"
fi

# Check if mobile test user exists and can login
MOBILE_LOGIN=$(curl -s -X POST "http://localhost:$HTTP_PORT/oauth/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password&username=$MOBILE_TEST_EMAIL&password=$MOBILE_TEST_PASSWORD")

MOBILE_TOKEN=$(echo "$MOBILE_LOGIN" | jq -r '.access_token // empty')
MOBILE_USER_ID=$(echo "$MOBILE_LOGIN" | jq -r '.user.user_id // empty')

if [ -n "$MOBILE_TOKEN" ]; then
    echo -e "${GREEN}Mobile test user verified: $MOBILE_TEST_EMAIL${NC}"
else
    echo -e "${YELLOW}Mobile test user not found or cannot login. Run reset-dev-db.sh first.${NC}"
fi

echo ""
echo -e "${BLUE}=== Step 4: Setting Up Friend Connection ===${NC}"

if [ -n "$WEB_TOKEN" ] && [ -n "$MOBILE_TOKEN" ] && [ -n "$WEB_USER_ID" ] && [ -n "$MOBILE_USER_ID" ]; then
    # Check if friend connection already exists
    FRIENDS_CHECK=$(curl -s "http://localhost:$HTTP_PORT/api/social/friends" \
        -H "Authorization: Bearer $WEB_TOKEN")

    # Send friend request from web to mobile user
    echo -e "Sending friend request from web to mobile user..."
    FRIEND_REQUEST=$(curl -s -X POST "http://localhost:$HTTP_PORT/api/social/friends/request" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $WEB_TOKEN" \
        -d "{\"receiver_id\": \"$MOBILE_USER_ID\"}")

    REQUEST_ID=$(echo "$FRIEND_REQUEST" | jq -r '.id // .data.id // empty')

    if [ -n "$REQUEST_ID" ]; then
        # Accept friend request from mobile side
        echo -e "Accepting friend request from mobile side..."
        ACCEPT_RESPONSE=$(curl -s -X POST "http://localhost:$HTTP_PORT/api/social/friends/accept/$REQUEST_ID" \
            -H "Authorization: Bearer $MOBILE_TOKEN")
        echo -e "${GREEN}Friend connection established between test users${NC}"
    else
        # May already be friends
        echo -e "${YELLOW}Friend request may already exist or users are already friends${NC}"
    fi
else
    echo -e "${YELLOW}Skipping friend setup - test users not fully available${NC}"
fi

echo ""
echo -e "${BLUE}=== Step 5: Creating Test Insights ===${NC}"

if [ -n "$WEB_TOKEN" ]; then
    # Share an insight from web user
    echo -e "Creating shared insight from web test user..."
    WEB_INSIGHT=$(curl -s -X POST "http://localhost:$HTTP_PORT/api/social/insights" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $WEB_TOKEN" \
        -d '{
            "insight_type": "achievement",
            "title": "Visual Test Achievement",
            "content": "This is a test insight from the web user for visual testing. Shared to verify cross-platform visibility.",
            "visibility": "friends_only",
            "sport_type": "run"
        }')

    if echo "$WEB_INSIGHT" | jq -e '.id // .data.id' > /dev/null 2>&1; then
        echo -e "${GREEN}Web user insight created${NC}"
    else
        echo -e "${YELLOW}Web user insight may already exist or creation skipped${NC}"
    fi
fi

if [ -n "$MOBILE_TOKEN" ]; then
    # Share an insight from mobile user
    echo -e "Creating shared insight from mobile test user..."
    MOBILE_INSIGHT=$(curl -s -X POST "http://localhost:$HTTP_PORT/api/social/insights" \
        -H "Content-Type: application/json" \
        -H "Authorization: Bearer $MOBILE_TOKEN" \
        -d '{
            "insight_type": "training_tip",
            "title": "Mobile Test Training Tip",
            "content": "This is a test insight from the mobile user for visual testing. Cross-platform sync verified!",
            "visibility": "friends_only",
            "sport_type": "ride"
        }')

    if echo "$MOBILE_INSIGHT" | jq -e '.id // .data.id' > /dev/null 2>&1; then
        echo -e "${GREEN}Mobile user insight created${NC}"
    else
        echo -e "${YELLOW}Mobile user insight may already exist or creation skipped${NC}"
    fi
fi

echo ""
echo -e "${BLUE}╔══════════════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${BLUE}║                         VISUAL TESTING ENVIRONMENT READY                    ║${NC}"
echo -e "${BLUE}╚══════════════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${CYAN}=== Test Credentials ===${NC}"
echo ""
echo -e "┌────────────────────┬────────────────────────────┬─────────────────────┐"
echo -e "│ ${CYAN}User Type${NC}          │ ${CYAN}Email${NC}                      │ ${CYAN}Password${NC}            │"
echo -e "├────────────────────┼────────────────────────────┼─────────────────────┤"
echo -e "│ Admin              │ admin@example.com          │ AdminPassword123    │"
echo -e "│ Web Test User      │ webtest@pierre.dev         │ WebTest123!         │"
echo -e "│ Mobile Test User   │ mobiletest@pierre.dev      │ MobileTest123!      │"
echo -e "│ Demo Users         │ alice@acme.com, etc.       │ DemoUser123!        │"
echo -e "└────────────────────┴────────────────────────────┴─────────────────────┘"
echo ""
echo -e "${CYAN}=== Service URLs ===${NC}"
echo ""
echo -e "┌────────────────────┬────────────────────────────┬─────────────────────┐"
echo -e "│ ${CYAN}Service${NC}            │ ${CYAN}URL${NC}                        │ ${CYAN}Port${NC}                │"
echo -e "├────────────────────┼────────────────────────────┼─────────────────────┤"
echo -e "│ Pierre Server      │ http://localhost:8081      │ 8081                │"
echo -e "│ Web Frontend       │ http://localhost:3000      │ 3000                │"
echo -e "│ Mobile (Expo)      │ http://localhost:8082      │ 8082                │"
echo -e "└────────────────────┴────────────────────────────┴─────────────────────┘"
echo ""
echo -e "${CYAN}=== Start Services ===${NC}"
echo ""
echo -e "  ${GREEN}1.${NC} Pierre Server:  ./bin/start-server.sh"
echo -e "  ${GREEN}2.${NC} Web Frontend:   cd frontend && npm run dev"
echo -e "  ${GREEN}3.${NC} iOS Simulator:  open -a Simulator"
echo -e "  ${GREEN}4.${NC} Mobile App:     cd frontend-mobile && bun start"
echo ""
echo -e "${GREEN}Visual testing environment is ready!${NC}"
echo -e "Run Claude Code with the visual testing prompt to execute scenarios."
echo ""
