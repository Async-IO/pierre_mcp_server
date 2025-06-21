# API Reference

This document covers the B2B API platform features, API key management, and authentication systems.

## B2B API Platform Features

### 🔑 API Key Management
- **Tiered Access**: Trial (1K/month), Starter (10K/month), Professional (100K/month), Enterprise (Unlimited)
- **Trial Keys**: 14-day auto-expiring trial keys with one-per-user limit
- **Rate Limiting**: Automatic monthly rate limiting with real-time tracking
- **Usage Analytics**: Detailed usage statistics per tool and time period
- **Secure Storage**: SHA-256 hashed keys with prefix-based identification

### 📊 Developer Dashboard
- **Real-time Monitoring**: WebSocket-based live updates
- **Usage Analytics**: Tool-specific usage breakdown and trends
- **Rate Limit Status**: Visual indicators and warnings
- **API Key Management**: Create, list, and deactivate keys

### 🔐 Enterprise Security
- **JWT Authentication**: 24-hour tokens with detailed error messages
- **API Key Authentication**: Production (`pk_live_`) and trial (`pk_trial_`) keys
- **Encrypted Storage**: AES-256-GCM for OAuth tokens at rest
- **CORS Support**: Full cross-origin resource sharing configuration
- **User Isolation**: Complete data separation between tenants

## Admin API Endpoints

### Complete Admin REST API

| Endpoint | Method | Description | Required Permission |
|----------|--------|-------------|-------------------|
| `/admin/provision-api-key` | POST | Create new API key for user | ProvisionKeys |
| `/admin/revoke-api-key` | POST | Revoke existing API key | RevokeKeys |
| `/admin/list-api-keys` | GET | List API keys with filters | ListKeys |
| `/admin/token-info` | GET | Get admin token information | ManageAdminTokens |
| `/admin/setup-status` | GET | Check if admin setup required | None (public) |
| `/admin/health` | GET | Admin service health check | None (public) |

### Admin Authentication

All admin endpoints (except `setup-status` and `health`) require an admin JWT token:

```bash
Authorization: Bearer <admin_jwt_token>
```

Admin tokens are generated using the `admin-setup` binary and include specific permissions.

### List API Keys Endpoint

Get filtered list of API keys with pagination:

```bash
GET /admin/list-api-keys?user_email=user@example.com&active_only=true&limit=50&offset=0
```

**Query Parameters:**
- `user_email` (optional): Filter by user email
- `active_only` (optional): Only return active keys (default: true)
- `limit` (optional): Number of keys to return (1-100, default: unset)
- `offset` (optional): Number of keys to skip (default: 0)

**Response:**
```json
{
  "success": true,
  "message": "Found 3 API keys",
  "data": {
    "filters": {
      "user_email": "user@example.com",
      "active_only": true,
      "limit": 50,
      "offset": 0
    },
    "keys": [
      {
        "id": "key_abc123",
        "user_id": "user_def456",
        "name": "Production Key",
        "description": "Main API key",
        "tier": "professional",
        "rate_limit": {
          "requests": 100000,
          "window": 2592000
        },
        "is_active": true,
        "created_at": "2024-01-15T10:30:00Z",
        "last_used_at": "2024-06-20T14:22:00Z",
        "expires_at": "2025-01-15T10:30:00Z",
        "usage_count": 0
      }
    ],
    "count": 3
  }
}
```

### Token Info Endpoint

Get detailed information about the current admin token:

```bash
GET /admin/token-info
Authorization: Bearer <admin_jwt_token>
```

**Response:**
```json
{
  "token_id": "admin_token_123",
  "service_name": "enterprise_provisioner",
  "permissions": ["provision_keys", "revoke_keys", "list_keys"],
  "is_super_admin": false,
  "created_at": "2024-01-01T00:00:00Z",
  "last_used_at": "2024-06-20T15:30:00Z",
  "usage_count": 42
}
```

### Setup Status Endpoint

Check if admin setup is required (public endpoint):

```bash
GET /admin/setup-status
```

**Response:**
```json
{
  "needs_setup": false,
  "admin_user_exists": true,
  "message": "System is properly configured"
}
```

### Admin Health Check

