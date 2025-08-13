#!/bin/bash

# ABOUTME: Architectural validation script to prevent facade patterns and ensure implementations work
# ABOUTME: Scans for stubbed implementations, facade patterns, and validates factory delegation

set -e

PROJECT_ROOT="/Users/jeanfrancoisarcand/workspace/strava_ai/pierre_mcp_server"
cd "$PROJECT_ROOT"

echo "🏗️  Architectural Validation Suite"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VALIDATION_FAILED=0

# Function to report validation failure
fail_validation() {
    echo -e "${RED}❌ ARCHITECTURAL VALIDATION FAILED${NC}"
    echo -e "${RED}$1${NC}"
    VALIDATION_FAILED=1
}

# Function to report warning
warn_validation() {
    echo -e "${YELLOW}⚠️  ARCHITECTURAL WARNING${NC}"
    echo -e "${YELLOW}$1${NC}"
}

# Function to report success
pass_validation() {
    echo -e "${GREEN}✅ $1${NC}"
}

echo -e "${BLUE}==== Checking for Stubbed Implementations ====${NC}"

# Check for stubbed implementations that return hardcoded values
STUB_PATTERNS=(
    "Err(anyhow::anyhow!(\".*not yet implemented"
    "// Stub implementation.*TODO"
    "unimplemented!()"
    "todo!()"
    "Ok(Vec::new()).*// Stub"
    "Ok(None).*// Stub"
    "Ok(()).*// Stub"
)

for pattern in "${STUB_PATTERNS[@]}"; do
    MATCHES=$(rg -n "$pattern" src/ || true)
    if [[ -n "$MATCHES" ]]; then
        fail_validation "Found stubbed implementations that could break functionality at runtime:
$MATCHES

These implementations appear complete but actually return hardcoded/empty values."
    fi
done

pass_validation "No obvious stubbed implementations found"

echo -e "${BLUE}==== Checking Factory Delegation Patterns ====${NC}"

# Check for factory enums that might have delegation issues
FACTORY_FILES=$(find src/ -name "*.rs" -exec grep -l "enum.*Database" {} \; || true)

for file in $FACTORY_FILES; do
    echo "Checking factory file: $file"
    
    # Look for match statements that might be incomplete
    MATCH_BLOCKS=$(rg -A 5 "match self" "$file" || true)
    if [[ -n "$MATCH_BLOCKS" ]]; then
        # Check if any match blocks have stubbed arms
        STUBBED_ARMS=$(echo "$MATCH_BLOCKS" | rg "// Stub|TODO|unimplemented|not yet implemented" || true)
        if [[ -n "$STUBBED_ARMS" ]]; then
            fail_validation "Factory delegation issue in $file:
$STUBBED_ARMS

Factory pattern has stubbed match arms that will break at runtime."
        fi
    fi
done

pass_validation "Factory delegation patterns look correct"

echo -e "${BLUE}==== Checking for Facade vs Implementation Gap ====${NC}"

# Check for traits with many methods but factories with few implementations
TRAIT_FILES=$(find src/ -name "*.rs" -exec grep -l "trait.*Provider" {} \; || true)

for file in $TRAIT_FILES; do
    echo "Checking provider trait: $file"
    
    METHOD_COUNT=$(rg "async fn" "$file" | wc -l || echo "0")
    if [[ $METHOD_COUNT -gt 20 ]]; then
        echo "Found large trait with $METHOD_COUNT methods in $file"
        
        # Check if factory has implementations for these methods
        FACTORY_FILE="src/database_plugins/factory.rs"
        if [[ -f "$FACTORY_FILE" ]]; then
            FACTORY_METHODS=$(rg "async fn" "$FACTORY_FILE" | wc -l || echo "0")
            COVERAGE_RATIO=$((FACTORY_METHODS * 100 / METHOD_COUNT))
            
            if [[ $COVERAGE_RATIO -lt 80 ]]; then
                warn_validation "Potential facade pattern: Trait has $METHOD_COUNT methods but factory only implements $FACTORY_METHODS (~${COVERAGE_RATIO}% coverage)"
            fi
        fi
    fi
done

echo -e "${BLUE}==== Checking Database Operation Tests ====${NC}"

# Check if we have tests that validate database operations work through factory
TEST_FILES=$(find tests/ -name "*test*.rs" -exec grep -l "Database::new" {} \; || true)

if [[ -z "$TEST_FILES" ]]; then
    warn_validation "No integration tests found that validate database operations through factory pattern"
else
    pass_validation "Found integration tests for database operations"
fi

echo -e "${BLUE}==== Checking for Runtime vs Compile-time Validation ====${NC}"

# Look for code that might compile but fail at runtime
RUNTIME_FAILURE_PATTERNS=(
    "Err(anyhow!"
    "panic!"
    "unimplemented!"
    "todo!"
)

for pattern in "${RUNTIME_FAILURE_PATTERNS[@]}"; do
    MATCHES=$(rg -n "$pattern" src/database_plugins/factory.rs || true)
    if [[ -n "$MATCHES" ]]; then
        fail_validation "Factory contains runtime failure patterns that could cause 0% functionality:
$MATCHES"
    fi
done

pass_validation "No obvious runtime failure patterns in factory"

echo -e "${BLUE}==== Checking for Missing Abstraction Implementation ====${NC}"

# Check if database providers implement all required methods
SQLITE_FILE="src/database_plugins/sqlite.rs"
POSTGRES_FILE="src/database_plugins/postgres.rs"
TRAIT_FILE="src/database_plugins/mod.rs"

if [[ -f "$TRAIT_FILE" ]]; then
    TRAIT_METHODS=$(rg "async fn" "$TRAIT_FILE" | rg -v "//" | wc -l || echo "0")
    
    if [[ -f "$SQLITE_FILE" ]]; then
        SQLITE_METHODS=$(rg "async fn" "$SQLITE_FILE" | rg -v "//" | wc -l || echo "0")
        if [[ $SQLITE_METHODS -lt $TRAIT_METHODS ]]; then
            warn_validation "SQLite provider may be missing implementations: $SQLITE_METHODS/$TRAIT_METHODS methods"
        fi
    fi
    
    if [[ -f "$POSTGRES_FILE" ]]; then
        POSTGRES_METHODS=$(rg "async fn" "$POSTGRES_FILE" | rg -v "//" | wc -l || echo "0")
        if [[ $POSTGRES_METHODS -lt $TRAIT_METHODS ]]; then
            warn_validation "PostgreSQL provider may be missing implementations: $POSTGRES_METHODS/$TRAIT_METHODS methods"
        fi
    fi
fi

echo -e "${BLUE}==== Running Critical Architecture Tests ====${NC}"

# Run our specific architectural validation test
if cargo test test_tenant_operations_work_through_factory --test tenant_context_resolution_test --quiet; then
    pass_validation "Critical architecture tests pass - factory delegation works"
else
    fail_validation "Critical architecture tests failed - factory delegation broken"
fi

echo -e "${BLUE}==== Architecture Validation Summary ====${NC}"

if [[ $VALIDATION_FAILED -eq 0 ]]; then
    echo -e "${GREEN}🎉 ARCHITECTURAL VALIDATION PASSED${NC}"
    echo -e "${GREEN}All checks passed - no facade patterns or broken abstractions detected${NC}"
    exit 0
else
    echo -e "${RED}💥 ARCHITECTURAL VALIDATION FAILED${NC}"
    echo -e "${RED}Critical architectural issues detected that could cause runtime failures${NC}"
    echo -e "${RED}Fix all issues above before proceeding${NC}"
    exit 1
fi