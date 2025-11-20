#!/bin/bash
# ABOUTME: Pre-push validation - Critical path tests (5-10 minutes)
# ABOUTME: Runs essential tests to catch 80% of issues before pushing to remote

set -e

echo "🚀 Pierre MCP Server - Pre-Push Validation"
echo "==========================================="
echo ""
echo "Running critical path tests to catch issues before push..."
echo ""

START_TIME=$(date +%s)

# Counter for tracking
PASSED=0
FAILED=0
TOTAL=0

# Function to run a test and track results
run_test() {
    local test_name=$1
    local description=$2

    ((TOTAL++))
    echo -n "[$TOTAL] $description... "

    if cargo test --test "$test_name" --quiet -- --test-threads=1 > /dev/null 2>&1; then
        echo "✅"
        ((PASSED++))
        return 0
    else
        echo "❌"
        ((FAILED++))
        echo "   Failed test: $test_name"
        # Show error details
        echo "   Running with output for details:"
        cargo test --test "$test_name" -- --test-threads=1 2>&1 | tail -20 | sed 's/^/   /'
        return 1
    fi
}

# ============================================================================
# TIER 1: Critical Infrastructure (must pass)
# ============================================================================
echo "🔧 Tier 1: Critical Infrastructure"
echo "-----------------------------------"

run_test "routes_health_http_test" "Health endpoints" || exit 1
run_test "database_test" "Database basics" || exit 1
run_test "crypto_keys_test" "Encryption & crypto keys" || exit 1

echo ""

# ============================================================================
# TIER 2: Security & Authentication (must pass)
# ============================================================================
echo "🔒 Tier 2: Security & Authentication"
echo "-------------------------------------"

run_test "auth_test" "Authentication" || exit 1
run_test "api_keys_test" "API key validation" || exit 1
run_test "jwt_secret_persistence_test" "JWT persistence" || exit 1
run_test "oauth2_security_test" "OAuth2 security" || exit 1
run_test "security_headers_test" "Security headers" || exit 1

echo ""

# ============================================================================
# TIER 3: MCP Protocol Compliance (critical for MCP functionality)
# ============================================================================
echo "🔌 Tier 3: MCP Protocol"
echo "-----------------------"

run_test "mcp_compliance_test" "MCP compliance" || exit 1
run_test "jsonrpc_test" "JSON-RPC protocol" || exit 1
run_test "mcp_tools_unit" "MCP tools" || exit 1

echo ""

# ============================================================================
# TIER 4: Core Functionality (important features)
# ============================================================================
echo "⚙️  Tier 4: Core Functionality"
echo "------------------------------"

run_test "errors_test" "Error handling (AppResult)" || exit 1
run_test "models_test" "Data models" || exit 1
run_test "database_plugins_test" "Database plugins (SQLite/Postgres)" || exit 1
run_test "simple_integration_test" "Basic integration" || exit 1

echo ""

# ============================================================================
# TIER 5: Multi-tenancy & Data Isolation (critical for production)
# ============================================================================
echo "🏢 Tier 5: Multi-tenancy"
echo "------------------------"

run_test "tenant_data_isolation" "Tenant isolation" || exit 1
run_test "tenant_context_resolution_test" "Tenant context" || exit 1

echo ""

# ============================================================================
# TIER 6: Protocols & Features (critical features)
# ============================================================================
echo "🔌 Tier 6: Protocols & Features"
echo "--------------------------------"

run_test "a2a_system_user_test" "A2A protocol basics" || exit 1
run_test "intelligence_algorithms_test" "Algorithm correctness" || exit 1
run_test "rate_limiting_middleware_test" "Rate limiting" || exit 1

echo ""

# ============================================================================
# Summary
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=========================================="
echo "Pre-Push Validation Complete"
echo "=========================================="
echo "Total tests:  $TOTAL"
echo "Passed:       $PASSED"
echo "Failed:       $FAILED"
echo "Duration:     ${DURATION}s (~$((DURATION / 60))m $((DURATION % 60))s)"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "❌ Some tests failed. Please fix before pushing."
    echo ""
    echo "To run the full test suite:"
    echo "  ./scripts/lint-and-test.sh"
    echo ""
    echo "To run specific category:"
    echo "  ./scripts/category-test-runner.sh <category>"
    exit 1
else
    echo "✅ All critical path tests passed!"
    echo ""
    echo "⚠️  Note: Full test suite will run in CI"
    echo "   To run locally: ./scripts/lint-and-test.sh"
fi
