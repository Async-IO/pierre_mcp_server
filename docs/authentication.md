<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Authentication

Pierre supports multiple authentication methods for different use cases.

## Authentication Methods

| method | use case | header | endpoints |
|--------|----------|--------|-----------|
| jwt tokens | mcp clients, web apps | `Authorization: Bearer <token>` | all authenticated endpoints |
| api keys | a2a systems | `X-API-Key: <key>` | a2a endpoints |
| oauth2 | provider integration | varies | fitness provider apis |

## JWT Authentication

### Registration

```bash
curl -X POST http://localhost:8081/api/auth/register \
  -H "Authorization: Bearer <admin_jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!",
    "display_name": "User Name"
  }'
```

Response:
```json
{
  "user_id": "uuid",
  "email": "user@example.com",
  "token": "jwt_token",
  "expires_at": "2024-01-01T00:00:00Z"
}
```

### Login

Uses OAuth2 Resource Owner Password Credentials (ROPC) flow:

```bash
curl -X POST http://localhost:8081/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=user@example.com&password=SecurePass123!"
```

Response includes jwt_token. Store securely.

### Using JWT Tokens

Include in authorization header:
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://localhost:8081/mcp
```

### Token Expiry

Default: 24 hours (configurable via `JWT_EXPIRY_HOURS`)

Refresh before expiry:
```bash
curl -X POST http://localhost:8081/api/auth/refresh \
  -H "Authorization: Bearer <current_token>"
```

## API Key Authentication

For a2a systems and service-to-service communication.

### Creating API Keys

Requires admin or user jwt:
```bash
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer <jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My A2A System",
    "tier": "professional"
  }'
```

Response:
```json
{
  "api_key": "generated_key",
  "name": "My A2A System",
  "tier": "professional",
  "created_at": "2024-01-01T00:00:00Z"
}
```

Save api key - cannot be retrieved later.

### Using API Keys

```bash
curl -H "X-API-Key: <api_key>" \
  http://localhost:8081/a2a/tools
```

### API Key Tiers

- `trial`: 1,000 requests/month (auto-expires after 14 days)
- `starter`: 10,000 requests/month
- `professional`: 100,000 requests/month
- `enterprise`: unlimited (no fixed monthly cap)

Rate limits are enforced per API key over a rolling 30-day window.

## OAuth2 (MCP Client Authentication)

Pierre acts as oauth2 authorization server for mcp clients.

### OAuth2 vs OAuth (Terminology)

Pierre implements two oauth systems:

1. **oauth2_server module** (`src/oauth2_server/`): pierre AS oauth2 server
   - mcp clients authenticate TO pierre
   - rfc 7591 dynamic client registration
   - issues jwt access tokens

2. **oauth2_client module** (`src/oauth2_client/`): pierre AS oauth2 client
   - pierre authenticates TO fitness providers (strava, garmin, fitbit, whoop)
   - manages provider tokens
   - handles token refresh

### OAuth2 Flow (MCP Clients)

Sdk handles automatically. Manual flow:

1. **register client**:
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:35535/oauth/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"]
  }'
```

2. **authorization** (browser):
```
http://localhost:8081/oauth2/authorize?
  client_id=<client_id>&
  redirect_uri=<redirect_uri>&
  response_type=code&
  code_challenge=<sha256_base64url(verifier)>&
  code_challenge_method=S256
```

3. **token exchange**:
```bash
curl -X POST http://localhost:8081/oauth2/token \
  -d "grant_type=authorization_code&\
      code=<code>&\
      client_id=<client_id>&\
      client_secret=<client_secret>&\
      code_verifier=<verifier>"
```

Receives jwt access token.

### PKCE Enforcement

Pierre requires pkce (rfc 7636) for security:
- code verifier: 43-128 random characters
- code challenge: base64url(sha256(verifier))
- challenge method: S256 only

No plain text challenge methods allowed.

## MCP Client Integration (Claude Code, VS Code, etc.)

mcp clients (claude code, vs code with cline/continue, cursor, etc.) connect to pierre via http-based mcp protocol.

### Authentication Flow

1. **user registration and login**:
```bash
# create user account
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'

# login to get jwt token (OAuth2 ROPC flow)
curl -X POST http://localhost:8081/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=user@example.com&password=SecurePass123!"
```

