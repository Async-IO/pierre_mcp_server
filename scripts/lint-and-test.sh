#!/bin/bash

# Pierre MCP Server - Comprehensive Lint and Test Runner
# This script runs all linting and testing for both Rust backend and TypeScript frontend

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

echo -e "${BLUE}==== Pierre MCP Server - Lint and Test Runner ====${NC}"
echo "Project root: $PROJECT_ROOT"

# Change to project root
cd "$PROJECT_ROOT"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Track overall success
ALL_PASSED=true

echo ""
echo -e "${BLUE}==== Rust Backend Checks ====${NC}"

# Check Rust formatting
echo -e "${BLUE}==== Checking Rust code formatting... ====${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}✅ Rust code formatting is correct${NC}"
else
    echo -e "${RED}❌ Rust code formatting issues found. Run 'cargo fmt --all' to fix.${NC}"
    ALL_PASSED=false
fi

# Run Clippy linter
echo -e "${BLUE}==== Running Rust linter (Clippy)... ====${NC}"
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo -e "${GREEN}✅ Rust linting passed${NC}"
else
    echo -e "${RED}❌ Rust linting failed${NC}"
    ALL_PASSED=false
fi

# Check Rust compilation
echo -e "${BLUE}==== Checking Rust compilation... ====${NC}"
if cargo check --all-targets; then
    echo -e "${GREEN}✅ Rust compilation check passed${NC}"
else
    echo -e "${RED}❌ Rust compilation failed${NC}"
    ALL_PASSED=false
fi

# Run Rust tests
echo -e "${BLUE}==== Running Rust tests... ====${NC}"
if cargo test --all-targets; then
    echo -e "${GREEN}✅ All Rust tests passed${NC}"
else
    echo -e "${RED}❌ Some Rust tests failed${NC}"
    ALL_PASSED=false
fi

# Frontend checks
if [ -d "frontend" ]; then
    echo ""
    echo -e "${BLUE}==== Frontend Checks ====${NC}"
    
    cd frontend
    
    # Run ESLint
    echo -e "${BLUE}==== Running frontend linter (ESLint)... ====${NC}"
    if npm run lint; then
        echo -e "${GREEN}✅ Frontend linting passed${NC}"
    else
        echo -e "${RED}❌ Frontend linting failed${NC}"
        ALL_PASSED=false
    fi
    
    # Run TypeScript type checking
    echo -e "${BLUE}==== Running TypeScript type checking... ====${NC}"
    if npm run type-check; then
        echo -e "${GREEN}✅ TypeScript type checking passed${NC}"
    else
        echo -e "${RED}❌ TypeScript type checking failed${NC}"
        ALL_PASSED=false
    fi
    
    # Run frontend tests
    echo -e "${BLUE}==== Running frontend tests... ====${NC}"
    if npm test; then
        echo -e "${GREEN}✅ Frontend tests passed${NC}"
    else
        echo -e "${RED}❌ Frontend tests failed${NC}"
        ALL_PASSED=false
    fi
    
    # Check frontend build
    echo -e "${BLUE}==== Checking frontend build... ====${NC}"
    if npm run build; then
        echo -e "${GREEN}✅ Frontend build successful${NC}"
    else
        echo -e "${RED}❌ Frontend build failed${NC}"
        ALL_PASSED=false
    fi
    
    cd ..
fi

# Additional checks
echo ""
echo -e "${BLUE}==== Additional Project Checks ====${NC}"

# Check for TODO/FIXME comments
echo -e "${BLUE}==== Checking for TODO/FIXME comments... ====${NC}"
TODO_COUNT=$(grep -r "TODO\|FIXME" --include="*.rs" src/ 2>/dev/null | wc -l | tr -d ' ')
if [ "$TODO_COUNT" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  Found ${TODO_COUNT} TODO/FIXME comments in Rust code${NC}"
    grep -r "TODO\|FIXME" --include="*.rs" src/ 2>/dev/null || true
else
    echo -e "${GREEN}✅ No TODO/FIXME comments found${NC}"
fi

# Check for security vulnerabilities (if cargo-audit is installed)
echo -e "${BLUE}==== Checking for security vulnerabilities... ====${NC}"
if command_exists cargo-audit; then
    if cargo audit; then
        echo -e "${GREEN}✅ No security vulnerabilities found${NC}"
    else
        echo -e "${YELLOW}⚠️  Security vulnerabilities detected${NC}"
        # Don't fail the build for vulnerabilities
    fi
else
    echo -e "${YELLOW}⚠️  cargo-audit not installed. Install with: cargo install cargo-audit${NC}"
fi

# Check documentation
echo -e "${BLUE}==== Checking documentation... ====${NC}"
if cargo doc --no-deps --quiet; then
    echo -e "${GREEN}✅ Documentation builds successfully${NC}"
else
    echo -e "${RED}❌ Documentation build failed${NC}"
    ALL_PASSED=false
fi

# Summary
echo ""
echo -e "${BLUE}==== Summary ====${NC}"
if [ "$ALL_PASSED" = true ]; then
    echo -e "${GREEN}✅ All checks passed! ✨${NC}"
    echo ""
    echo "✅ Rust formatting"
    echo "✅ Rust linting (Clippy)"
    echo "✅ Rust compilation"
    echo "✅ Rust tests"
    echo "✅ Frontend linting"
    echo "✅ TypeScript type checking"
    echo "✅ Frontend build"
    echo "✅ Documentation"
    echo ""
    echo -e "${GREEN}✅ Your code is ready for production! 🚀${NC}"
    exit 0
else
    echo -e "${RED}❌ Some checks failed. Please fix the issues above.${NC}"
    exit 1
fi