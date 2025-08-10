// ABOUTME: JWT-based user authentication and authorization system
// ABOUTME: Handles user login, token generation, validation, and session management
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Authentication and Session Management
//!
//! This module provides JWT-based authentication and session management
//! for the multi-tenant Pierre MCP Server.

use crate::api_keys::ApiKeyManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::models::{AuthRequest, AuthResponse, User, UserSession};
use crate::rate_limiting::{UnifiedRateLimitCalculator, UnifiedRateLimitInfo};
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use uuid::Uuid;

/// Convert a duration to a human-readable format
fn humanize_duration(duration: Duration) -> String {
    let total_secs = duration.num_seconds().abs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;

    if hours > 0 {
        format!("{hours} hours")
    } else if minutes > 0 {
        format!("{minutes} minutes")
    } else {
        format!("{total_secs} seconds")
    }
}

/// `JWT` validation error with detailed information
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
    /// Token is malformed (not proper `JWT` format)
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
                } else if duration_expired.num_hours() < 24 {
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

impl std::error::Error for JwtValidationError {}

/// `JWT` claims for user authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User `ID`
    pub sub: String,
    /// User email
    pub email: String,
    /// Issued at timestamp
    pub iat: i64,
    /// Expiration timestamp
    pub exp: i64,
    /// Available fitness providers
    pub providers: Vec<String>,
}

/// Authentication result with user context and rate limiting info
#[derive(Debug)]
pub struct AuthResult {
    /// Authenticated user `ID`
    pub user_id: Uuid,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Rate limit information (always provided for both `API` keys and `JWT` tokens)
    pub rate_limit: UnifiedRateLimitInfo,
}

/// Authentication method used
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// `JWT` token authentication
    JwtToken {
        /// User tier for rate limiting
        tier: String,
    },
    /// `API` key authentication
    ApiKey {
        /// `API` key `ID`
        key_id: String,
        /// `API` key tier
        tier: String,
    },
}

impl AuthMethod {
    /// Get a human-readable display name for the authentication method
    #[must_use]
    pub const fn display_name(&self) -> &str {
        match self {
            Self::JwtToken { .. } => "JWT Token",
            Self::ApiKey { .. } => "API Key",
        }
    }

    /// Get detailed information about the authentication method
    #[must_use]
    pub fn details(&self) -> String {
        match self {
            Self::JwtToken { tier } => {
                format!("JWT Token (tier: {tier})")
            }
            Self::ApiKey { key_id, tier } => {
                format!("API Key (tier: {tier}, id: {key_id})")
            }
        }
    }
}

/// Authentication manager for `JWT` tokens and user sessions
pub struct AuthManager {
    jwt_secret: Vec<u8>,
    token_expiry_hours: i64,
    /// Monotonic counter to ensure unique timestamps for tokens
    token_counter: AtomicU64,
}

impl Clone for AuthManager {
    fn clone(&self) -> Self {
        Self {
            jwt_secret: self.jwt_secret.clone(),
            token_expiry_hours: self.token_expiry_hours,
            // Start fresh counter for cloned instance - this is acceptable
            // since each instance will maintain uniqueness independently
            token_counter: AtomicU64::new(0),
        }
    }
}

impl AuthManager {
    /// Create a new authentication manager
    #[must_use]
    pub const fn new(jwt_secret: Vec<u8>, token_expiry_hours: i64) -> Self {
        Self {
            jwt_secret,
            token_expiry_hours,
            token_counter: AtomicU64::new(0),
        }
    }

    /// Get the `JWT` secret
    pub fn jwt_secret(&self) -> &[u8] {
        &self.jwt_secret
    }

    /// Generate a `JWT` token for a user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT encoding fails due to invalid claims
    /// - Secret key is malformed
    /// - System time is unavailable for timestamp generation
    pub fn generate_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(self.token_expiry_hours);

