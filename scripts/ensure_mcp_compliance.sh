#!/bin/bash

# Pierre MCP Compliance Validation Script
# Tests the pierre-claude-bridge against the MCP (Model Context Protocol) specification
# Can be run standalone or called from lint-and-test.sh
#
# Usage: ./scripts/ensure_mcp_compliance.sh

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

echo -e "${BLUE}==== Pierre MCP Compliance Validation ====${NC}"
echo "Project root: $PROJECT_ROOT"

# Track success
COMPLIANCE_PASSED=true

# Track Pierre MCP server PID if we start it
MCP_SERVER_PID=""
SERVER_LOG=""

# Cleanup function - shut down server if we started it
cleanup_mcp_server() {
    if [ -n "$MCP_SERVER_PID" ]; then
        echo ""
        echo -e "${BLUE}==== Shutting down Pierre MCP server (PID: $MCP_SERVER_PID)... ====${NC}"
        kill "$MCP_SERVER_PID" 2>/dev/null || true
        wait "$MCP_SERVER_PID" 2>/dev/null || true
        echo -e "${GREEN}[OK] Pierre MCP server stopped${NC}"
        MCP_SERVER_PID=""
    fi

    # Clean up temp log file
    if [ -n "$SERVER_LOG" ] && [ -f "$SERVER_LOG" ]; then
        rm -f "$SERVER_LOG"
    fi
}

# Register cleanup function to run on exit
trap cleanup_mcp_server EXIT INT TERM

# Change to SDK directory
cd "$PROJECT_ROOT/sdk"

echo ""
echo -e "${BLUE}==== MCP Spec Compliance Validation ====${NC}"

# Check if SDK directory exists
if [ ! -d "." ]; then
    echo -e "${RED}[FAIL] SDK directory not found${NC}"
    exit 1
fi

# Look for Python MCP validator
echo -e "${BLUE}==== Checking for MCP compliance validator (REQUIRED)... ====${NC}"
MCP_VALIDATOR_DIR=""
if [ -d "../validator" ]; then
    # Installed locally in worktree (for testing)
    MCP_VALIDATOR_DIR="../validator"
elif [ -d "$HOME/mcp-validator" ]; then
    MCP_VALIDATOR_DIR="$HOME/mcp-validator"
elif [ -d "./mcp-validator" ]; then
    MCP_VALIDATOR_DIR="./mcp-validator"
elif [ -d "../mcp-validator" ]; then
    MCP_VALIDATOR_DIR="../mcp-validator"
fi

if [ -z "$MCP_VALIDATOR_DIR" ] || [ ! -f "$MCP_VALIDATOR_DIR/mcp_testing/__init__.py" ]; then
    echo -e "${RED}[CRITICAL] Python MCP validator not installed - REQUIRED for MCP compliance${NC}"
    echo -e "${RED}           Per NO EXCEPTIONS POLICY: MCP spec compliance validation is mandatory${NC}"
    echo -e "${RED}           ${NC}"
    echo -e "${RED}           Install with:${NC}"
    echo -e "${RED}             git clone https://github.com/Janix-ai/mcp-validator.git ~/mcp-validator${NC}"
    echo -e "${RED}             cd ~/mcp-validator${NC}"
    echo -e "${RED}             python3 -m venv venv${NC}"
    echo -e "${RED}             source venv/bin/activate${NC}"
    echo -e "${RED}             pip install -r requirements.txt${NC}"
    echo -e "${RED}           ${NC}"
    echo -e "${RED}FAST FAIL: MCP compliance validation is REQUIRED for bridge implementation${NC}"
    exit 1
fi

echo -e "${GREEN}[OK] Python MCP validator found at: $MCP_VALIDATOR_DIR${NC}"

# Build the bridge before testing
echo -e "${BLUE}==== Building pierre-claude-bridge for compliance testing... ====${NC}"
if npm run build; then
    echo -e "${GREEN}[OK] Bridge built successfully${NC}"
else
    echo -e "${RED}[FAIL] Bridge build failed${NC}"
    exit 1
fi

# Check if Pierre MCP server is running (required for bridge testing)
echo -e "${BLUE}==== Checking if Pierre MCP server is accessible... ====${NC}"
SERVER_ALREADY_RUNNING=false
if curl -s -f -m 2 http://localhost:8080/health >/dev/null 2>&1; then
    echo -e "${GREEN}[OK] Pierre MCP server is already running${NC}"
    SERVER_ALREADY_RUNNING=true