response includes jwt token:
```json
{
  "jwt_token": "eyJ0eXAiOiJKV1Qi...",
  "expires_at": "2025-11-05T18:00:00Z",
  "user": {
    "id": "75059e8b-1f56-4fcf-a14e-860966783c93",
    "email": "user@example.com"
  }
}
```

2. **configure mcp client**:

option a: **claude code** - using `/mcp` command (interactive):
```bash
# in claude code session
/mcp add pierre-production \
  --url http://localhost:8081/mcp \
  --transport http \
  --header "Authorization: Bearer eyJ0eXAiOiJKV1Qi..."
```

manual configuration (`~/.config/claude-code/mcp_config.json`):
```json
{
  "mcpServers": {
    "pierre-production": {
      "url": "http://localhost:8081/mcp",
      "transport": "http",
      "headers": {
        "Authorization": "Bearer eyJ0eXAiOiJKV1Qi..."
      }
    }
  }
}
```

option b: **vs code** (cline, continue, cursor) - edit settings:

for cline extension (`~/.vscode/settings.json` or workspace settings):
```json
{
  "cline.mcpServers": {
    "pierre-production": {
      "url": "http://localhost:8081/mcp",
      "transport": "http",
      "headers": {
        "Authorization": "Bearer eyJ0eXAiOiJKV1Qi..."
      }
    }
  }
}
```

for continue extension:
```json
{
  "continue.mcpServers": [{
    "url": "http://localhost:8081/mcp",
    "headers": {
      "Authorization": "Bearer eyJ0eXAiOiJKV1Qi..."
    }
  }]
}
```

3. **automatic authentication**:

mcp clients include jwt token in all mcp requests:
```http
POST /mcp HTTP/1.1
Host: localhost:8081
Authorization: Bearer eyJ0eXAiOiJKV1Qi...
Content-Type: application/json

{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "connect_provider",
    "arguments": {"provider": "strava"}
  }
}
```

pierre's mcp server validates jwt on every request:
- extracts user_id from token
- validates signature using jwks
- checks expiration
- enforces rate limits per tenant

### MCP Endpoint Authentication Requirements

| endpoint | auth required | notes |
|----------|---------------|-------|
| `POST /mcp` (initialize) | no | discovery only |
| `POST /mcp` (tools/list) | no | unauthenticated tool listing |
| `POST /mcp` (tools/call) | yes | requires valid jwt |
| `POST /mcp` (prompts/list) | no | discovery only |
| `POST /mcp` (resources/list) | no | discovery only |

implementation: `src/mcp/multitenant.rs:1726`

### Token Expiry and Refresh

jwt tokens expire after 24 hours (default, configurable via `JWT_EXPIRY_HOURS`).

when token expires, user must:
1. login again to get new jwt token
2. update claude code configuration with new token

automatic refresh not implemented in most mcp clients (requires manual re-login).

### Connecting to Fitness Providers

once authenticated to pierre, connect to fitness providers:

1. **using mcp tool** (recommended):
```
user: "connect to strava"
```

mcp client calls `connect_provider` tool with jwt authentication:
- pierre validates jwt, extracts user_id
- generates oauth authorization url for that user_id
- opens browser for strava authorization
- callback stores strava token for user_id
- **no pierre login required** - user already authenticated via jwt!

2. **via rest api**:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/api/oauth/auth/strava/<user_id>
```

### Why No Pierre Login During Strava OAuth?

common question: "why don't i need to log into pierre when connecting to strava?"

**answer**: you're already authenticated!

sequence:
1. you logged into pierre (got jwt token)
2. configured your mcp client (claude code, vs code, cursor, etc.) with jwt token
3. mcp client includes jwt in every mcp request
4. when you say "connect to strava":
   - mcp client sends `tools/call` with jwt
   - pierre extracts user_id from jwt (e.g., `75059e8b-1f56-4fcf-a14e-860966783c93`)
   - generates oauth url: `http://localhost:8081/api/oauth/auth/strava/75059e8b-1f56-4fcf-a14e-860966783c93`
   - state parameter includes user_id: `75059e8b-1f56-4fcf-a14e-860966783c93:random_nonce`
5. browser opens strava authorization (you prove you own the strava account)
6. strava redirects to callback with code
7. pierre validates state, exchanges code for token
8. stores strava token for your user_id (from jwt)

