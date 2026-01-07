<!-- SPDX-License-Identifier: MIT OR Apache-2.0 -->
<!-- Copyright (c) 2025 Pierre Fitness Intelligence -->

# Chapter 15: OAuth 2.0 Server Implementation

This chapter explores how Pierre implements a full OAuth 2.0 authorization server for secure MCP client authentication. You'll learn about RFC 7591 dynamic client registration, PKCE (RFC 7636), authorization code flow, and JWT-based access tokens.

## What You'll Learn

- OAuth 2.0 authorization server implementation
- RFC 7591 dynamic client registration
- RFC 7636 PKCE (Proof Key for Code Exchange)
- OAuth discovery endpoint (RFC 8414)
- Authorization code flow with redirect
- JWT as OAuth access tokens
- Argon2 for client secret hashing
- Constant-time credential validation
- Multi-tenant OAuth isolation

## OAuth 2.0 Server Architecture

Pierre implements a standards-compliant OAuth 2.0 authorization server:

```
┌──────────────┐                   ┌──────────────┐
│ MCP Client   │                   │   Pierre     │
│ (SDK)        │                   │   OAuth 2.0  │
│              │                   │   Server     │
└──────────────┘                   └──────────────┘
        │                                  │
        │  1. POST /oauth2/register        │
        │  (dynamic client registration)   │
        ├─────────────────────────────────►│
        │                                  │
        │  client_id, client_secret        │
        │◄─────────────────────────────────┤
        │                                  │
        │  2. GET /oauth2/authorize        │
        │  (with PKCE code_challenge)      │
        ├─────────────────────────────────►│
        │                                  │
        │  Redirect to login page          │
        │◄─────────────────────────────────┤
        │                                  │
        │  3. POST /oauth2/login           │
        │  (user credentials)              │
        ├─────────────────────────────────►│
        │                                  │
        │  Redirect with auth code         │
        │◄─────────────────────────────────┤
        │                                  │
        │  4. POST /oauth2/token           │
        │  (exchange code + verifier)      │
        ├─────────────────────────────────►│
        │                                  │
        │  access_token (JWT)              │
        │◄─────────────────────────────────┤
```

**OAuth 2.0 flow**: Pierre supports authorization code flow with PKCE (mandatory for security).

## OAuth Context and Routes

The OAuth server shares context across all endpoint handlers:

**Source**: src/routes/oauth2.rs:36-49
```rust
/// OAuth 2.0 server context shared across all handlers
#[derive(Clone)]
pub struct OAuth2Context {
    /// Database for client and token storage
    pub database: Arc<Database>,
    /// Authentication manager for JWT operations
    pub auth_manager: Arc<AuthManager>,
    /// JWKS manager for public key operations
    pub jwks_manager: Arc<JwksManager>,
    /// Server configuration
    pub config: Arc<ServerConfig>,
    /// Rate limiter for OAuth endpoints
    pub rate_limiter: Arc<OAuth2RateLimiter>,
}
```

**Route registration**:

**Source**: src/routes/oauth2.rs:69-97
```rust
impl OAuth2Routes {
    /// Create all OAuth 2.0 routes with context
    pub fn routes(context: OAuth2Context) -> Router {
        Router::new()
            // RFC 8414: OAuth 2.0 Authorization Server Metadata
            .route(
                "/.well-known/oauth-authorization-server",
                get(Self::handle_discovery),
            )
            // RFC 7517: JWKS endpoint
            .route("/.well-known/jwks.json", get(Self::handle_jwks))
            // RFC 7591: Dynamic Client Registration
            .route("/oauth2/register", post(Self::handle_client_registration))
            // OAuth 2.0 Authorization endpoint
            .route("/oauth2/authorize", get(Self::handle_authorization))
            // OAuth 2.0 Token endpoint
            .route("/oauth2/token", post(Self::handle_token))
            // Login page and submission
            .route("/oauth2/login", get(Self::handle_oauth_login_page))
            .route("/oauth2/login", post(Self::handle_oauth_login_submit))
            // Token validation endpoints
            .route(
                "/oauth2/validate-and-refresh",
                post(Self::handle_validate_and_refresh),
            )
            .route("/oauth2/token-validate", post(Self::handle_token_validate))
            .with_state(context)
    }
}
```

