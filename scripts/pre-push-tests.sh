#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Pre-push validation - Critical path tests (optimized for speed)
# ABOUTME: Runs essential tests in single batch for efficient compilation

set -e

echo "🚀 Pierre MCP Server - Pre-Push Validation (Optimized)"
echo "======================================================="
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
# TIER 1: Schema Validation (must pass before other tests)
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
# TIER 2: Run all critical tests in single batch
# ============================================================================
# Single cargo invocation is faster than parallel batches because:
# - One Cargo lock acquisition
# - One dependency graph resolution
# - Cargo handles compilation parallelism internally
# - --test-threads handles test execution parallelism

echo "🔄 Running critical path tests..."
echo "---------------------------------"
echo ""
echo "Tests: 20 files covering infrastructure, security, protocol, and multi-tenancy"
echo ""

# Run all tests in a single cargo invocation
if cargo test \
    --test routes_health_http_test \
    --test database_test \
    --test crypto_keys_test \
    --test auth_test \
    --test api_keys_test \
    --test jwt_secret_persistence_test \
    --test oauth2_security_test \
    --test security_headers_test \
    --test mcp_compliance_test \
    --test jsonrpc_test \
    --test mcp_tools_unit \
    --test errors_test \
    --test models_test \
    --test database_plugins_test \
    --test simple_integration_test \
    --test tenant_data_isolation \
    --test tenant_context_resolution_test \
    --test a2a_system_user_test \
    --test intelligence_algorithms_test \
    --test rate_limiting_middleware_test \
    --quiet -- --test-threads=4 > "$RESULT_DIR/tests.log" 2>&1; then
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
echo "Total test files: 21 (schema + 20 critical)"
echo "Duration:         ${DURATION}s (~$((DURATION / 60))m $((DURATION % 60))s)"
echo ""

if [ "$TESTS_PASSED" = false ]; then
    echo "❌ Critical path tests failed!"
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
    echo "✅ All critical path tests passed!"
    echo ""
    echo "⚠️  Note: Full test suite will run in CI"
    echo "   To run locally: ./scripts/lint-and-test.sh"
fi
