#!/bin/bash
# ABOUTME: Deployment script for FitnessAnalysisAgent
# ABOUTME: Handles environment setup, validation, and agent execution

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
AGENT_NAME="FitnessAnalysisAgent"
LOG_FILE="${SCRIPT_DIR}/agent.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

# Print usage information
print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  --dev                    Run in development mode (single analysis)"
    echo "  --production            Run in production mode (continuous)"
    echo "  --config FILE           Use custom configuration file"
    echo "  --validate-only         Only validate configuration, don't run"
    echo "  --setup-demo            Setup demo environment with mock data"
    echo "  --help                  Show this help message"
    echo ""
    echo "Environment Variables:"
    echo "  PIERRE_A2A_CLIENT_ID     A2A client ID (required)"
    echo "  PIERRE_A2A_CLIENT_SECRET A2A client secret (required)"
    echo "  PIERRE_SERVER_URL        Pierre server URL (default: http://localhost:8081)"
    echo "  ANALYSIS_INTERVAL_HOURS  Analysis interval in hours (default: 24)"
    echo "  MAX_ACTIVITIES           Max activities per analysis (default: 200)"
    echo "  GENERATE_REPORTS         Generate analysis reports (default: true)"
    echo "  REPORT_OUTPUT_DIR        Report output directory (default: /tmp/fitness_reports)"
}

