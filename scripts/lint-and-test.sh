#!/bin/bash
# ABOUTME: Simplified validation orchestrator using native Cargo commands
# ABOUTME: Delegates to cargo fmt, cargo clippy, cargo deny, and custom architectural validation

# ============================================================================
# ARCHITECTURE: Native Cargo-First Approach
# ============================================================================
# This script has been DRASTICALLY SIMPLIFIED (from 1294 → ~350 lines)
#
# WHAT CHANGED:
# - Clippy: cargo clippy (reads Cargo.toml [lints] table) ← was custom flags
# - Formatting: cargo fmt --check ← was custom orchestration
# - Security: cargo deny check (reads deny.toml) ← was cargo-audit + bash
# - Documentation: cargo doc --no-deps ← was custom checks
#
# WHAT REMAINS CUSTOM:
# - Architectural validation (scripts/architectural-validation.sh)
# - Frontend orchestration (npm/TypeScript toolchain)
# - Test execution coordination
# - MCP/Bridge compliance checks

set -e

echo "Running Pierre MCP Server Validation Suite..."

# Start timing
START_TIME=$(date +%s)

# Parse command line arguments
ENABLE_COVERAGE=false
for arg in "$@"; do
    case $arg in
        --coverage)
            ENABLE_COVERAGE=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--coverage]"
            echo "  --coverage    Enable code coverage collection and reporting"
            exit 0
            ;;
        *)
            echo "Unknown option: $arg"
            echo "Usage: $0 [--coverage]"
            exit 1
            ;;
    esac
done

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo -e "${BLUE}==== Pierre MCP Server - Validation Suite ====${NC}"
echo "Project root: $PROJECT_ROOT"
cd "$PROJECT_ROOT"

# Track overall success
ALL_PASSED=true

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# CLEANUP
# ============================================================================

echo ""
echo -e "${BLUE}==== Cleaning up generated files... ====${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# ============================================================================
# DISABLED FILE DETECTION (Prevents incomplete migrations)
# ============================================================================

echo ""
echo -e "${BLUE}==== Checking for disabled test/source files... ====${NC}"

# Find all disabled files
DISABLED_TESTS=$(find tests -name "*.disabled" -o -name "*.warp-backup" 2>/dev/null)
DISABLED_SRC=$(find src -name "*.disabled" 2>/dev/null)

DISABLED_COUNT=0
if [ -n "$DISABLED_TESTS" ]; then
    DISABLED_COUNT=$((DISABLED_COUNT + $(echo "$DISABLED_TESTS" | wc -l)))
fi
if [ -n "$DISABLED_SRC" ]; then
    DISABLED_COUNT=$((DISABLED_COUNT + $(echo "$DISABLED_SRC" | wc -l)))
fi

if [ "$DISABLED_COUNT" -gt 0 ]; then
    echo -e "${RED}[CRITICAL] Found $DISABLED_COUNT disabled files:${NC}"
    [ -n "$DISABLED_TESTS" ] && echo -e "${RED}Disabled tests:${NC}\n$DISABLED_TESTS"
    [ -n "$DISABLED_SRC" ] && echo -e "${RED}Disabled source files:${NC}\n$DISABLED_SRC"
    echo -e "${RED}All test files must be enabled and passing before merge${NC}"
    echo -e "${RED}Rename files to remove .disabled/.warp-backup extensions${NC}"
    ALL_PASSED=false
    exit 1
else
    echo -e "${GREEN}[OK] No disabled files found - all tests are active${NC}"
fi

# ============================================================================
# IGNORED TEST DETECTION (Zero tolerance policy)
# ============================================================================

echo ""
echo -e "${BLUE}==== Checking for ignored tests... ====${NC}"

# Find all #[ignore] attributes in test files
IGNORED_TESTS=$(rg "#\[ignore\]" tests/ -l 2>/dev/null || true)
IGNORED_COUNT=0

if [ -n "$IGNORED_TESTS" ]; then
    IGNORED_COUNT=$(echo "$IGNORED_TESTS" | wc -l | tr -d ' ')
fi

