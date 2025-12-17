# Pierre MCP Server - Reference Part 2: Auth & Protocols

> Reference documentation for ChatGPT. Part 2: Authentication, OAuth, Protocols.

---

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

---

# OAuth2 Server

Pierre includes a standards-compliant oauth2 authorization server for secure mcp client authentication.

## Features

- authorization code flow with pkce (s256 only)
- dynamic client registration (rfc 7591)
- server-side state validation for csrf protection
- argon2id client secret hashing
- multi-tenant isolation
- refresh token rotation
- jwt-based access tokens

## Quick Start

### 1. Register OAuth2 Client

```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["https://example.com/callback"],
    "client_name": "My MCP Client",
    "grant_types": ["authorization_code"],
    "response_types": ["code"]
  }'
```

Response:
```json
{
  "client_id": "mcp_client_abc123",
  "client_secret": "secret_xyz789",
  "client_id_issued_at": 1640000000,
  "redirect_uris": ["https://example.com/callback"],
  "grant_types": ["authorization_code"],
  "response_types": ["code"]
}
```

**important:** save `client_secret` immediately. Cannot be retrieved later.

### 2. Generate PKCE Challenge

```python
import secrets
import hashlib
import base64

# generate code verifier (43-128 characters)
code_verifier = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode('utf-8').rstrip('=')

# generate code challenge (s256)
code_challenge = base64.urlsafe_b64encode(
    hashlib.sha256(code_verifier.encode('utf-8')).digest()
).decode('utf-8').rstrip('=')

# generate state (csrf protection)
state = secrets.token_urlsafe(32)

# store code_verifier and state in session
session['code_verifier'] = code_verifier
session['oauth_state'] = state
```

### 3. Initiate Authorization

Redirect user to authorization endpoint:

```
https://pierre.example.com/oauth2/authorize?
  response_type=code&
  client_id=mcp_client_abc123&
  redirect_uri=https://example.com/callback&
  state=<random_state>&
  code_challenge=<pkce_challenge>&
  code_challenge_method=S256&
  scope=read:activities write:goals
```

User will authenticate and authorize. Pierre redirects to callback with authorization code:

```
https://example.com/callback?
  code=auth_code_xyz&
  state=<same_random_state>
```

### 4. Validate State and Exchange Code

```python
# validate state parameter (csrf protection)
received_state = request.args.get('state')
stored_state = session.pop('oauth_state', None)

if not received_state or received_state != stored_state:
    return "csrf attack detected", 400

# exchange authorization code for tokens
code = request.args.get('code')
code_verifier = session.pop('code_verifier')
```

```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=auth_code_xyz" \
  -d "redirect_uri=https://example.com/callback" \
  -d "client_id=mcp_client_abc123" \
  -d "client_secret=secret_xyz789" \
  -d "code_verifier=<stored_code_verifier>"
```

Response:
```json
{
  "access_token": "jwt_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "refresh_token_abc",
  "scope": "read:activities write:goals"
}
```

### 5. Use Access Token

```bash
curl -H "Authorization: Bearer jwt_access_token" \
  http://localhost:8081/mcp
```

## Client Registration

### Register New Client

Endpoint: `POST /oauth2/register`

Required fields:
- `redirect_uris` - array of callback urls (https required except localhost)

Optional fields:
- `client_name` - display name
- `client_uri` - client homepage url
- `grant_types` - defaults to `["authorization_code"]`
- `response_types` - defaults to `["code"]`
- `scope` - space-separated scope list

### Redirect URI Validation

Pierre enforces strict redirect uri validation:

**allowed:**
- `https://` urls (production)
- `http://localhost:*` (development)
- `http://127.0.0.1:*` (development)
- `urn:ietf:wg:oauth:2.0:oob` (out-of-band for native apps)

**rejected:**
- `http://` non-localhost urls
- urls with fragments (`#`)
- wildcard domains (`*.example.com`)
- malformed urls

### Example Registrations

**web application:**
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["https://app.example.com/auth/callback"],
    "client_name": "Example Web App",
    "client_uri": "https://app.example.com",
    "scope": "read:activities read:athlete"
  }'
```

**native application:**
```bash
curl -X POST http://localhost:8081/oauth2/register \
  -H "Content-Type: application/json" \
  -d '{
    "redirect_uris": ["http://localhost:8080/callback"],
    "client_name": "Example Desktop App",
    "scope": "read:activities write:goals"
  }'
```

## Authorization Flow

### Step 1: Authorization Request

Build authorization url with required parameters:

```python
from urllib.parse import urlencode

params = {
    'response_type': 'code',
    'client_id': client_id,
    'redirect_uri': redirect_uri,
    'state': state,                    # required for csrf protection
    'code_challenge': code_challenge,  # required for pkce
    'code_challenge_method': 'S256',   # only s256 supported
    'scope': 'read:activities write:goals'  # optional
}

auth_url = f"https://pierre.example.com/oauth2/authorize?{urlencode(params)}"
```

Redirect user to `auth_url`.

### Step 2: User Authentication

If user not logged in, pierre displays login form. After successful login, shows authorization consent screen.

### Step 3: Authorization Callback

Pierre redirects to your `redirect_uri` with authorization code:

```
https://example.com/callback?code=<auth_code>&state=<state>
```

Error response (if user denies):
```
https://example.com/callback?error=access_denied&error_description=User+denied+authorization
```

### Step 4: Token Exchange

Exchange authorization code for access token:

```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=authorization_code" \
  -d "code=<auth_code>" \
  -d "redirect_uri=<same_redirect_uri>" \
  -d "client_id=<client_id>" \
  -d "client_secret=<client_secret>" \
  -d "code_verifier=<pkce_verifier>"
```

**important:** authorization codes expire in 10 minutes and are single-use.

## Token Management

### Access Tokens

Jwt-based tokens with 1-hour expiration (configurable).

Claims include:
- `sub` - user id
- `email` - user email
- `tenant_id` - tenant identifier
- `scope` - granted scopes
- `exp` - expiration timestamp

### Refresh Tokens

Use refresh token to obtain new access token without re-authentication:

```bash
curl -X POST http://localhost:8081/oauth2/token \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "grant_type=refresh_token" \
  -d "refresh_token=<refresh_token>" \
  -d "client_id=<client_id>" \
  -d "client_secret=<client_secret>"
```

Response:
```json
{
  "access_token": "new_jwt_access_token",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "new_refresh_token",
  "scope": "read:activities write:goals"
}
```

**refresh token rotation:** pierre issues new refresh token with each refresh request. Old refresh token is revoked.

### Token Validation

Validate access token and optionally refresh if expired:

```bash
curl -X POST http://localhost:8081/oauth2/validate \
  -H "Authorization: Bearer <access_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "refresh_token": "optional_refresh_token"
  }'
