# OAuth 2.0 Authorization Server

Pierre MCP Server implements a complete OAuth 2.0 Authorization Server compliant with RFC 7591 (Dynamic Client Registration) and RFC 8414 (Authorization Server Metadata). This allows MCP clients to authenticate users and obtain JWT access tokens for API access.

## Architecture Overview

The OAuth 2.0 Authorization Server consists of three main components:

1. **Client Registration Manager** (src/oauth2/client_registration.rs) - Handles RFC 7591 dynamic client registration
2. **Authorization Server** (src/oauth2/endpoints.rs) - Implements authorization and token endpoints
3. **Route Handlers** (src/oauth2/routes.rs) - HTTP endpoint mappings

All OAuth 2.0 endpoints are prefixed with `/oauth2/` and run on the same unified HTTP port (default 8081) as other protocols.

## Supported Grant Types

- `authorization_code` - Standard OAuth 2.0 authorization code flow
- `client_credentials` - Service-to-service authentication
- `refresh_token` - Token refresh without re-authentication

## Discovery Endpoint (RFC 8414)

### GET /.well-known/oauth-authorization-server

OAuth 2.0 Authorization Server Metadata endpoint for automatic client configuration (src/oauth2/routes.rs:47-69).

**Response** (200):
```json
{
  "issuer": "http://localhost:8081",
  "authorization_endpoint": "http://localhost:8081/oauth2/authorize",
  "token_endpoint": "http://localhost:8081/oauth2/token",
  "registration_endpoint": "http://localhost:8081/oauth2/register",
  "grant_types_supported": [
    "authorization_code",
    "client_credentials",
    "refresh_token"
  ],
  "response_types_supported": ["code"],
  "token_endpoint_auth_methods_supported": [
    "client_secret_post",
    "client_secret_basic"
  ],
  "scopes_supported": [
    "fitness:read",
    "activities:read",
    "profile:read"
  ],
  "response_modes_supported": ["query"],
  "code_challenge_methods_supported": ["S256", "plain"]
}
```

## Client Registration (RFC 7591)

### POST /oauth2/register

Dynamically register a new OAuth 2.0 client (src/oauth2/routes.rs:71-81, src/oauth2/client_registration.rs:27-93).

**Request**:
```json
{
  "redirect_uris": ["http://localhost:3000/callback"],
  "client_name": "My MCP Client",
  "client_uri": "https://example.com",
  "grant_types": ["authorization_code", "refresh_token"],
  "response_types": ["code"],
  "scope": "fitness:read activities:read"
}
```

**Response** (201):
```json
{
  "client_id": "oauth2_client_550e8400e29b41d4a716446655440000",
  "client_secret": "cs_1234567890abcdef1234567890abcdef",
  "client_id_issued_at": 1640995200,
  "client_secret_expires_at": 1672531200,
  "redirect_uris": ["http://localhost:3000/callback"],
  "grant_types": ["authorization_code", "refresh_token"],
  "response_types": ["code"],
  "client_name": "My MCP Client",
  "client_uri": "https://example.com",
  "scope": "fitness:read activities:read profile:read"
}
```

**Validation Rules** (src/oauth2/client_registration.rs:162-201):
- At least one redirect_uri required
- client_name required (max 255 characters)
- redirect_uris must be valid URLs
- grant_types must be from supported list
- response_types must be from supported list

**Client Credentials**:
- `client_id`: Generated using `oauth2_client_` prefix + UUID (src/oauth2/client_registration.rs:219-224)
- `client_secret`: Cryptographically secure random 32-byte value, base64-encoded (src/oauth2/client_registration.rs:226-233)
- `client_secret_hash`: SHA-256 hash stored in database (src/oauth2/client_registration.rs:235-238)
- Expires after 365 days by default (src/oauth2/client_registration.rs:56)

## Authorization Endpoint

### GET /oauth2/authorize

Initiate OAuth 2.0 authorization code flow (src/oauth2/routes.rs:83-114, src/oauth2/endpoints.rs:42-90).