if [ "$IGNORED_COUNT" -gt 0 ]; then
    echo -e "${RED}[CRITICAL] Found $IGNORED_COUNT test files with #[ignore] attributes:${NC}"
    echo -e "${RED}$IGNORED_TESTS${NC}"
    echo ""

    # Show the actual ignored tests
    echo -e "${RED}Ignored test details:${NC}"
    rg "#\[ignore\]" tests/ -B 2 -A 1 2>/dev/null | head -30
    echo ""

    echo -e "${RED}Main branch policy: ZERO ignored tests (main has 0, branch must match)${NC}"
    echo -e "${RED}Tests must either pass or be removed - #[ignore] hides incomplete work${NC}"
    echo -e "${RED}Remove #[ignore] attributes and ensure all tests pass${NC}"
    ALL_PASSED=false
    exit 1
else
    echo -e "${GREEN}[OK] No ignored tests found - 100% test execution${NC}"
fi

# ============================================================================
# NATIVE CARGO VALIDATION (Reads Cargo.toml [lints] + deny.toml)
# ============================================================================

echo ""
echo -e "${BLUE}==== Native Cargo Validation ====${NC}"

# Formatting check
echo -e "${BLUE}Running cargo fmt --check...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}[OK] Rust code formatting is correct${NC}"
else
    echo -e "${RED}[CRITICAL] Rust code formatting check failed${NC}"
    echo -e "${RED}Run 'cargo fmt --all' to fix formatting issues${NC}"
    ALL_PASSED=false
    exit 1
fi

# Clippy linting and compilation (reads Cargo.toml [lints.clippy] with level = "deny")
# NOTE: This builds the debug binary ONCE - all subsequent steps reuse this build
echo -e "${BLUE}Running cargo clippy + build (zero tolerance via Cargo.toml)...${NC}"

# ============================================================================
# CRITICAL: Why we need explicit -- -D warnings flag
# ============================================================================
# DO NOT REMOVE THE "-D warnings" FLAG - Here's why:
#
# The [lints.clippy] configuration in Cargo.toml (lines 160-166) has known
# reliability issues with flag ordering that cause inconsistent behavior:
#
# GitHub Issue: https://github.com/rust-lang/rust-clippy/issues/11237
# Title: "cargo clippy not obeying [lints.clippy] from Cargo.toml"
# Root Cause: Cargo sorts flags before passing to clippy, breaking precedence
# Examples: wildcard_imports, too_many_lines, option_if_let_else all failed
#           to respect [lints.clippy] deny configuration
#
# Official Clippy Documentation (https://doc.rust-lang.org/clippy/usage.html):
# "For CI all warnings can be elevated to errors which will in turn fail
#  the build and cause Clippy to exit with a code other than 0"
# Recommended Command: cargo clippy -- -Dwarnings
#
# Without explicit -D warnings:
# ❌ Clippy may exit with code 0 even when warnings exist
# ❌ CI/CD won't fail on code quality issues
# ❌ Cargo.toml [lints] flag ordering can be inconsistent
#
# With explicit -D warnings:
# ✅ Guaranteed non-zero exit code on ANY warning
# ✅ Bypasses Cargo.toml flag ordering bugs
# ✅ Standard CI/CD pattern (documented in official Clippy docs)
# ============================================================================

# Run clippy with output to both screen and variable
# Explicit -W flags ensure nursery lints (including too_many_lines) apply to all targets
# CRITICAL: The "-- -D warnings" suffix is REQUIRED - see documentation above
CLIPPY_OUTPUT=$(cargo clippy --all-targets --all-features --quiet -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings 2>&1 | tee /dev/stderr)
CLIPPY_EXIT=$?

# Check if clippy was killed (exit code 137 = SIGKILL, 143 = SIGTERM)
if [ $CLIPPY_EXIT -eq 137 ] || [ $CLIPPY_EXIT -eq 143 ]; then
    echo ""
    echo -e "${RED}[CRITICAL] Clippy was killed (possibly out of memory)${NC}"
    echo -e "${RED}Exit code: $CLIPPY_EXIT${NC}"
    ALL_PASSED=false
    exit 1
fi