```

Responses:

**valid token:**
```json
{
  "status": "valid",
  "expires_in": 1800
}
```

**token refreshed:**
```json
{
  "status": "refreshed",
  "access_token": "new_jwt_token",
  "refresh_token": "new_refresh_token",
  "token_type": "Bearer"
}
```

**invalid token:**
```json
{
  "status": "invalid",
  "reason": "token expired",
  "requires_full_reauth": true
}
```

## Security Features

### PKCE (Proof Key for Code Exchange)

Pierre requires pkce for all authorization code flows.

**supported methods:**
- `S256` (sha256) - required

**rejected methods:**
- `plain` - insecure, not supported

**implementation:**
1. Generate random `code_verifier` (43-128 characters)
2. Compute `code_challenge = base64url(sha256(code_verifier))`
3. Send `code_challenge` in authorization request
4. Send `code_verifier` in token exchange
5. Pierre validates `sha256(code_verifier) == code_challenge`

Prevents authorization code interception attacks.

### State Parameter Validation

Pierre implements defense-in-depth csrf protection with server-side state validation.

**client requirements:**
1. Generate cryptographically random state (≥128 bits entropy)
2. Store state in session before authorization request
3. Include state in authorization request
4. Validate state matches in callback

**server behavior:**
1. Stores state with 10-minute expiration
2. Binds state to client_id and user
3. Validates state on callback
4. Marks state as used (single-use)
5. Rejects expired, used, or mismatched states

**example implementation:**
```python
import secrets

# before authorization
state = secrets.token_urlsafe(32)
session['oauth_state'] = state

# in callback
received_state = request.args.get('state')
stored_state = session.pop('oauth_state', None)

if not received_state or received_state != stored_state:
    abort(400, "invalid state - possible csrf attack")
```

### Client Secret Hashing

Client secrets hashed with argon2id (memory-hard algorithm resistant to gpu attacks).

**verification:**
```bash
# validate client credentials
curl -X POST http://localhost:8081/oauth2/token \
  -d "client_id=<id>" \
  -d "client_secret=<secret>" \
  ...
```

Pierre verifies secret using constant-time comparison to prevent timing attacks.

### Multi-tenant Isolation

All oauth artifacts (codes, tokens, states) bound to tenant_id. Cross-tenant access prevented at database layer.

## Scopes

Pierre supports fine-grained permission control via oauth scopes.

### Available Scopes

**fitness data:**
- `read:activities` - read activity data
- `write:activities` - create/update activities
- `read:athlete` - read athlete profile
- `write:athlete` - update athlete profile

**goals and analytics:**
- `read:goals` - read fitness goals
- `write:goals` - create/update goals
- `read:analytics` - access analytics data

**administrative:**
- `admin:users` - manage users
- `admin:system` - system administration

### Requesting Scopes

Include in authorization request:

```
/oauth2/authorize?
  ...
  scope=read:activities read:athlete write:goals
```

### Scope Validation

Pierre validates requested scopes against client's registered scopes. Access tokens include granted scopes in jwt claims.

## Error Handling

### Authorization Errors

Returned as query parameters in redirect:

```
https://example.com/callback?
  error=invalid_request&
  error_description=missing+code_challenge&
  state=<state>
```

**common errors:**
- `invalid_request` - missing or invalid parameters
- `unauthorized_client` - client not authorized for this flow
- `access_denied` - user denied authorization
- `unsupported_response_type` - response_type not supported
- `invalid_scope` - requested scope invalid or not allowed
- `server_error` - internal server error

### Token Errors

Returned as json in response body:

```json
{
  "error": "invalid_grant",
  "error_description": "authorization code expired",
  "error_uri": "https://datatracker.ietf.org/doc/html/rfc6749#section-5.2"
}
```

**common errors:**
- `invalid_request` - malformed request
- `invalid_client` - client authentication failed
- `invalid_grant` - code expired, used, or invalid
- `unauthorized_client` - client not authorized
- `unsupported_grant_type` - grant type not supported

## Common Integration Patterns

### Web Application Flow

1. User clicks "connect with pierre"
2. App redirects to pierre authorization endpoint
3. User logs in (if needed) and approves
4. Pierre redirects back with authorization code
5. App exchanges code for tokens (server-side)
6. App stores tokens securely (encrypted database)
7. App uses access token for api requests
8. App refreshes token before expiration

### Native Application Flow

1. App opens system browser to authorization url
2. User authenticates and approves
3. Browser redirects to `http://localhost:port/callback`
4. App's local server receives callback
5. App exchanges code for tokens
6. App stores tokens securely (os keychain)

### Single Page Application (SPA) Flow

**recommended:** use authorization code flow with pkce:

1. Spa redirects to pierre authorization endpoint
2. Pierre redirects back with authorization code
3. Spa exchanges code for tokens via backend proxy
4. Backend stores refresh token
5. Backend returns short-lived access token to spa
6. Spa uses access token for api requests
7. Spa requests new access token via backend when expired

**not recommended:** implicit flow (deprecated)

## Troubleshooting

### Authorization Code Expired

**symptom:** `invalid_grant` error when exchanging code

**solution:** authorization codes expire in 10 minutes. Restart authorization flow.

### PKCE Validation Failed

**symptom:** `invalid_grant: pkce verification failed`

**solutions:**
- ensure `code_verifier` sent in token request matches original
- verify code_challenge computed as `base64url(sha256(code_verifier))`
- check no extra padding (`=`) in base64url encoding

### State Validation Failed

**symptom:** `invalid_grant: invalid state parameter`

**solutions:**
- ensure state sent in callback matches original request
- check state not expired (10-minute ttl)
- verify state not reused (single-use)
- confirm state stored in user session before authorization

### Redirect URI Mismatch

**symptom:** `invalid_request: redirect_uri mismatch`

**solutions:**
- redirect_uri in authorization request must exactly match registration
- redirect_uri in token request must match authorization request
- https required for non-localhost urls

### Client Authentication Failed

**symptom:** `invalid_client`

**solutions:**
- verify client_id correct
- verify client_secret correct (case-sensitive)
- ensure client_secret not expired
- check client not deleted

### Refresh Token Revoked

**symptom:** `invalid_grant: refresh token revoked or expired`

**solutions:**
- refresh tokens expire after 30 days of inactivity
- old refresh tokens revoked after successful refresh (rotation)
- restart authorization flow to obtain new tokens

## Configuration

### Token Lifetimes

Pierre currently uses fixed lifetimes for OAuth2 artifacts (configured in code, not via environment variables):

- Authorization codes: 10 minutes (single-use)
- Access tokens: 1 hour
- Refresh tokens: 30 days
- State parameters: 10 minutes

Changing these values requires a code change in the OAuth2 server configuration (see `src/oauth2_server/` and `src/constants/`).

## See Also

- authentication - jwt and api key authentication
- protocols - fitness provider integrations
- configuration - server configuration

---

# OAuth Client (Fitness Providers)