**Query Parameters**:
```
?response_type=code
&client_id=oauth2_client_550e8400e29b41d4a716446655440000
&redirect_uri=http://localhost:3000/callback
&scope=fitness:read+activities:read
&state=random_state_value
&code_challenge=sha256_hash_of_verifier
&code_challenge_method=S256
```

**Parameters**:
- `response_type` - Must be "code" (required)
- `client_id` - Registered OAuth 2.0 client ID (required)
- `redirect_uri` - Must match registered redirect URI (required)
- `scope` - Space-separated list of requested scopes (optional)
- `state` - CSRF protection token (recommended)
- `code_challenge` - PKCE code challenge (optional, for public clients)
- `code_challenge_method` - "S256" or "plain" (required if code_challenge provided)

**Flow**:

1. **Client validation** (src/oauth2/endpoints.rs:52-56):
   - Verify client_id exists in database
   - Check client is not expired

2. **Redirect URI validation** (src/oauth2/endpoints.rs:66-68):
   - Verify redirect_uri matches registered URIs

3. **User authentication check** (src/oauth2/endpoints.rs:72-73):
   - If user not authenticated, redirect to `/oauth2/login?redirect_to=/oauth2/authorize&...`
   - User enters email/password on login page

4. **Authorization code generation** (src/oauth2/endpoints.rs:76-84):
   - Generate cryptographically secure authorization code
   - Store code with client_id, user_id, redirect_uri, scope, expiry (10 minutes)
   - Store PKCE code_challenge if provided

**Success Response** (302 Redirect):
```
HTTP/1.1 302 Found
Location: http://localhost:3000/callback?code=auth_code_abc123def456&state=random_state_value
```

**Error Response** (400):
```json
{
  "error": "invalid_client",
  "error_description": "Client not found or expired"
}
```

**Error Codes** (src/oauth2/models.rs):
- `invalid_request` - Missing or malformed parameters
- `unauthorized_client` - Client not authorized for this grant type
- `access_denied` - User denied authorization
- `unsupported_response_type` - response_type not "code"
- `invalid_scope` - Requested scope invalid or unknown
- `server_error` - Internal server error

## Token Endpoint

### POST /oauth2/token

Exchange authorization code for access token (src/oauth2/routes.rs:117-129, src/oauth2/endpoints.rs:92-158).

**Request** (application/x-www-form-urlencoded):
```
grant_type=authorization_code
&code=auth_code_abc123def456
&redirect_uri=http://localhost:3000/callback
&client_id=oauth2_client_550e8400e29b41d4a716446655440000
&client_secret=cs_1234567890abcdef1234567890abcdef
&code_verifier=plain_text_verifier_for_pkce
```

**Parameters**:
- `grant_type` - "authorization_code", "client_credentials", or "refresh_token" (required)
- `code` - Authorization code from /oauth2/authorize (required for authorization_code grant)
- `redirect_uri` - Must match authorization request (required for authorization_code grant)
- `client_id` - OAuth 2.0 client ID (required)
- `client_secret` - OAuth 2.0 client secret (required)
- `code_verifier` - PKCE code verifier (required if code_challenge was provided)
- `refresh_token` - Refresh token (required for refresh_token grant)
- `scope` - Requested scope (optional for client_credentials grant)

