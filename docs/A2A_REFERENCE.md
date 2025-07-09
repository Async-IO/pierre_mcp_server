# A2A Protocol Reference

Complete guide to using the Pierre Fitness API via Agent-to-Agent (A2A) protocol for enterprise integrations and scalable fitness data access.

## Table of Contents

1. [Overview](#overview)
2. [Authentication Flow](#authentication-flow)
3. [Available Tools](#available-tools)
4. [Response Formats](#response-formats)
5. [Error Handling](#error-handling)
6. [Python Client Example](#python-client-example)
7. [Production Considerations](#production-considerations)

## Overview

The A2A protocol provides a JSON-RPC 2.0 REST API for enterprise integrations, offering:

- **High Throughput**: Stateless REST API design
- **Scalable**: Multi-tenant architecture with API key management
- **Enterprise Ready**: Rate limiting, monitoring, and comprehensive error handling
- **B2B Integration**: Designed for system-to-system communication

### A2A vs MCP Protocol

| Feature | A2A Protocol | MCP Protocol |
|---------|--------------|--------------|
| **Use Case** | Enterprise integration, B2B | AI assistants, real-time analysis |
| **Transport** | REST API (HTTP/HTTPS) | stdio, WebSocket |
| **Authentication** | Client credentials + Session tokens | JWT tokens |
| **State** | Stateless | Stateful |
| **Throughput** | High | Medium |
| **Target Users** | Enterprise developers | AI assistant developers |

## Authentication Flow

The A2A protocol uses a two-step authentication process:

### Step 1: Client Registration

Register your application to get client credentials:

```bash
curl -X POST http://localhost:8081/a2a/clients \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My Fitness App",
    "description": "AI-powered fitness analytics",
    "capabilities": ["fitness-data-analysis", "goal-management"],
    "contact_email": "developer@myapp.com"
  }'
```

**Response:**
```json
{
  "client_id": "a2a_client_53a7b091-ac77-4cbe-986c-8134d2c971c1",
  "client_secret": "a2a_secret_496d9c7b-9993-4800-9043-2b41d3de21d4",
  "api_key": "a2a_a2c54154-e2b9-42eb-bddf-8ffe9074412c",
  "public_key": "nKonNbpJD2dyzYdPg6IGzvljD5nUqHBql8LCyafhzl4=",
  "private_key": "e2sSaRMQZ7l2lTo6Tiyb9NdVZnW60YFLEwUNo/AmXvg=",
  "key_type": "ed25519"
}
```

### Step 2: Client Authentication

Authenticate with your client credentials to get a session token:

```bash
curl -X POST http://localhost:8081/a2a/auth \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "a2a_client_53a7b091-ac77-4cbe-986c-8134d2c971c1",
    "client_secret": "a2a_secret_496d9c7b-9993-4800-9043-2b41d3de21d4",
    "scopes": ["read", "write"]
  }'
```

**Response:**
```json
{
  "status": "authenticated",
  "session_token": "sess_58a0b76a-0b19-467c-b7fc-628d7f80d435",
  "expires_in": 86400,
  "token_type": "Bearer",
  "scope": "read write"
}
```

### Step 3: Tool Execution

Execute tools using the session token. **Important**: Tool execution requires a user's JWT token, not the A2A session token.

```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer USER_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "limit": 10,
        "provider": "strava"
      }
    },
    "id": 1
  }'
```

## Available Tools

### Core Data Access

#### `get_activities`
Retrieve user's fitness activities with pagination support.

**Parameters:**
- `provider` (string, optional): Fitness provider ("strava", "fitbit"). Default: "strava"
- `limit` (integer, optional): Maximum activities to return. Default: 100
- `offset` (integer, optional): Number of activities to skip. Default: 0

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "tools.execute",
  "params": {
    "tool_name": "get_activities",
    "parameters": {
      "provider": "strava",
      "limit": 5
    }
  },
  "id": 1
}
```

#### `get_athlete`
Get user's athlete profile information.

**Parameters:**
- `provider` (string, optional): Fitness provider. Default: "strava"

#### `analyze_activity`
Analyze a specific activity with AI insights.

**Parameters:**
- `activity_id` (string, required): Activity ID to analyze
- `provider` (string, optional): Fitness provider. Default: "strava"

### OAuth Integration

#### `connect_strava`
Initiate Strava OAuth connection for the user.

**Parameters:** None

**Response:**
```json
{
  "authorization_url": "https://www.strava.com/oauth/authorize?...",
  "state": "user_id:session_id",
  "provider": "strava"
}
```

#### `connect_fitbit`
Initiate Fitbit OAuth connection for the user.

**Parameters:** None

### Goal Management

#### `create_goal`
Create a new fitness goal for the user.

**Parameters:**
- `goal_type` (string, required): Type of goal ("distance", "time", "frequency")
- `target_value` (number, required): Target value for the goal
- `target_unit` (string, required): Unit ("km", "hours", "activities")
- `deadline` (string, optional): Goal deadline in ISO format

#### `get_goals`
Retrieve user's fitness goals.

**Parameters:**
- `status` (string, optional): Filter by status ("active", "completed", "all")

#### `update_goal`
Update an existing fitness goal.

**Parameters:**
- `goal_id` (string, required): Goal ID to update
- `target_value` (number, optional): New target value
- `deadline` (string, optional): New deadline

### Analytics Tools

#### `calculate_fitness_score`
Calculate comprehensive fitness score with AI analysis.

**Parameters:**
- `provider` (string, optional): Fitness provider. Default: "strava"

#### `generate_recommendations`
Generate personalized training recommendations.

**Parameters:**
- `provider` (string, optional): Fitness provider. Default: "strava"

#### `analyze_training_load`
Analyze training load and recovery metrics.

**Parameters:**
- `provider` (string, optional): Fitness provider. Default: "strava"

### System Tools

#### `client.info`
Get A2A client information and capabilities.

**Parameters:** None

#### `session.heartbeat`
Keep A2A session alive.

**Parameters:** None

#### `capabilities.list`
List available A2A capabilities.

**Parameters:** None

## Response Formats

### Successful Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "activities": [
      {
        "id": "15058226769",
        "name": "Morning Run",
        "sport_type": "Run",
        "distance_meters": 5088.4,
        "duration_seconds": 1656,
        "average_heart_rate": 152,
        "max_heart_rate": 168,
        "elevation_gain": 62.0,
        "start_date": "2025-07-09T14:52:48+00:00",
        "provider": "strava",
        "is_real_data": true
      }
    ],
    "total_count": 1,
    "provider": "strava"
  }
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Invalid or expired authentication token",
    "data": {
      "error_type": "authentication_error",
      "details": "JWT token validation failed"
    }
  }
}
```

## Error Handling

### Common Error Codes

| Code | Message | Description |
|------|---------|-------------|
| -32001 | Authentication Failed | Invalid client credentials or expired session |
| -32002 | Authorization Required | Missing or invalid JWT token for tool execution |
| -32003 | Rate Limit Exceeded | Too many requests within time window |
| -32000 | Tool Execution Failed | Internal error during tool execution |
| -32601 | Method Not Found | Invalid tool name or method |
| -32602 | Invalid Parameters | Missing or invalid tool parameters |

### Authentication Errors

#### Client Registration Failed
```json
{
  "error": {
    "code": -32001,
    "message": "Client registration failed",
    "data": {
      "error_type": "registration_error",
      "details": "Invalid email format"
    }
  }
}
```

#### Invalid Client Credentials
```json
{
  "error": {
    "code": -32001,
    "message": "A2A authentication failed",
    "data": {
      "error_type": "authentication_error",
      "client_id": "a2a_client_12345",
      "details": "Invalid client_secret"
    }
  }
}
```

#### Missing User JWT Token
```json
{
  "error": {
    "code": -32002,
    "message": "Missing Authorization header",
    "data": {
      "error_type": "authorization_error",
      "details": "Tool execution requires user JWT token"
    }
  }
}
```

### OAuth Connection Errors

#### Strava Not Connected
```json
{
  "result": {
    "activities": [
      {
        "error": "No Strava token found for user - please connect your Strava account first",
        "is_real_data": false,
        "note": "Connect your Strava account via the OAuth flow to get real data"
      }
    ]
  }
}
```

## Python Client Example

Here's a complete Python client implementation:

```python
import requests
import json
import time
from typing import Dict, List, Optional

class PierreA2AClient:
    def __init__(self, base_url: str = "http://localhost:8081"):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'User-Agent': 'Pierre-A2A-Client/1.0'
        })
        self.client_id = None
        self.client_secret = None
        self.session_token = None
        self.user_jwt_token = None
    
    def register_client(self, name: str, description: str, 
                       capabilities: List[str], contact_email: str) -> Dict:
        """Register A2A client and get credentials"""
        response = self.session.post(f'{self.base_url}/a2a/clients', json={
            'name': name,
            'description': description,
            'capabilities': capabilities,
            'contact_email': contact_email
        })
        
        if response.status_code == 200:
            credentials = response.json()
            self.client_id = credentials['client_id']
            self.client_secret = credentials['client_secret']
            return credentials
        else:
            raise Exception(f"Client registration failed: {response.text}")
    
    def authenticate(self, scopes: List[str] = None) -> bool:
        """Authenticate with client credentials"""
        if not self.client_id or not self.client_secret:
            raise ValueError("Client must be registered first")
        
        if scopes is None:
            scopes = ["read"]
        
        response = self.session.post(f'{self.base_url}/a2a/auth', json={
            'client_id': self.client_id,
            'client_secret': self.client_secret,
            'scopes': scopes
        })
        
        if response.status_code == 200:
            data = response.json()
            self.session_token = data['session_token']
            return True
        else:
            raise Exception(f"Authentication failed: {response.text}")
    
    def set_user_jwt_token(self, jwt_token: str):
        """Set user JWT token for tool execution"""
        self.user_jwt_token = jwt_token
        self.session.headers['Authorization'] = f'Bearer {jwt_token}'
    
    def execute_tool(self, tool_name: str, parameters: Dict = None) -> Dict:
        """Execute A2A tool"""
        if not self.user_jwt_token:
            raise ValueError("User JWT token required for tool execution")
        
        payload = {
            "jsonrpc": "2.0",
            "method": "tools.execute",
            "params": {
                "tool_name": tool_name,
                "parameters": parameters or {}
            },
            "id": int(time.time())
        }
        
        response = self.session.post(f'{self.base_url}/a2a/execute', json=payload)
        
        if response.status_code == 200:
            return response.json()
        else:
            raise Exception(f"Tool execution failed: {response.text}")
    
    def get_activities(self, limit: int = 10, provider: str = "strava") -> List[Dict]:
        """Get user's fitness activities"""
        result = self.execute_tool("get_activities", {
            "limit": limit,
            "provider": provider
        })
        
        if "result" in result and "activities" in result["result"]:
            return result["result"]["activities"]
        elif "error" in result:
            raise Exception(f"Error getting activities: {result['error']}")
        else:
            return []
    
    def connect_strava(self) -> Dict:
        """Initiate Strava OAuth connection"""
        return self.execute_tool("connect_strava")

# Example usage
if __name__ == "__main__":
    # Initialize client
    client = PierreA2AClient()
    
    # Register client
    credentials = client.register_client(
        name="My Fitness App",
        description="AI-powered fitness analytics",
        capabilities=["fitness-data-analysis"],
        contact_email="developer@myapp.com"
    )
    print(f"Client registered: {credentials['client_id']}")
    
    # Authenticate
    client.authenticate(scopes=["read", "write"])
    print("Client authenticated")
    
    # Set user JWT token (obtained through user authentication)
    client.set_user_jwt_token("your_user_jwt_token_here")
    
    # Execute tools
    activities = client.get_activities(limit=5)
    print(f"Retrieved {len(activities)} activities")
    
    # Connect to Strava
    strava_auth = client.connect_strava()
    print(f"Strava OAuth URL: {strava_auth['result']['authorization_url']}")
```

## Production Considerations

### Security
- **HTTPS Only**: Always use HTTPS in production
- **Client Secret Storage**: Store client secrets securely (environment variables, key management)
- **Token Rotation**: Implement session token refresh logic
- **Rate Limiting**: Respect rate limits to avoid blocking

### Performance
- **Connection Pooling**: Use persistent HTTP connections
- **Caching**: Cache session tokens until expiration
- **Batch Operations**: Process multiple activities in single requests when possible
- **Error Handling**: Implement exponential backoff for transient errors

### Monitoring
- **Usage Tracking**: Monitor API usage and rate limits
- **Error Logging**: Log all errors for debugging
- **Health Checks**: Implement health check endpoints
- **Performance Metrics**: Track response times and success rates

### Scalability
- **Load Balancing**: Distribute requests across multiple server instances
- **Database Scaling**: Use read replicas for high-throughput scenarios
- **Caching Layer**: Implement Redis or similar for session management
- **Async Processing**: Use background jobs for heavy analytics operations

### Error Recovery
- **Retry Logic**: Implement intelligent retry with exponential backoff
- **Circuit Breaker**: Prevent cascading failures
- **Fallback Mechanisms**: Provide degraded functionality when services are unavailable
- **Graceful Degradation**: Handle partial failures gracefully

## Support

For A2A protocol support:
- Check server logs for detailed error information
- Verify client credentials are valid and not expired
- Ensure user has completed OAuth flow for provider data access
- Review rate limiting and API usage statistics
- Test with provided Python client examples

For technical issues or questions, review the complete examples in `/examples/python/a2a/` directory.