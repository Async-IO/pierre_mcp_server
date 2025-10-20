# authentication

pierre supports multiple authentication methods for different use cases.

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

response:
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

response includes jwt token. store securely.

### using jwt tokens

include in authorization header:
```bash
curl -H "Authorization: Bearer <jwt_token>" \
  http://localhost:8081/mcp
```

### token expiry

default: 24 hours (configurable via `JWT_EXPIRY_HOURS`)

refresh before expiry:
```bash
curl -X POST http://localhost:8081/auth/refresh \
  -H "Authorization: Bearer <current_token>"
```

## api key authentication

for a2a systems and service-to-service communication.

### creating api keys

requires admin or user jwt:
```bash
curl -X POST http://localhost:8081/api/keys \
  -H "Authorization: Bearer <jwt_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "My A2A System",
    "tier": "professional"
  }'
```

response:
```json
{
  "api_key": "generated_key",
  "name": "My A2A System",
  "tier": "professional",
  "created_at": "2024-01-01T00:00:00Z"
}
```

save api key - cannot be retrieved later.

### using api keys

```bash
curl -H "X-API-Key: <api_key>" \
  http://localhost:8081/a2a/tools
```

### api key tiers

- `free`: 100 requests/day
- `professional`: 10,000 requests/day
- `enterprise`: unlimited

rate limits enforced per tier.

## oauth2 (mcp client authentication)

pierre acts as oauth2 authorization server for mcp clients.

### oauth2 vs oauth (terminology)

pierre implements two oauth systems:

1. **oauth2 module** (`src/oauth2/`): pierre AS oauth2 server
   - mcp clients authenticate TO pierre
   - rfc 7591 dynamic client registration
   - issues jwt access tokens

2. **oauth module** (`src/oauth/`): pierre AS oauth2 client
   - pierre authenticates TO fitness providers (strava, garmin, fitbit)
   - manages provider tokens
   - handles token refresh

### oauth2 flow (mcp clients)

sdk handles automatically. manual flow:

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

receives jwt access token.

### pkce enforcement

pierre requires pkce (rfc 7636) for security:
- code verifier: 43-128 random characters
- code challenge: base64url(sha256(verifier))
- challenge method: S256 only

no plain text challenge methods allowed.

## provider oauth (fitness data)

pierre acts as oauth client to fitness providers.

### supported providers

- strava (oauth2)
- garmin (oauth1 + oauth2)
- fitbit (oauth2)

### configuration

set environment variables:
```bash
# strava (local development)
export STRAVA_CLIENT_ID=your_id
export STRAVA_CLIENT_SECRET=your_secret
export STRAVA_REDIRECT_URI=http://localhost:8081/oauth/callback/strava  # local dev only

# strava (production)
export STRAVA_REDIRECT_URI=https://api.example.com/oauth/callback/strava  # required

# garmin (local development)
export GARMIN_CLIENT_ID=your_key
export GARMIN_CLIENT_SECRET=your_secret
export GARMIN_REDIRECT_URI=http://localhost:8081/oauth/callback/garmin  # local dev only

# garmin (production)
export GARMIN_REDIRECT_URI=https://api.example.com/oauth/callback/garmin  # required
```

**callback url security requirements**:
- http urls: local development only (localhost/127.0.0.1)
- https urls: required for production deployments
- failure to use https in production:
  - authorization codes transmitted unencrypted
  - vulnerable to token interception
  - most providers reject http callbacks in production

### connecting providers

via mcp tool:
```
user: "connect to strava"
```

or via rest api:
```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/oauth/connect/strava
```

opens browser for provider authentication. after approval, redirected to callback:
```bash
# local development
http://localhost:8081/oauth/callback/strava?code=<auth_code>

# production (https required)
https://api.example.com/oauth/callback/strava?code=<auth_code>
```

pierre exchanges code for access/refresh tokens, stores encrypted.

**security**: authorization codes in callback urls must be protected with tls in production. http callbacks leak codes to network observers.

### token storage

provider tokens stored encrypted in database:
- encryption key: tenant-specific key (derived from master key)
- algorithm: aes-256-gcm
- rotation: automatic refresh before expiry

### checking connection status

```bash
curl -H "Authorization: Bearer <jwt>" \
  http://localhost:8081/oauth/status
```

response:
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

asymmetric signing for distributed token verification.

public keys available at `/admin/jwks` (legacy) and `/oauth2/jwks` (oauth2 clients):
```bash
curl http://localhost:8081/oauth2/jwks
```

response (rfc 7517 compliant):
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

clients verify tokens using public key. pierre signs with private key.

benefits:
- private key never leaves server
- clients verify without shared secret
- supports key rotation with grace period
- browser caching reduces jwks endpoint load

**key rotation**: when keys are rotated, old keys are retained during grace period to allow existing tokens to validate. new tokens are signed with the current key.

### rate limiting

token bucket algorithm per authentication method:
- jwt tokens: per-tenant limits
- api keys: per-tier limits (free: 100/day, professional: 10,000/day, enterprise: unlimited)
- oauth2 endpoints: per-ip limits
  - `/oauth2/authorize`: 60 requests/minute
  - `/oauth2/token`: 30 requests/minute
  - `/oauth2/register`: 10 requests/minute

oauth2 rate limit responses include:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 59
X-RateLimit-Reset: 1704067200
Retry-After: 42
```

implementation: `src/rate_limiting.rs`, `src/oauth2/rate_limiting.rs`

### csrf protection

- state parameter in oauth flows
- pkce for oauth2 authorization
- origin validation for web requests

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
- oauth2 server: `src/oauth2/`
- provider oauth: `src/oauth/`
- encryption: `src/crypto/`, `src/key_management.rs`
- rate limiting: `src/rate_limiting.rs`