else
    echo -e "${YELLOW}[INFO] Pierre MCP server not running - starting it automatically...${NC}"

    # Start Pierre MCP server in background
    echo -e "${BLUE}==== Starting Pierre MCP server for testing... ====${NC}"

    # Check if we have a debug or release binary already (use absolute paths since we're in sdk/)
    SERVER_BINARY=""
    if [ -f "$PROJECT_ROOT/target/release/pierre-mcp-server" ]; then
        SERVER_BINARY="$PROJECT_ROOT/target/release/pierre-mcp-server"
        echo -e "${GREEN}[OK] Using existing release binary${NC}"
    elif [ -f "$PROJECT_ROOT/target/debug/pierre-mcp-server" ]; then
        SERVER_BINARY="$PROJECT_ROOT/target/debug/pierre-mcp-server"
        echo -e "${GREEN}[OK] Using existing debug binary${NC}"
    else
        echo -e "${BLUE}Building pierre-mcp-server (this may take a moment)...${NC}"
        # Build from project root, not from sdk/
        if (cd "$PROJECT_ROOT" && cargo build --bin pierre-mcp-server --quiet 2>&1); then
            SERVER_BINARY="$PROJECT_ROOT/target/debug/pierre-mcp-server"
            echo -e "${GREEN}[OK] Binary built successfully${NC}"
        else
            echo -e "${RED}[FAIL] Failed to build pierre-mcp-server${NC}"
            exit 1
        fi
    fi

    if [ -n "$SERVER_BINARY" ]; then
        # Start server with minimal environment (using CI test key)
        # Redirect to temp log file for debugging startup issues
        SERVER_LOG="/tmp/pierre-mcp-server-$$.log"
        HTTP_PORT=8080 \
        DATABASE_URL=sqlite::memory: \
        PIERRE_MASTER_ENCRYPTION_KEY=rEFe91l6lqLahoyl9OSzum9dKa40VvV5RYj8bHGNTeo= \
        "$SERVER_BINARY" >"$SERVER_LOG" 2>&1 &
        MCP_SERVER_PID=$!

        echo -e "${GREEN}[OK] Pierre MCP server started (PID: $MCP_SERVER_PID)${NC}"
        echo -e "${BLUE}     Server logs: $SERVER_LOG${NC}"

        # Wait for server to be ready (health check)
        echo -e "${BLUE}==== Waiting for Pierre MCP server to be ready... ====${NC}"
        MAX_WAIT=30
        WAIT_COUNT=0
        while [ $WAIT_COUNT -lt $MAX_WAIT ]; do
            # Check if server process is still alive
            if ! kill -0 "$MCP_SERVER_PID" 2>/dev/null; then
                echo -e "${RED}[FAIL] Server process died unexpectedly${NC}"
                echo -e "${RED}       Last 20 lines of server log:${NC}"
                tail -20 "$SERVER_LOG" 2>/dev/null || echo "No log output"
                exit 1
            fi

            if curl -s -f -m 2 http://localhost:8080/health >/dev/null 2>&1; then
                echo -e "${GREEN}[OK] Pierre MCP server is ready (took ${WAIT_COUNT}s)${NC}"
                break
            fi
            sleep 1
            WAIT_COUNT=$((WAIT_COUNT + 1))
        done

        if [ $WAIT_COUNT -ge $MAX_WAIT ]; then
            echo -e "${RED}[FAIL] Pierre MCP server failed to become ready after ${MAX_WAIT}s${NC}"
            echo -e "${RED}       Server process status: $(kill -0 "$MCP_SERVER_PID" 2>/dev/null && echo 'running' || echo 'dead')${NC}"
            echo -e "${RED}       Last 30 lines of server log:${NC}"
            tail -30 "$SERVER_LOG" 2>/dev/null || echo "No log output"
            exit 1
        fi
    fi
fi

# Run MCP compliance tests (REQUIRED - NO EXCEPTIONS POLICY)
echo -e "${BLUE}==== Running MCP protocol compliance tests (REQUIRED)... ====${NC}"
BRIDGE_PATH="$(pwd)/dist/cli.js"
cd "$MCP_VALIDATOR_DIR"

# Use venv Python if available, otherwise system Python
PYTHON_CMD="python3"
if [ -f "venv/bin/python" ]; then
    PYTHON_CMD="venv/bin/python"
fi

# Set PYTHONPATH to include validator directory for module imports
export PYTHONPATH="$MCP_VALIDATOR_DIR"

# Run validator with 5-minute timeout and verbose output
echo -e "${BLUE}     Testing bridge: node $BRIDGE_PATH${NC}"
echo -e "${BLUE}     Protocol version: 2025-06-18${NC}"
echo -e "${BLUE}     Timeout: 300 seconds${NC}"

if timeout 300 $PYTHON_CMD -m mcp_testing.scripts.compliance_report \
    --server-command "node $BRIDGE_PATH" \
    --protocol-version 2025-06-18 \
    --test-timeout 30 \
    --verbose; then
    echo -e "${GREEN}[OK] MCP spec compliance tests passed${NC}"
    cd - >/dev/null
else
    EXIT_CODE=$?
    if [ $EXIT_CODE -eq 124 ]; then
        echo -e "${RED}[FAIL] MCP compliance tests timed out after 5 minutes${NC}"
    else
        echo -e "${RED}[FAIL] MCP spec compliance tests failed (exit code: $EXIT_CODE)${NC}"
    fi
    echo -e "${RED}       Bridge implementation does not meet MCP protocol requirements${NC}"
    cd - >/dev/null
    COMPLIANCE_PASSED=false
fi

# Test with MCP Inspector (CLI mode) for quick validation
# Only run if compliance tests passed
if [ "$COMPLIANCE_PASSED" = true ]; then
    echo -e "${BLUE}==== Running MCP Inspector quick validation... ====${NC}"
    if [ -f "dist/cli.js" ]; then
        # Run inspector in CLI mode with a timeout
        if timeout 10 npx @modelcontextprotocol/inspector --cli node dist/cli.js 2>&1 | grep -q "Connected"; then
            echo -e "${GREEN}[OK] MCP Inspector validation passed${NC}"
        else
            echo -e "${YELLOW}[INFO] MCP Inspector test skipped (requires interactive testing)${NC}"
            echo -e "${YELLOW}       Run 'npm run inspect' in sdk/ directory for manual validation${NC}"
        fi
    else
        echo -e "${YELLOW}[WARN] Bridge not built - skipping inspector validation${NC}"
    fi
fi

echo ""
if [ "$COMPLIANCE_PASSED" = true ]; then
    echo -e "${GREEN}✅ MCP Compliance Validation PASSED${NC}"
    exit 0
else
    echo -e "${RED}❌ MCP Compliance Validation FAILED${NC}"
    echo -e "${RED}    View detailed report: validator/reports/${NC}"
    exit 1
fi