Pierre acts as an oauth 2.0 client to connect to fitness providers (strava, fitbit, garmin) on behalf of users.

## Overview

**oauth2_client module** (`src/oauth2_client/`):
- pierre connects TO fitness providers as oauth client
- handles user authorization and token management
- supports pkce for enhanced security
- multi-tenant credential isolation

**separate from oauth2_server**:
- oauth2_server: mcp clients connect TO pierre
- oauth2_client: pierre connects TO fitness providers

## Supported Providers

| provider | oauth version | pkce | status | scopes | implementation |
|----------|--------------|------|--------|--------|----------------|
| strava | oauth 2.0 | required | active | `activity:read_all` | `src/providers/strava.rs` |
| fitbit | oauth 2.0 | required | active | `activity`,`heartrate`,`location`,`nutrition`,`profile`,`settings`,`sleep`,`social`,`weight` | `src/providers/fitbit.rs` |
| garmin | oauth 2.0 | required | active | `wellness:read`,`activities:read` | `src/providers/garmin_provider.rs` |
| whoop | oauth 2.0 | required | active | `read:profile`,`read:body_measurement`,`read:workout`,`read:sleep`,`read:recovery`,`read:cycles` | `src/providers/whoop_provider.rs` |
| terra | oauth 2.0 | required | active | device-dependent (150+ wearables) | `src/providers/terra_provider.rs` |

**note**: providers require compile-time feature flags (`provider-strava`, `provider-fitbit`, `provider-whoop`, `provider-terra`, etc.).

Implementation: `src/oauth2_client/mod.rs`

## Configuration

### Environment Variables

**strava:**
```bash
export STRAVA_CLIENT_ID=your_client_id
export STRAVA_CLIENT_SECRET=your_client_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/api/oauth/callback/strava  # dev
```

**fitbit:**
```bash
export FITBIT_CLIENT_ID=your_client_id
export FITBIT_CLIENT_SECRET=your_client_secret
export FITBIT_REDIRECT_URI=http://localhost:8081/api/oauth/callback/fitbit  # dev
```

**garmin:**
```bash
export GARMIN_CLIENT_ID=your_consumer_key
export GARMIN_CLIENT_SECRET=your_consumer_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/api/oauth/callback/garmin  # dev
```

**whoop:**
```bash
export WHOOP_CLIENT_ID=your_client_id
export WHOOP_CLIENT_SECRET=your_client_secret
export WHOOP_REDIRECT_URI=http://localhost:8081/api/oauth/callback/whoop  # dev
```

**production:** use https redirect urls:
```bash
export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
export FITBIT_REDIRECT_URI=https://api.example.com/api/oauth/callback/fitbit
export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
export WHOOP_REDIRECT_URI=https://api.example.com/api/oauth/callback/whoop
```

Constants: `src/constants/oauth/providers.rs`

## Multi-tenant Architecture

### Credential Hierarchy

Credentials resolved in priority order:
1. **tenant-specific credentials** (database, encrypted)
2. **server-level credentials** (environment variables)

Implementation: `src/oauth2_client/tenant_client.rs`

### Tenant OAuth Client

**`TenantOAuthClient`** (`src/oauth2_client/tenant_client.rs:36-49`):
```rust
pub struct TenantOAuthClient {
    pub oauth_manager: Arc<Mutex<TenantOAuthManager>>,
}
```

**features:**
- tenant-specific credential isolation
- rate limiting per tenant per provider
- automatic credential fallback to server config

### Storing Tenant Credentials

**via authorization request headers:**
```bash
curl -X GET "http://localhost:8081/api/oauth/auth/strava/uuid" \
  -H "x-strava-client-id: tenant_client_id" \
  -H "x-strava-client_secret: tenant_client_secret"
```

Credentials stored encrypted in database, bound to tenant.

**via api:**
```rust
tenant_oauth_client.store_credentials(
    tenant_id,
    "strava",
    StoreCredentialsRequest {
        client_id: "tenant_client_id".to_string(),
        client_secret: "tenant_client_secret".to_string(),
        redirect_uri: "https://tenant.example.com/callback/strava".to_string(),
        scopes: vec!["activity:read_all".to_string()],
        configured_by: user_id,
    }
).await?;
```

Implementation: `src/oauth2_client/tenant_client.rs:21-34`

### Rate Limiting

**default limits** (`src/tenant/oauth_manager.rs`):
- strava: 1000 requests/day per tenant
- fitbit: 150 requests/day per tenant
- garmin: 1000 requests/day per tenant
- whoop: 1000 requests/day per tenant

**rate limit enforcement:**
```rust
let (current_usage, daily_limit) = manager
    .check_rate_limit(tenant_id, provider)?;

if current_usage >= daily_limit {
    return Err(AppError::invalid_input(format!(
        "Tenant {} exceeded daily rate limit for {}: {}/{}",
        tenant_id, provider, current_usage, daily_limit
    )));
}
```

Implementation: `src/oauth2_client/tenant_client.rs:64-75`

## OAuth Flow

### Step 1: Initiate Authorization

**via mcp tool:**
```
user: "connect to strava"
```

**via rest api:**
```bash
curl -H "Authorization: Bearer <jwt>" \
  "http://localhost:8081/api/oauth/auth/strava/<user_id>"
```

**flow manager** (`src/oauth2_client/flow_manager.rs:29-105`):
1. Validates user_id and tenant_id
2. Processes optional tenant credentials from headers
3. Generates authorization redirect url
4. Returns http 302 redirect to provider

### Step 2: User Authorizes at Provider

Pierre generates authorization url with:
- **pkce s256 challenge** (128-character verifier)
- **state parameter** for csrf protection (`{user_id}:{random_uuid}`)
- **provider scopes** (activity read, heartrate, etc.)

**pkce generation** (`src/oauth2_client/client.rs:35-58`):
```rust
pub fn generate() -> PkceParams {
    // 128-character random verifier (43-128 allowed by RFC)
    let code_verifier: String = (0..128)
        .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
        .collect();

    // S256 challenge: base64url(sha256(code_verifier))
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    PkceParams {
        code_verifier,
        code_challenge,
        code_challenge_method: "S256".into(),
    }
}
```

User authenticates with provider and grants permissions.

### Step 3: OAuth Callback

Provider redirects to pierre callback:
```
http://localhost:8081/api/oauth/callback/strava?
  code=authorization_code&
  state=user_id:random_uuid
```

**callback handling** (`src/routes/auth.rs`):
1. Validates state parameter (csrf protection)
2. Extracts user_id from state
3. Exchanges authorization code for access token
4. Encrypts tokens with aes-256-gcm
5. Stores in database (tenant-isolated)
6. Renders success page

### Step 4: Success Page

User sees branded html page:
- provider name and connection status
- user identifier
- pierre logo
- instructions to return to mcp client

Template: `templates/oauth_success.html`
Renderer: `src/oauth2_client/flow_manager.rs:350-393`

