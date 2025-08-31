# Python Examples and SDK

This guide covers Python integration examples for Pierre MCP Server, including both direct API usage and MCP protocol examples.

## Overview

The Python examples are for:
- Development and testing custom integrations
- Creating client libraries in other languages

> **Note**: For production AI assistant integration, use the native MCP protocol with Claude Desktop configuration shown in the main README.

## Available Examples

### Direct API Usage

Located in `examples/python/`:

#### Basic Authentication and API Calls
- `basic_usage.py` - Simple API authentication and data fetching
- `advanced_analysis.py` - Complex data analysis workflows
- `common/auth_utils.py` - Reusable authentication utilities
- `common/data_utils.py` - Data processing helpers

#### A2A Protocol Examples
- `a2a/api_client.py` - A2A client implementation
- `a2a/enterprise_demo.py` - Enterprise integration patterns

#### MCP Protocol Examples
- `simple_mcp_example.py` - Direct MCP JSON-RPC calls
- `mcp_stdio_example.py` - STDIO MCP client implementation
- `multitenant_mcp_example.py` - Multi-tenant MCP usage

### Provisioning and Testing
- `provisioning/run_demo.py` - Complete demo setup
- `provisioning/mock_strava_provider.py` - Mock data for testing
- `provisioning/provisioning_example.py` - Automated account setup

## Quick Example: Direct API Usage

```python
import requests
import json

# 1. Setup admin and user accounts
admin_response = requests.post('http://localhost:8081/admin/setup', 
    headers={'Content-Type': 'application/json'},
    data=json.dumps({
        'email': 'admin@example.com',
        'password': 'SecurePass123!',
        'display_name': 'Admin'
    }))
admin_token = admin_response.json()['admin_token']

# 2. Register and approve user
user_response = requests.post('http://localhost:8081/api/auth/register',
    headers={'Content-Type': 'application/json'},
    data=json.dumps({
        'email': 'user@example.com', 
        'password': 'pass123',
        'display_name': 'User'
    }))
user_id = user_response.json()['user_id']

# Approve user with tenant
requests.post(f'http://localhost:8081/admin/approve-user/{user_id}',
    headers={
        'Authorization': f'Bearer {admin_token}',
        'Content-Type': 'application/json'
    },
    data=json.dumps({
        'reason': 'Approved',
        'create_default_tenant': True,
        'tenant_name': 'User Org',
        'tenant_slug': 'user-org'
    }))

# 3. Login and get JWT token
login_response = requests.post('http://localhost:8081/api/auth/login',
    headers={'Content-Type': 'application/json'},
    data=json.dumps({
        'email': 'user@example.com',
        'password': 'pass123'
    }))
jwt_token = login_response.json()['jwt_token']

# 4. Make MCP call
mcp_request = {
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
}

mcp_response = requests.post('http://localhost:8080/mcp',
    headers={
        'Authorization': f'Bearer {jwt_token}',
        'Content-Type': 'application/json'
    },
    data=json.dumps(mcp_request))

tools = mcp_response.json()
print("Available MCP tools:", [tool['name'] for tool in tools['result']['tools']])
```

## Running the Examples

### Prerequisites
```bash
# Install Python dependencies
pip install requests python-dotenv

# Ensure server is running
cargo run --bin pierre-mcp-server
```

### Run Basic Examples
```bash
cd examples/python
python basic_usage.py
python advanced_analysis.py
```

### Run A2A Examples
```bash
cd examples/python/a2a
python enterprise_demo.py
```

### Run MCP Examples
```bash
cd examples/python
python simple_mcp_example.py
```

## A2A Protocol Python Client

Example A2A client implementation:

```python
import requests
import json

class PierreA2AClient:
    def __init__(self, base_url="http://localhost:8081"):
        self.base_url = base_url
        self.access_token = None
    
    def authenticate(self, client_id, client_secret):
        """Authenticate with A2A credentials"""
        response = requests.post(f"{self.base_url}/a2a/auth", 
            headers={'Content-Type': 'application/json'},
            data=json.dumps({
                'client_id': client_id,
                'client_secret': client_secret
            }))
        
        if response.status_code == 200:
            self.access_token = response.json()['access_token']
            return True
        return False
    
    def execute_tool(self, tool_name, parameters=None):
        """Execute an A2A tool call"""
        if not self.access_token:
            raise Exception("Not authenticated. Call authenticate() first.")
        
        response = requests.post(f"{self.base_url}/a2a/execute",
            headers={
                'Authorization': f'Bearer {self.access_token}',
                'Content-Type': 'application/json'
            },
            data=json.dumps({
                'tool': tool_name,
                'parameters': parameters or {}
            }))
        
        return response.json()

# Usage
client = PierreA2AClient()
client.authenticate('your_client_id', 'your_client_secret')
activities = client.execute_tool('get_strava_activities', {'limit': 10})
```

## MCP JSON-RPC Python Client

Example MCP protocol client:

```python
import requests
import json

class PierreMCPClient:
    def __init__(self, server_url="http://localhost:8080/mcp", jwt_token=None):
        self.server_url = server_url
        self.jwt_token = jwt_token
        self.request_id = 1
    
    def call_method(self, method, params=None):
        """Make MCP JSON-RPC call"""
        request = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": self.request_id
        }
        self.request_id += 1
        
        response = requests.post(self.server_url,
            headers={
                'Authorization': f'Bearer {self.jwt_token}',
                'Content-Type': 'application/json'
            },
            data=json.dumps(request))
        
        return response.json()
    
    def list_tools(self):
        """List available MCP tools"""
        return self.call_method("tools/list")
    
    def call_tool(self, tool_name, arguments=None):
        """Execute an MCP tool"""
        return self.call_method("tools/call", {
            "name": tool_name,
            "arguments": arguments or {}
        })

# Usage
client = PierreMCPClient(jwt_token='your_jwt_token')
tools = client.list_tools()
activities = client.call_tool('get_strava_activities', {'limit': 10})
```

## Development Patterns

### Error Handling
```python
try:
    response = client.call_tool('get_strava_activities')
    if 'error' in response:
        print(f"MCP Error: {response['error']['message']}")
    else:
        activities = response['result']
except requests.exceptions.RequestException as e:
    print(f"Network error: {e}")
```

### Multi-tenant Usage
```python
# Each user should have their own JWT token
user_tokens = {
    'user1@example.com': 'jwt_token_1',
    'user2@example.com': 'jwt_token_2'
}

for user, token in user_tokens.items():
    client = PierreMCPClient(jwt_token=token)
    activities = client.call_tool('get_strava_activities')
    print(f"User {user} has {len(activities['result'])} activities")
```

## Next Steps

- Review the complete examples in `examples/python/`
- See [A2A Protocol Guide](05-a2a-protocol.md) for detailed A2A documentation
- Check [MCP Protocol Guide](04-mcp-protocol.md) for complete MCP implementation
- Refer to [API Reference](14-api-reference.md) for all available endpoints and tools