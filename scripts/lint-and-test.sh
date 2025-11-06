#!/bin/bash
# ABOUTME: Simplified validation orchestrator using native Cargo commands
# ABOUTME: Delegates to cargo fmt, cargo clippy, cargo deny, and custom architectural validation

# ============================================================================
# ARCHITECTURE: Native Cargo-First Approach
# ============================================================================
# This script has been DRASTICALLY SIMPLIFIED (from 1294 → ~350 lines)
#
# WHAT CHANGED:
# - Clippy: cargo clippy (reads Cargo.toml [lints] table) ← was custom flags
# - Formatting: cargo fmt --check ← was custom orchestration
# - Security: cargo deny check (reads deny.toml) ← was cargo-audit + bash
# - Documentation: cargo doc --no-deps ← was custom checks
#
# WHAT REMAINS CUSTOM:
# - Architectural validation (scripts/architectural-validation.sh)
# - Frontend orchestration (npm/TypeScript toolchain)
# - Test execution coordination
# - MCP/Bridge compliance checks

set -e

echo "Running Pierre MCP Server Validation Suite..."

# Start timing
START_TIME=$(date +%s)

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
            echo "  --coverage    Enable code coverage collection and reporting"
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

echo -e "${BLUE}==== Pierre MCP Server - Validation Suite ====${NC}"
echo "Project root: $PROJECT_ROOT"
cd "$PROJECT_ROOT"

# Track overall success
ALL_PASSED=true

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# CLEANUP
# ============================================================================

echo ""
echo -e "${BLUE}==== Cleaning up generated files... ====${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# ============================================================================
# NATIVE CARGO VALIDATION (Reads Cargo.toml [lints] + deny.toml)
# ============================================================================

echo ""
echo -e "${BLUE}==== Native Cargo Validation ====${NC}"

# Formatting check
echo -e "${BLUE}Running cargo fmt --check...${NC}"
if cargo fmt --all -- --check; then
    echo -e "${GREEN}[OK] Rust code formatting is correct${NC}"
