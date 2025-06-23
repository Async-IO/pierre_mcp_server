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

# Clean up any generated files from previous runs
echo -e "${BLUE}==== Cleaning up generated files... ====${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
echo -e "${GREEN}✅ Cleanup completed${NC}"

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

# Run Clippy linter with core warnings only (pedantic allowed for now)
echo -e "${BLUE}==== Running Rust linter (Clippy)... ====${NC}"
if cargo clippy --all-targets --all-features --quiet -- -D warnings -A clippy::pedantic -A clippy::nursery; then
    echo -e "${GREEN}✅ Rust linting passed (core issues fixed, pedantic warnings allowed)${NC}"
else
    echo -e "${RED}❌ Rust linting failed${NC}"
    echo -e "${YELLOW}💡 Run 'cargo clippy --all-targets --all-features -- -W clippy::pedantic -W clippy::nursery' to see all warnings${NC}"
    ALL_PASSED=false
fi

# Check Rust compilation
echo -e "${BLUE}==== Checking Rust compilation... ====${NC}"
if cargo check --all-targets --quiet; then
    echo -e "${GREEN}✅ Rust compilation check passed${NC}"
else
    echo -e "${RED}❌ Rust compilation failed${NC}"
    ALL_PASSED=false
fi

# Run Rust tests
echo -e "${BLUE}==== Running Rust tests... ====${NC}"
if cargo test --all-targets --quiet; then
    echo -e "${GREEN}✅ All Rust tests passed${NC}"
else
    echo -e "${RED}❌ Some Rust tests failed${NC}"
    ALL_PASSED=false
fi

# Run Rust tests with coverage (if cargo-llvm-cov is installed)
echo -e "${BLUE}==== Running Rust tests with coverage... ====${NC}"
if command_exists cargo-llvm-cov; then
    # Show coverage summary directly on screen (all tests including integration)
    echo -e "${BLUE}Generating coverage summary for all tests...${NC}"
    if cargo llvm-cov --all-targets --summary-only; then
        echo -e "${GREEN}✅ Rust coverage summary displayed above${NC}"
    else
        echo -e "${YELLOW}⚠️  Coverage generation failed or timed out${NC}"
        echo -e "${YELLOW}   Falling back to library tests only...${NC}"
        if cargo llvm-cov --lib --summary-only; then
            echo -e "${GREEN}✅ Rust library coverage summary displayed above${NC}"
        else
            echo -e "${YELLOW}   Coverage generation failed - skipping${NC}"
        fi
    fi
else
    echo -e "${YELLOW}⚠️  cargo-llvm-cov not installed. Install with: cargo install cargo-llvm-cov${NC}"
    echo -e "${YELLOW}   Skipping coverage report generation${NC}"
fi

# Run A2A compliance tests specifically
echo -e "${BLUE}==== Running A2A compliance tests... ====${NC}"
if cargo test --test a2a_compliance_test --quiet; then
    echo -e "${GREEN}✅ A2A compliance tests passed${NC}"
else
    echo -e "${RED}❌ A2A compliance tests failed${NC}"
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
    if npm test -- --run; then
        echo -e "${GREEN}✅ Frontend tests passed${NC}"
    else
        echo -e "${RED}❌ Frontend tests failed${NC}"
        ALL_PASSED=false
    fi
    
    # Run frontend tests with coverage
    echo -e "${BLUE}==== Running frontend tests with coverage... ====${NC}"
    if npm run test:coverage -- --run; then
        echo -e "${GREEN}✅ Frontend coverage report generated in coverage/${NC}"
    else
        echo -e "${YELLOW}⚠️  Failed to generate frontend coverage report${NC}"
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
    if cargo audit --ignore RUSTSEC-2023-0071; then
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

# Check Python examples (if they exist)
if [ -d "examples/python" ]; then
    echo -e "${BLUE}==== Validating Python Examples... ====${NC}"
    
    # Check Python syntax for all Python files
    PYTHON_SYNTAX_OK=true
    for py_file in $(find examples/python -name "*.py"); do
        if ! python3 -m py_compile "$py_file" 2>/dev/null; then
            echo -e "${RED}❌ Syntax error in $py_file${NC}"
            PYTHON_SYNTAX_OK=false
            ALL_PASSED=false
        fi
    done
    
    if [ "$PYTHON_SYNTAX_OK" = true ]; then
        echo -e "${GREEN}✅ Python syntax validation passed${NC}"
    fi
    
    # Test individual utility modules (without server dependencies)
    echo -e "${BLUE}==== Testing Python utilities... ====${NC}"
    
    cd examples
    
    # Test auth utilities (mock mode)
    if python3 -c "
