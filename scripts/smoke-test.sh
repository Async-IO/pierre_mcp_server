#!/bin/bash
# ABOUTME: Quick validation script for rapid development feedback (2-3 minutes)
# ABOUTME: Runs format check, clippy on main code, unit tests, and one integration test

set -e

echo "🚀 Pierre MCP Server - Smoke Test"
echo "=================================="

START_TIME=$(date +%s)

# 1. Format check (fast)
echo -n "📋 Format check... "
cargo fmt --check && echo "✅" || { echo "❌"; exit 1; }

# 2. Clippy on main code only (skip tests)
echo -n "🔍 Clippy (lib + bins)... "
cargo clippy --lib --bins --quiet -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery \
  -A clippy::module_name_repetitions -A clippy::missing_errors_doc -A clippy::missing_panics_doc \
  -A clippy::too_many_lines -A clippy::must_use_candidate && echo "✅" || { echo "❌"; exit 1; }

# 3. Unit tests only
echo -n "🧪 Unit tests... "
cargo test --lib --quiet && echo "✅" || { echo "❌"; exit 1; }

# 4. One critical integration test
echo -n "🔗 Critical integration test... "
cargo test --test routes_health_http_test --quiet && echo "✅" || { echo "❌"; exit 1; }

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

echo ""
echo "✅ Smoke test passed in ${DURATION}s"
echo "⚠️  Run './scripts/lint-and-test.sh' before final commit"
