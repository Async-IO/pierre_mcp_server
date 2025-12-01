<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

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

- [authentication](authentication.md) - jwt and api key authentication
- [protocols](protocols.md) - fitness provider integrations
- [configuration](configuration.md) - server configuration
