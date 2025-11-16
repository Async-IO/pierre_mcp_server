# chapter 06: JWT authentication with RS256

This chapter explores JWT (JSON Web Token) authentication using RS256 asymmetric signing in the Pierre Fitness Platform. You'll learn how the platform implements secure token generation, validation, and session management using RSA key pairs from the JWKS system covered in Chapter 5.

## what you'll learn

- JWT structure and standard claims (iss, sub, aud, exp, iat, nbf, jti)
- RS256 vs HS256: why asymmetric signing matters
- Token generation with RSA private keys
- Token validation with RSA public keys
- Custom claims for permissions and multi-tenancy
- Token refresh patterns and session management
- Integration with JWKS for key rotation
- Detailed error handling for token validation
- Middleware-based authentication for MCP requests

## JWT structure and claims

JWT tokens consist of three base64-encoded parts separated by dots: `header.payload.signature`. The Pierre platform uses RS256 (RSA Signature with SHA-256) for asymmetric signing, allowing token verification without sharing the private key.

### standard JWT claims

The platform follows RFC 7519 for standard JWT claims:

**Source**: src/auth.rs:108-130
```rust
/// JWT claims for user authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User ID
    pub sub: String,
    /// User email
    pub email: String,
    /// Issued at timestamp (seconds since Unix epoch)
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Issuer (who issued the token)
    pub iss: String,
    /// JWT ID (unique identifier for this token)
    pub jti: String,
    /// Available fitness providers
    pub providers: Vec<String>,
    /// Audience (who the token is intended for)
    pub aud: String,
    /// Tenant ID (optional for backward compatibility with existing tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
}
```

Each claim serves a specific purpose:

- `sub` (Subject): Unique user identifier (UUID)
- `iss` (Issuer): Service that created the token ("pierre-mcp-server")
- `aud` (Audience): Intended recipient of the token ("mcp" or "admin-api")
- `exp` (Expiration): Unix timestamp when token becomes invalid
- `iat` (Issued At): Unix timestamp when token was created
- `jti` (JWT ID): Unique token identifier (prevents replay attacks)

### custom claims for multi-tenancy

The platform extends standard claims with domain-specific fields:

- `email`: User's email address for quick lookups
- `providers`: List of connected fitness providers (Garmin, Strava, etc.)
- `tenant_id`: Multi-tenant isolation identifier (optional for backward compatibility)

**Rust Idiom**: `#[serde(skip_serializing_if = "Option::is_none")]`

This attribute prevents including `null` values in the JSON payload, reducing token size. The `Option<String>` type provides compile-time safety for optional fields while maintaining backward compatibility with tokens that don't include `tenant_id`.

## RS256 vs HS256: asymmetric signing

The platform uses RS256 (RSA Signature with SHA-256) instead of HS256 (HMAC with SHA-256) for several security advantages:

### HS256: symmetric signing (not used)

```
┌─────────────┐                    ┌─────────────┐
│   Server    │                    │   Client    │
│             │                    │             │
│ Secret Key  │◄──────shared───────┤ Secret Key  │
│             │                    │             │
│ Sign Token  │────────────────────►│ Verify Token│
└─────────────┘                    └─────────────┘
```

**Problem**: The same secret key signs AND verifies tokens. If clients need to verify tokens, they must have the private key, which defeats the purpose of asymmetric cryptography.

### RS256: asymmetric signing (used by Pierre)

```
┌─────────────────┐                ┌─────────────────┐
│     Server      │                │     Client      │
│                 │                │                 │
│ Private Key     │                │  Public Key     │
│ (JWKS secret)   │                │  (JWKS public)  │
│                 │                │                 │
│ Sign Token ────►│────token──────►│ Verify Token    │
│                 │                │                 │
│ Rotate Keys     │◄───GET /jwks◄──┤ Fetch Public    │
└─────────────────┘                └─────────────────┘
```

**Advantage**: The server holds the private key (MEK-encrypted in the database). Clients download only public keys from `/.well-known/jwks.json` endpoint. Even if a client is compromised, attackers cannot forge tokens.