**key insight**: jwt token proves your identity to pierre. strava oauth proves you own the fitness account. no duplicate login needed.

### Security Considerations

**jwt token storage**: mcp clients store jwt tokens in configuration files:
- claude code: `~/.config/claude-code/mcp_config.json`
- vs code extensions: `.vscode/settings.json` or user settings

these files should have restricted permissions (chmod 600 for config files).

**token exposure**: jwt tokens in config files are sensitive. treat like passwords:
- don't commit to version control
- don't share tokens
- rotate regularly (re-login to get new token)
- revoke if compromised

**oauth state validation**: pierre validates oauth state parameters to prevent:
- csrf attacks (random nonce verified)
- user_id spoofing (state must match authenticated user)
- replay attacks (state used once)

**implementation**: `src/routes/auth.rs`, `src/mcp/multitenant.rs`

### Troubleshooting

**"authentication required" error**:
- check jwt token in your mcp client's configuration file
  - claude code: `~/.config/claude-code/mcp_config.json`
  - vs code: `.vscode/settings.json`
- verify token not expired (24h default)
- confirm token format: `Bearer eyJ0eXAi...`

**"invalid token" error**:
- token may be expired - login again
- token signature invalid - check `PIERRE_MASTER_ENCRYPTION_KEY`
- user account may be disabled - check user status

**fitness provider connection fails**:
- check oauth credentials (client_id, client_secret) at server startup
- verify redirect_uri matches provider registration
- see oauth credential validation logs for fingerprint debugging

**oauth credential debugging**:

pierre validates oauth credentials at startup and logs fingerprints:
```
OAuth provider strava: enabled=true, client_id=163846,
  secret_length=40, secret_fingerprint=f3c0d77f
```

use fingerprints to compare secrets without exposing actual values:
```bash
# check correct secret
echo -n "0f2b184c076e60a35e8ced43db9c3c20c5fcf4f3" | \
  sha256sum | cut -c1-8
# output: f3c0d77f ← correct

# check wrong secret
echo -n "1dfc45ad0a1f6983b835e4495aa9473d111d03bc" | \
  sha256sum | cut -c1-8
# output: 79092abb ← wrong!
```

if fingerprints don't match, you're using wrong credentials.

## Provider OAuth (Fitness Data)

Pierre acts as oauth client to fitness providers.

### Supported Providers

- strava (oauth2)
- garmin (oauth1 + oauth2)
- fitbit (oauth2)

### Configuration

Set environment variables:
```bash
# strava (local development)
export STRAVA_CLIENT_ID=your_id
export STRAVA_CLIENT_SECRET=your_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # local dev only

# strava (production)
export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava  # required

# garmin (local development)
export GARMIN_CLIENT_ID=your_key
export GARMIN_CLIENT_SECRET=your_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # local dev only

# garmin (production)
export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin  # required
```

**callback url security requirements**:
- http urls: local development only (localhost/127.0.0.1)
- https urls: required for production deployments
- failure to use https in production:
  - authorization codes transmitted unencrypted
  - vulnerable to token interception
  - most providers reject http callbacks in production

### Connecting Providers

Via mcp tool:
```
user: "connect to strava"
```

Or via rest api:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/oauth/connect/strava
```

Opens browser for provider authentication. After approval, redirected to callback:
```bash
# local development
http://localhost:8081/api/oauth/callback/strava?code=<auth_code>

