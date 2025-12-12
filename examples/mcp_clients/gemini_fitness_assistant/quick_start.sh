#!/bin/bash
#
# Quick Start Script for Gemini Fitness Assistant
#
# This script helps you quickly set up and run the Gemini Fitness Assistant
# example with Pierre MCP Server.
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}  Gemini Fitness Assistant - Quick Start${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check Python
echo -e "${YELLOW}Checking Python installation...${NC}"
if ! command -v python3 &> /dev/null; then
    echo -e "${RED}❌ Python 3 is not installed${NC}"
    echo "Please install Python 3.8 or higher: https://www.python.org/downloads/"
    exit 1
fi

PYTHON_VERSION=$(python3 --version | awk '{print $2}')
echo -e "${GREEN}✅ Python $PYTHON_VERSION found${NC}"
echo ""

# Check if Pierre server is running
echo -e "${YELLOW}Checking Pierre server...${NC}"
if ! curl -s http://localhost:8081/health &> /dev/null; then
    echo -e "${RED}❌ Pierre server is not running${NC}"
    echo ""
    echo "Please start Pierre server first:"
    echo "  cd ../../../"
    echo "  cargo run --bin pierre-mcp-server"
    echo ""
    exit 1
fi
echo -e "${GREEN}✅ Pierre server is running${NC}"
echo ""

# Install Python dependencies
echo -e "${YELLOW}Installing Python dependencies...${NC}"
if [ ! -d "venv" ]; then
    python3 -m venv venv
fi

source venv/bin/activate
pip install -q --upgrade pip
pip install -q -r requirements.txt
echo -e "${GREEN}✅ Dependencies installed${NC}"
echo ""

# Check for .env file
if [ ! -f ".env" ]; then
    echo -e "${YELLOW}Creating .env file from template...${NC}"
    cp .env.example .env
    echo -e "${GREEN}✅ .env file created${NC}"
    echo ""
fi

# Load environment variables
if [ -f ".env" ]; then
    set -a
    source .env
    set +a
fi

# Check for required environment variables
MISSING_VARS=()

if [ -z "$GEMINI_API_KEY" ] || [ "$GEMINI_API_KEY" == "your-gemini-api-key-here" ]; then
    MISSING_VARS+=("GEMINI_API_KEY")
fi

if [ -z "$PIERRE_EMAIL" ] || [ "$PIERRE_EMAIL" == "user@example.com" ]; then
    MISSING_VARS+=("PIERRE_EMAIL")
fi

if [ -z "$PIERRE_PASSWORD" ] || [ "$PIERRE_PASSWORD" == "SecurePass123!" ]; then
    MISSING_VARS+=("PIERRE_PASSWORD")
fi

if [ ${#MISSING_VARS[@]} -gt 0 ]; then
    echo -e "${RED}❌ Missing required configuration${NC}"
    echo ""
    echo "Please configure the following in .env file:"
    for var in "${MISSING_VARS[@]}"; do
        echo "  - $var"
    done
    echo ""

    if [[ " ${MISSING_VARS[@]} " =~ " GEMINI_API_KEY " ]]; then
        echo -e "${BLUE}Get a free Gemini API key:${NC}"
        echo "  1. Visit: https://ai.google.dev/gemini-api/docs/api-key"
        echo "  2. Click 'Get API Key'"
        echo "  3. Sign in with Google (no credit card required)"
        echo "  4. Copy your API key to .env file"
        echo ""
    fi

    if [[ " ${MISSING_VARS[@]} " =~ " PIERRE_EMAIL " ]] || [[ " ${MISSING_VARS[@]} " =~ " PIERRE_PASSWORD " ]]; then
        echo -e "${BLUE}Create a Pierre user account:${NC}"
        echo "  curl -X POST http://localhost:8081/admin/setup \\"
        echo "    -H 'Content-Type: application/json' \\"
        echo "    -d '{"
        echo "      \"email\": \"user@example.com\","
        echo "      \"password\": \"SecurePass123!\","
        echo "      \"display_name\": \"Test User\""
        echo "    }'"
        echo ""
    fi

    echo "After configuring .env, run this script again."
    exit 1
fi

echo -e "${GREEN}✅ Configuration complete${NC}"
echo ""

# Test authentication
echo -e "${YELLOW}Testing Pierre authentication...${NC}"
LOGIN_RESPONSE=$(curl -s -X POST http://localhost:8081/oauth/token \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=password&username=$PIERRE_EMAIL&password=$PIERRE_PASSWORD" \
    || echo "error")

if echo "$LOGIN_RESPONSE" | grep -q "token"; then
    echo -e "${GREEN}✅ Authentication successful${NC}"
else
    echo -e "${RED}❌ Authentication failed${NC}"
    echo ""
    echo "Make sure you have created a user account:"
    echo "  curl -X POST http://localhost:8081/admin/setup \\"
    echo "    -H 'Content-Type: application/json' \\"
    echo "    -d '{\"email\": \"$PIERRE_EMAIL\", \"password\": \"$PIERRE_PASSWORD\", \"display_name\": \"User\"}'"
    echo ""
    exit 1
fi
echo ""

# Run the assistant
echo -e "${GREEN}========================================${NC}"
echo -e "${GREEN}  Starting Gemini Fitness Assistant${NC}"
echo -e "${GREEN}========================================${NC}"
echo ""

# Check if --demo flag is passed
if [ "$1" == "--demo" ]; then
    echo -e "${BLUE}Running in demo mode...${NC}"
    echo ""
    python gemini_fitness_assistant.py --demo
else
    echo -e "${BLUE}Running in interactive mode...${NC}"
    echo ""
    python gemini_fitness_assistant.py
fi