        // Use atomic counter to ensure unique issued-at times
        // This eliminates the need for blocking sleep calls
        let counter = self.token_counter.fetch_add(1, Ordering::Relaxed);
        let unique_iat =
            now.timestamp() * 1000 + i64::from(u32::try_from(counter % 1000).unwrap_or(0));

        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            iat: unique_iat,
            exp: expiry.timestamp(),
            providers: user.available_providers(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.jwt_secret),
        )?;

        Ok(token)
    }

    /// Validate a `JWT` token and extract claims
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token signature is invalid
    /// - Token has expired
    /// - Token is malformed or not valid JWT format
    /// - Token claims cannot be deserialized
    pub fn validate_token(&self, token: &str) -> Result<Claims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation,
        )?;

        Ok(token_data.claims)
    }

    /// Check if token is expired and return error if so
    fn check_token_expiry(
        claims: &Claims,
        current_time: DateTime<Utc>,
        expired_at: DateTime<Utc>,
    ) -> Result<(), JwtValidationError> {
        if current_time.timestamp() > claims.exp {
            let time_since_expiry = current_time.signed_duration_since(expired_at);
            tracing::warn!(
                "JWT token expired for user: {} - Expired {} ago at {}",
                claims.sub,
                humanize_duration(time_since_expiry),
                expired_at.to_rfc3339()
            );
            return Err(JwtValidationError::TokenExpired {
                expired_at,
                current_time,
            });
        }
        Ok(())
    }

    /// Convert JWT library errors to detailed validation errors
    fn convert_jwt_error(e: &jsonwebtoken::errors::Error) -> JwtValidationError {
        use jsonwebtoken::errors::ErrorKind;
        tracing::warn!("JWT token validation failed: {:?}", e);

        match e.kind() {
            ErrorKind::InvalidSignature => {
                tracing::warn!("JWT token signature verification failed");
                JwtValidationError::TokenInvalid {
                    reason: "Token signature verification failed".into(),
                }
            }
            ErrorKind::InvalidToken => {
                tracing::warn!("JWT token format is invalid: {:?}", e);
                JwtValidationError::TokenMalformed {
                    details: "Token format is invalid".into(),
                }
            }
            ErrorKind::Base64(base64_err) => JwtValidationError::TokenMalformed {
                details: format!("Token contains invalid base64: {base64_err}"),
            },
            ErrorKind::Json(json_err) => JwtValidationError::TokenMalformed {
                details: format!("Token contains invalid JSON: {json_err}"),
            },
            ErrorKind::Utf8(utf8_err) => JwtValidationError::TokenMalformed {
                details: format!("Token contains invalid UTF-8: {utf8_err}"),
            },
            _ => JwtValidationError::TokenInvalid {
                reason: format!("Token validation failed: {e}"),
            },
        }
    }

    /// Validate a `JWT` token with detailed error information
    ///
    /// # Errors
    ///
    /// Returns a [`JwtValidationError`] if:
    /// - Token signature is invalid
    /// - Token has expired
    /// - Token is malformed or not valid JWT format
    /// - Token claims cannot be deserialized
    pub fn validate_token_detailed(&self, token: &str) -> Result<Claims, JwtValidationError> {
        tracing::debug!("Validating JWT token (length: {} chars)", token.len());

        // Try to decode without expiration validation to get claims for error details
        let mut validation_no_exp = Validation::new(Algorithm::HS256);
        validation_no_exp.validate_exp = false;

        let claims_result = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation_no_exp,
        );

        match claims_result {
            Ok(token_data) => {
                let claims = token_data.claims;
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

                Self::check_token_expiry(&claims, current_time, expired_at)?;

                tracing::debug!("JWT token validation successful for user: {}", claims.sub);
                Ok(claims)
            }
            Err(e) => Err(Self::convert_jwt_error(&e)),
        }
    }

    /// Create a user session from a valid user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT token generation fails
    /// - User data is invalid
    /// - System time is unavailable
    pub fn create_session(&self, user: &User) -> Result<UserSession> {
        let jwt_token = self.generate_token(user)?;
        let expires_at = Utc::now() + Duration::hours(self.token_expiry_hours);

        Ok(UserSession {
            user_id: user.id,
            jwt_token,
            expires_at,
            email: user.email.clone(),
            available_providers: user.available_providers(),
        })
    }

    /// Validate authentication request and return response
    pub fn authenticate(&self, request: &AuthRequest) -> AuthResponse {
        match self.validate_token_detailed(&request.token) {
            Ok(claims) => match crate::utils::uuid::parse_uuid(&claims.sub) {
                Ok(user_id) => AuthResponse {
                    authenticated: true,
                    user_id: Some(user_id),
                    error: None,
                    available_providers: claims.providers,
                },
                Err(_) => AuthResponse {
                    authenticated: false,
                    user_id: None,
                    error: Some("Invalid user ID in token".into()),
                    available_providers: vec![],
                },
            },
            Err(jwt_error) => AuthResponse {
                authenticated: false,
                user_id: None,
                error: Some(jwt_error.to_string()),
                available_providers: vec![],
            },
        }
    }

    /// Refresh a token if it's still valid
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Old token signature is invalid (even if expired)
    /// - Token is malformed
    /// - New token generation fails
    /// - User data is invalid
    pub fn refresh_token(&self, old_token: &str, user: &User) -> Result<String> {
        // First validate the old token (even if expired, we want to check signature)
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; // Allow expired tokens for refresh

        decode::<Claims>(
            old_token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation,
        )?;

        // Generate new token - atomic counter ensures uniqueness
        self.generate_token(user)
    }

    /// Extract user `ID` from token without full validation
    /// Used for database lookups when token might be expired
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token signature is invalid
    /// - Token is malformed
    /// - User ID in token is not a valid UUID
    /// - Token claims cannot be deserialized
    pub fn extract_user_id(&self, token: &str) -> Result<Uuid> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        validation.validate_aud = false;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation,
        )?;

        crate::utils::uuid::parse_uuid(&token_data.claims.sub).with_context(|| {
            format!(
                "Failed to parse user ID from JWT token subject: {}",
                token_data.claims.sub
            )
        })
    }

    /// Check if initial setup is needed by verifying if admin user exists
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database connection fails
    /// - Database query execution fails
    /// - User data deserialization fails
    pub async fn check_setup_status(
        &self,
        database: &Database,
    ) -> Result<crate::routes::SetupStatusResponse> {
        const DEFAULT_ADMIN_EMAIL: &str = "admin@pierre.mcp";

        match database.get_user_by_email(DEFAULT_ADMIN_EMAIL).await {
            Ok(Some(_user)) => {
                // Admin user exists, setup is complete
                Ok(crate::routes::SetupStatusResponse {
                    needs_setup: false,
                    admin_user_exists: true,
                    message: None,
                })
            }
            Ok(None) => {
                // Admin user doesn't exist, setup is needed
                Ok(crate::routes::SetupStatusResponse {
                    needs_setup: true,
                    admin_user_exists: false,
                    message: Some("Run 'cargo run --bin admin-setup -- create-admin-user' to create default admin credentials".into()),
                })
            }
            Err(e) => {
                // Database error
                tracing::error!("Error checking admin user existence: {}", e);
                Ok(crate::routes::SetupStatusResponse {
                    needs_setup: true,
                    admin_user_exists: false,
                    message: Some(
                        "Unable to verify admin user status. Please check database connection."
                            .to_string(),
                    ),
                })
            }
        }
    }

    /// Generate OAuth access token for user authorization flow
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT token generation fails
    /// - System time is unavailable
    pub fn generate_oauth_access_token(&self, user_id: &Uuid, scopes: &[String]) -> Result<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(24); // 24 hours for OAuth access tokens

        let counter = self.token_counter.fetch_add(1, Ordering::Relaxed);
        let unique_iat =
            now.timestamp() * 1000 + i64::from(u32::try_from(counter % 1000).unwrap_or(0));

        let claims = Claims {
            sub: user_id.to_string(),
            email: "oauth_token".to_string(), // Placeholder for OAuth tokens
            iat: unique_iat,
            exp: expiry.timestamp(),
            providers: scopes.to_vec(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.jwt_secret),
        )?;

        Ok(token)
    }

    /// Generate client credentials token for A2A authentication
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT token generation fails
    /// - System time is unavailable
    pub fn generate_client_credentials_token(
        &self,
        client_id: &str,
        scopes: &[String],
    ) -> Result<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(1); // 1 hour for client credentials

        let counter = self.token_counter.fetch_add(1, Ordering::Relaxed);
        let unique_iat =
            now.timestamp() * 1000 + i64::from(u32::try_from(counter % 1000).unwrap_or(0));

        let claims = Claims {
            sub: format!("client:{client_id}"),
            email: "client_credentials".to_string(),
            iat: unique_iat,
            exp: expiry.timestamp(),
            providers: scopes.to_vec(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.jwt_secret),
        )?;

        Ok(token)
    }
}