# production (https required)
https://api.example.com/api/oauth/callback/strava?code=<auth_code>
```

Pierre exchanges code for access/refresh tokens, stores encrypted.

**security**: authorization codes in callback urls must be protected with tls in production. Http callbacks leak codes to network observers.

### Token Storage

Provider tokens stored encrypted in database:
- encryption key: tenant-specific key (derived from master key)
- algorithm: aes-256-gcm
- rotation: automatic refresh before expiry

### Checking Connection Status

```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/oauth/status
```

Response:
```json
{
  "connected_providers": ["strava"],
  "strava": {
    "connected": true,
    "expires_at": "2024-01-01T00:00:00Z"
  },
  "garmin": {
    "connected": false
  }
}
```

## Web Application Security

### Cookie-Based Authentication (Production Web Apps)

Pierre implements secure cookie-based authentication for web applications using httpOnly cookies with CSRF protection.

#### Security Model

**httpOnly cookies** prevent JavaScript access to JWT tokens, eliminating XSS-based token theft:
```
Set-Cookie: auth_token=<jwt>; HttpOnly; Secure; SameSite=Strict; Max-Age=86400
```

**CSRF protection** uses double-submit cookie pattern with cryptographic tokens:
```
Set-Cookie: csrf_token=<token>; Secure; SameSite=Strict; Max-Age=1800
X-CSRF-Token: <token>  (sent in request header)
```

#### Cookie Security Flags

| flag | value | purpose |
|------|-------|---------|
| HttpOnly | true | prevents JavaScript access (XSS protection) |
| Secure | true | requires HTTPS (prevents sniffing) |
| SameSite | Strict | prevents cross-origin requests (CSRF mitigation) |
| Max-Age | 86400 (auth), 1800 (csrf) | automatic expiration |

#### Authentication Flow

**login** (`POST /oauth/token` - OAuth2 ROPC flow):
```bash
curl -X POST http://localhost:8081/oauth/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=password&username=user@example.com&password=SecurePass123!"
```

response sets two cookies and returns csrf token:
```json
{
  "jwt_token": "eyJ0eXAiOiJKV1Qi...",  // deprecated, for backward compatibility
  "csrf_token": "cryptographic_random_32bytes",
  "user": {"id": "uuid", "email": "user@example.com"},
  "expires_at": "2025-01-20T18:00:00Z"
}
```

cookies set automatically:
```
Set-Cookie: auth_token=eyJ0eXAiOiJKV1Qi...; HttpOnly; Secure; SameSite=Strict; Max-Age=86400
Set-Cookie: csrf_token=cryptographic_random_32bytes; Secure; SameSite=Strict; Max-Age=1800
```

**authenticated requests**:

browsers automatically include cookies. web apps must include csrf token header:
```bash
curl -X POST http://localhost:8081/api/something \
  -H "X-CSRF-Token: cryptographic_random_32bytes" \
  -H "Cookie: auth_token=...; csrf_token=..." \
  -d '{"data": "value"}'
```

server validates:
1. jwt token from `auth_token` cookie
2. csrf token from `csrf_token` cookie matches `X-CSRF-Token` header
3. csrf token is valid for authenticated user
4. csrf token not expired (30 minute lifetime)

**logout** (`POST /api/auth/logout`):
```bash
curl -X POST http://localhost:8081/api/auth/logout \
  -H "Cookie: auth_token=..."
```

server clears cookies:
```
Set-Cookie: auth_token=; Max-Age=0
Set-Cookie: csrf_token=; Max-Age=0
```

#### CSRF Protection Details

**token generation**:
- 256-bit (32 byte) cryptographic randomness
- user-scoped validation (token tied to specific user_id)
- 30-minute expiration
- stored in-memory (HashMap with automatic cleanup)

**validation requirements**:
- csrf validation required for: POST, PUT, DELETE, PATCH
- csrf validation skipped for: GET, HEAD, OPTIONS
- validation extracts:
  1. user_id from jwt token (auth_token cookie)
  2. csrf token from X-CSRF-Token header
  3. verifies token valid for that user_id
  4. verifies token not expired

**double-submit cookie pattern**:
```
1. server generates csrf token
2. server sets csrf_token cookie (JavaScript readable)
3. server returns csrf_token in JSON response
4. client stores csrf_token in memory
5. client includes X-CSRF-Token header in state-changing requests
6. server validates:
   - csrf_token cookie matches X-CSRF-Token header
   - token is valid for authenticated user_id
   - token not expired
```

**security benefits**:
- attacker cannot read csrf token (cross-origin restriction)
- attacker cannot forge valid csrf token (cryptographic randomness)
- attacker cannot reuse old token (user-scoped validation)
- attacker cannot use expired token (30-minute lifetime)

#### Frontend Integration (React/TypeScript)

**axios configuration**:
```typescript
// enable automatic cookie handling
axios.defaults.withCredentials = true;

