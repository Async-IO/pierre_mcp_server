# Quick Start Guide

Get Pierre MCP Server running in 5 minutes and make your first API call.

## Prerequisites

- Rust 1.70+ (`rustc --version`)
- SQLite 3.35+ (installed by default on macOS/Linux)
- Git
- curl and jq (for testing)

## Installation

### 1. Clone and Build

```bash
# Clone repository
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server

# Build release binary
cargo build --release

# Verify build
ls -lh target/release/pierre-mcp-server
```

Build time: ~3-5 minutes on first build.

### 2. Initialize Database and Configuration

```bash
# Clean start (removes existing data)
./scripts/fresh-start.sh

# This script:
# - Removes old data/users.db database
# - Loads environment variables from .envrc
# - Generates encryption keys if needed
```

### 3. Start Server

```bash
# Start server with debug logging
source .envrc && RUST_LOG=info cargo run --bin pierre-mcp-server
```

Server starts on port 8081 (configurable via HTTP_PORT).

**Expected output**:
```
2024-01-15T10:00:00.000Z  INFO pierre_mcp_server: Starting Pierre MCP Server
2024-01-15T10:00:00.001Z  INFO pierre_mcp_server: HTTP server listening on 0.0.0.0:8081
2024-01-15T10:00:00.002Z  INFO pierre_mcp_server: MCP endpoint: /mcp
2024-01-15T10:00:00.003Z  INFO pierre_mcp_server: Health endpoint: /admin/health
```

Verify server is running:
```bash
curl http://localhost:8081/admin/health
# Expected: {"status":"healthy","version":"1.0.0"}
```

## Complete Setup Workflow

Pierre requires admin approval for new users. Use the automated script:

```bash
# In a new terminal (server must be running)
./scripts/complete-user-workflow.sh
```

This script (scripts/complete-user-workflow.sh:1-184):
1. Creates admin user (admin@pierre.mcp)
2. Registers regular user (user@example.com)
3. Approves user with tenant creation
4. Logs in user and obtains JWT token
5. Tests MCP access
6. Saves credentials to `.workflow_test_env`

**Expected output**:
```
=== Pierre MCP Server Complete User Workflow Test ===
✅ Server is running

=== Step 1: Create Admin User ===
✅ Admin created successfully
Admin token (first 50 chars): eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

=== Step 2: Register Regular User ===
✅ User registered successfully
User ID: 550e8400-e29b-41d4-a716-446655440000

=== Step 3: Approve User with Tenant Creation ===
✅ User approved with tenant created
Tenant ID: tenant_550e8400-e29b-41d4-a716-446655440000

=== Step 4: User Login ===
✅ User logged in successfully
JWT Token (first 50 chars): eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...

=== Step 5: Test MCP Access ===
✅ MCP working: 26 tools available

🎉 Complete workflow test completed successfully!
```

Load the saved environment variables:
```bash
source .workflow_test_env
echo "JWT Token ready: ${JWT_TOKEN:0:50}..."
```

## Your First API Call

### Test MCP Protocol

List available tools:
```bash
curl -X POST http://localhost:8081/mcp \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "id": 1
  }' | jq '.result.tools[] | {name: .name, description: .description}' | head -20
```

**Expected output** (first 5 tools):
```json
{
  "name": "get_activities",
  "description": "Retrieve user fitness activities from connected providers"
}
{
  "name": "get_athlete",
  "description": "Get athlete profile information"
}
{
  "name": "get_stats",
  "description": "Get athlete statistics for a specific sport type"
}
{
  "name": "analyze_activity",
  "description": "AI-powered analysis of a specific fitness activity"
}
{
  "name": "get_connection_status",
  "description": "Check OAuth provider connection status"
}
```

### Check Connection Status

```bash
curl -X POST http://localhost:8081/mcp \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_connection_status",
      "arguments": {}
    },
    "id": 2
  }' | jq .
```