**Source**: src/auth.rs:232-243
```rust
// Get active RSA key from JWKS manager
let active_key = jwks_manager.get_active_key()?;
let encoding_key = active_key.encoding_key()?;

// Create RS256 header with kid
let mut header = Header::new(Algorithm::RS256);
header.kid = Some(active_key.kid.clone());

let token = encode(&header, &claims, &encoding_key)?;
```

The `kid` (Key ID) in the header allows the platform to rotate RSA keys without invalidating existing tokens. When validating a token, the platform looks up the corresponding public key by `kid`.

## token generation with JWKS integration

Token generation involves creating claims, selecting the active RSA key, and signing with the private key.

### user authentication tokens

The `AuthManager` generates tokens for authenticated users after successful login:

**Source**: src/auth.rs:212-243
```rust
/// Generate a JWT token for a user with RS256 asymmetric signing
///
/// # Errors
///
/// Returns an error if:
/// - JWT encoding fails due to invalid claims
/// - System time is unavailable for timestamp generation
/// - JWKS manager has no active key
pub fn generate_token(
    &self,
    user: &User,
    jwks_manager: &crate::admin::jwks::JwksManager,
) -> Result<String> {
    let now = Utc::now();
    let expiry = now + Duration::hours(self.token_expiry_hours);

    let claims = Claims {
        sub: user.id.to_string(),
        email: user.email.clone(),
        iat: now.timestamp(),
        exp: expiry.timestamp(),
        iss: crate::constants::service_names::PIERRE_MCP_SERVER.to_owned(),
        jti: Uuid::new_v4().to_string(),
        providers: user.available_providers(),
        aud: crate::constants::service_names::MCP.to_owned(),
        tenant_id: user.tenant_id.clone(),
    };

    // Get active RSA key from JWKS manager
    let active_key = jwks_manager.get_active_key()?;
    let encoding_key = active_key.encoding_key()?;

    // Create RS256 header with kid
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(active_key.kid.clone());

    let token = encode(&header, &claims, &encoding_key)?;

    Ok(token)
}
```

**Rust Idiom**: `Uuid::new_v4().to_string()`

Using UUIDv4 for `jti` (JWT ID) ensures each token has a globally unique identifier. This prevents token replay attacks and allows the platform to revoke specific tokens by tracking their `jti` in a revocation list.

### admin authentication tokens

Admin tokens use a separate claims structure with fine-grained permissions:

**Source**: src/admin/jwt.rs:171-188
```rust
/// JWT claims for admin tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminTokenClaims {
    // Standard JWT claims
    iss: String, // Issuer: "pierre-mcp-server"
    sub: String, // Subject: token ID
    aud: String, // Audience: "admin-api"
    exp: u64,    // Expiration time
    iat: u64,    // Issued at
    nbf: u64,    // Not before
    jti: String, // JWT ID: token ID

    // Custom claims
    service_name: String,
    permissions: Vec<crate::admin::models::AdminPermission>,
    is_super_admin: bool,
    token_type: String, // Always "admin"
}
```

Admin tokens include:
- `permissions`: List of specific admin permissions (e.g., `["users:read", "users:write"]`)
- `is_super_admin`: Boolean flag for unrestricted access
- `service_name`: Identifies which service created the token
- `token_type`: Discriminator to prevent user tokens from being used as admin tokens

**Source**: src/admin/jwt.rs:64-97
```rust
/// Generate JWT token using RS256 (asymmetric signing)
///
/// # Errors
/// Returns an error if JWT encoding fails
pub fn generate_token(
    &self,
    token_id: &str,
    service_name: &str,
    permissions: &AdminPermissions,
    is_super_admin: bool,
    expires_at: Option<DateTime<Utc>>,
    jwks_manager: &crate::admin::jwks::JwksManager,
) -> Result<String> {
    let now = Utc::now();
    let exp = expires_at.unwrap_or_else(|| now + Duration::days(365));

    let claims = AdminTokenClaims {
        // Standard JWT claims
        iss: service_names::PIERRE_MCP_SERVER.into(),
        sub: token_id.to_owned(),
        aud: service_names::ADMIN_API.into(),
        exp: u64::try_from(exp.timestamp().max(0)).unwrap_or(0),
        iat: u64::try_from(now.timestamp().max(0)).unwrap_or(0),
        nbf: u64::try_from(now.timestamp().max(0)).unwrap_or(0),
        jti: token_id.to_owned(),

        // Custom claims
        service_name: service_name.to_owned(),
        permissions: permissions.to_vec(),
        is_super_admin,
        token_type: "admin".into(),
    };

    // Sign with RS256 using JWKS
    Ok(jwks_manager
        .sign_admin_token(&claims)
        .map_err(|e| AppError::internal(format!("Failed to generate RS256 admin JWT: {e}")))?)
}
```

