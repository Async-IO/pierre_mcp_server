#!/bin/bash
# ABOUTME: Custom architectural validation that Cargo/Clippy cannot check
# ABOUTME: Enforces project-specific patterns using validation-patterns.toml

# This script contains ONLY validation logic that has NO native Cargo equivalent:
# 1. TOML-based pattern validation (NULL UUIDs, placeholders, Algorithm DI, etc.)
# 2. Clone/Arc usage analysis and documentation validation
# 3. Binary size enforcement (production quality gate)
# 4. Legacy function detection (UX anti-patterns)
#
# Everything else (formatting, linting, security) is now handled by:
# - cargo fmt --check (formatting)
# - cargo clippy (lints from Cargo.toml [lints] table)
# - cargo deny check (security via deny.toml)

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo -e "${BLUE}==== Pierre MCP Server - Architectural Validation ====${NC}"
echo "Project root: $PROJECT_ROOT"
cd "$PROJECT_ROOT"

# Track overall success
VALIDATION_FAILED=false

# Function to report validation failure
fail_validation() {
    echo -e "${RED}❌ ARCHITECTURAL VALIDATION FAILED${NC}"
    echo -e "${RED}$1${NC}"
    VALIDATION_FAILED=true
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

echo ""
echo -e "${BLUE}============================================================================${NC}"
echo -e "${BLUE}==== UNIFIED ARCHITECTURAL VALIDATION SUITE ====${NC}"
echo -e "${BLUE}============================================================================${NC}"
echo ""

# Load validation patterns from TOML configuration
VALIDATION_PATTERNS_FILE="$SCRIPT_DIR/validation-patterns.toml"
if [ ! -f "$VALIDATION_PATTERNS_FILE" ]; then
    echo -e "${RED}[CRITICAL] Validation patterns file not found: $VALIDATION_PATTERNS_FILE${NC}"
    exit 1
fi

# Parse TOML patterns
eval "$(python3 "$SCRIPT_DIR/parse-validation-patterns.py" "$VALIDATION_PATTERNS_FILE")"

# ============================================================================
# CRITICAL PATTERN VALIDATION (Fast-Fail)
# ============================================================================

echo -e "${BLUE}Checking for critical anti-patterns...${NC}"

# NULL UUID detection (absolute blocker)
NULL_UUIDS=$(rg "00000000-0000-0000-0000-000000000000" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$NULL_UUIDS" -gt 0 ]; then
    echo -e "${RED}❌ CRITICAL: Found $NULL_UUIDS null UUIDs (test/placeholder code)${NC}"
    rg "00000000-0000-0000-0000-000000000000" src/ -n
    fail_validation "Null UUIDs indicate incomplete implementation"
    exit 1
fi

# Implementation placeholders
IMPLEMENTATION_PLACEHOLDERS=$(rg "$CRITICAL_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$IMPLEMENTATION_PLACEHOLDERS" -gt 0 ]; then
    echo -e "${RED}❌ Found $IMPLEMENTATION_PLACEHOLDERS placeholder implementations${NC}"
    rg "$CRITICAL_PATTERNS" src/ -n | head -10
    fail_validation "Placeholder implementations must be completed"
fi

# FORBIDDEN anyhow! macro usage (CLAUDE.md violation)
TOML_ERROR_CONTEXT=$(rg "$ERROR_CONTEXT_ANTIPATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$TOML_ERROR_CONTEXT" -gt 0 ]; then
    echo -e "${RED}❌ FORBIDDEN: Found $TOML_ERROR_CONTEXT uses of anyhow! macro${NC}"
    rg "\\banyhow!\\(|anyhow::anyhow!\\(" src/ -g "!src/bin/*" -g "!tests/*" -n | head -5
    fail_validation "Use AppError/DatabaseError/ProviderError instead of anyhow!"
fi

# ============================================================================
# ALGORITHM DI ARCHITECTURE ENFORCEMENT
# ============================================================================

echo -e "${BLUE}Validating Algorithm DI patterns...${NC}"

TOTAL_ALGORITHM_VIOLATIONS=0
ALGORITHMS_WITH_VIOLATIONS=""

if [ -n "$MIGRATED_ALGORITHMS" ]; then
    for algo in $MIGRATED_ALGORITHMS; do
        algo_upper=$(echo "$algo" | tr '[:lower:]' '[:upper:]' | tr '-' '_')
        patterns_var="ALGORITHM_${algo_upper}_PATTERNS"
        excludes_var="ALGORITHM_${algo_upper}_EXCLUDES"
        name_var="ALGORITHM_${algo_upper}_NAME"

        eval "patterns=\$$patterns_var"
        eval "excludes=\$$excludes_var"
        eval "algo_name=\$$name_var"

        if [ -n "$patterns" ] && [ -n "$excludes" ]; then
            EXCLUDE_FLAGS=""
            for exclude in $excludes; do
                EXCLUDE_FLAGS="$EXCLUDE_FLAGS -g !$exclude"
            done

            violations=$(rg "$patterns" src/ $EXCLUDE_FLAGS 2>/dev/null | grep -v "^\s*//" | wc -l | awk '{print $1+0}')

            if [ "$violations" -gt 0 ]; then
                TOTAL_ALGORITHM_VIOLATIONS=$((TOTAL_ALGORITHM_VIOLATIONS + violations))
                if [ -z "$ALGORITHMS_WITH_VIOLATIONS" ]; then
                    ALGORITHMS_WITH_VIOLATIONS="$algo_name($violations)"
                else
                    ALGORITHMS_WITH_VIOLATIONS="$ALGORITHMS_WITH_VIOLATIONS, $algo_name($violations)"
                fi
            fi
        fi
    done
fi

if [ "$TOTAL_ALGORITHM_VIOLATIONS" -gt 0 ]; then
    echo -e "${RED}❌ Algorithm DI violations: $ALGORITHMS_WITH_VIOLATIONS${NC}"
    fail_validation "Use enum-based DI in src/intelligence/algorithms/"
else
    pass_validation "Algorithm DI architecture compliance"
fi

# ============================================================================
# BINARY SIZE VALIDATION (Production Quality Gate)
# ============================================================================

echo ""
echo -e "${BLUE}==== Binary Size Validation ====${NC}"

if [ -f "target/release/pierre-mcp-server" ]; then
    BINARY_SIZE=$(ls -lh target/release/pierre-mcp-server | awk '{print $5}')
    BINARY_SIZE_BYTES=$(ls -l target/release/pierre-mcp-server | awk '{print $5}')
    MAX_SIZE_BYTES=$((50 * 1024 * 1024))  # 50MB limit

    if [ "$BINARY_SIZE_BYTES" -le "$MAX_SIZE_BYTES" ]; then
        pass_validation "Binary size ($BINARY_SIZE) within limit (<50MB)"
    else
        echo -e "${RED}❌ Binary size ($BINARY_SIZE) exceeds 50MB limit${NC}"
        fail_validation "Binary size exceeds production quality gate"
    fi
else
    warn_validation "Binary not found - run 'cargo build --release' first"
fi

# ============================================================================
# BACKUP FILES CHECK (Development Hygiene)
# ============================================================================

echo ""
echo -e "${BLUE}==== Checking for backup files ====${NC}"

BACKUP_FILES=$(find src tests -name "*.backup" -o -name "*.bak" 2>/dev/null)
if [ -n "$BACKUP_FILES" ]; then
    echo -e "${RED}[FAIL] Backup files found (must be removed):${NC}"
    echo "$BACKUP_FILES"
    fail_validation "Remove backup files before commit"
else
    pass_validation "No backup files found"
fi

# ============================================================================
# LEGACY FUNCTION DETECTION (UX Anti-Patterns)
# ============================================================================

echo ""
echo -e "${BLUE}==== Legacy Function Detection ====${NC}"

LEGACY_OAUTH=$(rg "Legacy OAuth not supported|legacy.*oauth|connect_strava|connect_fitbit" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
DEPRECATED_FUNCTIONS=$(rg "deprecated.*use.*instead|Universal.*deprecated|ProviderManager deprecated" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
PLACEHOLDER_IMPLEMENTATIONS=$(rg "fn handle_.*-> Value" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
DISCARDED_EXPENSIVE_OPS=$(rg -B 2 -A 5 'let _ = \(' src/ | grep -v 'src/bin/' | rg '\.clone\(\)' | wc -l 2>/dev/null || echo 0)

LEGACY_ISSUES=0
LEGACY_ISSUES=$((LEGACY_ISSUES + LEGACY_OAUTH + DEPRECATED_FUNCTIONS + PLACEHOLDER_IMPLEMENTATIONS + DISCARDED_EXPENSIVE_OPS))

if [ "$LEGACY_ISSUES" -gt 0 ]; then
    echo -e "${RED}❌ Found $LEGACY_ISSUES legacy/stub functions that confuse users${NC}"
    [ "$LEGACY_OAUTH" -gt 0 ] && echo "  - Legacy OAuth patterns: $LEGACY_OAUTH"
    [ "$DEPRECATED_FUNCTIONS" -gt 0 ] && echo "  - Deprecated functions: $DEPRECATED_FUNCTIONS"
    [ "$PLACEHOLDER_IMPLEMENTATIONS" -gt 0 ] && echo "  - Placeholder handlers: $PLACEHOLDER_IMPLEMENTATIONS"
    [ "$DISCARDED_EXPENSIVE_OPS" -gt 0 ] && echo "  - Discarded expensive ops: $DISCARDED_EXPENSIVE_OPS"
    fail_validation "Remove legacy functions that advertise but don't work"
else
    pass_validation "No legacy UX anti-patterns detected"
fi

# ============================================================================
# SUMMARY
# ============================================================================

echo ""
echo -e "${BLUE}==== Architectural Validation Summary ====${NC}"

if [ "$VALIDATION_FAILED" = true ]; then
    echo -e "${RED}❌ Architectural validation FAILED${NC}"
    echo -e "${RED}Fix critical issues above before deployment${NC}"
    exit 1
else
    echo -e "${GREEN}✅ All architectural validations passed${NC}"
    exit 0
fi
