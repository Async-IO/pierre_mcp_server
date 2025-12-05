#!/bin/bash
# ABOUTME: Generate flamegraphs for CPU profiling of benchmarks
# ABOUTME: Requires cargo-flamegraph (cargo install flamegraph)
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/../.."
OUTPUT_DIR="${PROJECT_ROOT}/target/profiling"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Default values
BENCH_NAME="${1:-intelligence_bench}"
BENCH_FILTER="${2:-}"

print_usage() {
    echo "Usage: $0 [benchmark_name] [benchmark_filter]"
    echo ""
    echo "Arguments:"
    echo "  benchmark_name   - Name of benchmark suite (default: intelligence_bench)"
    echo "  benchmark_filter - Optional filter for specific benchmark function"
    echo ""
    echo "Available benchmark suites:"
    echo "  intelligence_bench  - Training load, recovery, nutrition calculations"
    echo "  cache_bench         - Cache operations (memory backend)"
    echo "  database_bench      - Database query performance"
    echo "  serialization_bench - JSON serialization/deserialization"
    echo ""
    echo "Examples:"
    echo "  $0 intelligence_bench"
    echo "  $0 intelligence_bench training_load"
    echo "  $0 cache_bench cache_set"
    echo ""
    echo "Output: target/profiling/<benchmark_name>.svg"
}

check_deps() {
    if ! command -v cargo-flamegraph &> /dev/null; then
        echo -e "${RED}Error: cargo-flamegraph not installed${NC}"
        echo ""
        echo "Install with:"
        echo "  cargo install flamegraph"
        echo ""
        echo "On Linux, you may also need:"
        echo "  sudo apt install linux-tools-common linux-tools-generic"
        echo "  Or for perf:"
        echo "  echo 0 | sudo tee /proc/sys/kernel/perf_event_paranoid"
        exit 1
    fi

    # Check perf permissions on Linux
    if [[ "$(uname)" == "Linux" ]]; then
        local paranoid
        paranoid=$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo "3")
        if [[ "$paranoid" -gt 1 ]]; then
            echo -e "${YELLOW}Warning: perf_event_paranoid is ${paranoid}${NC}"
            echo "For best results, run:"
            echo "  echo 0 | sudo tee /proc/sys/kernel/perf_event_paranoid"
            echo ""
        fi
    fi
}

generate_flamegraph() {
    local bench_name=$1
    local filter=$2

    mkdir -p "$OUTPUT_DIR"

    local output_file="${OUTPUT_DIR}/${bench_name}"
    if [[ -n "$filter" ]]; then
        output_file="${output_file}_${filter}"
    fi
    output_file="${output_file}.svg"

    echo -e "${GREEN}=== Generating Flamegraph ===${NC}"
    echo "Benchmark: ${bench_name}"
    if [[ -n "$filter" ]]; then
        echo "Filter:    ${filter}"
    fi
    echo "Output:    ${output_file}"
    echo ""

    cd "$PROJECT_ROOT"

    # Build flamegraph command
    local cmd="cargo flamegraph --bench ${bench_name} -o ${output_file}"
    if [[ -n "$filter" ]]; then
        cmd="${cmd} -- --bench \"${filter}\""
    fi

    echo "Running: ${cmd}"
    echo ""

    # Execute flamegraph
    eval "$cmd"

    echo ""
    echo -e "${GREEN}Flamegraph generated: ${output_file}${NC}"
    echo ""
    echo "Open in browser to view:"
    echo "  open ${output_file}  # macOS"
    echo "  xdg-open ${output_file}  # Linux"
}

# Main
if [[ "${1:-}" == "-h" ]] || [[ "${1:-}" == "--help" ]]; then
    print_usage
    exit 0
fi

check_deps
generate_flamegraph "$BENCH_NAME" "${BENCH_FILTER:-}"
