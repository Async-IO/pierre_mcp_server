#!/usr/bin/env zsh
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Pre-push validation - Smart test selection based on changed files
# ABOUTME: Only runs tests relevant to modified code for faster feedback

set -e

echo "🚀 Pierre MCP Server - Pre-Push Validation (Smart)"
echo "==================================================="
echo ""

# ============================================================================
# TIER 0: Code Formatting (must pass FIRST - prevents CI failures)
# ============================================================================
echo "🎨 Tier 0: Code Formatting"
echo "--------------------------"
echo -n "Checking cargo fmt... "

if cargo fmt --all -- --check > /dev/null 2>&1; then
    echo "✅"
else
    echo "❌"
    echo ""
    echo "❌ ERROR: Code is not properly formatted!"
    echo ""
    echo "The following files need formatting:"
    cargo fmt --all -- --check 2>&1 | grep "^Diff in" | sed 's/Diff in /  - /'
    echo ""
    echo "To fix this, run:"
    echo "  cargo fmt --all"
    echo ""
    exit 1
fi

echo ""

START_TIME=$(date +%s)

# Temp directory for test results
RESULT_DIR=$(mktemp -d)
trap 'rm -rf "$RESULT_DIR"' EXIT

# ============================================================================
# TIER 1: Schema Validation (always runs - fast and catches structural issues)
# ============================================================================
echo "📋 Tier 1: Schema & Registry Validation"
echo "----------------------------------------"
echo -n "Running schema consistency check... "

if cargo test --test schema_completeness_test --quiet -- --test-threads=4 > /dev/null 2>&1; then
    echo "✅"
else
    echo "❌"
    echo ""
    echo "Schema validation failed. Run: cargo test --test schema_completeness_test"
    exit 1
fi

echo ""

# ============================================================================
# TIER 2: Smart Test Selection Based on Changed Files
# ============================================================================

# Get current branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Determine base for comparison
if git rev-parse --verify "origin/$CURRENT_BRANCH" &>/dev/null; then
    BASE_REF="origin/$CURRENT_BRANCH"
elif git rev-parse --verify "origin/main" &>/dev/null; then
    BASE_REF="origin/main"
else
    BASE_REF="HEAD~1"
fi

# Get changed Rust files
CHANGED_FILES=$(git diff --name-only "$BASE_REF" HEAD 2>/dev/null | grep -E '\.(rs)$' || echo "")

if [ -z "$CHANGED_FILES" ]; then
    echo "📭 No Rust files changed - skipping tests"
    echo ""
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    echo "=========================================="
    echo "Pre-Push Validation Complete"
    echo "=========================================="
    echo "Duration: ${DURATION}s"
    echo "✅ Schema validation passed (no code tests needed)"
    exit 0
fi

echo "🔍 Analyzing changed files..."
echo "-----------------------------"

# Collect tests to run (using associative array to dedupe)
declare -A TESTS_TO_RUN

# Function to add tests for a module
add_tests() {
    for test in "$@"; do
        TESTS_TO_RUN["$test"]=1
    done
}

# Map changed files to relevant tests
for file in $CHANGED_FILES; do
    echo "  📄 $file"

    case "$file" in
        # Database layer
        src/database/*)
            add_tests database_test database_plugins_test tenant_data_isolation
            ;;

        # Authentication & Security
        src/auth/*|src/routes/auth.rs)
            add_tests auth_test api_keys_test jwt_secret_persistence_test oauth2_security_test
            ;;

        # Routes & HTTP
        src/routes/*)
            add_tests routes_health_http_test security_headers_test rate_limiting_middleware_test
            ;;

        # MCP Protocol
        src/protocols/*|src/mcp/*)
            add_tests mcp_compliance_test jsonrpc_test mcp_tools_unit
            ;;

        # Tools
        src/tools/*)
            add_tests mcp_tools_unit
            ;;

        # Intelligence/Algorithms
        src/intelligence/*)
            add_tests intelligence_algorithms_test
            ;;

        # A2A Protocol
        src/a2a/*)
            add_tests a2a_system_user_test
            ;;

        # Models
        src/models/*)
            add_tests models_test
            ;;

        # Errors
        src/errors/*)
            add_tests errors_test
            ;;

        # Crypto
        src/crypto/*)
            add_tests crypto_keys_test
            ;;

        # Tenant/Context
        src/context/*|src/tenant/*)
            add_tests tenant_context_resolution_test tenant_data_isolation
            ;;

        # Config changes - run broader set
        src/config/*)
            add_tests simple_integration_test
            ;;

        # Migrations - database tests
        migrations/*)
            add_tests database_test
            ;;

        # Test files - run the specific test
        tests/*.rs)
            test_name=$(basename "$file" .rs)
            # Only add if it's a known test file (not a helper module)
            if [[ "$test_name" != "common" && "$test_name" != "helpers" && "$test_name" != "fixtures" ]]; then
                add_tests "$test_name"
            fi
            ;;

        # Cargo.toml - schema check is enough (already ran)
        Cargo.toml|Cargo.lock)
            # Schema test already covers dependency changes
            ;;

        # Lib.rs or main modules - run core tests
        src/lib.rs|src/main.rs)
            add_tests simple_integration_test routes_health_http_test
            ;;

        # Binaries
        src/bin/*)
            # Binary changes don't need specific tests in pre-push
            ;;

        # Catch-all for other src/ files
        src/*)
            add_tests simple_integration_test
            ;;
    esac
done

echo ""

# Build the test command
TEST_COUNT=${#TESTS_TO_RUN[@]}

if [ "$TEST_COUNT" -eq 0 ]; then
    echo "📭 No tests mapped for changed files"
    echo ""
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
    echo "=========================================="
    echo "Pre-Push Validation Complete"
    echo "=========================================="
    echo "Duration: ${DURATION}s"
    echo "✅ Schema validation passed"
    exit 0
fi

echo "🔄 Running $TEST_COUNT targeted test file(s)..."
echo "------------------------------------------------"

# Build cargo test arguments
TEST_ARGS=""
for test in "${!TESTS_TO_RUN[@]}"; do
    echo "  🧪 $test"
    TEST_ARGS="$TEST_ARGS --test $test"
done

echo ""

# Run selected tests
if cargo test $TEST_ARGS --quiet -- --test-threads=4 > "$RESULT_DIR/tests.log" 2>&1; then
    TESTS_PASSED=true
else
    TESTS_PASSED=false
fi

# ============================================================================
# Summary
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=========================================="
echo "Pre-Push Validation Complete"
echo "=========================================="
echo "Test files run: $TEST_COUNT (targeted) + 1 (schema)"
echo "Duration:       ${DURATION}s (~$((DURATION / 60))m $((DURATION % 60))s)"
echo ""

if [ "$TESTS_PASSED" = false ]; then
    echo "❌ Targeted tests failed!"
    echo ""
    echo "Test output (last 50 lines):"
    echo "-----------------------------"
    tail -50 "$RESULT_DIR/tests.log"
    echo ""
    echo "To run individual tests for debugging:"
    echo "  cargo test --test <test_name> -- --nocapture"
    echo ""
    exit 1
else
    echo "✅ All targeted tests passed!"
    echo ""
    echo "⚠️  Note: Full test suite will run in CI"
    echo "   To run locally: ./scripts/lint-and-test.sh"
fi
