# Error Handling Reference

Complete guide to error codes, response formats, and troubleshooting for the Pierre MCP Fitness API.

## Error Response Formats

### MCP Protocol Errors

MCP errors follow JSON-RPC 2.0 specification:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Authentication failed",
    "data": {
      "error_type": "INVALID_TOKEN",
      "suggestion": "Please login again to get a new token",
      "documentation_url": "https://docs.pierre-fitness.com/authentication"
    }
  }
}
```

### HTTP API Errors

HTTP errors return JSON with standard structure:

```json
{
  "error": "Invalid API key",
  "message": "API key is invalid, expired, or rate limited",
  "details": {
    "key_prefix": "pk_trial_1234",
    "status": "expired",
    "expires_at": "2024-01-15T10:30:00Z"
  },
  "error_code": "API_KEY_EXPIRED",
  "timestamp": "2024-01-15T14:23:00Z"
}
```

### A2A Protocol Errors

A2A errors follow JSON-RPC format with specific error codes:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Tool execution failed: Invalid provider",
    "data": {
      "tool_name": "get_activities",
      "provider": "invalid_provider",
      "valid_providers": ["strava", "fitbit", "garmin"]
    }
  }
}
```

## Error Code Reference

### Authentication Errors (4xx)

#### 401 Unauthorized

**JWT Token Errors**

| Error Type | Code | Description | Solution |
|------------|------|-------------|----------|
| `INVALID_TOKEN` | -32001 | Token is malformed or invalid | Get new token via login |
| `EXPIRED_TOKEN` | -32001 | Token has expired | Refresh token or login again |
| `MISSING_TOKEN` | -32001 | No authentication provided | Include Bearer token |

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32001,
    "message": "Invalid or expired authentication token",
    "data": {
      "error_type": "EXPIRED_TOKEN",
      "expired_at": "2024-01-15T10:30:00Z",
      "suggestion": "Use /auth/refresh endpoint or login again"
    }
  }
}
```

**API Key Errors**

| Error Type | Code | Description | Solution |
|------------|------|-------------|----------|
| `API_KEY_INVALID` | 401 | Key not found or malformed | Check key format and validity |
| `API_KEY_EXPIRED` | 401 | Key has expired | Request new key |
| `API_KEY_DEACTIVATED` | 401 | Key has been deactivated | Contact administrator |

```json
{
  "error": "Invalid API key",
  "message": "API key pk_trial_1234 has expired",
  "details": {
    "key_prefix": "pk_trial_1234",
    "status": "expired",
    "expires_at": "2024-01-15T10:30:00Z",
    "renewal_url": "https://pierre-fitness.com/api-keys"
  },
  "error_code": "API_KEY_EXPIRED"
}
```

#### 403 Forbidden

**Permission Errors**

| Error Type | Code | Description | Solution |
|------------|------|-------------|----------|
| `INSUFFICIENT_PERMISSIONS` | -32002 | User lacks required permission | Request elevated access |
| `RESOURCE_ACCESS_DENIED` | -32002 | Access to specific resource denied | Check resource ownership |

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32002,
    "message": "Insufficient permissions",
    "data": {
      "required_permission": "admin_access",
      "user_permissions": ["read_activities", "write_goals"],
      "suggestion": "Contact administrator for elevated access"
    }
  }
}
```

### Rate Limiting Errors (429)

**Rate Limit Exceeded**

```json
{
  "error": "Rate limit exceeded",
  "message": "Monthly rate limit of 1000 requests exceeded",
  "details": {
    "limit": 1000,
    "window": 2592000,
    "reset_time": "2024-02-01T00:00:00Z",
    "requests_used": 1000,
    "upgrade_url": "https://pierre-fitness.com/upgrade"
  },
  "error_code": "RATE_LIMIT_EXCEEDED",
  "retry_after": 3600
}
```

**Burst Limit Exceeded**

```json
{
  "error": "Burst rate limit exceeded",
  "message": "Too many requests in short timeframe",
  "details": {
    "burst_limit": 50,
    "burst_window": 3600,
    "retry_after": 300
  },
  "error_code": "BURST_LIMIT_EXCEEDED",
  "retry_after": 300
}
```

### Client Errors (400)

#### Invalid Request Format

**MCP Protocol Violations**

| Error Type | Code | Description | Solution |
|------------|------|-------------|----------|
| `INVALID_REQUEST` | -32600 | Malformed JSON-RPC request | Fix JSON structure |
| `METHOD_NOT_FOUND` | -32601 | Unknown method name | Check available methods |
| `INVALID_PARAMS` | -32602 | Invalid parameters | Validate parameter format |

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32602,
    "message": "Invalid parameters",
    "data": {
      "parameter": "limit",
      "provided_value": -5,
      "expected": "positive integer between 1 and 100",
      "example": 20
    }
  }
}
```

#### Tool-Specific Errors

**Provider Connection Errors**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Provider not connected",
    "data": {
      "tool_name": "get_activities",
      "provider": "strava",
      "error_type": "PROVIDER_NOT_CONNECTED",
      "suggestion": "Use connect_strava tool to establish connection",
      "authorization_url": "https://www.strava.com/oauth/authorize?..."
    }
  }
}
```

