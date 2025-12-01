<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 16: OAuth 2.0 Client for Fitness Providers

This chapter explores how Pierre acts as an OAuth 2.0 client to connect to fitness providers like Strava and Fitbit. You'll learn about the OAuth client implementation, PKCE generation, token management, and provider-specific integrations.

## What You'll Learn

- OAuth 2.0 client implementation for fitness providers
- PKCE generation with SHA-256
- Authorization URL construction
- Token exchange and refresh
- Provider-specific OAuth flows (Strava, Fitbit)
- Token expiration and renewal
- Tenant-aware OAuth clients
- Rate limiting and usage tracking

## OAuth Client Architecture

Pierre implements a generic OAuth 2.0 client that works with multiple fitness providers:

```
┌──────────────┐                  ┌──────────────┐                  ┌──────────────┐
│   Pierre     │                  │   Fitness    │                  │    User      │
│   Server     │                  │   Provider   │                  │   Browser    │
│              │                  │  (Strava)    │                  │              │
└──────────────┘                  └──────────────┘                  └──────────────┘
        │                                 │                                 │
        │  1. Generate PKCE params        │                                 │
        │    (verifier + challenge)       │                                 │
        ├─────────────────────────────────┼────────────────────────────────►│
        │                                 │                                 │
        │  2. Build authorization URL     │                                 │
        │     with code_challenge         │                                 │
        ├─────────────────────────────────┼────────────────────────────────►│
        │                                 │                                 │
        │                                 │  3. User authorizes Pierre      │
        │                                 │◄────────────────────────────────┤
        │                                 │                                 │
        │  4. OAuth callback              │                                 │
        │◄────────────────────────────────┼─────────────────────────────────┤
        │     with authorization code     │                                 │
        │                                 │                                 │
        │  5. POST /oauth/token           │                                 │
        │     (code + code_verifier)      │                                 │
        ├────────────────────────────────►│                                 │
        │                                 │                                 │
        │  6. Access token + refresh token│                                 │
        │◄────────────────────────────────┤                                 │
        │                                 │                                 │
        │  7. Store tokens in database    │                                 │
        │                                 │                                 │
```

**Client role**: Pierre initiates OAuth flows with fitness providers to access user data.

## OAuth Client Configuration

Each OAuth client needs provider-specific configuration:

**Source**: src/oauth2_client/client.rs:16-33
```rust
/// OAuth 2.0 client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Config {
    /// OAuth client ID from provider
    pub client_id: String,
    /// OAuth client secret from provider
    pub client_secret: String,
    /// Authorization endpoint URL
    pub auth_url: String,
    /// Token endpoint URL
    pub token_url: String,
    /// Redirect URI for OAuth callbacks
    pub redirect_uri: String,
    /// OAuth scopes to request
    pub scopes: Vec<String>,
    /// Whether to use PKCE for enhanced security
    pub use_pkce: bool,
}
```

**Configuration fields**:
- `client_id`/`client_secret`: Provider application credentials
- `auth_url`: Provider's authorization endpoint (e.g., `https://www.strava.com/oauth/authorize`)
- `token_url`: Provider's token endpoint (e.g., `https://www.strava.com/oauth/token`)
- `redirect_uri`: Pierre's callback URL (e.g., `http://localhost:8081/api/oauth/callback/strava`)
- `scopes`: Requested permissions (e.g., `["activity:read_all", "profile:read"]`)
- `use_pkce`: Enable PKCE for security (recommended)

## PKCE Parameter Generation

Pierre generates PKCE parameters to protect authorization codes:

**Source**: src/oauth2_client/client.rs:35-70
```rust
/// `PKCE` (Proof Key for Code Exchange) parameters for enhanced `OAuth2` security
#[derive(Debug, Clone)]
pub struct PkceParams {
    /// Randomly generated code verifier (43-128 characters)
    pub code_verifier: String,
    /// SHA256 hash of code verifier, base64url encoded
    pub code_challenge: String,
    /// Challenge method (always "S256" for SHA256)
    pub code_challenge_method: String,
}

impl PkceParams {
    /// Generate `PKCE` parameters with `S256` challenge method
    #[must_use]
    pub fn generate() -> Self {
        // Generate a cryptographically secure random code verifier (43-128 characters)
        const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
        let mut rng = rand::thread_rng();
        let code_verifier: String = (0
            ..crate::constants::network_config::OAUTH_CODE_VERIFIER_LENGTH)
            .map(|_| CHARS[rng.gen_range(0..CHARS.len())] as char)
            .collect();

        // Create S256 code challenge
        let mut hasher = Sha256::new();
        hasher.update(code_verifier.as_bytes());
        let hash = hasher.finalize();
        let code_challenge = URL_SAFE_NO_PAD.encode(hash);

        Self {
            code_verifier,
            code_challenge,
            code_challenge_method: "S256".into(),
        }
    }
}
```