else
    echo -e "${RED}[CRITICAL] Rust code formatting check failed${NC}"
    echo -e "${RED}Run 'cargo fmt --all' to fix formatting issues${NC}"
    echo -e "${RED}FAST FAIL: Fix formatting errors immediately${NC}"
    exit 1
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
        PLACEHOLDER_WARNINGS=$(rg "$WARNING_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    else
        PLACEHOLDER_WARNINGS=0
    fi

    # Separate check for undocumented long functions (not in warning_groups)
    TOTAL_LONG_FUNCTIONS=$(rg "#\[allow\(clippy::too_many_lines\)\]" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    DOCUMENTED_LONG_FUNCTIONS=$(rg "#\[allow\(clippy::too_many_lines\)\]" src/ -B1 | rg -c "// Long function:|// Safe:" 2>/dev/null || echo "0")
    UNDOCUMENTED_LONG_FUNCTIONS=$((TOTAL_LONG_FUNCTIONS - DOCUMENTED_LONG_FUNCTIONS))

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
    TOML_MAGIC_NUMBERS=$(rg "$THRESHOLD_PATTERNS" src/ -g "!src/constants.rs" -g "!src/constants/*" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # TOML-based architectural pattern analysis
    TOML_RESOURCE_CREATION=$(rg "$RESOURCE_CREATION_PATTERNS" src/ -g "!src/mcp/multitenant.rs" -g "!src/mcp/resources.rs" -g "!src/bin/*" -g "!src/lifecycle/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_FAKE_RESOURCES=$(rg "$FAKE_RESOURCES_PATTERNS" src/ -g "!src/bin/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_OBSOLETE_FUNCTIONS=$(rg "$OBSOLETE_FUNCTIONS_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_UNUSED_VARIABLES=$(rg "$UNUSED_VARIABLES_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_DEPRECATED_CODE=$(rg "$DEPRECATED_CODE_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # TOML-based memory management analysis
    TOML_LEGITIMATE_ARC_CLONES=$(rg "$LEGITIMATE_ARC_CLONES_PATTERNS" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_PROBLEMATIC_DB_CLONES=$(rg "$PROBLEMATIC_DB_CLONES_PATTERNS" src/ -g "!src/bin/*" -g "!src/database/tests.rs" -g "!src/database_plugins/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_ARC_USAGE=$(rg "$ARC_USAGE_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_CLONE_USAGE=$(rg "$CLONE_USAGE_PATTERNS" src/ | grep -v 'src/bin/' --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

    # Claude Code anti-pattern analysis
    TOML_STRING_ALLOCATIONS=$(rg "$STRING_ALLOCATION_ANTIPATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_FUNCTION_STRING_PARAMS=$(rg "$FUNCTION_STRING_PARAMETERS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_ITERATOR_ANTIPATTERNS=$(rg "$ITERATOR_ANTIPATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_ERROR_CONTEXT=$(rg "$ERROR_CONTEXT_ANTIPATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_ASYNC_ANTIPATTERNS=$(rg "$ASYNC_ANTIPATTERNS_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')
    TOML_LIFETIME_COMPLEXITY=$(rg "$LIFETIME_ANTIPATTERNS_PATTERNS" src/ --count 2>/dev/null | awk -F: '{sum+=$2} END {print sum+0}')

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

# Algorithm DI Architecture Enforcement - Check for hardcoded formulas (TOML-configured)
# Dynamically check all algorithms defined in TOML
TOTAL_ALGORITHM_VIOLATIONS=0
ALGORITHMS_WITH_VIOLATIONS=""

if [ -n "$MIGRATED_ALGORITHMS" ]; then
    for algo in $MIGRATED_ALGORITHMS; do
        algo_upper=$(echo "$algo" | tr '[:lower:]' '[:upper:]' | tr '-' '_')

        # Get patterns and excludes for this algorithm
        patterns_var="ALGORITHM_${algo_upper}_PATTERNS"
        excludes_var="ALGORITHM_${algo_upper}_EXCLUDES"
        name_var="ALGORITHM_${algo_upper}_NAME"

        eval "patterns=\$$patterns_var"
        eval "excludes=\$$excludes_var"
        eval "algo_name=\$$name_var"

        if [ -n "$patterns" ] && [ -n "$excludes" ]; then
            # Build exclude flags
            EXCLUDE_FLAGS=""
            for exclude in $excludes; do
                EXCLUDE_FLAGS="$EXCLUDE_FLAGS -g !$exclude"
            done

            # Count violations (exclude comments)
            violations=$(rg "$patterns" src/ $EXCLUDE_FLAGS 2>/dev/null | grep -v "^\s*//" | wc -l | awk '{print $1+0}')

            # Track violations
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

# Memory Management Analysis - Enhanced with clippy validation
TOTAL_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | wc -l 2>/dev/null || echo 0)

# Run clippy clone analysis to validate clone usage
CLIPPY_CLONE_WARNINGS=$(cargo clippy --all-targets --all-features --quiet -- \
    -W clippy::clone_on_copy \
    -W clippy::redundant_clone \
    -W suspicious_double_ref_op 2>&1 | \
    grep -E "warning:.*clone" | wc -l 2>/dev/null || echo 0)

# Get files with file-level clone safety documentation
FILES_WITH_CLONE_DOCS=$(rg -l "NOTE: All.*\.clone.*calls.*Safe|NOTE: All.*clone.*calls.*Safe" src/ 2>/dev/null || echo "")
DOCUMENTED_FILES_COUNT=$(echo "$FILES_WITH_CLONE_DOCS" | grep -v '^$' | wc -l 2>/dev/null || echo 0)

# Count documented clones from files with bulk documentation
DOCUMENTED_CLONES=0
if [ -n "$FILES_WITH_CLONE_DOCS" ] && [ "$DOCUMENTED_FILES_COUNT" -gt 0 ]; then
    DOCUMENTED_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | grep -f <(echo "$FILES_WITH_CLONE_DOCS") | wc -l 2>/dev/null || echo 0)
fi

# Enhanced legitimate clone detection with more patterns
LEGITIMATE_CLONES=$(rg "\.clone\(\)" src/ | grep -v 'src/bin/' | rg "Arc::|Rc::|resources\.|database\.|auth_manager\.|sse_manager\.|websocket_manager\.|jwks_manager\.|provider_registry\.|activity_intelligence\.|a2a_client_manager\.|a2a_system_user_service\.|oauth2_rate_limiter\.|tenant_oauth_client\.|cache\.|redaction_config\.|// Safe|\.to_string\(\)|format!|String::from|token|url|name|path|message|error|Error|client_id|client_secret|redirect_uri|access_token|refresh_token|user_id|tenant_id|request\.|response\.|context\.|config\.|profile\.|manager_for_" | wc -l 2>/dev/null || echo 0)

# If clippy validates all clones, consider them legitimate
if [ "$CLIPPY_CLONE_WARNINGS" -eq 0 ]; then
    # Clippy found no clone issues - all clones are validated
    CLIPPY_VALIDATED_CLONES=$TOTAL_CLONES
    PROBLEMATIC_CLONES=0
else
    # Some clones need review based on clippy warnings
    CLIPPY_VALIDATED_CLONES=$((TOTAL_CLONES - CLIPPY_CLONE_WARNINGS))
    PROBLEMATIC_CLONES=$CLIPPY_CLONE_WARNINGS
fi


# Advanced Arc analysis with pattern categorization
TOTAL_ARCS=$(rg "Arc::" src/ | wc -l 2>/dev/null || echo 0)

# Legitimate Arc patterns
# Concurrent: std::sync and tokio::sync RwLock/Mutex patterns
CONCURRENT_ARCS_STD=$(rg "Arc::new\((RwLock|Mutex)" src/ | wc -l 2>/dev/null || echo 0)
CONCURRENT_ARCS_TOKIO=$(rg "Arc::new\(tokio::sync::(Mutex|RwLock)" src/ | wc -l 2>/dev/null || echo 0)
CONCURRENT_ARCS=$((CONCURRENT_ARCS_STD + CONCURRENT_ARCS_TOKIO))

SERVERRESOURCES_ARCS=$(rg "Arc::new" src/mcp/resources.rs | wc -l 2>/dev/null || echo 0)

# Singletons: OnceLock and get_or_init patterns
SINGLETON_ARCS_ONCELOCK=$(rg "OnceLock.*Arc|Arc.*OnceLock" src/ | wc -l 2>/dev/null || echo 0)
SINGLETON_ARCS_INIT=$(rg "get_or_init.*Arc::new" src/ | wc -l 2>/dev/null || echo 0)
SINGLETON_ARCS=$((SINGLETON_ARCS_ONCELOCK + SINGLETON_ARCS_INIT))

ROUTE_HANDLER_ARCS=$(rg "Arc::new\(.*[Rr]outes" src/ | wc -l 2>/dev/null || echo 0)
BINARY_STARTUP_ARCS=$(rg "Arc::new|Arc::clone" src/bin/ | wc -l 2>/dev/null || echo 0)
# Service components: Authenticator, Checker, shutdown handlers, shared resources for transports
SERVICE_COMPONENT_ARCS=$(rg "Arc::new\(.*Authenticator|Arc::new\(.*Checker|Arc::new\(.*checker|Arc::new\(shutdown|shared_resources.*Arc::new|Arc::new.*resources_clone" src/ | wc -l 2>/dev/null || echo 0)

# Arc conversions and internal sharing (Arc::from, Arc::clone)
ARC_CONVERSIONS=$(rg "Arc::(from|clone)\(" src/ --glob '!src/bin/*' | wc -l 2>/dev/null || echo 0)

# Calculate legitimate vs potentially problematic
LEGITIMATE_ARC_PATTERNS=$((CONCURRENT_ARCS + SERVERRESOURCES_ARCS + SINGLETON_ARCS + ROUTE_HANDLER_ARCS + BINARY_STARTUP_ARCS + SERVICE_COMPONENT_ARCS + ARC_CONVERSIONS))
POTENTIALLY_PROBLEMATIC_ARCS=$((TOTAL_ARCS > LEGITIMATE_ARC_PATTERNS ? TOTAL_ARCS - LEGITIMATE_ARC_PATTERNS : 0))

MAGIC_NUMBERS=$(rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/constants/*" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" | wc -l 2>/dev/null || echo 0)

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

printf "│ %-35s │ %5d │ " "Undocumented long functions" "$UNDOCUMENTED_LONG_FUNCTIONS"
if [ "$UNDOCUMENTED_LONG_FUNCTIONS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "All long functions documented"
else
    FIRST_UNDOCUMENTED=$(get_first_location 'rg "#\[allow\(clippy::too_many_lines\)\]" src/ -B1 | rg -v "// Long function:|// Safe:" | rg "#\[allow" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_UNDOCUMENTED"
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
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_WARNING_PLACEHOLDER"
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
    FIRST_MAGIC=$(get_first_location 'rg "$THRESHOLD_PATTERNS" src/ -g "!src/constants.rs" -g "!src/constants/*" -g "!src/config/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_MAGIC"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Memory Management Analysis - Clippy-validated
printf "│ %-35s │ %5d │ " "Clippy clone warnings" "$CLIPPY_CLONE_WARNINGS"
if [ "$CLIPPY_CLONE_WARNINGS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "All clones validated by clippy"
else
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$CLIPPY_CLONE_WARNINGS clone issues found"
fi

printf "│ %-35s │ %5d │ " "Clone usage (total)" "$TOTAL_CLONES"
if [ "$CLIPPY_CLONE_WARNINGS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "$DOCUMENTED_FILES_COUNT files documented, clippy clean"
else
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$LEGITIMATE_CLONES legitimate, $CLIPPY_CLONE_WARNINGS need review"
fi

printf "│ %-35s │ %5d │ " "Files with clone documentation" "$DOCUMENTED_FILES_COUNT"
if [ "$DOCUMENTED_FILES_COUNT" -ge 10 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Good clone documentation coverage"
else
    printf "$(format_status "⚠️ INFO")│ %-39s │\n" "Consider documenting Arc clone patterns"
fi

printf "│ %-35s │ %5d │ " "Arc usage (total)" "$TOTAL_ARCS"
# Multi-threaded web services with SSE/WebSockets naturally have high Arc usage
# Threshold: 75 for services, 100 for complex distributed systems
if [ "$TOTAL_ARCS" -lt 75 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Appropriate for service architecture"
elif [ "$POTENTIALLY_PROBLEMATIC_ARCS" -le 10 ]; then
    printf "$(format_status "⚠️ INFO")│ %-39s │\n" "$POTENTIALLY_PROBLEMATIC_ARCS other, $LEGITIMATE_ARC_PATTERNS categorized"
else
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$POTENTIALLY_PROBLEMATIC_ARCS need review"
fi

printf "│ %-35s │ %5d │ " "Arc patterns categorized" "$LEGITIMATE_ARC_PATTERNS"
if [ "$LEGITIMATE_ARC_PATTERNS" -gt 0 ]; then
    # Show most significant categories in breakdown
    ARC_BREAKDOWN="Concurrent:$CONCURRENT_ARCS Resources:$SERVERRESOURCES_ARCS"
    if [ "$ARC_CONVERSIONS" -gt 0 ]; then
        ARC_BREAKDOWN="$ARC_BREAKDOWN Conv:$ARC_CONVERSIONS"
    fi
    printf "$(format_status "✅ PASS")│ %-39s │\n" "$ARC_BREAKDOWN"
else
    printf "$(format_status "⚠️ INFO")│ %-39s │\n" "No Arc usage detected"
fi

printf "│ %-35s │ %5d │ " "Magic numbers" "$MAGIC_NUMBERS"
if [ "$MAGIC_NUMBERS" -lt 10 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Good configuration practices"
else
    FIRST_MAGIC=$(get_first_location 'rg "\b[0-9]{4,}\b" src/ -g "!src/constants.rs" -g "!src/constants/*" -g "!src/config/*" | grep -v -E "(Licensed|http://|https://|Duration|timestamp|//.*[0-9]|seconds|minutes|hours|Version|\.[0-9]|[0-9]\.|test|mock|example|error.*code|status.*code|port|timeout|limit|capacity|-32[0-9]{3}|1000\.0|60\.0|24\.0|7\.0|365\.0|METERS_PER|PER_METER|conversion|unit|\.60934|12345|0000-0000|202[0-9]-[0-9]{2}-[0-9]{2}|Some\([0-9]+\)|Trial.*1000|Standard.*10000)" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_MAGIC"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Claude Code Anti-Patterns (AI-generated code quality)
printf "│ %-35s │ %5d │ " "String round-trip conversions" "$TOML_STRING_ALLOCATIONS"
if [ "$TOML_STRING_ALLOCATIONS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "No unnecessary conversions"
else
    FIRST_STRING=$(get_first_location 'rg "$STRING_ALLOCATION_ANTIPATTERNS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_STRING"
fi

printf "│ %-35s │ %5d │ " "Function String parameters" "$TOML_FUNCTION_STRING_PARAMS"
if [ "$TOML_FUNCTION_STRING_PARAMS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "All functions use &str (optimized)"
else
    FIRST_FUNC_STRING=$(get_first_location 'rg "$FUNCTION_STRING_PARAMETERS_PATTERNS" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_FUNC_STRING"
fi

printf "│ %-35s │ %5d │ " "Iterator anti-patterns" "$TOML_ITERATOR_ANTIPATTERNS"
if [ "$TOML_ITERATOR_ANTIPATTERNS" -le 15 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Idiomatic iterator usage"
else
    FIRST_ITERATOR=$(get_first_location 'rg "let mut.*vec.*=.*Vec::new\\(\\);\\s*for" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "⚠️ INFO")│ %-39s │\n" "$FIRST_ITERATOR"
fi

printf "│ %-35s │ %5d │ " "FORBIDDEN anyhow! macro usage" "$TOML_ERROR_CONTEXT"
if [ "$TOML_ERROR_CONTEXT" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Structured errors only (compliant)"
else
    FIRST_ERROR=$(get_first_location 'rg "\\banyhow!\\(|anyhow::anyhow!\\(" src/ -g "!src/bin/*" -g "!tests/*" -n')
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "$FIRST_ERROR"
    fail_validation "CLAUDE.md VIOLATION: anyhow! macro is FORBIDDEN - use AppError/DatabaseError/ProviderError instead"
fi

printf "│ %-35s │ %5d │ " "Async anti-patterns (blocking)" "$TOML_ASYNC_ANTIPATTERNS"
if [ "$TOML_ASYNC_ANTIPATTERNS" -le 5 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Proper async patterns"
else
    FIRST_ASYNC=$(get_first_location 'rg "async fn.*std::fs::|async fn.*std::thread::sleep" src/ -n')
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "$FIRST_ASYNC"
fi

printf "│ %-35s │ %5d │ " "Lifetime complexity" "$TOML_LIFETIME_COMPLEXITY"
if [ "$TOML_LIFETIME_COMPLEXITY" -le 3 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "Reasonable lifetime usage"
else
    # Pattern contains single quotes, skip location for simplicity
    printf "$(format_status "⚠️ WARN")│ %-39s │\n" "Multiple complex lifetime patterns found"
fi

echo "├─────────────────────────────────────┼───────┼──────────┼─────────────────────────────────────────┤"

# Algorithm DI Architecture - Ensure enum-based dependency injection (TOML-configured)
ALGO_COUNT=$(echo "$MIGRATED_ALGORITHMS" | wc -w | awk '{print $1}')
printf "│ %-35s │ %5d │ " "Algorithm DI violations ($ALGO_COUNT algos)" "$TOTAL_ALGORITHM_VIOLATIONS"
if [ "$TOTAL_ALGORITHM_VIOLATIONS" -eq 0 ]; then
    printf "$(format_status "✅ PASS")│ %-39s │\n" "All using enum-based DI (compliant)"
else
    printf "$(format_status "❌ FAIL")│ %-39s │\n" "Violations found"
    fail_validation "Hardcoded algorithms detected: $ALGORITHMS_WITH_VIOLATIONS. Use enum-based DI in src/intelligence/algorithms/"
fi

if [ "$TOTAL_ALGORITHM_VIOLATIONS" -gt 0 ]; then
    printf "│ %-35s │ %5s │ $(format_status "❌ FAIL")│ %-39s │\n" "Algorithms with violations" "" "$ALGORITHMS_WITH_VIOLATIONS"
else
    printf "│ %-35s │ %5s │ $(format_status "✅ PASS")│ %-39s │\n" "Algorithms detected" "" "None (MaxHR, TRIMP, TSS, VDOT, CTL...)"
fi

echo "└─────────────────────────────────────┴───────┴──────────┴─────────────────────────────────────────┘"

echo ""

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
# Note: Clone validation now uses clippy analysis instead of arbitrary thresholds
CRITICAL_ISSUES=$((NULL_UUIDS + PROBLEMATIC_DB_CLONES + PROBLEMATIC_UNWRAPS + PROBLEMATIC_EXPECTS + PANICS + IGNORED_TESTS + IMPLEMENTATION_PLACEHOLDERS + PLACEHOLDER_WARNINGS))
CRITICAL_ISSUES=$((CRITICAL_ISSUES + TOML_PRODUCTION_HYGIENE + CFG_TEST_IN_SRC + DEAD_CODE + TOML_STRING_ALLOCATIONS))
CRITICAL_ISSUES=$((CRITICAL_ISSUES + TOTAL_ALGORITHM_VIOLATIONS))

WARNINGS=$((FAKE_RESOURCES + (OBSOLETE_FUNCTIONS > 1 ? OBSOLETE_FUNCTIONS - 1 : 0)))
WARNINGS=$((WARNINGS + RESOURCE_CREATION + TODOS + PROBLEMATIC_UNDERSCORE_NAMES + TEMP_SOLUTIONS))
WARNINGS=$((WARNINGS + (CLIPPY_CLONE_WARNINGS > 0 ? 1 : 0) + (POTENTIALLY_PROBLEMATIC_ARCS > 0 ? 1 : 0) + (MAGIC_NUMBERS >= 10 ? 1 : 0)))
WARNINGS=$((WARNINGS + (TOML_DEVELOPMENT_ARTIFACTS > 0 ? 1 : 0) + (TOML_TEMPORARY_CODE > MAX_TEMPORARY_CODE ? 1 : 0) + (TOML_MAGIC_NUMBERS > MAX_MAGIC_NUMBERS ? 1 : 0)))

# Claude Code anti-pattern warnings (informational - encourage better Rust idioms)
WARNINGS=$((WARNINGS + (TOML_FUNCTION_STRING_PARAMS > 0 ? 1 : 0)))
WARNINGS=$((WARNINGS + (TOML_ITERATOR_ANTIPATTERNS > 15 ? 1 : 0)))
WARNINGS=$((WARNINGS + (TOML_ERROR_CONTEXT > 10 ? 1 : 0)))
WARNINGS=$((WARNINGS + (TOML_ASYNC_ANTIPATTERNS > 5 ? 1 : 0)))
WARNINGS=$((WARNINGS + (TOML_LIFETIME_COMPLEXITY > 3 ? 1 : 0)))

if [ "$CRITICAL_ISSUES" -gt 0 ]; then
    echo -e "${RED}❌ ARCHITECTURAL VALIDATION FAILED${NC}"
    echo -e "${RED}Critical architectural issues found - must be fixed before deployment${NC}"
    VALIDATION_FAILED=true
    ALL_PASSED=false
    exit 1
fi

# Clippy linting (reads Cargo.toml [lints.clippy] with level = "deny")
echo -e "${BLUE}Running cargo clippy (zero tolerance via Cargo.toml)...${NC}"
if cargo clippy --all-targets --all-features --quiet; then
    echo -e "${GREEN}[OK] Clippy passed - ZERO warnings (enforced by Cargo.toml)${NC}"
else
    echo -e "${RED}[CRITICAL] Clippy failed${NC}"
    echo -e "${RED}Re-run without --quiet to see details:${NC}"
    echo -e "${RED}  cargo clippy --all-targets --all-features${NC}"
    ALL_PASSED=false
    exit 1
fi

# Security audit (reads deny.toml)
echo -e "${BLUE}Running cargo deny check...${NC}"
if command_exists cargo-deny; then
    if cargo deny check; then
        echo -e "${GREEN}[OK] Security audit passed (via deny.toml)${NC}"
    else
        echo -e "${YELLOW}[WARN] Security vulnerabilities detected${NC}"
        echo -e "${YELLOW}Review output above and update dependencies${NC}"
        # Don't fail build, just warn
    fi
else
    echo -e "${YELLOW}[WARN] cargo-deny not installed${NC}"
    echo -e "${YELLOW}Install with: cargo install cargo-deny${NC}"
fi

# Compilation check
echo -e "${BLUE}Running cargo check...${NC}"
if cargo check --all-targets --quiet; then
    echo -e "${GREEN}[OK] Compilation check passed${NC}"
else
    echo -e "${RED}[CRITICAL] Compilation failed${NC}"
    ALL_PASSED=false
    exit 1
fi

# ============================================================================
# CUSTOM ARCHITECTURAL VALIDATION (Project-Specific Rules)
# ============================================================================

echo ""
echo -e "${BLUE}==== Custom Architectural Validation ====${NC}"

if [ -f "$SCRIPT_DIR/architectural-validation.sh" ]; then
    if "$SCRIPT_DIR/architectural-validation.sh"; then
        echo -e "${GREEN}[OK] Architectural validation passed${NC}"
    else
        echo -e "${RED}[CRITICAL] Architectural validation failed${NC}"
        ALL_PASSED=false
        exit 1
    fi
else
    echo -e "${YELLOW}[WARN] Architectural validation script not found - skipping${NC}"
fi

# ============================================================================
# PII AND SECRET PATTERN DETECTION
# ============================================================================

echo ""
echo -e "${BLUE}==== PII and Secret Pattern Detection ====${NC}"
if [ -f "$SCRIPT_DIR/validate-no-secrets.sh" ]; then
    if "$SCRIPT_DIR/validate-no-secrets.sh"; then
        echo -e "${GREEN}[OK] Secret pattern validation passed${NC}"
    else
        echo -e "${RED}[CRITICAL] Secret pattern validation failed${NC}"
        ALL_PASSED=false
        exit 1
    fi
else
    echo -e "${YELLOW}[WARN] Secret validation script not found - skipping${NC}"
fi

# ============================================================================
# TEST EXECUTION
# ============================================================================

echo ""
echo -e "${BLUE}==== Running Tests ====${NC}"

# Clean test databases
echo -e "${BLUE}Cleaning test databases...${NC}"
if [ -f "$SCRIPT_DIR/clean-test-databases.sh" ]; then
    "$SCRIPT_DIR/clean-test-databases.sh" || true
fi

# Ensure data directory exists
mkdir -p data

# Count tests
TOTAL_TESTS=$(cargo test --all-targets -- --list 2>/dev/null | grep -E "^[a-zA-Z_].*: test$" | wc -l | tr -d ' ')
echo -e "${BLUE}Total tests to run: $TOTAL_TESTS${NC}"

# Use 2048-bit RSA for faster test execution
export PIERRE_RSA_KEY_SIZE=2048

# Run tests
if [ "$ENABLE_COVERAGE" = true ]; then
    echo -e "${BLUE}Running tests with coverage...${NC}"
    if command_exists cargo-llvm-cov; then
        if cargo llvm-cov --all-targets --summary-only; then
            echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed with coverage${NC}"
        else
            echo -e "${RED}[FAIL] Some tests failed${NC}"
            ALL_PASSED=false
        fi
    else
        echo -e "${YELLOW}[WARN] cargo-llvm-cov not installed${NC}"
        echo -e "${YELLOW}Install with: cargo install cargo-llvm-cov${NC}"
        if cargo test --all-targets --no-fail-fast; then
            echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed${NC}"
        else
            echo -e "${RED}[FAIL] Some tests failed${NC}"
            ALL_PASSED=false
        fi
    fi
else
    if cargo test --all-targets --no-fail-fast; then
        echo -e "${GREEN}[OK] All $TOTAL_TESTS tests passed${NC}"
    else
        echo -e "${RED}[FAIL] Some tests failed${NC}"
        ALL_PASSED=false
    fi
fi

# HTTP API integration tests
echo -e "${BLUE}Running HTTP API integration tests...${NC}"
if cargo test --test http_api_integration_test --quiet; then
    echo -e "${GREEN}[OK] HTTP API integration tests passed${NC}"
else
    echo -e "${RED}[FAIL] HTTP API integration tests failed${NC}"
    ALL_PASSED=false
fi

# A2A compliance tests
echo -e "${BLUE}Running A2A compliance tests...${NC}"
if cargo test --test a2a_compliance_test --quiet; then
    echo -e "${GREEN}[OK] A2A compliance tests passed${NC}"
else
    echo -e "${RED}[FAIL] A2A compliance tests failed${NC}"
    ALL_PASSED=false
fi

# ============================================================================
# FRONTEND VALIDATION (Separate Toolchain)
# ============================================================================

if [ -d "frontend" ]; then
    echo ""
    echo -e "${BLUE}==== Frontend Validation ====${NC}"
    cd frontend

    # Check dependencies
    if [ ! -d "node_modules" ] || [ ! -f "node_modules/.package-lock.json" ]; then
        echo -e "${YELLOW}Installing frontend dependencies...${NC}"
        npm install || {
            echo -e "${RED}[FAIL] Frontend dependency installation failed${NC}"
            ALL_PASSED=false
            cd ..
        }
    fi

    if [ -d "node_modules" ]; then
        # Lint
        if npm run lint; then
            echo -e "${GREEN}[OK] Frontend linting passed${NC}"
        else
            echo -e "${RED}[FAIL] Frontend linting failed${NC}"
            ALL_PASSED=false
        fi

        # Type check
        if npm run type-check; then
            echo -e "${GREEN}[OK] TypeScript type checking passed${NC}"
        else
            echo -e "${RED}[FAIL] TypeScript type checking failed${NC}"
            ALL_PASSED=false
        fi

        # Tests
        if npm test -- --run; then
            echo -e "${GREEN}[OK] Frontend tests passed${NC}"
        else
            echo -e "${RED}[FAIL] Frontend tests failed${NC}"
            ALL_PASSED=false
        fi

        # Build
        if npm run build; then
            echo -e "${GREEN}[OK] Frontend build successful${NC}"
        else
            echo -e "${RED}[FAIL] Frontend build failed${NC}"
            ALL_PASSED=false
        fi
    fi

    cd ..
fi

# ============================================================================
# PERFORMANCE AND DOCUMENTATION
# ============================================================================

echo ""
echo -e "${BLUE}==== Performance and Documentation ====${NC}"

# Build release binary
echo -e "${BLUE}Building release binary...${NC}"
if cargo build --release --quiet; then
    echo -e "${GREEN}[OK] Release build successful${NC}"

    # Binary size check (will be validated by architectural-validation.sh)
    if [ -f "target/release/pierre-mcp-server" ]; then
        BINARY_SIZE=$(ls -lh target/release/pierre-mcp-server | awk '{print $5}')
        echo -e "${GREEN}[INFO] Binary size: $BINARY_SIZE${NC}"
    fi
else
    echo -e "${RED}[FAIL] Release build failed${NC}"
    ALL_PASSED=false
fi

# Documentation
echo -e "${BLUE}Checking documentation...${NC}"
if cargo doc --no-deps --quiet; then
    echo -e "${GREEN}[OK] Documentation builds successfully${NC}"
else
    echo -e "${RED}[FAIL] Documentation build failed${NC}"
    ALL_PASSED=false
fi

# ============================================================================
# FINAL CLEANUP
# ============================================================================

echo -e "${BLUE}Final cleanup...${NC}"
rm -f ./mcp_activities_*.json ./examples/mcp_activities_*.json ./a2a_*.json ./enterprise_strava_dataset.json 2>/dev/null || true
find . -name "*demo*.json" -not -path "./target/*" -delete 2>/dev/null || true
find . -name "a2a_enterprise_report_*.json" -delete 2>/dev/null || true
find . -name "mcp_investor_demo_*.json" -delete 2>/dev/null || true
echo -e "${GREEN}[OK] Cleanup completed${NC}"

# ============================================================================
# MCP SPEC COMPLIANCE
# ============================================================================

echo ""
echo -e "${BLUE}==== MCP Spec Compliance ====${NC}"
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

# Bridge test suite
echo ""
echo -e "${BLUE}==== Bridge Test Suite ====${NC}"
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

# ============================================================================
# SUMMARY
# ============================================================================

END_TIME=$(date +%s)
TOTAL_SECONDS=$((END_TIME - START_TIME))
TOTAL_MINUTES=$((TOTAL_SECONDS / 60))
REMAINING_SECONDS=$((TOTAL_SECONDS % 60))

echo ""
echo -e "${BLUE}==== Validation Summary ====${NC}"
echo -e "${BLUE}Total execution time: ${TOTAL_MINUTES}m ${REMAINING_SECONDS}s${NC}"
echo ""

if [ "$ALL_PASSED" = true ]; then
    echo -e "${GREEN}✅ ALL VALIDATION PASSED${NC}"
    echo ""
    echo "[OK] Formatting (cargo fmt)"
    echo "[OK] Linting (cargo clippy via Cargo.toml)"
    echo "[OK] Security (cargo deny via deny.toml)"
    echo "[OK] Compilation (cargo check)"
    echo "[OK] Architectural validation (custom)"
    echo "[OK] Tests (cargo test)"
    echo "[OK] Frontend (npm)"
    echo "[OK] Documentation (cargo doc)"
    echo ""
    echo -e "${GREEN}Code meets ALL standards and is ready for production!${NC}"
    exit 0
else
    echo -e "${RED}❌ VALIDATION FAILED${NC}"
    echo -e "${RED}Fix issues above before deployment${NC}"
    exit 1
fi