## Token Management

### OAuth2Token Structure

**`OAuth2Token`** (`src/oauth2_client/client.rs:61-82`):
```rust
pub struct OAuth2Token {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

impl OAuth2Token {
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    }

    pub fn will_expire_soon(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now() + Duration::minutes(5))
    }
}
```

### Storage

Tokens stored in `users` table with provider-specific columns:

```sql
-- strava example
strava_access_token     TEXT      -- encrypted
strava_refresh_token    TEXT      -- encrypted
strava_expires_at       TIMESTAMP
strava_scope            TEXT      -- comma-separated
```

**encryption:**
- algorithm: aes-256-gcm
- key: tenant-specific (derived from `PIERRE_MASTER_ENCRYPTION_KEY`)
- unique key per tenant ensures isolation

Implementation: `src/database/tokens.rs`, `src/crypto/`, `src/key_management.rs`

### Automatic Refresh

Pierre refreshes expired tokens before api requests:

**refresh criteria:**
- access token expired or expiring within 5 minutes
- refresh token available and valid

**refresh flow** (`src/oauth2_client/client.rs:272-302`):
```rust
pub async fn refresh_token(&self, refresh_token: &str) -> AppResult<OAuth2Token> {
    let params = [
        ("client_id", self.config.client_id.as_str()),
        ("client_secret", self.config.client_secret.as_str()),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ];

    let response: TokenResponse = self
        .client
        .post(&self.config.token_url)
        .form(&params)
        .send()
        .await?
        .json()
        .await?;

    Ok(Self::token_from_response(response))
}
```

Note: PKCE (`code_verifier`) is only used during authorization code exchange, not token refresh per RFC 7636.

### Manual Token Operations

**get token:**
```rust
let token = database.get_oauth_token(user_id, "strava").await?;
```

**update token:**
```rust
database.update_oauth_token(
    user_id,
    "strava",
    OAuthToken {
        access_token: "new_token".to_string(),
        refresh_token: Some("new_refresh".to_string()),
        expires_at: Utc::now() + Duration::hours(6),
        scope: "activity:read_all".to_string(),
    }
).await?;
```

**clear token (disconnect):**
```rust
database.clear_oauth_token(user_id, "strava").await?;
```

Implementation: `src/database/tokens.rs`

## Connection Status

**check connection:**
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/api/oauth/status
```

Response:
```json
{
  "connected_providers": ["strava", "fitbit"],
  "providers": {
    "strava": {
      "connected": true,
      "expires_at": "2024-01-01T12:00:00Z",
      "scope": "activity:read_all",
      "auto_refresh": true
    },
    "fitbit": {
      "connected": true,
      "expires_at": "2024-01-01T14:00:00Z",
      "scope": "activity heartrate location",
      "auto_refresh": true
    },
    "garmin": {
      "connected": false
    }
  }
}
```

**disconnect provider:**

Use the `disconnect_provider` MCP tool to revoke a provider connection; there is no standalone REST `DELETE /api/oauth/disconnect/{provider}` endpoint.

Implementation: `src/routes/auth.rs`

## Security Features

### PKCE (Proof Key for Code Exchange)

**implementation** (`src/oauth2_client/client.rs:27-59`):

All provider oauth flows use pkce (rfc 7636):

**code verifier:**
- 128 characters
- cryptographically random
- allowed characters: `A-Z a-z 0-9 - . _ ~`

**code challenge:**
- sha256 hash of code verifier
- base64url encoded (no padding)
- method: s256 only

Prevents authorization code interception attacks.

### State Parameter Validation

**state format:** `{user_id}:{random_uuid}`

**validation** (`src/oauth2_client/flow_manager.rs:162-215`):
1. Extract user_id from state
2. Verify user exists and belongs to tenant
3. Ensure state not reused (single-use)

Invalid state results in authorization rejection.

### Token Encryption

**encryption** (`src/crypto/`, `src/key_management.rs`):
- algorithm: aes-256-gcm
- key derivation:
  - master key: `PIERRE_MASTER_ENCRYPTION_KEY` (base64, 32 bytes)
  - tenant keys: derived from master key using tenant_id
  - unique key per tenant ensures isolation

**encrypted fields:**
- access_token
- refresh_token
- client_secret (for tenant credentials)

Decryption requires:
1. Correct master key
2. Correct tenant_id
3. Valid encryption nonce

### Tenant Isolation

Oauth artifacts never shared between tenants:
- credentials stored per tenant_id
- tokens bound to user and tenant
- rate limits enforced per tenant
- database queries include tenant_id filter

Cross-tenant access prevented at database layer.

Implementation: `src/tenant/oauth_manager.rs`

## Provider-specific Details

### Strava

**auth url:** `https://www.strava.com/oauth/authorize`
**token url:** `https://www.strava.com/oauth/token`
**api base:** `https://www.strava.com/api/v3`

**default scopes:** `activity:read_all`

**available scopes:**
- `read` - read public profile
- `activity:read` - read non-private activities
- `activity:read_all` - read all activities (public and private)
- `activity:write` - create and update activities

**rate limits:**
- 100 requests per 15 minutes per access token
- 1000 requests per day per application

**token lifetime:**
- access token: 6 hours
- refresh token: permanent (until revoked)

Implementation: `src/providers/strava.rs`, `src/providers/strava_provider.rs`

### Fitbit

**auth url:** `https://www.fitbit.com/oauth2/authorize`
**token url:** `https://api.fitbit.com/oauth2/token`
**api base:** `https://api.fitbit.com/1`

**default scopes:** `activity heartrate location nutrition profile settings sleep social weight`

**scope details:**
- `activity` - steps, distance, calories, floors
- `heartrate` - heart rate data
- `location` - gps data
- `nutrition` - food and water logs
- `profile` - personal information
- `settings` - user preferences
- `sleep` - sleep logs
- `social` - friends and leaderboards
- `weight` - weight and body measurements

**rate limits:**
- 150 requests per hour per user

**token lifetime:**
- access token: 8 hours
- refresh token: 1 year

Implementation: `src/providers/fitbit.rs`

### Garmin

**auth url:** `https://connect.garmin.com/oauthConfirm`
**token url:** `https://connectapi.garmin.com/oauth-service/oauth/access_token`
**api base:** `https://apis.garmin.com`

**default scopes:** `wellness:read activities:read`

**scope details:**
- `wellness:read` - health metrics (sleep, stress, hrv)
- `activities:read` - workout and activity data
- `wellness:write` - update health data
- `activities:write` - create activities

**rate limits:**
- varies by api endpoint
- typically 1000 requests per day

**token lifetime:**
- access token: 1 year
- refresh token: not provided (long-lived access token)

Implementation: `src/providers/garmin_provider.rs`

### WHOOP