// request interceptor for csrf token
axios.interceptors.request.use((config) => {
  if (['POST', 'PUT', 'DELETE', 'PATCH'].includes(config.method?.toUpperCase() || '')) {
    const csrfToken = apiService.getCsrfToken();
    if (csrfToken && config.headers) {
      config.headers['X-CSRF-Token'] = csrfToken;
    }
  }
  return config;
});

// response interceptor for 401 errors
axios.interceptors.response.use(
  (response) => response,
  async (error) => {
    if (error.response?.status === 401) {
      // clear csrf token and redirect to login
      apiService.clearCsrfToken();
      window.location.href = '/login';
    }
    return Promise.reject(error);
  }
);
```

**login flow** (OAuth2 ROPC):
```typescript
async function login(email: string, password: string) {
  const params = new URLSearchParams({
    grant_type: 'password',
    username: email,
    password: password
  });
  const response = await axios.post('/oauth/token', params, {
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' }
  });

  // store csrf token in memory (cookies set automatically)
  apiService.setCsrfToken(response.data.csrf_token);

  // store user info in localStorage (not sensitive)
  localStorage.setItem('user', JSON.stringify(response.data.user));

  return response.data;
}
```

**logout flow**:
```typescript
async function logout() {
  try {
    // call backend to clear httpOnly cookies
    await axios.post('/api/auth/logout');
  } catch (error) {
    console.error('Logout failed:', error);
  } finally {
    // clear client-side state
    apiService.clearCsrfToken();
    localStorage.removeItem('user');
  }
}
```

#### Token Refresh

web apps can proactively refresh tokens using the refresh endpoint:

```typescript
async function refreshToken() {
  const response = await axios.post('/api/auth/refresh');

  // server sets new auth_token and csrf_token cookies
  apiService.setCsrfToken(response.data.csrf_token);

  return response.data;
}
```

refresh generates:
- new jwt token (24 hour expiry)
- new csrf token (30 minute expiry)
- both cookies updated automatically

**when to refresh**:
- proactively before jwt expires (24h default)
- after csrf token expires (30min default)
- after receiving 401 response with expired token

#### Implementation References

**backend**:
- csrf token manager: `src/security/csrf.rs`
- secure cookie utilities: `src/security/cookies.rs`
- csrf middleware: `src/middleware/csrf.rs`
- authentication middleware: `src/middleware/auth.rs` (cookie-aware)
- auth handlers: `src/routes/auth.rs` (login, refresh, logout)

**frontend**:
- api service: `frontend/src/services/api.ts`
- auth context: `frontend/src/contexts/AuthContext.tsx`

#### Backward Compatibility

pierre supports both cookie-based and bearer token authentication simultaneously:

1. **cookie-based** (web apps): jwt from httpOnly cookie
2. **bearer token** (api clients): `Authorization: Bearer <token>` header

middleware tries cookies first, falls back to authorization header.

### API Key Authentication (Service-to-Service)

for a2a systems and service-to-service communication, api keys provide simpler authentication without cookies or csrf.

## Security Features

### Password Hashing

- algorithm: argon2id (default) or bcrypt
- configurable work factor
- per-user salt

### Token Encryption

- jwt signing: rs256 asymmetric (rsa) or hs256 symmetric
  - rs256: 4096-bit rsa keys (production), 2048-bit (tests)
  - hs256: 64-byte secret (legacy)
- provider tokens: aes-256-gcm
- encryption keys: two-tier system
  - master key (env: `PIERRE_MASTER_ENCRYPTION_KEY`)
  - tenant keys (derived from master key)

### RS256/JWKS

Asymmetric signing for distributed token verification.

Public keys available at `/admin/jwks` (legacy) and `/oauth2/jwks` (oauth2 clients):
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
      "kid": "key_2024_01_01_123456",
      "n": "modulus_base64url",
      "e": "exponent_base64url"
    }
  ]
}
```

**cache-control headers**: jwks endpoint returns `Cache-Control: public, max-age=3600` allowing browsers to cache public keys for 1 hour.

Clients verify tokens using public key. Pierre signs with private key.

Benefits:
- private key never leaves server
- clients verify without shared secret
- supports key rotation with grace period
- browser caching reduces jwks endpoint load

**key rotation**: when keys are rotated, old keys are retained during grace period to allow existing tokens to validate. New tokens are signed with the current key.

### Rate Limiting