**Endpoints**:
- `/.well-known/oauth-authorization-server`: OAuth discovery (RFC 8414)
- `/.well-known/jwks.json`: Public keys for JWT verification
- `/oauth2/register`: Dynamic client registration (RFC 7591)
- `/oauth2/authorize`: Authorization endpoint (user consent)
- `/oauth2/token`: Token endpoint (code exchange)

## OAuth Discovery Endpoint

The discovery endpoint advertises server capabilities (RFC 8414):

**Source**: src/routes/oauth2.rs:100-128
```rust
/// Handle OAuth 2.0 discovery (RFC 8414)
async fn handle_discovery(State(context): State<OAuth2Context>) -> Json<serde_json::Value> {
    let issuer_url = context.config.oauth2_server.issuer_url.clone();

    // Use spawn_blocking for JSON serialization (CPU-bound operation)
    let discovery_json = tokio::task::spawn_blocking(move || {
        serde_json::json!({
            "issuer": issuer_url,
            "authorization_endpoint": format!("{issuer_url}/oauth2/authorize"),
            "token_endpoint": format!("{issuer_url}/oauth2/token"),
            "registration_endpoint": format!("{issuer_url}/oauth2/register"),
            "jwks_uri": format!("{issuer_url}/.well-known/jwks.json"),
            "grant_types_supported": ["authorization_code", "client_credentials", "refresh_token"],
            "response_types_supported": ["code"],
            "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
            "scopes_supported": ["fitness:read", "activities:read", "profile:read"],
            "response_modes_supported": ["query"],
            "code_challenge_methods_supported": ["S256"]
        })
    })
    .await
    .unwrap_or_else(|_| {
        serde_json::json!({
            "error": "internal_error",
            "error_description": "Failed to generate discovery document"
        })
    });

    Json(discovery_json)
}
```

**Discovery response** (example):
```json
{
  "issuer": "http://localhost:8081",
  "authorization_endpoint": "http://localhost:8081/oauth2/authorize",
  "token_endpoint": "http://localhost:8081/oauth2/token",
  "registration_endpoint": "http://localhost:8081/oauth2/register",
  "jwks_uri": "http://localhost:8081/.well-known/jwks.json",
  "grant_types_supported": ["authorization_code", "client_credentials", "refresh_token"],
  "response_types_supported": ["code"],
  "token_endpoint_auth_methods_supported": ["client_secret_post", "client_secret_basic"],
  "scopes_supported": ["fitness:read", "activities:read", "profile:read"],
  "response_modes_supported": ["query"],
  "code_challenge_methods_supported": ["S256"]
}
```

**Key fields**:
- `code_challenge_methods_supported: ["S256"]`: Only SHA-256 PKCE (no plain method for security)
- `grant_types_supported`: Authorization code, client credentials, refresh token
- `token_endpoint_auth_methods_supported`: Client authentication methods

## Dynamic Client Registration (rfc 7591)

MCP clients register dynamically to obtain OAuth credentials:

**Source**: src/oauth2_server/models.rs:11-26
```rust
/// OAuth 2.0 Client Registration Request (RFC 7591)
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    /// Redirect URIs for authorization code flow
    pub redirect_uris: Vec<String>,
    /// Optional client name for display
    pub client_name: Option<String>,
    /// Optional client URI for information
    pub client_uri: Option<String>,
    /// Grant types the client can use
    pub grant_types: Option<Vec<String>>,
    /// Response types the client can use
    pub response_types: Option<Vec<String>>,
    /// Scopes the client can request
    pub scope: Option<String>,
}
```

**Client registration handler**:

**Source**: src/oauth2_server/client_registration.rs:39-108
```rust
/// Register a new OAuth 2.0 client (RFC 7591)
///
/// # Errors
/// Returns an error if client registration validation fails or database storage fails
pub async fn register_client(
    &self,
    request: ClientRegistrationRequest,
) -> Result<ClientRegistrationResponse, OAuth2Error> {
    // Validate request
    Self::validate_registration_request(&request)?;

    // Generate client credentials
    let client_id = Self::generate_client_id();
    let client_secret = Self::generate_client_secret()?;
    let client_secret_hash = Self::hash_client_secret(&client_secret)?;

    // Set default values - only authorization_code by default for security (RFC 8252 best practices)
    // Clients must explicitly request client_credentials if needed
    let grant_types = request
        .grant_types
        .unwrap_or_else(|| vec!["authorization_code".to_owned()]);

    let response_types = request
        .response_types
        .unwrap_or_else(|| vec!["code".to_owned()]);

    let created_at = Utc::now();
    let expires_at = Some(created_at + Duration::days(365)); // 1 year expiry

    // Create client record
    let client = OAuth2Client {
        id: Uuid::new_v4().to_string(),
        client_id: client_id.clone(),
        client_secret_hash,
        redirect_uris: request.redirect_uris.clone(),
        grant_types: grant_types.clone(),
        response_types: response_types.clone(),
        client_name: request.client_name.clone(),
        client_uri: request.client_uri.clone(),
        scope: request.scope.clone(),
        created_at,
        expires_at,
    };

    // Store in database
    self.store_client(&client).await.map_err(|e| {
        tracing::error!(error = %e, client_id = %client_id, "Failed to store OAuth2 client registration in database");
        OAuth2Error::invalid_request("Failed to store client registration")
    })?;

    // Return registration response
    let default_client_uri = Self::get_default_client_uri();

    Ok(ClientRegistrationResponse {
        client_id,
        client_secret,
        client_id_issued_at: Some(created_at.timestamp()),
        client_secret_expires_at: expires_at.map(|dt| dt.timestamp()),
        redirect_uris: request.redirect_uris,
        grant_types,
        response_types,
        client_name: request.client_name,
        client_uri: request.client_uri.or(Some(default_client_uri)),
        scope: request
            .scope
            .or_else(|| Some("fitness:read activities:read profile:read".to_owned())),
    })
}
```

**Security measures**:
1. **Argon2 hashing**: Client secrets hashed before storage (never plaintext)
2. **365-day expiry**: Client registrations expire after 1 year
3. **Default grant types**: Only `authorization_code` by default (least privilege)
4. **Redirect URI validation**: URIs validated during registration

## Rust Idioms: Argon2 for Credential Hashing

Pierre uses Argon2 (winner of Password Hashing Competition) for client secret hashing:

**Conceptual implementation** (from client_registration.rs):
```rust
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

fn hash_client_secret(secret: &str) -> Result<String, OAuth2Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(secret.as_bytes(), &salt)
        .map_err(|e| OAuth2Error::invalid_request("Failed to hash client secret"))?;

    Ok(password_hash.to_string())
}
```

**Why Argon2**:
- **Memory-hard**: Resistant to GPU/ASIC attacks
- **Tunable**: Adjustable time/memory cost parameters
- **Winner of PHC**: Industry-standard recommendation
- **Constant-time**: Safe against timing attacks

## Authorization Endpoint with PKCE

The authorization endpoint requires PKCE (Proof Key for Code Exchange) for security:

