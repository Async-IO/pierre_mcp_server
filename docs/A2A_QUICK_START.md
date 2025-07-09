# A2A Quick Start Guide

Get started with the Pierre Fitness API A2A protocol in 5 minutes. This guide shows you how to register your application, authenticate, and execute tools to get real fitness data.

## Prerequisites

- Pierre server running on `localhost:8081`
- Python 3.7+ with `requests` library
- Valid user account with connected Strava/Fitbit

## Step 1: Register Your A2A Client

First, register your application to get client credentials:

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

**Important**: Save your `client_id` and `client_secret` securely. You'll need them for authentication.

## Step 2: Authenticate Your Client

Use your client credentials to get a session token:

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

## Step 3: Get User JWT Token

A2A tool execution requires a user's JWT token. Register a user and login:

```bash
# Register user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPass123",
    "display_name": "Test User"
  }'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "UserPass123"
  }'
```

**Response:**
```json
{
  "jwt_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_at": "2025-07-10T21:47:46.321827+00:00",
  "user": {
    "user_id": "d3461ba9-38c5-47fe-b31f-c1726e6105b4",
    "email": "user@example.com",
    "display_name": "Test User"
  }
}
```

## Step 4: Connect to Strava

Before getting real data, connect the user to Strava:

```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "connect_strava",
      "parameters": {}
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "authorization_url": "https://www.strava.com/oauth/authorize?client_id=163846&redirect_uri=...",
    "state": "d3461ba9-38c5-47fe-b31f-c1726e6105b4:cf07d961-ff19-4371-9ec9-97b259020c51",
    "provider": "strava"
  }
}
```

Visit the `authorization_url` in your browser to complete the OAuth flow.

## Step 5: Get Fitness Activities

Now you can get real fitness data:

```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "limit": 5,
        "provider": "strava"
      }
    },
    "id": 1
  }'
```

**Response:**
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

## Complete Python Example

Here's a complete working example:

```python
import requests
import json

class QuickA2AClient:
    def __init__(self, base_url="http://localhost:8081"):
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({'Content-Type': 'application/json'})
        
    def register_client(self, name, description, capabilities, contact_email):
        """Register A2A client"""
        response = self.session.post(f'{self.base_url}/a2a/clients', json={
            'name': name,
            'description': description,
            'capabilities': capabilities,
            'contact_email': contact_email
        })
        return response.json()
    
    def authenticate_client(self, client_id, client_secret, scopes=None):
        """Authenticate client and get session token"""
        if scopes is None:
            scopes = ["read", "write"]
            
        response = self.session.post(f'{self.base_url}/a2a/auth', json={
            'client_id': client_id,
            'client_secret': client_secret,
            'scopes': scopes
        })
        return response.json()
    
    def execute_tool(self, tool_name, parameters, jwt_token):
        """Execute A2A tool with user JWT token"""
        headers = {'Authorization': f'Bearer {jwt_token}'}
        payload = {
            "jsonrpc": "2.0",
            "method": "tools.execute",
            "params": {
                "tool_name": tool_name,
                "parameters": parameters
            },
            "id": 1
        }
        
        response = self.session.post(
            f'{self.base_url}/a2a/execute', 
            json=payload, 
            headers=headers
        )
        return response.json()

# Usage example
if __name__ == "__main__":
    client = QuickA2AClient()
    
    # 1. Register client
    print("1. Registering A2A client...")
    credentials = client.register_client(
        name="Quick Start App",
        description="A2A quick start example",
        capabilities=["fitness-data-analysis"],
        contact_email="developer@example.com"
    )
    print(f"Client registered: {credentials['client_id']}")
    
    # 2. Authenticate client
    print("2. Authenticating client...")
    auth_response = client.authenticate_client(
        client_id=credentials['client_id'],
        client_secret=credentials['client_secret']
    )
    print(f"Client authenticated: {auth_response['status']}")
    
    # 3. Execute tool (requires user JWT token)
    print("3. Executing get_activities tool...")
    user_jwt_token = "YOUR_USER_JWT_TOKEN_HERE"  # From Step 3 above
    
    result = client.execute_tool(
        tool_name="get_activities",
        parameters={"limit": 3, "provider": "strava"},
        jwt_token=user_jwt_token
    )
    
    if "result" in result:
        activities = result["result"]["activities"]
        print(f"Retrieved {len(activities)} activities")
        for activity in activities:
            print(f"  - {activity['name']}: {activity['distance_meters']/1000:.1f}km")
    else:
        print(f"Error: {result.get('error', 'Unknown error')}")
```

## Common Issues

### 1. Authentication Failed
**Error**: `A2A authentication failed`
**Solution**: Verify your client credentials are correct and the client is registered.

### 2. Missing Authorization Header
**Error**: `Missing Authorization header`
**Solution**: Tool execution requires a user JWT token, not the A2A session token.

### 3. No Strava Token Found
**Error**: `No Strava token found for user`
**Solution**: Complete the OAuth flow using the `connect_strava` tool.

### 4. Rate Limit Exceeded
**Error**: `Rate limit exceeded`
**Solution**: Implement exponential backoff and respect rate limits.

## Next Steps

1. **Explore More Tools**: Try `calculate_fitness_score`, `generate_recommendations`, and `analyze_activity`
2. **Read Full Documentation**: Check [A2A_REFERENCE.md](A2A_REFERENCE.md) for complete API reference
3. **Production Setup**: Review security and scaling considerations
4. **Error Handling**: Implement robust error handling and retry logic

## Resources

- **Complete A2A Reference**: [A2A_REFERENCE.md](A2A_REFERENCE.md)
- **Python Examples**: `/examples/python/a2a/`
- **API Reference**: [API_REFERENCE.md](API_REFERENCE.md)
- **Getting Started Guide**: [GETTING_STARTED.md](GETTING_STARTED.md)

---

**🚀 You're now ready to build with the A2A protocol!** The Pierre Fitness API provides comprehensive fitness data access for enterprise integrations.