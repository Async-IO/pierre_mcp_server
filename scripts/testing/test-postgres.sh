#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: PostgreSQL database plugin integration test runner
# ABOUTME: Starts PostgreSQL via Docker and runs database operation tests
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

# Test PostgreSQL database plugin integration
# This script starts PostgreSQL via Docker and runs tests against it

set -e

echo "🐘 Setting up PostgreSQL testing environment..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$PROJECT_ROOT"

echo -e "${BLUE}==== PostgreSQL Database Plugin Testing ====${NC}"
echo "Project root: $PROJECT_ROOT"

# Check if docker and docker-compose are available
if ! command -v docker &> /dev/null; then
    echo -e "${RED}❌ Docker is not installed or not in PATH${NC}"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo -e "${RED}❌ docker-compose is not installed or not in PATH${NC}"
    exit 1
fi

# Stop any existing containers
echo -e "${BLUE}==== Stopping existing PostgreSQL containers... ====${NC}"
docker-compose -f docker-compose.postgres.yml down --volumes --remove-orphans || true

# Start PostgreSQL
echo -e "${BLUE}==== Starting PostgreSQL container... ====${NC}"
docker-compose -f docker-compose.postgres.yml up -d postgres

# Wait for PostgreSQL to be ready
echo -e "${BLUE}==== Waiting for PostgreSQL to be ready... ====${NC}"
timeout=60
counter=0
while ! docker-compose -f docker-compose.postgres.yml exec -T postgres pg_isready -U pierre -d pierre_mcp_server &> /dev/null; do
    if [ $counter -eq $timeout ]; then
        echo -e "${RED}❌ PostgreSQL failed to start within $timeout seconds${NC}"
        docker-compose -f docker-compose.postgres.yml logs postgres
        exit 1
    fi
    echo -e "${YELLOW}⏳ Waiting for PostgreSQL... ($counter/$timeout)${NC}"
    sleep 1
    ((counter++))
done

echo -e "${GREEN}✅ PostgreSQL is ready!${NC}"

# Show PostgreSQL connection info
echo -e "${BLUE}==== PostgreSQL Connection Info ====${NC}"
echo "Host: localhost:5432"
echo "Database: pierre_mcp_server"
echo "User: pierre"
echo "Connection String: postgresql://pierre:pierre_dev_password@localhost:5432/pierre_mcp_server"

# Test basic connectivity
echo -e "${BLUE}==== Testing PostgreSQL connectivity... ====${NC}"
docker-compose -f docker-compose.postgres.yml exec -T postgres psql -U pierre -d pierre_mcp_server -c "SELECT version();"

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ PostgreSQL connectivity test passed${NC}"
else
    echo -e "${RED}❌ PostgreSQL connectivity test failed${NC}"
    exit 1
fi

# Set environment variables for testing
export DATABASE_URL="postgresql://pierre:pierre_dev_password@localhost:5432/pierre_mcp_server"
export ENCRYPTION_KEY="YWJjZGVmZ2hpamtsbW5vcHFyc3R1dnd4eXowMTIzNDU2"
export RUST_LOG=debug

# Run database plugin tests with PostgreSQL features
echo -e "${BLUE}==== Running database plugin tests with PostgreSQL... ====${NC}"
echo -e "${YELLOW}📝 Note: Watch for log messages like:${NC}"
echo -e "${YELLOW}   🗄️  Detected database type: PostgreSQL${NC}"
echo -e "${YELLOW}   🐘 Initializing PostgreSQL database${NC}"
echo -e "${YELLOW}   ✅ PostgreSQL database initialized successfully${NC}"
echo ""
cargo test --features postgresql database_plugins_test --verbose

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Database plugin tests passed with PostgreSQL${NC}"
else
    echo -e "${RED}❌ Database plugin tests failed with PostgreSQL${NC}"
    echo -e "${YELLOW}💡 Check the logs above for error details${NC}"
    exit 1
fi

# Run all tests with PostgreSQL to ensure compatibility
echo -e "${BLUE}==== Running all tests with PostgreSQL... ====${NC}"
cargo test --features postgresql -- --test-threads=1

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ All tests passed with PostgreSQL${NC}"
else
    echo -e "${RED}❌ Some tests failed with PostgreSQL${NC}"
    echo -e "${YELLOW}💡 Check the logs above for error details${NC}"
fi

# Optional: Leave PostgreSQL running for manual testing
if [ "$1" = "--keep-running" ]; then
    echo -e "${BLUE}==== PostgreSQL left running for manual testing ====${NC}"
    echo "To connect: psql postgresql://pierre:pierre_dev_password@localhost:5432/pierre_mcp_server"
    echo "To stop: docker-compose -f docker-compose.postgres.yml down"
    echo ""
    echo "pgAdmin is available at: http://localhost:8080"
    echo "  Email: admin@pierre.local"
    echo "  Password: admin"
    echo ""
    echo "To start pgAdmin: docker-compose -f docker-compose.postgres.yml --profile admin up -d pgadmin"
else
    echo -e "${BLUE}==== Cleaning up PostgreSQL containers... ====${NC}"
    docker-compose -f docker-compose.postgres.yml down --volumes
    echo -e "${GREEN}✅ Cleanup completed${NC}"
fi

echo -e "${GREEN}✅ PostgreSQL testing completed successfully! 🎉${NC}"