**PKCE generation steps**:
1. **Generate verifier**: Random 43-128 character string from allowed charset
2. **Hash verifier**: SHA-256 hash of verifier bytes
3. **Base64url encode**: URL-safe base64 encoding without padding
4. **Return params**: Verifier (kept secret) and challenge (sent to provider)

## Rust Idioms: Base64url Encoding

**Source**: src/oauth2_client/client.rs:9
```rust
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
```

**Usage**:
```rust
let code_challenge = URL_SAFE_NO_PAD.encode(hash);
```

**Why URL_SAFE_NO_PAD**:
- **URL-safe**: Uses `-` and `_` instead of `+` and `/` (safe in query parameters)
- **No padding**: Omits trailing `=` characters (RFC 7636 requirement)
- **Standard compliant**: Matches OAuth 2.0 PKCE specification

## Authorization URL Construction

The client builds authorization URLs for user consent:

**Source**: src/oauth2_client/client.rs:149-177
```rust
/// Get authorization `URL` with `PKCE` support
///
/// # Errors
///
/// Returns an error if the authorization URL is malformed
pub fn get_authorization_url_with_pkce(
    &self,
    state: &str,
    pkce: &PkceParams,
) -> Result<String> {
    let mut url = Url::parse(&self.config.auth_url).context("Invalid auth URL")?;

    let mut query_pairs = url.query_pairs_mut();
    query_pairs
        .append_pair("client_id", &self.config.client_id)
        .append_pair("redirect_uri", &self.config.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &self.config.scopes.join(" "))
        .append_pair("state", state);

    if self.config.use_pkce {
        query_pairs
            .append_pair("code_challenge", &pkce.code_challenge)
            .append_pair("code_challenge_method", &pkce.code_challenge_method);
    }

    drop(query_pairs);
    Ok(url.to_string())
}
```

**Generated URL example**:
```
https://www.strava.com/oauth/authorize?
  client_id=12345&
  redirect_uri=http://localhost:8081/api/oauth/callback/strava&
  response_type=code&
  scope=activity:read_all%20profile:read&
  state=abc123&
  code_challenge=E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM&
  code_challenge_method=S256
```

**Query parameters**:
- `response_type=code`: Authorization code flow
- `scope`: Space-separated permissions
- `state`: CSRF protection token
- `code_challenge`/`code_challenge_method`: PKCE security

## OAuth Token Structure

The client handles OAuth tokens with expiration tracking:

**Source**: src/oauth2_client/client.rs:72-101
```rust
/// OAuth 2.0 access token with expiration and refresh capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Token {
    /// The access token string
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiration timestamp (UTC)
    pub expires_at: Option<DateTime<Utc>>,
    /// Optional refresh token for getting new access tokens
    pub refresh_token: Option<String>,
    /// Granted OAuth scopes
    pub scope: Option<String>,
}

impl OAuth2Token {
    /// Check if the token is expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now())
    }

    /// Check if the token will expire within 5 minutes
    #[must_use]
    pub fn will_expire_soon(&self) -> bool {
        self.expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now() + Duration::minutes(5))
    }
}
```

**Expiration logic**:
- `is_expired()`: Token expired (Utc::now() >= expires_at)
- `will_expire_soon()`: Token expires within 5 minutes (proactive refresh)

## Rust Idioms: Option::is_some_and

**Source**: src/oauth2_client/client.rs:90-93
```rust
pub fn is_expired(&self) -> bool {
    self.expires_at
        .is_some_and(|expires_at| expires_at <= Utc::now())
}
```

**Idiom**: `Option::is_some_and(predicate)` combines `is_some()` and predicate check in one operation.

