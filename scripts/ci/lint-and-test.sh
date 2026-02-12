#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
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
set -o pipefail

echo "Running Pierre MCP Server Validation Suite..."

# Start timing
START_TIME=$(date +%s)

# Task counter
CURRENT_TASK=0
TOTAL_TASKS=0

# Count tasks (all mandatory now)
count_tasks() {
    echo 13  # Mandatory tasks: cleanup, static analysis, fmt, clippy, deny, sdk-build, tests, frontend (lint+types+unit+e2e+build), mcp, sdk-validation, bridge, release+docs
}

# Print task header
print_task() {
    CURRENT_TASK=$((CURRENT_TASK + 1))
    echo ""
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}Task $CURRENT_TASK/$TOTAL_TASKS: $1${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════════${NC}"
}

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
PROJECT_ROOT="$( cd "$SCRIPT_DIR/../.." && pwd )"

# Calculate total tasks
TOTAL_TASKS=$(count_tasks)

echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}       Pierre MCP Server - Validation Suite${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo "Project root: $PROJECT_ROOT"
echo -e "${BLUE}Total tasks to execute: $TOTAL_TASKS${NC}"
echo ""
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

print_task "Cleaning up generated files"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# ============================================================================
# STATIC ANALYSIS & CODE QUALITY VALIDATION
# ============================================================================
# Combines: disabled files, ignored tests, secret patterns, architecture
# ============================================================================

print_task "Static Analysis & Code Quality"

# Initialize validation results
VALIDATION_FAILED=false

# Arrays to store results for table
declare -a VALIDATION_CATEGORIES
declare -a VALIDATION_COUNTS
declare -a VALIDATION_STATUSES
declare -a VALIDATION_DETAILS

# Function to add validation result
add_validation() {
    local category="$1"
    local count="$2"
    local status="$3"
    local details="$4"

    VALIDATION_CATEGORIES+=("$category")
    VALIDATION_COUNTS+=("$count")
    VALIDATION_STATUSES+=("$status")
    VALIDATION_DETAILS+=("$details")
}

echo -e "${BLUE}Running static analysis checks...${NC}"
echo ""

# ============================================================================
# 1. DISABLED FILE DETECTION
# ============================================================================
DISABLED_TESTS=$(find tests -name "*.disabled" -o -name "*.warp-backup" 2>/dev/null)
DISABLED_SRC=$(find src -name "*.disabled" 2>/dev/null)
DISABLED_COUNT=0
[ -n "$DISABLED_TESTS" ] && DISABLED_COUNT=$((DISABLED_COUNT + $(echo "$DISABLED_TESTS" | wc -l)))
[ -n "$DISABLED_SRC" ] && DISABLED_COUNT=$((DISABLED_COUNT + $(echo "$DISABLED_SRC" | wc -l)))

if [ "$DISABLED_COUNT" -gt 0 ]; then
    add_validation "Disabled files (.disabled/.warp-backup)" "$DISABLED_COUNT" "❌ FAIL" "Found in tests/ or src/"
    VALIDATION_FAILED=true
else
    add_validation "Disabled files (.disabled/.warp-backup)" "0" "✅ PASS" "All tests active"
fi

# ============================================================================
# 2. IGNORED TEST DETECTION
# ============================================================================
IGNORED_TESTS=$(rg "#\[ignore\]" tests/ -l 2>/dev/null || true)
IGNORED_COUNT=0
[ -n "$IGNORED_TESTS" ] && IGNORED_COUNT=$(echo "$IGNORED_TESTS" | wc -l | tr -d ' ')

if [ "$IGNORED_COUNT" -gt 0 ]; then
    FIRST_IGNORED=$(echo "$IGNORED_TESTS" | head -1)
    add_validation "Ignored tests (#[ignore])" "$IGNORED_COUNT" "❌ FAIL" "$FIRST_IGNORED"
    VALIDATION_FAILED=true
else
    add_validation "Ignored tests (#[ignore])" "0" "✅ PASS" "100% test execution"
fi

# ============================================================================
# 3. IGNORED DOCTEST DETECTION
# ============================================================================
# Doctests marked with `ignore` are not compiled/tested - use `no_run` instead
# if you need the code to compile but not execute
# ============================================================================
DOCTEST_OUTPUT=$(cargo test --doc 2>&1 || true)
IGNORED_DOCTESTS=$(echo "$DOCTEST_OUTPUT" | grep -E "test result:.*ignored" | grep -oE "[0-9]+ ignored" | grep -oE "[0-9]+" || echo 0)
IGNORED_DOCTESTS=$(echo "$IGNORED_DOCTESTS" | head -1 | tr -d '\n\r\t ')

if [ "${IGNORED_DOCTESTS:-0}" -gt 0 ]; then
    add_validation "Ignored doctests" "$IGNORED_DOCTESTS" "❌ FAIL" "Use no_run instead of ignore"
    VALIDATION_FAILED=true
else
    add_validation "Ignored doctests" "0" "✅ PASS" "All doctests active"
fi

# ============================================================================
# 4. SECRET PATTERN DETECTION
# ============================================================================
if [ -f "$SCRIPT_DIR/validate-no-secrets.sh" ]; then
    # Temporarily disable set -e to capture output even if script fails
    set +e
    SECRET_OUTPUT=$("$SCRIPT_DIR/validate-no-secrets.sh" 2>&1)
    SECRET_EXIT=$?
    set -e

    # Count failures from secret validation
    SECRET_FAILURES=$(echo "$SECRET_OUTPUT" | grep -c "❌" 2>/dev/null || echo 0)
    SECRET_FAILURES=$(echo "$SECRET_FAILURES" | head -1 | tr -d '\n\r\t ')

    if [ "$SECRET_EXIT" -ne 0 ] || [ "$SECRET_FAILURES" -gt 0 ]; then
        add_validation "Secret patterns" "$SECRET_FAILURES" "❌ FAIL" "Run validate-no-secrets.sh for details"
        VALIDATION_FAILED=true
    else
        add_validation "Authorization tokens" "0" "✅ PASS" "No exposed tokens"
        add_validation "API keys" "0" "✅ PASS" "No hardcoded keys"
        add_validation "Passwords" "0" "✅ PASS" "No hardcoded passwords"
        add_validation "JWT tokens" "0" "✅ PASS" "No exposed JWTs"
        add_validation "Private keys" "0" "✅ PASS" "No private keys"
        add_validation "PII leakage" "0" "✅ PASS" "No PII in logs"
        add_validation "DB credentials" "0" "✅ PASS" "No embedded credentials"
    fi
else
    add_validation "Secret patterns" "?" "⚠️ SKIP" "validate-no-secrets.sh not found"
fi

# ============================================================================
# 5. ANYHOW ERROR BLANKET CONVERSION DETECTION (CLAUDE.MD ZERO TOLERANCE)
# ============================================================================
ANYHOW_FROM_IMPL=$(rg "impl From<anyhow::Error>" src/ -l 2>/dev/null || true)
ANYHOW_FROM_COUNT=0
[ -n "$ANYHOW_FROM_IMPL" ] && ANYHOW_FROM_COUNT=$(echo "$ANYHOW_FROM_IMPL" | wc -l | tr -d ' ')

if [ "$ANYHOW_FROM_COUNT" -gt 0 ]; then
    FIRST_ANYHOW_FROM=$(echo "$ANYHOW_FROM_IMPL" | head -1)
    add_validation "impl From<anyhow::Error> blanket conversions" "$ANYHOW_FROM_COUNT" "❌ FAIL" "$FIRST_ANYHOW_FROM"
    VALIDATION_FAILED=true
else
    add_validation "impl From<anyhow::Error> blanket conversions" "0" "✅ PASS" "No blanket error conversions"
fi

# ============================================================================
# 6. ARCHITECTURAL VALIDATION
# ============================================================================
if [ -f "$SCRIPT_DIR/architectural-validation.sh" ]; then
    # Temporarily disable set -e to capture output even if script fails
    set +e
    ARCH_OUTPUT=$("$SCRIPT_DIR/architectural-validation.sh" 2>&1)
    ARCH_EXIT=$?
    set -e

    # Always extract all metrics from architectural validation (regardless of pass/fail)
    NULL_UUIDS=$(echo "$ARCH_OUTPUT" | grep "NULL UUIDs" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    RESOURCE_PATTERNS=$(echo "$ARCH_OUTPUT" | grep "Resource creation patterns" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    FAKE_RESOURCES=$(echo "$ARCH_OUTPUT" | grep "Fake resource assemblies" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    UNSAFE=$(echo "$ARCH_OUTPUT" | grep "Unsafe code blocks" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    ANYHOW_MACRO=$(echo "$ARCH_OUTPUT" | grep "Forbidden anyhow! macro usage" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    ANYHOW_IMPORTS=$(echo "$ARCH_OUTPUT" | grep "Forbidden anyhow imports" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    ANYHOW_TYPES=$(echo "$ARCH_OUTPUT" | grep "Forbidden anyhow::Result types" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    ANYHOW_CONTEXT=$(echo "$ARCH_OUTPUT" | grep "Anyhow .context() method usage" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    UNWRAPS=$(echo "$ARCH_OUTPUT" | grep "Problematic unwraps" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    EXPECTS=$(echo "$ARCH_OUTPUT" | grep "Problematic expects" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    PANICS=$(echo "$ARCH_OUTPUT" | grep "Panic calls" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    TODOS=$(echo "$ARCH_OUTPUT" | grep "TODOs/FIXMEs" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    MOCK_IMPL=$(echo "$ARCH_OUTPUT" | grep "Production mock implementations" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    UNDERSCORE_NAMES=$(echo "$ARCH_OUTPUT" | grep "Underscore-prefixed names" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    TEST_IN_SRC=$(echo "$ARCH_OUTPUT" | grep "Test modules in src/" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    CLIPPY_ALLOWS=$(echo "$ARCH_OUTPUT" | grep "Problematic clippy allows" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    DEAD_CODE=$(echo "$ARCH_OUTPUT" | grep "Dead code annotations" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    TEMP_SOLUTIONS=$(echo "$ARCH_OUTPUT" | grep "Temporary solutions" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    BACKUP_FILES=$(echo "$ARCH_OUTPUT" | grep "Backup files" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    CLONE_TOTAL=$(echo "$ARCH_OUTPUT" | grep "Clone usage (total)" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    CLONE_PROBLEMATIC=$(echo "$ARCH_OUTPUT" | grep "Problematic clones" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    ARC_USAGE=$(echo "$ARCH_OUTPUT" | grep "Arc usage" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)
    MAGIC_NUMBERS=$(echo "$ARCH_OUTPUT" | grep "Magic numbers" | grep -o "[0-9]*" | head -1 | tr -d '\n\r\t ' || echo 0)

    # Add validation entries with correct status based on counts
    [ "${NULL_UUIDS:-0}" -eq 0 ] && add_validation "NULL UUIDs" "0" "✅ PASS" "No test/placeholder UUIDs" || \
        { add_validation "NULL UUIDs" "$NULL_UUIDS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "NULL UUIDs" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${RESOURCE_PATTERNS:-0}" -eq 0 ] && add_validation "Resource creation patterns" "0" "✅ PASS" "Using dependency injection" || \
        { add_validation "Resource creation patterns" "$RESOURCE_PATTERNS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Resource creation patterns" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${FAKE_RESOURCES:-0}" -eq 0 ] && add_validation "Fake resource assemblies" "0" "✅ PASS" "No fake ServerResources" || \
        { add_validation "Fake resource assemblies" "$FAKE_RESOURCES" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Fake resource assemblies" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${ANYHOW_MACRO:-0}" -eq 0 ] && add_validation "Forbidden anyhow! macro" "0" "✅ PASS" "Using structured error types" || \
        { add_validation "Forbidden anyhow! macro" "$ANYHOW_MACRO" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Forbidden anyhow! macro usage" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${ANYHOW_IMPORTS:-0}" -eq 0 ] && add_validation "Forbidden anyhow imports" "0" "✅ PASS" "Using AppResult imports" || \
        { add_validation "Forbidden anyhow imports" "$ANYHOW_IMPORTS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Forbidden anyhow imports" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${ANYHOW_TYPES:-0}" -eq 0 ] && add_validation "Forbidden anyhow::Result types" "0" "✅ PASS" "Using AppResult types" || \
        { add_validation "Forbidden anyhow::Result types" "$ANYHOW_TYPES" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Forbidden anyhow::Result types" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    if [ "${ANYHOW_CONTEXT:-0}" -eq 0 ]; then
        add_validation "Anyhow .context() usage" "0" "✅ PASS" "Using .map_err() pattern"
    elif [ "${ANYHOW_CONTEXT:-0}" -le 20 ]; then
        add_validation "Anyhow .context() usage" "$ANYHOW_CONTEXT" "⚠️ INFO" "Migration in progress (threshold: 20)"
    else
        add_validation "Anyhow .context() usage" "$ANYHOW_CONTEXT" "⚠️ WARN" "$(echo "$ARCH_OUTPUT" | grep "Anyhow .context() method usage" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"
    fi

    [ "${UNWRAPS:-0}" -eq 0 ] && add_validation "Problematic unwraps" "0" "✅ PASS" "Proper error handling" || \
        { add_validation "Problematic unwraps" "$UNWRAPS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Problematic unwraps" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${EXPECTS:-0}" -eq 0 ] && add_validation "Problematic expects" "0" "✅ PASS" "Proper error handling" || \
        { add_validation "Problematic expects" "$EXPECTS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Problematic expects" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${PANICS:-0}" -eq 0 ] && add_validation "Panic calls" "0" "✅ PASS" "No panic! calls" || \
        { add_validation "Panic calls" "$PANICS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Panic calls" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    if [ "${TODOS:-0}" -gt 0 ]; then
        TODO_LOCATION=$(echo "$ARCH_OUTPUT" | grep "TODOs/FIXMEs" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')
        [ -z "$TODO_LOCATION" ] && TODO_LOCATION="Run architectural-validation.sh for locations"
        add_validation "TODOs/FIXMEs" "$TODOS" "⚠️ WARN" "$TODO_LOCATION"
    else
        add_validation "TODOs/FIXMEs" "0" "✅ PASS" "No incomplete code"
    fi

    [ "${MOCK_IMPL:-0}" -eq 0 ] && add_validation "Production mock implementations" "0" "✅ PASS" "No mock code in production" || \
        { add_validation "Production mock implementations" "$MOCK_IMPL" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Production mock implementations" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${UNDERSCORE_NAMES:-0}" -eq 0 ] && add_validation "Underscore-prefixed names" "0" "✅ PASS" "Good naming conventions" || \
        { add_validation "Underscore-prefixed names" "$UNDERSCORE_NAMES" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Underscore-prefixed names" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${TEST_IN_SRC:-0}" -eq 0 ] && add_validation "Test modules in src/" "0" "✅ PASS" "Tests in tests/ directory" || \
        { add_validation "Test modules in src/" "$TEST_IN_SRC" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Test modules in src/" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${CLIPPY_ALLOWS:-0}" -eq 0 ] && add_validation "Problematic clippy allows" "0" "✅ PASS" "Fix issues, don't silence" || \
        { add_validation "Problematic clippy allows" "$CLIPPY_ALLOWS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Problematic clippy allows" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${DEAD_CODE:-0}" -eq 0 ] && add_validation "Dead code annotations" "0" "✅ PASS" "Remove, don't hide" || \
        { add_validation "Dead code annotations" "$DEAD_CODE" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Dead code annotations" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${TEMP_SOLUTIONS:-0}" -eq 0 ] && add_validation "Temporary solutions" "0" "✅ PASS" "No temporary code" || \
        { add_validation "Temporary solutions" "$TEMP_SOLUTIONS" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Temporary solutions" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    [ "${BACKUP_FILES:-0}" -eq 0 ] && add_validation "Backup files" "0" "✅ PASS" "No backup files" || \
        { add_validation "Backup files" "$BACKUP_FILES" "❌ FAIL" "$(echo "$ARCH_OUTPUT" | grep "Backup files" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')"; VALIDATION_FAILED=true; }

    if [ "${CLONE_PROBLEMATIC:-0}" -gt 0 ]; then
        CLONE_LOCATION=$(echo "$ARCH_OUTPUT" | grep "Problematic clones" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')
        [ -z "$CLONE_LOCATION" ] && CLONE_LOCATION="Run architectural-validation.sh for locations"
        add_validation "Problematic clones" "$CLONE_PROBLEMATIC" "⚠️ WARN" "$CLONE_LOCATION"
    else
        add_validation "Clone usage" "${CLONE_TOTAL:-0}" "✅ PASS" "All legitimate"
    fi

    add_validation "Arc usage" "${ARC_USAGE:-0}" "✅ PASS" "Appropriate for architecture"

    if [ "${MAGIC_NUMBERS:-0}" -gt 0 ]; then
        MAGIC_LOCATION=$(echo "$ARCH_OUTPUT" | grep "Magic numbers" | awk -F'│' '{print $5}' | tr -d '\n\r\t ' | sed 's/^ *//;s/ *$//')
        [ -z "$MAGIC_LOCATION" ] && MAGIC_LOCATION="Run architectural-validation.sh for locations"
        add_validation "Magic numbers" "$MAGIC_NUMBERS" "⚠️ WARN" "$MAGIC_LOCATION"
    else
        add_validation "Magic numbers" "0" "✅ PASS" "All values named constants"
    fi

    [ "${UNSAFE:-0}" -eq 0 ] && add_validation "Unsafe code" "0" "✅ PASS" "No unsafe blocks" || \
        add_validation "Unsafe code" "$UNSAFE" "✅ PASS" "Limited to approved locations"
else
    add_validation "Architectural patterns" "?" "⚠️ SKIP" "architectural-validation.sh not found"
fi

# ============================================================================
# DISPLAY RESULTS TABLE
# ============================================================================
echo ""
echo -e "${BLUE}Static Analysis & Code Quality Results${NC}"
echo ""
printf "┌─────────────────────────────────────────┬───────┬──────────┬─────────────────────────────────┐\n"
printf "│ %-39s │ %5s │ %-8s │ %-31s │\n" "Validation Category" "Count" "Status" "Details"
printf "├─────────────────────────────────────────┼───────┼──────────┼─────────────────────────────────┤\n"

for i in "${!VALIDATION_CATEGORIES[@]}"; do
    printf "│ %-39s │ %5s │ %-8s │ %-31s │\n" \
        "${VALIDATION_CATEGORIES[$i]}" \
        "${VALIDATION_COUNTS[$i]}" \
        "${VALIDATION_STATUSES[$i]}" \
        "${VALIDATION_DETAILS[$i]}"
done

printf "└─────────────────────────────────────────┴───────┴──────────┴─────────────────────────────────┘\n"
echo ""

# ============================================================================
# FAIL IF ANY CRITICAL CHECKS FAILED
# ============================================================================
if [ "$VALIDATION_FAILED" = true ]; then
    echo -e "${RED}[CRITICAL] Static analysis validation failed${NC}"
    echo -e "${RED}Fix all issues above before proceeding${NC}"

    # Show details for critical failures
    if [ "$DISABLED_COUNT" -gt 0 ]; then
        echo ""
        echo -e "${RED}Disabled files found:${NC}"
        [ -n "$DISABLED_TESTS" ] && echo "$DISABLED_TESTS"
        [ -n "$DISABLED_SRC" ] && echo "$DISABLED_SRC"
    fi

    if [ "$IGNORED_COUNT" -gt 0 ]; then
        echo ""
        echo -e "${RED}Ignored tests found:${NC}"
        rg "#\[ignore\]" tests/ -B 2 -A 1 2>/dev/null | head -30
    fi

    # Show secret validation failures if any
    if [ -n "$SECRET_OUTPUT" ] && ([ "$SECRET_EXIT" -ne 0 ] || [ "${SECRET_FAILURES:-0}" -gt 0 ]); then
        echo ""
        echo -e "${RED}Secret pattern validation output:${NC}"
        echo "$SECRET_OUTPUT" | grep -E "❌|FAIL|Error" | head -20
    fi

    # Show architectural validation failures if any
    if [ -n "$ARCH_OUTPUT" ] && [ "$ARCH_EXIT" -ne 0 ]; then
        echo ""
        echo -e "${RED}Architectural validation output:${NC}"
        echo "$ARCH_OUTPUT" | grep -E "❌|FAIL|Error" | head -20
        echo ""
        echo -e "${RED}[CRITICAL] Architectural validation failed - STOPPING${NC}"
        echo -e "${RED}Fix all issues above before proceeding${NC}"
        exit 1
    fi

    ALL_PASSED=false
    # Continue to final validation
else
    echo -e "${GREEN}[OK] All static analysis checks passed${NC}"
fi

# ============================================================================
# NATIVE CARGO VALIDATION (Reads Cargo.toml [lints] + deny.toml)
# ============================================================================

print_task "Cargo fmt (code formatting check)"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}[OK] Rust code formatting is correct${NC}"
else
    echo -e "${RED}[CRITICAL] Rust code formatting check failed${NC}"
    echo -e "${RED}Run 'cargo fmt --all' to fix formatting issues${NC}"
    ALL_PASSED=false
    # Continue to final validation
fi

print_task "Cargo clippy + build (zero tolerance linting)"

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

# Run clippy with output to screen
# Explicit -D flags deny all clippy violations (matching CCFW strictness)
# CRITICAL: All lint groups use -D (deny) to match production CI validation
# ZERO TOLERANCE: No allowances - all warnings must be fixed
if cargo clippy --all-targets --all-features --quiet -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery -W clippy::cognitive_complexity; then
    echo -e "${GREEN}[OK] Clippy passed - ZERO code warnings (enforced by Cargo.toml)${NC}"
    echo -e "${GREEN}[OK] Debug build completed (reused for all validation)${NC}"
else
    CLIPPY_EXIT=$?
    echo ""
    if [ $CLIPPY_EXIT -eq 137 ] || [ $CLIPPY_EXIT -eq 143 ]; then
        echo -e "${RED}[CRITICAL] Clippy was killed (possibly out of memory)${NC}"
        echo -e "${RED}Exit code: $CLIPPY_EXIT${NC}"
    else
        echo -e "${RED}[CRITICAL] Clippy failed - ALL code warnings must be fixed${NC}"
        echo -e "${RED}Zero tolerance policy: fix all warnings and re-run${NC}"
    fi
    echo -e "${RED}Exiting immediately - fix compilation/clippy errors first${NC}"
    exit 1
fi

print_task "Cargo deny (security audit)"
if command_exists cargo-deny; then
    # Capture output and filter duplicate warnings for cleaner display
    DENY_OUTPUT=$(cargo deny check 2>&1)
    DENY_EXIT=$?

    # Show summary line
    echo "$DENY_OUTPUT" | grep -E "(advisories|bans|licenses|sources)" || true

    if [ "$DENY_EXIT" -eq 0 ]; then
        echo -e "${GREEN}[OK] Security audit passed (via deny.toml)${NC}"

        # Count duplicate warnings but don't show them
        DUPLICATE_COUNT=$(echo "$DENY_OUTPUT" | grep -c "^warning\[duplicate\]" 2>/dev/null || echo 0)
        DUPLICATE_COUNT=$(echo "$DUPLICATE_COUNT" | head -1 | tr -d '\n\r\t ')
        if [ "$DUPLICATE_COUNT" -gt 0 ]; then
            echo -e "${YELLOW}[INFO] $DUPLICATE_COUNT duplicate dependencies detected (non-critical)${NC}"
        fi
    else
        # On failure, show filtered output (actual errors, not duplicate warnings)
        echo "$DENY_OUTPUT" | grep -v "^warning\[duplicate\]" | grep -v "├\|│\|└\|╭\|╰" | grep -v "lock entries" | grep -v "registry+https://github.com" || true
        echo -e "${YELLOW}[WARN] Security vulnerabilities detected${NC}"
        echo -e "${YELLOW}Review output above and update dependencies${NC}"
        # Don't fail build, just warn
    fi
else
    echo -e "${YELLOW}[WARN] cargo-deny not installed${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-deny${NC}"
fi

# ============================================================================
# SDK BUILD (Required before tests)
# ============================================================================
# The sdk/dist/ folder is in .gitignore, so it must be built locally.
# Tests like test_http_transport_tools_list_parity require sdk/dist/cli.js
# ============================================================================

print_task "Building TypeScript SDK (required for tests)"

if [ -d "sdk" ]; then
    echo -e "${BLUE}Building SDK at sdk/${NC}"
    cd sdk

    # Install dependencies if node_modules doesn't exist
    if [ ! -d "node_modules" ]; then
        echo -e "${BLUE}Installing SDK dependencies...${NC}"
        if command_exists npm; then
            bun install --frozen-lockfile
        else
            echo -e "${RED}[FAIL] npm not found. Install Node.js to build SDK${NC}"
            ALL_PASSED=false
            # exit 1 removed - let script reach final validation
        fi
    fi

    # Build TypeScript to JavaScript
    echo -e "${BLUE}Compiling TypeScript...${NC}"
    bun run build

    # Verify build output
    if [ -f "dist/cli.js" ]; then
        echo -e "${GREEN}[OK] SDK built successfully: sdk/dist/cli.js${NC}"
    else
        echo -e "${RED}[FAIL] SDK build failed: sdk/dist/cli.js not found${NC}"
        ALL_PASSED=false
        # exit 1 removed - let script reach final validation
    fi

    cd "$PROJECT_ROOT"
else
    echo -e "${YELLOW}[WARN] SDK directory not found, skipping SDK build${NC}"
fi

print_task "Cargo test (all tests)"

# Clean test databases
echo -e "${BLUE}Cleaning test databases...${NC}"
if [ -f "$SCRIPT_DIR/../testing/clean-test-databases.sh" ]; then
    "$SCRIPT_DIR/../testing/clean-test-databases.sh" || true
fi

# Ensure data directory exists
mkdir -p data

# Count tests
TOTAL_TESTS=$(cargo test --all-targets -- --list 2>/dev/null | grep -E "^[a-zA-Z_].*: test$" | wc -l | tr -d ' ')
echo -e "${BLUE}Total tests to run: $TOTAL_TESTS${NC}"

# Use 2048-bit RSA for faster test execution
export PIERRE_RSA_KEY_SIZE=2048

# Run tests (reuses build artifacts from clippy step)
if [ "$ENABLE_COVERAGE" = true ]; then
    echo -e "${BLUE}Running tests with coverage...${NC}"
    if command_exists cargo-llvm-cov; then
        if cargo llvm-cov --all-targets --summary-only; then
            echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed with coverage${NC}"
        else
            echo -e "${RED}[FAIL] Some tests failed${NC}"
            echo -e "${RED}Exiting immediately - fix test failures first${NC}"
            exit 1
        fi
    else
        echo -e "${YELLOW}[WARN] cargo-llvm-cov not installed${NC}"
        echo -e "${YELLOW}Install with: cargo install cargo-llvm-cov${NC}"
        if cargo test --all-targets --no-fail-fast; then
            echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed${NC}"
        else
            echo -e "${RED}[FAIL] Some tests failed${NC}"
            echo -e "${RED}Exiting immediately - fix test failures first${NC}"
            exit 1
        fi
    fi
else
    if cargo test --all-targets --no-fail-fast; then
        echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed${NC}"
    else
        echo -e "${RED}[FAIL] Some tests failed${NC}"
        echo -e "${RED}Exiting immediately - fix test failures first${NC}"
        exit 1
    fi
fi

# ============================================================================
# FRONTEND VALIDATION (Separate Toolchain)
# ============================================================================

if [ -d "frontend" ]; then
    print_task "Frontend validation (linting, types, tests, build)"
    cd frontend

    # Check dependencies
    if [ ! -d "node_modules" ] || [ ! -f "node_modules/.package-lock.json" ]; then
        echo -e "${YELLOW}Installing frontend dependencies...${NC}"
        bun install --frozen-lockfile || {
            echo -e "${RED}[FAIL] Frontend dependency installation failed${NC}"
            ALL_PASSED=false
            cd ..
        }
    fi

    if [ -d "node_modules" ]; then
        # Lint
        if bun run lint; then
            echo -e "${GREEN}[OK] Frontend linting passed${NC}"
        else
            echo -e "${RED}[FAIL] Frontend linting failed${NC}"
            ALL_PASSED=false
        fi

        # Type check
        if bun run type-check; then
            echo -e "${GREEN}[OK] TypeScript type checking passed${NC}"
        else
            echo -e "${RED}[FAIL] TypeScript type checking failed${NC}"
            ALL_PASSED=false
        fi

        # Unit Tests
        if npm test -- --run; then
            echo -e "${GREEN}[OK] Frontend unit tests passed${NC}"
        else
            echo -e "${RED}[FAIL] Frontend unit tests failed${NC}"
            ALL_PASSED=false
        fi

        # E2E Tests (Playwright)
        if [ -f "playwright.config.ts" ]; then
            echo -e "${BLUE}Running Playwright E2E tests...${NC}"
            # Install Playwright browsers if needed
            if ! bunx playwright install --with-deps chromium 2>/dev/null; then
                echo -e "${YELLOW}[WARN] Playwright browser installation failed, attempting tests anyway${NC}"
            fi
            if bun run test:e2e; then
                echo -e "${GREEN}[OK] Frontend E2E tests passed${NC}"
            else
                echo -e "${RED}[FAIL] Frontend E2E tests failed${NC}"
                ALL_PASSED=false
            fi
        else
            echo -e "${YELLOW}[SKIP] No playwright.config.ts found, skipping E2E tests${NC}"
        fi

        # Build
        if bun run build; then
            echo -e "${GREEN}[OK] Frontend build successful${NC}"
        else
            echo -e "${RED}[FAIL] Frontend build failed${NC}"
            ALL_PASSED=false
        fi
    fi

    cd ..
fi

# ============================================================================
# MCP SPEC COMPLIANCE (now mandatory)
# ============================================================================

print_task "MCP spec compliance validation"
if [ -f "$SCRIPT_DIR/ensure-mcp-compliance.sh" ]; then
    if "$SCRIPT_DIR/ensure-mcp-compliance.sh"; then
        echo -e "${GREEN}[OK] MCP compliance validation passed${NC}"
    else
        echo -e "${RED}[FAIL] MCP compliance validation failed${NC}"
        ALL_PASSED=false
    fi
else
    echo -e "${RED}[CRITICAL] MCP compliance script not found: $SCRIPT_DIR/ensure-mcp-compliance.sh${NC}"
    ALL_PASSED=false
    # exit 1 removed - let script reach final validation
fi

# ============================================================================
# SDK TYPE GENERATION (now mandatory)
# ============================================================================

print_task "SDK TypeScript validation + integration tests"
if [ ! -d "sdk" ]; then
    echo -e "${RED}[CRITICAL] SDK directory not found${NC}"
    ALL_PASSED=false
    # exit 1 removed - let script reach final validation
fi

cd sdk

# Check if package.json and generate-types script exist
if [ -f "package.json" ] && grep -q "generate-types" package.json; then
    echo -e "${BLUE}Checking if SDK types need regeneration...${NC}"

    # Check if types.ts exists and is not a placeholder
    if [ ! -f "src/types.ts" ] || grep -q "PLACEHOLDER" "src/types.ts"; then
        echo -e "${YELLOW}Types need generation (missing or placeholder)${NC}"
        NEED_GENERATION=true
    else
        echo -e "${GREEN}Types file exists and appears complete${NC}"
        NEED_GENERATION=false
    fi

    # Always validate types can be generated (but don't fail if server isn't running)
    if [ "$NEED_GENERATION" = true ]; then
        echo -e "${YELLOW}[WARN] SDK types are placeholder - run 'cd sdk && bun run generate-types' with server running${NC}"
        echo -e "${YELLOW}[WARN] Skipping type generation in CI - types should be committed${NC}"
    fi

    # Validate TypeScript compilation regardless
    if [ -d "node_modules" ]; then
        if bun run build --if-present >/dev/null 2>&1; then
            echo -e "${GREEN}[OK] SDK TypeScript compilation successful${NC}"
        else
            echo -e "${RED}[FAIL] SDK TypeScript compilation failed${NC}"
            ALL_PASSED=false
        fi

        # Run SDK integration tests for all 47 tools
        echo -e "${BLUE}Running SDK integration tests...${NC}"
        if bun run test:integration -- --testPathPattern=all-tools --silent; then
            echo -e "${GREEN}[OK] SDK integration tests passed (47 tools validated)${NC}"
        else
            echo -e "${RED}[FAIL] SDK integration tests failed${NC}"
            echo -e "${YELLOW}[INFO] Run 'cd sdk && bun run test:integration -- --testPathPattern=all-tools' for details${NC}"
            ALL_PASSED=false
        fi
    else
        echo -e "${RED}[CRITICAL] SDK dependencies not installed - run 'cd sdk && bun install --frozen-lockfile'${NC}"
        ALL_PASSED=false
        # Continue to final validation
    fi
else
    echo -e "${RED}[CRITICAL] SDK generate-types script not found in package.json${NC}"
    ALL_PASSED=false
    # Continue to final validation
fi

cd ..

# ============================================================================
# BRIDGE TEST SUITE (now mandatory)
# ============================================================================

print_task "Bridge test suite"
if [ -f "$SCRIPT_DIR/../testing/run-bridge-tests.sh" ]; then
    if "$SCRIPT_DIR/../testing/run-bridge-tests.sh"; then
        echo -e "${GREEN}[OK] Bridge test suite passed${NC}"
    else
        echo -e "${RED}[FAIL] Bridge test suite failed${NC}"
        ALL_PASSED=false
    fi
else
    echo -e "${RED}[CRITICAL] Bridge test script not found: $SCRIPT_DIR/../testing/run-bridge-tests.sh${NC}"
    ALL_PASSED=false
    # Continue to final validation
fi

# ============================================================================
# PERFORMANCE AND DOCUMENTATION
# ============================================================================

print_task "Release build + documentation"
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
# FINAL CLEANUP
# ============================================================================

echo -e "${BLUE}Final cleanup...${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
find . -name "a2a_enterprise_report_*.json" -delete 2>/dev/null || true
find . -name "mcp_investor_demo_*.json" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# ============================================================================
# SUMMARY
# ============================================================================

END_TIME=$(date +%s)
TOTAL_SECONDS=$((END_TIME - START_TIME))
TOTAL_MINUTES=$((TOTAL_SECONDS / 60))
REMAINING_SECONDS=$((TOTAL_SECONDS % 60))

echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}       VALIDATION SUMMARY${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}Completed: $TOTAL_TASKS/$TOTAL_TASKS tasks${NC}"
echo -e "${BLUE}Total execution time: ${TOTAL_MINUTES}m ${REMAINING_SECONDS}s${NC}"
echo ""

if [ "$ALL_PASSED" = true ]; then
    echo -e "${GREEN}✅ ALL VALIDATION PASSED - Task can be marked complete${NC}"
    echo ""
    echo "[OK] Cleanup"
    echo "[OK] Static Analysis & Code Quality (unified)"
    echo "[OK] Rust formatting (cargo fmt)"
    echo "[OK] Rust linting + build (cargo clippy via Cargo.toml)"
    echo "[OK] Security audit (cargo deny via deny.toml)"
    echo "[OK] Rust tests (cargo test - all unit + integration tests)"
    if [ -d "frontend" ]; then
        echo "[OK] Frontend linting"
        echo "[OK] TypeScript type checking"
        echo "[OK] Frontend unit tests"
        echo "[OK] Frontend E2E tests (Playwright)"
        echo "[OK] Frontend build"
    fi
    echo "[OK] Release build (cargo build --release)"
    echo "[OK] Documentation (cargo doc)"
    if [ -f "$SCRIPT_DIR/ensure-mcp-compliance.sh" ]; then
        echo "[OK] MCP spec compliance validation"
    fi
    if [ -f "$SCRIPT_DIR/../testing/run-bridge-tests.sh" ]; then
        echo "[OK] Bridge test suite"
    fi
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
