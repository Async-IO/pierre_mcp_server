#!/bin/bash
# ABOUTME: Automated security review for CI — validates authorization, tenant isolation, logging, and query safety
# ABOUTME: Companion script to .claude/skills/security-review/SKILL.md with machine-enforceable checks
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
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$PROJECT_ROOT"

VALIDATION_FAILED=false

pass() { echo -e "${GREEN}  ✅ $1${NC}"; }
warn() { echo -e "${YELLOW}  ⚠️  $1${NC}"; }
fail() { echo -e "${RED}  ❌ $1${NC}"; VALIDATION_FAILED=true; }

echo -e "${BLUE}=========================================${NC}"
echo -e "${BLUE}  SECURITY REVIEW (CI)${NC}"
echo -e "${BLUE}=========================================${NC}"

# ============================================================================
# 1. Authorization Boundaries (informational — hard to fully automate)
# ============================================================================
echo ""
echo -e "${BLUE}--- 1. Authorization Boundaries ---${NC}"

SUPER_ADMIN_CHECKS=$(rg "super.?admin|SuperAdmin" src/routes/ --type rust -l 2>/dev/null | wc -l | tr -d ' ')
if [ "$SUPER_ADMIN_CHECKS" -gt 0 ]; then
    SUPER_ADMIN_GATING=$(rg "is_super_admin" src/routes/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    if [ "$SUPER_ADMIN_GATING" -gt 0 ]; then
        pass "Super-admin gating found ($SUPER_ADMIN_GATING checks across routes)"
    else
        warn "Routes reference super-admin but no is_super_admin checks found"
    fi
else
    pass "No super-admin routes to check"
fi

# ============================================================================
# 2. Multi-Tenant Isolation (informational)
# ============================================================================
echo ""
echo -e "${BLUE}--- 2. Multi-Tenant Isolation ---${NC}"

# Count SQL queries and those with tenant_id
TOTAL_SQL=$(rg "sqlx::query" src/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
SQL_WITH_TENANT=$(rg "sqlx::query" src/ --type rust -A 10 2>/dev/null | rg "tenant_id" | wc -l | tr -d ' ')

if [ "$TOTAL_SQL" -gt 0 ]; then
    pass "SQL queries: $TOTAL_SQL total, $SQL_WITH_TENANT reference tenant_id"
else
    pass "No SQL queries found"
fi

# ============================================================================
# 3. Logging Hygiene (HARD FAIL)
# ============================================================================
echo ""
echo -e "${BLUE}--- 3. Logging Hygiene ---${NC}"

# Check for sensitive data in log statements at INFO+ level
# Strategy: match log macros that interpolate actual secret values as variables
# Pattern 1: Inline interpolation like {access_token} or {password}
# Pattern 2: Positional args like info!("...", access_token) — the secret as a trailing arg
# Excludes: src/bin/ (CLIs), IDs (_id suffix), failure/error descriptions
SECRETS_INLINE=$(rg '(info!|warn!|error!)\(.*\{(access_token|refresh_token|client_secret|api_key|password|secret_key)\}' src/ --type rust -g '!src/bin/*' -n 2>/dev/null | \
  rg -v 'redact|REDACT|mask|\*\*\*' | wc -l | tr -d ' ')
SECRETS_POSITIONAL=$(rg '(info!|warn!|error!)\(.*,\s*(access_token|refresh_token|client_secret|api_key(?!_id)|password|secret_key)\s*[,)]' src/ --type rust -g '!src/bin/*' -n 2>/dev/null | \
  rg -v 'redact|REDACT|mask|\*\*\*' | wc -l | tr -d ' ')
SECRETS_IN_LOGS=$((SECRETS_INLINE + SECRETS_POSITIONAL))

if [ "$SECRETS_IN_LOGS" -eq 0 ]; then
    pass "No secrets detected in INFO+ log statements"
else
    fail "Found $SECRETS_IN_LOGS potential secrets in log statements"
    rg '(info!|warn!|error!)\(.*\{(access_token|refresh_token|client_secret|api_key|password|secret_key)\}' src/ --type rust -g '!src/bin/*' -n 2>/dev/null | \
      rg -v 'redact|REDACT|mask|\*\*\*' | head -3
    rg '(info!|warn!|error!)\(.*,\s*(access_token|refresh_token|client_secret|api_key(?!_id)|password|secret_key)\s*[,)]' src/ --type rust -g '!src/bin/*' -n 2>/dev/null | \
      rg -v 'redact|REDACT|mask|\*\*\*' | head -3
fi

# ============================================================================
# 4. OAuth & Protocol (informational)
# ============================================================================
echo ""
echo -e "${BLUE}--- 4. OAuth & Protocol Compliance ---${NC}"

STATE_VALIDATION=$(rg "state.*param|validate.*state|verify.*state|state_matches" src/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
PKCE_REFS=$(rg "code_challenge|code_verifier" src/ --type rust --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

if [ "$STATE_VALIDATION" -gt 0 ]; then
    pass "OAuth state validation: $STATE_VALIDATION references"
else
    warn "No OAuth state validation patterns found"
fi

if [ "$PKCE_REFS" -gt 0 ]; then
    pass "PKCE enforcement: $PKCE_REFS references"
else
    warn "No PKCE references found"
fi

# ============================================================================
# 5. Template & Query Safety (HARD FAIL)
# ============================================================================
echo ""
echo -e "${BLUE}--- 5. Template & Query Safety ---${NC}"

# Check for format! used to build SQL queries (injection risk)
# Uses Perl-compatible regex for lookahead
# Excludes: dynamic query builders that use bind parameters (where_clause with ?N or $N)
FORMAT_SQL=$(rg 'format!\(.*(?:SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)' src/ --type rust -n 2>/dev/null | \
  rg -v 'test|//.*format|\$[0-9]|\?[0-9]|where_clause|bind_values|push_bind|param_index|placeholder' | \
  wc -l | tr -d ' ')

if [ "$FORMAT_SQL" -eq 0 ]; then
    pass "No format!() SQL injection risks"
else
    fail "Found $FORMAT_SQL format!() SQL construction patterns"
    rg 'format!\(.*(?:SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)' src/ --type rust -n 2>/dev/null | \
      rg -v 'test|//.*format|\$[0-9]|\?[0-9]|where_clause|bind_values|push_bind|param_index|placeholder' | head -5
fi

# Check for unescaped HTML interpolation
HTML_UNESCAPED=$(rg 'text/html|Content-Type.*html' src/ --type rust -B 5 -A 10 2>/dev/null | \
  rg 'format!' | rg -v 'html_escape|encode_text' | wc -l | tr -d ' ')

if [ "$HTML_UNESCAPED" -eq 0 ]; then
    pass "HTML output properly escaped"
else
    fail "Found $HTML_UNESCAPED unescaped HTML interpolation patterns"
    rg 'text/html|Content-Type.*html' src/ --type rust -B 5 -A 10 -n 2>/dev/null | \
      rg 'format!' | rg -v 'html_escape|encode_text' | head -5
fi

# ============================================================================
# 6. Static OAuth/Config State (HARD FAIL)
# ============================================================================
echo ""
echo -e "${BLUE}--- 6. Tenant Isolation in Non-DB Code ---${NC}"

# Check for global mutable OAuth credential storage that should be per-tenant
# Excludes: read-only app config (ServerConfig, RouteTimeoutConfig), comment lines, provider definitions
GLOBAL_OAUTH_STATE=$(rg 'static.*OAuth.*Mutex|static.*OAuth.*RwLock|LazyLock.*OAuth.*token|LazyLock.*OAuth.*credential' src/ --type rust -n 2>/dev/null | \
  rg -v 'test|//|DEFAULT' | wc -l | tr -d ' ')

if [ "$GLOBAL_OAUTH_STATE" -eq 0 ]; then
    pass "No global mutable OAuth credential storage"
else
    fail "Found $GLOBAL_OAUTH_STATE global OAuth credential storage patterns (should be per-tenant)"
    rg 'static.*OAuth.*Mutex|static.*OAuth.*RwLock|LazyLock.*OAuth.*token|LazyLock.*OAuth.*credential' src/ --type rust -n 2>/dev/null | \
      rg -v 'test|//|DEFAULT' | head -5
fi

# ============================================================================
# SUMMARY
# ============================================================================
echo ""
echo -e "${BLUE}=========================================${NC}"
if [ "$VALIDATION_FAILED" = true ]; then
    echo -e "${RED}  SECURITY REVIEW: FAILED${NC}"
    echo -e "${RED}  Fix issues above before merging${NC}"
    echo -e "${BLUE}=========================================${NC}"
    exit 1
else
    echo -e "${GREEN}  SECURITY REVIEW: PASSED${NC}"
    echo -e "${BLUE}=========================================${NC}"
    exit 0
fi
