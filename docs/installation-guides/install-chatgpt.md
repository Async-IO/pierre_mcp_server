# Installing Pierre MCP Server with ChatGPT Desktop

Install and configure Pierre MCP Server for ChatGPT Desktop integration.

## Prerequisites

- ChatGPT Desktop application installed
- Node.js and npm 
- Git
- Rust (if building from source)

## Installation

### Automated Setup

1. Clone and build:
```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server
cargo build --release
```

2. Complete automated setup:
```bash
# Clean database and start server
./scripts/fresh-start.sh
source .envrc && RUST_LOG=debug cargo run --bin pierre-mcp-server &

# Run complete workflow (admin + user + tenant + JWT token)
./scripts/complete-user-workflow.sh

# Load environment with JWT token
source .workflow_test_env
echo "JWT Token ready: ${JWT_TOKEN:0:50}..."
```

## ChatGPT Desktop Configuration

### Configuration File Location

ChatGPT Desktop uses the following configuration file locations:

- **macOS**: `~/Library/Application Support/ChatGPT/config.json`
- **Windows**: `%APPDATA%\ChatGPT\config.json`

### Create Configuration

If the configuration file doesn't exist, create it:

```bash
# macOS
mkdir -p ~/Library/Application\ Support/ChatGPT/
touch ~/Library/Application\ Support/ChatGPT/config.json

# Windows (PowerShell)
New-Item -Path "$env:APPDATA\ChatGPT" -ItemType Directory -Force
New-Item -Path "$env:APPDATA\ChatGPT\config.json" -ItemType File -Force
```

### Configuration Content

Add the following configuration:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "/path/to/pierre_mcp_server/scripts/mcp-client.sh",
      "env": {
        "PIERRE_JWT_TOKEN": "YOUR_JWT_TOKEN_FROM_SETUP",
        "PIERRE_SERVER_URL": "http://127.0.0.1:8080/mcp"
      }
    }
  }
}
```

**Note**: Replace `/path/to/pierre_mcp_server` with your actual installation path and `YOUR_JWT_TOKEN_FROM_SETUP` with the token from the automated setup.

## Fitness Provider Integration

### Connect to Strava

1. **Initiate OAuth flow:**
```bash
curl "http://localhost:8081/api/oauth/strava/auth" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

2. **Complete authorization in browser** - you'll be redirected to Strava

3. **Verify connection:**
```bash
curl "http://localhost:8081/api/oauth/providers/status" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

## Testing with ChatGPT Desktop

### Restart Application

After updating the configuration file, restart ChatGPT Desktop to load the MCP server configuration.

### Test Commands

Try these fitness-related queries in ChatGPT Desktop:

1. **Connection check:**
   "Check my fitness data connection status"

2. **Recent activities:**
   "What are my recent workout activities?"

3. **Athlete profile:**
   "Show me my fitness profile information"

4. **Activity analysis:**
   "Analyze my most recent running activity"

5. **Performance trends:**
   "What are my recent performance trends?"

### Verify Tool Access

Ask ChatGPT: "What fitness tools do you have access to?"

You should see tools like:
- Activity data retrieval
- Performance analysis
- Goal tracking
- Weather integration
- Custom analytics

## Troubleshooting

### ChatGPT Desktop Not Loading MCP Server

1. **Verify configuration syntax:**
```bash
# Check JSON is valid
cat ~/Library/Application\ Support/ChatGPT/config.json | jq .
```

2. **Check server status:**
```bash
curl http://localhost:8080/health
```

3. **Review application logs:**
   - macOS: Look in Console.app for ChatGPT entries
   - Windows: Check Event Viewer for application errors

### MCP Connection Issues

1. **Test MCP client script directly:**
```bash
export PIERRE_JWT_TOKEN="$JWT_TOKEN"
export PIERRE_SERVER_URL="http://127.0.0.1:8080/mcp"
./scripts/mcp-client.sh
```

2. **Verify token validity:**
```bash
curl -H "Authorization: Bearer $JWT_TOKEN" \
  http://localhost:8081/api/auth/verify
