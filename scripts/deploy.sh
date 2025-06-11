#!/bin/bash
# Pierre MCP Server - Deployment Script

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
COMPOSE_FILE="docker-compose.yml"
ENV_MODE="${1:-development}"

print_usage() {
    echo -e "${BLUE}Usage: $0 [development|production|stop|logs|status]${NC}"
    echo -e "${BLUE}  development - Start development environment${NC}"
    echo -e "${BLUE}  production  - Start production environment${NC}"
    echo -e "${BLUE}  stop        - Stop all services${NC}"
    echo -e "${BLUE}  logs        - Show service logs${NC}"
    echo -e "${BLUE}  status      - Show service status${NC}"
}

check_dependencies() {
    echo -e "${YELLOW}🔍 Checking dependencies...${NC}"
    
    if ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker not found. Please install Docker.${NC}"
        exit 1
    fi
    
    if ! command -v docker-compose &> /dev/null && ! docker compose version &> /dev/null; then
        echo -e "${RED}❌ Docker Compose not found. Please install Docker Compose.${NC}"
        exit 1
    fi
    
    echo -e "${GREEN}✅ Dependencies check passed${NC}"
}

setup_environment() {
    echo -e "${YELLOW}⚙️  Setting up environment...${NC}"
    
    # Create .env file if it doesn't exist
    if [[ ! -f ".env" ]]; then
        echo -e "${YELLOW}📝 Creating .env file from template...${NC}"
        cp .env.example .env
        echo -e "${YELLOW}⚠️  Please edit .env file with your configuration before running again.${NC}"
        exit 1
    fi
    
    # Create required directories
    mkdir -p backups data
    
    echo -e "${GREEN}✅ Environment setup complete${NC}"
}

start_development() {
    echo -e "${GREEN}🚀 Starting development environment...${NC}"
    
    COMPOSE_FILE="docker-compose.yml"
    export COMPOSE_PROFILES="debug"
    
    docker-compose -f "${COMPOSE_FILE}" up -d
    
    echo -e "${GREEN}✅ Development environment started${NC}"
    echo -e "${BLUE}🌐 MCP Server: http://localhost:8080${NC}"
    echo -e "${BLUE}🌐 HTTP API: http://localhost:8081${NC}"
    echo -e "${BLUE}🌐 Database Browser: http://localhost:8082${NC}"
}

start_production() {
    echo -e "${GREEN}🚀 Starting production environment...${NC}"
    
    COMPOSE_FILE="docker-compose.prod.yml"
    
    # Check if production image exists
    if ! docker images pierre-mcp-server:latest | grep -q latest; then
        echo -e "${YELLOW}📦 Building production image...${NC}"
        ./scripts/build.sh latest
    fi
    
    docker-compose -f "${COMPOSE_FILE}" up -d
    
    echo -e "${GREEN}✅ Production environment started${NC}"
    echo -e "${BLUE}🌐 MCP Server: http://localhost:8080${NC}"
    echo -e "${BLUE}🌐 HTTP API: http://localhost:8081${NC}"
}

stop_services() {
    echo -e "${YELLOW}🛑 Stopping all services...${NC}"
    
    docker-compose -f docker-compose.yml down 2>/dev/null || true
    docker-compose -f docker-compose.prod.yml down 2>/dev/null || true
    
    echo -e "${GREEN}✅ All services stopped${NC}"
}

show_logs() {
    echo -e "${BLUE}📋 Service logs:${NC}"
    
    if docker-compose -f docker-compose.yml ps -q pierre-mcp-server 2>/dev/null | grep -q .; then
        docker-compose -f docker-compose.yml logs -f --tail=50 pierre-mcp-server
    elif docker-compose -f docker-compose.prod.yml ps -q pierre-mcp-server-prod 2>/dev/null | grep -q .; then
        docker-compose -f docker-compose.prod.yml logs -f --tail=50 pierre-mcp-server-prod
    else
        echo -e "${YELLOW}⚠️  No running services found${NC}"
    fi
}

show_status() {
    echo -e "${BLUE}📊 Service status:${NC}"
    
    echo -e "${YELLOW}Development services:${NC}"
    docker-compose -f docker-compose.yml ps 2>/dev/null || echo "Not running"
    
    echo -e "${YELLOW}Production services:${NC}"
    docker-compose -f docker-compose.prod.yml ps 2>/dev/null || echo "Not running"
    
    echo -e "${YELLOW}Health checks:${NC}"
    if curl -s http://localhost:8081/health &>/dev/null; then
        echo -e "${GREEN}✅ HTTP API is healthy${NC}"
    else
        echo -e "${RED}❌ HTTP API is not responding${NC}"
    fi
}

# Main execution
case "${ENV_MODE}" in
    "development"|"dev")
        check_dependencies
        setup_environment
        start_development
        ;;
    "production"|"prod")
        check_dependencies
        setup_environment
        start_production
        ;;
    "stop")
        stop_services
        ;;
    "logs")
        show_logs
        ;;
    "status")
        show_status
        ;;
    *)
        print_usage
        exit 1
        ;;
esac