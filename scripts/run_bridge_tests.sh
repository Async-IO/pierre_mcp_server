#!/bin/bash

# ABOUTME: Run complete bridge test suite (unit, integration, E2E)
# ABOUTME: Validates bridge functionality from CLI parsing to full MCP Client simulation

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo -e "${BLUE}==== Bridge Test Suite ====${NC}"
echo "Project root: $PROJECT_ROOT"

# Track test results
ALL_TESTS_PASSED=true

# Change to SDK directory
cd "$PROJECT_ROOT/sdk"

echo ""
# Only run npm install if node_modules doesn't exist
if [ ! -d "node_modules" ]; then
    echo -e "${BLUE}==== Installing Dependencies ====${NC}"
    if npm install; then
        echo -e "${GREEN}[OK] Dependencies installed${NC}"
    else
        echo -e "${RED}[FAIL] npm install failed${NC}"
        exit 1
    fi
else
    echo -e "${GREEN}[OK] Dependencies already installed (node_modules exists)${NC}"
fi

echo ""
echo -e "${BLUE}==== Building Bridge ====${NC}"
if npm run build; then
    echo -e "${GREEN}[OK] Bridge built successfully${NC}"
else
    echo -e "${RED}[FAIL] Bridge build failed${NC}"
    exit 1
fi

echo ""
echo -e "${BLUE}==== Running Unit Tests (Fast, No Server Required) ====${NC}"
if npm run test:unit; then
    echo -e "${GREEN}[OK] Unit tests passed${NC}"
else
    echo -e "${RED}[FAIL] Unit tests failed${NC}"
    ALL_TESTS_PASSED=false
fi

echo ""
echo -e "${BLUE}==== Running Integration Tests (Requires Pierre Server) ====${NC}"
if npm run test:integration -- --forceExit; then
    echo -e "${GREEN}[OK] Integration tests passed${NC}"
else
    echo -e "${RED}[FAIL] Integration tests failed${NC}"
    ALL_TESTS_PASSED=false
fi

echo ""
echo -e "${BLUE}==== Running E2E Tests (Full MCP Client Simulation) ====${NC}"
if npm run test:e2e -- --forceExit; then
    echo -e "${GREEN}[OK] E2E tests passed${NC}"
else
    echo -e "${RED}[FAIL] E2E tests failed${NC}"
    ALL_TESTS_PASSED=false
fi

echo ""
if [ "$ALL_TESTS_PASSED" = true ]; then
    echo -e "${GREEN}✅ All Bridge Tests PASSED${NC}"
    exit 0
else
    echo -e "${RED}❌ Some Bridge Tests FAILED${NC}"
    exit 1
fi