```

3. **Check server logs:**
```bash
RUST_LOG=debug cargo run --bin pierre-mcp-server
```

### No Fitness Data Available

1. **Verify OAuth connection:**
```bash
curl "http://localhost:8081/api/oauth/providers/status" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

2. **Re-authenticate with Strava:**
```bash
# Disconnect and reconnect
curl -X POST "http://localhost:8081/api/oauth/strava/disconnect" \
  -H "Authorization: Bearer $JWT_TOKEN"

curl "http://localhost:8081/api/oauth/strava/auth" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

3. **Test data retrieval:**
```bash
curl "http://localhost:8081/api/activities" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

## Advanced Configuration

### Custom Ports Configuration

If using non-standard ports:

```bash
# Start with custom ports
MCP_PORT=9080 HTTP_PORT=9081 cargo run --bin pierre-mcp-server
```

Update ChatGPT Desktop config:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "env": {
        "PIERRE_JWT_TOKEN": "YOUR_JWT_TOKEN",
        "PIERRE_SERVER_URL": "http://127.0.0.1:9080/mcp"
      }
    }
  }
}
```

### Environment Variables

Create `.env` file for configuration:

```bash
# Database
DATABASE_URL=sqlite:./data/pierre.db
PIERRE_MASTER_ENCRYPTION_KEY=your_32_byte_key

# Servers
MCP_PORT=8080
HTTP_PORT=8081

# OAuth
STRAVA_CLIENT_ID=your_client_id
STRAVA_CLIENT_SECRET=your_client_secret

# Logging
RUST_LOG=info
```

### Production Setup

For production deployment:

1. **Use PostgreSQL:**
```bash
DATABASE_URL=postgresql://user:pass@localhost:5432/pierre
```

2. **Enable HTTPS:**
```bash
# Use reverse proxy with SSL
# Update config to https://your-domain.com/mcp
```

3. **Process management:**
```bash
# Use systemd, Docker, or process manager
sudo systemctl enable pierre-mcp-server
```

## ChatGPT Desktop Specific Features

### Context Awareness

ChatGPT Desktop with Pierre MCP Server can:

- Access your fitness history during conversations
- Provide context-aware workout recommendations
- Analyze performance trends over time
- Correlate activities with weather and location data

### Multi-modal Analysis

If your ChatGPT Desktop supports images, you can:

- Upload workout screenshots for analysis
- Share fitness charts for interpretation
- Get visual feedback on form and technique

### Conversation Continuity

Pierre MCP Server maintains state across ChatGPT sessions:

- Goal progress tracking
- Performance metric history
- Personalized recommendations based on past conversations

## Security Considerations

### Token Management

1. **Rotate JWT tokens regularly:**
```bash
# Get new token
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "pass123"}' | jq -r '.jwt_token')
```

2. **Use environment variables:**
```bash
# Store in .env file, not in config directly
echo "PIERRE_JWT_TOKEN=$JWT_TOKEN" >> .env
```

### Network Security

1. **Use HTTPS in production**
2. **Implement rate limiting**
3. **Enable audit logging**
4. **Regular security updates**

## Getting Help

For ChatGPT Desktop specific issues:

1. Check ChatGPT Desktop's official MCP documentation
2. Review the [main installation guide](../getting-started.md)
3. Create a GitHub issue with:
   - ChatGPT Desktop version
   - Operating system
   - Configuration file (sanitized)
   - Error messages
   - Steps to reproduce

## Known Limitations

- ChatGPT Desktop MCP support may vary by version
- Some features may require specific ChatGPT subscription levels
- File upload integration depends on ChatGPT Desktop capabilities
- WebSocket connections may have different timeout behaviors compared to Claude Desktop