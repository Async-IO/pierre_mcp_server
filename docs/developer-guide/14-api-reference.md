# API Reference

This document provides a comprehensive reference for all Pierre MCP Server REST API endpoints, request/response formats, and authentication requirements.

## Table of Contents

1. [Base URL and Versioning](#base-url-and-versioning)
2. [Authentication](#authentication)
3. [Error Handling](#error-handling)
4. [Rate Limiting](#rate-limiting)
5. [Authentication Routes](#authentication-routes)
6. [OAuth Routes](#oauth-routes)
7. [API Key Management Routes](#api-key-management-routes)
8. [Dashboard Routes](#dashboard-routes)
9. [A2A (Agent-to-Agent) Routes](#a2a-agent-to-agent-routes)
10. [Admin Routes](#admin-routes)
11. [Tenant Management Routes](#tenant-management-routes)
12. [Configuration Routes](#configuration-routes)
13. [WebSocket Endpoints](#websocket-endpoints)
14. [Response Codes](#response-codes)

## Base URL and Versioning

```
Production: https://api.pierremcp.com (port 8081 behind load balancer)
Development: http://localhost:8081 (HTTP API and authentication)
MCP Protocol: http://localhost:8080 (MCP JSON-RPC)
```

**API Version**: All endpoints are currently version 1.0. Future versions will be indicated in the URL path.

**Content-Type**: All requests must use `Content-Type: application/json` unless otherwise specified.

## Authentication

Pierre MCP Server supports two authentication methods:

### 1. JWT Bearer Tokens (Web Sessions)
Used for web dashboard and user interface interactions.

```http
Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...
```

### 2. API Keys (Programmatic Access)
Used for MCP clients and A2A integrations.

```http
X-API-Key: pk_live_abc123def456ghi789...
```

### 3. A2A Session Tokens
Used for Agent-to-Agent protocol after authentication.

```http
Authorization: Bearer a2a_session_token_xyz...
```

## Error Handling

All API errors follow a consistent JSON format:

```json
{
  "error": "authentication_failed",
  "message": "Invalid API key provided",
  "details": {
    "error_code": "E_AUTH_001",
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req_abc123"
  }
}
```

### Common Authentication Errors

**User Account Pending Approval:**
```json
{
  "error": "authentication_failed",
  "message": "Your account is pending admin approval",
  "details": {
    "error_code": "E_USER_001",
    "user_status": "pending",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

**User Account Suspended:**
```json
{
  "error": "authentication_failed",
  "message": "Your account has been suspended",
  "details": {
    "error_code": "E_USER_002", 
    "user_status": "suspended",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

**Invalid Credentials:**
```json
{
  "error": "authentication_failed",
  "message": "Invalid email or password",
  "details": {
    "error_code": "E_AUTH_002",
    "timestamp": "2024-01-15T10:30:00Z"
  }
}
```

## Rate Limiting

Rate limits are enforced per API key/user with the following headers included in all responses:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995260
Retry-After: 60
```

## Authentication Routes

### POST /api/auth/register

Register a new user account.

**Request**:
```json
{
  "email": "user@example.com",
  "password": "securepassword123",
  "display_name": "John Doe"
}
```

**Response** (201):
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "User registered successfully. Your account is pending admin approval."
}
```

**Validation**:
- Email must be valid format
- Password must be at least 8 characters
- Email must be unique

### POST /api/auth/login

Authenticate user and receive JWT token.

**Request**:
```json
{
  "email": "user@example.com",
  "password": "securepassword123"
}
```

**Response** (200):
```json
{
  "jwt_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-16T10:30:00Z",
  "user": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "display_name": "John Doe",
    "is_admin": false
  }
}
```

### POST /api/auth/refresh-token

Refresh an expired JWT token.

**Request**:
```json
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "user_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Response** (200):
```json
{
  "jwt_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expires_at": "2024-01-16T10:30:00Z",
  "user": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "display_name": "John Doe",
    "is_admin": false
  }
}
```

## OAuth Routes

### GET /api/oauth/{provider}/authorize

Get OAuth authorization URL for fitness provider.

**Parameters**:
- `provider`: `strava` | `fitbit`

**Query Parameters**:
- `tenant_id`: UUID (optional, for multi-tenant setups)

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "authorization_url": "https://www.strava.com/oauth/authorize?client_id=...",
  "state": "550e8400-e29b-41d4-a716-446655440000:abc123",
  "instructions": "Visit the URL above to authorize access to your Strava account.",
  "expires_in_minutes": 10
}
```

### GET /api/oauth/{provider}/callback

Handle OAuth callback after user authorization.

**Parameters**:
- `provider`: `strava` | `fitbit`

**Query Parameters**:
- `code`: Authorization code from provider
- `state`: CSRF protection state parameter

**Response** (200):
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "provider": "strava",
  "expires_at": "2024-07-15T10:30:00Z",
  "scopes": "read,activity:read_all"
}
```

### GET /api/oauth/status

Get connection status for all OAuth providers.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
[
  {
    "provider": "strava",
    "connected": true,
    "expires_at": "2024-07-15T10:30:00Z",
    "scopes": "read,activity:read_all"
  },
  {
    "provider": "fitbit",
    "connected": false,
    "expires_at": null,
    "scopes": null
  }
]
```

### DELETE /api/oauth/{provider}/disconnect

Disconnect from an OAuth provider.

**Parameters**:
- `provider`: `strava` | `fitbit`

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "message": "Successfully disconnected from strava"
}
```

## API Key Management Routes

### POST /api/keys/create

Create a new API key.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Request**:
```json
{
  "name": "Production Key",
  "description": "Main application API key",
  "tier": "premium"
}
```

**Response** (201):
```json
{
  "api_key": "pk_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234yz567",
  "key_info": {
    "id": "key_550e8400-e29b-41d4-a716-446655440000",
    "name": "Production Key",
    "description": "Main application API key",
    "tier": "premium",
    "key_prefix": "pk_live_abc123",
    "is_active": true,
    "last_used_at": null,
    "expires_at": null,
    "created_at": "2024-01-15T10:30:00Z"
  },
  "warning": "Store this API key securely. It will not be shown again."
}
```

### POST /api/keys/create-simple

Create a trial API key with simplified request.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Request**:
```json
{
  "name": "Trial Key",
  "description": "Trial API key for testing"
}
```

**Response** (201):
```json
{
  "api_key": "pk_trial_abc123def456ghi789jkl012mno345pqr678stu901vwx234",
  "key_info": {
    "id": "key_550e8400-e29b-41d4-a716-446655440000",
    "name": "Trial Key",
    "description": "Trial API key for testing",
    "tier": "trial",
    "key_prefix": "pk_trial_abc123",
    "is_active": true,
    "last_used_at": null,
    "expires_at": "2024-02-15T10:30:00Z",
    "created_at": "2024-01-15T10:30:00Z"
  },
  "warning": "This is a trial API key that will expire on 2024-02-15. Store it securely - it cannot be recovered once lost."
}
```

### GET /api/keys/list

List all API keys for the authenticated user.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "api_keys": [
    {
      "id": "key_550e8400-e29b-41d4-a716-446655440000",
      "name": "Production Key",
      "description": "Main application API key",
      "tier": "premium",
      "key_prefix": "pk_live_abc123",
      "is_active": true,
      "last_used_at": "2024-01-15T09:30:00Z",
      "expires_at": null,
      "created_at": "2024-01-15T10:30:00Z"
    }
  ]
}
```

### DELETE /api/keys/{key_id}

Deactivate an API key.

**Parameters**:
- `key_id`: API key identifier

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "message": "API key key_550e8400-e29b-41d4-a716-446655440000 has been deactivated",
  "deactivated_at": "2024-01-15T10:30:00Z"
}
```

### GET /api/keys/{key_id}/usage

Get usage statistics for an API key.

**Parameters**:
- `key_id`: API key identifier

**Query Parameters**:
- `start_date`: ISO 8601 datetime (required)
- `end_date`: ISO 8601 datetime (required)

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "stats": {
    "total_requests": 1500,
    "successful_requests": 1485,
    "failed_requests": 15,
    "total_response_time_ms": 750000,
    "average_response_time_ms": 500,
    "tool_usage": {
      "get_activities": 800,
      "get_athlete_stats": 400,
      "upload_activity": 300
    },
    "requests_by_hour": {
      "2024-01-15T00:00:00Z": 50,
      "2024-01-15T01:00:00Z": 75
    }
  }
}
```

## Dashboard Routes

### GET /api/dashboard/overview

Get dashboard overview with key metrics.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "total_api_keys": 3,
  "active_api_keys": 2,
  "total_requests_today": 1250,
  "total_requests_this_month": 45000,
  "current_month_usage_by_tier": [
    {
      "tier": "premium",
      "key_count": 1,
      "total_requests": 35000,
      "average_requests_per_key": 35000.0
    },
    {
      "tier": "trial",
      "key_count": 1,
      "total_requests": 10000,
      "average_requests_per_key": 10000.0
    }
  ],
  "recent_activity": [
    {
      "timestamp": "2024-01-15T10:25:00Z",
      "api_key_name": "Production Key",
      "tool_name": "get_activities",
      "status_code": 200,
      "response_time_ms": 150
    }
  ]
}
```

### GET /api/dashboard/analytics

Get usage analytics for charts and visualization.

**Query Parameters**:
- `days`: Number of days to analyze (default: 7)

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "time_series": [
    {
      "timestamp": "2024-01-15T00:00:00Z",
      "request_count": 1200,
      "error_count": 15,
      "average_response_time": 245.5
    }
  ],
  "top_tools": [
    {
      "tool_name": "get_activities",
      "request_count": 8000,
      "success_rate": 98.5,
      "average_response_time": 180.2
    }
  ],
  "error_rate": 1.2,
  "average_response_time": 234.8
}
```

### GET /api/dashboard/rate-limits

Get rate limit overview for all API keys.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
[
  {
    "api_key_id": "key_550e8400-e29b-41d4-a716-446655440000",
    "api_key_name": "Production Key",
    "tier": "premium",
    "current_usage": 1250,
    "limit": 10000,
    "usage_percentage": 12.5,
    "reset_date": "2024-02-01T00:00:00Z"
  }
]
```

### GET /api/dashboard/logs

Get request logs with filtering options.

**Query Parameters**:
- `api_key_id`: Filter by specific API key (optional)
- `time_range`: `1h` | `24h` | `7d` | `30d` (default: `1h`)
- `status`: Filter by HTTP status code (optional)
- `tool`: Filter by tool name (optional)

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
[
  {
    "id": "log_550e8400-e29b-41d4-a716-446655440000",
    "timestamp": "2024-01-15T10:25:00Z",
    "api_key_id": "key_550e8400-e29b-41d4-a716-446655440000",
    "api_key_name": "Production Key",
    "tool_name": "get_activities",
    "status_code": 200,
    "response_time_ms": 150,
    "error_message": null,
    "request_size_bytes": 256,
    "response_size_bytes": 2048
  }
]
```

### GET /api/dashboard/stats

Get request statistics summary.

**Query Parameters**:
- `api_key_id`: Filter by specific API key (optional)
- `time_range`: `1h` | `24h` | `7d` | `30d` (default: `1h`)

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "total_requests": 1500,
  "successful_requests": 1485,
  "failed_requests": 15,
  "average_response_time": 234.5,
  "min_response_time": 45,
  "max_response_time": 1200,
  "requests_per_minute": 25.0,
  "error_rate": 1.0
}
```

## A2A (Agent-to-Agent) Routes

### GET /.well-known/agent.json

Get A2A agent card for discovery.

**Response** (200):
```json
{
  "agent_id": "pierre-fitness-ai",
  "name": "Pierre Fitness AI",
  "version": "1.0.0",
  "description": "AI-powered fitness data analysis and insights platform",
  "capabilities": [
    "fitness-data-analysis",
    "goal-management", 
    "activity-insights",
    "performance-metrics"
  ],
  "protocols": ["A2A", "MCP"],
  "endpoints": {
    "auth": "/a2a/auth",
    "tools": "/a2a/tools"
  },
  "authentication": {
    "type": "client_credentials",
    "token_endpoint": "/a2a/auth"
  },
  "rate_limits": {
    "requests_per_minute": 60,
    "burst_limit": 100
  }
}
```

### POST /a2a/register

Register a new A2A client.

**Request**:
```json
{
  "name": "Discord Training Bot",
  "description": "Discord bot for fitness tracking",
  "capabilities": ["webhook", "notification"],
  "redirect_uris": ["https://discord.example.com/callback"],
  "contact_email": "admin@example.com",
  "agent_version": "1.2.0",
  "documentation_url": "https://example.com/docs"
}
```

**Response** (201):
```json
{
  "client_id": "a2a_client_550e8400e29b41d4a716446655440000",
  "client_secret": "cs_1234567890abcdef1234567890abcdef",
  "api_key": "pk_a2a_abc123def456ghi789jkl012mno345",
  "public_key": "-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA...",
  "private_key": "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQC...",
  "key_type": "RSA",
  "status": "registered",
  "registered_at": "2024-01-15T10:30:00Z"
}
```

### POST /a2a/auth

Authenticate A2A client and get session token.

**Request**:
```json
{
  "client_id": "a2a_client_550e8400e29b41d4a716446655440000",
  "client_secret": "cs_1234567890abcdef1234567890abcdef",
  "scopes": ["read", "write"]
}
```

**Response** (200):
```json
{
  "status": "authenticated",
  "session_token": "a2a_session_xyz789abc123def456ghi789",
  "expires_in": 86400,
  "token_type": "Bearer",
  "scope": "read write"
}
```

### POST /a2a/tools

Execute A2A tools using JSON-RPC protocol.

**Headers**:
```http
Authorization: Bearer jwt_token
Content-Type: application/json
```

**Request** (JSON-RPC 2.0):
```json
{
  "jsonrpc": "2.0",
  "method": "tools.execute",
  "params": {
    "tool_name": "get_activities",
    "parameters": {
      "limit": 10,
      "activity_type": "run"
    }
  },
  "id": 1
}
```

**Response** (200):
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "activities": [
      {
        "id": "12345678",
        "name": "Morning Run",
        "distance": 5000,
        "duration": 1800,
        "type": "Run"
      }
    ],
    "total_count": 10
  }
}
```

**Available Methods**:
- `tools.execute`: Execute fitness tools
- `client.info`: Get client information
- `session.heartbeat`: Keep session alive
- `capabilities.list`: List available capabilities

### GET /a2a/dashboard

Get A2A dashboard overview.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "total_clients": 5,
  "active_clients": 3,
  "total_sessions": 12,
  "active_sessions": 2,
  "requests_today": 450,
  "requests_this_month": 15000,
  "most_used_capability": "fitness-data-analysis",
  "error_rate": 2.1,
  "usage_by_tier": [
    {
      "tier": "basic",
      "client_count": 3,
      "request_count": 12000,
      "percentage": 80.0
    }
  ]
}
```

### GET /a2a/clients

List all A2A clients.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
[
  {
    "client_id": "a2a_client_550e8400e29b41d4a716446655440000",
    "name": "Discord Training Bot",
    "description": "Discord bot for fitness tracking",
    "capabilities": ["webhook", "notification"],
    "is_active": true,
    "created_at": "2024-01-15T10:30:00Z",
    "last_active": "2024-01-15T10:25:00Z"
  }
]
```

### GET /a2a/clients/{client_id}/usage

Get usage statistics for an A2A client.

**Parameters**:
- `client_id`: A2A client identifier

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "client_id": "a2a_client_550e8400e29b41d4a716446655440000",
  "total_requests": 5000,
  "successful_requests": 4950,
  "failed_requests": 50,
  "average_response_time": 125.5,
  "requests_by_capability": {
    "fitness-data-analysis": 3000,
    "goal-management": 2000
  },
  "error_rate": 1.0
}
```

### GET /a2a/clients/{client_id}/rate-limit

Get rate limit status for an A2A client.

**Parameters**:
- `client_id`: A2A client identifier

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "client_id": "a2a_client_550e8400e29b41d4a716446655440000",
  "current_usage": 45,
  "limit": 1000,
  "usage_percentage": 4.5,
  "reset_time": "2024-01-15T11:00:00Z",
  "blocked": false
}
```

### DELETE /a2a/clients/{client_id}

Deactivate an A2A client.

**Parameters**:
- `client_id`: A2A client identifier

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "message": "Client deactivated successfully",
  "deactivated_at": "2024-01-15T10:30:00Z"
}
```

## Admin Routes

### GET /admin/setup/status

Check if initial admin setup is required.

**Response** (200):
```json
{
  "needs_setup": false,
  "admin_user_exists": true,
  "message": "System is ready for use"
}
```

### POST /admin/setup

Initialize the system with first admin user.

**Request**:
```json
{
  "email": "admin@example.com",
  "password": "securepassword123",
  "display_name": "System Administrator"
}
```

**Response** (201):
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Admin user created successfully"
}
```

### GET /admin/users

List all users in the system.

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Query Parameters**:
- `status`: `active` | `pending` | `suspended` (optional)
- `page`: Page number (default: 1)
- `limit`: Results per page (default: 50)

**Response** (200):
```json
{
  "users": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com",
      "display_name": "John Doe",
      "user_status": "active",
      "tenant_id": "tenant_550e8400-e29b-41d4-a716-446655440000",
      "created_at": "2024-01-15T10:30:00Z",
      "last_active": "2024-01-15T10:25:00Z"
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 50,
    "total": 125,
    "total_pages": 3
  }
}
```

### PUT /admin/users/{user_id}/status

Update user account status.

**Parameters**:
- `user_id`: User identifier

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Request**:
```json
{
  "status": "active",
  "reason": "Account verification completed"
}
```

**Response** (200):
```json
{
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "active",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

### GET /admin/pending-users

List all users awaiting admin approval.

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Response** (200):
```json
{
  "users": [
    {
      "user_id": "550e8400-e29b-41d4-a716-446655440000",
      "email": "user@example.com",
      "display_name": "John Doe",
      "user_status": "pending",
      "created_at": "2024-01-15T10:30:00Z"
    }
  ],
  "total": 1
}
```

### POST /admin/approve-user/{user_id}

Approve a pending user account.

**Parameters**:
- `user_id`: User identifier

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Request**:
```json
{
  "reason": "Approved for production access"
}
```

**Response** (200):
```json
{
  "success": true,
  "message": "User user@example.com approved successfully",
  "user": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "display_name": "John Doe",
    "user_status": "active",
    "approved_by": "admin-user-id",
    "approved_at": "2024-01-15T10:30:00Z"
  }
}
```

### POST /admin/suspend-user/{user_id}

Suspend a user account.

**Parameters**:
- `user_id`: User identifier

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Request**:
```json
{
  "reason": "Policy violation - inappropriate content"
}
```

**Response** (200):
```json
{
  "success": true,
  "message": "User user@example.com suspended successfully",
  "user": {
    "user_id": "550e8400-e29b-41d4-a716-446655440000",
    "email": "user@example.com",
    "display_name": "John Doe", 
    "user_status": "suspended",
    "suspended_by": "admin-user-id",
    "suspended_at": "2024-01-15T10:30:00Z"
  }
}
```

## Tenant Management Routes

### POST /api/tenants

Create a new tenant.

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Request**:
```json
{
  "name": "ACME Fitness Corp",
  "plan_type": "enterprise",
  "settings": {
    "max_users": 1000,
    "custom_branding": true,
    "advanced_analytics": true
  }
}
```

**Response** (201):
```json
{
  "tenant_id": "tenant_550e8400-e29b-41d4-a716-446655440000",
  "name": "ACME Fitness Corp",
  "plan_type": "enterprise",
  "settings": {
    "max_users": 1000,
    "custom_branding": true,
    "advanced_analytics": true
  },
  "created_at": "2024-01-15T10:30:00Z"
}
```

### GET /api/tenants

List all tenants.

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Response** (200):
```json
[
  {
    "tenant_id": "tenant_550e8400-e29b-41d4-a716-446655440000",
    "name": "ACME Fitness Corp",
    "plan_type": "enterprise",
    "user_count": 245,
    "created_at": "2024-01-15T10:30:00Z",
    "last_active": "2024-01-15T10:25:00Z"
  }
]
```

### PUT /api/tenants/{tenant_id}/oauth

Configure OAuth settings for a tenant.

**Parameters**:
- `tenant_id`: Tenant identifier

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Request**:
```json
{
  "provider": "strava",
  "client_id": "your_strava_client_id",
  "client_secret": "your_strava_client_secret",
  "redirect_uri": "https://yourapp.com/oauth/strava/callback",
  "scopes": ["read", "activity:read_all"],
  "is_active": true
}
```

**Response** (200):
```json
{
  "tenant_id": "tenant_550e8400-e29b-41d4-a716-446655440000",
  "provider": "strava",
  "redirect_uri": "https://yourapp.com/oauth/strava/callback",
  "scopes": ["read", "activity:read_all"],
  "is_active": true,
  "updated_at": "2024-01-15T10:30:00Z"
}
```

## Configuration Routes

### GET /api/config/fitness

Get fitness configuration settings.

**Headers**:
```http
Authorization: Bearer jwt_token
```

**Response** (200):
```json
{
  "sport_types": {
    "Run": "run",
    "Ride": "bike_ride",
    "Swim": "swim"
  },
  "intelligence": {
    "effort_thresholds": {
      "light_max": 3.0,
      "moderate_max": 5.0,
      "hard_max": 7.0
    },
    "zone_thresholds": {
      "recovery_max": 60.0,
      "endurance_max": 70.0,
      "tempo_max": 80.0,
      "threshold_max": 90.0
    }
  }
}
```

### PUT /api/config/fitness

Update fitness configuration (Admin only).

**Headers**:
```http
Authorization: Bearer admin_jwt_token
```

**Request**:
```json
{
  "intelligence": {
    "effort_thresholds": {
      "light_max": 2.5,
      "moderate_max": 5.0,
      "hard_max": 7.5
    }
  }
}
```

**Response** (200):
```json
{
  "message": "Fitness configuration updated successfully",
  "updated_at": "2024-01-15T10:30:00Z"
}
```

## WebSocket Endpoints

### WS /ws

WebSocket endpoint for MCP protocol communication.

**Connection Parameters**:
```
wss://api.pierremcp.com/ws
```

**Authentication**:
```http
X-API-Key: pk_live_abc123def456ghi789...
```

**Protocol**: MCP (Model Context Protocol) v2025-06-18 using JSON-RPC 2.0

**Example Messages**:

**Initialize**:
```json
{
  "jsonrpc": "2.0",
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {
      "roots": {
        "listChanged": true
      },
      "sampling": {}
    },
    "clientInfo": {
      "name": "Claude Desktop",
      "version": "0.4.0"
    }
  },
  "id": 1
}
```

**Tool Call**:
```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "limit": 10,
      "activity_type": "run"
    }
  },
  "id": 2
}
```

## Response Codes

### Success Codes
- **200 OK**: Request successful
- **201 Created**: Resource created successfully
- **204 No Content**: Request successful, no content to return

### Client Error Codes
- **400 Bad Request**: Invalid request format or parameters
- **401 Unauthorized**: Authentication required or failed
- **403 Forbidden**: Access denied (valid auth but insufficient permissions)
- **404 Not Found**: Resource not found
- **409 Conflict**: Resource already exists or conflict
- **422 Unprocessable Entity**: Valid request but semantic errors
- **429 Too Many Requests**: Rate limit exceeded

### Server Error Codes
- **500 Internal Server Error**: Unexpected server error
- **502 Bad Gateway**: External service error
- **503 Service Unavailable**: Server temporarily unavailable

### Pierre-Specific Error Codes

| Code | Error Type | Description |
|------|------------|-------------|
| E_AUTH_001 | authentication_failed | Invalid API key or JWT token |
| E_AUTH_002 | insufficient_permissions | Valid auth but missing required permissions |
| E_RATE_001 | rate_limit_exceeded | API rate limit exceeded |
| E_OAUTH_001 | oauth_token_expired | OAuth token needs refresh |
| E_OAUTH_002 | oauth_token_invalid | OAuth token is invalid or revoked |
| E_TOOL_001 | tool_execution_failed | MCP/A2A tool execution error |
| E_TENANT_001 | tenant_not_found | Tenant configuration missing |
| E_USER_001 | user_not_approved | User account pending approval |

### Rate Limiting Details

Rate limits are applied per API key with different tiers:

| Tier | Requests/Minute | Burst Limit | Monthly Quota |
|------|-----------------|-------------|---------------|
| Trial | 60 | 100 | 10,000 |
| Basic | 300 | 500 | 50,000 |
| Premium | 1,000 | 2,000 | 500,000 |
| Enterprise | Unlimited | 5,000 | Unlimited |

When rate limited, the API returns HTTP 429 with:

```json
{
  "error": "rate_limit_exceeded",
  "message": "Rate limit of 60 requests per minute exceeded",
  "details": {
    "limit": 60,
    "remaining": 0,
    "reset_time": "2024-01-15T10:31:00Z",
    "retry_after": 45
  }
}
```

### WebSocket Error Codes

For WebSocket connections using MCP protocol:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32600,
    "message": "Invalid Request",
    "data": {
      "error_type": "protocol_error",
      "details": "Missing required field: method"
    }
  }
}
```

| JSON-RPC Code | Pierre Error | Description |
|---------------|--------------|-------------|
| -32700 | Parse Error | Invalid JSON |
| -32600 | Invalid Request | Malformed JSON-RPC request |
| -32601 | Method Not Found | Unknown method name |
| -32602 | Invalid Params | Invalid method parameters |
| -32603 | Internal Error | Server-side processing error |
| -32000 | Tool Error | MCP tool execution failed |
| -32001 | Auth Error | Authentication/authorization failed |

This API reference provides comprehensive documentation for integrating with Pierre MCP Server. For additional examples and SDKs, see the integration guides in the developer documentation.