Check admin service health (public endpoint):

```bash
GET /admin/health
```

**Response:**
```json
{
  "status": "healthy",
  "service": "pierre-mcp-admin-api",
  "timestamp": "2024-06-20T15:45:00Z",
  "version": "0.1.0"
}
```

### Error Response Formats

All admin API endpoints return standardized error responses:

**Standard Error Response:**
```json
{
  "success": false,
  "message": "Detailed error description",
  "data": null
}
```

**Common HTTP Status Codes:**
- `400 Bad Request`: Invalid request parameters or JSON body
- `401 Unauthorized`: Missing or invalid admin token
- `403 Forbidden`: Admin token lacks required permissions
- `404 Not Found`: Requested resource (API key, user) not found
- `500 Internal Server Error`: Database or system error

**Authentication Error Example:**
```json
{
  "success": false,
  "message": "Token expired at 2024-06-20T10:00:00Z, current time is 2024-06-20T15:30:00Z",
  "data": null
}
```

**Rate Limiting Error Example:**
```json
{
  "success": false,
  "message": "Rate limit exceeded. Monthly limit: 100000, current usage: 100000. Resets: 2024-07-01T00:00:00Z",
  "data": {
    "rate_limit": {
      "limit": 100000,
      "current": 100000,
      "reset_date": "2024-07-01T00:00:00Z",
      "tier": "professional"
    }
  }
}
```

## API Key System

### Creating API Keys (Enterprise Model)

Pierre uses an enterprise API key provisioning model where only administrators can create API keys for users. This ensures proper access control and compliance with enterprise security requirements.

```bash
# Step 1: User registers and logs in (standard flow)
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "your@email.com", "password": "your_password"}'

# Step 2: Administrator provisions API key for the user
# Only administrators with ProvisionKeys permission can create API keys
curl -X POST http://localhost:8081/admin/provision-api-key \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user_email": "user@company.com",
    "tier": "professional", 
    "rate_limit_requests": 100000,
    "rate_limit_period": "month",
    "expires_in_days": 365
  }'

# Step 3: User receives API key through secure channel
# API keys are distributed through email or admin dashboard
```

### Using API Keys with MCP

```json
{
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "provider": "strava",
      "limit": 10
    }
  },
  "auth": "pk_live_your_api_key_here"
}
```

### Rate Limiting Systems

| Tier | Monthly Limit | Key Prefix | Expiration |
|------|--------------|------------|------------|
| Trial | 1,000 requests | `pk_trial_` | 14 days |
| Starter | 10,000 requests | `pk_live_` | None |
| Professional | 100,000 requests | `pk_live_` | None |
| Enterprise | Unlimited | `pk_live_` | None |

## Testing the System

### Quick Start Script

```bash
# Test the trial key system
./scripts/test_trial_keys.sh
```

This script will:
1. Register a test user
2. Setup admin authentication
3. Provision a trial API key via admin endpoint
4. Test rate limiting
5. Verify enterprise provisioning workflow

### Manual Testing (Enterprise Model)

```bash
# Step 1: Register a test user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "secure_password",
    "display_name": "Test User"
  }'

# Step 2: Admin provisions API key for the test user
# Note: This requires admin credentials with ProvisionKeys permission
curl -X POST http://localhost:8081/admin/provision-api-key \
  -H "Authorization: Bearer ADMIN_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "user_email": "test@example.com",
    "tier": "trial",
    "rate_limit_requests": 1000,
    "rate_limit_period": "month",
    "expires_in_days": 14,
    "name": "Test Trial Key",
    "description": "Testing the platform features"
  }'

# Step 3: Test API key usage (user receives key from admin)
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer PROVISIONED_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "id": 1,
    "params": {
      "tool_name": "get_connection_status",
      "parameters": {}
    }
  }'
```

## Real-World Connection Flows

### 🤖 AI Assistant Integration (MCP Protocol)

**For users connecting AI assistants like Claude Desktop or GitHub Copilot:**

#### Complete User Journey

1. **Account Setup**
   ```bash
   # User registers on your Pierre instance
   curl -X POST https://your-pierre-server.com/auth/register \
     -H "Content-Type: application/json" \
     -d '{
       "email": "user@example.com",
       "password": "secure_password",
       "display_name": "John Runner"
     }'
   ```