**Source**: src/oauth2_server/endpoints.rs:70-156
```rust
/// Handle authorization request (GET /oauth/authorize)
///
/// # Errors
/// Returns an error if client validation fails, invalid parameters, or authorization code generation fails
pub async fn authorize(
    &self,
    request: AuthorizeRequest,
    user_id: Option<Uuid>,     // From authentication
    tenant_id: Option<String>, // From JWT claims
) -> Result<AuthorizeResponse, OAuth2Error> {
    // Validate client
    let client = self
        .client_manager
        .get_client(&request.client_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Client lookup failed for client_id={}: {:#}",
                request.client_id,
                e
            );
            OAuth2Error::invalid_client()
        })?;

    // Validate response type
    if request.response_type != "code" {
        return Err(OAuth2Error::invalid_request(
            "Only 'code' response_type is supported",
        ));
    }

    // Validate redirect URI
    if !client.redirect_uris.contains(&request.redirect_uri) {
        return Err(OAuth2Error::invalid_request("Invalid redirect_uri"));
    }

    // Validate PKCE parameters (RFC 7636)
    if let Some(ref code_challenge) = request.code_challenge {
        // Validate code_challenge format (base64url-encoded, 43-128 characters)
        if code_challenge.len() < 43 || code_challenge.len() > 128 {
            return Err(OAuth2Error::invalid_request(
                "code_challenge must be between 43 and 128 characters",
            ));
        }

        // Validate code_challenge_method - only S256 is allowed (RFC 7636 security best practice)
        let method = request.code_challenge_method.as_deref().unwrap_or("S256");
        if method != "S256" {
            return Err(OAuth2Error::invalid_request(
                "code_challenge_method must be 'S256' (plain method is not supported for security reasons)",
            ));
        }
    } else {
        // PKCE is required for authorization code flow
        return Err(OAuth2Error::invalid_request(
            "code_challenge is required for authorization_code flow (PKCE)",
        ));
    }

    // User authentication required
    let user_id =
        user_id.ok_or_else(|| OAuth2Error::invalid_request("User authentication required"))?;

    // Generate authorization code with tenant isolation and state binding
    let tenant_id = tenant_id.unwrap_or_else(|| user_id.to_string());
    let auth_code = self
        .generate_authorization_code(AuthCodeParams {
            client_id: &request.client_id,
            user_id,
            tenant_id: &tenant_id,
            redirect_uri: &request.redirect_uri,
            scope: request.scope.as_deref(),
            state: request.state.as_deref(),
            code_challenge: request.code_challenge.as_deref(),
            code_challenge_method: request.code_challenge_method.as_deref(),
        })
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to generate authorization code for client_id={}: {:#}",
                request.client_id,
                e
            );
            OAuth2Error::invalid_request("Failed to generate authorization code")
        })?;

    Ok(AuthorizeResponse {
        code: auth_code,
        state: request.state,
    })
}
```

**PKCE validation**:
1. **Required**: `code_challenge` mandatory (no fallback to plain OAuth)
2. **S256 only**: SHA-256 method required (plain method rejected for security)
3. **Length validation**: 43-128 characters (base64url-encoded SHA-256)

## PKCE Flow Explained

PKCE prevents authorization code interception attacks:

```
Client generates random verifier:
  verifier = random(43-128 chars)

Client creates challenge:
  challenge = base64url(sha256(verifier))

Authorization request includes challenge:
  GET /oauth2/authorize?
    client_id=...&
    redirect_uri=...&
    code_challenge=<challenge>&
    code_challenge_method=S256

Server stores challenge with authorization code

Token request includes verifier:
  POST /oauth2/token
    grant_type=authorization_code&
    code=<auth_code>&
    code_verifier=<verifier>&
    ...

Server validates:
  if base64url(sha256(verifier)) == stored_challenge:
    issue_token()
  else:
    reject_request()
```

**Security benefit**: Even if authorization code is intercepted, attacker cannot exchange it without the original `code_verifier` (which never leaves the client).

## Token Endpoint

The token endpoint exchanges authorization codes for JWT access tokens:

