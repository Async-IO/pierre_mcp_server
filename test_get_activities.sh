#!/bin/bash
# ABOUTME: Test script to reproduce get_activities failure via stdio transport
# ABOUTME: Simulates Claude Desktop's exact request pattern

set -e

# Get JWT token from database
USER_ID="0d7aca2b-64a1-4bee-8b9f-1bcaba7f453e"
JWT_SECRET="O440hcJ2Zm6suziFI7L495k81F6N3zCzbw0FDGripuoCBhJoNVMPGuF7XWC8ptwa"

# Generate a test JWT token
# For testing, we'll use an existing valid token from the logs
JWT_TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIwZDdhY2EyYi02NGExLTRiZWUtOGI5Zi0xYmNhYmE3ZjQ1M2UiLCJlbWFpbCI6Im9hdXRoXzBkN2FjYTJiLTY0YTEtNGJlZS04YjlmLTFiY2FiYTdmNDUzZUBzeXN0ZW0ubG9jYWwiLCJpYXQiOjE3NTkxOTE2MDAwMDAsImV4cCI6MTc1OTI3ODAwMCwicHJvdmlkZXJzIjpbInJlYWQ6Zml0bmVzcyIsIndyaXRlOmZpdG5lc3MiXX0"

echo "=== Testing get_activities via stdio ==="
echo ""

# Test via stdin/stdout
(
  # Initialize
  echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"clientInfo":{"name":"test-client","version":"1.0"}}}'

  # Wait for response
  sleep 1

  # Send initialized notification
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}'

  sleep 1

  # List tools (with auth)
  echo "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\",\"auth_token\":\"Bearer ${JWT_TOKEN}\"}"

  sleep 1

  # Call get_activities
  echo "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"get_activities\",\"arguments\":{\"provider\":\"strava\",\"limit\":10}},\"auth_token\":\"Bearer ${JWT_TOKEN}\"}"

  sleep 2

) | ./target/debug/pierre-mcp-server 2>&1 | tee /tmp/stdio-test.log

echo ""
echo "=== Test complete - check /tmp/stdio-test.log for output ==="