**Response** (200):
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "refresh_token": "refresh_abc123def456ghi789",
  "scope": "fitness:read activities:read"
}
```

**Access Token Format**:

Pierre issues JWT tokens as access tokens for MCP compatibility (src/oauth2/endpoints.rs:130-145):

```json
{
  "sub": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@example.com",
  "iat": 1640995200,
  "exp": 1641081600,
  "providers": ["strava", "fitbit"],
  "client_id": "oauth2_client_550e8400e29b41d4a716446655440000",
  "scope": "fitness:read activities:read"
}
```

**Token Validation** (src/oauth2/endpoints.rs:96-112):

1. **Client authentication**:
   - Verify client_id and client_secret (SHA-256 hash comparison)
   - Check client not expired

2. **Authorization code validation** (for authorization_code grant):
   - Verify code exists and not expired (10 minute TTL)
   - Verify code not already used (prevents replay attacks)
   - Verify redirect_uri matches authorization request
   - Verify client_id matches authorization request

3. **PKCE validation** (if code_challenge was provided):
   - Verify code_verifier provided
   - Compute SHA-256(code_verifier)
   - Compare with stored code_challenge
   - Reject if mismatch

4. **JWT generation**:
   - Extract user_id from authorization code
   - Generate JWT with user claims + OAuth metadata
   - Set expiry to 24 hours (configurable via JWT_EXPIRY_HOURS)

**Refresh Token Flow**:

For `grant_type=refresh_token`:
```
grant_type=refresh_token
&refresh_token=refresh_abc123def456ghi789
&client_id=oauth2_client_550e8400e29b41d4a716446655440000
&client_secret=cs_1234567890abcdef1234567890abcdef
```

Response includes new access_token and optionally new refresh_token.

## JWKS Endpoint

### GET /oauth2/jwks

JSON Web Key Set endpoint for JWT token verification (src/oauth2/routes.rs).

**Response** (200):
```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "pierre-2024-01",
      "n": "base64_encoded_modulus...",
      "e": "AQAB"
    }
  ]
}
```

MCP clients can use this endpoint to verify JWT signatures without storing shared secrets.

## Login Flow

### GET /oauth2/login

OAuth 2.0 login page for user authentication (src/oauth2/routes.rs:99-103).

**Query Parameters**:
- `redirect_to` - URL to redirect after successful login
- Original authorize parameters preserved in query string

**Response**: HTML login form with email/password fields

### POST /oauth2/login

Process login form submission (src/oauth2/routes.rs:106-112).

**Form Data** (application/x-www-form-urlencoded):
```
email=user@example.com
&password=securepassword123
&redirect_to=/oauth2/authorize?response_type=code&client_id=...
```

**Success**: Sets session cookie and redirects to `redirect_to` URL

**Error**: Returns to login page with error message

## Security Considerations

### Client Secret Storage

Client secrets are NEVER stored in plaintext. The server stores SHA-256 hashes (src/oauth2/client_registration.rs:235-238):

```rust
pub fn hash_client_secret(secret: &str) -> String {
    let digest = digest(&SHA256, secret.as_bytes());
    general_purpose::STANDARD.encode(digest.as_ref())
}
```

### Authorization Code Security

Authorization codes (src/oauth2/endpoints.rs:160-193):
- Expire after 10 minutes
- One-time use only (marked as used after redemption)
- Bound to specific client_id and redirect_uri
- Support PKCE for public clients (prevents authorization code interception)

### PKCE (Proof Key for Code Exchange)

For public clients (mobile apps, SPAs) that cannot securely store client_secret:

1. Client generates random `code_verifier` (43-128 characters)
2. Client computes `code_challenge = SHA256(code_verifier)`
3. Authorization request includes `code_challenge` and `code_challenge_method=S256`
4. Token request includes `code_verifier`
5. Server validates `SHA256(code_verifier) == code_challenge`

Implementation: src/oauth2/endpoints.rs:114-125

### State Parameter

The `state` parameter prevents CSRF attacks:
- Client generates random state value
- Includes in authorization request
- Server returns state in redirect
- Client validates state matches original value

### Scope Validation

Scopes control access to specific API resources (src/oauth2/endpoints.rs:148-154):

- `fitness:read` - Read fitness data
- `activities:read` - Read activity data
- `profile:read` - Read user profile
- `activities:write` - Create/modify activities
- `goals:read` - Read fitness goals
- `goals:write` - Create/modify goals

Tokens are restricted to requested and approved scopes.

## Integration Examples

### MCP Client Registration and Authentication

```bash
# Step 1: Register OAuth 2.0 client
CLIENT_RESPONSE=$(curl -s -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:3000/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code", "refresh_token"],
    "response_types": ["code"],
    "scope": "fitness:read activities:read"
  }')

CLIENT_ID=$(echo $CLIENT_RESPONSE | jq -r '.client_id')
CLIENT_SECRET=$(echo $CLIENT_RESPONSE | jq -r '.client_secret')

