# Authentication Guide

Complete guide to authentication in the Pierre MCP Fitness API platform.

## Overview

The Pierre MCP Server supports multiple authentication methods for different use cases:

- **JWT Tokens**: For user authentication in web applications
- **API Keys**: For production integrations and B2B customers  
- **A2A Authentication**: For agent-to-agent communication
- **OAuth2 Flow**: For fitness provider connections (Strava, Fitbit, etc.)

## JWT Authentication

### JWT Token Structure

JWT tokens include the following claims:

```json
{
  "sub": "user_12345",           // User ID (subject)
  "email": "user@example.com",   // User email
  "iat": 1705123456,             // Issued at (Unix timestamp)
  "exp": 1705209856,             // Expires at (Unix timestamp)
  "iss": "pierre-mcp-server",    // Issuer
  "aud": "pierre-api",           // Audience
  "permissions": [               // User permissions
    "read_activities",
    "write_goals",
    "admin_access"
  ]
}
```

### Getting a JWT Token

**1. User Registration and Login**

```bash
# Register new user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password123",
    "display_name": "John Doe"
  }'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "secure_password123"
  }'
```

**Response:**
```json
{
  "jwt_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyXzEyMzQ1IiwiZW1haWwiOiJ1c2VyQGV4YW1wbGUuY29tIiwiaWF0IjoxNzA1MTIzNDU2LCJleHAiOjE3MDUyMDk4NTYsImlzcyI6InBpZXJyZS1tY3Atc2VydmVyIiwiYXVkIjoicGllcnJlLWFwaSJ9.signature_here",
  "expires_at": "2024-01-16T10:30:56Z",
  "user": {
    "user_id": "user_12345",
    "email": "user@example.com",
    "display_name": "John Doe"
  }
}
```

**2. Using JWT Token**

Include the JWT token in the Authorization header:

```bash
# MCP request with JWT
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "id": 1,
    "params": {
      "name": "get_activities",
      "arguments": {
        "provider": "strava",
        "limit": 10
      }
    }
  }'
```

### Token Refresh

JWT tokens expire after 24 hours. Refresh them before expiration:

```bash
curl -X POST http://localhost:8081/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{
    "token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
    "user_id": "user_12345"
  }'
```

## API Key Authentication

### API Key Types

- **Trial Keys**: `pk_trial_...` - 14-day expiration, 1,000 requests/month
- **Production Keys**: `pk_live_...` - Long-term, tiered rate limits

### Getting an API Key

**1. Request Trial Key (Self-Service)**

```bash
curl -X POST http://localhost:8081/api-keys/request-trial \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <jwt_token>" \
  -d '{
    "name": "My App Integration",
    "description": "Testing fitness data integration"
  }'
```

**Response:**
```json
{
  "success": true,
  "api_key": "pk_trial_1234567890abcdef",
  "rate_limit": {
    "requests": 1000,
    "window": 2592000
  },
  "expires_at": "2024-02-01T10:30:00Z"
}
```

**2. Enterprise API Key (Admin Required)**

Contact your account manager or use admin endpoints for production keys.

### Using API Keys

Include API key in the `X-API-Key` header:

```bash
curl -X POST http://localhost:8081/tools/get_activities \
  -H "Content-Type: application/json" \
  -H "X-API-Key: pk_live_1234567890abcdef" \
  -d '{
    "provider": "strava",
    "limit": 20
  }'
```

### Rate Limits by Tier

| Tier | Monthly Requests | Burst Limit | Key Prefix |
|------|------------------|-------------|------------|
| Trial | 1,000 | 50/hour | `pk_trial_` |
| Starter | 10,000 | 100/hour | `pk_live_` |
| Professional | 100,000 | 500/hour | `pk_live_` |
| Enterprise | Unlimited | 1000/hour | `pk_live_` |

## A2A Authentication

### Client Registration

Register your agent for A2A communication:

```bash
curl -X POST http://localhost:8081/a2a/clients \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <admin_jwt_token>" \
  -d '{
    "name": "My Fitness Bot",
    "description": "AI fitness assistant that analyzes workout data",
    "capabilities": ["fitness-data-analysis", "goal-management"],
    "contact_email": "developer@myapp.com",
    "redirect_uris": ["https://myapp.com/callback"]
  }'
```

**Response:**
```json
{
  "client_id": "a2a_client_abc123",
  "client_secret": "a2a_secret_def456",
  "created_at": "2024-01-15T10:30:00Z"
}
```

### A2A Authentication Flow

**1. Get Session Token**

```bash
curl -X POST http://localhost:8081/a2a/auth \
  -H "Content-Type: application/json" \
  -d '{
    "client_id": "a2a_client_abc123",
    "client_secret": "a2a_secret_def456",
    "scopes": ["read", "write"]
  }'
```

**Response:**
```json
{
  "status": "authenticated",
  "session_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...",
  "expires_in": 86400,
  "token_type": "Bearer",
  "scope": "read write"
}
```

**2. Execute Tools**

```bash
curl -X POST http://localhost:8081/a2a/execute \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "id": 1,
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "provider": "strava",
        "limit": 10
      }
    }
  }'
```

## OAuth2 Flow (Provider Connections)

### Connecting Fitness Providers

**1. Generate Authorization URL**

```bash
curl -X POST http://localhost:8081/tools/connect_strava \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <jwt_token>" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "id": 1,
    "params": {
      "name": "connect_strava",
      "arguments": {}
    }
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "authorization_url": "https://www.strava.com/oauth/authorize?client_id=123&redirect_uri=http://localhost:8081/oauth/strava/callback&response_type=code&scope=read,activity:read_all&state=abc123",
    "state": "abc123"
  }
}
```

**2. Handle OAuth Callback**

After user authorizes, the callback URL receives the authorization code which is automatically exchanged for tokens.

**3. Check Connection Status**

```bash
curl -X POST http://localhost:8081/tools/get_connection_status \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <jwt_token>" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "id": 1,
    "params": {
      "name": "get_connection_status",
      "arguments": {}
    }
  }'
```

## Error Handling

### Common Authentication Errors

**Invalid JWT Token (401)**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Invalid or expired authentication token",
    "data": {
      "error_type": "INVALID_TOKEN",
      "suggestion": "Please login again to get a new token"
    }
  }
}
```

**Invalid API Key (403)**
```json
{
  "error": "Invalid API key",
  "message": "API key is invalid, expired, or rate limited",
  "details": {
    "key_prefix": "pk_trial_1234",
    "status": "expired",
    "expires_at": "2024-01-15T10:30:00Z"
  }
}
```

**Rate Limited (429)**
```json
{
  "error": "Rate limit exceeded",
  "message": "Monthly rate limit of 1000 requests exceeded",
  "details": {
    "limit": 1000,
    "window": 2592000,
    "reset_time": "2024-02-01T00:00:00Z",
    "upgrade_url": "https://pierre-fitness.com/upgrade"
  }
}
```

### Debugging Authentication Issues

**1. Validate JWT Token**

Decode JWT token at [jwt.io](https://jwt.io) to check:
- Expiration time (`exp` claim)
- User ID (`sub` claim)  
- Permissions (`permissions` claim)

**2. Check API Key Status**

```bash
curl -X GET http://localhost:8081/api-keys/status \
  -H "X-API-Key: pk_trial_1234567890abcdef"
```

**3. Test Connection**

```bash
curl -X GET http://localhost:8081/health \
  -H "Authorization: Bearer <jwt_token>"
```

## Security Best Practices

### Token Storage
- Store JWT tokens securely (encrypted storage, secure cookies)
- Never expose API keys in client-side code
- Use environment variables for production keys

### Token Rotation
- Refresh JWT tokens before expiration
- Rotate API keys regularly
- Monitor for unusual usage patterns

### Rate Limiting
- Implement client-side rate limiting
- Handle 429 responses gracefully
- Cache responses when appropriate

### CORS and Headers
- Configure CORS properly for web applications
- Use HTTPS in production
- Validate all input parameters