**auth url:** `https://api.prod.whoop.com/oauth/oauth2/auth`
**token url:** `https://api.prod.whoop.com/oauth/oauth2/token`
**api base:** `https://api.prod.whoop.com/developer/v1`

**default scopes:** `offline read:profile read:body_measurement read:workout read:sleep read:recovery read:cycles`

**scope details:**
- `offline` - offline access for token refresh
- `read:profile` - user profile information
- `read:body_measurement` - body measurements (weight, height)
- `read:workout` - workout/activity data with strain scores
- `read:sleep` - sleep sessions and metrics
- `read:recovery` - daily recovery scores
- `read:cycles` - physiological cycle data

**rate limits:**
- varies by endpoint
- standard api rate limiting applies

**token lifetime:**
- access token: 1 hour
- refresh token: long-lived (requires `offline` scope)

Implementation: `src/providers/whoop_provider.rs`

## Error Handling

### Authorization Errors

Displayed on html error page (`templates/oauth_error.html`):

**common errors:**
- `access_denied` - user denied authorization
- `invalid_request` - missing or invalid parameters
- `invalid_scope` - requested scope not available
- `server_error` - provider api error

Renderer: `src/oauth2_client/flow_manager.rs:329-347`

### Callback Errors

Returned as query parameters:
```
http://localhost:8081/api/oauth/callback/strava?
  error=access_denied&
  error_description=User+declined+authorization
```

### Token Errors

**expired token:**
- automatically refreshed before api request
- no user action required

**invalid refresh token:**
- user must re-authorize
- connection status shows disconnected

**rate limit exceeded:**
```json
{
  "error": "rate_limit_exceeded",
  "provider": "strava",
  "retry_after_secs": 3600,
  "limit_type": "daily quota"
}
```

Implementation: `src/providers/errors.rs`

## Troubleshooting

### Authorization Fails

**symptom:** redirect to provider fails or returns error

**solutions:**
- verify provider credentials (client_id, client_secret)
- check redirect_uri matches provider configuration exactly
- ensure redirect_uri uses https in production
- confirm provider api credentials active and approved

### Callback Error: State Validation Failed

**symptom:** `invalid state parameter` error on callback

**solutions:**
- ensure user_id in authorization request matches authenticated user
- check user exists in database
- verify tenant association correct
- confirm no url encoding issues in state parameter

### Token Refresh Fails

**symptom:** api requests fail with authentication error

**solutions:**
- check refresh token not expired or revoked
- verify provider credentials still valid
- ensure network connectivity to provider api
- re-authorize user to obtain new tokens

### Rate Limit Exceeded

**symptom:** api requests rejected with rate limit error

**solutions:**
- check current usage via tenant_oauth_manager
- wait for daily reset (midnight utc)
- request rate limit increase from provider
- optimize api call patterns to reduce requests

### Encryption Key Mismatch

**symptom:** cannot decrypt stored tokens

**solutions:**
- verify `PIERRE_MASTER_ENCRYPTION_KEY` unchanged
- check key is valid base64 (32 bytes decoded)
- ensure key not rotated without token re-encryption
- re-authorize users if key changed

## Implementation References

- oauth2 client: `src/oauth2_client/client.rs`
- oauth flow manager: `src/oauth2_client/flow_manager.rs`
- tenant client: `src/oauth2_client/tenant_client.rs`
- tenant oauth manager: `src/tenant/oauth_manager.rs`
- provider implementations: `src/providers/`
- token storage: `src/database/tokens.rs`
- route handlers: `src/routes/auth.rs`
- templates: `templates/oauth_success.html`, `templates/oauth_error.html`

## See Also

- oauth2 server - mcp client authentication
- authentication - authentication methods and jwt tokens
- configuration - environment variables

---

# Protocols

Pierre implements three protocols on a single http port (8081).

## MCP (Model Context Protocol)

Json-rpc 2.0 protocol for ai assistant integration.

### Endpoints

- `POST /mcp` - main mcp endpoint
- `GET /mcp/sse/{session_id}` - sse transport for streaming (session-scoped)

### Transport

Pierre supports both http and sse transports:
- http: traditional request-response
- sse: server-sent events for streaming responses

Sdk handles transport negotiation automatically.

### Authentication

Mcp requests require jwt bearer token in authorization header:
```
Authorization: Bearer <jwt_token>
```

Obtained via oauth2 flow (sdk handles automatically).

### Request Format

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

### Response Format

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

### Output Format Parameter

Most data-returning tools support an optional `format` parameter for output serialization:

| Format | Description | Use Case |
|--------|-------------|----------|
| `json` | Standard JSON (default) | Universal compatibility |
| `toon` | Token-Oriented Object Notation | ~40% fewer LLM tokens |

Example with TOON format:
```json
{
  "jsonrpc": "2.0",
  "id": "1",
  "method": "tools/call",
  "params": {
    "name": "get_activities",
    "arguments": {
      "provider": "strava",
      "limit": 100,
      "format": "toon"
    }
  }
}
```

TOON format responses include `format: "toon"` and `content_type: "application/vnd.toon"` in the result. Use TOON for large datasets (year summaries, batch analysis) to reduce LLM context usage.

See TOON specification for format details.

### MCP Methods

- `initialize` - start session
- `tools/list` - list available tools
- `tools/call` - execute tool
- `resources/list` - list resources
- `prompts/list` - list prompts

Implementation: `src/mcp/protocol.rs`, `src/protocols/universal/`

## OAuth2 Authorization Server

Rfc 7591 (dynamic client registration) + rfc 7636 (pkce) compliant oauth2 server for mcp client authentication.

### Endpoints

- `GET /.well-known/oauth-authorization-server` - server metadata (rfc 8414)
- `POST /oauth2/register` - dynamic client registration
- `GET /oauth2/authorize` - authorization endpoint
- `POST /oauth2/token` - token endpoint
- `GET /oauth2/jwks` - json web key set
- `GET /.well-known/jwks.json` - jwks at standard oidc location
- `POST /oauth2/validate-and-refresh` - validate and refresh jwt tokens
- `POST /oauth2/token-validate` - validate jwt token

### Registration Flow

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

### PKCE Requirement

Pierre enforces pkce (rfc 7636) for all authorization code flows. Clients must:
- generate code verifier (43-128 characters)
- create code challenge: `base64url(sha256(verifier))`
- include challenge in authorization request
- include verifier in token request

### Server Discovery (RFC 8414)

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

### JWKS Endpoint

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

### Key Rotation

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

### Rate Limiting

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

## A2A (Agent-to-Agent Protocol)

Protocol for autonomous ai systems to communicate.

### Endpoints

- `GET /a2a/status` - protocol status
- `GET /a2a/tools` - available tools
- `POST /a2a/execute` - execute tool
- `GET /a2a/monitoring` - monitoring info

### Authentication

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

### Agent Cards

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

### Request Format

```json
{
  "tool": "analyze_activity",
  "parameters": {
    "activity_id": "12345",
    "analysis_type": "comprehensive"
  }
}
```

