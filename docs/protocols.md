# protocols

Pierre implements three protocols on a single http port (8081).

## mcp (model context protocol)

Json-rpc 2.0 protocol for ai assistant integration.

### endpoints

- `POST /mcp` - main mcp endpoint
- `GET /mcp/sse` - sse transport for streaming

### transport

Pierre supports both http and sse transports:
- http: traditional request-response
- sse: server-sent events for streaming responses

Sdk handles transport negotiation automatically.

### authentication

Mcp requests require jwt bearer token in authorization header:
```
Authorization: Bearer <jwt_token>
```

Obtained via oauth2 flow (sdk handles automatically).

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

Implementation: `src/mcp/protocol.rs`, `src/protocols/universal/`

## oauth2 authorization server

Rfc 7591 (dynamic client registration) + rfc 7636 (pkce) compliant oauth2 server for mcp client authentication.

### endpoints

- `GET /.well-known/oauth-authorization-server` - server metadata (rfc 8414)
- `POST /oauth2/register` - dynamic client registration
- `GET /oauth2/authorize` - authorization endpoint
- `POST /oauth2/token` - token endpoint
- `GET /oauth2/jwks` - json web key set
- `GET /.well-known/jwks.json` - jwks at standard oidc location
- `POST /oauth2/validate-and-refresh` - validate and refresh jwt tokens
- `POST /oauth2/token-validate` - validate jwt token

### registration flow

1. **client registration** (rfc 7591):
```bash
# local development (http allowed for localhost)
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "My MCP Client (Dev)",
    "grant_types": ["authorization_code"]
  }'

# production (https required)
curl -X POST https://api.example.com/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["https://client.example.com/oauth/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"]
  }'
```

Response:
```json
{
  "client_id": "generated_client_id",
  "client_secret": "generated_secret",
  "redirect_uris": ["http://localhost:35535/oauth/callback"],
  "grant_types": ["authorization_code"]
}
```

**callback url security**: redirect_uris using http only permitted for localhost/127.0.0.1 in development. Production clients must use https to protect authorization codes from interception.

2. **authorization request**:
```
GET /oauth2/authorize?
  client_id=<client_id>&
  redirect_uri=<redirect_uri>&
  response_type=code&
  code_challenge=<pkce_challenge>&
  code_challenge_method=S256
```

User authenticates in browser, redirected to:
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

Response:
```json
{
  "access_token": "jwt_token",
  "token_type": "Bearer",
  "expires_in": 86400
}
```

Jwt access token used for all mcp requests.

### pkce requirement

Pierre enforces pkce (rfc 7636) for all authorization code flows. Clients must:
- generate code verifier (43-128 characters)
- create code challenge: `base64url(sha256(verifier))`
- include challenge in authorization request
- include verifier in token request

### server discovery (rfc 8414)

Pierre provides oauth2 server metadata for automatic configuration:

```bash
curl http://localhost:8081/.well-known/oauth-authorization-server
```

Response includes:
```json
{
  "issuer": "http://localhost:8081",
  "authorization_endpoint": "http://localhost:8081/oauth2/authorize",
  "token_endpoint": "http://localhost:8081/oauth2/token",
  "jwks_uri": "http://localhost:8081/oauth2/jwks",
  "registration_endpoint": "http://localhost:8081/oauth2/register",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code"],
  "code_challenge_methods_supported": ["S256"]
}
```

Issuer url configurable via `OAUTH2_ISSUER_URL` environment variable.

### jwks endpoint

Public keys for jwt token verification available at `/oauth2/jwks`:

```bash
curl http://localhost:8081/oauth2/jwks
```

Response (rfc 7517 compliant):
```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "key_2024_01_01",
      "n": "modulus_base64url",
      "e": "exponent_base64url"
    }
  ]
}
```

**cache-control headers**: jwks endpoint returns `Cache-Control: public, max-age=3600` allowing browsers to cache public keys for 1 hour.

### key rotation

Pierre supports rs256 key rotation with grace period:
- new keys generated with timestamp-based kid (e.g., `key_2024_01_01_123456`)
- old keys retained during grace period for existing token validation
- tokens issued with old keys remain valid until expiration
- new tokens signed with current key

Clients should:
1. Fetch jwks on startup
2. Cache public keys for 1 hour (respects cache-control header)
3. Refresh jwks if unknown kid encountered
4. Verify token signature using matching kid

### rate limiting

Oauth2 endpoints protected by per-ip token bucket rate limiting:

| endpoint | requests per minute |
|----------|---------------------|
| `/oauth2/authorize` | 60 (1/second) |
| `/oauth2/token` | 30 (1/2 seconds) |
| `/oauth2/register` | 10 (1/6 seconds) |

Rate limit headers included in all responses:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
X-RateLimit-Reset: 1704067200
```

429 response when limit exceeded:
```json
{
  "error": "rate_limit_exceeded",
  "error_description": "Rate limit exceeded. Retry after 42 seconds."
}
```

Headers:
```
Retry-After: 42
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1704067200
```

Implementation: `src/oauth2_server/`, `src/oauth2_server/rate_limiting.rs`

## a2a (agent-to-agent protocol)

Protocol for autonomous ai systems to communicate.

### endpoints

- `GET /a2a/status` - protocol status
- `GET /a2a/tools` - available tools
- `POST /a2a/execute` - execute tool
- `GET /a2a/monitoring` - monitoring info

### authentication

A2a uses api keys:
```
X-API-Key: <api_key>
```

Create api key via admin endpoint:
```bash
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer <admin_jwt>" \
  -H "Content-Type: application/json" \
  -d '{"name": "My A2A System", "tier": "professional"}'
```

### agent cards

Agents advertise capabilities via agent cards:
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

Implementation: `src/a2a/`, `src/protocols/universal/`

## rest api

Traditional rest endpoints for web applications.

### authentication endpoints

- `POST /auth/register` - user registration
- `POST /auth/login` - user login
- `POST /auth/logout` - logout
- `POST /auth/refresh` - refresh jwt token

### provider oauth endpoints

- `GET /api/oauth/connect/{provider}` - initiate oauth (strava, garmin, fitbit)
- `GET /api/oauth/callback/{provider}` - oauth callback
- `GET /api/oauth/status` - connection status
- `POST /api/oauth/disconnect/{provider}` - disconnect provider

### admin endpoints

- `POST /admin/setup` - create admin user
- `POST /admin/users` - manage users
- `GET /admin/analytics` - usage analytics

### configuration endpoints

- `GET /api/configuration/catalog` - config catalog
- `GET /api/configuration/profiles` - available profiles
- `GET /api/configuration/user` - user config
- `PUT /api/configuration/user` - update config

Implementation: `src/routes.rs`, `src/admin_routes.rs`, `src/configuration_routes.rs`

## sse (server-sent events)

Real-time notifications for oauth completions and system events.

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

Implementation: `src/notifications/sse.rs`, `src/sse.rs`

## protocol comparison

| feature | mcp | oauth2 | a2a | rest |
|---------|-----|--------|-----|------|
| primary use | ai assistants | client auth | agent comms | web apps |
| auth method | jwt bearer | - | api key | jwt bearer |
| transport | http + sse | http | http | http |
| format | json-rpc 2.0 | oauth2 | json | json |
| implementation | `src/mcp/` | `src/oauth2_server/` | `src/a2a/` | `src/routes/` |

## choosing a protocol

- **ai assistant integration**: use mcp (claude, chatgpt)
- **web application**: use rest api
- **autonomous agents**: use a2a
- **client authentication**: use oauth2 (for mcp clients)

All protocols share the same business logic via `src/protocols/universal/`.
