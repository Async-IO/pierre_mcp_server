#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: CI validation script to detect secret patterns that should never appear in logs or code
# ABOUTME: Prevents PII leakage, credential exposure, and GDPR/CCPA violations
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

# Pierre MCP Server - Secret Pattern Detection
# This script validates that sensitive data patterns are not present in source code or logs

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}==== Pierre MCP Server - Secret Pattern Detection ====${NC}"
echo ""

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Change to project root
cd "$PROJECT_ROOT"

VALIDATION_FAILED=false

# Function to report validation failure
fail_validation() {
    echo -e "${RED}❌ SECRET PATTERN DETECTED${NC}"
    echo -e "${RED}$1${NC}"
    VALIDATION_FAILED=true
}

# Function to report success
pass_validation() {
    echo -e "${GREEN}✅ $1${NC}"
}

echo -e "${BLUE}Scanning for secret patterns in src/ directory...${NC}"
echo ""

# ============================================================================
# CRITICAL PATTERNS: Authorization tokens and credentials
# ============================================================================

echo -e "${BLUE}[1/7] Checking for exposed authorization tokens...${NC}"
# Only match Bearer tokens with actual token values (20+ chars), not documentation
EXPOSED_TOKENS=$(rg -i "bearer\s+[A-Za-z0-9\.\-_]{20,}" src/ -g "!src/middleware/redaction.rs" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$EXPOSED_TOKENS" -eq 0 ]; then
    pass_validation "No authorization tokens found in source code"
else
    fail_validation "Found $EXPOSED_TOKENS authorization tokens in source code"
    echo -e "${YELLOW}Locations:${NC}"
    rg -i "bearer\s+[A-Za-z0-9\.\-_]{20,}" src/ -g "!src/middleware/redaction.rs" -g "!tests/*" -n | head -5
    echo ""
fi

# ============================================================================
# CRITICAL PATTERNS: API keys and secrets
# ============================================================================

echo -e "${BLUE}[2/7] Checking for hardcoded API keys...${NC}"
HARDCODED_KEYS=$(rg -i "api[_-]?key\s*[=:]\s*['\"][a-zA-Z0-9]{20,}['\"]|client[_-]?secret\s*[=:]\s*['\"][a-zA-Z0-9]{20,}['\"]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$HARDCODED_KEYS" -eq 0 ]; then
    pass_validation "No hardcoded API keys found in source code"
else
    fail_validation "Found $HARDCODED_KEYS hardcoded API keys in source code"
    echo -e "${YELLOW}Locations:${NC}"
    rg -i "api[_-]?key\s*[=:]\s*['\"][a-zA-Z0-9]{20,}['\"]|client[_-]?secret\s*[=:]\s*['\"][a-zA-Z0-9]{20,}['\"]" src/ -n | head -5
    echo ""
fi

# ============================================================================
# CRITICAL PATTERNS: Passwords in code
# ============================================================================

echo -e "${BLUE}[3/7] Checking for hardcoded passwords...${NC}"
HARDCODED_PASSWORDS=$(rg -i "password\s*[=:]\s*['\"][^'\"]{8,}['\"]" src/ -g "!tests/*" -g "!examples/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$HARDCODED_PASSWORDS" -eq 0 ]; then
    pass_validation "No hardcoded passwords found in production code"
else
    fail_validation "Found $HARDCODED_PASSWORDS hardcoded passwords in production code"
    echo -e "${YELLOW}Locations:${NC}"
    rg -i "password\s*[=:]\s*['\"][^'\"]{8,}['\"]" src/ -g "!tests/*" -g "!examples/*" -n | head -5
    echo ""
fi

# ============================================================================
# CRITICAL PATTERNS: JWT tokens
# ============================================================================

echo -e "${BLUE}[4/7] Checking for exposed JWT tokens...${NC}"
EXPOSED_JWTS=$(rg -i "eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+" src/ -g "!tests/*" -g "!src/middleware/redaction.rs" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$EXPOSED_JWTS" -eq 0 ]; then
    pass_validation "No JWT tokens found in production code"
else
    fail_validation "Found $EXPOSED_JWTS JWT tokens in production code"
    echo -e "${YELLOW}Locations:${NC}"
    rg -i "eyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+" src/ -g "!tests/*" -g "!src/middleware/redaction.rs" -n | head -5
    echo ""
fi

# ============================================================================
# WARNING PATTERNS: Private keys (RSA, SSH, etc.)
# ============================================================================

echo -e "${BLUE}[5/7] Checking for private keys...${NC}"
PRIVATE_KEYS=$(rg -i "-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$PRIVATE_KEYS" -eq 0 ]; then
    pass_validation "No private keys found in source code"
else
    fail_validation "Found $PRIVATE_KEYS private keys in source code"
    echo -e "${YELLOW}Locations:${NC}"
    rg -i "-----BEGIN (RSA |EC |DSA )?PRIVATE KEY-----" src/ -n | head -5
    echo ""
fi

# ============================================================================
# WARNING PATTERNS: Unredacted PII in logs
# ============================================================================

echo -e "${BLUE}[6/7] Checking for potential PII leakage patterns...${NC}"
# Check for logging statements that might leak PII without redaction
PII_LOGGING=$(rg "log::|tracing::|info!|debug!|warn!|error!" src/ | rg -i "email|password|token|secret|authorization|cookie|session" | rg -v "// Safe|redact|mask" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$PII_LOGGING" -eq 0 ]; then
    pass_validation "No obvious PII leakage patterns in logging statements"
else
    echo -e "${YELLOW}⚠️  Found $PII_LOGGING logging statements that may leak PII${NC}"
    echo -e "${YELLOW}Review these locations to ensure PII is properly redacted:${NC}"
    rg "log::|tracing::|info!|debug!|warn!|error!" src/ | rg -i "email|password|token|secret|authorization|cookie|session" | rg -v "// Safe|redact|mask" -n | head -10
    echo -e "${YELLOW}Note: This is a warning - verify that redaction is applied${NC}"
    echo ""
fi

# ============================================================================
# WARNING PATTERNS: Database connection strings with credentials
# ============================================================================

echo -e "${BLUE}[7/7] Checking for database connection strings with embedded credentials...${NC}"
DB_CREDENTIALS=$(rg -i "postgres://[^:]+:[^@]+@|mysql://[^:]+:[^@]+@|mongodb://[^:]+:[^@]+@" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
if [ "$DB_CREDENTIALS" -eq 0 ]; then
    pass_validation "No database connection strings with embedded credentials"
else
    fail_validation "Found $DB_CREDENTIALS database connection strings with embedded credentials"
    echo -e "${YELLOW}Locations:${NC}"
    rg -i "postgres://[^:]+:[^@]+@|mysql://[^:]+:[^@]+@|mongodb://[^:]+:[^@]+@" src/ -n | head -5
    echo ""
fi

# ============================================================================
# SUMMARY
# ============================================================================

echo ""
echo -e "${BLUE}==== Secret Pattern Detection Summary ====${NC}"

if [ "$VALIDATION_FAILED" = true ]; then
    echo -e "${RED}❌ VALIDATION FAILED${NC}"
    echo -e "${RED}Found sensitive data patterns that must be removed before deployment${NC}"
    echo ""
    echo -e "${YELLOW}Remediation steps:${NC}"
    echo -e "${YELLOW}1. Remove hardcoded secrets from source code${NC}"
    echo -e "${YELLOW}2. Use environment variables for sensitive configuration${NC}"
    echo -e "${YELLOW}3. Ensure PII redaction middleware is applied to all logging${NC}"
    echo -e "${YELLOW}4. Use the redaction utilities in src/middleware/redaction.rs${NC}"
    echo ""
    exit 1
else
    echo -e "${GREEN}✅ All secret pattern validations passed${NC}"
    echo -e "${GREEN}No sensitive data patterns detected in source code${NC}"
    exit 0
fi