**Equivalent verbose code**:
```rust
// Less idiomatic:
self.expires_at.is_some() && self.expires_at.unwrap() <= Utc::now()

// Idiomatic:
self.expires_at.is_some_and(|expires_at| expires_at <= Utc::now())
```

**Benefits**:
- **No unwrap**: Predicate only called if Some
- **Concise**: Single method call instead of chaining
- **Clear intent**: "check if some AND condition holds"

## Token Exchange

The client exchanges authorization codes for access tokens:

**Source**: src/oauth2_client/client.rs:205-237
```rust
/// Exchange authorization code with `PKCE` support
///
/// # Errors
///
/// Returns an error if the token exchange request fails or response is invalid
pub async fn exchange_code_with_pkce(
    &self,
    code: &str,
    pkce: &PkceParams,
) -> Result<OAuth2Token> {
    let mut params = vec![
        ("client_id", self.config.client_id.as_str()),
        ("client_secret", self.config.client_secret.as_str()),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", self.config.redirect_uri.as_str()),
    ];

    if self.config.use_pkce {
        params.push(("code_verifier", &pkce.code_verifier));
    }

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

**Token exchange flow**:
1. **Build form params**: Client credentials, auth code, grant type, redirect URI
2. **Add PKCE verifier**: Include `code_verifier` if PKCE enabled
3. **POST to token endpoint**: Send form-encoded request
4. **Parse response**: Extract access token, refresh token, expiration
5. **Return OAuth2Token**: Structured token with expiration tracking

## Token Refresh

The client refreshes expired tokens automatically:

**Source**: src/oauth2_client/client.rs:239-262 (conceptual)
```rust
/// Refresh an expired access token
///
/// # Errors
///
/// Returns an error if the token refresh request fails or response is invalid
pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuth2Token> {
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

**Refresh flow**:
1. **Use refresh token**: Include `refresh_token` from previous response
2. **Grant type**: `refresh_token` instead of `authorization_code`
3. **New access token**: Provider issues fresh access token
4. **Update storage**: Replace old token in database

## Provider-Specific Clients

Pierre includes specialized clients for Strava and Fitbit:

**Strava token exchange** (src/oauth2_client/client.rs:372-395):
```rust
/// Exchange Strava authorization code with `PKCE` support
pub async fn exchange_strava_code_with_pkce(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    pkce: &PkceParams,
) -> Result<(OAuth2Token, serde_json::Value)> {
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("code_verifier", &pkce.code_verifier),
    ];

    let client = oauth_client();
    let response: TokenResponse = client
        .post("https://www.strava.com/oauth/token")
        .form(&params)
        .send()
        .await?
        .json()
        .await?;

    // Strava returns athlete data with token response
    let token = OAuth2Client::token_from_response(response.clone());
    let athlete = response.athlete.unwrap_or_default();

    Ok((token, athlete))
}
```

**Strava specifics**:
- **Athlete data**: Strava returns athlete profile with token response
- **Hardcoded endpoint**: `https://www.strava.com/oauth/token`
- **PKCE support**: Strava supports code_verifier parameter

**Fitbit token exchange** (src/oauth2_client/client.rs:522-545):
```rust
/// Exchange Fitbit authorization code with `PKCE` support
pub async fn exchange_fitbit_code_with_pkce(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
    pkce: &PkceParams,
) -> Result<(OAuth2Token, serde_json::Value)> {
    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("code", code),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
        ("code_verifier", &pkce.code_verifier),
    ];

    let client = oauth_client();
    let response: TokenResponse = client
        .post("https://api.fitbit.com/oauth2/token")
        .form(&params)
        .send()
        .await?
        .json()
        .await?;

    let token = OAuth2Client::token_from_response(response);
    Ok((token, serde_json::json!({})))
}
```

**Fitbit specifics**:
- **Redirect URI required**: Fitbit validates redirect_uri in token request
- **No user data**: Fitbit doesn't return user profile with token response
- **Hardcoded endpoint**: `https://api.fitbit.com/oauth2/token`

## Tenant-Aware OAuth Client

Pierre wraps the generic OAuth client with tenant-specific rate limiting:

**Source**: src/tenant/oauth_client.rs:36-49
```rust
/// Tenant-aware OAuth client with credential isolation and rate limiting
pub struct TenantOAuthClient {
    /// Shared OAuth manager instance for handling tenant-specific OAuth operations
    pub oauth_manager: Arc<Mutex<TenantOAuthManager>>,
}

impl TenantOAuthClient {
    /// Create new tenant OAuth client with provided manager
    #[must_use]
    pub fn new(oauth_manager: TenantOAuthManager) -> Self {
        Self {
            oauth_manager: Arc::new(Mutex::new(oauth_manager)),
        }
    }
}
```

**Get OAuth client with rate limiting**:

**Source**: src/tenant/oauth_client.rs:59-93
```rust
/// Get `OAuth2Client` configured for specific tenant and provider
///
/// # Errors
///
/// Returns an error if:
/// - Tenant exceeds daily rate limit for the provider
/// - No OAuth credentials configured for tenant and provider
/// - OAuth configuration creation fails
pub async fn get_oauth_client(
    &self,
    tenant_context: &TenantContext,
    provider: &str,
    database: &Database,
) -> Result<OAuth2Client> {
    // Check rate limit first
    let manager = self.oauth_manager.lock().await;
    let (current_usage, daily_limit) =
        manager.check_rate_limit(tenant_context.tenant_id, provider)?;

    if current_usage >= daily_limit {
        return Err(AppError::invalid_input(format!(
            "Tenant {} has exceeded daily rate limit for provider {}: {}/{}",
            tenant_context.tenant_id, provider, current_usage, daily_limit
        ))
        .into());
    }

    // Get tenant credentials
    let credentials = manager
        .get_credentials(tenant_context.tenant_id, provider, database)
        .await?;
    drop(manager);

    // Build OAuth2Config from tenant credentials
    let oauth_config = Self::build_oauth_config(&credentials, provider)?;

    Ok(OAuth2Client::new(oauth_config))
}
```

**Tenant isolation**:
1. **Rate limit check**: Enforce daily API call limits per tenant
2. **Credential lookup**: Tenant-specific OAuth app credentials
3. **OAuth client creation**: Generic client with tenant configuration
4. **Usage tracking**: Increment counter after successful operations

## OAuth Flow Integration

Providers use the tenant-aware OAuth client for authentication:

**Strava provider integration** (src/providers/strava.rs:220-237):
```rust
pub async fn exchange_code_with_pkce(
    &mut self,
    code: &str,
    redirect_uri: &str,
    pkce: &crate::oauth2_client::PkceParams,
) -> Result<(String, String)> {
    let credentials = self.oauth_manager.get_credentials(...).await?;

    let (token, athlete) = crate::oauth2_client::strava::exchange_strava_code_with_pkce(
        &credentials.client_id,
        &credentials.client_secret,
        code,
        redirect_uri,
        pkce,
    )
    .await?;

    // Store token in database
    self.store_token(&token).await?;

    Ok((token.access_token, athlete["id"].as_str().unwrap_or_default().to_owned()))
}
```

**Integration steps**:
1. **Get credentials**: Tenant-specific OAuth app credentials from manager
2. **Exchange code**: Call provider-specific token exchange function
3. **Store token**: Save access token and refresh token to database
4. **Return result**: Access token and user ID for subsequent API calls

## Key Takeaways

1. **Generic OAuth client**: Single `OAuth2Client` implementation works with all providers.

2. **PKCE mandatory**: All OAuth flows use SHA-256 PKCE for security.

3. **Provider specifics**: Strava/Fitbit have different response formats and endpoint URLs.

4. **Token expiration**: `will_expire_soon()` enables proactive token refresh (5-minute buffer).

5. **Tenant isolation**: Each tenant has separate OAuth credentials and rate limits.

6. **Rate limiting**: Daily API call limits prevent tenant abuse of provider APIs.

7. **Refresh tokens**: Long-lived refresh tokens avoid repeated user authorization.

8. **Base64url encoding**: URL-safe base64 without padding matches OAuth 2.0 spec.

9. **Option::is_some_and**: Idiomatic Rust for conditional checks on Option values.

10. **Credential fallback**: Tenant-specific credentials with server-level fallback for flexibility.

---

**Next Chapter**: [Chapter 17: Provider Data Models & Rate Limiting](./chapter-17-provider-models.md) - Learn how Pierre abstracts fitness provider APIs with unified interfaces and handles rate limiting across multiple providers.