**Rust Idiom**: `u64::try_from(exp.timestamp().max(0)).unwrap_or(0)`

This pattern handles two edge cases:
1. `max(0)`: Prevents negative timestamps (before Unix epoch)
2. `try_from()`: Safely converts `i64` to `u64` (timestamps should always be positive)
3. `unwrap_or(0)`: Falls back to epoch if conversion fails (defensive programming)

The combination ensures the `exp` claim is always a valid positive integer.

### OAuth access tokens

The platform generates OAuth 2.0 access tokens with limited scopes:

**Source**: src/auth.rs:588-622
```rust
/// Generate OAuth access token with RS256 asymmetric signing
///
/// This method uses RSA private key from JWKS manager for token signing.
/// Clients can verify tokens using the public key from /.well-known/jwks.json
///
/// # Errors
///
/// Returns an error if:
/// - JWT token generation fails
/// - System time is unavailable
/// - JWKS manager has no active key
pub fn generate_oauth_access_token(
    &self,
    jwks_manager: &crate::admin::jwks::JwksManager,
    user_id: &Uuid,
    scopes: &[String],
    tenant_id: Option<String>,
) -> Result<String> {
    let now = Utc::now();
    let expiry =
        now + Duration::hours(crate::constants::limits::OAUTH_ACCESS_TOKEN_EXPIRY_HOURS);

    let claims = Claims {
        sub: user_id.to_string(),
        email: format!("oauth_{user_id}@system.local"),
        iat: now.timestamp(),
        exp: expiry.timestamp(),
        iss: crate::constants::service_names::PIERRE_MCP_SERVER.to_owned(),
        jti: Uuid::new_v4().to_string(),
        providers: scopes.to_vec(),
        aud: crate::constants::service_names::MCP.to_owned(),
        tenant_id,
    };

    // Get active RSA key from JWKS manager
    let active_key = jwks_manager.get_active_key()?;
    let encoding_key = active_key.encoding_key()?;

    // Create RS256 header with kid
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(active_key.kid.clone());

    let token = encode(&header, &claims, &encoding_key)?;

    Ok(token)
}
```

OAuth tokens use the `providers` claim to store granted scopes (e.g., `["read:activities", "write:workouts"]`). This allows the platform to enforce fine-grained permissions without database lookups.

## token validation and error handling

Token validation verifies the RS256 signature and checks expiration, audience, and issuer claims.

### RS256 signature verification

The platform uses the `kid` from the token header to look up the correct public key:

**Source**: src/auth.rs:256-292
```rust
/// Validate a RS256 JWT token using JWKS public keys
///
/// # Errors
///
/// Returns an error if:
/// - Token signature is invalid
/// - Token has expired
/// - Token is malformed or not valid JWT format
/// - Token header doesn't contain kid (key ID)
/// - JWKS manager doesn't have the specified key
/// - Token claims cannot be deserialized
pub fn validate_token(
    &self,
    token: &str,
    jwks_manager: &crate::admin::jwks::JwksManager,
) -> Result<Claims> {
    // Extract kid from token header
    let header = jsonwebtoken::decode_header(token)?;
    let kid = header.kid.ok_or_else(|| -> anyhow::Error {
        AppError::auth_invalid("Token header missing kid (key ID)").into()
    })?;

    tracing::debug!("Validating RS256 JWT token with kid: {}", kid);

    // Get public key from JWKS manager
    let key_pair = jwks_manager.get_key(&kid).ok_or_else(|| -> anyhow::Error {
        AppError::auth_invalid(format!("Key not found in JWKS: {kid}")).into()
    })?;

    let decoding_key =
        key_pair
            .decoding_key()
            .map_err(|e| JwtValidationError::TokenInvalid {
                reason: format!("Failed to get decoding key: {e}"),
            })?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = true;
    validation.set_audience(&[crate::constants::service_names::MCP]);
    validation.set_issuer(&[crate::constants::service_names::PIERRE_MCP_SERVER]);

    let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
        tracing::error!("RS256 JWT validation failed: {:?}", e);
        e
    })?;

    Ok(token_data.claims)
}
```

