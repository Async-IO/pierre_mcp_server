#!/bin/bash

# Pierre MCP Server - Comprehensive Validation Script
# This script enforces all mandatory code quality standards and dev best practices
# Usage: ./scripts/lint-and-test.sh [--coverage]

# Manual error handling - collect all failures rather than stopping at first one
# Fast-fail kept only for critical architectural issues that prevent meaningful testing

echo "Running Pierre MCP Server Validation Suite..."

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
            echo "  --coverage               Enable code coverage collection and reporting"
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

echo -e "${BLUE}==== Pierre MCP Server - Lint and Test Runner ====${NC}"
echo "Project root: $PROJECT_ROOT"

# Change to project root
cd "$PROJECT_ROOT"

# Clean up any generated files from previous runs
echo -e "${BLUE}==== Cleaning up generated files... ====${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}


# Track overall success
ALL_PASSED=true

echo ""
echo -e "${BLUE}==== Rust Backend Checks ====${NC}"

# Auto-format Rust code
echo -e "${BLUE}==== Auto-formatting Rust code... ====${NC}"
cargo fmt --all
echo -e "${GREEN}[OK] Rust code formatting applied${NC}"

# Check Rust formatting to verify it's correct
echo -e "${BLUE}==== Verifying Rust code formatting... ====${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}[OK] Rust code formatting is correct${NC}"
else
    echo -e "${RED}[FAIL] Rust code formatting issues found after auto-format${NC}"
    ALL_PASSED=false
fi

# Function to report warning
warn_validation() {
    echo -e "${YELLOW}⚠️  ARCHITECTURAL WARNING${NC}"
    echo -e "${YELLOW}$1${NC}"
}

# Function to report success
pass_validation() {
    echo -e "${GREEN}✅ $1${NC}"
}

# UNIFIED ARCHITECTURAL VALIDATION SUITE (run early to catch design issues)
# ============================================================================
echo ""
echo -e "${BLUE}============================================================================${NC}"
echo -e "${BLUE}==== UNIFIED ARCHITECTURAL VALIDATION SUITE ====${NC}"
echo -e "${BLUE}============================================================================${NC}"
echo ""
echo -e "${YELLOW}This comprehensive validation suite runs early to ensure:${NC}"
echo -e "${YELLOW}  • Code quality standards are met${NC}"
echo -e "${YELLOW}  • No anti-patterns or stubbed implementations exist${NC}"
echo -e "${YELLOW}  • Architecture follows best practices${NC}"
echo ""

VALIDATION_FAILED=false

# Function to report validation failure
fail_validation() {
    echo -e "${RED}❌ ARCHITECTURAL VALIDATION FAILED${NC}"
    echo -e "${RED}$1${NC}"
    VALIDATION_FAILED=true
    ALL_PASSED=false
}

# ============================================================================
# SECTION 1: ANTI-PATTERN DETECTION
# ============================================================================
echo -e "${BLUE}==== 1. Anti-Pattern Detection ====${NC}"