**Data Validation Errors**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Invalid goal parameters",
    "data": {
      "tool_name": "set_goal",
      "validation_errors": [
        {
          "field": "target_date",
          "error": "Date must be in the future",
          "provided": "2023-01-01",
          "minimum": "2024-01-16"
        },
        {
          "field": "target_value",
          "error": "Must be positive number",
          "provided": -100
        }
      ]
    }
  }
}
```

### Server Errors (5xx)

#### 500 Internal Server Error

**Database Errors**

```json
{
  "error": "Database error",
  "message": "Failed to retrieve user data",
  "details": {
    "error_type": "DATABASE_CONNECTION_FAILED",
    "request_id": "req_abc123",
    "timestamp": "2024-01-15T14:23:00Z"
  },
  "error_code": "DATABASE_ERROR"
}
```

**Provider API Errors**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Provider API error",
    "data": {
      "provider": "strava",
      "provider_error": "Rate limit exceeded",
      "provider_status": 429,
      "retry_after": 3600,
      "suggestion": "Try again in 1 hour"
    }
  }
}
```

#### 503 Service Unavailable

**Maintenance Mode**

```json
{
  "error": "Service unavailable",
  "message": "System is temporarily unavailable for maintenance",
  "details": {
    "maintenance_start": "2024-01-15T14:00:00Z",
    "estimated_end": "2024-01-15T16:00:00Z",
    "status_url": "https://status.pierre-fitness.com"
  },
  "error_code": "MAINTENANCE_MODE"
}
```

## A2A Specific Errors

### Client Registration Errors

**Invalid Client Request**

```json
{
  "error": "Invalid client registration",
  "message": "Client name already exists",
  "details": {
    "field": "name",
    "conflict": "My Fitness Bot",
    "suggestion": "Choose a unique client name"
  },
  "error_code": "CLIENT_NAME_CONFLICT"
}
```

### Tool Execution Errors

**Tool Not Found**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32601,
    "message": "Method 'invalid_tool' not found",
    "data": {
      "available_methods": [
        "tools.execute",
        "client.info",
        "session.heartbeat",
        "capabilities.list"
      ]
    }
  }
}
```

**Capability Not Granted**

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Capability not granted",
    "data": {
      "tool_name": "set_goal",
      "required_capability": "goal-management",
      "granted_capabilities": ["fitness-data-analysis"],
      "suggestion": "Request goal-management capability during client registration"
    }
  }
}
```

## Troubleshooting Guide

### Common Issues and Solutions

#### 1. Authentication Problems

**Problem**: "Invalid or expired authentication token"

**Solutions**:
1. Check token expiration: Decode JWT at [jwt.io](https://jwt.io)
2. Refresh token using `/auth/refresh` endpoint
3. Login again if refresh fails
4. Verify token format includes `Bearer ` prefix

**Problem**: "API key invalid"

**Solutions**:
1. Verify key format (`pk_live_` or `pk_trial_`)
2. Check key expiration date
3. Confirm key is included in `X-API-Key` header
4. Request new key if expired

#### 2. Rate Limiting Issues

**Problem**: "Rate limit exceeded"

**Solutions**:
1. Implement exponential backoff
2. Cache responses to reduce requests
3. Check `retry_after` header value
4. Upgrade to higher tier if needed

**Example Retry Logic**:
```python
import time
import requests

def api_request_with_retry(url, headers, data, max_retries=3):
    for attempt in range(max_retries):
        response = requests.post(url, headers=headers, json=data)
        
        if response.status_code == 429:
            retry_after = int(response.headers.get('retry-after', 60))
            time.sleep(retry_after)
            continue
            
        return response
    
    raise Exception("Max retries exceeded")
```

#### 3. Provider Connection Issues

**Problem**: "Provider not connected"

**Solutions**:
1. Use `connect_strava` tool to get authorization URL
2. Complete OAuth flow in browser
3. Check connection status with `get_connection_status`
4. Re-authorize if token expired

#### 4. Tool Parameter Validation

**Problem**: "Invalid parameters"

**Solutions**:
1. Check required parameters in tool documentation
2. Validate parameter types (string, number, boolean)
3. Ensure enums match allowed values
4. Check parameter ranges and limits

### Debug Mode

Enable debug mode to get detailed error information:

**Environment Variable**:
```bash
export PIERRE_DEBUG=true
```

**Debug Response Format**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Tool execution failed",
    "data": {
      "debug_info": {
        "stack_trace": "...",
        "request_id": "req_abc123",
        "execution_time_ms": 150,
        "database_queries": 3,
        "external_api_calls": 1
      }
    }
  }
}
```

### Getting Help

1. **Documentation**: Check [docs.pierre-fitness.com](https://docs.pierre-fitness.com)
2. **Status Page**: Monitor [status.pierre-fitness.com](https://status.pierre-fitness.com)
3. **Support**: Contact support with error codes and request IDs
4. **Community**: Join discussions at [community.pierre-fitness.com](https://community.pierre-fitness.com)

### Request ID Tracking

All errors include a `request_id` for tracking:

```json
{
  "error": "Database error",
  "request_id": "req_abc123def456",
  "timestamp": "2024-01-15T14:23:00Z"
}
```

Include the request ID when contacting support for faster resolution.