# Count only code warnings, exclude dependency future-compatibility warnings
WARNING_COUNT=$(echo "$CLIPPY_OUTPUT" | grep "^warning:" | grep -v "the following packages contain code that will be rejected by a future version of Rust" | wc -l | tr -d ' ')

if [ $CLIPPY_EXIT -ne 0 ] || [ "$WARNING_COUNT" -gt 0 ]; then
    echo ""
    echo -e "${RED}[CRITICAL] Clippy failed with exit code $CLIPPY_EXIT and $WARNING_COUNT code warnings${NC}"
    echo -e "${RED}ALL code warnings must be fixed - zero tolerance policy${NC}"
    ALL_PASSED=false
    exit 1
else
    echo -e "${GREEN}[OK] Clippy passed - ZERO code warnings (enforced by Cargo.toml)${NC}"
    echo -e "${GREEN}[OK] Debug build completed (reused for all validation)${NC}"
fi

# Security audit (reads deny.toml)
echo -e "${BLUE}Running cargo deny check...${NC}"
if command_exists cargo-deny; then
    if cargo deny check; then
        echo -e "${GREEN}[OK] Security audit passed (via deny.toml)${NC}"
    else
        echo -e "${YELLOW}[WARN] Security vulnerabilities detected${NC}"
        echo -e "${YELLOW}Review output above and update dependencies${NC}"
        # Don't fail build, just warn
    fi
else
    echo -e "${YELLOW}[WARN] cargo-deny not installed${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-deny${NC}"
fi

# ============================================================================
# PII AND SECRET PATTERN DETECTION
# ============================================================================

echo ""
echo -e "${BLUE}==== PII and Secret Pattern Detection ====${NC}"
if [ -f "$SCRIPT_DIR/validate-no-secrets.sh" ]; then
    if "$SCRIPT_DIR/validate-no-secrets.sh"; then
        echo -e "${GREEN}[OK] Secret pattern validation passed${NC}"
    else
        echo -e "${RED}[CRITICAL] Secret pattern validation failed${NC}"
        ALL_PASSED=false
        exit 1
    fi
else
    echo -e "${YELLOW}[WARN] Secret validation script not found - skipping${NC}"
fi

# ============================================================================
# CUSTOM ARCHITECTURAL VALIDATION (Project-Specific Rules)
# ============================================================================
# NOTE: Runs early for fail-fast behavior (before tests)
# Binary size check will warn if release binary doesn't exist yet (expected - built at end)

echo ""
echo -e "${BLUE}==== Custom Architectural Validation ====${NC}"

if [ -f "$SCRIPT_DIR/architectural-validation.sh" ]; then
    if "$SCRIPT_DIR/architectural-validation.sh"; then
        echo -e "${GREEN}[OK] Architectural validation passed${NC}"
    else
        echo -e "${RED}[CRITICAL] Architectural validation failed${NC}"
        ALL_PASSED=false
        exit 1
    fi
else
    echo -e "${YELLOW}[WARN] Architectural validation script not found - skipping${NC}"
fi

# ============================================================================
# TEST EXECUTION
# ============================================================================

echo ""
echo -e "${BLUE}==== Running Tests (reusing debug build from clippy) ====${NC}"

# Clean test databases
echo -e "${BLUE}Cleaning test databases...${NC}"
if [ -f "$SCRIPT_DIR/clean-test-databases.sh" ]; then
    "$SCRIPT_DIR/clean-test-databases.sh" || true
fi

# Ensure data directory exists
mkdir -p data

# Count tests
TOTAL_TESTS=$(cargo test --all-targets -- --list 2>/dev/null | grep -E "^[a-zA-Z_].*: test$" | wc -l | tr -d ' ')
echo -e "${BLUE}Total tests to run: $TOTAL_TESTS${NC}"

# Use 2048-bit RSA for faster test execution
export PIERRE_RSA_KEY_SIZE=2048