**Key rotation support**: The `kid` lookup allows the platform to rotate RSA keys without invalidating existing tokens. Tokens signed with old keys remain valid as long as the old key pair exists in JWKS.

**Rust Idiom**: `ok_or_else(|| -> anyhow::Error { ... })`

This pattern converts `Option<T>` to `Result<T, E>` with lazy error construction. The closure only executes if the option is `None`, avoiding unnecessary allocations for successful cases.

### detailed validation errors

The platform provides detailed error messages for debugging token issues:

**Source**: src/auth.rs:44-104
```rust
/// JWT validation error with detailed information
#[derive(Debug, Clone)]
pub enum JwtValidationError {
    /// Token has expired
    TokenExpired {
        /// When the token expired
        expired_at: DateTime<Utc>,
        /// Current time for reference
        current_time: DateTime<Utc>,
    },
    /// Token signature is invalid
    TokenInvalid {
        /// Reason for invalidity
        reason: String,
    },
    /// Token is malformed (not proper JWT format)
    TokenMalformed {
        /// Details about malformation
        details: String,
    },
}

impl std::fmt::Display for JwtValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TokenExpired {
                expired_at,
                current_time,
            } => {
                let duration_expired = current_time.signed_duration_since(*expired_at);
                if duration_expired.num_minutes() < 60 {
                    write!(
                        f,
                        "JWT token expired {} minutes ago at {}",
                        duration_expired.num_minutes(),
                        expired_at.format("%Y-%m-%d %H:%M:%S UTC")
                    )
                } else if duration_expired.num_hours() < USER_SESSION_EXPIRY_HOURS {
                    write!(
                        f,
                        "JWT token expired {} hours ago at {}",
                        duration_expired.num_hours(),
                        expired_at.format("%Y-%m-%d %H:%M:%S UTC")
                    )
                } else {
                    write!(
                        f,
                        "JWT token expired {} days ago at {}",
                        duration_expired.num_days(),
                        expired_at.format("%Y-%m-%d %H:%M:%S UTC")
                    )
                }
            }
            Self::TokenInvalid { reason } => {
                write!(f, "JWT token signature is invalid: {reason}")
            }
            Self::TokenMalformed { details } => {
                write!(f, "JWT token is malformed: {details}")
            }
        }
    }
}
```

**User experience**: Human-readable error messages help developers debug authentication issues. For example, "JWT token expired 3 hours ago at 2025-01-15 14:30:00 UTC" is more actionable than "Token expired".

### expiration checking

The platform separates signature verification from expiration checking for better error messages:

**Source**: src/auth.rs:381-421
```rust
/// Decode RS256 JWT token claims without expiration validation
fn decode_token_claims(
    token: &str,
    jwks_manager: &crate::admin::jwks::JwksManager,
) -> Result<Claims, JwtValidationError> {
    // Extract kid from token header
    let header =
        jsonwebtoken::decode_header(token).map_err(|e| JwtValidationError::TokenMalformed {
            details: format!("Failed to decode token header: {e}"),
        })?;

    let kid = header
        .kid
        .ok_or_else(|| JwtValidationError::TokenMalformed {
            details: "Token header missing kid (key ID)".to_owned(),
        })?;

    // Get public key from JWKS manager
    let key_pair =
        jwks_manager
            .get_key(&kid)
            .ok_or_else(|| JwtValidationError::TokenInvalid {
                reason: format!("Key not found in JWKS: {kid}"),
            })?;

    let decoding_key =
        key_pair
            .decoding_key()
            .map_err(|e| JwtValidationError::TokenInvalid {
                reason: format!("Failed to get decoding key: {e}"),
            })?;

    let mut validation_no_exp = Validation::new(Algorithm::RS256);
    validation_no_exp.validate_exp = false;
    validation_no_exp.set_audience(&[crate::constants::service_names::MCP]);
    validation_no_exp.set_issuer(&[crate::constants::service_names::PIERRE_MCP_SERVER]);

    decode::<Claims>(token, &decoding_key, &validation_no_exp)
        .map(|token_data| token_data.claims)
        .map_err(|e| Self::convert_jwt_error(&e))
}
```

