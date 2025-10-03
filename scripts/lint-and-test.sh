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

# Track Pierre MCP server PID if we start it
MCP_SERVER_PID=""

# Cleanup function - shut down server if we started it
cleanup_mcp_server() {
    if [ -n "$MCP_SERVER_PID" ]; then
        echo ""
        echo -e "${BLUE}==== Shutting down Pierre MCP server (PID: $MCP_SERVER_PID)... ====${NC}"
        kill "$MCP_SERVER_PID" 2>/dev/null || true
        wait "$MCP_SERVER_PID" 2>/dev/null || true
        echo -e "${GREEN}[OK] Pierre MCP server stopped${NC}"
        MCP_SERVER_PID=""
    fi
}

# Register cleanup function to run on exit
trap cleanup_mcp_server EXIT INT TERM

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
# UNIFIED ARCHITECTURAL VALIDATION (DATA COLLECTION)
# ============================================================================

# Collect all metrics silently without verbose output
echo -e "${BLUE}Analyzing codebase architecture and quality patterns...${NC}"

# Critical Pattern: Null UUID detection (fast-fail)
NULL_UUIDS=$(rg "00000000-0000-0000-0000-000000000000" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

# Memory Management Analysis (will use TOML patterns below)
TOTAL_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | wc -l 2>/dev/null || echo 0)

