#!/bin/bash
# ABOUTME: Refreshes the Stitch MCP access token in Claude Code config
# ABOUTME: Run this when the token expires (~1 hour) or at session start

set -e

# Get fresh token from gcloud
ACCESS_TOKEN=$(gcloud auth application-default print-access-token 2>/dev/null)

if [ -z "$ACCESS_TOKEN" ]; then
    echo "Error: Failed to get access token. Run: gcloud auth application-default login"
    exit 1
fi

# Update Claude Code config
claude mcp remove stitch -s user 2>/dev/null || true
claude mcp add stitch \
  --transport http https://stitch.googleapis.com/mcp \
  --header "Authorization: Bearer ${ACCESS_TOKEN}" \
  --header "X-Goog-User-Project: pierre-fitness-intelligence" \
  -s user

echo "✅ Stitch MCP token refreshed successfully"
echo "   Token will expire in ~1 hour"
