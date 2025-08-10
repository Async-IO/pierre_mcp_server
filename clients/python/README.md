# Pierre MCP Client - Python

Python client library for connecting to Pierre MCP Server for fitness data analysis.

## Installation

```bash
pip install pierre-mcp-client
```

## Quick Start

```python
from pierre_mcp import PierreMCPClient
import asyncio

async def main():
    async with PierreMCPClient(
        server_url="http://localhost:8081",
        tenant_id="your-tenant-id", 
        jwt_token="your-jwt-token"
    ) as client:
        # List available tools
        tools = await client.list_tools()
        print(f"Available tools: {[tool['name'] for tool in tools]}")
        
        # Get recent activities
        activities = await client.call_tool(
            "get_activities", 
            {"provider": "strava", "limit": 5}
        )
        print(f"Recent activities: {activities}")

asyncio.run(main())
```

## Requirements

- Python 3.8+
- aiohttp
- A running Pierre MCP Server instance
- Valid tenant ID and JWT token

## API Reference

### PierreMCPClient

Main client class for connecting to Pierre MCP Server.

#### Methods

- `connect()`: Establish connection to server
- `close()`: Close connection
- `list_tools()`: Get available MCP tools
- `call_tool(name, params)`: Execute a specific tool
- `get_oauth_status(provider)`: Check OAuth connection status
- `get_authorization_url(provider)`: Get OAuth authorization URL

### Exceptions

- `PierreMCPError`: Base exception
- `AuthenticationError`: JWT token issues
- `TenantError`: Tenant access problems

## Development

```bash
# Install development dependencies
pip install -e .[dev]

# Run tests
pytest

# Format code
black pierre_mcp/

# Type checking
mypy pierre_mcp/
```