#!/usr/bin/env bash
# ABOUTME: Pre-push validation script - runs all checks before pushing
# ABOUTME: Creates validation-passed marker in git dir (supports worktrees)
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -e

PROJECT_ROOT="$(git rev-parse --show-toplevel)"
GIT_DIR="$(git rev-parse --git-dir)"
MARKER_FILE="$GIT_DIR/validation-passed"
VALIDATION_TTL_MINUTES=15

echo ""
echo "🔍 Pierre MCP Server - Pre-Push Validation"
echo "==========================================="
echo ""

START_TIME=$(date +%s)

# Remove any stale marker
rm -f "$MARKER_FILE"

# ============================================================================
# Detect changed file types
# ============================================================================
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

if git rev-parse --verify "origin/$CURRENT_BRANCH" &>/dev/null; then
    BASE_REF="origin/$CURRENT_BRANCH"
elif git rev-parse --verify "origin/main" &>/dev/null; then
    BASE_REF="origin/main"
else
    BASE_REF="HEAD~1"
fi

CHANGED_FILES=$(git diff --name-only "$BASE_REF" HEAD 2>/dev/null || git diff --name-only HEAD~1 HEAD 2>/dev/null || echo "")

HAS_RUST_CHANGES=false
HAS_FRONTEND_CHANGES=false
HAS_SDK_CHANGES=false
HAS_MOBILE_CHANGES=false