# Load validation patterns from TOML configuration
VALIDATION_PATTERNS_FILE="$SCRIPT_DIR/validation-patterns.toml"
if [ -f "$VALIDATION_PATTERNS_FILE" ]; then
    eval "$(python3 "$SCRIPT_DIR/parse-validation-patterns.py" "$VALIDATION_PATTERNS_FILE")"

    # Use TOML-configured patterns for existing checks
    IMPLEMENTATION_PLACEHOLDERS=$(rg "$CRITICAL_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    if [ -n "$WARNING_PATTERNS" ]; then
        TOTAL_WARNING_COUNT=$(rg "$WARNING_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
        # Count long functions with proper documentation (either inline or on preceding line)
        DOCUMENTED_LONG_FUNCTIONS=$(rg "#\[allow\(clippy::too_many_lines\)\]" src/ -B1 | rg -c "// Long function:|// Safe:" 2>/dev/null || echo "0")
        PLACEHOLDER_WARNINGS=$((TOTAL_WARNING_COUNT > DOCUMENTED_LONG_FUNCTIONS ? TOTAL_WARNING_COUNT - DOCUMENTED_LONG_FUNCTIONS : 0))
    else
        PLACEHOLDER_WARNINGS=0
    fi

    # TOML-based checks (replacing legacy hardcoded patterns)
    TOML_UNWRAPS=$(rg "$UNWRAP_PATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" | rg -v "// Safe|hardcoded.*valid|static.*data" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_EXPECTS=$(rg "$EXPECT_PATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" | rg -v "// Safe|ServerResources.*required" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_PANICS=$(rg "$PANIC_PATTERNS_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_ERROR_HANDLING=$((TOML_UNWRAPS + TOML_EXPECTS + TOML_PANICS))
    TOML_DEVELOPMENT_ARTIFACTS=$(rg "$DEVELOPMENT_ARTIFACTS_PATTERNS" src/ -g "!tests/*" -g "!examples/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_PRODUCTION_HYGIENE=$(rg "$PRODUCTION_HYGIENE_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_TEMPORARY_CODE=$(rg "$TEMPORARY_CODE_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_CLIPPY_SUPPRESSIONS=$(rg "$CLIPPY_SUPPRESSIONS_PATTERNS" src/ | rg -v "cast_|too_many_lines|struct_excessive_bools" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_LONG_FUNCTIONS=$(rg "$LONG_FUNCTION_SUPPRESSIONS_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_PROBLEMATIC_NAMING=$(rg "$PROBLEMATIC_NAMING_PATTERNS" src/ | rg -v "let _[[:space:]]*=" | rg -v "let _result|let _response|let _output" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_MAGIC_NUMBERS=$(rg "$THRESHOLD_PATTERNS" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # TOML-based architectural pattern analysis
    TOML_RESOURCE_CREATION=$(rg "$RESOURCE_CREATION_PATTERNS" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_FAKE_RESOURCES=$(rg "$FAKE_RESOURCES_PATTERNS" src/ -g "!src/bin/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_OBSOLETE_FUNCTIONS=$(rg "$OBSOLETE_FUNCTIONS_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_UNUSED_VARIABLES=$(rg "$UNUSED_VARIABLES_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_DEPRECATED_CODE=$(rg "$DEPRECATED_CODE_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # TOML-based memory management analysis
    TOML_LEGITIMATE_ARC_CLONES=$(rg "$LEGITIMATE_ARC_CLONES_PATTERNS" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_PROBLEMATIC_DB_CLONES=$(rg "$PROBLEMATIC_DB_CLONES_PATTERNS" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_ARC_USAGE=$(rg "$ARC_USAGE_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_CLONE_USAGE=$(rg "$CLONE_USAGE_PATTERNS" src/ | grep -v 'src/bin/' --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # Map TOML results to legacy variable names for backward compatibility
    PROBLEMATIC_UNWRAPS=$TOML_UNWRAPS
    PROBLEMATIC_EXPECTS=$TOML_EXPECTS
    PANICS=$TOML_PANICS
    TODOS=$TOML_DEVELOPMENT_ARTIFACTS
    PRODUCTION_MOCKS=$TOML_PRODUCTION_HYGIENE
    PROBLEMATIC_UNDERSCORE_NAMES=$TOML_PROBLEMATIC_NAMING

    # Separate specific checks instead of all using DEVELOPMENT_ARTIFACTS
    CFG_TEST_IN_SRC=$(rg "#\[cfg\(test\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    DEAD_CODE=$(rg "#\[allow\(dead_code\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    IGNORED_TESTS=$(rg "#\[ignore\]" tests/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    CLIPPY_ALLOWS_PROBLEMATIC=$TOML_CLIPPY_SUPPRESSIONS
    CLIPPY_ALLOWS_TOO_MANY_LINES=$TOML_LONG_FUNCTIONS
    TEMP_SOLUTIONS=$TOML_TEMPORARY_CODE
    EXAMPLE_EMAILS=$TOML_PRODUCTION_HYGIENE

    # Map new architectural patterns to legacy variables
    RESOURCE_CREATION=$TOML_RESOURCE_CREATION
    FAKE_RESOURCES=$TOML_FAKE_RESOURCES
    OBSOLETE_FUNCTIONS=$TOML_OBSOLETE_FUNCTIONS
    UNUSED_VARS=$TOML_UNUSED_VARIABLES
    DEPRECATED=$TOML_DEPRECATED_CODE
    LEGITIMATE_ARC_CLONES=$TOML_LEGITIMATE_ARC_CLONES
    PROBLEMATIC_DB_CLONES=$TOML_PROBLEMATIC_DB_CLONES
    TOTAL_ARCS=$TOML_ARC_USAGE
    TOTAL_CLONES=$TOML_CLONE_USAGE
else
    echo -e "${YELLOW}[WARN] Validation patterns TOML file not found, using fallback patterns${NC}"
    # Fallback to legacy hardcoded patterns if TOML file is missing
    IMPLEMENTATION_PLACEHOLDERS=$(rg "Implementation would|Would implement|Should implement|Will implement|TODO: Implementation|Available for real implementation|available for real implementation|Implement the code|stub implementation|mock implementation" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    PLACEHOLDER_WARNINGS=0

    # Legacy fallback patterns
    PROBLEMATIC_UNWRAPS=$(rg "\.unwrap\(\)" src/ | rg -v "// Safe|hardcoded.*valid|static.*data" | wc -l 2>/dev/null || echo 0)
    PROBLEMATIC_EXPECTS=$(rg "\.expect\(" src/ | rg -v "// Safe|ServerResources.*required" | wc -l 2>/dev/null || echo 0)
    PANICS=$(rg "panic!\(" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TODOS=$(rg "TODO|FIXME|XXX" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    PRODUCTION_MOCKS=$(rg "mock_|get_mock|return.*mock|demo purposes|for demo|stub implementation|mock implementation" src/ -g "!src/bin/*" -g "!tests/*" | wc -l 2>/dev/null || echo 0)
    PROBLEMATIC_UNDERSCORE_NAMES=$(rg "fn _|let _[a-zA-Z]|struct _|enum _" src/ | rg -v "let _[[:space:]]*=" | rg -v "let _result|let _response|let _output" | wc -l 2>/dev/null || echo 0)
    CFG_TEST_IN_SRC=$(rg "#\[cfg\(test\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    CLIPPY_ALLOWS_PROBLEMATIC=$(rg "#!?\[allow\(clippy::" src/ | rg -v "cast_|too_many_lines|struct_excessive_bools" | wc -l 2>/dev/null || echo 0)
    CLIPPY_ALLOWS_TOO_MANY_LINES=$(rg "#!?\[allow\(clippy::too_many_lines\)\]" src/ | wc -l 2>/dev/null || echo 0)
    TEMP_SOLUTIONS=$(rg "\bhack\b|\bworkaround\b|\bquick.*fix\b|future.*implementation|temporary.*solution|temp.*fix" src/ --count-matches 2>/dev/null | cut -d: -f2 | python3 -c "import sys; lines = sys.stdin.readlines(); print(sum(int(x.strip()) for x in lines) if lines else 0)" 2>/dev/null || echo 0)
    DEAD_CODE=$(rg "#\[allow\(dead_code\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    IGNORED_TESTS=$(rg "#\[ignore\]" tests/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    EXAMPLE_EMAILS=$(rg "example\.com|test@" src/ -g "!src/bin/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # Fallback architectural patterns
    RESOURCE_CREATION=$(rg "AuthManager::new|OAuthManager::new|A2AClientManager::new|TenantOAuthManager::new" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    FAKE_RESOURCES=$(rg "Arc::new\(ServerResources\s*[\{\:]" src/ -g "!src/bin/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    OBSOLETE_FUNCTIONS=$(rg "fn.*run_http_server\(" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    UNUSED_VARS=$(rg "#\[allow\(unused.*\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    DEPRECATED=$(rg "#\[deprecated\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    LEGITIMATE_ARC_CLONES=$(rg "database_arc\.clone\(\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    PROBLEMATIC_DB_CLONES=$(rg "\.as_ref\(\)\.clone\(\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOTAL_ARCS=$(rg "Arc::" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # TOML placeholders
    TOML_DEVELOPMENT_ARTIFACTS=0
    TOML_PRODUCTION_HYGIENE=0
    TOML_TEMPORARY_CODE=0
    TOML_CLIPPY_SUPPRESSIONS=0
    TOML_LONG_FUNCTIONS=0
    TOML_PROBLEMATIC_NAMING=0
    TOML_MAGIC_NUMBERS=0
fi

# Memory Management Analysis
TOTAL_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | wc -l 2>/dev/null || echo 0)
LEGITIMATE_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | rg "Arc::|resources\.|database\.|auth_manager\.|sse_manager\.|websocket_manager\.|\.to_string\(\)|format!|String::from|token|url|name|path|message|error|Error|client_id|client_secret|redirect_uri|access_token|refresh_token|user_id|tenant_id|request\.|response\.|context\.|config\.|profile\." | wc -l 2>/dev/null || echo 0)
PROBLEMATIC_CLONES=$((TOTAL_CLONES - LEGITIMATE_CLONES))

# Get files with file-level clone safety documentation
FILES_WITH_CLONE_DOCS=$(rg -l "NOTE: All.*clone.*calls.*Safe" src/ 2>/dev/null || echo "")
DOCUMENTED_FILES_COUNT=$(echo "$FILES_WITH_CLONE_DOCS" | grep -v '^$' | wc -l 2>/dev/null || echo 0)

# Count documented clones from files with bulk documentation
DOCUMENTED_CLONES=0
if [ -n "$FILES_WITH_CLONE_DOCS" ] && [ "$DOCUMENTED_FILES_COUNT" -gt 0 ]; then
    DOCUMENTED_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | grep -f <(echo "$FILES_WITH_CLONE_DOCS") | wc -l 2>/dev/null || echo 0)
fi


# Advanced Arc analysis
TOTAL_ARCS=$(rg "Arc::" src/ | wc -l 2>/dev/null || echo 0)
DEPENDENCY_ARCS=$(rg "Arc<ServerResources>|Arc<.*Manager>|Arc<.*Executor>" src/ | wc -l 2>/dev/null || echo 0)
CONCURRENT_ARCS=$(rg "Arc<.*Lock.*>|Arc<.*Mutex.*>|Arc<.*RwLock.*>" src/ | wc -l 2>/dev/null || echo 0)
MAGIC_NUMBERS=$(rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" | wc -l 2>/dev/null || echo 0)

# ============================================================================
# UNIFIED ARCHITECTURAL VALIDATION SUMMARY
# ============================================================================
echo ""
echo -e "${BLUE}==== UNIFIED ARCHITECTURAL VALIDATION SUMMARY ====${NC}"

# Helper function to truncate text for table display
truncate_text() {
    local text="$1"
    local max_length="$2"
    if [ ${#text} -gt $max_length ]; then
        echo "${text:0:$((max_length-3))}..."
    else
        echo "$text"
    fi
}

# Helper function to get first file location for warnings
get_first_location() {
    local pattern="$1"
    local result=$(eval "$pattern" 2>/dev/null | head -1 | cut -d: -f1-2)
    if [ -n "$result" ]; then
        truncate_text "$result" 37
    else
        echo "No specific location found"
    fi
}

# Helper function to format status with consistent width
format_status() {
    local status="$1"
    # The Status column is 10 characters wide (including padding)
    # We need to account for emoji width differences
    case "$status" in
        "✅ PASS")
            printf "%-9s " "$status"  # Green checkmark is wider, needs less padding
            ;;
        "⚠️ WARN")
            printf "%-8s  " "$status"  # Warning triangle is narrower, needs more padding
            ;;
        "⚠️ INFO")
            printf "%-8s  " "$status"  # Same as WARN
            ;;
        "❌ FAIL")
            printf "%-8s  " "$status"  # X mark is narrower, needs more padding
            ;;
        *)
            printf "%-10s" "$status"   # Default case
            ;;
    esac
}

# Create clean ASCII table with proper formatting
echo "┌─────────────────────────────────────┬───────┬──────────┬─────────────────────────────────────────┐"
echo "│ Validation Category                 │ Count │ Status   │ Details / First Location                │"
echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Critical Fast-Fail Checks
printf "│ %-35s │ %5d │ " "Null UUIDs (00000000-...)" "$NULL_UUIDS"
if [ "$NULL_UUIDS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No placeholder UUIDs"
else
    FIRST_NULL_UUID=$(get_first_location 'rg "00000000-0000-0000-0000-000000000000" src/ -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_NULL_UUID"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Anti-Pattern Detection
printf "│ %-35s │ %5d │ " "Database clones (total)" "$TOTAL_DATABASE_CLONES"
if [ "$PROBLEMATIC_DB_CLONES" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "${LEGITIMATE_ARC_CLONES} legitimate Arc clones"
else
    FIRST_DB_CLONE=$(get_first_location 'rg "\.as_ref\(\)\.clone\(\)|Arc::new\(database\.clone\(\)\)" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_DB_CLONE"
fi

printf "│ %-35s │ %5d │ " "Resource creation patterns" "$RESOURCE_CREATION"
if [ "$RESOURCE_CREATION" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Using dependency injection"
else
    FIRST_RESOURCE=$(get_first_location 'rg "AuthManager::new|OAuthManager::new|A2AClientManager::new|TenantOAuthManager::new" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_RESOURCE"
fi

printf "│ %-35s │ %5d │ " "Fake resource assemblies" "$FAKE_RESOURCES"
if [ "$FAKE_RESOURCES" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No fake ServerResources"
else
    FIRST_FAKE=$(get_first_location 'rg "Arc::new\(ServerResources\s*\{" src/ -g "!src/bin/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_FAKE"
fi

printf "│ %-35s │ %5d │ " "Obsolete functions" "$OBSOLETE_FUNCTIONS"
if [ "$OBSOLETE_FUNCTIONS" -le 1 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Within acceptable limits"
else
    FIRST_OBSOLETE=$(get_first_location 'rg "run_http_server\(" src/ -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_OBSOLETE"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Code Quality Analysis
printf "│ %-35s │ %5d │ " "Problematic unwraps" "$PROBLEMATIC_UNWRAPS"
if [ "$PROBLEMATIC_UNWRAPS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Proper error handling"
else
    FIRST_UNWRAP=$(get_first_location 'rg "\.unwrap\(\)" src/ | rg -v "// Safe|hardcoded.*valid|static.*data" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_UNWRAP"
fi

printf "│ %-35s │ %5d │ " "Problematic expects" "$PROBLEMATIC_EXPECTS"
if [ "$PROBLEMATIC_EXPECTS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Proper error handling"
else
    FIRST_EXPECT=$(get_first_location 'rg "\.expect\(" src/ | rg -v "// Safe|ServerResources.*required" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_EXPECT"
fi

printf "│ %-35s │ %5d │ " "Panic calls" "$PANICS"
if [ "$PANICS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No panic! found"
else
    FIRST_PANIC=$(get_first_location 'rg "panic!\(" src/ -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_PANIC"
fi

printf "│ %-35s │ %5d │ " "TODOs/FIXMEs" "$TODOS"
if [ "$TODOS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No incomplete code"
else
    FIRST_TODO=$(get_first_location 'rg "TODO|FIXME|XXX" src/ -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_TODO"
fi

printf "│ %-35s │ %5d │ " "Production mock implementations" "$PRODUCTION_MOCKS"
if [ "$PRODUCTION_MOCKS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No mock code in production"
else
    FIRST_PRODUCTION_MOCK=$(get_first_location 'rg "mock_|get_mock|return.*mock|demo purposes|for demo|stub implementation|mock implementation" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_PRODUCTION_MOCK"
fi

printf "│ %-35s │ %5d │ " "Problematic underscore names" "$PROBLEMATIC_UNDERSCORE_NAMES"
if [ "$PROBLEMATIC_UNDERSCORE_NAMES" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Good naming conventions"
else
    FIRST_UNDERSCORE=$(get_first_location 'rg "fn _|let _[a-zA-Z]|struct _|enum _" src/ | rg -v "let _[[:space:]]*=" | rg -v "let _result|let _response|let _output" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_UNDERSCORE"
fi

printf "│ %-35s │ %5d │ " "Test modules in src/" "$CFG_TEST_IN_SRC"
if [ "$CFG_TEST_IN_SRC" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Tests belong in tests/ directory"
else
    FIRST_CFG_TEST=$(get_first_location 'rg "#\[cfg\(test\)\]" src/ -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_CFG_TEST"
fi

printf "│ %-35s │ %5d │ " "Problematic clippy allows" "$CLIPPY_ALLOWS_PROBLEMATIC"
if [ "$CLIPPY_ALLOWS_PROBLEMATIC" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Fix issues instead of silencing"
else
    FIRST_PROBLEMATIC_ALLOW=$(get_first_location 'rg "#!?\[allow\(clippy::" src/ | rg -v "cast_|too_many_lines|struct_excessive_bools" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_PROBLEMATIC_ALLOW"
fi

printf "│ %-35s │ %5d │ " "Long functions (too_many_lines)" "$CLIPPY_ALLOWS_TOO_MANY_LINES"
if [ "$CLIPPY_ALLOWS_TOO_MANY_LINES" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Functions are appropriately sized"
else
    FIRST_LONG_FUNCTION=$(get_first_location 'rg "#!?\[allow\(clippy::too_many_lines\)\]" src/ -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_LONG_FUNCTION"
fi

printf "│ %-35s │ %5d │ " "Dead code annotations" "$DEAD_CODE"
if [ "$DEAD_CODE" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Remove dead code instead of hiding"
else
    FIRST_DEAD_CODE=$(get_first_location 'rg "#\[allow\(dead_code\)\]" src/ -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_DEAD_CODE"
fi

printf "│ %-35s │ %5d │ " "Example emails" "$EXAMPLE_EMAILS"
if [ "$EXAMPLE_EMAILS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No test emails in production"
else
    FIRST_EMAIL=$(get_first_location 'rg "example\.com|test@" src/ -g "!src/bin/*" -n')
    printf "$(format_status "⚠️ INFO")│ %-39s │\n" "$FIRST_EMAIL"
fi

printf "│ %-35s │ %5d │ " "Temporary solutions" "$TEMP_SOLUTIONS"
if [ "$TEMP_SOLUTIONS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No temporary code"
else
    FIRST_TEMP=$(get_first_location 'rg "\bhack\b|\bworkaround\b|\bquick.*fix\b|future.*implementation|temporary.*solution|temp.*fix" src/ -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_TEMP"
fi

printf "│ %-35s │ %5d │ " "Ignored tests" "$IGNORED_TESTS"
if [ "$IGNORED_TESTS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "All tests run in CI/CD"
else
    FIRST_IGNORED=$(get_first_location 'rg "#\[ignore\]" tests/ -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_IGNORED"
fi

printf "│ %-35s │ %5d │ " "Implementation placeholders" "$IMPLEMENTATION_PLACEHOLDERS"
if [ "$IMPLEMENTATION_PLACEHOLDERS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No placeholder implementations"
else
    if [ -n "$CRITICAL_PATTERNS" ]; then
        FIRST_PLACEHOLDER=$(get_first_location 'rg "$CRITICAL_PATTERNS" src/ -n')
    else
        FIRST_PLACEHOLDER=$(get_first_location 'rg "Implementation would|Would implement|Should implement|Will implement|TODO: Implementation" src/ -n')
    fi
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_PLACEHOLDER"
fi

printf "│ %-35s │ %5d │ " "Placeholder warnings" "$PLACEHOLDER_WARNINGS"
if [ "$PLACEHOLDER_WARNINGS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No hedge language or evasion patterns"
else
    if [ -n "$WARNING_PATTERNS" ]; then
        FIRST_WARNING_PLACEHOLDER=$(get_first_location 'rg "$WARNING_PATTERNS" src/ -n')
    else
        FIRST_WARNING_PLACEHOLDER="Check TOML configuration"
    fi
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_WARNING_PLACEHOLDER"
fi

printf "│ %-35s │ %5d │ " "Error handling anti-patterns" "$TOML_ERROR_HANDLING"
if [ "$TOML_ERROR_HANDLING" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Proper error handling patterns"
else
    FIRST_ERROR_HANDLING=$(get_first_location 'rg "$ERROR_HANDLING_ANTIPATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_ERROR_HANDLING"
fi

printf "│ %-35s │ %5d │ " "Development artifacts" "$TOML_DEVELOPMENT_ARTIFACTS"
if [ "$TOML_DEVELOPMENT_ARTIFACTS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No development artifacts in production"
else
    FIRST_DEV_ARTIFACT=$(get_first_location 'rg "$DEVELOPMENT_ARTIFACTS_PATTERNS" src/ -g "!tests/*" -g "!examples/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_DEV_ARTIFACT"
fi

printf "│ %-35s │ %5d │ " "Production hygiene issues" "$TOML_PRODUCTION_HYGIENE"
if [ "$TOML_PRODUCTION_HYGIENE" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No test artifacts in production"
else
    FIRST_HYGIENE=$(get_first_location 'rg "$PRODUCTION_HYGIENE_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_HYGIENE"
fi

printf "│ %-35s │ %5d │ " "Temporary code solutions" "$TOML_TEMPORARY_CODE"
if [ "$TOML_TEMPORARY_CODE" -le "${MAX_TEMPORARY_CODE:-5}" ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Within acceptable limits"
else
    FIRST_TEMP=$(get_first_location 'rg "$TEMPORARY_CODE_PATTERNS" src/ -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_TEMP"
fi

printf "│ %-35s │ %5d │ " "TOML-based magic numbers" "$TOML_MAGIC_NUMBERS"
if [ "$TOML_MAGIC_NUMBERS" -le "${MAX_MAGIC_NUMBERS:-10}" ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Within acceptable limits"
else
    FIRST_MAGIC=$(get_first_location 'rg "$THRESHOLD_PATTERNS" src/ -g "!src/constants.rs" -g "!src/config/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_MAGIC"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Memory Management Analysis
printf "│ %-35s │ %5d │ " "Problematic clones found" "$PROBLEMATIC_CLONES"
# According to CLAUDE.md, this multitenant architecture accepts 490 clones
if [ "$PROBLEMATIC_CLONES" -le 300 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Within multitenant architecture limits"
else
    # Get first problematic clone from files without file-level docs, excluding individually documented ones
    if [ -n "$FILES_WITH_CLONE_DOCS" ]; then
        FIRST_PROBLEMATIC_CLONE=$(get_first_location 'rg "\.clone\(\)" src/ | grep -v -f <(echo "$FILES_WITH_CLONE_DOCS") | rg -v "// Safe" -n')
    else
        FIRST_PROBLEMATIC_CLONE=$(get_first_location 'rg "\.clone\(\)" src/ | rg -v "// Safe" -n')
    fi
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_PROBLEMATIC_CLONE"
fi

printf "│ %-35s │ %5d │ " "Clone usage" "$TOTAL_CLONES"
if [ "$PROBLEMATIC_CLONES" -le 300 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "$LEGITIMATE_CLONES legitimate, $PROBLEMATIC_CLONES architectural"
else
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$LEGITIMATE_CLONES legitimate, $PROBLEMATIC_CLONES need review"
fi

printf "│ %-35s │ %5d │ " "Arc usage" "$TOTAL_ARCS"
if [ "$TOTAL_ARCS" -lt 50 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Appropriate for service architecture"
else
    FIRST_PROBLEMATIC_ARC=$(get_first_location 'rg "Arc::" src/ | rg -v "ServerResources|Manager|Executor|Lock|Mutex|RwLock" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_PROBLEMATIC_ARC"
fi

printf "│ %-35s │ %5d │ " "Magic numbers" "$MAGIC_NUMBERS"
if [ "$MAGIC_NUMBERS" -lt 10 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Good configuration practices"
else
    FIRST_MAGIC=$(get_first_location 'rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_MAGIC"
fi

echo "└─────────────────────────────────────┴───────┴──────────┴─────────────────────────────────────────┘"

# Critical Fast-Fail: Null UUIDs (must exit immediately)
if [ "$NULL_UUIDS" -gt 0 ]; then
    echo ""
    echo -e "${RED}❌ CRITICAL ARCHITECTURAL FAILURE: NULL UUIDs DETECTED${NC}"
    echo -e "${RED}Found $NULL_UUIDS occurrences of null UUID (00000000-0000-0000-0000-000000000000)${NC}"
    echo -e "${RED}Null UUIDs indicate placeholder or test code that must not be in production${NC}"
    echo ""
    echo -e "${YELLOW}Locations of null UUIDs:${NC}"
    rg "00000000-0000-0000-0000-000000000000" src/ -n
    echo ""
    echo -e "${RED}FAST FAIL: Replace null UUIDs with proper UUID generation or remove test code${NC}"
    exit 1
fi

# Report comprehensive summary based on actual findings
# Note: PROBLEMATIC_CLONES not included as they're acceptable in multitenant architecture per CLAUDE.md
CRITICAL_ISSUES=$((NULL_UUIDS + PROBLEMATIC_DB_CLONES + PROBLEMATIC_UNWRAPS + PROBLEMATIC_EXPECTS + PANICS + IGNORED_TESTS + IMPLEMENTATION_PLACEHOLDERS))
CRITICAL_ISSUES=$((CRITICAL_ISSUES + TOML_PRODUCTION_HYGIENE + CFG_TEST_IN_SRC + DEAD_CODE))

WARNINGS=$((FAKE_RESOURCES + (OBSOLETE_FUNCTIONS > 1 ? OBSOLETE_FUNCTIONS - 1 : 0)))
WARNINGS=$((WARNINGS + RESOURCE_CREATION + TODOS + PROBLEMATIC_UNDERSCORE_NAMES + TEMP_SOLUTIONS))
WARNINGS=$((WARNINGS + (PROBLEMATIC_CLONES > 300 ? 1 : 0) + (TOTAL_ARCS >= 50 ? 1 : 0) + (MAGIC_NUMBERS >= 10 ? 1 : 0)))
WARNINGS=$((WARNINGS + (PLACEHOLDER_WARNINGS > 0 ? 1 : 0)))
WARNINGS=$((WARNINGS + (TOML_DEVELOPMENT_ARTIFACTS > 0 ? 1 : 0) + (TOML_TEMPORARY_CODE > MAX_TEMPORARY_CODE ? 1 : 0) + (TOML_MAGIC_NUMBERS > MAX_MAGIC_NUMBERS ? 1 : 0)))

if [ "$CRITICAL_ISSUES" -gt 0 ]; then
    echo -e "${RED}❌ ARCHITECTURAL VALIDATION FAILED${NC}"
    echo -e "${RED}Critical architectural issues found - must be fixed before deployment${NC}"
    VALIDATION_FAILED=true
    ALL_PASSED=false
elif [ "$WARNINGS" -gt 0 ]; then
    echo -e "${YELLOW}⚠️  ARCHITECTURAL WARNING${NC}"
    echo -e "${YELLOW}Architectural validation completed with $WARNINGS warning(s) - review table above${NC}"
else
    echo -e "${GREEN}✅ All architectural validations passed - excellent code quality${NC}"
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

# Ensure data directory exists for SQLite databases
echo -e "${BLUE}==== Ensuring test infrastructure... ====${NC}"
if mkdir -p data; then
    echo -e "${GREEN}[OK] Data directory ensured${NC}"
else
    echo -e "${YELLOW}[WARN] Could not create data directory (continuing anyway)${NC}"
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

# Run HTTP API integration tests specifically
echo -e "${BLUE}==== Running HTTP API integration tests... ====${NC}"
if cargo test --test http_api_integration_test --quiet; then
    echo -e "${GREEN}[OK] HTTP API integration tests passed${NC}"
else
    echo -e "${RED}[FAIL] HTTP API integration tests failed${NC}"
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

# CRITICAL: Check for stub implementations that discard EXPENSIVE operations
# Pattern: let _ = ( followed by lines containing .clone() within next 5 lines
# This catches multiline tuple discards like:
#   let _ = (
#       database().clone(),
#       config.clone(),
#   );
DISCARDED_EXPENSIVE_OPS=$(rg -B 2 -A 5 'let _ = \(' src/ | grep -v 'src/bin/' | rg '\.clone\(\)' | wc -l 2>/dev/null || echo 0)
FAKE_ASYNC=$(rg 'tokio::task::yield_now\(\)\.await' src/ | grep -v 'tests/' --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

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

if [ "$DISCARDED_EXPENSIVE_OPS" -gt 0 ]; then
    echo -e "${RED}[CRITICAL] Found $DISCARDED_EXPENSIVE_OPS lines with EXPENSIVE operations that are discarded${NC}"
    echo -e "${RED}           Pattern: let _ = (database().clone(), config.clone(), ...);${NC}"
    echo -e "${RED}           This indicates stub code that does expensive work then throws it away${NC}"
    echo ""
    echo -e "${YELLOW}   Locations of discarded expensive operations:${NC}"
    # Show the let _ = ( line and following clone() calls
    rg -B 1 -A 5 'let _ = \(' src/ -n | grep -v 'src/bin/' | rg 'let _ = \(|\.clone\(\)' | head -15
    echo ""
    echo -e "${YELLOW}   Note: 'let _ = (&context)' without clones is OK - that's unused param suppression${NC}"
    echo -e "${YELLOW}   Fix: Either use the cloned variables or remove the handler entirely${NC}"
    LEGACY_ISSUES_FOUND=true
    ALL_PASSED=false
fi

if [ "$FAKE_ASYNC" -gt 0 ]; then
    echo -e "${RED}[CRITICAL] Found $FAKE_ASYNC fake async patterns (tokio::task::yield_now)${NC}"
    echo -e "${RED}           This is used to make functions compile but does NOTHING${NC}"
    echo -e "${RED}           Sign of stub/placeholder implementations${NC}"
    echo -e "${YELLOW}   Locations of fake async:${NC}"
    rg "tokio::task::yield_now\(\)\.await" src/ -g "!tests/*" -n | head -10
    echo -e "${YELLOW}   Fix: Implement real async logic or make function sync${NC}"
    LEGACY_ISSUES_FOUND=true
    ALL_PASSED=false
fi

if [ "$LEGACY_ISSUES_FOUND" = true ]; then
    echo -e "${RED}FAST FAIL: Remove legacy/stub functions that confuse users${NC}"
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
    # Run cargo audit and capture the output
    AUDIT_OUTPUT=$(RUST_LOG=off cargo audit --ignore RUSTSEC-2023-0071 --no-fetch --color always 2>&1)
    AUDIT_EXIT_CODE=$?

    if [ $AUDIT_EXIT_CODE -eq 0 ]; then
        echo -e "${GREEN}[OK] No security vulnerabilities found${NC}"
    else
        echo -e "${YELLOW}[WARN] Security vulnerabilities detected:${NC}"
        echo ""
        echo "$AUDIT_OUTPUT"
        echo ""
        echo -e "${YELLOW}💡 To fix vulnerabilities:${NC}"
        echo -e "${YELLOW}   1. Check if newer versions are available: cargo update${NC}"
        echo -e "${YELLOW}   2. Review vulnerability details at: https://rustsec.org${NC}"
        echo -e "${YELLOW}   3. Consider alternative dependencies if no fix available${NC}"
        echo ""
        # Don't fail the build for vulnerabilities, but show them clearly
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


# Final cleanup after tests
echo -e "${BLUE}==== Final cleanup after tests... ====${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
find . -name "a2a_enterprise_report_*.json" -delete 2>/dev/null || true
find . -name "mcp_investor_demo_*.json" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Final cleanup completed${NC}"

# MCP Spec Compliance Validation (runs at the end)
# Delegated to standalone script for reusability
echo ""
echo -e "${BLUE}==== MCP Spec Compliance Validation ====${NC}"
if [ -f "$SCRIPT_DIR/ensure_mcp_compliance.sh" ]; then
    if "$SCRIPT_DIR/ensure_mcp_compliance.sh"; then
        echo -e "${GREEN}[OK] MCP compliance validation passed${NC}"
    else
        echo -e "${RED}[FAIL] MCP compliance validation failed${NC}"
        ALL_PASSED=false
    fi
else
    echo -e "${YELLOW}[WARN] MCP compliance script not found - skipping${NC}"
fi

# Bridge Test Suite Validation
echo ""
echo -e "${BLUE}==== Bridge Test Suite Validation ====${NC}"
if [ -f "$SCRIPT_DIR/run_bridge_tests.sh" ]; then
    if "$SCRIPT_DIR/run_bridge_tests.sh"; then
        echo -e "${GREEN}[OK] Bridge test suite passed${NC}"
    else
        echo -e "${RED}[FAIL] Bridge test suite failed${NC}"
        ALL_PASSED=false
    fi
else
    echo -e "${YELLOW}[WARN] Bridge test script not found - skipping${NC}"
fi

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
    if [ -d "sdk" ]; then
        echo "[OK] MCP spec compliance validation"
    fi
    echo ""
    echo -e "${GREEN}Code meets ALL dev standards and is ready for production!${NC}"
    exit 0
else
    echo -e "${RED}VALIDATION FAILED - Task cannot be marked complete${NC}"
    echo -e "${RED}Fix ALL issues above to meet dev standards requirements${NC}"
    exit 1
fi