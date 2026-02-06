#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Installs Expo Go on the booted iOS Simulator if not already present
# ABOUTME: Fast path for development — no Xcode build needed

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

EXPO_GO_BUNDLE_ID="host.exp.Exponent"
EXPO_PORT="${EXPO_PORT:-8082}"

# Find booted simulator
BOOTED_UDID=$(xcrun simctl list devices booted -j 2>/dev/null | python3 -c "
import sys, json
data = json.load(sys.stdin)
for runtime, devices in data.get('devices', {}).items():
    for d in devices:
        if d.get('state') == 'Booted':
            print(d['udid'])
            sys.exit(0)
print('')
" 2>/dev/null)

if [ -z "$BOOTED_UDID" ]; then
    echo -e "${YELLOW}No iOS Simulator is booted. Start one first:${NC}"
    echo "    open -a Simulator"
    exit 1
fi

SIM_NAME=$(xcrun simctl list devices booted -j 2>/dev/null | python3 -c "
import sys, json
data = json.load(sys.stdin)
for runtime, devices in data.get('devices', {}).items():
    for d in devices:
        if d.get('state') == 'Booted':
            print(d.get('name', 'Unknown'))
            sys.exit(0)
" 2>/dev/null)

echo "Simulator: $SIM_NAME ($BOOTED_UDID)"

# Check if Expo Go is already installed
APP_INSTALLED=$(xcrun simctl listapps "$BOOTED_UDID" 2>/dev/null \
    | plutil -convert json -o - - 2>/dev/null \
    | python3 -c "
import sys, json
data = json.load(sys.stdin)
print('yes' if '$EXPO_GO_BUNDLE_ID' in data else 'no')
" 2>/dev/null || echo "no")

if [ "$APP_INSTALLED" = "yes" ]; then
    echo -e "${GREEN}Expo Go is already installed${NC}"
    exit 0
fi

echo "Installing Expo Go on $SIM_NAME..."

# Navigate to frontend-mobile for npx expo context
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT/frontend-mobile"

# Start Metro with --ios --go to install Expo Go and launch it
# Then shut down Metro — the setup script starts it separately
npx expo start --ios --go --port "$EXPO_PORT" &
METRO_PID=$!

# Wait for Expo Go to appear on the simulator (up to 60 seconds)
for i in {1..60}; do
    INSTALLED=$(xcrun simctl listapps "$BOOTED_UDID" 2>/dev/null \
        | plutil -convert json -o - - 2>/dev/null \
        | python3 -c "
import sys, json
data = json.load(sys.stdin)
print('yes' if '$EXPO_GO_BUNDLE_ID' in data else 'no')
" 2>/dev/null || echo "no")

    if [ "$INSTALLED" = "yes" ]; then
        echo -e "${GREEN}Expo Go installed successfully${NC}"
        # Kill the temporary Metro process
        kill "$METRO_PID" 2>/dev/null || true
        wait "$METRO_PID" 2>/dev/null || true
        exit 0
    fi
    sleep 1
done

# Timed out
kill "$METRO_PID" 2>/dev/null || true
echo -e "${YELLOW}Timed out waiting for Expo Go to install. Try running manually:${NC}"
echo "    cd frontend-mobile && npx expo start --ios --go --port $EXPO_PORT"
exit 1
