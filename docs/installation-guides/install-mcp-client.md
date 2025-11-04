# Installing Pierre MCP Client

Install and configure the Pierre MCP Client SDK for any MCP-compatible application (Claude Desktop, ChatGPT, or custom MCP clients).

## Prerequisites

- **MCP-compatible application** installed (Claude Desktop, ChatGPT Desktop, etc.)
- **Node.js 18+** and npm
- **Pierre Fitness Platform** running (see [main README](../../README.md) for server setup)

## Quick Start

### 1. Install the SDK

**Option A: Install from npm (Recommended)**

```bash
npm install -g pierre-mcp-client@next
```

The package is published with the `@next` tag during v0.x development.

**Option B: Use npx (No installation required)**

Skip installation and use npx directly in your MCP client configuration.

**Option C: Build from source**

```bash
git clone https://github.com/Async-IO/pierre_mcp_server.git
cd pierre_mcp_server/sdk
npm install
npm run build
```

### 2. Start Pierre Fitness Platform

```bash
# If you haven't already, start the server
cd pierre_mcp_server
cargo run --bin pierre-mcp-server
```

The platform will start on port 8081 by default.

### 3. Configure Your MCP Client

The configuration varies slightly by MCP client but follows the same pattern.

## Configuration by MCP Client

### Claude Desktop

**Configuration File Location:**
- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/claude/claude_desktop_config.json`

**Configuration:**

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

### ChatGPT Desktop

**Configuration File Location:**
- **macOS**: `~/Library/Application Support/ChatGPT/config.json`
- **Windows**: `%APPDATA%\ChatGPT\config.json`

Create the file if it doesn't exist:

```bash
# macOS
mkdir -p ~/Library/Application\ Support/ChatGPT/
touch ~/Library/Application\ Support/ChatGPT/config.json

# Windows (PowerShell)
New-Item -Path "$env:APPDATA\ChatGPT" -ItemType Directory -Force
New-Item -Path "$env:APPDATA\ChatGPT\config.json" -ItemType File -Force
```

**Configuration:**

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

### Other MCP Clients

For any MCP-compatible client, use the stdio transport configuration:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

If using a locally installed package:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "pierre-mcp-client",
      "args": [
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

## How It Works

The Pierre MCP Client SDK provides automatic OAuth 2.0 authentication:

1. **Automatic Client Registration**: The SDK registers as an OAuth 2.0 client with Pierre using RFC 7591 dynamic client registration
2. **Browser-Based Authentication**: Opens your default browser for secure authentication
3. **Token Management**: Automatically handles token refresh and storage
4. **Stdio Transport**: Provides stdio transport for seamless MCP client integration

No manual JWT token management required!

## Testing the Integration

### 1. Restart Your MCP Client

After updating the configuration, restart your MCP application to load the Pierre MCP Server connection.

### 2. Verify Connection

Ask your AI assistant:
> "What fitness-related tools do you have access to?"

You should see a list of available tools including:
- `get_activities` - Retrieve fitness activities
- `get_athlete` - Get athlete profile
- `get_stats` - Get athlete statistics
- `analyze_activity` - Analyze specific activities
- `set_goal` - Set fitness goals
- `track_progress` - Track goal progress
- And 19 more tools...

### 3. Test Basic Functionality

Try these commands:

**Check connection status:**
> "Check my fitness provider connection status"

**Get recent activities:**
> "Show me my recent workout activities"

**Analyze performance:**
> "Analyze my most recent running activity"

## Connecting Fitness Providers

After configuring your MCP client, you need to connect fitness data providers like Strava or Garmin.

### Connect to Strava

The SDK will prompt you to connect Strava when you first use fitness tools. Alternatively, you can connect manually:

```bash
# Get your JWT token (if needed for direct API access)
JWT_TOKEN=$(curl -s -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "your@email.com", "password": "your_password"}' | jq -r '.jwt_token')

# Get Strava authorization URL
curl "http://localhost:8081/oauth/strava/connect" \
  -H "Authorization: Bearer $JWT_TOKEN"

# Open the URL in your browser to authorize
```

### Connect to Garmin

```bash
# Get Garmin authorization URL
curl "http://localhost:8081/oauth/garmin/connect" \
  -H "Authorization: Bearer $JWT_TOKEN"

# Complete authorization in browser
```

### Verify Connection

```bash
curl "http://localhost:8081/oauth/status" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

## Troubleshooting

### MCP Client Not Connecting

**1. Verify Pierre MCP Server is running:**

```bash
curl http://localhost:8081/health
```

Expected response: `{"status":"healthy","version":"0.1.0"}`

**2. Check configuration file syntax:**