echo "Client ID: $CLIENT_ID"
echo "Client Secret: $CLIENT_SECRET"

# Step 2: Generate PKCE code verifier and challenge (optional, for public clients)
CODE_VERIFIER=$(openssl rand -base64 32 | tr -d "=+/" | cut -c1-43)
CODE_CHALLENGE=$(echo -n $CODE_VERIFIER | openssl dgst -sha256 -binary | base64 | tr -d "=+/" | tr '/+' '_-')

# Step 3: Generate authorization URL
STATE=$(openssl rand -hex 16)
AUTH_URL="http://localhost:8081/oauth2/authorize?response_type=code&client_id=$CLIENT_ID&redirect_uri=http://localhost:3000/callback&scope=fitness:read+activities:read&state=$STATE&code_challenge=$CODE_CHALLENGE&code_challenge_method=S256"

echo "Authorization URL: $AUTH_URL"
echo "Visit this URL in browser to authorize"

# Step 4: After user authorization, extract code from callback
# Callback will be: http://localhost:3000/callback?code=AUTH_CODE&state=STATE
# Verify STATE matches

# Step 5: Exchange authorization code for access token
TOKEN_RESPONSE=$(curl -s -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code&code=$AUTH_CODE&redirect_uri=http://localhost:3000/callback&client_id=$CLIENT_ID&client_secret=$CLIENT_SECRET&code_verifier=$CODE_VERIFIER")

ACCESS_TOKEN=$(echo $TOKEN_RESPONSE | jq -r '.access_token')
REFRESH_TOKEN=$(echo $TOKEN_RESPONSE | jq -r '.refresh_token')

echo "Access Token: $ACCESS_TOKEN"

# Step 6: Use access token to call MCP tools
curl -X POST http://localhost:8081/mcp \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "get_activities",
      "arguments": {"limit": 5}
    },
    "id": 1
  }'

