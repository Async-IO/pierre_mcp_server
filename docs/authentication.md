# authentication

Pierre supports multiple authentication methods for different use cases.

## authentication methods

| method | use case | header | endpoints |
|--------|----------|--------|-----------|
| jwt tokens | mcp clients, web apps | `Authorization: Bearer <token>` | all authenticated endpoints |
| api keys | a2a systems | `X-API-Key: <key>` | a2a endpoints |
| oauth2 | provider integration | varies | fitness provider apis |

## jwt authentication

### registration

```bash
curl -X POST http://localhost:8081/auth/register \
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

### login

```bash
curl -X POST http://localhost:8081/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'
```

Response includes jwt token. Store securely.

### using jwt tokens

Include in authorization header:
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://localhost:8081/mcp
```

### token expiry

Default: 24 hours (configurable via `JWT_EXPIRY_HOURS`)

Refresh before expiry:
```bash
curl -X POST http://localhost:8081/auth/refresh \
  -H "Authorization: Bearer <current_token>"
```

## api key authentication

For a2a systems and service-to-service communication.

### creating api keys

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

### using api keys

```bash
curl -H "X-API-Key: <api_key>" \
  http://localhost:8081/a2a/tools
```

### api key tiers

- `free`: 100 requests/day
- `professional`: 10,000 requests/day
- `enterprise`: unlimited

Rate limits enforced per tier.

## oauth2 (mcp client authentication)

Pierre acts as oauth2 authorization server for mcp clients.

### oauth2 vs oauth (terminology)

Pierre implements two oauth systems:

1. **oauth2_server module** (`src/oauth2_server/`): pierre AS oauth2 server
   - mcp clients authenticate TO pierre
   - rfc 7591 dynamic client registration
   - issues jwt access tokens

2. **oauth2_client module** (`src/oauth2_client/`): pierre AS oauth2 client
   - pierre authenticates TO fitness providers (strava, garmin, fitbit)
   - manages provider tokens
   - handles token refresh

### oauth2 flow (mcp clients)

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

### pkce enforcement

Pierre requires pkce (rfc 7636) for security:
- code verifier: 43-128 random characters
- code challenge: base64url(sha256(verifier))
- challenge method: S256 only

No plain text challenge methods allowed.

## mcp client integration (claude code, vs code, etc.)

mcp clients (claude code, vs code with cline/continue, cursor, etc.) connect to pierre via http-based mcp protocol.

### authentication flow

1. **user registration and login**:
```bash
# create user account
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'

# login to get jwt token
curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "email": "user@example.com",
    "password": "SecurePass123!"
  }'
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

### mcp endpoint authentication requirements

| endpoint | auth required | notes |
|----------|---------------|-------|
| `POST /mcp` (initialize) | no | discovery only |
| `POST /mcp` (tools/list) | no | unauthenticated tool listing |
| `POST /mcp` (tools/call) | yes | requires valid jwt |
| `POST /mcp` (prompts/list) | no | discovery only |
| `POST /mcp` (resources/list) | no | discovery only |

implementation: `src/mcp/multitenant.rs:1726`

### token expiry and refresh

jwt tokens expire after 24 hours (default, configurable via `JWT_EXPIRY_HOURS`).

when token expires, user must:
1. login again to get new jwt token
2. update claude code configuration with new token

automatic refresh not implemented in most mcp clients (requires manual re-login).

### connecting to fitness providers

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

### why no pierre login during strava oauth?

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

### security considerations

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

### troubleshooting

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

## provider oauth (fitness data)

Pierre acts as oauth client to fitness providers.

### supported providers

- strava (oauth2)
- garmin (oauth1 + oauth2)
- fitbit (oauth2)

### configuration

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

### connecting providers

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

### token storage

Provider tokens stored encrypted in database:
- encryption key: tenant-specific key (derived from master key)
- algorithm: aes-256-gcm
- rotation: automatic refresh before expiry

### checking connection status

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

## security features

### password hashing

- algorithm: argon2id (default) or bcrypt
- configurable work factor
- per-user salt

### token encryption

- jwt signing: rs256 asymmetric (rsa) or hs256 symmetric
  - rs256: 4096-bit rsa keys (production), 2048-bit (tests)
  - hs256: 64-byte secret (legacy)
- provider tokens: aes-256-gcm
- encryption keys: two-tier system
  - master key (env: `PIERRE_MASTER_ENCRYPTION_KEY`)
  - tenant keys (derived from master key)

### rs256/jwks

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

### rate limiting

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

### csrf protection

- state parameter in oauth flows
- pkce for oauth2 authorization
- origin validation for web requests

### atomic token operations

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

## troubleshooting

### "invalid token" errors

- check token expiry: jwt tokens expire after 24h (default)
- verify token format: must be `Bearer <token>`
- ensure token not revoked: check `/oauth/status`

### oauth2 flow fails

- verify redirect uri exactly matches registration
- check pkce challenge/verifier match
- ensure code not expired (10 min lifetime)

### provider oauth fails

- verify provider credentials (client_id, client_secret)
- check redirect uri accessible from browser
- ensure callback endpoint reachable

### api key rejected

- verify api key active: not deleted or expired
- check rate limits: may be throttled
- ensure correct header: `X-API-Key` (case-sensitive)

## implementation references

- jwt authentication: `src/auth.rs`
- api key management: `src/api_keys.rs`
- oauth2 server: `src/oauth2_server/`
- provider oauth: `src/oauth2_client/`
- encryption: `src/crypto/`, `src/key_management.rs`
- rate limiting: `src/rate_limiting.rs`
