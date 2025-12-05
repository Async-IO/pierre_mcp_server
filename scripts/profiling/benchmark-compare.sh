#!/bin/bash
# ABOUTME: Compare benchmark results between runs or branches
# ABOUTME: Uses criterion's built-in comparison functionality
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/../.."

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

ACTION="${1:-}"
BASELINE_NAME="${2:-main}"
BENCH_SUITE="${3:-}"

print_usage() {
    echo "Usage: $0 <action> [baseline_name] [benchmark_suite]"
    echo ""
    echo "Actions:"
    echo "  save      - Save current benchmark as baseline"
    echo "  compare   - Compare against saved baseline"
    echo "  report    - Open HTML report in browser"
    echo "  clean     - Remove saved baselines"
    echo ""
    echo "Arguments:"
    echo "  baseline_name   - Name for the baseline (default: main)"
    echo "  benchmark_suite - Specific suite to run (default: all)"
    echo ""
    echo "Examples:"
    echo "  $0 save main                    # Save baseline named 'main'"
    echo "  $0 compare main                 # Compare current against 'main'"
    echo "  $0 save feature-x intelligence  # Save specific benchmark"
    echo "  $0 report                       # Open HTML report"
    echo ""
    echo "Typical workflow:"
    echo "  1. On main branch:  $0 save main"
    echo "  2. Switch to feature branch, make changes"
    echo "  3. Compare results: $0 compare main"
    echo "  4. View report:     $0 report"
}

save_baseline() {
    local name=$1
    local suite=$2

    echo -e "${GREEN}=== Saving Benchmark Baseline: ${name} ===${NC}"
    echo ""

    cd "$PROJECT_ROOT"

    local cmd="cargo bench"
    if [[ -n "$suite" ]]; then
        cmd="${cmd} --bench ${suite}"
    fi
    cmd="${cmd} -- --save-baseline ${name}"

    echo "Running: ${cmd}"
    echo ""

    eval "$cmd"

    echo ""
    echo -e "${GREEN}Baseline '${name}' saved successfully${NC}"
    echo "Location: target/criterion/"
}

compare_baseline() {
    local name=$1
    local suite=$2

    echo -e "${GREEN}=== Comparing Against Baseline: ${name} ===${NC}"
    echo ""

    cd "$PROJECT_ROOT"

    local cmd="cargo bench"
    if [[ -n "$suite" ]]; then
        cmd="${cmd} --bench ${suite}"
    fi
    cmd="${cmd} -- --baseline ${name}"

    echo "Running: ${cmd}"
    echo ""

    eval "$cmd"

    echo ""
    echo -e "${GREEN}Comparison complete${NC}"
    echo ""
    echo "Legend:"
    echo -e "  ${GREEN}+X.XX%${NC} = regression (slower)"
    echo -e "  ${BLUE}-X.XX%${NC} = improvement (faster)"
    echo ""
    echo "View full report:"
    echo "  $0 report"
}

open_report() {
    local report_path="${PROJECT_ROOT}/target/criterion/report/index.html"

    if [[ ! -f "$report_path" ]]; then
        echo -e "${RED}Error: No benchmark report found${NC}"
        echo "Run benchmarks first: cargo bench"
        exit 1
    fi

    echo -e "${GREEN}Opening benchmark report...${NC}"

    case "$(uname)" in
        Darwin)
            open "$report_path"
            ;;
        Linux)
            if command -v xdg-open &> /dev/null; then
                xdg-open "$report_path"
            else
                echo "Report: file://${report_path}"
            fi
            ;;
        *)
            echo "Report: file://${report_path}"
            ;;
    esac
}

clean_baselines() {
    local baselines_dir="${PROJECT_ROOT}/target/criterion"

    if [[ ! -d "$baselines_dir" ]]; then
        echo "No baselines to clean"
        exit 0
    fi

    echo -e "${YELLOW}This will remove all saved baselines in:${NC}"
    echo "  ${baselines_dir}"
    echo ""
    echo "Continue? (y/N)"
    read -r response

    if [[ "$response" =~ ^[Yy]$ ]]; then
        rm -rf "$baselines_dir"
        echo -e "${GREEN}Baselines cleaned${NC}"
    else
        echo "Cancelled"
    fi
}

# Main
if [[ -z "$ACTION" ]] || [[ "$ACTION" == "-h" ]] || [[ "$ACTION" == "--help" ]]; then
    print_usage
    exit 0
fi

case "$ACTION" in
    save)
        save_baseline "$BASELINE_NAME" "${BENCH_SUITE:-}"
        ;;
    compare)
        compare_baseline "$BASELINE_NAME" "${BENCH_SUITE:-}"
        ;;
    report)
        open_report
        ;;
    clean)
        clean_baselines
        ;;
    *)
        echo -e "${RED}Unknown action: ${ACTION}${NC}"
        print_usage
        exit 1
        ;;
esac
