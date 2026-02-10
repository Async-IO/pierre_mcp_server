#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Complete development environment setup - database, seeds, OAuth users, all servers
# ABOUTME: One command to go from zero to fully running dev environment with test data

set -eo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

# Parse command line args
BUILD_MODE="release"
TARGET_DIR="release"
NATIVE_BUILD=false
STREAM_LOGS=false
SKIP_SYNTHETIC=false
for arg in "$@"; do
    case $arg in
        --debug)
            BUILD_MODE="debug"
            TARGET_DIR="debug"
            shift
            ;;
        --native)
            NATIVE_BUILD=true
            shift
            ;;
        --stream-logs)
            STREAM_LOGS=true
            shift
            ;;
        --no-synthetic)
            SKIP_SYNTHETIC=true
            shift
            ;;
    esac
done

# Project paths
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

# Log directory
LOG_DIR="$PROJECT_ROOT/logs"
mkdir -p "$LOG_DIR"

# Log files
SERVER_LOG="$LOG_DIR/pierre-server.log"
FRONTEND_LOG="$LOG_DIR/frontend.log"
EXPO_LOG="$LOG_DIR/expo.log"

# Ports
SERVER_PORT=8081
FRONTEND_PORT=5173
EXPO_PORT=8082

# Credentials - use .envrc values or defaults
# These are set after sourcing .envrc below
WEB_TEST_EMAIL="webtest@pierre.dev"
WEB_TEST_PASSWORD="WebTest123!"
MOBILE_TEST_EMAIL="mobiletest@pierre.dev"
MOBILE_TEST_PASSWORD="MobileTest1234"
DEMO_PASSWORD="DemoUser123!"

print_step() {
    echo -e "${GREEN}[$1/$TOTAL_STEPS]${NC} $2"
}

TOTAL_STEPS=9

echo ""
echo -e "${BLUE}============================================================================${NC}"
echo -e "${BLUE}   PIERRE DEVELOPMENT ENVIRONMENT SETUP${NC}"
echo -e "${BLUE}   Database + Seeds + OAuth Users + All Servers${NC}"
echo -e "${BLUE}============================================================================${NC}"
echo ""

# Load environment
if [ ! -f "$PROJECT_ROOT/.envrc" ]; then
    echo -e "${RED}ERROR: .envrc not found. Copy from .envrc.example and configure.${NC}"
    echo -e "${RED}Run: cp .envrc.example .envrc${NC}"
    exit 1
fi
set -a
source "$PROJECT_ROOT/.envrc"
set +a

# Validate critical environment variables
MISSING_VARS=()
[ -z "$DATABASE_URL" ] && MISSING_VARS+=("DATABASE_URL")
[ -z "$PIERRE_MASTER_ENCRYPTION_KEY" ] && MISSING_VARS+=("PIERRE_MASTER_ENCRYPTION_KEY")