```bash
# macOS/Linux
cat ~/Library/Application\ Support/Claude/claude_desktop_config.json | jq .

# Windows (PowerShell)
Get-Content "$env:APPDATA\Claude\claude_desktop_config.json" | ConvertFrom-Json
```

**3. Check application logs:**

- **Claude Desktop**:
  - macOS: `~/Library/Logs/Claude/`
  - Windows: `%LOCALAPPDATA%\Claude\logs\`

- **ChatGPT Desktop**:
  - Check application console/developer tools

### SDK Installation Issues

**Node.js version too old:**

```bash
node --version  # Should be 18.0.0 or higher
```

Update Node.js if needed: https://nodejs.org/

**npm permission errors:**

```bash
# Use npx instead (no installation required)
# Or fix npm permissions:
npm config set prefix ~/.npm-global
export PATH=~/.npm-global/bin:$PATH
```

### OAuth Flow Not Starting

**1. Verify server OAuth configuration:**

```bash
# Check environment variables
echo $STRAVA_CLIENT_ID
echo $STRAVA_CLIENT_SECRET
echo $STRAVA_REDIRECT_URI
```

**2. Test OAuth endpoint directly:**

```bash
curl "http://localhost:8081/.well-known/oauth-authorization-server"
```

Should return server metadata including authorization and token endpoints.

### No Fitness Data Available

**1. Verify OAuth connection:**

```bash
curl "http://localhost:8081/oauth/status" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

**2. Reconnect provider:**

```bash
# Disconnect
curl -X POST "http://localhost:8081/oauth/disconnect/strava" \
  -H "Authorization: Bearer $JWT_TOKEN"

# Get new authorization URL
curl "http://localhost:8081/oauth/strava/connect" \
  -H "Authorization: Bearer $JWT_TOKEN"
```

## Advanced Configuration

### Custom Server Port

If Pierre MCP Server is running on a non-standard port:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "http://localhost:9081"
      ]
    }
  }
}
```

### Custom Server URL

For remote servers:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "https://pierre.example.com"
      ]
    }
  }
}
```

### Using Installed Package

If you installed globally with `npm install -g`:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "pierre-mcp-client",
      "args": [
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

### Using Local Build

If building from source:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": [
        "/absolute/path/to/pierre_mcp_server/sdk/dist/cli.js",
        "--server",
        "http://localhost:8081"
      ]
    }
  }
}
```

## Security Considerations

### Token Storage

The SDK stores OAuth tokens securely in your user directory:
- **macOS/Linux**: `~/.pierre-mcp-tokens.json`
- **Windows**: `%USERPROFILE%\.pierre-mcp-tokens.json`

### HTTPS in Production

For production deployments, always use HTTPS:

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "npx",
      "args": [
        "-y",
        "pierre-mcp-client@next",
        "--server",
        "https://pierre.example.com"
      ]
    }
  }
}
```

### Network Security

- Use firewall rules to restrict access to Pierre MCP Server
- Enable rate limiting (see server configuration)
- Regular security updates for both SDK and server

## Getting Help

### Documentation

- [Main README](../../README.md) - Server setup and overview
- [Developer Guide](../developer-guide/) - Complete documentation
- [API Reference](../developer-guide/14-api-reference.md) - REST API documentation

### Support Channels

1. **GitHub Issues**: https://github.com/Async-IO/pierre_mcp_server/issues
2. **Discussions**: https://github.com/Async-IO/pierre_mcp_server/discussions

When reporting issues, include:
- Operating system and version
- MCP client name and version
- Node.js version (`node --version`)
- Pierre MCP Server version
- Configuration file (sanitize tokens)
- Error messages and logs

## SDK Command-Line Options

The Pierre MCP Client supports several command-line options:

```bash
pierre-mcp-client --help
```

**Options:**
- `--server <url>` - Pierre MCP Server URL (required)
- `--version` - Show SDK version
- `--help` - Show help message

## What's Next?

Once you have Pierre MCP Client connected:

1. **Connect fitness providers** (Strava, Garmin)
2. **Explore available tools** - Ask your AI assistant what it can do
3. **Set fitness goals** - Use goal tracking and recommendations
4. **Analyze activities** - Get AI-powered insights on your workouts
5. **Track progress** - Monitor your fitness journey over time

## Version Information

- **Package**: `pierre-mcp-client`
- **Current Version**: 0.1.0
- **NPM Tag**: `next` (pre-release)
- **Minimum Node.js**: 18.0.0
- **License**: MIT

Once Pierre reaches v1.0.0, the package will be available on the `latest` tag:

```bash
npm install -g pierre-mcp-client  # Future stable release
```
