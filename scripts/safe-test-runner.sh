#!/usr/bin/env bash
# ABOUTME: Safe incremental test runner for Claude Code Web to prevent OOM crashes
# ABOUTME: Runs tests in small batches with memory cleanup pauses

set -euo pipefail

# Configuration
BATCH_SIZE=5
PAUSE_SECONDS=2
LOG_DIR="test-results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
SUMMARY_FILE="${LOG_DIR}/summary_${TIMESTAMP}.txt"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Create log directory
mkdir -p "${LOG_DIR}"

# Get all test files (excluding helpers and disabled tests)
mapfile -t TEST_FILES < <(find tests -name "*.rs" -not -name "*.disabled" -not -path "*/helpers/*" | sort)

TOTAL_TESTS=${#TEST_FILES[@]}
echo "Found ${TOTAL_TESTS} test files to run"
echo "Batch size: ${BATCH_SIZE}, Pause: ${PAUSE_SECONDS}s between batches"
echo "Results will be saved to: ${LOG_DIR}"
echo ""

# Initialize counters
PASSED=0
FAILED=0
CURRENT=0

# Initialize summary file
{
    echo "Pierre MCP Server - Test Execution Summary"
    echo "=========================================="
    echo "Started: $(date)"
    echo "Total test files: ${TOTAL_TESTS}"
    echo ""
} > "${SUMMARY_FILE}"

# Function to extract test name from file path
get_test_name() {
    basename "$1" .rs
}

# Function to run a single test
run_test() {
    local test_file=$1
    local test_name
    test_name=$(get_test_name "${test_file}")

    local log_file="${LOG_DIR}/${test_name}_${TIMESTAMP}.log"

    echo -n "[$((CURRENT + 1))/${TOTAL_TESTS}] Running ${test_name}... "

    # Run the test and capture output
    if cargo test --test "${test_name}" --quiet -- --test-threads=1 > "${log_file}" 2>&1; then
        echo -e "${GREEN}PASSED${NC}"
        echo "✓ ${test_name}" >> "${SUMMARY_FILE}"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}FAILED${NC}"
        echo "✗ ${test_name}" >> "${SUMMARY_FILE}"
        ((FAILED++))

        # Append failure details to summary
        echo "  Error details:" >> "${SUMMARY_FILE}"
        tail -20 "${log_file}" | sed 's/^/    /' >> "${SUMMARY_FILE}"
        echo "" >> "${SUMMARY_FILE}"
        return 1
    fi
}

# Main execution loop
echo "Starting test execution..."
echo ""

for test_file in "${TEST_FILES[@]}"; do
    run_test "${test_file}"
    ((CURRENT++))

    # Pause between batches
    if (( CURRENT % BATCH_SIZE == 0 )) && (( CURRENT < TOTAL_TESTS )); then
        echo -e "${YELLOW}Batch completed. Pausing ${PAUSE_SECONDS}s for memory cleanup...${NC}"
        sleep "${PAUSE_SECONDS}"
        echo ""
    fi
done

# Final summary
{
    echo ""
    echo "=========================================="
    echo "Completed: $(date)"
    echo "Total: ${TOTAL_TESTS}"
    echo "Passed: ${PASSED}"
    echo "Failed: ${FAILED}"
    echo "Success rate: $(awk "BEGIN {printf \"%.1f\", ($PASSED/$TOTAL_TESTS)*100}")%"
} >> "${SUMMARY_FILE}"

echo ""
echo "=========================================="
echo "Test Execution Complete"
echo "=========================================="
echo -e "Total:   ${TOTAL_TESTS}"
echo -e "Passed:  ${GREEN}${PASSED}${NC}"
echo -e "Failed:  ${RED}${FAILED}${NC}"
echo ""
echo "Full summary: ${SUMMARY_FILE}"

# Exit with error if any tests failed
if (( FAILED > 0 )); then
    exit 1
fi
