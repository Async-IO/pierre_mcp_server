#!/bin/bash
# SPDX-License-Identifier: MIT OR Apache-2.0
# Copyright (c) 2025 Pierre Fitness Intelligence
# ABOUTME: Complete user registration and approval workflow test script
# ABOUTME: Implements all 5 steps from HOW_TO_REGISTER_A_USER.md with proper error handling
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

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

# Set default provider to strava for OAuth testing
export PIERRE_DEFAULT_PROVIDER=strava

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

# Step 1: Create or Get Admin Token
echo -e "\n${BLUE}=== Step 1: Create or Get Admin Token ===${NC}"

ADMIN_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@pierre.mcp",
    "password": "adminpass123",
    "display_name": "System Administrator"
  }')

# Extract admin token for future use
ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.data.admin_token')

if [[ "$ADMIN_TOKEN" == "null" || -z "$ADMIN_TOKEN" ]]; then
    # Admin already exists - generate admin token using CLI tool
    echo -e "${YELLOW}Admin already exists, generating admin token via CLI...${NC}"

    # Generate admin token using admin-setup CLI
    ADMIN_SETUP_OUTPUT=$(RUST_LOG=warn cargo run --bin admin-setup -- generate-token --service workflow_script --super-admin --expires-days 1 2>&1)

    # Extract the JWT token from CLI output
    # Format: "Key YOUR JWT TOKEN (SAVE THIS NOW):" then "======" then the actual token then "======"
    # The token starts with "eyJ" (base64-encoded JSON header)
    ADMIN_TOKEN=$(echo "$ADMIN_SETUP_OUTPUT" | grep -E "^eyJ" | head -1 | tr -d '[:space:]')

    if [[ -z "$ADMIN_TOKEN" || ! "$ADMIN_TOKEN" =~ ^eyJ ]]; then
        echo -e "${RED}❌ Failed to generate admin token via CLI${NC}"
        echo "CLI Output: $ADMIN_SETUP_OUTPUT"
        exit 1
    fi

    echo -e "${GREEN}✅ Admin token generated via CLI (admin already existed)${NC}"
else
    echo -e "${GREEN}✅ Admin created successfully${NC}"
fi
echo "Admin token (first 50 chars): ${ADMIN_TOKEN:0:50}..."

# Step 2: Register Regular User (or skip if exists)
echo -e "\n${BLUE}=== Step 2: Register Regular User ===${NC}"

USER_RESPONSE=$(curl -s -X POST http://localhost:$HTTP_PORT/api/auth/register \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123",
    "display_name": "Regular User"
  }')

# Extract user ID for approval
USER_ID=$(echo $USER_RESPONSE | jq -r '.user_id')
USER_ALREADY_EXISTS=false

if [[ "$USER_ID" == "null" || -z "$USER_ID" ]]; then
    # Check if user already exists - try to login and extract user_id from response
    echo -e "${YELLOW}User registration returned no ID, checking if user exists...${NC}"

    CHECK_LOGIN=$(curl -s -X POST http://localhost:$HTTP_PORT/api/auth/login \
      -H "Content-Type: application/json" \
      -d '{
        "email": "user@example.com",
        "password": "userpass123"
      }')

    USER_ID=$(echo $CHECK_LOGIN | jq -r '.user.user_id')

    if [[ "$USER_ID" != "null" && -n "$USER_ID" ]]; then
        USER_ALREADY_EXISTS=true
        echo -e "${GREEN}✅ User already exists (ID: $USER_ID)${NC}"
    else
        echo -e "${RED}❌ Failed to register user or find existing user${NC}"
        echo "Register Response: $USER_RESPONSE"
        echo "Login Response: $CHECK_LOGIN"
        exit 1
    fi
else
    echo -e "${GREEN}✅ User registered successfully${NC}"
    echo "User ID: $USER_ID"
fi

# Step 3: Approve User WITH Tenant Creation (skip if user already exists and is approved)
echo -e "\n${BLUE}=== Step 3: Approve User with Tenant Creation ===${NC}"

if [[ "$USER_ALREADY_EXISTS" == "true" ]]; then
    echo -e "${YELLOW}User already exists, checking tenant assignment...${NC}"

    # Try to get tenant from login response
    TENANT_ID=$(echo $CHECK_LOGIN | jq -r '.user.tenant_id // empty')

    if [[ -z "$TENANT_ID" || "$TENANT_ID" == "null" ]]; then
        # User exists but might not have tenant - try approval anyway
        APPROVAL_RESPONSE=$(curl -s -X POST "http://localhost:$HTTP_PORT/admin/approve-user/$USER_ID" \
          -H "Content-Type: application/json" \
          -H "Authorization: Bearer $ADMIN_TOKEN" \
          -d '{
            "reason": "User registration approved",
            "create_default_tenant": true,
            "tenant_name": "User Organization",
            "tenant_slug": "user-org"
          }')

        TENANT_ID=$(echo $APPROVAL_RESPONSE | jq -r '.data.tenant_created.tenant_id // .data.tenant_id // empty')

        if [[ -z "$TENANT_ID" || "$TENANT_ID" == "null" ]]; then
            # Check if user is already approved with existing tenant
            FINAL_LOGIN=$(curl -s -X POST http://localhost:$HTTP_PORT/api/auth/login \
              -H "Content-Type: application/json" \
              -d '{
                "email": "user@example.com",
                "password": "userpass123"
              }')

            # Try to extract tenant_id from JWT claims
            JWT=$(echo $FINAL_LOGIN | jq -r '.jwt_token')
            if [[ -n "$JWT" && "$JWT" != "null" ]]; then
                # Decode JWT payload (base64) to get tenant_id
                PAYLOAD=$(echo $JWT | cut -d'.' -f2 | base64 -d 2>/dev/null || echo "{}")
                TENANT_ID=$(echo $PAYLOAD | jq -r '.tenant_id // empty')
            fi

            if [[ -z "$TENANT_ID" || "$TENANT_ID" == "null" ]]; then
                echo -e "${RED}❌ Could not find or create tenant for user${NC}"
                exit 1
            fi
        fi
    fi

    echo -e "${GREEN}✅ User already approved with tenant${NC}"
    echo "Tenant ID: $TENANT_ID"
else
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
    TENANT_ID=$(echo $APPROVAL_RESPONSE | jq -r '.data.tenant_created.tenant_id')

    if [[ "$TENANT_ID" == "null" || -z "$TENANT_ID" ]]; then
        echo -e "${RED}❌ Failed to approve user or create tenant${NC}"
        exit 1
    fi

    echo -e "${GREEN}✅ User approved with tenant created${NC}"
    echo "Tenant ID: $TENANT_ID"
fi

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