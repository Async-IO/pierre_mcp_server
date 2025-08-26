#!/bin/bash

# Test script for business API key provisioning system

echo "Testing Business API Key Provisioning System"
echo "============================================="

# Set base URL
BASE_URL="http://localhost:8081"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test user credentials
EMAIL="trial_test@example.com"
PASSWORD="testpassword123"

echo -e "\n${YELLOW}1. Register test user${NC}"
REGISTER_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\",\"display_name\":\"Trial Test User\"}")

echo "Response: $REGISTER_RESPONSE"

echo -e "\n${YELLOW}2. Login to get JWT token${NC}"
LOGIN_RESPONSE=$(curl -s -X POST "$BASE_URL/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"email\":\"$EMAIL\",\"password\":\"$PASSWORD\"}")

JWT_TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"jwt_token":"[^"]*"' | cut -d'"' -f4)
echo "JWT Token obtained: ${JWT_TOKEN:0:20}..."

echo -e "\n${YELLOW}3. Setup admin authentication and provision API key${NC}"
# Note: In deployment, admin tokens are generated via /admin/setup API
# For testing, we simulate admin token (this would be a real admin JWT in production)
ADMIN_TOKEN="simulated_admin_token_for_testing"

# Provision API key via admin endpoint (simulated for testing)
echo "Simulating admin API key provisioning..."
TRIAL_KEY_RESPONSE=$(curl -s -X POST "$BASE_URL/admin/provision-api-key" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d "{
    \"user_email\": \"$EMAIL\",
    \"tier\": \"trial\",
    \"rate_limit_requests\": 1000,
    \"rate_limit_period\": \"month\",
    \"expires_in_days\": 14,
    \"name\": \"Test Trial Key\",
    \"description\": \"Testing business provisioning\"
  }")

echo "Trial Key Response: $TRIAL_KEY_RESPONSE"

# Extract the API key
API_KEY=$(echo "$TRIAL_KEY_RESPONSE" | grep -o '"api_key":"[^"]*"' | cut -d'"' -f4)
if [[ $API_KEY == pk_trial_* ]]; then
  echo -e "${GREEN}✓ Trial key created successfully: ${API_KEY:0:20}...${NC}"
else
  echo -e "${RED}✗ Failed to create trial key${NC}"
  exit 1
fi

echo -e "\n${YELLOW}4. List API keys to verify trial key${NC}"
LIST_RESPONSE=$(curl -s -X GET "$BASE_URL/api/keys" \
  -H "Authorization: Bearer $JWT_TOKEN")

echo "API Keys: $LIST_RESPONSE"

echo -e "\n${YELLOW}5. Test business provisioning controls${NC}"
# In business model, only admins can provision keys
# Regular users cannot create keys themselves
echo "Testing that regular users cannot self-provision keys..."
SELF_SERVICE_RESPONSE=$(curl -s -X POST "$BASE_URL/api/keys/trial" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "name": "Unauthorized Key",
    "description": "Should be blocked"
  }')

if [[ $SELF_SERVICE_RESPONSE == *"forbidden"* || $SELF_SERVICE_RESPONSE == *"not found"* || $SELF_SERVICE_RESPONSE == *"unauthorized"* ]]; then
  echo -e "${GREEN}✓ Correctly blocked self-service API key creation${NC}"
else
  echo -e "${RED}✗ Should have blocked self-service API key creation${NC}"
  echo "Response: $SELF_SERVICE_RESPONSE"
fi

echo -e "\n${YELLOW}6. Test trial key authentication${NC}"
# Try to use the trial key to access the MCP server
TEST_RESPONSE=$(curl -s -X POST "$BASE_URL/dashboard/overview" \
  -H "Authorization: $API_KEY")

echo "Trial key test response: $TEST_RESPONSE"

echo -e "\n${GREEN}Business API Key Provisioning Test Complete!${NC}"
echo -e "\n${YELLOW}Note:${NC} In deployment:"
echo "  • Admin tokens are generated via /admin/setup API"
echo "  • Only administrators can provision API keys for users"
echo "  • Users receive keys through secure channels (email/dashboard)"
echo "  • Self-service key creation is disabled for security"