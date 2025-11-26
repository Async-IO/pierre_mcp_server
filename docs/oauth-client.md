# oauth client (fitness providers)

Pierre acts as an oauth 2.0 client to connect to fitness providers (strava, fitbit, garmin) on behalf of users.

## overview

**oauth2_client module** (`src/oauth2_client/`):
- pierre connects TO fitness providers as oauth client
- handles user authorization and token management
- supports pkce for enhanced security
- multi-tenant credential isolation

**separate from oauth2_server**:
- oauth2_server: mcp clients connect TO pierre
- oauth2_client: pierre connects TO fitness providers

## supported providers

| provider | oauth version | pkce | scopes | implementation |
|----------|--------------|------|--------|----------------|
| strava | oauth 2.0 | required | `activity:read_all` | `src/providers/strava.rs` |
| fitbit | oauth 2.0 | required | `activity`,`heartrate`,`location`,`nutrition`,`profile`,`settings`,`sleep`,`social`,`weight` | `src/providers/fitbit.rs` |
| garmin | oauth 2.0 | required | `wellness:read`,`activities:read` | `src/providers/garmin_provider.rs` |

Implementation: `src/oauth2_client/mod.rs`

## configuration

### environment variables

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

**production:** use https redirect urls:
```bash
export STRAVA_REDIRECT_URI=https://api.example.com/api/oauth/callback/strava
export FITBIT_REDIRECT_URI=https://api.example.com/api/oauth/callback/fitbit
export GARMIN_REDIRECT_URI=https://api.example.com/api/oauth/callback/garmin
```

Constants: `src/constants/oauth/providers.rs`

## multi-tenant architecture

### credential hierarchy

Credentials resolved in priority order:
1. **tenant-specific credentials** (database, encrypted)
2. **server-level credentials** (environment variables)

Implementation: `src/oauth2_client/tenant_client.rs`

### tenant oauth client

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

### storing tenant credentials

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

### rate limiting

**default limits** (`src/tenant/oauth_manager.rs`):
- strava: 1000 requests/day per tenant
- fitbit: 150 requests/day per tenant
- garmin: 1000 requests/day per tenant

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

## oauth flow

### step 1: initiate authorization

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

### step 2: user authorizes at provider

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

### step 3: oauth callback

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

### step 4: success page

User sees branded html page:
- provider name and connection status
- user identifier
- pierre logo
- instructions to return to mcp client

Template: `templates/oauth_success.html`
Renderer: `src/oauth2_client/flow_manager.rs:350-393`

## token management

### oauth2token structure

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

### storage

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

### automatic refresh

Pierre refreshes expired tokens before api requests:

**refresh criteria:**
- access token expired or expiring within 5 minutes
- refresh token available and valid

**refresh flow** (`src/oauth2_client/client.rs:178-230`):
```rust
pub async fn refresh_token(
    &self,
    refresh_token: &str,
    code_verifier: Option<&str>,
) -> Result<OAuth2Token> {
    let mut params = vec![
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", &self.config.client_id),
        ("client_secret", &self.config.client_secret),
    ];

    if let Some(verifier) = code_verifier {
        params.push(("code_verifier", verifier));
    }

    let response = self.client
        .post(&self.config.token_url)
        .form(&params)
        .send()
        .await?
        .json::<TokenResponse>()
        .await?;

    // Convert to OAuth2Token with expiry calculation
    Ok(OAuth2Token { ... })
}
```

### manual token operations

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

## connection status

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

## security features

### pkce (proof key for code exchange)

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

### state parameter validation

**state format:** `{user_id}:{random_uuid}`

**validation** (`src/oauth2_client/flow_manager.rs:162-215`):
1. Extract user_id from state
2. Verify user exists and belongs to tenant
3. Ensure state not reused (single-use)

Invalid state results in authorization rejection.

### token encryption

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

### tenant isolation

Oauth artifacts never shared between tenants:
- credentials stored per tenant_id
- tokens bound to user and tenant
- rate limits enforced per tenant
- database queries include tenant_id filter

Cross-tenant access prevented at database layer.

Implementation: `src/tenant/oauth_manager.rs`

## provider-specific details

### strava

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

### fitbit

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

### garmin

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

**status:** in development

Implementation: `src/providers/garmin_provider.rs`

## error handling

### authorization errors

Displayed on html error page (`templates/oauth_error.html`):

**common errors:**
- `access_denied` - user denied authorization
- `invalid_request` - missing or invalid parameters
- `invalid_scope` - requested scope not available
- `server_error` - provider api error

Renderer: `src/oauth2_client/flow_manager.rs:329-347`

### callback errors

Returned as query parameters:
```
http://localhost:8081/api/oauth/callback/strava?
  error=access_denied&
  error_description=User+declined+authorization
```

### token errors

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

## troubleshooting

### authorization fails

**symptom:** redirect to provider fails or returns error

**solutions:**
- verify provider credentials (client_id, client_secret)
- check redirect_uri matches provider configuration exactly
- ensure redirect_uri uses https in production
- confirm provider api credentials active and approved

### callback error: state validation failed

**symptom:** `invalid state parameter` error on callback

**solutions:**
- ensure user_id in authorization request matches authenticated user
- check user exists in database
- verify tenant association correct
- confirm no url encoding issues in state parameter

### token refresh fails

**symptom:** api requests fail with authentication error

**solutions:**
- check refresh token not expired or revoked
- verify provider credentials still valid
- ensure network connectivity to provider api
- re-authorize user to obtain new tokens

### rate limit exceeded

**symptom:** api requests rejected with rate limit error

**solutions:**
- check current usage via tenant_oauth_manager
- wait for daily reset (midnight utc)
- request rate limit increase from provider
- optimize api call patterns to reduce requests

### encryption key mismatch

**symptom:** cannot decrypt stored tokens

**solutions:**
- verify `PIERRE_MASTER_ENCRYPTION_KEY` unchanged
- check key is valid base64 (32 bytes decoded)
- ensure key not rotated without token re-encryption
- re-authorize users if key changed

## implementation references

- oauth2 client: `src/oauth2_client/client.rs`
- oauth flow manager: `src/oauth2_client/flow_manager.rs`
- tenant client: `src/oauth2_client/tenant_client.rs`
- tenant oauth manager: `src/tenant/oauth_manager.rs`
- provider implementations: `src/providers/`
- token storage: `src/database/tokens.rs`
- route handlers: `src/routes/auth.rs`
- templates: `templates/oauth_success.html`, `templates/oauth_error.html`

## see also

- [oauth2 server](oauth2-server.md) - mcp client authentication
- [authentication](authentication.md) - authentication methods and jwt tokens
- [configuration](configuration.md) - environment variables