Token bucket algorithm per authentication method:
- jwt tokens: per-tenant limits
- api keys: per-tier limits (free: 100/day, professional: 10,000/day, enterprise: unlimited)
- oauth2 endpoints: per-ip limits
  - `/oauth2/authorize`: 60 requests/minute
  - `/oauth2/token`: 30 requests/minute
  - `/oauth2/register`: 10 requests/minute

Oauth2 rate limit responses include:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
X-RateLimit-Reset: 1704067200
Retry-After: 42
```

Implementation: `src/rate_limiting.rs`, `src/oauth2/rate_limiting.rs`

### CSRF Protection

pierre implements comprehensive csrf protection for web applications:

**web application requests**:
- double-submit cookie pattern (see "Web Application Security" section above)
- 256-bit cryptographic csrf tokens
- user-scoped validation
- 30-minute token expiration
- automatic header validation for POST/PUT/DELETE/PATCH

**oauth flows**:
- state parameter validation in oauth flows (prevents csrf in oauth redirects)
- pkce for oauth2 authorization (code challenge verification)
- origin validation for web requests

see "Web Application Security" section above for detailed csrf implementation.

### Atomic Token Operations

Pierre prevents toctou (time-of-check to time-of-use) race conditions in token operations.

**problem**: token reuse attacks

Standard token validation flow vulnerable to race conditions:
```
thread 1: check token valid → ✓ valid
thread 2: check token valid → ✓ valid
thread 1: revoke token → success
thread 2: revoke token → success (token used twice!)
```

**solution**: atomic check-and-revoke

Pierre uses database-level atomic operations:
```sql
-- single atomic transaction
UPDATE oauth2_refresh_tokens
SET revoked_at = NOW()
WHERE token = ? AND revoked_at IS NULL
RETURNING *
```

Benefits:
- **race condition elimination**: only one thread can consume token
- **database-level garantees**: transaction isolation prevents concurrent access
- **zero-trust security**: every token exchange verified atomically

**vulnerable endpoints protected**:
- `POST /oauth2/token` (refresh token grant)
- token refresh operations
- authorization code exchange

**implementation details**:

Atomic operations in database plugins (`src/database_plugins/`):
```rust
/// atomically consume oauth2 refresh token (check-and-revoke in single operation)
async fn consume_refresh_token(&self, token: &str) -> Result<RefreshToken, DatabaseError>
```

Sqlite implementation uses `RETURNING` clause:
```rust
UPDATE oauth2_refresh_tokens
SET revoked_at = datetime('now')
WHERE token = ? AND revoked_at IS NULL
RETURNING *
```

Postgresql implementation uses same pattern with `RETURNING`:
```rust
UPDATE oauth2_refresh_tokens
SET revoked_at = NOW()
WHERE token = $1 AND revoked_at IS NULL
RETURNING *
```

If query returns no rows, token either:
- doesn't exist
- already revoked (race condition detected)
- expired

All three cases result in authentication failure, preventing token reuse.

Security guarantees:
- **serializability**: database transactions prevent concurrent modifications
- **atomicity**: check and revoke happen in single operation
- **consistency**: no partial state changes possible
- **isolation**: concurrent requests see consistent view

Implementation: `src/database_plugins/sqlite.rs`, `src/database_plugins/postgres.rs`, `src/oauth2/endpoints.rs`

## Troubleshooting

### "Invalid Token" Errors

- check token expiry: jwt tokens expire after 24h (default)
- verify token format: must be `Bearer <token>`
- ensure token not revoked: check `/oauth/status`

### OAuth2 Flow Fails

- verify redirect uri exactly matches registration
- check pkce challenge/verifier match
- ensure code not expired (10 min lifetime)

### Provider OAuth Fails

- verify provider credentials (client_id, client_secret)
- check redirect uri accessible from browser
- ensure callback endpoint reachable

### API Key Rejected

- verify api key active: not deleted or expired
- check rate limits: may be throttled
- ensure correct header: `X-API-Key` (case-sensitive)

## Implementation References

- jwt authentication: `src/auth.rs`
- api key management: `src/api_keys.rs`
- oauth2 server: `src/oauth2_server/`
- provider oauth: `src/oauth2_client/`
- encryption: `src/crypto/`, `src/key_management.rs`
- rate limiting: `src/rate_limiting.rs`