2. **Strava Connection**
   ```bash
   # User connects their Strava account via OAuth
   curl -X GET "https://your-pierre-server.com/oauth/auth/strava/{user_id}" \
     -H "Authorization: Bearer USER_JWT_TOKEN"
   # Returns: Real Strava OAuth URL for browser authorization
   ```

3. **AI Assistant Configuration**
   ```json
   // ~/.claude/claude_desktop_config.json
   {
     "mcpServers": {
       "pierre-fitness": {
         "command": "mcp-client",
         "args": ["--server", "wss://your-pierre-server.com:8080"],
         "env": {
           "JWT_TOKEN": "your_jwt_token_here"
         }
       }
     }
   }
   ```

4. **Natural Language Queries**
   ```
   User → Claude: "What was my longest run this year and where did I run it?"
   Claude → Pierre: get_activities + get_activity_intelligence
   Pierre → Strava: Fetch real activity data with location
   Claude → User: "Your longest run was 21.5km in Saint-Hippolyte, Québec on March 15th..."
   ```

### 🔗 Developer Integration (A2A Protocol)

**For applications and AI agents connecting programmatically:**

#### A2A Client Registration
```bash
# Developer registers their application
curl -X POST https://your-pierre-server.com/a2a/clients \
  -H "Authorization: Bearer USER_JWT_TOKEN" \
  -d '{
    "name": "FitnessCoach AI",
    "description": "AI-powered fitness coaching application",
    "capabilities": ["fitness-analysis", "goal-tracking", "performance-prediction"],
    "redirect_uris": ["https://myapp.com/oauth/callback"],
    "contact_email": "developer@myapp.com"
  }'
# Returns: client_id, client_secret, api_key
```

#### Real-Time Data Access
```bash
# Application accesses user's fitness data
curl -X POST https://your-pierre-server.com/a2a/execute \
  -H "Authorization: Bearer A2A_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "id": 1,
    "params": {
      "tool_name": "get_activities",
      "parameters": {
        "provider": "strava",
        "limit": 20
      }
    }
  }'
```

### 🔄 Real OAuth Flow Example

**Complete end-to-end example with actual Strava data:**

```bash
# 1. User registration
curl -X POST http://localhost:8081/auth/register \
  -d '{"email": "athlete@example.com", "password": "secure123"}'
# Returns: {"user_id": "ca33ad77-728b-4e6d-83c5-d2878a69a9dc"}

# 2. Login for JWT token
curl -X POST http://localhost:8081/auth/login \
  -d '{"email": "athlete@example.com", "password": "secure123"}'
# Returns: {"jwt_token": "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."}

# 3. Generate Strava OAuth URL
curl -X GET "http://localhost:8081/oauth/auth/strava/ca33ad77-728b-4e6d-83c5-d2878a69a9dc" \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..."
# Returns: {"authorization_url": "https://www.strava.com/oauth/authorize?client_id=163846&..."}

# 4. User visits URL in browser → Strava authorization → automatic callback processing

# 5. Access real fitness data
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9..." \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools.execute",
    "params": {
      "tool_name": "get_athlete",
      "parameters": {"provider": "strava"}
    }
  }'
# Returns: Real Strava athlete data with profile, stats, and activity access
```

## Usage Analytics & Monitoring

Pierre provides comprehensive analytics for both users and developers:

```bash
# User dashboard metrics
curl -X GET http://localhost:8081/api/usage/summary \
  -H "Authorization: Bearer USER_JWT_TOKEN"

# A2A client analytics
curl -X GET http://localhost:8081/a2a/usage \
  -H "Authorization: Bearer A2A_API_KEY"

# Real-time health monitoring
curl -X GET http://localhost:8081/health
```

## Security Features

- **Encryption at Rest**: All OAuth tokens encrypted with AES-256-GCM
- **JWT Authentication**: Stateless authentication with configurable expiry
- **User Isolation**: Complete data separation between users
- **Secure Defaults**: Encryption keys auto-generated if not provided
- **No Shared State**: Each user's data completely isolated