**Expected output**:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"providers\":[{\"provider\":\"strava\",\"connected\":false},{\"provider\":\"fitbit\",\"connected\":false}]}"
      }
    ]
  }
}
```

## Connect to Strava (Optional)

To access real fitness data, connect a Strava account:

### 1. Get Strava OAuth URL

```bash
# Get authorization URL (src/routes/auth.rs:565-609)
STRAVA_AUTH=$(curl -s "http://localhost:8081/api/oauth/auth/strava/$USER_ID" \
  -H "Authorization: Bearer $JWT_TOKEN")

echo $STRAVA_AUTH | jq -r '.authorization_url'
```

### 2. Authorize in Browser

Copy the URL from step 1 and open in browser. You'll be redirected to Strava to authorize access.

After authorization, Strava redirects to:
```
http://localhost:8081/api/oauth/callback/strava?code=AUTH_CODE&state=STATE
```

Pierre automatically exchanges the code for tokens and stores them.

### 3. Verify Connection

```bash
curl -X POST http://localhost:8081/mcp \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_connection_status",
      "arguments": {}
    },
    "id": 3
  }' | jq '.result.content[0].text | fromjson'
```

**Expected output** (after Strava connection):
```json
{
  "providers": [
    {
      "provider": "strava",
      "connected": true,
      "expires_at": "2024-07-15T10:00:00Z",
      "scopes": "read,activity:read_all"
    },
    {
      "provider": "fitbit",
      "connected": false
    }
  ]
}
```

### 4. Fetch Activities

```bash
curl -X POST http://localhost:8081/mcp \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_activities",
      "arguments": {
        "limit": 5
      }
    },
    "id": 4
  }' | jq '.result.content[0].text | fromjson | .activities[] | {name, sport_type, distance_meters, duration_seconds}'
```

**Expected output**:
```json
{
  "name": "Morning Run",
  "sport_type": "Run",
  "distance_meters": 5000,
  "duration_seconds": 1800
}
{
  "name": "Evening Ride",
  "sport_type": "Ride",
  "distance_meters": 20000,
  "duration_seconds": 3600
}
```

## Connect to Claude Desktop

Configure Claude Desktop to use Pierre MCP Server:

### 1. Create Configuration File

**macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8081/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_JWT_TOKEN_HERE"
      }
    }
  }
}
```

Replace `YOUR_JWT_TOKEN_HERE` with the JWT token from `.workflow_test_env`:
```bash
cat .workflow_test_env | grep JWT_TOKEN
```

### 2. Restart Claude Desktop

Close and reopen Claude Desktop to load the new configuration.

### 3. Test in Claude

Try these queries in Claude:

1. "What fitness tools do you have access to?"
2. "Check my fitness provider connection status"
3. "Show me my recent activities" (if Strava connected)

Claude should respond using Pierre's MCP tools.

## Manual Setup (Alternative)

If you prefer manual steps instead of the automated script:

### 1. Create Admin User

```bash
ADMIN_RESPONSE=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{
    "email": "admin@example.com",
    "password": "SecurePass123!",
    "display_name": "System Administrator"
  }')

ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.admin_token')
echo "Admin Token: $ADMIN_TOKEN"
```

### 2. Register User

```bash
USER_RESPONSE=$(curl -s -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123",
    "display_name": "Regular User"
  }')

USER_ID=$(echo $USER_RESPONSE | jq -r '.user_id')
echo "User ID: $USER_ID"
```

### 3. Approve User

```bash
APPROVAL_RESPONSE=$(curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -d '{
    "reason": "Approved for access",
    "create_default_tenant": true,
    "tenant_name": "My Organization",
    "tenant_slug": "my-org"
  }')

echo $APPROVAL_RESPONSE | jq
```

### 4. User Login

```bash
LOGIN_RESPONSE=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "userpass123"
  }')

JWT_TOKEN=$(echo $LOGIN_RESPONSE | jq -r '.jwt_token')
echo "JWT Token: $JWT_TOKEN"
```

## Troubleshooting

### Server Won't Start

**Issue**: `Address already in use` error

**Solution**: Change port or kill existing process
```bash
# Use different port
HTTP_PORT=9081 cargo run --bin pierre-mcp-server

# Or kill existing process
lsof -ti:8081 | xargs kill -9
```

