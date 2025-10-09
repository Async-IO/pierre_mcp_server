#!/bin/bash
# ABOUTME: Complete end-to-end test script for user registration and approval workflow  
# ABOUTME: Tests server-first admin setup, user registration, and admin approval process
#
# Licensed under either of Apache License, Version 2.0 or MIT License at your option.
# Copyright ©2025 Async-IO.org

set -e

echo "=== FRESH TEST - COMPLETE WORKFLOW ==="

# Clean up any existing database
echo "🧹 Cleaning database..."
./scripts/fresh-start.sh

# Check server health first
echo "🔍 Checking server health..."
curl -s http://localhost:8081/admin/health | jq . || {
    echo "❌ Server health check failed - is the server running?"
    exit 1
}

# Step 1: Create admin user via server endpoint
echo "1️⃣ Creating admin user..."
ADMIN_RESPONSE=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "cheffamille@apache.org",
    "password": "testpass123",
    "display_name": "ChefFamille Admin"
  }')

echo "Admin setup response:"
echo "$ADMIN_RESPONSE" | jq .

# Extract admin token
ADMIN_TOKEN=$(echo "$ADMIN_RESPONSE" | jq -r .admin_token)
if [[ "$ADMIN_TOKEN" == "null" || -z "$ADMIN_TOKEN" ]]; then
    echo "❌ Failed to extract admin token"
    exit 1
fi

echo "Admin token extracted: ${ADMIN_TOKEN:0:50}..."

# Step 2: Register a regular user
echo "2️⃣ Registering regular user..."
USER_RESPONSE=$(curl -s -X POST http://localhost:8081/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "cheffamille@apache.org",
    "password": "userpass123",
    "display_name": "ChefFamille User"
  }')

echo "User registration response:"
echo "$USER_RESPONSE" | jq .

# Extract user ID
USER_ID=$(echo "$USER_RESPONSE" | jq -r .user_id)
if [[ "$USER_ID" == "null" || -z "$USER_ID" ]]; then
    echo "❌ Failed to extract user ID"
    exit 1
fi

echo "User registered with ID: $USER_ID"

# Step 3: Approve the user using admin token
echo "3️⃣ Approving user..."
APPROVAL_RESPONSE=$(curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "reason": "Test user approval"
  }')

echo "User approval response:"
echo "$APPROVAL_RESPONSE" | jq .

# Check if approval was successful
SUCCESS=$(echo "$APPROVAL_RESPONSE" | jq -r .success)
if [[ "$SUCCESS" != "true" ]]; then
    echo "❌ User approval failed"
    exit 1
fi

echo "✅ User approved successfully!"

# Step 4: Generate Claude Desktop config (optional)
echo "4️⃣ Generating Claude Desktop MCP config..."

# Generate a service token for Claude Desktop
CONFIG_RESPONSE=$(curl -s -X POST http://localhost:8081/admin/tokens \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "service_name": "claude_desktop_mcp_test",
    "service_description": "Claude Desktop MCP integration token",
    "is_super_admin": false,
    "expires_in_days": 365,
    "permissions": ["ManageUsers", "ProvisionKeys"]
  }')

echo "Claude Desktop token response:"
echo "$CONFIG_RESPONSE" | jq .

MCP_TOKEN=$(echo "$CONFIG_RESPONSE" | jq -r .data.jwt_token)
if [[ "$MCP_TOKEN" == "null" || -z "$MCP_TOKEN" ]]; then
    echo "⚠️ Warning: Failed to generate MCP token, but main workflow succeeded"
else
    echo "Claude Desktop MCP config:"
    cat << EOF
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "stdio",
      "args": [
        "curl",
        "-X", "POST",
        "http://localhost:8080/mcp",
        "-H", "Authorization: Bearer $MCP_TOKEN",
        "-H", "Content-Type: application/json",
        "-d", "@-"
      ]
    }
  }
}
EOF
fi

echo ""
echo "🎉 COMPLETE WORKFLOW TEST PASSED!"
echo "✅ Admin created successfully"
echo "✅ User registered successfully"  
echo "✅ User approved successfully"
echo "✅ System ready for production use"