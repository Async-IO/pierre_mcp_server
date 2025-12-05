#!/bin/bash
# ABOUTME: Orchestration script for k6 load tests
# ABOUTME: Runs scenarios with configurable profiles (smoke, load, stress, soak)
#
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCENARIOS_DIR="${SCRIPT_DIR}/scenarios"
CONFIG_DIR="${SCRIPT_DIR}/config"
RESULTS_DIR="${SCRIPT_DIR}/results"

# Default values
BASE_URL="${BASE_URL:-http://localhost:8081}"
SCENARIO="${1:-health_check}"
PROFILE="${2:-smoke}"
API_KEY="${API_KEY:-}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

print_usage() {
    echo "Usage: $0 <scenario> [profile]"
    echo ""
    echo "Scenarios:"
    echo "  health_check    - Health endpoint baseline (default)"
    echo "  auth_flow       - Authentication endpoints"
    echo "  mcp_tools       - MCP tool invocations"
    echo "  mixed_workload  - Realistic mixed traffic"
    echo ""
    echo "Profiles:"
    echo "  smoke   - Quick sanity check (1 VU, 30s) (default)"
    echo "  load    - Normal load test (50 VUs, 5min)"
    echo "  stress  - Stress test (200 VUs, 18min)"
    echo "  soak    - Soak test (20 VUs, 1hr)"
    echo ""
    echo "Environment variables:"
    echo "  BASE_URL  - Server URL (default: http://localhost:8081)"
    echo "  API_KEY   - API key for authenticated requests"
    echo ""
    echo "Examples:"
    echo "  $0 health_check smoke"
    echo "  $0 mcp_tools load"
    echo "  BASE_URL=https://staging.example.com $0 mixed_workload stress"
}

check_k6() {
    if ! command -v k6 &> /dev/null; then
        echo -e "${RED}Error: k6 is not installed${NC}"
        echo ""
        echo "Install k6:"
        echo "  macOS:  brew install k6"
        echo "  Linux:  sudo gpg -k && sudo gpg --no-default-keyring --keyring /usr/share/keyrings/k6-archive-keyring.gpg --keyserver hkp://keyserver.ubuntu.com:80 --recv-keys C5AD17C747E3415A3642D57D77C6C491D6AC1D69 && echo \"deb [signed-by=/usr/share/keyrings/k6-archive-keyring.gpg] https://dl.k6.io/deb stable main\" | sudo tee /etc/apt/sources.list.d/k6.list && sudo apt-get update && sudo apt-get install k6"
        echo "  Docker: docker pull grafana/k6"
        echo ""
        echo "See: https://k6.io/docs/get-started/installation/"
        exit 1
    fi
}

check_server() {
    echo -e "${YELLOW}Checking server health...${NC}"
    if curl -s -f "${BASE_URL}/health" > /dev/null 2>&1; then
        echo -e "${GREEN}Server is healthy${NC}"
    else
        echo -e "${RED}Warning: Server at ${BASE_URL} may not be running${NC}"
        echo "Continue anyway? (y/N)"
        read -r response
        if [[ ! "$response" =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
}

run_test() {
    local scenario=$1
    local profile=$2
    local scenario_file="${SCENARIOS_DIR}/${scenario}.js"
    local config_file="${CONFIG_DIR}/${profile}.json"

    # Validate scenario exists
    if [[ ! -f "$scenario_file" ]]; then
        echo -e "${RED}Error: Scenario '${scenario}' not found at ${scenario_file}${NC}"
        exit 1
    fi

    # Validate config exists
    if [[ ! -f "$config_file" ]]; then
        echo -e "${RED}Error: Profile '${profile}' not found at ${config_file}${NC}"
        exit 1
    fi

    # Create results directory
    mkdir -p "$RESULTS_DIR"

    echo ""
    echo -e "${GREEN}=== Running Load Test ===${NC}"
    echo "Scenario: ${scenario}"
    echo "Profile:  ${profile}"
    echo "URL:      ${BASE_URL}"
    echo ""

    # Run k6 with scenario and config
    k6 run \
        --config "$config_file" \
        --env "BASE_URL=${BASE_URL}" \
        --env "API_KEY=${API_KEY}" \
        --out "json=${RESULTS_DIR}/${scenario}_${profile}_$(date +%Y%m%d_%H%M%S).json" \
        "$scenario_file"

    echo ""
    echo -e "${GREEN}Test completed!${NC}"
    echo "Results saved to: ${RESULTS_DIR}/"
}

# Main
if [[ "${1:-}" == "-h" ]] || [[ "${1:-}" == "--help" ]]; then
    print_usage
    exit 0
fi

check_k6
check_server
run_test "$SCENARIO" "$PROFILE"
