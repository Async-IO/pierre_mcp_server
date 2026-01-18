#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Pre-push validation - Critical path tests (optimized for speed)
# ABOUTME: Runs essential tests in parallel batches to catch 80% of issues before pushing

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

# Temp directory for parallel job results
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
# TIERS 2-7: Run in parallel batches
# ============================================================================
# Group tests into batches that can run concurrently
# Each batch runs tests in parallel internally (--test-threads=4)

echo "🔄 Running test batches in parallel..."
echo "--------------------------------------"
echo ""

# Batch A: Infrastructure & Security (Tiers 2-3)
run_batch_a() {
    cargo test \
        --test routes_health_http_test \
        --test database_test \
        --test crypto_keys_test \
        --test auth_test \
        --test api_keys_test \
        --test jwt_secret_persistence_test \
        --test oauth2_security_test \
        --test security_headers_test \
        --quiet -- --test-threads=4 2>&1
}

# Batch B: MCP Protocol & Core (Tiers 4-5)
run_batch_b() {
    cargo test \
        --test mcp_compliance_test \
        --test jsonrpc_test \
        --test mcp_tools_unit \
        --test errors_test \
        --test models_test \
        --test database_plugins_test \
        --test simple_integration_test \
        --quiet -- --test-threads=4 2>&1
}

# Batch C: Multi-tenancy & Features (Tiers 6-7)
run_batch_c() {
    cargo test \
        --test tenant_data_isolation \
        --test tenant_context_resolution_test \
        --test a2a_system_user_test \
        --test intelligence_algorithms_test \
        --test rate_limiting_middleware_test \
        --quiet -- --test-threads=4 2>&1
}

# Run all batches in parallel
echo "  [A] Infrastructure & Security (8 tests)"
echo "  [B] MCP Protocol & Core (7 tests)"
echo "  [C] Multi-tenancy & Features (5 tests)"
echo ""
echo -n "Running batches A, B, C in parallel... "

# Start all batches in background
run_batch_a > "$RESULT_DIR/batch_a.log" 2>&1 && touch "$RESULT_DIR/batch_a.ok" &
PID_A=$!

run_batch_b > "$RESULT_DIR/batch_b.log" 2>&1 && touch "$RESULT_DIR/batch_b.ok" &
PID_B=$!

run_batch_c > "$RESULT_DIR/batch_c.log" 2>&1 && touch "$RESULT_DIR/batch_c.ok" &
PID_C=$!

# Wait for all to complete
wait $PID_A $PID_B $PID_C 2>/dev/null || true

# Check results
FAILED=0
FAILED_BATCHES=""

if [ -f "$RESULT_DIR/batch_a.ok" ]; then
    echo -n "A✅ "
else
    echo -n "A❌ "
    FAILED=1
    FAILED_BATCHES="$FAILED_BATCHES A"
fi

if [ -f "$RESULT_DIR/batch_b.ok" ]; then
    echo -n "B✅ "
else
    echo -n "B❌ "
    FAILED=1
    FAILED_BATCHES="$FAILED_BATCHES B"
fi

if [ -f "$RESULT_DIR/batch_c.ok" ]; then
    echo "C✅"
else
    echo "C❌"
    FAILED=1
    FAILED_BATCHES="$FAILED_BATCHES C"
fi

echo ""

# ============================================================================
# Summary
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "=========================================="
echo "Pre-Push Validation Complete"
echo "=========================================="
echo "Total test files: 21"
echo "Batches:          3 (parallel)"
echo "Duration:         ${DURATION}s (~$((DURATION / 60))m $((DURATION % 60))s)"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "❌ Some batches failed:$FAILED_BATCHES"
    echo ""
    echo "Failed batch logs:"

    if [ ! -f "$RESULT_DIR/batch_a.ok" ]; then
        echo ""
        echo "=== Batch A (Infrastructure & Security) ==="
        cat "$RESULT_DIR/batch_a.log" | tail -30
    fi

    if [ ! -f "$RESULT_DIR/batch_b.ok" ]; then
        echo ""
        echo "=== Batch B (MCP Protocol & Core) ==="
        cat "$RESULT_DIR/batch_b.log" | tail -30
    fi

    if [ ! -f "$RESULT_DIR/batch_c.ok" ]; then
        echo ""
        echo "=== Batch C (Multi-tenancy & Features) ==="
        cat "$RESULT_DIR/batch_c.log" | tail -30
    fi

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