if [ ${#MISSING_VARS[@]} -ne 0 ]; then
    echo -e "${RED}ERROR: Missing required environment variables:${NC}"
    for var in "${MISSING_VARS[@]}"; do
        echo -e "${RED}  - $var${NC}"
    done
    echo -e "${RED}Please check your .envrc file${NC}"
    exit 1
fi

echo -e "${GREEN}Environment validated successfully${NC}"
echo ""

# Admin credentials from .envrc (with fallback defaults)
ADMIN_EMAIL="${ADMIN_EMAIL:-admin@example.com}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-AdminPassword123}"

# Step 1: Stop all services
print_step 1 "Stopping existing services..."
pkill -f "pierre-mcp-server" 2>/dev/null || true
pkill -f "vite.*frontend" 2>/dev/null || true
pkill -f "expo.*8082" 2>/dev/null || true
pkill -f "@expo/metro" 2>/dev/null || true
sleep 2
echo "    Done"

# Step 2: Reset database
print_step 2 "Resetting database (fresh migrations)..."
DB_PATH="$PROJECT_ROOT/data/users.db"
BACKUP_DIR="$PROJECT_ROOT/data/backups"
mkdir -p "$BACKUP_DIR"

if [ -f "$DB_PATH" ]; then
    BACKUP_NAME="users_$(date +%Y%m%d_%H%M%S).db"
    cp "$DB_PATH" "$BACKUP_DIR/$BACKUP_NAME"
    echo "    Backed up to: $BACKUP_DIR/$BACKUP_NAME"
    rm -f "$DB_PATH" "$DB_PATH-shm" "$DB_PATH-wal"
fi
echo "    Database cleared"

# Step 3: Build binaries
print_step 3 "Building server binaries ($BUILD_MODE mode)..."
if [ "$BUILD_MODE" = "release" ]; then
    cargo build --release --bin pierre-mcp-server --bin pierre-cli --bin seed-coaches --bin seed-demo-data --bin seed-social --bin seed-mobility --bin seed-synthetic-activities 2>&1 | tail -3
else
    cargo build --bin pierre-mcp-server --bin pierre-cli --bin seed-coaches --bin seed-demo-data --bin seed-social --bin seed-mobility --bin seed-synthetic-activities 2>&1 | tail -3
fi
echo "    Build complete"

# Step 4: Run migrations and seeders
# pierre-cli runs database migrations on startup, so no temporary server needed.
# All seeders use direct DB access (no HTTP API required).
print_step 4 "Running migrations and seeders..."

# Create admin user (also runs database migrations)
echo "    Creating admin user (runs migrations)..."
./target/$TARGET_DIR/pierre-cli user create \
    --email "$ADMIN_EMAIL" \
    --password "$ADMIN_PASSWORD" \
    --force 2>&1 | tail -3

# Seed coaches
echo "    Seeding AI coaches (9 personas)..."
./target/$TARGET_DIR/seed-coaches 2>&1 | tail -3

# Seed demo users (direct DB, no server needed)
echo "    Seeding demo users..."
./target/$TARGET_DIR/seed-demo-data --days 30 2>&1 | tail -3

# Seed social data (includes webtest/mobiletest users)
echo "    Seeding social test data..."
./target/$TARGET_DIR/seed-social 2>&1 | tail -3

# Seed mobility data
echo "    Seeding mobility data (stretches, yoga)..."
./target/$TARGET_DIR/seed-mobility 2>&1 | tail -3

# Seed synthetic activities for test users
if [ "$SKIP_SYNTHETIC" != "true" ]; then
    echo "    Seeding synthetic activities for test users..."
    ./target/$TARGET_DIR/seed-synthetic-activities --email "$WEB_TEST_EMAIL" --count 30 --days 30 2>&1 | tail -1
    ./target/$TARGET_DIR/seed-synthetic-activities --email "$MOBILE_TEST_EMAIL" --count 30 --days 30 2>&1 | tail -1
else
    echo "    Skipping synthetic activities (--no-synthetic)"
fi

echo "    All seeders complete"

# Step 5: Start Pierre server
print_step 5 "Starting Pierre MCP Server (port $SERVER_PORT)..."
RUST_LOG=info ./target/$TARGET_DIR/pierre-mcp-server > "$SERVER_LOG" 2>&1 &
SERVER_PID=$!

# Wait for health check
for i in {1..30}; do
    if curl -s -f "http://localhost:$SERVER_PORT/health" > /dev/null 2>&1; then
        echo "    Server ready (PID: $SERVER_PID)"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}    Server failed to start. Check: tail -f $SERVER_LOG${NC}"
        exit 1
    fi
    sleep 1
done

# Step 6: Install frontend dependencies
print_step 6 "Installing frontend dependencies..."
if [ -d "$PROJECT_ROOT/frontend" ]; then
    cd "$PROJECT_ROOT/frontend"
    bun install --frozen-lockfile > /dev/null 2>&1
    echo "    frontend/ dependencies installed"
fi
if [ -d "$PROJECT_ROOT/frontend-mobile" ]; then
    cd "$PROJECT_ROOT/frontend-mobile"
    bun install --frozen-lockfile > /dev/null 2>&1
    echo "    frontend-mobile/ dependencies installed"
fi
cd "$PROJECT_ROOT"

# Step 7: Start web frontend
print_step 7 "Starting Web Frontend (port $FRONTEND_PORT)..."
if [ -d "$PROJECT_ROOT/frontend" ]; then
    cd "$PROJECT_ROOT/frontend"
    bun run dev > "$FRONTEND_LOG" 2>&1 &
    FRONTEND_PID=$!
    cd "$PROJECT_ROOT"
    echo "    Frontend starting (PID: $FRONTEND_PID)"
else
    echo -e "${YELLOW}    frontend/ not found, skipping${NC}"
    FRONTEND_PID=""
fi

# Step 8: Start Expo Mobile
print_step 8 "Starting Expo Mobile (port $EXPO_PORT)..."
if [ -d "$PROJECT_ROOT/frontend-mobile" ]; then
    cd "$PROJECT_ROOT/frontend-mobile"

    if [ "$NATIVE_BUILD" = "true" ]; then
        # --native flag: build native app with Xcode (for speech recognition, native MMKV)
        echo "    Native build requested — using bin/build-native-app.sh..."
        "$PROJECT_ROOT/bin/build-native-app.sh" --no-bundler > "$EXPO_LOG" 2>&1 &
        EXPO_PID=$!
        echo "    Building native iOS app (this may take several minutes)..."
        echo "    Watch progress: tail -f $EXPO_LOG"
        # Start Metro in dev-client mode since native build uses --no-bundler
        npx expo start --dev-client --port "$EXPO_PORT" >> "$EXPO_LOG" 2>&1 &
    else
        # Default: use Expo Go (fast, no Xcode needed)
        # --ios installs Expo Go if missing and launches on simulator
        # --go forces Expo Go mode (not dev client)
        npx expo start --ios --go --port "$EXPO_PORT" > "$EXPO_LOG" 2>&1 &
        EXPO_PID=$!
    fi

    cd "$PROJECT_ROOT"
    echo "    Expo starting (PID: $EXPO_PID)"
else
    echo -e "${YELLOW}    frontend-mobile/ not found, skipping${NC}"
    EXPO_PID=""
fi

# Step 9: Generate admin token
print_step 9 "Generating admin API token..."
ADMIN_LOGIN=$(curl -s -X POST "http://localhost:$SERVER_PORT/oauth/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password&username=$ADMIN_EMAIL&password=$ADMIN_PASSWORD")
ADMIN_TOKEN=$(echo "$ADMIN_LOGIN" | jq -r '.access_token // empty')

if [ -n "$ADMIN_TOKEN" ]; then
    echo "$ADMIN_TOKEN" > "$LOG_DIR/admin-token.txt"
    echo "    Token saved to: $LOG_DIR/admin-token.txt"
else
    echo -e "${YELLOW}    Could not generate token (will work after first login)${NC}"
    ADMIN_TOKEN="(login to generate)"
fi

# Wait for services to stabilize
sleep 2

# Print summary
echo ""
echo -e "${BLUE}============================================================================${NC}"
echo -e "${BLUE}   SETUP COMPLETE${NC}"
echo -e "${BLUE}============================================================================${NC}"
echo ""
echo -e "${CYAN}=== Test Credentials ===${NC}"
echo ""
printf "%-20s %-30s %-20s\n" "User Type" "Email" "Password"
printf "%-20s %-30s %-20s\n" "────────────────────" "──────────────────────────────" "────────────────────"
printf "%-20s %-30s %-20s\n" "Admin" "$ADMIN_EMAIL" "$ADMIN_PASSWORD"
printf "%-20s %-30s %-20s\n" "Web Test" "$WEB_TEST_EMAIL" "$WEB_TEST_PASSWORD"
printf "%-20s %-30s %-20s\n" "Mobile Test" "$MOBILE_TEST_EMAIL" "$MOBILE_TEST_PASSWORD"
printf "%-20s %-30s %-20s\n" "Demo Users" "alice@acme.com, bob@acme.com" "$DEMO_PASSWORD"
echo ""
echo -e "${CYAN}=== Admin API Token ===${NC}"
echo ""
if [ "$ADMIN_TOKEN" != "(login to generate)" ]; then
    echo "${ADMIN_TOKEN:0:60}..."
else
    echo "$ADMIN_TOKEN"
fi
echo ""
echo -e "${CYAN}=== Services ===${NC}"
echo ""
printf "%-15s %-35s %-10s %-8s\n" "Service" "URL" "Status" "PID"
printf "%-15s %-35s %-10s %-8s\n" "───────────────" "───────────────────────────────────" "──────────" "────────"

# Check server
if curl -s -f "http://localhost:$SERVER_PORT/health" > /dev/null 2>&1; then
    printf "%-15s %-35s ${GREEN}%-10s${NC} %-8s\n" "Pierre Server" "http://localhost:$SERVER_PORT" "Running" "$SERVER_PID"
else
    printf "%-15s %-35s ${RED}%-10s${NC} %-8s\n" "Pierre Server" "http://localhost:$SERVER_PORT" "Down" "-"
fi

# Check frontend (port-based: bun/vite may spawn child processes with different PIDs)
if [ -z "$FRONTEND_PID" ]; then
    printf "%-15s %-35s ${YELLOW}%-10s${NC} %-8s\n" "Web Frontend" "http://localhost:$FRONTEND_PORT" "Skipped" "-"
elif curl -s -o /dev/null --connect-timeout 2 "http://localhost:$FRONTEND_PORT" 2>/dev/null; then
    printf "%-15s %-35s ${GREEN}%-10s${NC} %-8s\n" "Web Frontend" "http://localhost:$FRONTEND_PORT" "Running" "$FRONTEND_PID"
else
    printf "%-15s %-35s ${YELLOW}%-10s${NC} %-8s\n" "Web Frontend" "http://localhost:$FRONTEND_PORT" "Starting" "$FRONTEND_PID"
fi

# Check Expo (port-based: bun spawns Metro as a child process with a different PID)
if [ -z "$EXPO_PID" ]; then
    printf "%-15s %-35s ${YELLOW}%-10s${NC} %-8s\n" "Expo Mobile" "http://localhost:$EXPO_PORT" "Skipped" "-"
elif curl -s -o /dev/null --connect-timeout 2 "http://localhost:$EXPO_PORT" 2>/dev/null; then
    printf "%-15s %-35s ${GREEN}%-10s${NC} %-8s\n" "Expo Mobile" "http://localhost:$EXPO_PORT" "Running" "$EXPO_PID"
else
    printf "%-15s %-35s ${YELLOW}%-10s${NC} %-8s\n" "Expo Mobile" "http://localhost:$EXPO_PORT" "Starting" "$EXPO_PID"
fi

echo ""
echo -e "${CYAN}=== Log Files ===${NC}"
echo ""
echo "  Pierre Server:  tail -f $SERVER_LOG"
echo "  Web Frontend:   tail -f $FRONTEND_LOG"
echo "  Expo Mobile:    tail -f $EXPO_LOG"
echo "  All logs:       tail -f $LOG_DIR/*.log"
echo ""
echo -e "${CYAN}=== Quick Commands ===${NC}"
echo ""
echo "  Stop all:       pkill -f pierre-mcp-server; pkill -f vite; pkill -f expo"
echo "  Server only:    ./bin/start-server.sh"
echo "  Reset & start:  ./bin/setup-db-with-seeds-and-oauth-and-start-servers.sh"
echo ""
echo -e "${GREEN}Ready for development!${NC}"
echo ""

# Stream all logs to console if --stream-logs was passed
if [ "$STREAM_LOGS" = "true" ]; then
    echo -e "${CYAN}Streaming all logs (Ctrl+C to stop)...${NC}"
    echo ""
    tail -f "$SERVER_LOG" "$FRONTEND_LOG" "$EXPO_LOG" 2>/dev/null
fi
