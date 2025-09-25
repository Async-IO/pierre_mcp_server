# Pierre MCP Bridge

MCP-compliant bridge connecting MCP clients to Pierre Fitness MCP Server with OAuth 2.0 authentication.

## Features

- MCP specification compliant (stdio transport)
- Streamable HTTP transport to Pierre MCP Server
- OAuth 2.0 authentication with automatic browser flow
- Dynamic client registration (RFC 7591)
- JWT token management and refresh
- Real-time connection retry logic
- Comprehensive error handling and logging

## Installation

The bridge is included with Pierre MCP Server:

```bash
cd pierre_mcp_server/sdk
npm install
npm run build
```

## Usage

### Command Line

For testing the bridge directly:

```bash
node dist/cli.js --server http://localhost:8081 --verbose
```

### Options

- `-s, --server <url>` - Pierre MCP server URL (default: http://localhost:8081)
- `-v, --verbose` - Enable verbose logging

### MCP Client Configuration

For Claude Desktop, add to your configuration file:

- **macOS**: `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows**: `%APPDATA%\Claude\claude_desktop_config.json`
- **Linux**: `~/.config/claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": [
        "/absolute/path/to/pierre_mcp_server/sdk/dist/cli.js",
        "--server", "http://localhost:8081",
        "--verbose"
      ],
      "env": {}
    }
  }
}
```

## Authentication Flow

When an MCP client starts, the bridge automatically handles OAuth 2.0 authentication:

1. **Client Registration**: Registers a new OAuth client with Pierre MCP Server using dynamic registration (RFC 7591)
2. **Browser Authorization**: Opens the user's browser to Pierre's authorization page
3. **User Authentication**: User logs in and authorizes the application
4. **Token Exchange**: Authorization code is exchanged for JWT access tokens
5. **Authenticated Requests**: All subsequent MCP requests include Bearer tokens

No manual token management is required.

## Architecture

```
┌─────────────────┐    stdio     ┌─────────────────┐    HTTP+OAuth   ┌─────────────────┐
│   MCP Client    │ ◄─────────► │ Pierre Bridge   │ ◄─────────────► │ Pierre MCP      │
│                 │              │                 │                 │ Server          │
└─────────────────┘              └─────────────────┘                 └─────────────────┘
```

The bridge acts as a protocol translator and OAuth client:
- **Inbound**: Receives MCP requests from clients via stdio
- **OAuth Handling**: Manages OAuth 2.0 flow with browser-based authentication
- **Outbound**: Forwards authenticated requests to Pierre MCP Server via HTTP
- **Token Management**: Handles JWT token storage, refresh, and injection

## Requirements

- Node.js 18+
- Active Pierre MCP Server instance
- User account on Pierre MCP Server

## Development

### Build

```bash
npm run build
```

### Development Mode

```bash
npm run dev -- --server http://localhost:8081 --verbose
```

## Troubleshooting

### Connection Issues

1. **Server unreachable**: Verify Pierre MCP Server is running on the specified URL
2. **Authentication failed**: Check that user account exists on Pierre MCP Server
3. **OAuth flow fails**: Ensure browser can access Pierre MCP Server authorization endpoints
4. **Port conflicts**: OAuth callback server uses dynamic port allocation to avoid conflicts

### Verbose Logging

Enable verbose logging to see detailed bridge operations including OAuth flow:

```bash
node dist/cli.js --server http://localhost:8081 --verbose
```

This will show:
- OAuth client registration
- Browser authorization flow
- Token exchange process
- MCP request/response details
- Connection retry attempts

## License

MIT