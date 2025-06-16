#!/bin/bash

# Test script for trial API keys

echo "Testing Trial API Key System"
echo "============================"

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

echo -e "\n${YELLOW}3. Create a trial API key${NC}"
TRIAL_KEY_RESPONSE=$(curl -s -X POST "$BASE_URL/api/keys/trial" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "name": "My Trial Key",
    "description": "Testing trial functionality"
  }')

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

echo -e "\n${YELLOW}5. Try to create another trial key (should fail)${NC}"
SECOND_TRIAL_RESPONSE=$(curl -s -X POST "$BASE_URL/api/keys/trial" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{
    "name": "Second Trial Key",
    "description": "Should fail"
  }')

if [[ $SECOND_TRIAL_RESPONSE == *"already has a trial"* ]]; then
  echo -e "${GREEN}✓ Correctly prevented creating second trial key${NC}"
else
  echo -e "${RED}✗ Should have prevented second trial key${NC}"
fi

echo -e "\n${YELLOW}6. Test trial key authentication${NC}"
# Try to use the trial key to access the MCP server
TEST_RESPONSE=$(curl -s -X POST "$BASE_URL/dashboard/overview" \
  -H "Authorization: $API_KEY")

echo "Trial key test response: $TEST_RESPONSE"

echo -e "\n${GREEN}Trial Key System Test Complete!${NC}"