# Step 7: Refresh token when access token expires
REFRESH_RESPONSE=$(curl -s -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token&refresh_token=$REFRESH_TOKEN&client_id=$CLIENT_ID&client_secret=$CLIENT_SECRET")

NEW_ACCESS_TOKEN=$(echo $REFRESH_RESPONSE | jq -r '.access_token')
```

### Service-to-Service Authentication (Client Credentials)

For automated systems that don't require user authorization:

```bash
# Register client (same as above)

# Request access token with client_credentials grant
TOKEN_RESPONSE=$(curl -s -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=client_credentials&client_id=$CLIENT_ID&client_secret=$CLIENT_SECRET&scope=fitness:read")

ACCESS_TOKEN=$(echo $TOKEN_RESPONSE | jq -r '.access_token')

# Use access token (token represents client, not user)
curl -X POST http://localhost:8081/mcp \
  -H "Authorization: Bearer $ACCESS_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "tools/list", "id": 1}'
```

## MCP Client Configuration

Configure MCP clients to use OAuth 2.0 flow:

**Claude Desktop (claude_desktop_config.json)**:
```json
{
  "mcpServers": {
    "pierre-fitness": {
      "url": "http://127.0.0.1:8081/mcp",
      "oauth": {
        "enabled": true,
        "discovery_url": "http://127.0.0.1:8081/.well-known/oauth-authorization-server",
        "client_name": "Claude Desktop",
        "redirect_uri": "http://localhost:35535/oauth/callback",
        "scopes": ["fitness:read", "activities:read", "profile:read"]
      }
    }
  }
}
```

The MCP client will:
1. Fetch OAuth 2.0 configuration from discovery endpoint
2. Dynamically register via /oauth2/register
3. Open browser for user authorization
4. Exchange code for access token
5. Use access token for all MCP requests
6. Refresh token when expired

## Database Schema

OAuth 2.0 data is stored in the following tables:

**oauth2_clients** (src/database_plugins/sqlite.rs):
```sql
CREATE TABLE oauth2_clients (
    id TEXT PRIMARY KEY,
    client_id TEXT UNIQUE NOT NULL,
    client_secret_hash TEXT NOT NULL,
    redirect_uris TEXT NOT NULL,  -- JSON array
    grant_types TEXT NOT NULL,    -- JSON array
    response_types TEXT NOT NULL, -- JSON array
    client_name TEXT NOT NULL,
    client_uri TEXT,
    scope TEXT,
    created_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP
);
```

**oauth2_authorization_codes**:
```sql
CREATE TABLE oauth2_authorization_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT,
    code_challenge TEXT,
    code_challenge_method TEXT,
    expires_at TIMESTAMP NOT NULL,
    used BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP NOT NULL
);
```

**oauth2_tokens**:
```sql
CREATE TABLE oauth2_tokens (
    id TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    access_token_hash TEXT NOT NULL,
    refresh_token_hash TEXT,
    scope TEXT,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL
);
```

## Troubleshooting

### Invalid Client Error

**Symptom**: `{"error": "invalid_client"}`

**Causes**:
- client_id not found in database
- client_secret incorrect (hash mismatch)
- Client registration expired

**Solution**:
- Verify client_id and client_secret from registration response
- Check client expiry: `SELECT expires_at FROM oauth2_clients WHERE client_id = 'your_client_id';`
- Re-register client if expired

### Invalid Grant Error

**Symptom**: `{"error": "invalid_grant"}`

**Causes**:
- Authorization code expired (>10 minutes old)
- Authorization code already used
- redirect_uri mismatch between authorize and token requests
- PKCE code_verifier invalid

**Solution**:
- Ensure token exchange happens within 10 minutes of authorization
- Don't retry failed token requests with same code
- Verify redirect_uri exactly matches (including trailing slashes)
- For PKCE, verify code_verifier generates matching code_challenge

### Access Denied Error

**Symptom**: Redirect to `?error=access_denied`

**Causes**:
- User clicked "Deny" on consent screen
- User account suspended or pending approval

**Solution**:
- User must authorize the application
- Check user account status in database

### PKCE Validation Failure

**Symptom**: `{"error": "invalid_grant", "error_description": "PKCE validation failed"}`

**Causes**:
- code_verifier not provided when code_challenge was sent
- SHA256(code_verifier) != code_challenge
- code_challenge_method not "S256"

**Solution**:
- Store code_verifier securely on client
- Verify SHA-256 hash: `echo -n "verifier" | openssl dgst -sha256 -binary | base64 | tr -d "=+/" | tr '/+' '_-'`
- Ensure code_challenge_method is "S256" (plain is supported but not recommended)

## Performance Considerations

### Database Indexing

Ensure indexes exist on frequently queried columns:
```sql
CREATE INDEX idx_oauth2_clients_client_id ON oauth2_clients(client_id);
CREATE INDEX idx_oauth2_auth_codes_code ON oauth2_authorization_codes(code);
CREATE INDEX idx_oauth2_auth_codes_expires_at ON oauth2_authorization_codes(expires_at);
CREATE INDEX idx_oauth2_tokens_client_user ON oauth2_tokens(client_id, user_id);
```

### Authorization Code Cleanup

Expired authorization codes should be periodically cleaned:
```sql
DELETE FROM oauth2_authorization_codes WHERE expires_at < datetime('now');
```

Consider running this as a cron job or scheduled task.

### Token Caching

MCP clients should cache access tokens and only refresh when expired (check `expires_in` from token response).

## Standards Compliance

- **RFC 6749** - OAuth 2.0 Authorization Framework
- **RFC 7591** - OAuth 2.0 Dynamic Client Registration Protocol
- **RFC 8414** - OAuth 2.0 Authorization Server Metadata
- **RFC 7636** - Proof Key for Code Exchange (PKCE)
- **RFC 7519** - JSON Web Token (JWT) for access tokens

## Related Documentation

- [Authentication Guide](06-authentication.md) - JWT authentication and claims structure
- [API Reference](14-api-reference.md) - Complete REST API documentation
- [MCP Protocol](04-mcp-protocol.md) - MCP over OAuth 2.0
- [Security Guide](17-security-guide.md) - Security best practices
