# JavaScript MCP Clients

This directory contains JavaScript clients and bridges for connecting to the Pierre MCP Server from various environments.

## Files

### `http-mcp-client.js`
Production-ready HTTP MCP client for Claude Desktop integration. Provides direct HTTP transport without local bridges.

**Features:**
- Direct HTTP communication with Pierre MCP Server
- Built-in API key authentication
- Production-ready error handling
- Readline interface for Claude Desktop

**Usage:**
```bash
node http-mcp-client.js
```

### `pierre-bridge.js`
Basic MCP bridge implementation for HTTP transport.

### `pierre-mcp-bridge.js` 
Advanced MCP bridge with enhanced features and error handling.

### `pierre-mcp-client-template.js`
Template for creating custom MCP clients with Pierre MCP Server integration.

### `claude_desktop_http_config.json`
Sample Claude Desktop configuration for HTTP-based MCP server connection.

**Example configuration:**
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["/path/to/http-mcp-client.js"]
    }
  }
}
```

## Setup Instructions

1. **Install Node.js** (version 16 or higher)

2. **Get your API key** from the Pierre MCP Server admin interface

3. **Update configuration** in the client files:
   - Replace `API_KEY` with your actual API key
   - Update `SERVER_HOST` and `SERVER_PORT` if needed

4. **Configure Claude Desktop**:
   - Add the client configuration to your Claude Desktop MCP settings
   - Restart Claude Desktop to apply changes

## Security Notes

- **Never commit API keys to version control**
- Store API keys securely in environment variables or config files
- Use HTTPS in production environments
- Regularly rotate API keys

## Development

These clients are designed to work with the Pierre MCP Server's HTTP endpoints. For development:

1. Start the Pierre MCP Server locally
2. Create a test API key through the admin interface
3. Update the client configuration
4. Test the connection with Claude Desktop

## Support

For issues with these JavaScript clients:

1. Check the Pierre MCP Server logs
2. Verify API key is valid and not expired
3. Ensure server is running and accessible
4. Check Claude Desktop MCP server logs