while IFS= read -r file; do
    case "$file" in
        *.rs|Cargo.toml|Cargo.lock) HAS_RUST_CHANGES=true ;;
        frontend/*) HAS_FRONTEND_CHANGES=true ;;
        sdk/*) HAS_SDK_CHANGES=true ;;
        frontend-mobile/*) HAS_MOBILE_CHANGES=true ;;
    esac
done <<< "$CHANGED_FILES"

echo "📋 Changed file types:"
echo "   Rust: $HAS_RUST_CHANGES"
echo "   Frontend: $HAS_FRONTEND_CHANGES"
echo "   SDK: $HAS_SDK_CHANGES"
echo "   Mobile: $HAS_MOBILE_CHANGES"
echo ""

# ============================================================================
# TIER 0: Code Formatting
# ============================================================================
if [[ "$HAS_RUST_CHANGES" == "true" ]]; then
    echo "🎨 Tier 0: Code Formatting"
    echo "--------------------------"
    echo -n "Checking cargo fmt... "

    if cargo fmt --all -- --check > /dev/null 2>&1; then
        echo "✅"
    else
        echo "❌"
        echo ""
        echo "Code is not properly formatted. Run:"
        echo "  cargo fmt --all"
        exit 1
    fi
    echo ""
fi

# ============================================================================
# TIER 1: Architectural Validation
# ============================================================================
if [[ "$HAS_RUST_CHANGES" == "true" ]] && [[ -f "$PROJECT_ROOT/scripts/ci/architectural-validation.sh" ]]; then
    echo "📐 Tier 1: Architectural Validation"
    echo "------------------------------------"
    if ! "$PROJECT_ROOT/scripts/ci/architectural-validation.sh"; then
        echo ""
        echo "❌ Architectural validation failed!"
        exit 1
    fi
    echo ""
fi

# ============================================================================
# TIER 2: Schema Validation
# ============================================================================
if [[ "$HAS_RUST_CHANGES" == "true" ]]; then
    echo "📋 Tier 2: Schema Validation"
    echo "----------------------------"
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
fi

# ============================================================================
# TIER 3: Targeted Tests (Smart Selection)
# ============================================================================
if [[ "$HAS_RUST_CHANGES" == "true" ]]; then
    echo "🧪 Tier 3: Targeted Tests"
    echo "-------------------------"

    RUST_CHANGED_FILES=$(echo "$CHANGED_FILES" | grep -E '\.(rs)$' || echo "")

    if [[ -z "$RUST_CHANGED_FILES" ]]; then
        echo "No Rust files changed - skipping targeted tests"
    else
        # Collect tests to run (using associative array to dedupe)
        declare -A TESTS_TO_RUN

        add_tests() {
            for test in "$@"; do
                TESTS_TO_RUN["$test"]=1
            done
        }

        while IFS= read -r file; do
            case "$file" in
                src/database/*) add_tests database_test database_plugins_test tenant_data_isolation ;;
                src/auth/*|src/routes/auth.rs) add_tests auth_test api_keys_test jwt_secret_persistence_test oauth2_security_test ;;
                src/routes/*) add_tests routes_health_http_test security_headers_test rate_limiting_middleware_test ;;
                src/protocols/*|src/mcp/*) add_tests mcp_compliance_test jsonrpc_test mcp_tools_unit ;;
                src/tools/*) add_tests mcp_tools_unit ;;
                src/intelligence/*) add_tests intelligence_algorithms_test ;;
                src/a2a/*) add_tests a2a_system_user_test ;;
                src/models/*) add_tests models_test ;;
                src/errors/*) add_tests errors_test ;;
                src/crypto/*) add_tests crypto_keys_test ;;
                src/context/*|src/tenant/*) add_tests tenant_context_resolution_test tenant_data_isolation ;;
                src/config/*) add_tests simple_integration_test ;;
                migrations/*) add_tests database_test ;;
                tests/*.rs)
                    # Only process files directly in tests/, not subdirectories
                    # (case pattern * matches / in bash, so we need explicit check)
                    if [[ "$file" =~ ^tests/[^/]+\.rs$ ]]; then
                        test_name=$(basename "$file" .rs)
                        if [[ "$test_name" != "common" && "$test_name" != "helpers" && "$test_name" != "fixtures" ]]; then
                            add_tests "$test_name"
                        fi
                    fi
                    ;;
                src/lib.rs|src/main.rs) add_tests simple_integration_test routes_health_http_test ;;
                src/*) add_tests simple_integration_test ;;
            esac
        done <<< "$RUST_CHANGED_FILES"

        TEST_COUNT=${#TESTS_TO_RUN[@]}

        if [[ "$TEST_COUNT" -eq 0 ]]; then
            echo "No tests mapped for changed files"
        else
            echo "Running $TEST_COUNT targeted test file(s):"

            TEST_ARGS=""
            for test in "${!TESTS_TO_RUN[@]}"; do
                echo "  🧪 $test"
                TEST_ARGS="$TEST_ARGS --test $test"
            done
            echo ""

            if ! cargo test $TEST_ARGS --quiet -- --test-threads=4; then
                echo ""
                echo "❌ Targeted tests failed!"
                echo ""
                echo "Run individual tests for debugging:"
                echo "  cargo test --test <test_name> -- --nocapture"
                exit 1
            fi
            echo "✅ Targeted tests passed"
        fi
    fi
    echo ""
fi

# ============================================================================
# TIER 4: Frontend Tests (if changed)
# ============================================================================
if [[ "$HAS_FRONTEND_CHANGES" == "true" ]]; then
    echo "🌐 Tier 4: Frontend Tests"
    echo "-------------------------"
    if [[ -f "$PROJECT_ROOT/scripts/ci/pre-push-frontend-tests.sh" ]]; then
        if ! "$PROJECT_ROOT/scripts/ci/pre-push-frontend-tests.sh"; then
            echo "❌ Frontend tests failed!"
            exit 1
        fi
    else
        echo "⚠️  pre-push-frontend-tests.sh not found, skipping"
    fi
    echo ""
fi

# ============================================================================
# TIER 5: SDK Tests (if changed)
# ============================================================================
if [[ "$HAS_SDK_CHANGES" == "true" ]]; then
    echo "📦 Tier 5: SDK Tests"
    echo "--------------------"
    if [[ -d "$PROJECT_ROOT/sdk/node_modules" ]]; then
        echo "Running SDK unit tests..."
        if ! (cd "$PROJECT_ROOT/sdk" && npm run test:unit --silent 2>&1 | tail -5); then
            echo "❌ SDK tests failed!"
            exit 1
        fi
        echo "✅ SDK tests passed"
    else
        echo "⚠️  sdk/node_modules not found, skipping"
    fi
    echo ""
fi

# ============================================================================
# TIER 6: Mobile Tests (if changed)
# ============================================================================
if [[ "$HAS_MOBILE_CHANGES" == "true" ]]; then
    echo "📱 Tier 6: Mobile Tests"
    echo "-----------------------"
    if [[ -f "$PROJECT_ROOT/scripts/ci/pre-push-mobile-tests.sh" ]]; then
        if ! "$PROJECT_ROOT/scripts/ci/pre-push-mobile-tests.sh"; then
            echo "❌ Mobile tests failed!"
            exit 1
        fi
    else
        echo "⚠️  pre-push-mobile-tests.sh not found, skipping"
    fi
    echo ""
fi

# ============================================================================
# SUCCESS - Create marker file
# ============================================================================
END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Create marker with timestamp and commit hash
CURRENT_COMMIT=$(git rev-parse HEAD)
echo "$END_TIME $CURRENT_COMMIT" > "$MARKER_FILE"

echo "==========================================="
echo "✅ All validations passed!"
echo "==========================================="
echo ""
echo "Duration: ${DURATION}s (~$((DURATION / 60))m $((DURATION % 60))s)"
echo "Marker:   .git/validation-passed (valid for ${VALIDATION_TTL_MINUTES} minutes)"
echo ""
echo "You can now push:"
echo "  git push"
echo ""