**Source**: src/oauth2_server/endpoints.rs:163-186
```rust
/// Handle token request (POST /oauth/token)
///
/// # Errors
/// Returns an error if client validation fails or token generation fails
pub async fn token(&self, request: TokenRequest) -> Result<TokenResponse, OAuth2Error> {
    // ALWAYS validate client credentials for ALL grant types (RFC 6749 Section 6)
    // RFC 6749 §6 states: "If the client type is confidential or the client was issued
    // client credentials, the client MUST authenticate with the authorization server"
    // MCP clients are confidential clients, so authentication is REQUIRED
    self.client_manager
        .validate_client(&request.client_id, &request.client_secret)
        .await
        .inspect_err(|e| {
            tracing::error!(
                client_id = %request.client_id,
                grant_type = %request.grant_type,
                error = ?e,
                "OAuth client validation failed"
            );
        })?;

    match request.grant_type.as_str() {
        "authorization_code" => self.handle_authorization_code_grant(request).await,
        "client_credentials" => self.handle_client_credentials_grant(request),
        "refresh_token" => self.handle_refresh_token_grant(request).await,
        _ => Err(OAuth2Error::unsupported_grant_type()),
    }
}
```

**Grant types**:
- `authorization_code`: Exchange authorization code for access token (with PKCE verification)
- `client_credentials`: Machine-to-machine authentication (no user context)
- `refresh_token`: Renew expired access token without re-authentication

## Constant-Time Client Validation

Client credential validation uses constant-time comparison to prevent timing attacks:

**Source**: src/oauth2_server/client_registration.rs:114-153
```rust
/// Validate client credentials
///
/// # Errors
/// Returns an error if client is not found, credentials are invalid, or client is expired
pub async fn validate_client(
    &self,
    client_id: &str,
    client_secret: &str,
) -> Result<OAuth2Client, OAuth2Error> {
    tracing::debug!("Validating OAuth client: {}", client_id);

    let client = self.get_client(client_id).await.map_err(|e| {
        tracing::warn!("OAuth client {} not found: {}", client_id, e);
        OAuth2Error::invalid_client()
    })?;

    tracing::debug!("OAuth client {} found, validating secret", client_id);

    // Verify client secret using constant-time comparison via Argon2
    let parsed_hash = PasswordHash::new(&client.client_secret_hash).map_err(|e| {
        tracing::error!("Failed to parse stored password hash: {}", e);
        OAuth2Error::invalid_client()
    })?;

    let argon2 = Argon2::default();
    if argon2
        .verify_password(client_secret.as_bytes(), &parsed_hash)
        .is_err()
    {
        tracing::warn!("OAuth client {} secret validation failed", client_id);
        return Err(OAuth2Error::invalid_client());
    }

    // Check if client is expired
    if let Some(expires_at) = client.expires_at {
        if Utc::now() > expires_at {
            tracing::warn!("OAuth client {} has expired", client_id);
            return Err(OAuth2Error::invalid_client());
        }
    }

    tracing::info!("OAuth client {} validated successfully", client_id);
    Ok(client)
}
```

**Constant-time guarantee**: Argon2's `verify_password` uses constant-time comparison to prevent timing side-channel attacks.

## Rust Idioms: Constant-Time Operations

**Timing attack vulnerability**:
```rust
// VULNERABLE: Early return leaks information about secret length
if client_secret.len() != stored_secret.len() {
    return Err(...); // Attacker learns length immediately
}

for (a, b) in client_secret.bytes().zip(stored_secret.bytes()) {
    if a != b {
        return Err(...); // Attacker learns position of mismatch
    }
}
```

**Constant-time solution** (Argon2):
```rust
// SECURE: Always takes same time regardless of input
argon2.verify_password(client_secret.as_bytes(), &parsed_hash)
```

**Why this matters**: Timing attacks can recover secrets character-by-character by measuring response times.

## Multi-Tenant OAuth Management

Pierre provides tenant-specific OAuth credential isolation:

