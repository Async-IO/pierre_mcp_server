#!/bin/bash
# ABOUTME: Pre-push validation for web frontend (frontend/) - TypeScript, lint, tests
# ABOUTME: Runs essential checks to catch issues before pushing (~5-10 seconds)
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
FRONTEND_DIR="$PROJECT_ROOT/frontend"

echo "🌐 Pierre Frontend - Pre-Push Validation"
echo "========================================="
echo ""

# Check if frontend directory exists
if [ ! -d "$FRONTEND_DIR" ]; then
    echo "❌ Error: frontend/ directory not found"
    exit 1
fi

# Check if node_modules exists
if [ ! -d "$FRONTEND_DIR/node_modules" ]; then
    echo "⚠️  Warning: frontend/node_modules not found."
    echo "   Run 'cd frontend && npm install' to enable validation."
    exit 0
fi

cd "$FRONTEND_DIR"

START_TIME=$(date +%s)
PASSED=0
FAILED=0

# ============================================================================
# TIER 0: TypeScript Type Checking (fastest feedback)
# ============================================================================
echo "📘 Tier 0: TypeScript Type Checking"
echo "------------------------------------"
echo -n "Running type-check... "

if npm run type-check > /dev/null 2>&1; then
    echo "✅"
    ((PASSED++))
else
    echo "❌"
    ((FAILED++))
    echo ""
    echo "TypeScript errors found:"
    npm run type-check 2>&1 | head -30
    echo ""
    echo "Run 'cd frontend && npm run type-check' to see all errors."
    exit 1
fi

echo ""

# ============================================================================
# TIER 1: ESLint (code quality)
# ============================================================================
echo "🔍 Tier 1: ESLint"
echo "-----------------"
echo -n "Running lint... "

if npm run lint -- --quiet > /dev/null 2>&1; then
    echo "✅"
    ((PASSED++))
else
    echo "❌"
    ((FAILED++))
    echo ""
    echo "Lint errors found:"
    npm run lint 2>&1 | head -30
    echo ""
    echo "Run 'cd frontend && npm run lint' to see all errors."
    exit 1
fi

echo ""

# ============================================================================
# TIER 2: Unit Tests (functionality)
# ============================================================================
echo "🧪 Tier 2: Unit Tests"
echo "---------------------"
echo -n "Running tests... "

if npm test -- --run --reporter=dot > /dev/null 2>&1; then
    echo "✅"
    ((PASSED++))
else
    echo "❌"
    ((FAILED++))
    echo ""
    echo "Test failures:"
    npm test -- --run 2>&1 | tail -30
    echo ""
    echo "Run 'cd frontend && npm test' to see details."
    exit 1
fi

echo ""

# ============================================================================
# Summary
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo "========================================="
echo "Frontend Pre-Push Validation Complete"
echo "========================================="
echo "Checks passed: $PASSED/3"
echo "Duration:      ${DURATION}s"
echo ""

if [ $FAILED -gt 0 ]; then
    echo "❌ Some checks failed. Please fix before pushing."
    exit 1
else
    echo "✅ All frontend checks passed!"
    echo ""
    echo "⚠️  Note: E2E tests run in CI (require browser)"
    echo "   To run locally: cd frontend && npm run test:e2e"
fi