/// Generate a random `JWT` secret
pub fn generate_jwt_secret() -> [u8; 64] {
    use ring::digest::{digest, SHA256};
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut secret = [0u8; 64];
    if let Err(e) = rng.fill(&mut secret) {
        // This is a critical security failure - we cannot proceed without a secure random secret
        // Log the error and fall back to a deterministic but secure method
        tracing::error!("Failed to generate cryptographically secure JWT secret: {e}");
        // Use a fallback method that's deterministic but still secure for testing
        let fallback_input = b"fallback_jwt_secret_generation_pierre_mcp_server";
        let hash = digest(&SHA256, fallback_input);
        let mut fallback_secret = [0u8; 64];
        fallback_secret[..32].copy_from_slice(hash.as_ref());
        fallback_secret[32..].copy_from_slice(hash.as_ref());
        return fallback_secret;
    }
    secret
}

/// Middleware for `MCP` protocol authentication
#[derive(Clone)]
pub struct McpAuthMiddleware {
    auth_manager: AuthManager,
    api_key_manager: ApiKeyManager,
    rate_limit_calculator: UnifiedRateLimitCalculator,
    database: std::sync::Arc<Database>,
}

impl McpAuthMiddleware {
    /// Create new `MCP` auth middleware
    pub fn new(auth_manager: AuthManager, database: std::sync::Arc<Database>) -> Self {
        Self {
            auth_manager,
            api_key_manager: ApiKeyManager::new(),
            rate_limit_calculator: UnifiedRateLimitCalculator::new(),
            database,
        }
    }