### Database Error

**Issue**: `unable to open database file`

**Solution**: Ensure data directory exists
```bash
mkdir -p data
chmod 755 data
```

### Admin Setup Fails

**Issue**: `{"error": "Admin user already exists"}`

**Solution**: Admin already created. Skip to user registration, or reset:
```bash
./scripts/fresh-start.sh  # Removes all data
```

### Invalid JWT Token

**Issue**: `{"error": "authentication_failed"}`

**Causes**:
- Token expired (24 hour TTL)
- Wrong token format
- Server restarted (tokens invalidated)

**Solution**: Login again to get fresh token:
```bash
curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "userpass123"}' \
  | jq -r '.jwt_token'
```

### MCP Connection Timeout

**Issue**: Claude Desktop shows "Connection timeout"

**Causes**:
- Server not running
- Firewall blocking port 8081
- Wrong URL in configuration

**Solution**: Verify server accessibility
```bash
curl http://localhost:8081/admin/health
```

### Strava OAuth Fails

**Issue**: `Invalid redirect_uri` error

**Cause**: STRAVA_REDIRECT_URI environment variable not set

**Solution**: Configure Strava OAuth credentials
```bash
# In .envrc
export STRAVA_CLIENT_ID="your_strava_app_client_id"
export STRAVA_CLIENT_SECRET="your_strava_app_client_secret"
export STRAVA_REDIRECT_URI="http://localhost:8081/api/oauth/callback/strava"

source .envrc
```

Get credentials from https://www.strava.com/settings/api

## Next Steps

### Learn More

- [API Reference](../developer-guide/14-api-reference.md) - Complete REST API documentation
- [MCP Protocol](../developer-guide/04-mcp-protocol.md) - MCP JSON-RPC protocol details
- [OAuth 2.0 Server](../developer-guide/oauth2-authorization-server.md) - OAuth 2.0 client registration
- [Authentication](../developer-guide/06-authentication.md) - JWT authentication and claims
- [Tool Development](../developer-guide/tool-development.md) - Creating custom fitness tools

### Production Deployment

For production use:

1. **Use PostgreSQL** instead of SQLite
2. **Enable HTTPS** with reverse proxy (nginx/Apache)
3. **Set secure environment variables** (not defaults)
4. **Configure monitoring** and logging
5. **Set up automated backups**

See [Deployment Guide](../operations/deployment-guide.md) for details.

### Develop Custom Tools

Extend Pierre with custom fitness analysis tools:

```rust
// src/protocols/universal/handlers/custom.rs
pub async fn my_custom_tool(
    args: ToolArguments,
    resources: &ServerResources,
) -> Result<ToolResponse> {
    // Your custom logic here
    Ok(ToolResponse::success("Custom tool executed"))
}
```

Register in `src/protocols/universal/tool_registry.rs`.

See [Tool Development Guide](../developer-guide/tool-development.md).

### Integrate with Your Application

Use Pierre as a backend for your fitness application:

- Python SDK: `pip install pierre-mcp-client`
- JavaScript SDK: `npm install @pierre/mcp-client`
- Direct HTTP: See [API Reference](../developer-guide/14-api-reference.md)

## Summary

You've now:
- ✅ Built and started Pierre MCP Server
- ✅ Created admin and user accounts
- ✅ Made your first MCP API call
- ✅ (Optional) Connected Strava for real fitness data
- ✅ (Optional) Configured Claude Desktop integration

**Credentials from this quick start**:
- Admin: `admin@pierre.mcp` / `adminpass123`
- User: `user@example.com` / `userpass123`
- JWT Token: Saved in `.workflow_test_env`

**Server endpoints**:
- Health check: `http://localhost:8081/admin/health`
- MCP protocol: `http://localhost:8081/mcp`
- REST API: `http://localhost:8081/api/*`
- OAuth 2.0: `http://localhost:8081/oauth2/*`

For questions or issues, see [Troubleshooting Guide](../developer-guide/16-testing-strategy.md) or open an issue on GitHub.
