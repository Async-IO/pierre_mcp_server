#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Development server startup script for Pierre MCP Server
# ABOUTME: Sets up database, creates users, and starts both backend and frontend servers

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_ROOT"

# Default configuration
export RUST_LOG="${RUST_LOG:-info}"
export HTTP_PORT="${HTTP_PORT:-8081}"
export DATABASE_URL="${DATABASE_URL:-sqlite:./data/users.db}"
export PIERRE_MASTER_ENCRYPTION_KEY="${PIERRE_MASTER_ENCRYPTION_KEY:-W5ZGEOcnt+Ge9lq8ASHfnEkeGryihMoRWUtudKobsM4=}"

# Default users
ADMIN_EMAIL="${ADMIN_EMAIL:-admin@pierre.local}"
ADMIN_PASSWORD="${ADMIN_PASSWORD:-AdminPass123!}"
USER_EMAIL="${USER_EMAIL:-user@pierre.local}"
USER_PASSWORD="${USER_PASSWORD:-UserPass123!}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Cleanup function
cleanup() {
    log_info "Shutting down servers..."
    pkill -f "pierre-mcp-server" 2>/dev/null || true
    pkill -f "vite" 2>/dev/null || true
    exit 0
}

trap cleanup SIGINT SIGTERM

# Stop any existing servers
log_info "Stopping existing servers..."
pkill -f "pierre-mcp-server" 2>/dev/null || true
pkill -f "vite" 2>/dev/null || true
sleep 1

# Create data directory
mkdir -p ./data

# Build the project
log_info "Building project..."
cargo build --release --bin pierre-mcp-server --bin admin-setup 2>&1 | tail -5

# Start backend server in background
log_info "Starting backend server on port $HTTP_PORT..."
./target/release/pierre-mcp-server &
BACKEND_PID=$!
sleep 3

# Check if backend is running
if ! kill -0 $BACKEND_PID 2>/dev/null; then
    log_error "Backend server failed to start"
    exit 1
fi
log_info "Backend server started (PID: $BACKEND_PID)"

# Create admin user
log_info "Creating admin user: $ADMIN_EMAIL"
./target/release/admin-setup create-admin-user \
    --email "$ADMIN_EMAIL" \
    --password "$ADMIN_PASSWORD" 2>&1 | grep -v "^$" || true

# Create regular user via API registration
log_info "Creating regular user: $USER_EMAIL"
curl -s -X POST "http://localhost:$HTTP_PORT/api/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$USER_EMAIL\",\"password\":\"$USER_PASSWORD\"}" \
    | grep -q "user" && log_info "User created successfully" || log_warn "User may already exist"

# Start frontend server
log_info "Starting frontend server..."
cd frontend
bun run dev &
FRONTEND_PID=$!
cd "$PROJECT_ROOT"

sleep 2

# Print summary
echo ""
echo "=========================================="
echo "  Pierre MCP Server Development Setup"
echo "=========================================="
echo ""
echo "Backend:  http://localhost:$HTTP_PORT"
echo "Frontend: http://localhost:5173"
echo ""
echo "Admin User:"
echo "  Email:    $ADMIN_EMAIL"
echo "  Password: $ADMIN_PASSWORD"
echo ""
echo "Regular User:"
echo "  Email:    $USER_EMAIL"
echo "  Password: $USER_PASSWORD"
echo ""
echo "Press Ctrl+C to stop all servers"
echo "=========================================="
echo ""

# Wait for servers
wait
