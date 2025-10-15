# protocols

pierre implements three protocols on a single http port (8081).

## mcp (model context protocol)

json-rpc 2.0 protocol for ai assistant integration.

### endpoints

- `POST /mcp` - main mcp endpoint
- `GET /mcp/sse` - sse transport for streaming

### transport

pierre supports both http and sse transports:
- http: traditional request-response
- sse: server-sent events for streaming responses

sdk handles transport negotiation automatically.

### authentication

mcp requests require jwt bearer token in authorization header:
```
Authorization: Bearer <jwt_token>
```

obtained via oauth2 flow (sdk handles automatically).

### request format

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "limit": 5
    }
  }
}
```

### response format

```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "[activity data...]"
      }
    ]
  }
}
```

### mcp methods

- `initialize` - start session
- `tools/list` - list available tools
- `tools/call` - execute tool
- `resources/list` - list resources
- `prompts/list` - list prompts

implementation: `src/mcp/protocol.rs`, `src/protocols/universal/`

## oauth2 authorization server

rfc 7591 (dynamic client registration) + rfc 7636 (pkce) compliant oauth2 server for mcp client authentication.

### endpoints

- `GET /.well-known/oauth-authorization-server` - server metadata (rfc 8414)
- `POST /oauth2/register` - dynamic client registration
- `GET /oauth2/authorize` - authorization endpoint
- `POST /oauth2/token` - token endpoint
- `GET /oauth2/jwks` - json web key set

### registration flow

1. **client registration** (rfc 7591):
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"]
  }'
```

response:
```json
{
  "client_id": "generated_client_id",
  "client_secret": "generated_secret",
  "redirect_uris": ["http://localhost:35535/oauth/callback"],
  "grant_types": ["authorization_code"]
}
```

2. **authorization request**:
```
GET /oauth2/authorize?
  client_id=<client_id>&
  redirect_uri=<redirect_uri>&
  response_type=code&
  code_challenge=<pkce_challenge>&
  code_challenge_method=S256
```

user authenticates in browser, redirected to:
```
<redirect_uri>?code=<authorization_code>
```

3. **token exchange**:
```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&\
      code=<authorization_code>&\
      client_id=<client_id>&\
      client_secret=<client_secret>&\
      redirect_uri=<redirect_uri>&\
      code_verifier=<pkce_verifier>"
```

response:
```json
{
  "access_token": "jwt_token",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

jwt access token used for all mcp requests.

### pkce requirement

pierre enforces pkce (rfc 7636) for all authorization code flows. clients must:
- generate code verifier (43-128 characters)
- create code challenge: `base64url(sha256(verifier))`
- include challenge in authorization request
- include verifier in token request

implementation: `src/oauth2/`

## a2a (agent-to-agent protocol)

protocol for autonomous ai systems to communicate.

### endpoints

- `GET /a2a/status` - protocol status
- `GET /a2a/tools` - available tools
- `POST /a2a/execute` - execute tool
- `GET /a2a/monitoring` - monitoring info

### authentication

a2a uses api keys:
```
X-API-Key: <api_key>
```

create api key via admin endpoint:
```bash
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer <admin_jwt>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My A2A System", "tier": "professional"}'
```

### agent cards

agents advertise capabilities via agent cards:
```json
{
  "agent_id": "fitness-analyzer",
  "name": "Fitness Analyzer Agent",
  "version": "1.0.0",
  "capabilities": [
    "activity_analysis",
    "performance_prediction",
    "goal_tracking"
  ],
  "endpoints": [
    {
      "path": "/a2a/execute",
      "method": "POST",
      "description": "Execute fitness analysis"
    }
  ]
}
```

### request format

```json
{
  "tool": "analyze_activity",
  "parameters": {
    "activity_id": "12345",
    "analysis_type": "comprehensive"
  }
}
```

### response format

```json
{
  "success": true,
  "result": {
    "analysis": {...},
    "recommendations": [...]
  }
}
```

implementation: `src/a2a/`, `src/protocols/universal/`

## rest api

traditional rest endpoints for web applications.

### authentication endpoints

- `POST /auth/register` - user registration
- `POST /auth/login` - user login
- `POST /auth/logout` - logout
- `POST /auth/refresh` - refresh jwt token

### provider oauth endpoints

- `GET /oauth/connect/{provider}` - initiate oauth (strava, garmin, fitbit)
- `GET /oauth/callback/{provider}` - oauth callback
- `GET /oauth/status` - connection status
- `POST /oauth/disconnect/{provider}` - disconnect provider

### admin endpoints

- `POST /admin/setup` - create admin user
- `POST /admin/users` - manage users
- `GET /admin/analytics` - usage analytics

### configuration endpoints

- `GET /api/configuration/catalog` - config catalog
- `GET /api/configuration/profiles` - available profiles
- `GET /api/configuration/user` - user config
- `PUT /api/configuration/user` - update config

implementation: `src/routes.rs`, `src/admin_routes.rs`, `src/configuration_routes.rs`

## sse (server-sent events)

real-time notifications for oauth completions and system events.

### endpoint

```
GET /notifications/sse?user_id=<user_id>
```

### event types

- `oauth_complete` - oauth flow completed
- `oauth_error` - oauth flow failed
- `system_status` - system status update

### example

```javascript
const eventSource = new EventSource('/notifications/sse?user_id=user-123');

eventSource.onmessage = function(event) {
  const notification = JSON.parse(event.data);
  if (notification.type === 'oauth_complete') {
    console.log('OAuth completed for provider:', notification.provider);
  }
};
```

implementation: `src/notifications/sse.rs`, `src/sse.rs`

## protocol comparison

| feature | mcp | oauth2 | a2a | rest |
|---------|-----|--------|-----|------|
| primary use | ai assistants | client auth | agent comms | web apps |
| auth method | jwt bearer | - | api key | jwt bearer |
| transport | http + sse | http | http | http |
| format | json-rpc 2.0 | oauth2 | json | json |
| implementation | `src/mcp/` | `src/oauth2/` | `src/a2a/` | `src/routes.rs` |

## choosing a protocol

- **ai assistant integration**: use mcp (claude, chatgpt)
- **web application**: use rest api
- **autonomous agents**: use a2a
- **client authentication**: use oauth2 (for mcp clients)

all protocols share the same business logic via `src/protocols/universal/`.
