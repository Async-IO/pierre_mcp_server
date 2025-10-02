#!/bin/bash
# ABOUTME: Complete user registration and approval workflow test script
# ABOUTME: Implements all 5 steps from HOW_TO_REGISTER_A_USER.md with proper error handling

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Load HTTP_PORT from .envrc if available
if [ -f .envrc ]; then
    source .envrc
fi

# Use HTTP_PORT from environment or default to 8081
HTTP_PORT=${HTTP_PORT:-8081}

echo -e "${BLUE}=== Pierre MCP Server Complete User Workflow Test ===${NC}"
echo -e "${BLUE}Using server port: $HTTP_PORT${NC}"

# Check if server is running
if ! curl -s -f http://localhost:$HTTP_PORT/admin/health > /dev/null; then
    echo -e "${RED}❌ Server not running on http://localhost:$HTTP_PORT${NC}"
    echo "Please start the server first:"
    echo "  source .envrc && RUST_LOG=debug cargo run --bin pierre-mcp-server"
    exit 1
fi

echo -e "${GREEN}✅ Server is running${NC}"

# Step 1: Create Admin User
echo -e "\n${BLUE}=== Step 1: Create Admin User ===${NC}"

ADMIN_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@pierre.mcp",
    "password": "adminpass123",
    "display_name": "System Administrator"
  }')

# Extract admin token for future use
ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.admin_token')

if [[ "$ADMIN_TOKEN" == "null" || -z "$ADMIN_TOKEN" ]]; then
    echo -e "${RED}❌ Failed to create admin or extract admin token${NC}"
    echo "Response: $ADMIN_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ Admin created successfully${NC}"
echo "Admin token (first 50 chars): ${ADMIN_TOKEN:0:50}..."

# Step 2: Register Regular User
echo -e "\n${BLUE}=== Step 2: Register Regular User ===${NC}"

USER_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123",
    "display_name": "Regular User"
  }')

# Extract user ID for approval
USER_ID=$(echo $USER_RESPONSE | jq -r '.user_id')

if [[ "$USER_ID" == "null" || -z "$USER_ID" ]]; then
    echo -e "${RED}❌ Failed to register user or extract user ID${NC}"
    echo "Response: $USER_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ User registered successfully${NC}"
echo "User ID: $USER_ID"

# Step 3: Approve User WITH Tenant Creation
echo -e "\n${BLUE}=== Step 3: Approve User with Tenant Creation ===${NC}"

APPROVAL_RESPONSE=$(curl -s -X POST "http://localhost:$HTTP_PORT/admin/approve-user/$USER_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "reason": "User registration approved",
    "create_default_tenant": true,
    "tenant_name": "User Organization",
    "tenant_slug": "user-org"
  }')

echo "Approval result:"
echo $APPROVAL_RESPONSE | jq

# Extract tenant info
TENANT_ID=$(echo $APPROVAL_RESPONSE | jq -r '.tenant_created.tenant_id')

if [[ "$TENANT_ID" == "null" || -z "$TENANT_ID" ]]; then
    echo -e "${RED}❌ Failed to approve user or create tenant${NC}"
    exit 1
fi

echo -e "${GREEN}✅ User approved with tenant created${NC}"
echo "Tenant ID: $TENANT_ID"

# Step 4: User Login
echo -e "\n${BLUE}=== Step 4: User Login ===${NC}"

LOGIN_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123"
  }')

# Extract JWT token for MCP access
JWT_TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.jwt_token')

if [[ "$JWT_TOKEN" == "null" || -z "$JWT_TOKEN" ]]; then
    echo -e "${RED}❌ Failed to login user or extract JWT token${NC}"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ User logged in successfully${NC}"
echo "JWT Token (first 50 chars): ${JWT_TOKEN:0:50}..."

# Step 5: Test MCP Access
echo -e "\n${BLUE}=== Step 5: Test MCP Access ===${NC}"

TOOLS_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }')

TOOLS_COUNT=$(echo $TOOLS_RESPONSE | jq '.result.tools | length')

if [[ "$TOOLS_COUNT" == "null" || "$TOOLS_COUNT" -lt 20 ]]; then
    echo -e "${RED}❌ MCP access failed or insufficient tools available${NC}"
    echo "Response: $TOOLS_RESPONSE"
    exit 1
fi

echo -e "${GREEN}✅ MCP working: $TOOLS_COUNT tools available${NC}"

# Test connection status
echo -e "\n${BLUE}=== Testing Connection Status ===${NC}"

CONNECTION_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_connection_status",
      "arguments": {}
    },
    "id": 2
  }')

echo "Connection Status:"
echo $CONNECTION_RESPONSE | jq '.result'

# Save important values for later use
cat << EOF > .workflow_test_env
# Generated by complete-user-workflow.sh on $(date)
export ADMIN_TOKEN="$ADMIN_TOKEN"
export USER_ID="$USER_ID" 
export TENANT_ID="$TENANT_ID"
export JWT_TOKEN="$JWT_TOKEN"
EOF

echo -e "\n${GREEN}🎉 Complete workflow test completed successfully!${NC}"
echo -e "${BLUE}Environment variables saved to .workflow_test_env${NC}"

echo -e "\n${YELLOW}Summary:${NC}"
echo "- Admin User: admin@pierre.mcp"
echo "- Regular User: user@example.com (ID: $USER_ID)"
echo "- Tenant: User Organization (ID: $TENANT_ID)"
echo "- MCP Tools Available: $TOOLS_COUNT"
echo ""
echo "To reuse these variables in another session:"
echo "  source .workflow_test_env"
echo ""
echo -e "${GREEN}✅ Ready for Strava integration testing!${NC}"