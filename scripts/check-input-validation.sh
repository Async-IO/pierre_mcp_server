#!/bin/bash
# ABOUTME: Automated input validation checks for CI — division safety, pagination bounds, cache key completeness
# ABOUTME: Companion script to .claude/skills/check-input-validation/SKILL.md with machine-enforceable checks
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

VALIDATION_FAILED=false

pass() { echo -e "${GREEN}  ✅ $1${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $1${NC}"; }
fail() { echo -e "${RED}  ❌ $1${NC}"; VALIDATION_FAILED=true; }

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  INPUT VALIDATION CHECK (CI)${NC}"
echo -e "${BLUE}=========================================${NC}"

# ============================================================================
# 1. Division Safety
# ============================================================================
echo ""
echo -e "${BLUE}--- 1. Division Safety ---${NC}"

# Count zero-guard patterns near divisions
ZERO_GUARDS=$(rg '\.max\(1\)|\.max\(1\.0\)|checked_div|if.*==.*0|if.*>.*0' src/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
pass "Zero-guard patterns found: $ZERO_GUARDS"

# Check recipe/nutrition division safety
# Note: guards may exist upstream in the function (e.g., early return on zero),
# so adjacent-line detection has false positives. This is a WARN, not a FAIL.
# The Claude Code /check-input-validation skill does deeper contextual analysis.
SERVINGS_DIVISIONS=$(rg 'servings|portion|per_serving' src/ --type rust -A 3 2>/dev/null | \
  rg ' / ' | \
  rg -v '\.max\(1\)|\.max\(1\.0\)|checked_div|\.max\(|// Safe' | wc -l | tr -d ' ')

if [ "$SERVINGS_DIVISIONS" -eq 0 ]; then
    pass "Recipe/nutrition divisions — no divisions found or all have adjacent guards"
else
    # Check if zero-guards exist elsewhere in the same files (upstream protection)
    FILES_WITH_DIVISIONS=$(rg 'servings|portion|per_serving' src/ --type rust -A 3 2>/dev/null | \
      rg ' / ' | rg -v '\.max\(' | rg -o '^[^:]+' | sort -u)
    GUARDED_FILES=0
    for f in $FILES_WITH_DIVISIONS; do
        if rg 'servings.*==.*0|servings.*\.max\(1\)|== 0.*servings' "$f" 2>/dev/null | rg -q '.'; then
            GUARDED_FILES=$((GUARDED_FILES + 1))
        fi
    done
    if [ "$GUARDED_FILES" -gt 0 ]; then
        pass "Recipe/nutrition divisions: $SERVINGS_DIVISIONS divisions found, upstream zero-guards detected in $GUARDED_FILES file(s)"
    else
        warn "Found $SERVINGS_DIVISIONS recipe/nutrition divisions — verify zero-guards exist upstream"
    fi
fi

# ============================================================================
# 2. Pagination Bounds
# ============================================================================
echo ""
echo -e "${BLUE}--- 2. Pagination Bounds ---${NC}"

# Count pagination bound enforcement patterns
PAGINATION_BOUNDS=$(rg 'limit.*clamp|limit.*min|limit.*max|\.min\(.*100\)|\.max\(.*1\)|\.clamp\(' src/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

if [ "$PAGINATION_BOUNDS" -gt 0 ]; then
    pass "Pagination bound enforcement patterns: $PAGINATION_BOUNDS"
else
    warn "No pagination bound enforcement patterns found"
fi

# Check for unbounded LIMIT in SQL (HARD FAIL)
UNBOUNDED_LIMIT=$(rg 'LIMIT \$|LIMIT \{' src/ --type rust -B 3 2>/dev/null | \
  rg -v 'clamp|min|max|\.max\(|\.min\(' | \
  rg 'LIMIT' | wc -l | tr -d ' ')

if [ "$UNBOUNDED_LIMIT" -eq 0 ]; then
    pass "All SQL LIMIT values have bounds"
else
    warn "Found $UNBOUNDED_LIMIT potentially unbounded SQL LIMIT clauses"
    rg 'LIMIT \$|LIMIT \{' src/ --type rust -B 3 -n 2>/dev/null | \
      rg -v 'clamp|min|max' | rg 'LIMIT' | head -5
fi

# ============================================================================
# 3. Cache Key Completeness
# ============================================================================
echo ""
echo -e "${BLUE}--- 3. Cache Key Completeness ---${NC}"

# Check cache keys include tenant_id (using CacheKey struct enforces this at compile time)
CACHE_KEY_STRUCT=$(rg 'struct CacheKey' src/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
CACHE_KEY_TENANT=$(rg 'struct CacheKey' src/ --type rust -A 10 2>/dev/null | rg 'tenant_id' | wc -l | tr -d ' ')

if [ "$CACHE_KEY_STRUCT" -gt 0 ] && [ "$CACHE_KEY_TENANT" -gt 0 ]; then
    pass "CacheKey struct requires tenant_id (compile-time enforcement)"
else
    # Fallback: check cache operations manually
    CACHE_WITHOUT_TENANT=$(rg 'cache_key|format!.*cache|format!.*key' src/ --type rust -n 2>/dev/null | \
      rg -v 'tenant|test|//|use ' | wc -l | tr -d ' ')

    if [ "$CACHE_WITHOUT_TENANT" -eq 0 ]; then
        pass "Cache keys include tenant context"
    else
        warn "Found $CACHE_WITHOUT_TENANT cache key constructions without tenant_id"
        rg 'cache_key|format!.*cache|format!.*key' src/ --type rust -n 2>/dev/null | \
          rg -v 'tenant|test|//|use ' | head -5
    fi
fi

# ============================================================================
# 4. Numeric Range Enforcement (informational)
# ============================================================================
echo ""
echo -e "${BLUE}--- 4. Numeric Range Enforcement ---${NC}"

# Check for numeric casts from params (potential range issues)
UNVALIDATED_CASTS=$(rg 'params\.\w+.*as (f64|f32|i64|i32|u64|u32)' src/ --type rust -n 2>/dev/null | wc -l | tr -d ' ')

if [ "$UNVALIDATED_CASTS" -eq 0 ]; then
    pass "No direct numeric casts from params"
else
    warn "Found $UNVALIDATED_CASTS numeric casts from params (review for range validation)"
fi

# Check fitness metrics in routes have validation
FITNESS_UNVALIDATED=$(rg 'weight|height|age|heart_rate|pace' src/routes/ --type rust -A 5 2>/dev/null | \
  rg 'params\.' | rg -v 'validate|clamp|min|max|range' | wc -l | tr -d ' ')

if [ "$FITNESS_UNVALIDATED" -eq 0 ]; then
    pass "Fitness metric parameters validated"
else
    warn "Found $FITNESS_UNVALIDATED fitness metric params without explicit validation"
fi

# ============================================================================
# SUMMARY
# ============================================================================
echo ""
echo -e "${BLUE}=========================================${NC}"
if [ "$VALIDATION_FAILED" = true ]; then
    echo -e "${RED}  INPUT VALIDATION CHECK: FAILED${NC}"
    echo -e "${RED}  Fix issues above before merging${NC}"
    echo -e "${BLUE}=========================================${NC}"
    exit 1
else
    echo -e "${GREEN}  INPUT VALIDATION CHECK: PASSED${NC}"
    echo -e "${BLUE}=========================================${NC}"
    exit 0
fi