**Design pattern**: Decode first with `validate_exp = false`, then check expiration manually. This allows detailed expiration errors while still verifying the signature for refresh tokens.

**Source**: src/auth.rs:423-438
```rust
/// Validate claims expiration with detailed logging
fn validate_claims_expiry(claims: &Claims) -> Result<(), JwtValidationError> {
    let current_time = Utc::now();
    let expired_at = DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now);

    tracing::debug!(
        "Token validation details - User: {}, Issued: {}, Expires: {}, Current: {}",
        claims.sub,
        DateTime::from_timestamp(claims.iat, 0)
            .map_or_else(|| "unknown".into(), |d| d.to_rfc3339()),
        expired_at.to_rfc3339(),
        current_time.to_rfc3339()
    );

    Self::check_token_expiry(claims, current_time, expired_at)
}
```

## session management and token refresh

The platform creates sessions after successful authentication and supports token refresh for better user experience.

### session creation

**Source**: src/auth.rs:449-464
```rust
/// Create a user session from a valid user with RS256 token
///
/// # Errors
///
/// Returns an error if:
/// - JWT token generation fails
/// - User data is invalid
/// - System time is unavailable
/// - JWKS manager has no active key
pub fn create_session(
    &self,
    user: &User,
    jwks_manager: &crate::admin::jwks::JwksManager,
) -> Result<UserSession> {
    let jwt_token = self.generate_token(user, jwks_manager)?;
    let expires_at = Utc::now() + Duration::hours(self.token_expiry_hours);

    Ok(UserSession {
        user_id: user.id,
        jwt_token,
        expires_at,
        email: user.email.clone(),
        available_providers: user.available_providers(),
    })
}
```

The `UserSession` struct contains everything a client needs to interact with the API:
- `jwt_token`: RS256-signed JWT for authentication
- `expires_at`: When the token becomes invalid
- `available_providers`: Which fitness providers the user has connected

### token refresh pattern

**Source**: src/auth.rs:515-529
```rust
/// Refresh a token if it's still valid (RS256)
///
/// # Errors
///
/// Returns an error if:
/// - Old token signature is invalid (even if expired)
/// - Token is malformed
/// - New token generation fails
/// - User data is invalid
/// - JWKS manager has no active key
pub fn refresh_token(
    &self,
    old_token: &str,
    user: &User,
    jwks_manager: &crate::admin::jwks::JwksManager,
) -> Result<String> {
    // First validate the old token signature (even if expired)
    // This ensures the refresh request is legitimate
    Self::decode_token_claims(old_token, jwks_manager).map_err(|e| -> anyhow::Error {
        AppError::auth_invalid(format!("Failed to validate old token for refresh: {e}")).into()
    })?;

    // Generate new token - atomic counter ensures uniqueness
    self.generate_token(user, jwks_manager)
}
```

**Security**: The refresh pattern validates the old token's signature even if expired. This prevents attackers from forging expired tokens to request new ones.

**Rust Idiom**: Decode without expiration check (`decode_token_claims`) ensures legitimate expired tokens can be refreshed while forged tokens are rejected.

## middleware-based authentication

The platform uses middleware to authenticate MCP requests with both JWT tokens and API keys.

### request authentication flow