### Response Format

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

## REST API

Traditional rest endpoints for web applications.

### Authentication Endpoints

- `POST /api/auth/register` - user registration (admin-provisioned)
- `POST /api/auth/login` - user login
- `POST /api/auth/logout` - logout
- `POST /api/auth/refresh` - refresh jwt token

### Provider OAuth Endpoints

- `GET /api/oauth/auth/{provider}/{user_id}` - initiate oauth (strava, garmin, fitbit, whoop)
- `GET /api/oauth/callback/{provider}` - oauth callback
- `GET /api/oauth/status` - connection status

### Admin Endpoints

- `POST /admin/setup` - create admin user
- `POST /admin/users` - manage users
- `GET /admin/analytics` - usage analytics

### Configuration Endpoints

- `GET /api/configuration/catalog` - config catalog
- `GET /api/configuration/profiles` - available profiles
- `GET /api/configuration/user` - user config
- `PUT /api/configuration/user` - update config

Implementation: `src/routes.rs`, `src/admin_routes.rs`, `src/configuration_routes.rs`

## SSE (Server-Sent Events)

Real-time notifications for oauth completions and system events.

### Endpoint

```
GET /notifications/sse?user_id=<user_id>
```

### Event Types

- `oauth_complete` - oauth flow completed
- `oauth_error` - oauth flow failed
- `system_status` - system status update

### Example

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

## Protocol Comparison

| feature | mcp | oauth2 | a2a | rest |
|---------|-----|--------|-----|------|
| primary use | ai assistants | client auth | agent comms | web apps |
| auth method | jwt bearer | - | api key | jwt bearer |
| transport | http + sse | http | http | http |
| format | json-rpc 2.0 | oauth2 | json | json |
| implementation | `src/mcp/` | `src/oauth2_server/` | `src/a2a/` | `src/routes/` |

## Choosing a Protocol

- **ai assistant integration**: use mcp (claude, chatgpt)
- **web application**: use rest api
- **autonomous agents**: use a2a
- **client authentication**: use oauth2 (for mcp clients)

All protocols share the same business logic via `src/protocols/universal/`.

---

# provider registration guide

This guide shows how pierre's pluggable provider architecture supports **1 to x providers simultaneously** and how new providers are registered.

## provider registration flow

```
┌──────────────────────────────────────────────────────┐
│  Step 1: Application Startup                         │
│  ProviderRegistry::new() called                      │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 2: Factory Registration (1 to x providers)     │
│                                                       │
│  registry.register_factory("strava", StravaFactory)  │
│  registry.register_factory("garmin", GarminFactory)  │
│  registry.register_factory("fitbit", FitbitFactory)  │
│  registry.register_factory("synthetic", SynthFactory)│
│  registry.register_factory("whoop", WhoopFactory)    │ <- built-in
│  registry.register_factory("terra", TerraFactory)    │ <- built-in
│  registry.register_factory("polar", PolarFactory)    │ <- custom example
│  ... unlimited providers ...                         │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 3: Environment Configuration Loading           │
│                                                       │
│  For each registered provider:                       │
│    config = load_provider_env_config(                │
│      provider_name,                                  │
│      default_auth_url,                               │
│      default_token_url,                              │
│      default_api_base_url,                           │
│      default_revoke_url,                             │
│      default_scopes                                  │
│    )                                                 │
│    registry.set_default_config(provider, config)     │
└────────────┬─────────────────────────────────────────┘
             │
             ▼
┌──────────────────────────────────────────────────────┐
│  Step 4: Runtime Usage                               │
│                                                       │
│  // Check if provider is available                   │
│  if registry.is_supported("strava") { ... }          │
│                                                       │
│  // List all available providers                     │
│  let providers = registry.supported_providers();     │
│  // ["strava", "garmin", "fitbit", "synthetic",      │
│  //  "whoop", "polar", ...]                          │
│                                                       │
│  // Create provider instance                         │
│  let provider = registry.create_provider("strava");  │
│                                                       │
│  // Use provider through FitnessProvider trait       │
│  let activities = provider.get_activities(...).await;│
└──────────────────────────────────────────────────────┘
```

## how providers are registered

### example: registering strava (built-in)

**Location**: `src/providers/registry.rs:71-94`

```rust
impl ProviderRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
            default_configs: HashMap::new(),
        };

        // 1. Register factory
        registry.register_factory(
            oauth_providers::STRAVA,  // "strava"
            Box::new(StravaProviderFactory),
        );

        // 2. Load environment configuration
        let (_client_id, _client_secret, auth_url, token_url,
             api_base_url, revoke_url, scopes) =
            crate::config::environment::load_provider_env_config(
                oauth_providers::STRAVA,
                "https://www.strava.com/oauth/authorize",
                "https://www.strava.com/oauth/token",
                "https://www.strava.com/api/v3",
                Some("https://www.strava.com/oauth/deauthorize"),
                &[oauth_providers::STRAVA_DEFAULT_SCOPES.to_owned()],
            );

        // 3. Set default configuration
        registry.set_default_config(
            oauth_providers::STRAVA,
            ProviderConfig {
                name: oauth_providers::STRAVA.to_owned(),
                auth_url,
                token_url,
                api_base_url,
                revoke_url,
                default_scopes: scopes,
            },
        );

        // Repeat for Garmin, Fitbit, Synthetic, etc.
        // ...

        registry
    }
}
```

### example: registering custom provider (whoop)

**Location**: `src/providers/registry.rs` (add to `new()` method)

```rust
// Register Whoop provider
registry.register_factory(
    "whoop",
    Box::new(WhoopProviderFactory),
);

let (_, _, auth_url, token_url, api_base_url, revoke_url, scopes) =
    crate::config::environment::load_provider_env_config(
        "whoop",
        "https://api.prod.whoop.com/oauth/authorize",
        "https://api.prod.whoop.com/oauth/token",
        "https://api.prod.whoop.com/developer/v1",
        Some("https://api.prod.whoop.com/oauth/revoke"),
        &["read:workout".to_owned(), "read:profile".to_owned()],
    );

registry.set_default_config(
    "whoop",
    ProviderConfig {
        name: "whoop".to_owned(),
        auth_url,
        token_url,
        api_base_url,
        revoke_url,
        default_scopes: scopes,
    },
);
```

**That's it!** Whoop is now registered and available alongside Strava, Garmin, and others.

## environment variables for 1 to x providers

Pierre supports **unlimited providers simultaneously**. Just set environment variables for each:

