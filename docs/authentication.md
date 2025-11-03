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