```
┌──────────────────────────────────────────────────────────────┐
│                     MCP Request                              │
│                                                              │
│  Authorization: Bearer eyJhbGc...  or  pk_live_abc123...    │
└────────────────────────┬─────────────────────────────────────┘
                         │
                         ▼
          ┌──────────────────────────┐
          │  McpAuthMiddleware       │
          │                          │
          │  authenticate_request()  │
          └──────────────────────────┘
                         │
            ┌────────────┴────────────┐
            │                         │
            ▼                         ▼
    ┌───────────────┐         ┌──────────────┐
    │  JWT Token    │         │  API Key     │
    │  (Bearer)     │         │  (pk_live_)  │
    └───────────────┘         └──────────────┘
            │                         │
            ▼                         ▼
    ┌───────────────┐         ┌──────────────┐
    │ validate_token│         │ hash + lookup│
    │ with JWKS     │         │ in database  │
    └───────────────┘         └──────────────┘
            │                         │
            └────────────┬────────────┘
                         ▼
                 ┌──────────────┐
                 │  AuthResult  │
                 │              │
                 │  - user_id   │
                 │  - tier      │
                 │  - rate_limit│
                 └──────────────┘
```

**Source**: src/middleware/auth.rs:65-136
```rust
#[tracing::instrument(
    skip(self, auth_header),
    fields(
        auth_method = tracing::field::Empty,
        user_id = tracing::field::Empty,
        tenant_id = tracing::field::Empty,
        success = tracing::field::Empty,
    )
)]
pub async fn authenticate_request(&self, auth_header: Option<&str>) -> Result<AuthResult> {
    tracing::debug!("=== AUTH MIDDLEWARE AUTHENTICATE_REQUEST START ===");
    tracing::debug!("Auth header provided: {}", auth_header.is_some());

    let auth_str = if let Some(header) = auth_header {
        // Security: Do not log auth header content to prevent token leakage
        tracing::debug!(
            "Authentication attempt with header type: {}",
            if header.starts_with(key_prefixes::API_KEY_LIVE) {
                "API_KEY"
            } else if header.starts_with("Bearer ") {
                "JWT_TOKEN"
            } else {
                "UNKNOWN"
            }
        );
        header
    } else {
        tracing::warn!("Authentication failed: Missing authorization header");
        return Err(auth_error("Missing authorization header - Request authentication requires Authorization header with Bearer token or API key").into());
    };

    // Try API key authentication first (starts with pk_live_)
    if auth_str.starts_with(key_prefixes::API_KEY_LIVE) {
        tracing::Span::current().record("auth_method", "API_KEY");
        tracing::debug!("Attempting API key authentication");
        match self.authenticate_api_key(auth_str).await {
            Ok(result) => {
                tracing::Span::current()
                    .record("user_id", result.user_id.to_string())
                    .record("tenant_id", result.user_id.to_string()) // Use user_id as tenant_id for now
                    .record("success", true);
                tracing::info!(
                    "API key authentication successful for user: {}",
                    result.user_id
                );
                Ok(result)
            }
            Err(e) => {
                tracing::Span::current().record("success", false);
                tracing::warn!("API key authentication failed: {}", e);
                Err(e)
            }
        }
    }
    // Then try Bearer token authentication
    else if let Some(token) = auth_str.strip_prefix("Bearer ") {
        tracing::Span::current().record("auth_method", "JWT_TOKEN");
        tracing::debug!("Attempting JWT token authentication");
        match self.authenticate_jwt_token(token).await {
            Ok(result) => {
                tracing::Span::current()
                    .record("user_id", result.user_id.to_string())
                    .record("tenant_id", result.user_id.to_string()) // Use user_id as tenant_id for now
                    .record("success", true);
                tracing::info!("JWT authentication successful for user: {}", result.user_id);
                Ok(result)
            }
            Err(e) => {
                tracing::Span::current().record("success", false);
                tracing::warn!("JWT authentication failed: {}", e);
                Err(e)
            }
        }
    } else {
        tracing::Span::current()
            .record("auth_method", "INVALID")
            .record("success", false);
        tracing::warn!("Authentication failed: Invalid authorization header format (expected 'Bearer ...' or 'pk_live_...')");
        Err(AppError::auth_invalid("Invalid authorization header format - must be 'Bearer <token>' or 'pk_live_<api_key>'").into())
    }
}
```

