#!/usr/bin/env bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Validates that the codebase is ready for a release.
# ABOUTME: Checks version consistency, build, tests, and npm package integrity.

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

ERRORS=0
WARNINGS=0

print_check() {
    echo -e "${BLUE}[CHECK]${NC} $1"
}

print_pass() {
    echo -e "  ${GREEN}✓${NC} $1"
}

print_fail() {
    echo -e "  ${RED}✗${NC} $1"
    ((ERRORS++))
}

print_warn() {
    echo -e "  ${YELLOW}⚠${NC} $1"
    ((WARNINGS++))
}

print_info() {
    echo -e "  ${BLUE}ℹ${NC} $1"
}

# Change to repo root
cd "$(dirname "$0")/.."
REPO_ROOT=$(pwd)

echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Pierre Release Validation${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""

# 1. Version consistency check
print_check "Version consistency"

CARGO_VERSION=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
SDK_VERSION=$(grep '"version"' sdk/package.json | head -1 | sed 's/.*"version": "\(.*\)".*/\1/')

print_info "Cargo.toml version: ${CARGO_VERSION}"
print_info "sdk/package.json version: ${SDK_VERSION}"

if [ "$CARGO_VERSION" == "$SDK_VERSION" ]; then
    print_pass "Versions are consistent"
else
    print_fail "Version mismatch: Cargo.toml (${CARGO_VERSION}) != sdk/package.json (${SDK_VERSION})"
fi

# 2. Check if version already exists on npm
print_check "npm version availability"

PACKAGE_NAME=$(grep '"name"' sdk/package.json | head -1 | sed 's/.*"name": "\(.*\)".*/\1/')

if npm view "${PACKAGE_NAME}@${SDK_VERSION}" version 2>/dev/null; then
    print_fail "Version ${SDK_VERSION} already exists on npm"
else
    print_pass "Version ${SDK_VERSION} is available on npm"
fi

# 3. Check for uncommitted changes
print_check "Git status"

if git diff-index --quiet HEAD -- 2>/dev/null; then
    print_pass "No uncommitted changes"
else
    print_warn "Uncommitted changes detected"
    git status --short | head -10
fi

# 4. Check current branch
print_check "Git branch"

CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" == "main" ]; then
    print_pass "On main branch"
else
    print_warn "Not on main branch (current: ${CURRENT_BRANCH})"
fi

# 5. Check if tag exists
print_check "Git tag"

TAG_NAME="v${SDK_VERSION}"
if git rev-parse "$TAG_NAME" > /dev/null 2>&1; then
    print_warn "Tag ${TAG_NAME} already exists"
else
    print_pass "Tag ${TAG_NAME} does not exist"
fi

# 6. Rust build check
print_check "Rust build"

if cargo check --quiet 2>/dev/null; then
    print_pass "Rust compilation check passed"
else
    print_fail "Rust compilation failed"
fi

# 7. Rust clippy check
print_check "Rust clippy"

if cargo clippy --quiet -- -D warnings 2>/dev/null; then
    print_pass "Clippy check passed"
else
    print_fail "Clippy has warnings/errors"
fi

# 8. Rust tests
print_check "Rust tests (quick subset)"

if cargo test --lib --quiet -- --test-threads=4 2>/dev/null; then
    print_pass "Rust library tests passed"
else
    print_fail "Rust tests failed"
fi

# 9. SDK build check
print_check "SDK build"

if (cd sdk && npm run build --silent 2>/dev/null); then
    print_pass "SDK build passed"
else
    print_fail "SDK build failed"
fi

# 10. SDK type check
print_check "SDK type check"

if (cd sdk && npm run type-check --silent 2>/dev/null); then
    print_pass "SDK type check passed"
else
    print_fail "SDK type check failed"
fi

# 11. SDK lint
print_check "SDK lint"

if (cd sdk && npm run lint --silent 2>/dev/null); then
    print_pass "SDK lint passed"
else
    print_fail "SDK lint failed"
fi

# 12. SDK unit tests
print_check "SDK unit tests"

if (cd sdk && npm run test:unit --silent 2>/dev/null); then
    print_pass "SDK unit tests passed"
else
    print_fail "SDK unit tests failed"
fi

# 13. Package contents verification
print_check "SDK package contents"

if [ -f "sdk/dist/index.js" ] && [ -f "sdk/dist/cli.js" ]; then
    print_pass "Required dist files exist"
else
    print_fail "Missing dist files (run 'npm run build' in sdk/)"
fi

# 14. CHANGELOG check
print_check "CHANGELOG entry"

if [ -f "CHANGELOG.md" ]; then
    if grep -q "## \[${SDK_VERSION}\]" CHANGELOG.md 2>/dev/null; then
        print_pass "CHANGELOG entry found for ${SDK_VERSION}"
    else
        print_warn "No CHANGELOG entry for version ${SDK_VERSION}"
    fi
else
    print_warn "CHANGELOG.md not found"
fi

# Summary
echo ""
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo -e "${BLUE}  Summary${NC}"
echo -e "${BLUE}═══════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Version: ${GREEN}${SDK_VERSION}${NC}"
echo -e "  Errors:   ${ERRORS}"
echo -e "  Warnings: ${WARNINGS}"
echo ""

if [ $ERRORS -gt 0 ]; then
    echo -e "${RED}❌ Release validation FAILED${NC}"
    echo ""
    echo "Fix the errors above before releasing."
    exit 1
elif [ $WARNINGS -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Release validation passed with warnings${NC}"
    echo ""
    echo "Review warnings above. You may proceed if they are expected."
    exit 0
else
    echo -e "${GREEN}✅ Release validation PASSED${NC}"
    echo ""
    echo "Ready to release! Run:"
    echo "  ./scripts/release/prepare-release.sh ${SDK_VERSION}"
    exit 0
fi