```bash
# Default provider (required)
export PIERRE_DEFAULT_PROVIDER=strava

# Provider 1: Strava
export PIERRE_STRAVA_CLIENT_ID=abc123
export PIERRE_STRAVA_CLIENT_SECRET=secret123

# Provider 2: Garmin
export PIERRE_GARMIN_CLIENT_ID=xyz789
export PIERRE_GARMIN_CLIENT_SECRET=secret789

# Provider 3: Fitbit
export PIERRE_FITBIT_CLIENT_ID=fitbit123
export PIERRE_FITBIT_CLIENT_SECRET=fitbit_secret

# Provider 4: Synthetic (no credentials needed!)
# Automatically available - no env vars required

# Provider 5: Custom Whoop
export PIERRE_WHOOP_CLIENT_ID=whoop_client
export PIERRE_WHOOP_CLIENT_SECRET=whoop_secret

# Provider 6: Custom Polar
export PIERRE_POLAR_CLIENT_ID=polar_client
export PIERRE_POLAR_CLIENT_SECRET=polar_secret

# ... unlimited providers ...
```

## dynamic discovery at runtime

Tools automatically discover all registered providers:

### connection status for all providers

**Request**:
```json
{
  "method": "tools/call",
  "params": {
    "name": "get_connection_status"
  }
}
```

**Response** (discovers all 1 to x providers):
```json
{
  "success": true,
  "result": {
    "providers": {
      "strava": { "connected": true, "status": "connected" },
      "garmin": { "connected": true, "status": "connected" },
      "fitbit": { "connected": false, "status": "disconnected" },
      "synthetic": { "connected": true, "status": "connected" },
      "whoop": { "connected": true, "status": "connected" },
      "polar": { "connected": false, "status": "disconnected" }
    }
  }
}
```

**Implementation** (`src/protocols/universal/handlers/connections.rs:84-110`):
```rust
// Multi-provider mode - check all supported providers from registry
let providers_to_check = executor.resources.provider_registry.supported_providers();
let mut providers_status = serde_json::Map::new();

for provider in providers_to_check {
    let is_connected = matches!(
        executor
            .auth_service
            .get_valid_token(user_uuid, provider, request.tenant_id.as_deref())
            .await,
        Ok(Some(_))
    );

    providers_status.insert(
        provider.to_owned(),
        serde_json::json!({
            "connected": is_connected,
            "status": if is_connected { "connected" } else { "disconnected" }
        }),
    );
}
```

**Key benefit**: No hardcoded provider lists! Add/remove providers without changing tool code.

### dynamic error messages

**Request** (invalid provider):
```json
{
  "method": "tools/call",
  "params": {
    "name": "connect_provider",
    "arguments": {
      "provider": "unknown_provider"
    }
  }
}
```

**Response** (automatically lists all registered providers):
```json
{
  "success": false,
  "error": "Provider 'unknown_provider' is not supported. Supported providers: strava, garmin, fitbit, synthetic, whoop, polar"
}
```

**Implementation** (`src/protocols/universal/handlers/connections.rs:332-340`):
```rust
if !is_provider_supported(provider, &executor.resources.provider_registry) {
    let supported_providers = executor
        .resources
        .provider_registry
        .supported_providers()
        .join(", ");
    return Ok(connection_error(format!(
        "Provider '{provider}' is not supported. Supported providers: {supported_providers}"
    )));
}
```

## provider factory implementations

Each provider implements `ProviderFactory`:

### strava factory

```rust
struct StravaProviderFactory;

impl ProviderFactory for StravaProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(StravaProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["strava"]
    }
}
```

### synthetic factory (oauth-free!)

```rust
struct SyntheticProviderFactory;

impl ProviderFactory for SyntheticProviderFactory {
    fn create(&self, _config: ProviderConfig) -> Box<dyn FitnessProvider> {
        // Ignores config - generates synthetic data
        Box::new(SyntheticProvider::default())
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["synthetic"]
    }
}
```

### custom whoop factory (example)

```rust
pub struct WhoopProviderFactory;

impl ProviderFactory for WhoopProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(WhoopProvider::new(config))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["whoop"]
    }
}
```

## simultaneous multi-provider usage

Users can connect to **all providers simultaneously** and aggregate data:

### example: aggregating activities from all connected providers

```rust
pub async fn get_all_activities_from_all_providers(
    user_id: Uuid,
    tenant_id: Uuid,
    registry: &ProviderRegistry,
    auth_service: &AuthService,
) -> Vec<Activity> {
    let mut all_activities = Vec::new();

    // Iterate through all registered providers
    for provider_name in registry.supported_providers() {
        // Check if user is connected to this provider
        if let Ok(Some(credentials)) = auth_service
            .get_valid_token(user_id, &provider_name, Some(&tenant_id.to_string()))
            .await
        {
            // Create provider instance
            if let Some(provider) = registry.create_provider(&provider_name) {
                // Set credentials
                if provider.set_credentials(credentials).await.is_ok() {
                    // Fetch activities
                    if let Ok(activities) = provider.get_activities(Some(50), None).await {
                        all_activities.extend(activities);
                    }
                }
            }
        }
    }

    // Sort by date (most recent first)
    all_activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));

    // Deduplicate if needed (same activity synced to multiple providers)
    all_activities
}
```

**Result**: Activities from Strava, Garmin, Fitbit, Whoop, Polar all in one unified list!

## configuration best practices

### development (single provider)
```bash
# Use synthetic provider - no OAuth needed
export PIERRE_DEFAULT_PROVIDER=synthetic
```

### production (multi-provider deployment)
```bash
# Default to strava
export PIERRE_DEFAULT_PROVIDER=strava

# Configure all active providers
export PIERRE_STRAVA_CLIENT_ID=${STRAVA_CLIENT_ID_SECRET}
export PIERRE_STRAVA_CLIENT_SECRET=${STRAVA_SECRET}

export PIERRE_GARMIN_CLIENT_ID=${GARMIN_KEY}
export PIERRE_GARMIN_CLIENT_SECRET=${GARMIN_SECRET}

export PIERRE_FITBIT_CLIENT_ID=${FITBIT_KEY}
export PIERRE_FITBIT_CLIENT_SECRET=${FITBIT_SECRET}
```

### testing (mix synthetic + real)
```bash
# Test with both synthetic and real provider
export PIERRE_DEFAULT_PROVIDER=synthetic
export PIERRE_STRAVA_CLIENT_ID=test_id
export PIERRE_STRAVA_CLIENT_SECRET=test_secret
```

## summary

**1 to x providers simultaneously**:
- ✅ Register unlimited providers via factory pattern
- ✅ Each provider independently configured via environment variables
- ✅ Runtime discovery via `supported_providers()` and `is_supported()`
- ✅ Zero code changes to add/remove providers
- ✅ Tools automatically adapt to available providers
- ✅ Users can connect to all providers at once
- ✅ Data aggregation across multiple providers
- ✅ Synthetic provider for OAuth-free development

**Key files**:
- `src/providers/registry.rs` - Central registry managing all providers
- `src/providers/core.rs` - `FitnessProvider` trait and `ProviderFactory` trait
- `src/config/environment.rs` - Environment-based configuration loading
- `src/protocols/universal/handlers/connections.rs` - Dynamic provider discovery

