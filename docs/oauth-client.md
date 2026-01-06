# OAuth Client (Fitness Providers)

Pierre acts as an oauth 2.0 client to connect to fitness providers (strava, fitbit, garmin, whoop, coros, terra) on behalf of users.

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
| coros | oauth 2.0 | required | active | `read:workouts`,`read:sleep`,`read:daily` | `src/providers/coros_provider.rs` |
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

**coros:**
```bash
export COROS_CLIENT_ID=your_client_id
export COROS_CLIENT_SECRET=your_client_secret
export COROS_REDIRECT_URI=http://localhost:8081/api/oauth/callback/coros  # dev
```

**production:** use https redirect urls:
```bash
export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
export FITBIT_REDIRECT_URI=https://api.example.com/api/oauth/callback/fitbit
export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
export WHOOP_REDIRECT_URI=https://api.example.com/api/oauth/callback/whoop
export COROS_REDIRECT_URI=https://api.example.com/api/oauth/callback/coros
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
- coros: 1000 requests/day per tenant

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

### COROS

**auth url:** `https://open.coros.com/oauth2/authorize` (placeholder - update when API docs received)
**token url:** `https://open.coros.com/oauth2/token` (placeholder - update when API docs received)
**api base:** `https://open.coros.com/api/v1` (placeholder - update when API docs received)

**note:** COROS API documentation is private. Apply for developer access at [COROS Developer Portal](https://support.coros.com/hc/en-us/articles/17085887816340).

**default scopes:** `read:workouts read:sleep read:daily`

**scope details:**
- `read:workouts` - workout/activity data
- `read:sleep` - sleep sessions and metrics
- `read:daily` - daily summaries (steps, heart rate, recovery)

**rate limits:**
- varies by endpoint
- standard api rate limiting applies

**token lifetime:**
- access token: varies (update when API docs received)
- refresh token: varies (update when API docs received)

Implementation: `src/providers/coros_provider.rs`

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

- [oauth2 server](oauth2-server.md) - mcp client authentication
- [authentication](authentication.md) - authentication methods and jwt tokens
- [configuration](configuration.md) - environment variables