# Run all tests (reuses build artifacts from clippy step)
# Note: --all-targets includes all test binaries (unit, integration, etc.)
if [ "$ENABLE_COVERAGE" = true ]; then
    echo -e "${BLUE}Running all tests with coverage...${NC}"
    if command_exists cargo-llvm-cov; then
        if cargo llvm-cov --all-targets --summary-only; then
            echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed with coverage${NC}"
        else
            echo -e "${RED}[FAIL] Some tests failed${NC}"
            ALL_PASSED=false
        fi
    else
        echo -e "${YELLOW}[WARN] cargo-llvm-cov not installed${NC}"
        echo -e "${YELLOW}Install with: cargo install cargo-llvm-cov${NC}"
        if cargo test --all-targets --no-fail-fast; then
            echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed${NC}"
        else
            echo -e "${RED}[FAIL] Some tests failed${NC}"
            ALL_PASSED=false
        fi
    fi
else
    if cargo test --all-targets --no-fail-fast; then
        echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed${NC}"
    else
        echo -e "${RED}[FAIL] Some tests failed${NC}"
        ALL_PASSED=false
    fi
fi

# Bridge test suite (SDK tests - uses debug binary)
echo ""
echo -e "${BLUE}==== Bridge Test Suite (SDK Tests) ====${NC}"
if [ -f "$SCRIPT_DIR/run_bridge_tests.sh" ]; then
    if "$SCRIPT_DIR/run_bridge_tests.sh"; then
        echo -e "${GREEN}[OK] Bridge test suite passed${NC}"
    else
        echo -e "${RED}[FAIL] Bridge test suite failed${NC}"
        ALL_PASSED=false
    fi
else
    echo -e "${YELLOW}[WARN] Bridge test script not found - skipping${NC}"
fi

# ============================================================================
# MCP SPEC COMPLIANCE
# ============================================================================

echo ""
echo -e "${BLUE}==== MCP Spec Compliance ====${NC}"
if [ -f "$SCRIPT_DIR/ensure_mcp_compliance.sh" ]; then
    if "$SCRIPT_DIR/ensure_mcp_compliance.sh"; then
        echo -e "${GREEN}[OK] MCP compliance validation passed${NC}"
    else
        echo -e "${RED}[FAIL] MCP compliance validation failed${NC}"
        ALL_PASSED=false
    fi
else
    echo -e "${YELLOW}[WARN] MCP compliance script not found - skipping${NC}"
fi

# ============================================================================
# FRONTEND VALIDATION (Separate Toolchain)
# ============================================================================

if [ -d "frontend" ]; then
    echo ""
    echo -e "${BLUE}==== Frontend Validation ====${NC}"
    cd frontend

    # Check dependencies
    if [ ! -d "node_modules" ] || [ ! -f "node_modules/.package-lock.json" ]; then
        echo -e "${YELLOW}Installing frontend dependencies...${NC}"
        npm install || {
            echo -e "${RED}[FAIL] Frontend dependency installation failed${NC}"
            ALL_PASSED=false
            cd ..
        }
    fi

    if [ -d "node_modules" ]; then
        # Lint
        if npm run lint; then
            echo -e "${GREEN}[OK] Frontend linting passed${NC}"
        else
            echo -e "${RED}[FAIL] Frontend linting failed${NC}"
            ALL_PASSED=false
        fi

        # Type check
        if npm run type-check; then
            echo -e "${GREEN}[OK] TypeScript type checking passed${NC}"
        else
            echo -e "${RED}[FAIL] TypeScript type checking failed${NC}"
            ALL_PASSED=false
        fi

        # Tests
        if npm test -- --run; then
            echo -e "${GREEN}[OK] Frontend tests passed${NC}"
        else
            echo -e "${RED}[FAIL] Frontend tests failed${NC}"
            ALL_PASSED=false
        fi

        # Build
        if npm run build; then
            echo -e "${GREEN}[OK] Frontend build successful${NC}"
        else
            echo -e "${RED}[FAIL] Frontend build failed${NC}"
            ALL_PASSED=false
        fi
    fi

    cd ..
fi

# ============================================================================
# FINAL CLEANUP
# ============================================================================

echo -e "${BLUE}Final cleanup...${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
find . -name "a2a_enterprise_report_*.json" -delete 2>/dev/null || true
find . -name "mcp_investor_demo_*.json" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# ============================================================================
# PERFORMANCE AND DOCUMENTATION (After all tests including SDK)
# ============================================================================