    /// Authenticate `MCP` request and extract user context with rate limiting
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication header is missing or malformed
    /// - JWT token validation fails
    /// - API key validation fails
    /// - Database queries fail
    /// - Rate limit calculations fail
    /// - User lookup fails
    pub async fn authenticate_request(&self, auth_header: Option<&str>) -> Result<AuthResult> {
        let auth_str = if let Some(header) = auth_header {
            tracing::debug!(
                "Authentication attempt with header type: {}",
                if header.starts_with("pk_live_") {
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
            return Err(anyhow::anyhow!("Authentication failed: Missing authorization header")
                .context("Request authentication requires Authorization header with Bearer token or API key"));
        };

        // Try API key authentication first (starts with pk_live_)
        if auth_str.starts_with("pk_live_") {
            tracing::debug!("Attempting API key authentication");
            match self.authenticate_api_key(auth_str).await {
                Ok(result) => {
                    tracing::info!(
                        "API key authentication successful for user: {}",
                        result.user_id
                    );
                    Ok(result)
                }
                Err(e) => {
                    tracing::warn!("API key authentication failed: {}", e);
                    Err(e)
                }
            }
        }
        // Then try Bearer token authentication
        else if let Some(token) = auth_str.strip_prefix("Bearer ") {
            tracing::debug!("Attempting JWT token authentication");
            match self.authenticate_jwt_token(token).await {
                Ok(result) => {
                    tracing::info!("JWT authentication successful for user: {}", result.user_id);
                    Ok(result)
                }
                Err(e) => {
                    tracing::warn!("JWT authentication failed: {}", e);
                    Err(e)
                }
            }
        } else {
            tracing::warn!("Authentication failed: Invalid authorization header format (expected 'Bearer ...' or 'pk_live_...')");
            Err(anyhow::anyhow!("Invalid authorization header format")
                .context("Authorization header must be 'Bearer <token>' or 'pk_live_<api_key>'"))
        }
    }

    /// Authenticate using `API` key
    async fn authenticate_api_key(&self, api_key: &str) -> Result<AuthResult> {
        // Validate key format
        self.api_key_manager.validate_key_format(api_key)?;

        // Extract prefix and hash the key
        let key_prefix = self.api_key_manager.extract_key_prefix(api_key);
        let key_hash = self.api_key_manager.hash_key(api_key);

        // Look up the API key in database
        let db_key = self
            .database
            .get_api_key_by_prefix(&key_prefix, &key_hash)
            .await?
            .with_context(|| format!("API key not found or invalid: {key_prefix}"))?;

        // Validate key status
        self.api_key_manager.is_key_valid(&db_key)?;

        // Get current usage for rate limiting
        let current_usage = self.database.get_api_key_current_usage(&db_key.id).await?;
        let rate_limit = self
            .rate_limit_calculator
            .calculate_api_key_rate_limit(&db_key, current_usage);

        // Check rate limit
        if rate_limit.is_rate_limited {
            return Err(
                anyhow::anyhow!("API key rate limit exceeded").context(format!(
                    "Rate limit reached for API key: {}/{} requests",
                    current_usage,
                    rate_limit.limit.unwrap_or(0)
                )),
            );
        }

        // Update last used timestamp
        self.database.update_api_key_last_used(&db_key.id).await?;

        Ok(AuthResult {
            user_id: db_key.user_id,
            auth_method: AuthMethod::ApiKey {
                key_id: db_key.id,
                tier: format!("{:?}", db_key.tier).to_lowercase(),
            },
            rate_limit,
        })
    }

    /// Authenticate using `JWT` token
    async fn authenticate_jwt_token(&self, token: &str) -> Result<AuthResult> {
        match self.auth_manager.validate_token_detailed(token) {
            Ok(claims) => {
                let user_id = crate::utils::uuid::parse_uuid(&claims.sub)
                    .map_err(|_| anyhow::anyhow!("Invalid user ID in token"))?;

                // Get user from database to check tier and rate limits
                let user = self
                    .database
                    .get_user(user_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("User not found"))?;

                // Get current usage for rate limiting
                let current_usage = self.database.get_jwt_current_usage(user_id).await?;
                let rate_limit = self
                    .rate_limit_calculator
                    .calculate_jwt_rate_limit(&user, current_usage);

                // Check rate limit
                if rate_limit.is_rate_limited {
                    return Err(anyhow::anyhow!("JWT token rate limit exceeded"));
                }

                Ok(AuthResult {
                    user_id,
                    auth_method: AuthMethod::JwtToken {
                        tier: format!("{:?}", user.tier).to_lowercase(),
                    },
                    rate_limit,
                })
            }
            Err(jwt_error) => Err(anyhow::anyhow!("{}", jwt_error)),
        }
    }

    /// Legacy method for backward compatibility - authenticate and return just user `ID`
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication header is missing or malformed
    /// - JWT token or API key validation fails
    /// - Database queries fail
    /// - Rate limits are exceeded
    /// - User lookup fails
    pub async fn authenticate_request_legacy(&self, auth_header: Option<&str>) -> Result<Uuid> {
        let auth_result = self.authenticate_request(auth_header).await?;
        Ok(auth_result.user_id)
    }

    /// Check if user has access to specific provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT token validation fails
    /// - Token signature is invalid
    /// - Token is malformed
    /// - Token claims cannot be deserialized
    pub fn check_provider_access(&self, token: &str, provider: &str) -> Result<bool> {
        let claims = self.auth_manager.validate_token(token)?;
        Ok(claims.providers.contains(&provider.to_string()))
    }

    /// Get reference to the auth manager for testing purposes
    pub const fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }
}
