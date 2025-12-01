#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Category-based test runner for targeted test execution
# ABOUTME: Runs tests by category (mcp, admin, oauth, security, etc.) to prevent OOM

set -euo pipefail

CATEGORY=${1:-}
BATCH_SIZE=3
PAUSE_SECONDS=2

if [[ -z "${CATEGORY}" ]]; then
    echo "Usage: $0 <category>"
    echo ""
    echo "Available categories:"
    echo "  mcp          - MCP server tests"
    echo "  admin        - Admin functionality tests"
    echo "  oauth        - OAuth2 tests"
    echo "  security     - Security tests"
    echo "  database     - Database tests"
    echo "  intelligence - Intelligence/analytics tests"
    echo "  config       - Configuration tests"
    echo "  auth         - Authentication tests"
    echo "  integration  - Integration tests"
    echo "  all          - All tests (use safe-test-runner.sh instead)"
    exit 1
fi

# Define test patterns for each category
declare -A PATTERNS=(
    ["mcp"]="mcp_*"
    ["admin"]="admin_*"
    ["oauth"]="oauth*"
    ["security"]="security_* enterprise_security_*"
    ["database"]="database_*"
    ["intelligence"]="intelligence_* sleep_* training_* performance_*"
    ["config"]="config* analysis_config_*"
    ["auth"]="auth_* jwt_* api_key*"
    ["integration"]="*integration* *e2e*"
)

if [[ ! -v PATTERNS["${CATEGORY}"] ]]; then
    echo "Error: Unknown category '${CATEGORY}'"
    echo "Run '$0' without arguments to see available categories"
    exit 1
fi

PATTERN=${PATTERNS["${CATEGORY}"]}

echo "=========================================="
echo "Category Test Runner: ${CATEGORY}"
echo "=========================================="
echo ""

# Find matching test files
mapfile -t TEST_FILES < <(
    for pat in ${PATTERN}; do
        find tests -name "${pat}.rs" -not -name "*.disabled" 2>/dev/null
    done | sort -u
)

if [[ ${#TEST_FILES[@]} -eq 0 ]]; then
    echo "No test files found for category '${CATEGORY}'"
    exit 1
fi

echo "Found ${#TEST_FILES[@]} test file(s) in category '${CATEGORY}'"
echo ""

PASSED=0
FAILED=0
CURRENT=0

# Run tests
for test_file in "${TEST_FILES[@]}"; do
    test_name=$(basename "${test_file}" .rs)
    ((CURRENT++))

    echo -n "[${CURRENT}/${#TEST_FILES[@]}] Running ${test_name}... "

    if cargo test --test "${test_name}" --quiet -- --test-threads=1 > /dev/null 2>&1; then
        echo "✓ PASSED"
        ((PASSED++))
    else
        echo "✗ FAILED"
        ((FAILED++))
        # Show error details
        echo "  Running with output for details:"
        cargo test --test "${test_name}" -- --test-threads=1 2>&1 | tail -30 | sed 's/^/  /'
    fi

    # Pause between batches
    if (( CURRENT % BATCH_SIZE == 0 )) && (( CURRENT < ${#TEST_FILES[@]} )); then
        echo "  Pausing ${PAUSE_SECONDS}s..."
        sleep "${PAUSE_SECONDS}"
        echo ""
    fi
done

echo ""
echo "=========================================="
echo "Category '${CATEGORY}' Results"
echo "=========================================="
echo "Total:  ${#TEST_FILES[@]}"
echo "Passed: ${PASSED}"
echo "Failed: ${FAILED}"

if (( FAILED > 0 )); then
    exit 1
fi