For detailed implementation guide, see Chapter 17.5: Pluggable Provider Architecture.

---

# LLM Provider Integration

This document describes Pierre's LLM (Large Language Model) provider abstraction layer, which enables pluggable AI model integration with streaming support for the chat functionality.

## Overview

The LLM module provides a trait-based abstraction that allows Pierre to integrate with multiple AI providers (Gemini, OpenAI, Ollama, etc.) through a unified interface. The design mirrors the fitness provider SPI pattern for consistency.

```
┌─────────────────────────────────────────────────────────────────┐
│                    LlmProviderRegistry                          │
│              Manages multiple LLM providers                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
         ┌───────────────────┼───────────────────┐
         │                   │                   │
         ▼                   ▼                   ▼
   ┌───────────┐      ┌───────────┐      ┌───────────┐
   │  Gemini   │      │  OpenAI   │      │  Ollama   │
   │ Provider  │      │ Provider  │      │ Provider  │
   └─────┬─────┘      └─────┬─────┘      └─────┬─────┘
         │                  │                   │
         └──────────────────┴───────────────────┘
                           │
                           ▼
               ┌───────────────────────┐
               │   LlmProvider Trait   │
               │   (shared interface)  │
               └───────────────────────┘
```

## Configuration

### Environment Variables

| Variable | Description | Required |
|----------|-------------|----------|
| `GEMINI_API_KEY` | Google Gemini API key | Yes (for Gemini) |

### Supported Models

#### Gemini (Default Provider)

| Model | Description | Default |
|-------|-------------|---------|
| `gemini-2.0-flash-exp` | Latest experimental flash model | ✓ |
| `gemini-1.5-pro` | Production-ready pro model | |
| `gemini-1.5-flash` | Fast, efficient model | |
| `gemini-1.0-pro` | Legacy pro model | |

## Quick Start

### Basic Usage

```rust
use pierre_mcp_server::llm::{
    GeminiProvider, LlmProvider, ChatMessage, ChatRequest,
};

// Create provider from environment variable
let provider = GeminiProvider::from_env()?;

// Build a chat request
let request = ChatRequest::new(vec![
    ChatMessage::system("You are a helpful fitness assistant."),
    ChatMessage::user("What's a good warm-up routine?"),
])
.with_temperature(0.7)
.with_max_tokens(1000);

// Get a response
let response = provider.complete(&request).await?;
println!("{}", response.content);
```

### Streaming Responses

```rust
use futures_util::StreamExt;

let request = ChatRequest::new(vec![
    ChatMessage::user("Explain the benefits of interval training"),
])
.with_streaming();

let mut stream = provider.complete_stream(&request).await?;

while let Some(chunk) = stream.next().await {
    match chunk {
        Ok(chunk) => {
            print!("{}", chunk.delta);
            if chunk.is_final {
                println!("\n[Done]");
            }
        }
        Err(e) => eprintln!("Error: {e}"),
    }
}
```

## API Reference

### LlmCapabilities

Bitflags indicating provider features:

| Flag | Description |
|------|-------------|
| `STREAMING` | Supports streaming responses |
| `FUNCTION_CALLING` | Supports function/tool calling |
| `VISION` | Supports image input |
| `JSON_MODE` | Supports structured JSON output |
| `SYSTEM_MESSAGES` | Supports system role messages |

```rust
// Check capabilities
let caps = provider.capabilities();
if caps.supports_streaming() {
    // Use streaming API
}
```

### ChatMessage

Message structure for conversations:

```rust
// Constructor methods
let system = ChatMessage::system("You are helpful");
let user = ChatMessage::user("Hello!");
let assistant = ChatMessage::assistant("Hi there!");
```

### ChatRequest

Request configuration with builder pattern:

```rust
let request = ChatRequest::new(messages)
    .with_model("gemini-1.5-pro")    // Override default model
    .with_temperature(0.7)            // 0.0 to 1.0
    .with_max_tokens(2000)            // Max output tokens
    .with_streaming();                // Enable streaming
```

### ChatResponse

Response structure:

| Field | Type | Description |
|-------|------|-------------|
| `content` | `String` | Generated text |
| `model` | `String` | Model used |
| `usage` | `Option<TokenUsage>` | Token counts |
| `finish_reason` | `Option<String>` | Why generation stopped |

### StreamChunk

Streaming chunk structure:

| Field | Type | Description |
|-------|------|-------------|
| `delta` | `String` | Incremental text |
| `is_final` | `bool` | Whether this is the last chunk |
| `finish_reason` | `Option<String>` | Reason if final |

## Provider Registry

The `LlmProviderRegistry` manages multiple providers:

```rust
use pierre_mcp_server::llm::LlmProviderRegistry;

let mut registry = LlmProviderRegistry::new();

// Register providers
registry.register(Box::new(GeminiProvider::from_env()?));
// registry.register(Box::new(OpenAIProvider::from_env()?));

// Set default
registry.set_default("gemini")?;

// Get provider by name
let provider = registry.get("gemini");

// List all registered
let names: Vec<&str> = registry.list();
```

## Adding New Providers

To implement a new LLM provider:

1. **Implement the trait**:

```rust
use async_trait::async_trait;
use pierre_mcp_server::llm::{
    LlmProvider, LlmCapabilities, ChatRequest, ChatResponse,
    ChatStream, AppError,
};

pub struct MyProvider {
    api_key: String,
    // ...
}

#[async_trait]
impl LlmProvider for MyProvider {
    fn name(&self) -> &'static str {
        "myprovider"
    }

    fn display_name(&self) -> &'static str {
        "My Custom Provider"
    }

    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::STREAMING | LlmCapabilities::SYSTEM_MESSAGES
    }

    fn default_model(&self) -> &'static str {
        "my-model-v1"
    }

    fn available_models(&self) -> &'static [&'static str] {
        &["my-model-v1", "my-model-v2"]
    }

    async fn complete(&self, request: &ChatRequest) -> Result<ChatResponse, AppError> {
        // Implementation
    }

    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        // Implementation
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        // Implementation
    }
}
```

2. **Register the provider**:

```rust
registry.register(Box::new(MyProvider::new(api_key)));
```

## Error Handling

All provider methods return `Result<T, AppError>`:

```rust
match provider.complete(&request).await {
    Ok(response) => println!("{}", response.content),
    Err(AppError { code, message, .. }) => {
        match code {
            ErrorCode::RateLimitExceeded => // Handle rate limit
            ErrorCode::AuthenticationFailed => // Handle auth error
            _ => // Handle other errors
        }
    }
}
```

## Testing

Run LLM-specific tests:

```bash
# Unit tests
cargo test --test llm_test

# With output
cargo test --test llm_test -- --nocapture
```

## See Also

- Chapter 26: LLM Provider Architecture
- Configuration Guide
- Error Reference

---