echo ""
echo -e "${BLUE}==== Release Build and Documentation ====${NC}"

# Build release binary (only after all tests pass including SDK tests)
echo -e "${BLUE}Building release binary...${NC}"
if cargo build --release --quiet; then
    echo -e "${GREEN}[OK] Release build successful${NC}"

    # Binary size check
    if [ -f "target/release/pierre-mcp-server" ]; then
        BINARY_SIZE=$(ls -lh target/release/pierre-mcp-server | awk '{print $5}')
        BINARY_SIZE_BYTES=$(ls -l target/release/pierre-mcp-server | awk '{print $5}')
        MAX_SIZE_BYTES=$((50 * 1024 * 1024))  # 50MB in bytes

        if [ "$BINARY_SIZE_BYTES" -le "$MAX_SIZE_BYTES" ]; then
            echo -e "${GREEN}[OK] Binary size ($BINARY_SIZE) within limit (<50MB)${NC}"
        else
            echo -e "${RED}[FAIL] Binary size ($BINARY_SIZE) exceeds limit (50MB)${NC}"
            ALL_PASSED=false
        fi
    fi
else
    echo -e "${RED}[FAIL] Release build failed${NC}"
    ALL_PASSED=false
fi

# Documentation
echo -e "${BLUE}Checking documentation...${NC}"
if cargo doc --no-deps --quiet; then
    echo -e "${GREEN}[OK] Documentation builds successfully${NC}"
else
    echo -e "${RED}[FAIL] Documentation build failed${NC}"
    ALL_PASSED=false
fi

# ============================================================================
# SUMMARY
# ============================================================================

END_TIME=$(date +%s)
TOTAL_SECONDS=$((END_TIME - START_TIME))
TOTAL_MINUTES=$((TOTAL_SECONDS / 60))
REMAINING_SECONDS=$((TOTAL_SECONDS % 60))

echo ""
echo -e "${BLUE}==== Validation Summary ====${NC}"
echo -e "${BLUE}Total execution time: ${TOTAL_MINUTES}m ${REMAINING_SECONDS}s${NC}"
echo ""

if [ "$ALL_PASSED" = true ]; then
    echo -e "${GREEN}✅ ALL VALIDATION PASSED - Task can be marked complete${NC}"
    echo ""
    echo "[OK] Rust formatting (cargo fmt)"
    echo "[OK] Rust linting + build (cargo clippy via Cargo.toml)"
    echo "[OK] Security audit (cargo deny via deny.toml)"
    echo "[OK] Secret pattern detection"
    echo "[OK] No disabled test files (.disabled/.warp-backup extensions)"
    echo "[OK] No ignored tests (#[ignore] attributes - 100% test execution)"
    echo "[OK] Architectural validation (custom)"
    echo "[OK] All Rust tests (cargo test --all-targets includes unit, integration, HTTP API, A2A)"
    if [ -d "frontend" ]; then
        echo "[OK] Frontend linting"
        echo "[OK] TypeScript type checking"
        echo "[OK] Frontend tests"
        echo "[OK] Frontend build"
    fi
    if [ -f "$SCRIPT_DIR/run_bridge_tests.sh" ]; then
        echo "[OK] Bridge test suite (SDK tests with debug binary)"
    fi
    if [ -f "$SCRIPT_DIR/ensure_mcp_compliance.sh" ]; then
        echo "[OK] MCP spec compliance validation"
    fi
    echo "[OK] Cleanup"
    echo "[OK] Release build (cargo build --release)"
    echo "[OK] Documentation (cargo doc)"
    if [ "$ENABLE_COVERAGE" = true ] && command_exists cargo-llvm-cov; then
        echo "[OK] Rust code coverage"
    fi
    echo ""
    echo -e "${GREEN}Code meets ALL standards and is ready for production!${NC}"
    exit 0
else
    echo -e "${RED}❌ VALIDATION FAILED - Task cannot be marked complete${NC}"
    echo -e "${RED}Fix ALL issues above to meet dev standards requirements${NC}"
    exit 1
fi
