#!/bin/bash
# ABOUTME: Pre-push validation for mobile (frontend-mobile/) - TypeScript, lint, tests
# ABOUTME: Runs essential checks to catch issues before pushing (~5-10 seconds)
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MOBILE_DIR="$PROJECT_ROOT/frontend-mobile"

echo "📱 Pierre Mobile - Pre-Push Validation"
echo "======================================="
echo ""

# Check if mobile directory exists
if [ ! -d "$MOBILE_DIR" ]; then
    echo "❌ Error: frontend-mobile/ directory not found"
    exit 1
fi

# Check if node_modules exists
if [ ! -d "$MOBILE_DIR/node_modules" ]; then
    echo "⚠️  Warning: frontend-mobile/node_modules not found."
    echo "   Run 'cd frontend-mobile && bun install' to enable validation."
    exit 0
fi

cd "$MOBILE_DIR"

START_TIME=$(date +%s)
PASSED=0
FAILED=0

# ============================================================================
# TIER 0: TypeScript Type Checking (fastest feedback)
# ============================================================================
echo "📘 Tier 0: TypeScript Type Checking"
echo "------------------------------------"
echo -n "Running typecheck... "

if bun run typecheck > /dev/null 2>&1; then
    echo "✅"
    PASSED=$((PASSED + 1))
else
    echo "❌"
    FAILED=$((FAILED + 1))
    echo ""
    echo "TypeScript errors found:"
    bun run typecheck 2>&1 | head -30
    echo ""
    echo "Run 'cd frontend-mobile && bun run typecheck' to see all errors."
    exit 1
fi

echo ""

# ============================================================================
# TIER 1: ESLint (code quality)
# ============================================================================
echo "🔍 Tier 1: ESLint"
echo "-----------------"
echo -n "Running lint... "

if bun run lint > /dev/null 2>&1; then
    echo "✅"
    PASSED=$((PASSED + 1))
else
    echo "❌"
    FAILED=$((FAILED + 1))
    echo ""
    echo "Lint errors found:"
    bun run lint 2>&1 | head -30
    echo ""
    echo "Run 'cd frontend-mobile && bun run lint' to see all errors."
    exit 1
fi

echo ""

# ============================================================================
# TIER 2: Unit Tests (functionality)
# ============================================================================
echo "🧪 Tier 2: Unit Tests"
echo "---------------------"
echo -n "Running tests... "

if bun run test --silent > /dev/null 2>&1; then
    echo "✅"
    PASSED=$((PASSED + 1))
    # Show summary
    bun run test --silent 2>&1 | grep -E "^(Test Suites|Tests):" | sed 's/^/   /'
else
    echo "❌"
    FAILED=$((FAILED + 1))
    echo ""
    echo "Test failures:"
    bun run test 2>&1 | tail -30
    echo ""
    echo "Run 'cd frontend-mobile && bun run test' to see details."
    exit 1
fi

echo ""

# ============================================================================
# Summary
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "======================================="
echo "Mobile Pre-Push Validation Complete"
echo "======================================="
echo "Checks passed: $PASSED/3"
echo "Duration:      ${DURATION}s"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "❌ Some checks failed. Please fix before pushing."
    exit 1
else
    echo "✅ All mobile checks passed!"
    echo ""
    echo "⚠️  Note: E2E tests run in CI (require iOS Simulator)"
    echo "   To run locally: cd frontend-mobile && bun run e2e:test"
fi
