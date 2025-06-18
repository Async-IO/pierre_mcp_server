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

## API Key System

### Creating API Keys

```bash
# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "your@email.com", "password": "your_password"}'

# Create a production API key
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Production Key",
    "description": "Main production API key",
    "tier": "Professional"
  }'

# Create a trial key (14-day expiration)
curl -X POST http://localhost:8081/api/keys/trial \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Trial Key",
    "description": "Testing the platform"
  }'
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
2. Login and get JWT token
3. Create a trial API key
4. Test rate limiting
5. Verify one-trial-per-user enforcement

### Manual Testing

```bash
# Register a test user
curl -X POST http://localhost:8081/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "secure_password",
    "display_name": "Test User"
  }'

# Login to get JWT token
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "test@example.com",
    "password": "secure_password"
  }'

# Create a trial API key
curl -X POST http://localhost:8081/api/keys/trial \
  -H "Authorization: Bearer YOUR_JWT_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test Trial Key",
    "description": "Testing the platform features"
  }'

# Test API key usage
curl -X POST http://localhost:8081/a2a/execute \
  -H "Authorization: Bearer YOUR_API_KEY" \
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