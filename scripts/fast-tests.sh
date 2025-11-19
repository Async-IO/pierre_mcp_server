#!/bin/bash
# ABOUTME: Fast test runner - Unit and quick component tests only (< 5 minutes)
# ABOUTME: Excludes slow E2E, comprehensive, and integration tests

set -e

echo "⚡ Pierre MCP Server - Fast Tests"
echo "=================================="
echo ""
echo "Running unit and fast component tests (excluding slow tests)..."
echo ""

START_TIME=$(date +%s)

# ============================================================================
# STEP 1: Run library unit tests
# ============================================================================
echo "📚 Step 1: Library Unit Tests"
echo "------------------------------"
echo ""

cargo test --lib --quiet && echo "✅ Library unit tests passed" || { echo "❌ Library unit tests failed"; exit 1; }

echo ""

# ============================================================================
# STEP 2: Run fast integration tests (excluding slow patterns)
# ============================================================================
echo "🧪 Step 2: Fast Integration Tests"
echo "----------------------------------"
echo "Excluding: *e2e*, *comprehensive*, *integration*, large routes tests"
echo ""

# Get all test files excluding slow patterns
mapfile -t FAST_TEST_FILES < <(
    find tests -name "*.rs" -not -name "*.disabled" -not -path "*/helpers/*" \
        ! -name "*e2e*.rs" \
        ! -name "*comprehensive*.rs" \
        ! -name "*integration*.rs" \
        ! -name "routes_comprehensive_test.rs" \
        ! -name "routes_dashboard_test.rs" \
        ! -name "routes_admin_test.rs" \
        ! -name "routes_a2a_test.rs" \
        ! -name "oauth_routes_test.rs" \
        ! -name "oauth_token_refresh_test.rs" \
        ! -name "rate_limiting_test.rs" \
        ! -name "mcp_multitenant_test.rs" \
        ! -name "mcp_multitenant_complete_test.rs" \
        ! -name "database_plugins_comprehensive_test.rs" \
        ! -name "protocols_universal_test.rs" \
        ! -name "test_all_tools.rs" \
        ! -name "common.rs" \
        | sort
)

TOTAL=${#FAST_TEST_FILES[@]}
PASSED=0
FAILED=0
CURRENT=0

echo "Found ${TOTAL} fast test files to run"
echo ""

# Run tests with minimal parallelism to balance speed and resource usage
for test_file in "${FAST_TEST_FILES[@]}"; do
    test_name=$(basename "${test_file}" .rs)
    ((CURRENT++))

    echo -n "[$CURRENT/$TOTAL] ${test_name}... "

    if cargo test --test "${test_name}" --quiet -- --test-threads=2 > /dev/null 2>&1; then
        echo "✅"
        ((PASSED++))
    else
        echo "❌"
        ((FAILED++))
        # Show brief error info
        echo "  Error in: ${test_name}"
    fi
done

echo ""

# ============================================================================
# Summary
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=========================================="
echo "Fast Tests Complete"
echo "=========================================="
echo "Integration tests: $TOTAL"
echo "Passed:            $PASSED"
echo "Failed:            $FAILED"
echo "Duration:          ${DURATION}s (~$((DURATION / 60))m $((DURATION % 60))s)"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "❌ Some tests failed."
    echo ""
    echo "To see details, run failed tests individually:"
    echo "  cargo test --test <test_name>"
    echo ""
    echo "To run all tests including slow ones:"
    echo "  ./scripts/safe-test-runner.sh"
    exit 1
else
    echo "✅ All fast tests passed!"
    echo ""
    echo "💡 Skipped slow tests (e2e, comprehensive, large integration tests)"
    echo "   Run full suite with: ./scripts/safe-test-runner.sh"
    echo "   Run pre-push tests: ./scripts/pre-push-tests.sh"
fi