# Validate required environment variables
validate_environment() {
    log_info "Validating environment configuration..."
    
    local missing_vars=()
    
    if [[ -z "${PIERRE_A2A_CLIENT_ID:-}" ]]; then
        missing_vars+=("PIERRE_A2A_CLIENT_ID")
    fi
    
    if [[ -z "${PIERRE_A2A_CLIENT_SECRET:-}" ]]; then
        missing_vars+=("PIERRE_A2A_CLIENT_SECRET")
    fi
    
    if [[ ${#missing_vars[@]} -gt 0 ]]; then
        log_error "Missing required environment variables:"
        for var in "${missing_vars[@]}"; do
            log_error "  - $var"
        done
        echo ""
        log_info "To register an A2A client, run:"
        log_info "curl -X POST http://localhost:8081/a2a/clients \\"
        log_info "  -H \"Authorization: Bearer \$ADMIN_TOKEN\" \\"
        log_info "  -H \"Content-Type: application/json\" \\"
        log_info "  -d '{\"name\": \"Fitness Analyzer\", \"description\": \"Autonomous fitness analysis\"}'"
        return 1
    fi
    
    log_success "Environment validation passed"
    return 0
}

# Check if Pierre server is accessible
check_server_connectivity() {
    local server_url="${PIERRE_SERVER_URL:-http://localhost:8081}"
    
    log_info "Checking Pierre server connectivity at $server_url..."
    
    if curl -s --connect-timeout 5 "$server_url/health" >/dev/null 2>&1; then
        log_success "Pierre server is accessible"
        return 0
    else
        log_warning "Pierre server not accessible at $server_url"
        log_info "Make sure the Pierre server is running:"
        log_info "  cd pierre_mcp_server && cargo run --bin pierre-mcp-server"
        return 1
    fi
}

# Test A2A authentication
test_a2a_authentication() {
    local server_url="${PIERRE_SERVER_URL:-http://localhost:8081}"
    
    log_info "Testing A2A authentication..."
    
    local auth_response
    auth_response=$(curl -s -w "%{http_code}" \
        -X POST "$server_url/a2a/auth" \
        -H "Content-Type: application/json" \
        -d "{
            \"client_id\": \"${PIERRE_A2A_CLIENT_ID}\",
            \"client_secret\": \"${PIERRE_A2A_CLIENT_SECRET}\"
        }" 2>/dev/null) || {
        log_error "Failed to connect to A2A authentication endpoint"
        return 1
    }
    
    local http_code="${auth_response: -3}"
    local response_body="${auth_response%???}"
    
    if [[ "$http_code" == "200" ]]; then
        log_success "A2A authentication test successful"
        return 0
    else
        log_error "A2A authentication failed (HTTP $http_code)"
        if [[ "$http_code" == "401" ]]; then
            log_error "Invalid client credentials. Please check PIERRE_A2A_CLIENT_ID and PIERRE_A2A_CLIENT_SECRET"
        fi
        return 1
    fi
}

# Setup demo environment
setup_demo() {
    log_info "Setting up demo environment..."
    
    # Set demo environment variables
    export DEVELOPMENT_MODE="true"
    export ANALYSIS_INTERVAL_HOURS="1"
    export MAX_ACTIVITIES_PER_ANALYSIS="50"
    export GENERATE_REPORTS="true"
    export REPORT_OUTPUT_DIR="${SCRIPT_DIR}/demo_reports"
    
    # Create demo reports directory
    mkdir -p "$REPORT_OUTPUT_DIR"
    
    # Set demo credentials if not provided
    if [[ -z "${PIERRE_A2A_CLIENT_ID:-}" ]]; then
        log_warning "Using demo A2A client credentials"
        log_info "In production, register a real A2A client first"
        export PIERRE_A2A_CLIENT_ID="demo_fitness_analyzer"
        export PIERRE_A2A_CLIENT_SECRET="demo_secret_123"
    fi
    
    log_success "Demo environment configured"
    log_info "Reports will be saved to: $REPORT_OUTPUT_DIR"
}

# Build the agent
build_agent() {
    log_info "Building $AGENT_NAME..."
    
    cd "$SCRIPT_DIR"
    
    if cargo build --release 2>>"$LOG_FILE"; then
        log_success "Agent built successfully"
        return 0
    else
        log_error "Agent build failed. Check $LOG_FILE for details"
        return 1
    fi
}

# Run tests
run_tests() {
    log_info "Running agent tests..."
    
    cd "$SCRIPT_DIR"
    
    if cargo test --release 2>>"$LOG_FILE"; then
        log_success "All tests passed"
        return 0
    else
        log_error "Some tests failed. Check $LOG_FILE for details"
        return 1
    fi
}

# Run the agent
run_agent() {
    local mode="${1:-development}"
    
    log_info "Starting $AGENT_NAME in $mode mode..."
    log_info "Log file: $LOG_FILE"
    
    cd "$SCRIPT_DIR"
    
    # Set mode-specific environment
    if [[ "$mode" == "development" ]]; then
        export DEVELOPMENT_MODE="true"
        log_info "Development mode: will run single analysis and exit"
    else
        export DEVELOPMENT_MODE="false"
        log_info "Production mode: continuous operation"
    fi
    
    # Run the agent
    if cargo run --release --bin fitness_analyzer 2>>"$LOG_FILE"; then
        log_success "Agent completed successfully"
        return 0
    else
        log_error "Agent execution failed. Check $LOG_FILE for details"
        return 1
    fi
}

# Display configuration
show_configuration() {
    log_info "Current Configuration:"
    echo "  Server URL: ${PIERRE_SERVER_URL:-http://localhost:8081}"
    echo "  Client ID: ${PIERRE_A2A_CLIENT_ID:-<not set>}"
    echo "  Analysis Interval: ${ANALYSIS_INTERVAL_HOURS:-24} hours"
    echo "  Max Activities: ${MAX_ACTIVITIES_PER_ANALYSIS:-200}"
    echo "  Development Mode: ${DEVELOPMENT_MODE:-false}"
    echo "  Generate Reports: ${GENERATE_REPORTS:-true}"
    echo "  Report Directory: ${REPORT_OUTPUT_DIR:-/tmp/fitness_reports}"
}

# Main execution
main() {
    local mode="development"
    local validate_only=false
    local setup_demo=false
    
    # Initialize log file
    echo "$(date '+%Y-%m-%d %H:%M:%S') - Starting $AGENT_NAME deployment" > "$LOG_FILE"
    
    # Parse command line arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --dev|--development)
                mode="development"
                shift
                ;;
            --prod|--production)
                mode="production"
                shift
                ;;
            --validate-only)
                validate_only=true
                shift
                ;;
            --setup-demo)
                setup_demo=true
                shift
                ;;
            --help)
                print_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                print_usage
                exit 1
                ;;
        esac
    done
    
    echo ""
    log_info "🤖 $AGENT_NAME Deployment Script"
    log_info "================================="
    echo ""
    
    # Setup demo if requested
    if [[ "$setup_demo" == "true" ]]; then
        setup_demo
        echo ""
    fi
    
    # Show current configuration
    show_configuration
    echo ""
    
    # Validate environment
    if ! validate_environment; then
        exit 1
    fi
    echo ""
    
    # If validate-only mode, exit here
    if [[ "$validate_only" == "true" ]]; then
        log_success "Configuration validation complete"
        exit 0
    fi
    
    # Check server connectivity (warning only)
    check_server_connectivity || log_warning "Continuing without server connectivity check"
    echo ""
    
    # Test A2A authentication (warning only in development)
    if ! test_a2a_authentication; then
        if [[ "$mode" == "production" ]]; then
            log_error "A2A authentication required for production mode"
            exit 1
        else
            log_warning "Continuing without A2A authentication in development mode"
        fi
    fi
    echo ""
    
    # Build agent
    if ! build_agent; then
        exit 1
    fi
    echo ""
    
    # Run tests
    if ! run_tests; then
        log_warning "Tests failed, but continuing with deployment"
    fi
    echo ""
    
    # Run agent
    if ! run_agent "$mode"; then
        exit 1
    fi
    
    echo ""
    log_success "🎉 $AGENT_NAME deployment completed successfully!"
    
    # Show report location if applicable
    if [[ "${GENERATE_REPORTS:-true}" == "true" ]]; then
        local report_dir="${REPORT_OUTPUT_DIR:-/tmp/fitness_reports}"
        if [[ -d "$report_dir" ]] && [[ -n "$(ls -A "$report_dir" 2>/dev/null)" ]]; then
            echo ""
            log_info "📄 Analysis reports available in: $report_dir"
            ls -la "$report_dir"/*.json 2>/dev/null | tail -3
        fi
    fi
}

# Execute main function with all arguments
main "$@"