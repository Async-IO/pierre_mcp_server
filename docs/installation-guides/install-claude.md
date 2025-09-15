# Installing Pierre MCP Server with Claude Desktop

Install and configure Pierre MCP Server to work with Claude Desktop.

## Prerequisites

- Claude Desktop installed
- Node.js and npm (for development)
- Git
- Rust (if building from source)

## Installation

### Automated Setup

1. Clone and build the server:
```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

2. Run automated setup:
```bash
# Clean start and run complete workflow
./scripts/fresh-start.sh
source .envrc && RUST_LOG=debug cargo run --bin pierre-mcp-server &

# Complete setup (creates admin, user, tenant, and gets JWT token)
./scripts/complete-user-workflow.sh

# Load saved environment variables
source .workflow_test_env
echo "JWT Token: ${JWT_TOKEN:0:50}..."
```

### Manual Setup

1. Start the server:
```bash
cargo run --bin pierre-mcp-server
```

2. Create admin account:
```bash
ADMIN_RESPONSE=$(curl -s -X POST http://localhost:8081/admin/setup \
  -H "Content-Type: application/json" \
  -d '{"email": "admin@example.com", "password": "SecurePass123!", "display_name": "Admin"}')

ADMIN_TOKEN=$(echo $ADMIN_RESPONSE | jq -r '.admin_token')
```

3. Register and approve user:
```bash
USER_ID=$(curl -s -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123", "display_name": "User"}' | jq -r '.user_id')

curl -s -X POST "http://localhost:8081/admin/approve-user/$USER_ID" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Approved", "create_default_tenant": true, "tenant_name": "User Org", "tenant_slug": "user-org"}'
```

4. Get JWT token:
```bash
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123"}' | jq -r '.jwt_token')
```

## Claude Desktop Configuration

### Configuration File Location

The Claude Desktop configuration file is located at:

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`

### Configuration Content

Create or update the configuration file with the following content:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8080/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_JWT_TOKEN_FROM_ABOVE"
      }
    }
  }
}
```

Replace `YOUR_JWT_TOKEN_FROM_ABOVE` with the JWT token obtained from the authentication process.

### Alternative Configuration (Direct Connection)

If you prefer a direct WebSocket connection (requires server to be running):

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "ws://127.0.0.1:8080/ws",
      "headers": {
        "Authorization": "Bearer YOUR_JWT_TOKEN_FROM_ABOVE"
      }
    }
  }
}
```

## Fitness Provider Setup

### Connect to Strava

1. **Start OAuth flow:**
```bash
curl "http://localhost:8081/api/oauth/strava/auth" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

2. **Complete authentication in browser** - this will redirect you to Strava for authorization

3. **Verify connection:**
```bash
curl "http://localhost:8081/api/oauth/providers/status" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

## Testing the Integration

### Restart Claude Desktop

After updating the configuration, restart Claude Desktop to load the new MCP server configuration.

### Test Basic Functionality

Try these commands in Claude Desktop:

1. **Check connection status:**
   "What's my fitness provider connection status?"

2. **Get recent activities:**
   "Show me my recent fitness activities"

3. **Get athlete information:**
   "What's my athlete profile information?"

4. **Analyze an activity:**
   "Analyze my latest running activity"

### Verify MCP Tools

You can verify that Pierre MCP Server tools are available by asking Claude:
"What fitness-related tools do you have access to?"

Claude should respond with a list of available tools including:
- `get_activities`
- `get_athlete` 
- `get_stats`
- `get_activity_intelligence`
- `analyze_activity`
- And more...

## Troubleshooting

### Claude Desktop Not Connecting

1. **Check configuration file syntax:**
```bash
# Validate JSON syntax
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json | jq .
```

2. **Verify server is running:**
```bash
curl http://localhost:8080/health
```

3. **Check Claude Desktop logs:**
   - macOS: `~/Library/Logs/Claude/`
   - Windows: `%LOCALAPPDATA%\Claude\logs\`

### MCP Tools Not Available

1. **Verify JWT token is valid:**
```bash
curl -H "Authorization: Bearer $JWT_TOKEN" http://localhost:8081/api/auth/verify
```

2. **Check server logs:**
```bash
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

3. **Test MCP connection directly:**
```bash
# Test the MCP endpoint
curl -X POST "http://127.0.0.1:8080/mcp" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $JWT_TOKEN" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
```

### OAuth Issues

1. **Verify environment variables:**
```bash
echo $STRAVA_CLIENT_ID
echo $STRAVA_CLIENT_SECRET
```

2. **Check OAuth configuration:**
```bash
curl "http://localhost:8081/api/oauth/config" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

3. **Reset OAuth tokens:**
```bash
curl -X POST "http://localhost:8081/api/oauth/strava/disconnect" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

## Advanced Configuration

### Custom Server Ports

If you need to use different ports:

```bash
# Start server with custom ports
MCP_PORT=9080 HTTP_PORT=9081 cargo run --bin pierre-mcp-server
```

Update your Claude Desktop configuration accordingly:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:9080/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_JWT_TOKEN"
      }
    }
  }
}
```

### Environment Variables

Create a `.env` file for persistent configuration:

```bash
# Core Configuration
DATABASE_URL=sqlite:./data/pierre.db
PIERRE_MASTER_ENCRYPTION_KEY=your_32_byte_base64_key

# Server Ports
MCP_PORT=8080
HTTP_PORT=8081

# OAuth Providers
STRAVA_CLIENT_ID=your_strava_client_id
STRAVA_CLIENT_SECRET=your_strava_client_secret

# Logging
RUST_LOG=info
```

### Production Deployment

For production use with Claude Desktop:

1. **Use PostgreSQL database:**
```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre
```

2. **Enable HTTPS:**
```bash
# Configure reverse proxy (nginx/Apache)
# Point Claude Desktop to https://your-domain.com/mcp
```

3. **Set up systemd service:**
```bash
# Create /etc/systemd/system/pierre-mcp-server.service
sudo systemctl enable pierre-mcp-server
sudo systemctl start pierre-mcp-server
```

## Getting Help

If you encounter issues:

1. Check the [main troubleshooting guide](../developer-guide/16-testing-strategy.md)
2. Review Claude Desktop's MCP documentation
3. Open an issue on GitHub with:
   - Your operating system and version
   - Claude Desktop version
   - Pierre MCP Server version
   - Configuration file content (remove sensitive tokens)
   - Error messages from logs