**Rust Idiom**: `#[tracing::instrument(skip(self, auth_header), fields(...))]`

This attribute automatically creates a tracing span for the function with structured fields. The `skip(self, auth_header)` prevents logging sensitive data (JWT tokens). The empty fields get populated dynamically using `record()`.

**Security**: The middleware logs authentication attempts without exposing token contents, balancing observability with security.

### JWT authentication in middleware

**Source**: src/middleware/auth.rs:194-228
```rust
/// Authenticate using RS256 JWT token
async fn authenticate_jwt_token(&self, token: &str) -> Result<AuthResult> {
    let claims = self
        .auth_manager
        .validate_token_detailed(token, &self.jwks_manager)?;

    let user_id = crate::utils::uuid::parse_uuid(&claims.sub)
        .map_err(|_| AppError::auth_invalid("Invalid user ID in token"))?;

    // Get user from database to check tier and rate limits
    let user = self
        .database
        .get_user(user_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("User {user_id}")))?;

    // Get current usage for rate limiting
    let current_usage = self.database.get_jwt_current_usage(user_id).await?;
    let rate_limit = self
        .rate_limit_calculator
        .calculate_jwt_rate_limit(&user, current_usage);

    // Check rate limit
    if rate_limit.is_rate_limited {
        return Err(auth_error("JWT token rate limit exceeded").into());
    }

    Ok(AuthResult {
        user_id,
        auth_method: AuthMethod::JwtToken {
            tier: format!("{:?}", user.tier).to_lowercase(),
        },
        rate_limit,
    })
}
```

The middleware:
1. Validates token signature with RS256 using JWKS
2. Extracts user ID from `sub` claim
3. Looks up user in database for current rate limit tier
4. Calculates rate limit based on tier and current usage
5. Returns `AuthResult` with user context and rate limit info

### authentication result

**Source**: src/auth.rs:133-158
```rust
/// Authentication result with user context and rate limiting info
#[derive(Debug)]
pub struct AuthResult {
    /// Authenticated user ID
    pub user_id: Uuid,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Rate limit information (always provided for both API keys and JWT tokens)
    pub rate_limit: UnifiedRateLimitInfo,
}

/// Authentication method used
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// JWT token authentication
    JwtToken {
        /// User tier for rate limiting
        tier: String,
    },
    /// API key authentication
    ApiKey {
        /// API key ID
        key_id: String,
        /// API key tier
        tier: String,
    },
}
```

The `AuthResult` provides downstream handlers with:
- `user_id`: For database queries and multi-tenant isolation
- `auth_method`: For logging and analytics
- `rate_limit`: For enforcing API usage limits

## real-world usage patterns

### admin API authentication

**Source**: src/admin/jwt.rs:190-251
```rust
/// Token generation configuration
#[derive(Debug, Clone)]
pub struct TokenGenerationConfig {
    /// Service name for the token
    pub service_name: String,
    /// Optional human-readable description
    pub service_description: Option<String>,
    /// Permissions granted to this token
    pub permissions: Option<AdminPermissions>,
    /// Token expiration in days (None for no expiration)
    pub expires_in_days: Option<u64>,
    /// Whether this is a super admin token with full privileges
    pub is_super_admin: bool,
}

impl TokenGenerationConfig {
    /// Create config for regular admin token
    #[must_use]
    pub fn regular_admin(service_name: String) -> Self {
        Self {
            service_name,
            service_description: None,
            permissions: Some(AdminPermissions::default_admin()),
            expires_in_days: Some(365), // 1 year
            is_super_admin: false,
        }
    }

    /// Create config for super admin token
    #[must_use]
    pub fn super_admin(service_name: String) -> Self {
        Self {
            service_name,
            service_description: Some("Super Admin Token".into()),
            permissions: Some(AdminPermissions::super_admin()),
            expires_in_days: None, // Never expires
            is_super_admin: true,
        }
    }

    /// Get effective permissions
    #[must_use]
    pub fn get_permissions(&self) -> AdminPermissions {
        self.permissions.as_ref().map_or_else(
            || {
                if self.is_super_admin {
                    AdminPermissions::super_admin()
                } else {
                    AdminPermissions::default_admin()
                }
            },
            std::clone::Clone::clone,
        )
    }

    /// Get expiration date
    #[must_use]
    pub fn get_expiration(&self) -> Option<DateTime<Utc>> {
        self.expires_in_days
            .map(|days| Utc::now() + Duration::days(i64::try_from(days).unwrap_or(365)))
    }
}
```

