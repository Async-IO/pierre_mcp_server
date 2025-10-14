# Pierre MCP Client

MCP client SDK for connecting to Pierre Fitness MCP Server. Works with Claude Desktop, ChatGPT, and any MCP-compatible application.

## Installation

```bash
npm install pierre-mcp-client@next
```

## Usage

### With npx (No Installation)

```bash
npx -y pierre-mcp-client@next --server http://localhost:8081
```

### MCP Client Configuration

Add to your MCP client configuration file:

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

**Configuration file locations:**
- **Claude Desktop**: `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS)
- **ChatGPT**: `~/Library/Application Support/ChatGPT/config.json` (macOS)
- See [full installation guide](https://github.com/Async-IO/pierre_mcp_server/blob/main/docs/installation-guides/install-mcp-client.md) for all platforms

## What It Does

The Pierre MCP Client automatically:
- Registers with Pierre MCP Server using OAuth 2.0
- Opens your browser for authentication
- Manages tokens and token refresh
- Provides stdio transport for MCP clients

No manual token management required!

## Available Tools

Once connected, your AI assistant can access 25 fitness tools including:
- Activity retrieval and analysis
- Goal setting and progress tracking
- Performance trend analysis
- Training recommendations
- And more...

Ask your AI assistant: *"What fitness tools do you have access to?"*

## Requirements

- **Node.js**: 18.0.0 or higher
- **Pierre MCP Server**: Running on port 8081 (or custom port)

## Configuration Options

```bash
pierre-mcp-client --server <url> [--verbose]
```

**Options:**
- `--server` - Pierre MCP Server URL (required)
- `--verbose` - Enable debug logging (optional)

## Example

```bash
# Start Pierre MCP Server
cargo run --bin pierre-mcp-server

# In another terminal, test the client
npx -y pierre-mcp-client@next --server http://localhost:8081 --verbose
```

## Troubleshooting

### Authentication Issues

If the browser doesn't open for authentication, check:
```bash
# Verify server is running
curl http://localhost:8081/health
```

### Token Cache

Tokens are stored in `~/.pierre-mcp-client/`. To force re-authentication:
```bash
rm -rf ~/.pierre-mcp-client/
```

## Documentation

- [Installation Guide](https://github.com/Async-IO/pierre_mcp_server/blob/main/docs/installation-guides/install-mcp-client.md)
- [Server Documentation](https://github.com/Async-IO/pierre_mcp_server)
- [API Reference](https://github.com/Async-IO/pierre_mcp_server/blob/main/docs/developer-guide/14-api-reference.md)

## Support

- **GitHub Issues**: https://github.com/Async-IO/pierre_mcp_server/issues
- **Discussions**: https://github.com/Async-IO/pierre_mcp_server/discussions

## License

MIT