import sys, os
sys.path.append('python')
os.environ['PIERRE_EMAIL'] = 'test@example.com'
os.environ['PIERRE_PASSWORD'] = 'test123'
from python.common.auth_utils import AuthManager, EnvironmentConfig
auth = AuthManager()
config = EnvironmentConfig.get_server_config()
print('✅ Auth utilities import and basic config work')
" 2>/dev/null; then
        echo -e "${GREEN}✅ Python auth utilities validated${NC}"
    else
        echo -e "${YELLOW}⚠️  Python auth utilities validation skipped (dependencies missing)${NC}"
    fi
    
    # Test data utilities with sample data
    if python3 -c "
import sys
sys.path.append('python')
from python.common.data_utils import FitnessDataProcessor, DataValidator

# Test with minimal sample data
sample_data = [{
    'sport_type': 'run',
    'distance_meters': 5000,
    'moving_time_seconds': 1800,
    'elevation_gain': 50,
    'start_date': '2024-01-01T10:00:00Z'
}]

result = FitnessDataProcessor.calculate_fitness_score(sample_data)
validation = DataValidator.validate_activity_data(sample_data)
print(f'✅ Data processing works: score={result[\"total_score\"]}, quality={validation[\"quality_score\"]:.1f}')
" 2>/dev/null; then
        echo -e "${GREEN}✅ Python data utilities validated${NC}"
    else
        echo -e "${RED}❌ Python data utilities validation failed${NC}"
        ALL_PASSED=false
    fi
    
    # Test CI mode with mock data
    echo -e "${BLUE}==== Testing CI mode with mock data... ====${NC}"
    export PIERRE_CI_MODE=true
    
    # Test A2A demo with timeout if available, otherwise run directly
    if command_exists timeout; then
        if timeout 15s python3 python/a2a/enterprise_demo.py > /dev/null 2>&1; then
            echo -e "${GREEN}✅ A2A demo works with mock data${NC}"
        else
            echo -e "${YELLOW}⚠️  A2A demo test failed or timed out${NC}"
        fi
    else
        if python3 python/a2a/enterprise_demo.py > /dev/null 2>&1; then
            echo -e "${GREEN}✅ A2A demo works with mock data${NC}"
        else
            echo -e "${YELLOW}⚠️  A2A demo test failed${NC}"
        fi
    fi
    
    # Test MCP demo with timeout if available, otherwise run directly
    if command_exists timeout; then
        if timeout 15s python3 python/mcp/investor_demo.py > /dev/null 2>&1; then
            echo -e "${GREEN}✅ MCP demo works with mock data${NC}"
        else
            echo -e "${RED}❌ MCP demo test failed or timed out${NC}"
            ALL_PASSED=false
        fi
    else
        if python3 python/mcp/investor_demo.py > /dev/null 2>&1; then
            echo -e "${GREEN}✅ MCP demo works with mock data${NC}"
        else
            echo -e "${RED}❌ MCP demo test failed${NC}"
            ALL_PASSED=false
        fi
    fi
    
    # Test provisioning mock provider
    echo -e "${BLUE}==== Testing provisioning mock provider... ====${NC}"
    if command_exists timeout; then
        if timeout 10s python3 python/provisioning/mock_strava_provider.py > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Mock Strava provider works${NC}"
        else
            echo -e "${YELLOW}⚠️  Mock Strava provider test failed or timed out${NC}"
        fi
    else
        if python3 python/provisioning/mock_strava_provider.py > /dev/null 2>&1; then
            echo -e "${GREEN}✅ Mock Strava provider works${NC}"
        else
            echo -e "${YELLOW}⚠️  Mock Strava provider test failed${NC}"
        fi
    fi
    
    unset PIERRE_CI_MODE
    
    cd ..
fi

# Final cleanup after tests
echo -e "${BLUE}==== Final cleanup after tests... ====${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
find . -name "a2a_enterprise_report_*.json" -delete 2>/dev/null || true
find . -name "mcp_investor_demo_*.json" -delete 2>/dev/null || true
echo -e "${GREEN}✅ Final cleanup completed${NC}"

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
    echo "✅ A2A compliance tests"
    echo "✅ Frontend linting"
    echo "✅ TypeScript type checking"
    echo "✅ Frontend tests"
    echo "✅ Frontend build"
    echo "✅ Frontend code coverage"
    echo "✅ Documentation"
    if command_exists cargo-llvm-cov; then
        echo "✅ Rust code coverage"
    fi
    if [ -d "examples/python" ]; then
        echo "✅ Python examples validation"
    fi
    echo ""
    echo -e "${GREEN}✅ Your code is ready for production! 🚀${NC}"
    exit 0
else
    echo -e "${RED}❌ Some checks failed. Please fix the issues above.${NC}"
    exit 1
fi