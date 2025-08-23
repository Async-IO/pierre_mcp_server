# JavaScript MCP Clients

JavaScript clients and bridges for connecting to the Pierre MCP Server from MCP-compatible applications.

## Files

### `http-mcp-client.js`
HTTP MCP client for Claude Desktop integration. Provides direct HTTP transport to Pierre MCP Server.

**Implementation:**
- HTTP communication with Pierre MCP Server port 8080
- Environment variable API key authentication
- JSON-RPC 2.0 error handling
- Readline interface for MCP protocol

**Usage:**
```bash
export PIERRE_API_KEY="your_api_key_here"
node http-mcp-client.js
```

### `pierre-bridge.js`
Basic MCP bridge implementation for HTTP transport with built-in initialize handling.

### `pierre-mcp-bridge.js` 
Advanced MCP bridge with environment variable configuration and enhanced error handling.

### `pierre-mcp-client-template.js`
Template for creating custom MCP clients with HTTPS support and configurable endpoints.

### `claude_desktop_http_config.json`
Claude Desktop configuration example for HTTP-based MCP server connection.

**Local Development Configuration:**
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node",
      "args": ["/path/to/http-mcp-client.js"],
      "env": {
        "PIERRE_API_KEY": "your_api_key_here",
        "PIERRE_MCP_SERVER_URL": "http://localhost:8080"
      }
    }
  }
}
```

**Production/Cloud Configuration:**
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "command": "node", 
      "args": ["/path/to/http-mcp-client.js"],
      "env": {
        "PIERRE_API_KEY": "your_api_key_here",
        "PIERRE_MCP_SERVER_URL": "https://your-domain.com"
      }
    }
  }
}
```

## Setup Instructions

1. **Install Node.js** (version 16 or higher)

2. **Get API key** from Pierre MCP Server admin interface

3. **Configure environment variables**:
   ```bash
   export PIERRE_API_KEY="your_api_key_here"
   export PIERRE_MCP_SERVER_URL="http://localhost:8080"  # Optional
   ```

4. **Configure Claude Desktop**:
   - Update MCP settings with client configuration
   - Restart Claude Desktop

## Security Notes

- Store API keys in environment variables only
- Never commit API keys to version control
- Use HTTPS in production environments
- Rotate API keys regularly

## Development Setup

1. Start Pierre MCP Server locally
2. Create API key through admin interface
3. Set environment variables
4. Test with Claude Desktop

## Deployment Configurations

### Local Development
```bash
export PIERRE_API_KEY="your_api_key_here"
export PIERRE_MCP_SERVER_URL="http://localhost:8080"
```

### Production/Cloud Deployment
```bash
export PIERRE_API_KEY="your_api_key_here"
export PIERRE_MCP_SERVER_URL="https://your-domain.com"
```

**Requirements for cloud deployment:**
- HTTPS endpoint (not HTTP)
- Publicly accessible domain name
- Valid SSL certificate
- Firewall configured to allow HTTPS traffic

## Troubleshooting

### Local Development Issues
1. **Connection refused on localhost:8080**: Verify Pierre MCP Server is running locally
2. **Port binding errors**: Check if port 8080 is already in use

### Production/Cloud Issues  
1. **Connection refused on domain**: 
   - Verify HTTPS endpoint is accessible: `curl https://your-domain.com/health`
   - Check SSL certificate validity
   - Confirm firewall allows HTTPS (port 443) traffic
2. **Certificate errors**: Ensure valid SSL/TLS certificate is installed
3. **DNS issues**: Verify domain name resolves correctly

### Common Issues
1. **Authentication failed**: Check API key validity and format
2. **Parse errors**: Verify JSON-RPC 2.0 request format  
3. **Claude Desktop connection issues**: Check MCP server logs in Claude Desktop diagnostics