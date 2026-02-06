#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Builds and installs the native Pierre app on the iOS Simulator via Xcode
# ABOUTME: Required only for testing native modules (speech recognition, native MMKV)

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

NATIVE_BUNDLE_ID="com.pierre.fitness"
EXPO_PORT="${EXPO_PORT:-8082}"

# Parse args
NO_BUNDLER=false
for arg in "$@"; do
    case $arg in
        --no-bundler)
            NO_BUNDLER=true
            shift
            ;;
    esac
done

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

echo "Building native app for $SIM_NAME ($BOOTED_UDID)..."
echo -e "${YELLOW}This requires Xcode and may take several minutes on first build.${NC}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT/frontend-mobile"

if [ "$NO_BUNDLER" = "true" ]; then
    echo "Building without bundler (Metro must be running separately)..."
    npx expo run:ios --no-bundler --device "$BOOTED_UDID"
else
    echo "Building and starting Metro on port $EXPO_PORT..."
    npx expo run:ios --device "$BOOTED_UDID" --port "$EXPO_PORT"
fi

echo -e "${GREEN}Native app built and installed on $SIM_NAME${NC}"