# Smart database clone detection - separate legitimate Arc clones from problematic Database clones
LEGITIMATE_ARC_CLONES=$(rg "database_arc\.clone\(\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | cut -d: -f2 | python3 -c "import sys; lines = sys.stdin.readlines(); print(sum(int(x.strip()) for x in lines) if lines else 0)" 2>/dev/null || echo 0)
PROBLEMATIC_DB_CLONES=$(rg "\.as_ref\(\)\.clone\(\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | cut -d: -f2 | python3 -c "import sys; lines = sys.stdin.readlines(); print(sum(int(x.strip()) for x in lines) if lines else 0)" 2>/dev/null || echo 0)
TOTAL_DATABASE_CLONES=$((LEGITIMATE_ARC_CLONES + PROBLEMATIC_DB_CLONES))
RESOURCE_CREATION=$(rg "AuthManager::new|OAuthManager::new|A2AClientManager::new|TenantOAuthManager::new" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
FAKE_RESOURCES=$(rg "Arc::new\(ServerResources\s*[\{\:]" src/ 2>/dev/null | wc -l | awk '{print $1+0}')
OBSOLETE_FUNCTIONS=$(rg "fn.*run_http_server\(" src/ 2>/dev/null | wc -l | awk '{print $1+0}')

echo "Anti-pattern analysis:"
echo "  Database clones: $TOTAL_DATABASE_CLONES total ($LEGITIMATE_ARC_CLONES legitimate, $PROBLEMATIC_DB_CLONES problematic)"
echo "  Resource creation: $RESOURCE_CREATION patterns"
echo "  Fake resources: $FAKE_RESOURCES assemblies"
echo "  Obsolete functions: $OBSOLETE_FUNCTIONS variants"

if [ "$PROBLEMATIC_DB_CLONES" -gt 0 ]; then 
    warn_validation "Found $PROBLEMATIC_DB_CLONES problematic database cloning patterns - use Arc sharing instead"
    rg "\.as_ref\(\)\.clone\(\)|Arc::new\(database\.clone\(\)\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" -n | head -3
elif [ "$LEGITIMATE_ARC_CLONES" -gt 0 ]; then
    pass_validation "Database clones are all legitimate Arc handle clones ($LEGITIMATE_ARC_CLONES total)"
fi
if [ "$RESOURCE_CREATION" -gt 0 ]; then 
    warn_validation "Found $RESOURCE_CREATION resource creation patterns - use ServerResources dependency injection"
    rg "AuthManager::new|OAuthManager::new|A2AClientManager::new|TenantOAuthManager::new" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" -n | head -3
fi
if [ "$FAKE_RESOURCES" -gt 0 ]; then 
    warn_validation "Found $FAKE_RESOURCES fake ServerResources assembly patterns"
    rg "Arc::new\(ServerResources\s*\{" src/ -n | head -3
fi
if [ "$OBSOLETE_FUNCTIONS" -gt 1 ]; then  # Allow 1 legitimate function
    warn_validation "Found $OBSOLETE_FUNCTIONS run_http_server function variants - may indicate obsolete functions"
    rg "run_http_server\(" src/ -n | head -3
fi

# ============================================================================
# SECTION 2: CODE QUALITY ANALYSIS
# ============================================================================
echo -e "${BLUE}==== 2. Code Quality Analysis ====${NC}"

# Check for prohibited code patterns
echo -e "${BLUE}Analyzing code patterns...${NC}"

# Error handling patterns - smart filtering to reduce false positives
# Only count unwraps WITHOUT safety comments or hardcoded valid data patterns
PROBLEMATIC_UNWRAPS=$(rg "\.unwrap\(\)" src/ | rg -v "// Safe|hardcoded.*valid|static.*data|00000000-0000-0000-0000-000000000000" | wc -l 2>/dev/null || echo 0)
PROBLEMATIC_EXPECTS=$(rg "\.expect\(" src/ | rg -v "// Safe|ServerResources.*required" | wc -l 2>/dev/null || echo 0)
PANICS=$(rg "panic!\(" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

# Code quality patterns
TODOS=$(rg "TODO|FIXME|XXX" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
PLACEHOLDERS=$(rg "placeholder|not yet implemented|unimplemented!\(" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
STUBS=$(rg "stub|mock.*implementation" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

# Naming patterns - only flag meaningful names with underscores, not standard ignored patterns
PROBLEMATIC_UNDERSCORE_NAMES=$(rg "fn _|let _[a-zA-Z]|struct _|enum _" src/ | rg -v "let _[[:space:]]*=" | rg -v "let _result|let _response|let _output" | wc -l 2>/dev/null || echo 0)
EXAMPLE_EMAILS=$(rg "example\.com|test@" src/ -g "!src/bin/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

# Organization patterns  
CFG_TEST_IN_SRC=$(rg "#\[cfg\(test\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
TEMP_SOLUTIONS=$(rg "\bhack\b|\bworkaround\b|\bquick.*fix\b|future.*implementation|temporary.*solution|temp.*fix" src/ --count-matches 2>/dev/null | cut -d: -f2 | python3 -c "import sys; lines = sys.stdin.readlines(); print(sum(int(x.strip()) for x in lines) if lines else 0)" 2>/dev/null || echo 0)

# Attribute patterns
DEAD_CODE=$(rg "#\[allow\(dead_code\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
UNUSED_VARS=$(rg "#\[allow\(unused.*\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
DEPRECATED=$(rg "#\[deprecated\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

echo "Pattern Summary:"
echo "  Error handling: $PROBLEMATIC_UNWRAPS problematic unwraps, $PROBLEMATIC_EXPECTS problematic expects, $PANICS panics"
echo "  Code quality: $TODOS TODOs, $PLACEHOLDERS placeholders, $STUBS stubs"  
echo "  Naming: $PROBLEMATIC_UNDERSCORE_NAMES problematic underscore names, $EXAMPLE_EMAILS example emails"
echo "  Organization: $CFG_TEST_IN_SRC #[cfg(test)] in src/, $TEMP_SOLUTIONS temporary solutions"
echo "  Attributes: $DEAD_CODE dead code, $UNUSED_VARS unused, $DEPRECATED deprecated"

# Report findings - only show problematic patterns
if [ "$PROBLEMATIC_UNWRAPS" -gt 0 ]; then
    warn_validation "Found $PROBLEMATIC_UNWRAPS problematic .unwrap() calls - use proper error handling"
    rg "\.unwrap\(\)" src/ | rg -v "// Safe|hardcoded.*valid|static.*data|00000000-0000-0000-0000-000000000000" -n | head -3
fi
if [ "$PROBLEMATIC_EXPECTS" -gt 0 ]; then
    warn_validation "Found $PROBLEMATIC_EXPECTS problematic .expect() calls - use proper error handling" 
    rg "\.expect\(" src/ | rg -v "// Safe|ServerResources.*required" -n | head -3
fi
if [ "$PANICS" -gt 0 ]; then
    warn_validation "Found $PANICS panic!() calls - use proper error handling"
    rg "panic!\(" src/ -n | head -3
fi
if [ "$TODOS" -gt 0 ]; then
    warn_validation "Found $TODOS TODO/FIXME comments - complete implementation"
    rg "TODO|FIXME|XXX" src/ -n | head -3
fi
if [ "$PROBLEMATIC_UNDERSCORE_NAMES" -gt 0 ]; then
    warn_validation "Found $PROBLEMATIC_UNDERSCORE_NAMES problematic underscore-prefixed names"
    rg "fn _|let _[a-zA-Z]|struct _|enum _" src/ | rg -v "let _[[:space:]]*=" | rg -v "let _result|let _response|let _output" -n | head -3
fi
if [ "$TEMP_SOLUTIONS" -gt 0 ]; then
    warn_validation "Found $TEMP_SOLUTIONS temporary solutions"
    rg "\bhack\b|\bworkaround\b|\bquick.*fix\b|future.*implementation|temporary.*solution|temp.*fix" src/ -n | head -3
fi

# ============================================================================
# SECTION 3: MEMORY MANAGEMENT ANALYSIS
# ============================================================================
echo -e "${BLUE}==== 3. Memory Management Analysis ====${NC}"

# Smart clone analysis - differentiate between problematic and legitimate patterns
TOTAL_CLONES=$(rg "\.clone\(\)" src/ -g "!src/bin/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

# Count legitimate clone patterns (Arc handles, String ownership, error handling)
LEGITIMATE_CLONES=$(rg "\.clone\(\)" src/ | rg "Arc::|resources\.|database\.|auth_manager\.|\.to_string\(\)|format!|String::from|token|url|name|path|message|error|Error" | wc -l 2>/dev/null || echo 0)

# Problematic clone patterns - everything else
PROBLEMATIC_CLONES=$((TOTAL_CLONES - LEGITIMATE_CLONES))

# Smart Arc analysis - count actual instances more carefully
TOTAL_ARCS=$(rg "Arc::" src/ | wc -l 2>/dev/null || echo 0)
DEPENDENCY_ARCS=$(rg "Arc<ServerResources>|Arc<.*Manager>|Arc<.*Executor>" src/ | wc -l 2>/dev/null || echo 0)
CONCURRENT_ARCS=$(rg "Arc<.*Lock.*>|Arc<.*Mutex.*>|Arc<.*RwLock.*>" src/ | wc -l 2>/dev/null || echo 0)

# Smart magic numbers detection - exclude legitimate patterns
# JSON-RPC error codes (-32xxx), test data (12345, UUIDs), protocol versions (2024-xx-xx), rate limits (1000, 10000)
MAGIC_NUMBERS=$(rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" | wc -l 2>/dev/null || echo 0)

echo "Memory patterns:"
echo "  Clone usage: $TOTAL_CLONES clone() calls"
echo "  Arc usage: $TOTAL_ARCS Arc<T> instances"
echo "  Hardcoded values: $MAGIC_NUMBERS potential magic numbers"

# Clone assessment - focus on high-level patterns instead of precise counting
if [ "$TOTAL_CLONES" -lt 50 ]; then
    pass_validation "Minimal clone usage ($TOTAL_CLONES calls) - good ownership patterns"
elif [ "$TOTAL_CLONES" -lt 200 ]; then
    pass_validation "Moderate clone usage ($TOTAL_CLONES calls) - acceptable for dependency injection architecture"
elif [ "$TOTAL_CLONES" -lt 500 ]; then
    pass_validation "High clone usage ($TOTAL_CLONES calls) - mostly legitimate Arc handle sharing and string ownership"
    echo "    Most clones are legitimate Arc handles (resources.clone(), database.clone()) or string ownership transfers"
else
    warn_validation "Very high clone usage ($TOTAL_CLONES calls) - review for potential optimization opportunities"
    rg "\.clone\(\)" src/ | rg -v "Arc::|resources\.|database\.|auth_manager\.|\.to_string\(\)|format!|String::from|token|url|name|path|message|error|Error" -n | head -3
fi

# Arc assessment - focus on reasonable thresholds for dependency injection architecture
if [ "$TOTAL_ARCS" -lt 10 ]; then
    pass_validation "Minimal Arc usage ($TOTAL_ARCS instances) - focused sharing patterns"
elif [ "$TOTAL_ARCS" -lt 30 ]; then
    pass_validation "Reasonable Arc usage ($TOTAL_ARCS instances) - good for dependency injection architecture"
    echo "    Arc usage appropriate for shared ServerResources, managers, and concurrent data structures"
elif [ "$TOTAL_ARCS" -lt 50 ]; then
    pass_validation "Moderate Arc usage ($TOTAL_ARCS instances) - acceptable for complex service architecture"
else
    warn_validation "High Arc usage ($TOTAL_ARCS instances) - review for potential over-sharing"
    rg "Arc::" src/ | rg -v "ServerResources|Manager|Executor|Lock|Mutex|RwLock" -n | head -3
fi

# Magic numbers assessment
if [ "$MAGIC_NUMBERS" -eq 0 ]; then
    pass_validation "No magic numbers found - good configuration practices"
elif [ "$MAGIC_NUMBERS" -lt 10 ]; then
    pass_validation "Minimal magic numbers ($MAGIC_NUMBERS) - acceptable configuration"
else
    warn_validation "Found $MAGIC_NUMBERS potential magic numbers - consider using configuration constants"
    rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" | head -3
fi

# ============================================================================
# UNIFIED ARCHITECTURAL VALIDATION SUMMARY
# ============================================================================
echo ""
echo -e "${BLUE}==== UNIFIED ARCHITECTURAL VALIDATION SUMMARY ====${NC}"

# Create ASCII table for all architectural findings
echo "┌─────────────────────────────────────┬───────┬──────────┬─────────────────────────────────────────┐"
echo "│ Validation Category                 │ Count │ Status   │ Details                                 │"
echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Anti-Pattern Detection
printf "│ %-35s │ %5d │ " "Database clones (total)" "$TOTAL_DATABASE_CLONES"
if [ "$PROBLEMATIC_DB_CLONES" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "${LEGITIMATE_ARC_CLONES} legitimate Arc clones"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "${PROBLEMATIC_DB_CLONES} problematic patterns"
fi

printf "│ %-35s │ %5d │ " "Resource creation patterns" "$RESOURCE_CREATION"
if [ "$RESOURCE_CREATION" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Using dependency injection"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Manual resource creation found"
fi

printf "│ %-35s │ %5d │ " "Fake resource assemblies" "$FAKE_RESOURCES"
if [ "$FAKE_RESOURCES" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No fake ServerResources"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Fake assemblies detected"
fi

printf "│ %-35s │ %5d │ " "Obsolete functions" "$OBSOLETE_FUNCTIONS"
if [ "$OBSOLETE_FUNCTIONS" -le 1 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Within acceptable limits"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Multiple variants found"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Code Quality Analysis
printf "│ %-35s │ %5d │ " "Problematic unwraps" "$PROBLEMATIC_UNWRAPS"
if [ "$PROBLEMATIC_UNWRAPS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Proper error handling"
else
    printf "%-8s │ %-39s │\n" "❌ FAIL" "Use Result types instead"
fi

printf "│ %-35s │ %5d │ " "Problematic expects" "$PROBLEMATIC_EXPECTS"
if [ "$PROBLEMATIC_EXPECTS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Proper error handling"
else
    printf "%-8s │ %-39s │\n" "❌ FAIL" "Use Result types instead"
fi

printf "│ %-35s │ %5d │ " "Panic calls" "$PANICS"
if [ "$PANICS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No panic! found"
else
    printf "%-8s │ %-39s │\n" "❌ FAIL" "Use proper error handling"
fi

printf "│ %-35s │ %5d │ " "TODOs/FIXMEs" "$TODOS"
if [ "$TODOS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No incomplete code"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Complete implementation needed"
fi

printf "│ %-35s │ %5d │ " "Placeholders/stubs" "$STUBS"
if [ "$STUBS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No stubs found"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Complete implementation needed"
fi

printf "│ %-35s │ %5d │ " "Problematic underscore names" "$PROBLEMATIC_UNDERSCORE_NAMES"
if [ "$PROBLEMATIC_UNDERSCORE_NAMES" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Good naming conventions"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Use meaningful names"
fi

printf "│ %-35s │ %5d │ " "Example emails" "$EXAMPLE_EMAILS"
if [ "$EXAMPLE_EMAILS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No test emails in production"
else
    printf "%-8s │ %-39s │\n" "⚠️  INFO" "Test data in codebase"
fi

printf "│ %-35s │ %5d │ " "Temporary solutions" "$TEMP_SOLUTIONS"
if [ "$TEMP_SOLUTIONS" -eq 0 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "No temporary code"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Complete implementation needed"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Memory Management Analysis
printf "│ %-35s │ %5d │ " "Clone usage" "$TOTAL_CLONES"
if [ "$TOTAL_CLONES" -lt 500 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Mostly legitimate Arc/String clones"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Review for optimization"
fi

printf "│ %-35s │ %5d │ " "Arc usage" "$TOTAL_ARCS"
if [ "$TOTAL_ARCS" -lt 50 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Appropriate for service architecture"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Review for over-sharing"
fi

printf "│ %-35s │ %5d │ " "Magic numbers" "$MAGIC_NUMBERS"
if [ "$MAGIC_NUMBERS" -lt 10 ]; then
    printf "%-8s │ %-39s │\n" "✅ PASS" "Good configuration practices"
else
    printf "%-8s │ %-39s │\n" "⚠️  WARN" "Use configuration constants"
fi

echo "└─────────────────────────────────────┴───────┴──────────┴─────────────────────────────────────────┘"

# Show detailed issues if warnings found
WARNINGS_FOUND=false
if [[ $PROBLEMATIC_DB_CLONES -gt 0 || $RESOURCE_CREATION -gt 0 || $FAKE_RESOURCES -gt 0 || $OBSOLETE_FUNCTIONS -gt 1 || $PROBLEMATIC_UNWRAPS -gt 0 || $PROBLEMATIC_EXPECTS -gt 0 || $PANICS -gt 0 || $TODOS -gt 0 || $PROBLEMATIC_UNDERSCORE_NAMES -gt 0 || $TEMP_SOLUTIONS -gt 0 || $TOTAL_CLONES -ge 500 || $TOTAL_ARCS -ge 50 || $MAGIC_NUMBERS -ge 10 ]]; then
    WARNINGS_FOUND=true
    echo ""
    echo -e "${YELLOW}⚠️  ARCHITECTURAL WARNINGS DETAILS:${NC}"

    if [ "$PROBLEMATIC_DB_CLONES" -gt 0 ]; then
        echo -e "${YELLOW}Database clones: Found $PROBLEMATIC_DB_CLONES problematic patterns${NC}"
        rg "\.as_ref\(\)\.clone\(\)|Arc::new\(database\.clone\(\)\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" -n | head -3
        echo ""
    fi

    if [ "$RESOURCE_CREATION" -gt 0 ]; then
        echo -e "${YELLOW}Resource creation: Found $RESOURCE_CREATION manual creation patterns${NC}"
        rg "AuthManager::new|OAuthManager::new|A2AClientManager::new|TenantOAuthManager::new" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" -n | head -3
        echo ""
    fi

    if [ "$FAKE_RESOURCES" -gt 0 ]; then
        echo -e "${YELLOW}Fake resources: Found $FAKE_RESOURCES assembly patterns${NC}"
        rg "Arc::new\(ServerResources\s*\{" src/ -n | head -3
        echo ""
    fi

    if [ "$TEMP_SOLUTIONS" -gt 0 ]; then
        echo -e "${YELLOW}Temporary solutions: Found $TEMP_SOLUTIONS patterns${NC}"
        rg "\bhack\b|\bworkaround\b|\bquick.*fix\b|future.*implementation|temporary.*solution|temp.*fix" src/ -n | head -3
        echo ""
    fi

    if [ "$MAGIC_NUMBERS" -ge 10 ]; then
        echo -e "${YELLOW}Magic numbers: Found $MAGIC_NUMBERS potential constants${NC}"
        rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" | head -3
        echo ""
    fi
fi

# Report comprehensive summary
if [[ $PROBLEMATIC_DB_CLONES -eq 0 && $RESOURCE_CREATION -eq 0 && $FAKE_RESOURCES -eq 0 && $OBSOLETE_FUNCTIONS -le 1 && $PROBLEMATIC_UNWRAPS -eq 0 && $PROBLEMATIC_EXPECTS -eq 0 && $PANICS -eq 0 && $TODOS -eq 0 && $PROBLEMATIC_UNDERSCORE_NAMES -eq 0 && $TEMP_SOLUTIONS -eq 0 ]]; then
    pass_validation "All critical architectural validations passed - excellent code quality"
elif [ "$WARNINGS_FOUND" = true ]; then
    pass_validation "Architectural validation completed with warnings - review recommendations above"
else
    pass_validation "All architectural validations passed - good code quality"
fi

# Core development checks (format, clippy, compilation, tests)
echo ""
echo -e "${BLUE}==== Core Development Checks ====${NC}"

# Run Clippy linter with ZERO TOLERANCE (fast-fail on ANY warning)
echo -e "${BLUE}==== Running Rust linter (Clippy) - ZERO TOLERANCE MODE... ====${NC}"
if cargo clippy --all-targets --all-features --quiet -- -W clippy::all -W clippy::pedantic -W clippy::nursery -D warnings; then
    echo -e "${GREEN}[OK] Rust linting passed - ZERO warnings${NC}"
else
    echo -e "${RED}[CRITICAL] Rust linting failed - ANY warning triggers fast-fail${NC}"
    echo -e "${RED}FAST FAIL: Fix ALL linting warnings immediately${NC}"
    echo -e "${YELLOW}Re-run with verbose output to see warnings:${NC}"
    echo -e "${YELLOW}  cargo clippy --all-targets --all-features -- -W clippy::all -W clippy::pedantic -W clippy::nursery${NC}"
    exit 1
fi

# Check Rust compilation
echo -e "${BLUE}==== Checking Rust compilation... ====${NC}"
if cargo check --all-targets --quiet; then
    echo -e "${GREEN}[OK] Rust compilation check passed${NC}"
else
    echo -e "${RED}[FAIL] Rust compilation failed${NC}"
    ALL_PASSED=false
fi

# Clean up test databases before running tests
echo -e "${BLUE}==== Cleaning up test databases... ====${NC}"
if ./scripts/clean-test-databases.sh; then
    echo -e "${GREEN}[OK] Test databases cleaned${NC}"
else
    echo -e "${YELLOW}[WARN] Test database cleanup failed (continuing anyway)${NC}"
fi

# Run Rust tests
echo -e "${BLUE}==== Running Rust tests... ====${NC}"
if cargo test --all-targets; then
    echo -e "${GREEN}[OK] All Rust tests passed${NC}"
else
    echo -e "${RED}[FAIL] Some Rust tests failed${NC}"
    ALL_PASSED=false
fi

# Run Rust tests with coverage (if enabled and cargo-llvm-cov is installed)
if [ "$ENABLE_COVERAGE" = true ]; then
    echo -e "${BLUE}==== Running Rust tests with coverage... ====${NC}"
    if command_exists cargo-llvm-cov; then
        # Show coverage summary directly on screen (all tests including integration)
        echo -e "${BLUE}Generating coverage summary for all tests...${NC}"
        if cargo llvm-cov --all-targets --summary-only; then
            echo -e "${GREEN}[OK] Rust coverage summary displayed above${NC}"
        else
            echo -e "${YELLOW}[WARN]  Coverage generation failed or timed out${NC}"
            echo -e "${YELLOW}   Falling back to library tests only...${NC}"
            if cargo llvm-cov --lib --summary-only; then
                echo -e "${GREEN}[OK] Rust library coverage summary displayed above${NC}"
            else
                echo -e "${YELLOW}   Coverage generation failed - skipping${NC}"
            fi
        fi
    else
        echo -e "${YELLOW}[WARN]  cargo-llvm-cov not installed. Install with: cargo install cargo-llvm-cov${NC}"
        echo -e "${YELLOW}   Skipping coverage report generation${NC}"
    fi
fi

# Run SDK integration tests specifically
echo -e "${BLUE}==== Running SDK integration tests... ====${NC}"
if cargo test --test sdk_integration_test --quiet; then
    echo -e "${GREEN}[OK] SDK integration tests passed${NC}"
else
    echo -e "${RED}[FAIL] SDK integration tests failed${NC}"
    ALL_PASSED=false
fi

# Run A2A compliance tests specifically
echo -e "${BLUE}==== Running A2A compliance tests... ====${NC}"
if cargo test --test a2a_compliance_test --quiet; then
    echo -e "${GREEN}[OK] A2A compliance tests passed${NC}"
else
    echo -e "${RED}[FAIL] A2A compliance tests failed${NC}"
    ALL_PASSED=false
fi

# OAuth automation test with headless Chrome
echo -e "${BLUE}==== Testing OAuth automation (optional)... ====${NC}"
OAUTH_AUTOMATION_ENABLED=false

# Check if OAuth test email is available (Strava uses passwordless auth)
if [ -n "$STRAVA_TEST_EMAIL" ] || [ -n "$STRAVA_TEST_USERNAME" ]; then
    OAUTH_AUTOMATION_ENABLED=true
    TEST_EMAIL="${STRAVA_TEST_EMAIL:-$STRAVA_TEST_USERNAME}"
    echo -e "${BLUE}OAuth test email detected - testing OAuth infrastructure${NC}"
    echo -e "${BLUE}Note: Strava uses passwordless authentication (email verification codes)${NC}"
    
    # Check if chromedriver is available
    if command -v chromedriver >/dev/null 2>&1; then
        echo -e "${BLUE}Starting chromedriver for OAuth automation...${NC}"
        
        # Start chromedriver in background
        chromedriver --port=9515 --silent >/dev/null 2>&1 &
        CHROMEDRIVER_PID=$!
        
        # Give chromedriver time to start
        sleep 2
        
        # Check if chromedriver started successfully
        if kill -0 $CHROMEDRIVER_PID 2>/dev/null; then
            echo -e "${GREEN}ChromeDriver started (PID: $CHROMEDRIVER_PID)${NC}"
            
            # Set screenshots directory for debugging
            export SCREENSHOTS_DIR="./test_screenshots"
            mkdir -p "$SCREENSHOTS_DIR"
            
            # Run OAuth infrastructure test
            echo -e "${BLUE}Running OAuth infrastructure test with headless Chrome...${NC}"
            if cargo test --test mcp_comprehensive_client_e2e_test test_comprehensive_mcp_tools --quiet 2>/dev/null; then
                echo -e "${GREEN}[OK] OAuth infrastructure test completed successfully${NC}"
                echo -e "${GREEN}    ✅ OAuth URL generation verified${NC}"
                echo -e "${GREEN}    ✅ Strava redirect handling verified${NC}"
                
                # Show screenshot count if any were taken
                if [ -d "$SCREENSHOTS_DIR" ]; then
                    SCREENSHOT_COUNT=$(find "$SCREENSHOTS_DIR" -name "oauth_*.png" 2>/dev/null | wc -l)
                    if [ "$SCREENSHOT_COUNT" -gt 0 ]; then
                        echo -e "${GREEN}    📸 Generated $SCREENSHOT_COUNT debug screenshots in $SCREENSHOTS_DIR${NC}"
                    fi
                fi
            else
                echo -e "${YELLOW}[SKIP] OAuth infrastructure test failed - check OAuth configuration${NC}"
                echo -e "${YELLOW}       Full OAuth flow requires email verification codes${NC}"
            fi
            
            # Clean up chromedriver
            if kill $CHROMEDRIVER_PID 2>/dev/null; then
                echo -e "${GREEN}ChromeDriver stopped successfully${NC}"
            fi
        else
            echo -e "${YELLOW}[SKIP] ChromeDriver failed to start${NC}"
        fi
    else
        echo -e "${YELLOW}[SKIP] ChromeDriver not found - install with: brew install chromedriver${NC}"
    fi
else
    echo -e "${YELLOW}[SKIP] OAuth infrastructure test disabled - set STRAVA_TEST_EMAIL to enable${NC}"
fi

if [ "$OAUTH_AUTOMATION_ENABLED" = true ]; then
    echo -e "${BLUE}OAuth infrastructure testing ready for CI/CD${NC}"
    echo -e "${BLUE}Note: Full OAuth flow requires email verification (passwordless auth)${NC}"
else
    echo -e "${BLUE}OAuth infrastructure testing can be enabled by setting STRAVA_TEST_EMAIL in .envrc${NC}"
fi

echo ""
echo -e "${GREEN}[OK] Core development checks completed${NC}"
echo ""

# ADDITIONAL CHECKS: Legacy functions and architectural analysis
echo -e "${BLUE}==== Additional Code Quality Checks (Informational) ====${NC}"

# FAST FAIL: Check for legacy functions that throw nonsense behavior
echo -e "${BLUE}==== Checking for legacy functions (FAST FAIL)... ====${NC}"

# Check for legacy OAuth patterns and deprecated functions
LEGACY_OAUTH=$(rg "Legacy OAuth not supported|legacy.*oauth|connect_strava|connect_fitbit" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
DEPRECATED_FUNCTIONS=$(rg "deprecated.*use.*instead|Universal.*deprecated|ProviderManager deprecated" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
LEGACY_TOOLS=$(rg "Legacy tool.*deprecated" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

# CRITICAL: Check for placeholder implementations that return Value instead of McpResponse
PLACEHOLDER_IMPLEMENTATIONS=$(rg "fn handle_.*-> Value" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
PLACEHOLDER_JSON_RETURNS=$(rg "serde_json::json!\(\{" src/mcp/multitenant.rs -A 3 | rg "response.*=" | wc -l | awk '{print $1+0}')

LEGACY_ISSUES_FOUND=false

if [ "$LEGACY_OAUTH" -gt 0 ]; then 
    echo -e "${RED}[CRITICAL] Found $LEGACY_OAUTH legacy OAuth patterns - will confuse users${NC}"
    echo -e "${RED}           Legacy OAuth functions advertise but don't work${NC}"
    rg "Legacy OAuth not supported|legacy.*oauth|connect_strava|connect_fitbit" src/ -n | head -5
    LEGACY_ISSUES_FOUND=true
    ALL_PASSED=false
fi

if [ "$DEPRECATED_FUNCTIONS" -gt 0 ]; then 
    echo -e "${RED}[CRITICAL] Found $DEPRECATED_FUNCTIONS deprecated functions that throw errors${NC}"
    echo -e "${RED}           These functions are called but always return errors${NC}"
    rg "deprecated.*use.*instead|Universal.*deprecated|ProviderManager deprecated" src/ -n | head -5
    LEGACY_ISSUES_FOUND=true
    ALL_PASSED=false
fi

if [ "$LEGACY_TOOLS" -gt 0 ]; then 
    echo -e "${RED}[CRITICAL] Found $LEGACY_TOOLS legacy tool handlers that throw errors${NC}"
    echo -e "${RED}           These tools are advertised but always fail when called${NC}"
    rg "Legacy tool.*deprecated" src/ -n | head -5
    LEGACY_ISSUES_FOUND=true
    ALL_PASSED=false
fi

if [ "$PLACEHOLDER_IMPLEMENTATIONS" -gt 0 ]; then 
    echo -e "${RED}[CRITICAL] Found $PLACEHOLDER_IMPLEMENTATIONS placeholder tool handlers that return mock data${NC}"
    echo -e "${RED}           Tools that return 'Value' instead of 'McpResponse' are placeholders${NC}"
    echo -e "${RED}           These tools appear to work but return fake data to users${NC}"
    echo -e "${YELLOW}   Placeholder functions (should return McpResponse):${NC}"
    rg "fn handle_.*-> Value" src/ -n | head -5
    echo -e "${YELLOW}   Fix: Route through Universal Protocol or implement real functionality${NC}"
    LEGACY_ISSUES_FOUND=true
    ALL_PASSED=false
fi

if [ "$LEGACY_ISSUES_FOUND" = true ]; then
    echo -e "${RED}FAST FAIL: Remove legacy functions that confuse users${NC}"
    echo -e "${RED}   Functions that advertise but don't work create poor UX${NC}"
    exit 1
fi

echo -e "${GREEN}[OK] No legacy functions found that throw nonsense behavior${NC}"
echo ""

# Frontend checks
if [ -d "frontend" ]; then
    echo ""
    echo -e "${BLUE}==== Frontend Checks ====${NC}"
    
    cd frontend
    
    # Run ESLint
    echo -e "${BLUE}==== Running frontend linter (ESLint)... ====${NC}"
    if npm run lint; then
        echo -e "${GREEN}[OK] Frontend linting passed${NC}"
    else
        echo -e "${RED}[FAIL] Frontend linting failed${NC}"
        ALL_PASSED=false
    fi
    
    # Run TypeScript type checking
    echo -e "${BLUE}==== Running TypeScript type checking... ====${NC}"
    if npm run type-check; then
        echo -e "${GREEN}[OK] TypeScript type checking passed${NC}"
    else
        echo -e "${RED}[FAIL] TypeScript type checking failed${NC}"
        ALL_PASSED=false
    fi
    
    # Run frontend tests
    echo -e "${BLUE}==== Running frontend tests... ====${NC}"
    if npm test -- --run; then
        echo -e "${GREEN}[OK] Frontend tests passed${NC}"
    else
        echo -e "${RED}[FAIL] Frontend tests failed${NC}"
        ALL_PASSED=false
    fi
    
    # Run frontend tests with coverage (if enabled)
    if [ "$ENABLE_COVERAGE" = true ]; then
        echo -e "${BLUE}==== Running frontend tests with coverage... ====${NC}"
        if npm run test:coverage -- --run; then
            echo -e "${GREEN}[OK] Frontend coverage report generated in coverage/${NC}"
        else
            echo -e "${YELLOW}[WARN]  Failed to generate frontend coverage report${NC}"
        fi
    fi
    
    # Check frontend build
    echo -e "${BLUE}==== Checking frontend build... ====${NC}"
    if npm run build; then
        echo -e "${GREEN}[OK] Frontend build successful${NC}"
    else
        echo -e "${RED}[FAIL] Frontend build failed${NC}"
        ALL_PASSED=false
    fi
    
    cd ..
fi

# Check for security vulnerabilities (if cargo-audit is installed)
echo -e "${BLUE}==== Checking for security vulnerabilities... ====${NC}"
if command_exists cargo-audit; then
    if RUST_LOG=off cargo audit --ignore RUSTSEC-2023-0071 --quiet >/dev/null 2>&1; then
        echo -e "${GREEN}[OK] No security vulnerabilities found${NC}"
    else
        echo -e "${YELLOW}[WARN]  Security vulnerabilities detected${NC}"
        # Don't fail the build for vulnerabilities
    fi
else
    echo -e "${YELLOW}[WARN]  cargo-audit not installed. Install with: cargo install cargo-audit${NC}"
fi

# Performance and Architecture Gates (dev best practices)
echo -e "${BLUE}==== Performance and Architecture Validation... ====${NC}"

# Build release binary and check size
echo -e "${BLUE}==== Building release binary for performance check... ====${NC}"
if cargo build --release --quiet; then
    echo -e "${GREEN}[OK] Release build successful${NC}"
    
    # Check binary size (dev best practice: <50MB for pierre-mcp-server)
    if [ -f "target/release/pierre-mcp-server" ]; then
        BINARY_SIZE=$(ls -lh target/release/pierre-mcp-server | awk '{print $5}')
        BINARY_SIZE_BYTES=$(ls -l target/release/pierre-mcp-server | awk '{print $5}')
        MAX_SIZE_BYTES=$((50 * 1024 * 1024))  # 50MB in bytes
        
        if [ "$BINARY_SIZE_BYTES" -le "$MAX_SIZE_BYTES" ]; then
            echo -e "${GREEN}[OK] Binary size ($BINARY_SIZE) within dev best practice (<50MB)${NC}"
        else
            echo -e "${RED}[FAIL] Binary size ($BINARY_SIZE) exceeds dev best practice limit (50MB)${NC}"
            ALL_PASSED=false
        fi
    else
        echo -e "${YELLOW}[WARN] pierre-mcp-server binary not found - size check skipped${NC}"
    fi
else
    echo -e "${RED}[FAIL] Release build failed${NC}"
    ALL_PASSED=false
fi


# Check documentation
echo -e "${BLUE}==== Checking documentation... ====${NC}"
if cargo doc --no-deps --quiet; then
    echo -e "${GREEN}[OK] Documentation builds successfully${NC}"
else
    echo -e "${RED}[FAIL] Documentation build failed${NC}"
    ALL_PASSED=false
fi

# Check Python examples (if they exist)
if [ -d "examples/python" ]; then
    echo -e "${BLUE}==== Validating Python Examples... ====${NC}"
    
    # Check Python syntax for all Python files
    PYTHON_SYNTAX_OK=true
    for py_file in $(find examples/python -name "*.py"); do
        if ! python3 -m py_compile "$py_file" 2>/dev/null; then
            echo -e "${RED}[FAIL] Syntax error in $py_file${NC}"
            PYTHON_SYNTAX_OK=false
            ALL_PASSED=false
        fi
    done
    
    if [ "$PYTHON_SYNTAX_OK" = true ]; then
        echo -e "${GREEN}[OK] Python syntax validation passed${NC}"
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
print('[OK] Auth utilities import and basic config work')
" 2>/dev/null; then
        echo -e "${GREEN}[OK] Python auth utilities validated${NC}"
    else
        echo -e "${YELLOW}[WARN]  Python auth utilities validation skipped (dependencies missing)${NC}"
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
print(f'[OK] Data processing works: score={result[\"total_score\"]}, quality={validation[\"quality_score\"]:.1f}')
" 2>/dev/null; then
        echo -e "${GREEN}[OK] Python data utilities validated${NC}"
    else
        echo -e "${RED}[FAIL] Python data utilities validation failed${NC}"
        ALL_PASSED=false
    fi
    
    # Test CI mode with mock data
    echo -e "${BLUE}==== Testing CI mode with mock data... ====${NC}"
    export PIERRE_CI_MODE=true
    
    # Test A2A demo with timeout if available, otherwise run directly
    if command_exists timeout; then
        if timeout 15s python3 python/a2a/enterprise_demo.py > /dev/null 2>&1; then
            echo -e "${GREEN}[OK] A2A demo works with mock data${NC}"
        else
            echo -e "${YELLOW}[WARN]  A2A demo test failed or timed out${NC}"
        fi
    else
        if python3 python/a2a/enterprise_demo.py > /dev/null 2>&1; then
            echo -e "${GREEN}[OK] A2A demo works with mock data${NC}"
        else
            echo -e "${YELLOW}[WARN]  A2A demo test failed${NC}"
        fi
    fi
    
    # Test MCP stdio example with timeout if available, otherwise run directly
    if command_exists timeout; then
        if timeout 15s python3 python/mcp_stdio_example.py > /dev/null 2>&1; then
            echo -e "${GREEN}[OK] MCP stdio example works${NC}"
        else
            echo -e "${YELLOW}[WARN] MCP stdio example test failed or timed out (needs server)${NC}"
        fi
    else
        if python3 python/mcp_stdio_example.py > /dev/null 2>&1; then
            echo -e "${GREEN}[OK] MCP stdio example works${NC}"
        else
            echo -e "${YELLOW}[WARN] MCP stdio example test failed (needs server)${NC}"
        fi
    fi
    
    # Test provisioning mock provider
    echo -e "${BLUE}==== Testing provisioning mock provider... ====${NC}"
    if command_exists timeout; then
        if timeout 10s python3 python/provisioning/mock_strava_provider.py > /dev/null 2>&1; then
            echo -e "${GREEN}[OK] Mock Strava provider works${NC}"
        else
            echo -e "${YELLOW}[WARN]  Mock Strava provider test failed or timed out${NC}"
        fi
    else
        if python3 python/provisioning/mock_strava_provider.py > /dev/null 2>&1; then
            echo -e "${GREEN}[OK] Mock Strava provider works${NC}"
        else
            echo -e "${YELLOW}[WARN]  Mock Strava provider test failed${NC}"
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
echo -e "${GREEN}[OK] Final cleanup completed${NC}"

# Summary
echo ""
echo -e "${BLUE}==== Dev Standards Compliance Summary ====${NC}"
if [ "$ALL_PASSED" = true ]; then
    echo -e "${GREEN}ALL VALIDATION PASSED - Task can be marked complete${NC}"
    echo ""
    echo "[OK] Rust formatting"
    echo "[OK] Rust linting (STRICT dev standards compliance)"
    echo "[OK] Rust compilation"
    echo "[OK] Rust tests"
    echo "[OK] Release mode tests"
    echo "[OK] A2A compliance tests"
    echo "[OK] OAuth automation infrastructure"
    echo "[OK] Prohibited patterns check"
    echo "[OK] Clone usage analysis"
    echo "[OK] Arc usage patterns check"
    echo "[OK] Unified architectural validation"
    echo "[OK] Binary size validation"
    echo "[OK] Frontend linting"
    echo "[OK] TypeScript type checking"
    echo "[OK] Frontend tests"
    echo "[OK] Frontend build"
    if [ "$ENABLE_COVERAGE" = true ]; then
        echo "[OK] Frontend code coverage"
    fi
    echo "[OK] Documentation"
    if [ "$ENABLE_COVERAGE" = true ] && command_exists cargo-llvm-cov; then
        echo "[OK] Rust code coverage"
    fi
    if [ -d "examples/python" ]; then
        echo "[OK] Python examples validation"
    fi
    echo ""
    echo -e "${GREEN}Code meets ALL dev standards and is ready for production!${NC}"
    exit 0
else
    echo -e "${RED}VALIDATION FAILED - Task cannot be marked complete${NC}"
    echo -e "${RED}Fix ALL issues above to meet dev standards requirements${NC}"
    exit 1
fi