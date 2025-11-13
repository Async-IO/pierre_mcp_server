#!/usr/bin/env bash
# ABOUTME: Quick compilation check without running tests - safe for Claude Code Web
# ABOUTME: Validates that all test code compiles without triggering OOM

set -euo pipefail

echo "=========================================="
echo "Quick Test Compilation Check"
echo "=========================================="
echo ""

echo "Step 1: Checking workspace compilation..."
if cargo check --workspace --tests 2>&1 | tail -50; then
    echo "✓ All test code compiles successfully"
    echo ""
else
    echo "✗ Compilation failed"
    exit 1
fi

echo "Step 2: Counting test files..."
ACTIVE_TESTS=$(find tests -name "*.rs" -not -name "*.disabled" -not -path "*/helpers/*" | wc -l)
DISABLED_TESTS=$(find tests -name "*.disabled" | wc -l)

echo "  Active test files: ${ACTIVE_TESTS}"
echo "  Disabled test files: ${DISABLED_TESTS}"
echo ""

echo "=========================================="
echo "✓ Compilation check complete"
echo "=========================================="
echo ""
echo "To run tests safely, use:"
echo "  ./scripts/safe-test-runner.sh           # All tests in batches"
echo "  ./scripts/category-test-runner.sh mcp   # Just MCP tests"
echo "  cargo test --test <name>                # Single test file"