**Builder pattern**: The `TokenGenerationConfig` provides constructor methods (`regular_admin`, `super_admin`) for common configurations while allowing custom settings.

### OAuth token generation

The platform generates OAuth access tokens for external client applications:

**Source**: src/auth.rs:624-668
```rust
/// Generate client credentials token with RS256 asymmetric signing
///
/// This method uses RSA private key from JWKS manager for token signing.
/// Clients can verify tokens using the public key from /.well-known/jwks.json
///
/// # Errors
///
/// Returns an error if:
/// - JWT token generation fails
/// - System time is unavailable
/// - JWKS manager has no active key
pub fn generate_client_credentials_token(
    &self,
    jwks_manager: &crate::admin::jwks::JwksManager,
    client_id: &str,
    scopes: &[String],
    tenant_id: Option<String>,
) -> Result<String> {
    let now = Utc::now();
    let expiry = now + Duration::hours(1); // 1 hour for client credentials

    let claims = Claims {
        sub: format!("client:{client_id}"),
        email: "client_credentials".to_owned(),
        iat: now.timestamp(),
        exp: expiry.timestamp(),
        iss: crate::constants::service_names::PIERRE_MCP_SERVER.to_owned(),
        jti: Uuid::new_v4().to_string(),
        providers: scopes.to_vec(),
        aud: crate::constants::service_names::MCP.to_owned(),
        tenant_id,
    };

    // Get active RSA key from JWKS manager
    let active_key = jwks_manager.get_active_key()?;
    let encoding_key = active_key.encoding_key()?;

    // Create RS256 header with kid
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(active_key.kid.clone());

    let token = encode(&header, &claims, &encoding_key)?;

    Ok(token)
}
```

**Design decision**: Client credentials tokens use `sub: format!("client:{client_id}")` to distinguish them from user tokens. The prefix allows middleware to apply different authorization rules.

## key takeaways

1. **RS256 asymmetric signing**: Uses RSA key pairs from JWKS (Chapter 5) for secure token signing. Clients verify with public keys, server signs with private key.

2. **Standard JWT claims**: Platform follows RFC 7519 with `iss`, `sub`, `aud`, `exp`, `iat`, `jti` for interoperability. Custom claims extend functionality without breaking standards.

3. **Key rotation support**: The `kid` (key ID) in token headers allows seamless RSA key rotation. Old tokens remain valid until expiration.

4. **Detailed error handling**: `JwtValidationError` enum provides human-readable messages for debugging ("token expired 3 hours ago" vs "invalid token").

5. **Middleware authentication**: `McpAuthMiddleware` supports both JWT tokens and API keys with unified rate limiting and user context extraction.

6. **Token refresh pattern**: Validates old token signature even if expired, prevents forged refresh requests while improving UX.

7. **Multi-tenant claims**: `tenant_id` claim enables data isolation, `providers` claim restricts access to connected fitness providers.

8. **Separate admin tokens**: `AdminTokenClaims` with fine-grained permissions prevents privilege escalation from user tokens to admin APIs.

9. **Structured logging**: `#[tracing::instrument]` provides observability without exposing sensitive token data in logs.

10. **OAuth integration**: Platform generates standard OAuth 2.0 access tokens and client credentials tokens for third-party integrations.

---

**Next Chapter**: [Chapter 07: Multi-Tenant Database Isolation](./chapter-07-multi-tenant-isolation.md) - Learn how the Pierre platform enforces tenant boundaries at the database layer using JWT claims and row-level security.
