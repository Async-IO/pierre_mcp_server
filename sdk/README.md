# Pierre-Claude Bridge

MCP-compliant bridge connecting Claude Desktop to Pierre Fitness MCP Server via Streamable HTTP + OAuth 2.0.

## Features

- ✅ 100% MCP specification compliant
- 🚀 Streamable HTTP transport support
- 🔐 OAuth 2.0 authentication with JWT tokens
- 📡 Real-time notifications and progress updates
- 🛠️ Bridge all MCP operations (tools, resources, prompts, completion)
- 🔧 Easy NPX installation for end users

## Installation

### Global Installation (Recommended)

```bash
npm install -g pierre-claude-bridge
```

### Using NPX (No installation required)

```bash
npx pierre-claude-bridge --server http://localhost:8081 --token YOUR_JWT_TOKEN
```

## Usage

### Command Line

```bash
pierre-claude-bridge --server http://localhost:8081 --token YOUR_JWT_TOKEN --verbose
```

### Options

- `-s, --server <url>` - Pierre MCP server URL (default: http://localhost:8081)
- `-t, --token <jwt>` - JWT authentication token
- `--oauth-client-id <id>` - OAuth 2.0 client ID (future use)
- `--oauth-client-secret <secret>` - OAuth 2.0 client secret (future use)
- `-v, --verbose` - Enable verbose logging

### Claude Desktop Configuration

Add to your Claude Desktop config file (`~/.config/claude/claude_desktop_config.json` on Linux/macOS or `%APPDATA%\\Claude\\claude_desktop_config.json` on Windows):

```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "pierre-claude-bridge",
      "args": [
        "--server", "http://localhost:8081",
        "--token", "YOUR_JWT_TOKEN_HERE"
      ]
    }
  }
}
```

### Programmatic Usage

```typescript
import { PierreClaudeBridge } from 'pierre-claude-bridge';

const bridge = new PierreClaudeBridge({
  pierreServerUrl: 'http://localhost:8081',
  jwtToken: 'YOUR_JWT_TOKEN',
  verbose: true
});

await bridge.start();
```

## Architecture

```
┌─────────────────┐    stdio     ┌─────────────────┐    HTTP/SSE    ┌─────────────────┐
│   Claude Desktop │ ◄─────────► │ Pierre Bridge   │ ◄────────────► │ Pierre MCP      │
│                 │              │                 │                │ Server          │
└─────────────────┘              └─────────────────┘                └─────────────────┘
```

The bridge acts as a protocol translator:
- **Inbound**: Receives MCP requests from Claude Desktop via stdio
- **Outbound**: Forwards requests to Pierre MCP Server via Streamable HTTP
- **Bidirectional**: Forwards notifications and progress updates in real-time

## Requirements

- Node.js 18+
- Active Pierre MCP Server instance
- Valid JWT authentication token

## Development

### Build

```bash
npm run build
```

### Development Mode

```bash
npm run dev -- --server http://localhost:8081 --token YOUR_JWT_TOKEN --verbose
```

## Troubleshooting

### Connection Issues

1. **Server unreachable**: Verify Pierre MCP Server is running on the specified URL
2. **Authentication failed**: Check JWT token is valid and not expired
3. **Protocol mismatch**: Ensure Pierre MCP Server supports Streamable HTTP transport

### Verbose Logging

Enable verbose logging to see detailed bridge operations:

```bash
pierre-claude-bridge --server http://localhost:8081 --token YOUR_JWT_TOKEN --verbose
```

## License

MIT