#!/bin/bash
# ABOUTME: Memory profiling script using heaptrack or valgrind
# ABOUTME: Identifies memory allocations and potential leaks
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="${SCRIPT_DIR}/../.."
OUTPUT_DIR="${PROJECT_ROOT}/target/profiling"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

TOOL="${1:-}"
TARGET="${2:-pierre-mcp-server}"

print_usage() {
    echo "Usage: $0 <tool> [target]"
    echo ""
    echo "Tools:"
    echo "  heaptrack - Memory allocation profiling (recommended)"
    echo "  valgrind  - Memory leak detection"
    echo "  dhat      - Heap profiling via DHAT"
    echo ""
    echo "Targets:"
    echo "  pierre-mcp-server  - Main server binary (default)"
    echo "  bench:<name>       - Benchmark binary"
    echo ""
    echo "Examples:"
    echo "  $0 heaptrack"
    echo "  $0 valgrind pierre-mcp-server"
    echo "  $0 heaptrack bench:intelligence_bench"
}

check_heaptrack() {
    if ! command -v heaptrack &> /dev/null; then
        echo -e "${RED}Error: heaptrack not installed${NC}"
        echo ""
        echo "Install with:"
        echo "  Ubuntu/Debian: sudo apt install heaptrack heaptrack-gui"
        echo "  Fedora: sudo dnf install heaptrack"
        echo "  macOS: Not available - use Instruments instead"
        exit 1
    fi
}

check_valgrind() {
    if ! command -v valgrind &> /dev/null; then
        echo -e "${RED}Error: valgrind not installed${NC}"
        echo ""
        echo "Install with:"
        echo "  Ubuntu/Debian: sudo apt install valgrind"
        echo "  Fedora: sudo dnf install valgrind"
        echo "  macOS: brew install valgrind (limited support)"
        exit 1
    fi
}

build_target() {
    local target=$1

    echo -e "${YELLOW}Building ${target} in release mode...${NC}"

    cd "$PROJECT_ROOT"

    if [[ "$target" == bench:* ]]; then
        local bench_name="${target#bench:}"
        cargo build --release --bench "$bench_name"
    else
        cargo build --release --bin "$target"
    fi
}

run_heaptrack() {
    local target=$1

    check_heaptrack
    build_target "$target"
    mkdir -p "$OUTPUT_DIR"

    local binary_path
    local output_file

    if [[ "$target" == bench:* ]]; then
        local bench_name="${target#bench:}"
        binary_path="${PROJECT_ROOT}/target/release/deps/${bench_name}-*"
        output_file="${OUTPUT_DIR}/heaptrack_${bench_name}"
    else
        binary_path="${PROJECT_ROOT}/target/release/${target}"
        output_file="${OUTPUT_DIR}/heaptrack_${target}"
    fi

    echo ""
    echo -e "${GREEN}=== Running Heaptrack ===${NC}"
    echo "Target: ${target}"
    echo ""

    # Run with heaptrack
    heaptrack -o "$output_file" "$binary_path" &

    local pid=$!
    echo "Started process with PID: ${pid}"
    echo ""
    echo "Press Ctrl+C to stop profiling..."
    echo "Or wait for the process to finish."

    # Wait for process or interrupt
    wait $pid || true

    echo ""
    echo -e "${GREEN}Profile saved to: ${output_file}.gz${NC}"
    echo ""
    echo "Analyze with:"
    echo "  heaptrack_gui ${output_file}.gz"
    echo "  heaptrack_print ${output_file}.gz"
}

run_valgrind() {
    local target=$1

    check_valgrind
    build_target "$target"
    mkdir -p "$OUTPUT_DIR"

    local binary_path
    local output_file

    if [[ "$target" == bench:* ]]; then
        local bench_name="${target#bench:}"
        binary_path="${PROJECT_ROOT}/target/release/deps/${bench_name}-*"
        output_file="${OUTPUT_DIR}/valgrind_${bench_name}.txt"
    else
        binary_path="${PROJECT_ROOT}/target/release/${target}"
        output_file="${OUTPUT_DIR}/valgrind_${target}.txt"
    fi

    echo ""
    echo -e "${GREEN}=== Running Valgrind ===${NC}"
    echo "Target: ${target}"
    echo ""

    # Run with valgrind memcheck
    valgrind \
        --leak-check=full \
        --show-leak-kinds=all \
        --track-origins=yes \
        --verbose \
        --log-file="$output_file" \
        "$binary_path" &

    local pid=$!
    echo "Started process with PID: ${pid}"
    echo ""
    echo "Press Ctrl+C to stop..."

    sleep 10  # Let it run for a bit
    kill -TERM $pid 2>/dev/null || true
    wait $pid 2>/dev/null || true

    echo ""
    echo -e "${GREEN}Valgrind report saved to: ${output_file}${NC}"
    echo ""
    echo "View with:"
    echo "  less ${output_file}"
}

run_dhat() {
    local target=$1

    echo -e "${YELLOW}DHAT profiling requires code instrumentation${NC}"
    echo ""
    echo "Add to your benchmark or test:"
    echo ""
    echo '  // At top of file'
    echo '  #[global_allocator]'
    echo '  static ALLOC: dhat::Alloc = dhat::Alloc;'
    echo ''
    echo '  // In main or test'
    echo '  let _profiler = dhat::Profiler::new_heap();'
    echo ''
    echo "Then add to Cargo.toml [dev-dependencies]:"
    echo '  dhat = "0.3"'
    echo ""
    echo "After running, open dhat-heap.json with:"
    echo "  https://nicois.github.io/dhat-viewer/"
}

# Main
if [[ -z "$TOOL" ]] || [[ "$TOOL" == "-h" ]] || [[ "$TOOL" == "--help" ]]; then
    print_usage
    exit 0
fi

case "$TOOL" in
    heaptrack)
        run_heaptrack "$TARGET"
        ;;
    valgrind)
        run_valgrind "$TARGET"
        ;;
    dhat)
        run_dhat "$TARGET"
        ;;
    *)
        echo -e "${RED}Unknown tool: ${TOOL}${NC}"
        print_usage
        exit 1
        ;;
esac
