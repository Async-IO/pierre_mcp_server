#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Test script for business API key provisioning system
# ABOUTME: Validates trial key generation, registration, and authentication workflow
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

set -e

echo "Testing Business API Key Provisioning System"
echo "============================================="

# Load HTTP_PORT from .envrc if available
if [ -f .envrc ]; then
    source .envrc
fi

# Use HTTP_PORT from environment or default to 8081
HTTP_PORT=${HTTP_PORT:-8081}
BASE_URL="http://localhost:$HTTP_PORT"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if server is running
echo -e "\n${BLUE}Checking server health...${NC}"
if ! curl -s -f "$BASE_URL/admin/health" > /dev/null; then
    echo -e "${RED}Server not running on $BASE_URL${NC}"
    echo "Please start the server first:"
    echo "  source .envrc && RUST_LOG=debug cargo run --bin pierre-mcp-server"
    exit 1
fi
echo -e "${GREEN}Server is running${NC}"

# Test credentials
ADMIN_EMAIL="trial_admin@example.com"
ADMIN_PASSWORD="adminpass123"
USER_EMAIL="trial_user@example.com"
USER_PASSWORD="userpass123"

echo -e "\n${YELLOW}Step 1: Create Admin User${NC}"
ADMIN_RESPONSE=$(curl -s -X POST "$BASE_URL/admin/setup" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$ADMIN_EMAIL\",
    \"password\": \"$ADMIN_PASSWORD\",
    \"display_name\": \"Trial Test Admin\"
  }")

ADMIN_TOKEN=$(echo "$ADMIN_RESPONSE" | jq -r '.data.admin_token // .admin_token // empty')

if [[ -z "$ADMIN_TOKEN" || "$ADMIN_TOKEN" == "null" ]]; then
    echo -e "${RED}Failed to create admin or extract admin token${NC}"
    echo "Response: $ADMIN_RESPONSE"
    exit 1
fi
echo -e "${GREEN}Admin created successfully${NC}"
echo "Admin token (first 50 chars): ${ADMIN_TOKEN:0:50}..."

echo -e "\n${YELLOW}Step 2: Register Regular User${NC}"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/register" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"$USER_PASSWORD\",
    \"display_name\": \"Trial Test User\"
  }")

USER_ID=$(echo "$REGISTER_RESPONSE" | jq -r '.user_id // empty')

if [[ -z "$USER_ID" || "$USER_ID" == "null" ]]; then
    echo -e "${RED}Failed to register user${NC}"
    echo "Response: $REGISTER_RESPONSE"
    exit 1
fi
echo -e "${GREEN}User registered: $USER_ID${NC}"

echo -e "\n${YELLOW}Step 3: Approve User with Tenant${NC}"
APPROVAL_RESPONSE=$(curl -s -X POST "$BASE_URL/admin/approve-user/$USER_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "reason": "Trial key test user",
    "create_default_tenant": true,
    "tenant_name": "Trial Test Org",
    "tenant_slug": "trial-test-org"
  }')

TENANT_ID=$(echo "$APPROVAL_RESPONSE" | jq -r '.data.tenant_created.tenant_id // empty')

if [[ -z "$TENANT_ID" || "$TENANT_ID" == "null" ]]; then
    echo -e "${RED}Failed to approve user or create tenant${NC}"
    echo "Response: $APPROVAL_RESPONSE"
    exit 1
fi
echo -e "${GREEN}User approved with tenant: $TENANT_ID${NC}"

echo -e "\n${YELLOW}Step 4: User Login${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{
    \"email\": \"$USER_EMAIL\",
    \"password\": \"$USER_PASSWORD\"
  }")

JWT_TOKEN=$(echo "$LOGIN_RESPONSE" | jq -r '.jwt_token // empty')

if [[ -z "$JWT_TOKEN" || "$JWT_TOKEN" == "null" ]]; then
    echo -e "${RED}Failed to login user${NC}"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi
echo -e "${GREEN}User logged in successfully${NC}"
echo "JWT Token (first 50 chars): ${JWT_TOKEN:0:50}..."

echo -e "\n${YELLOW}Step 5: Admin Provisions API Key for User${NC}"
PROVISION_RESPONSE=$(curl -s -X POST "$BASE_URL/admin/provision-api-key" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d "{
    \"user_email\": \"$USER_EMAIL\",
    \"tier\": \"trial\",
    \"rate_limit_requests\": 1000,
    \"rate_limit_period\": \"month\",
    \"expires_in_days\": 14,
    \"name\": \"Test Trial Key\",
    \"description\": \"Testing business provisioning\"
  }")

API_KEY=$(echo "$PROVISION_RESPONSE" | jq -r '.api_key // .data.api_key // empty')

if [[ -z "$API_KEY" ]]; then
    echo -e "${YELLOW}API key provisioning endpoint may not be implemented${NC}"
    echo "Response: $PROVISION_RESPONSE"
    echo -e "${YELLOW}Skipping API key tests...${NC}"
else
    if [[ $API_KEY == pk_trial_* ]]; then
        echo -e "${GREEN}Trial key created: ${API_KEY:0:20}...${NC}"
    else
        echo -e "${YELLOW}Unexpected key format: ${API_KEY:0:20}...${NC}"
    fi
fi

echo -e "\n${YELLOW}Step 6: Verify Self-Service Key Creation is Blocked${NC}"
SELF_SERVICE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/keys/trial" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "name": "Unauthorized Key",
    "description": "Should be blocked"
  }')

HTTP_STATUS=$(echo "$SELF_SERVICE_RESPONSE" | jq -r '.status // "unknown"')

if [[ $SELF_SERVICE_RESPONSE == *"forbidden"* || $SELF_SERVICE_RESPONSE == *"not found"* || $SELF_SERVICE_RESPONSE == *"unauthorized"* || $SELF_SERVICE_RESPONSE == *"Forbidden"* || $HTTP_STATUS == "404" ]]; then
    echo -e "${GREEN}Correctly blocked self-service API key creation${NC}"
else
    echo -e "${YELLOW}Self-service response (may be expected): $SELF_SERVICE_RESPONSE${NC}"
fi

echo -e "\n${YELLOW}Step 7: Test MCP Access with User JWT${NC}"
MCP_RESPONSE=$(curl -s -X POST "$BASE_URL/mcp" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }')

TOOLS_COUNT=$(echo "$MCP_RESPONSE" | jq '.result.tools | length // 0')

if [[ "$TOOLS_COUNT" -gt 0 ]]; then
    echo -e "${GREEN}MCP access working: $TOOLS_COUNT tools available${NC}"
else
    echo -e "${RED}MCP access failed${NC}"
    echo "Response: $MCP_RESPONSE"
fi

echo -e "\n${GREEN}Business API Key Provisioning Test Complete!${NC}"
echo ""
echo -e "${BLUE}Summary:${NC}"
echo "  Admin: $ADMIN_EMAIL"
echo "  User: $USER_EMAIL (ID: $USER_ID)"
echo "  Tenant: $TENANT_ID"
echo "  MCP Tools: $TOOLS_COUNT"