**Source**: src/tenant/oauth_manager.rs:14-46
```rust
/// Credential configuration for storing OAuth credentials
#[derive(Debug, Clone)]
pub struct CredentialConfig {
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (to be encrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// User who configured these credentials
    pub configured_by: Uuid,
}

/// Per-tenant OAuth credentials with decrypted secret
#[derive(Debug, Clone)]
pub struct TenantOAuthCredentials {
    /// Tenant ID that owns these credentials
    pub tenant_id: Uuid,
    /// OAuth provider name
    pub provider: String,
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (decrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// Daily rate limit for this tenant
    pub rate_limit_per_day: u32,
}
```

**Credential resolution**:

**Source**: src/tenant/oauth_manager.rs:76-100
```rust
/// Load OAuth credentials for a specific tenant and provider
///
/// # Errors
///
/// Returns an error if no credentials are found for the tenant/provider combination
pub async fn get_credentials(
    &self,
    tenant_id: Uuid,
    provider: &str,
    database: &Database,
) -> Result<TenantOAuthCredentials> {
    // Priority 1: Try tenant-specific credentials first (in-memory cache, then database)
    if let Some(credentials) = self
        .try_tenant_specific_credentials(tenant_id, provider, database)
        .await
    {
        return Ok(credentials);
    }

    // Priority 2: Fallback to server-level OAuth configuration
    if let Some(credentials) = self.try_server_level_credentials(tenant_id, provider) {
        return Ok(credentials);
    }

    // No credentials found - return error
    Err(AppError::not_found(format!(
        "No OAuth credentials configured for tenant {} and provider {}. Configure {}_CLIENT_ID and {}_CLIENT_SECRET environment variables, or provide tenant-specific credentials via the MCP OAuth configuration tool.",
        tenant_id, provider, provider.to_uppercase(), provider.to_uppercase()
    )).into())
}
```

**Credential priority**:
1. **Tenant-specific credentials** (highest priority): Custom OAuth apps per tenant
2. **Server-level credentials** (fallback): Shared OAuth apps from environment variables
3. **Error** (no credentials): Inform user how to configure

## OAuth Rate Limiting

Pierre implements rate limiting for OAuth endpoints:

**Source**: src/routes/oauth2.rs:136-149
```rust
async fn handle_client_registration(
    State(context): State<OAuth2Context>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(request): Json<ClientRegistrationRequest>,
) -> Response {
    // Extract client IP from connection using Axum's ConnectInfo extractor
    let client_ip = addr.ip();
    let rate_status = context.rate_limiter.check_rate_limit("register", client_ip);

    if rate_status.is_limited {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({
                "error": "too_many_requests",
                "error_description": "Rate limit exceeded"
            })),
        )
            .into_response();
    }
    // ... continue registration
}
```

**Rate-limited endpoints**:
- `/oauth2/register`: Prevent client registration spam
- `/oauth2/authorize`: Prevent authorization request floods
- `/oauth2/token`: Prevent token exchange brute-forcing

## Key Takeaways

1. **RFC compliance**: Pierre implements RFC 7591 (client registration), RFC 7636 (PKCE), RFC 8414 (discovery).

2. **PKCE mandatory**: Authorization code flow requires PKCE with SHA-256 (no plain method).

3. **Argon2 hashing**: Client secrets hashed with Argon2 (memory-hard, constant-time verification).

4. **Constant-time validation**: Client credential verification prevents timing attacks.

5. **JWT access tokens**: OAuth access tokens are JWTs (same format as Pierre authentication tokens).

6. **Multi-tenant isolation**: Tenant-specific OAuth credentials with separate rate limits.

7. **Discovery endpoint**: RFC 8414 metadata allows clients to auto-discover OAuth configuration.

8. **365-day expiry**: Client registrations expire after 1 year (security best practice).

9. **Rate limiting**: OAuth endpoints protected against abuse with IP-based rate limiting.

10. **Grant type defaults**: Only `authorization_code` by default (least privilege principle).

---

**Next Chapter**: [Chapter 16: OAuth 2.0 Client for Fitness Providers](./chapter-16-oauth-client.md) - Learn how Pierre acts as an OAuth client to connect to fitness providers like Strava and Fitbit.
