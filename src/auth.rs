// ABOUTME: JWT-based user authentication and authorization system
// ABOUTME: Handles user login, token generation, validation, and session management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Authentication and Session Management
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource sharing for auth managers across threads
// - String ownership for JWT claims and session data
// - Database result ownership transfers
//!
//! This module provides JWT-based authentication and session management
//! for the multi-tenant Pierre MCP Server.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{
    decode, encode,
    errors::{Error as JwtError, ErrorKind},
    Algorithm, Header, Validation,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, warn};
use uuid::Uuid;

use crate::admin::jwks::JwksManager;
use crate::constants::{
    limits::{OAUTH_ACCESS_TOKEN_EXPIRY_HOURS, USER_SESSION_EXPIRY_HOURS},
    service_names::{MCP, PIERRE_MCP_SERVER},
    time_constants::SECONDS_PER_HOUR,
};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::models::{AuthRequest, AuthResponse, User, UserSession};
use crate::rate_limiting::UnifiedRateLimitInfo;
use crate::utils::uuid::parse_uuid;

/// Response for checking system setup status
///
/// Used by admin setup flow to determine if initial configuration is needed.
#[derive(Debug, Serialize)]
pub struct SetupStatusResponse {
    /// Whether the system needs initial setup
    pub needs_setup: bool,
    /// Whether an admin user already exists
    pub admin_user_exists: bool,
    /// Optional status message
    pub message: Option<String>,
}

/// Convert a duration to a human-readable format
fn humanize_duration(duration: Duration) -> String {
    let total_secs = duration.num_seconds().abs();
    let hours = total_secs / i64::from(SECONDS_PER_HOUR);
    let minutes = (total_secs % i64::from(SECONDS_PER_HOUR)) / 60;

    if hours > 0 {
        format!("{hours} hours")
    } else if minutes > 0 {
        format!("{minutes} minutes")
    } else {
        format!("{total_secs} seconds")
    }
}

/// `JWT` validation error with detailed information
#[non_exhaustive]
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

impl Display for JwtValidationError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

impl Error for JwtValidationError {}

/// `JWT` claims for user authentication
///
/// The `active_tenant_id` field represents the currently active tenant context
/// for this session. Users can belong to multiple tenants (via `tenant_users` table)
/// and switch between them by obtaining a new token with a different `active_tenant_id`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// User `ID`
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
    /// Active tenant `ID` for this session (user can belong to multiple tenants)
    /// This is the tenant context for all operations in this session.
    /// Use POST /api/tenants/switch to change the active tenant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_tenant_id: Option<String>,
    /// Original user ID when impersonating (the super admin)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonator_id: Option<String>,
    /// Impersonation session ID for audit trail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub impersonation_session_id: Option<String>,
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
    /// Active tenant ID from JWT claims (for multi-tenant user tenant selection)
    /// Users can belong to multiple tenants and this field indicates which tenant
    /// should be used for the current request. Extracted from JWT `active_tenant_id` claim.
    pub active_tenant_id: Option<Uuid>,
}

/// Authentication method used
#[non_exhaustive]
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
    token_expiry_hours: i64,
}

impl Clone for AuthManager {
    fn clone(&self) -> Self {
        Self {
            token_expiry_hours: self.token_expiry_hours,
        }
    }
}

impl AuthManager {
    /// Create a new authentication manager
    #[must_use]
    pub const fn new(token_expiry_hours: i64) -> Self {
        Self { token_expiry_hours }
    }

    /// Generate a `JWT` token for a user with RS256 asymmetric signing
    ///
    /// # Arguments
    /// * `user` - The user to generate a token for
    /// * `jwks_manager` - JWKS manager for signing
    /// * `active_tenant_id` - Optional active tenant ID for this session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT encoding fails due to invalid claims
    /// - System time is unavailable for timestamp generation
    /// - JWKS manager has no active key
    pub fn generate_token(&self, user: &User, jwks_manager: &JwksManager) -> AppResult<String> {
        self.generate_token_with_tenant(user, jwks_manager, None)
    }

    /// Generate a `JWT` token for a user with a specific active tenant
    ///
    /// # Arguments
    /// * `user` - The user to generate a token for
    /// * `jwks_manager` - JWKS manager for signing
    /// * `active_tenant_id` - The tenant ID to set as active for this session
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT encoding fails due to invalid claims
    /// - System time is unavailable for timestamp generation
    /// - JWKS manager has no active key
    pub fn generate_token_with_tenant(
        &self,
        user: &User,
        jwks_manager: &JwksManager,
        active_tenant_id: Option<String>,
    ) -> AppResult<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(self.token_expiry_hours);

        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            iat: now.timestamp(),
            exp: expiry.timestamp(),
            iss: PIERRE_MCP_SERVER.to_owned(),
            jti: Uuid::new_v4().to_string(),
            providers: user.available_providers(),
            aud: MCP.to_owned(),
            active_tenant_id,
            impersonator_id: None,
            impersonation_session_id: None,
        };

        // Get active RSA key from JWKS manager
        let active_key = jwks_manager.get_active_key()?;
        let encoding_key = active_key.encoding_key()?;

        // Create RS256 header with kid
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(active_key.kid.clone());

        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to encode JWT token: {e}")))?;

        Ok(token)
    }

    /// Generate an impersonation `JWT` token for a super admin impersonating another user
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JWT encoding fails due to invalid claims
    /// - System time is unavailable for timestamp generation
    /// - JWKS manager has no active key
    pub fn generate_impersonation_token(
        &self,
        target_user: &User,
        impersonator_id: Uuid,
        session_id: &str,
        jwks_manager: &JwksManager,
        active_tenant_id: Option<String>,
    ) -> AppResult<String> {
        let now = Utc::now();
        // Impersonation tokens have shorter expiry (1 hour)
        let expiry = now + Duration::hours(1);

        let claims = Claims {
            sub: target_user.id.to_string(),
            email: target_user.email.clone(),
            iat: now.timestamp(),
            exp: expiry.timestamp(),
            iss: PIERRE_MCP_SERVER.to_owned(),
            jti: Uuid::new_v4().to_string(),
            providers: target_user.available_providers(),
            aud: MCP.to_owned(),
            active_tenant_id,
            impersonator_id: Some(impersonator_id.to_string()),
            impersonation_session_id: Some(session_id.to_owned()),
        };

        // Get active RSA key from JWKS manager
        let active_key = jwks_manager.get_active_key()?;
        let encoding_key = active_key.encoding_key()?;

        // Create RS256 header with kid
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(active_key.kid.clone());

        let token = encode(&header, &claims, &encoding_key).map_err(|e| {
            AppError::internal(format!("Failed to encode impersonation JWT token: {e}"))
        })?;

        Ok(token)
    }

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
    pub fn validate_token(&self, token: &str, jwks_manager: &JwksManager) -> AppResult<Claims> {
        // Extract kid from token header
        let header = jsonwebtoken::decode_header(token)
            .map_err(|e| AppError::auth_invalid(format!("Failed to decode token header: {e}")))?;
        let kid = header.kid.ok_or_else(|| -> AppError {
            AppError::auth_invalid("Token header missing kid (key ID)")
        })?;

        debug!("Validating RS256 JWT token with kid: {}", kid);

        // Get public key from JWKS manager
        let key_pair = jwks_manager.get_key(&kid).ok_or_else(|| -> AppError {
            AppError::auth_invalid(format!("Key not found in JWKS: {kid}"))
        })?;

        let decoding_key = key_pair
            .decoding_key()
            .map_err(|e| AppError::auth_invalid(format!("Failed to get decoding key: {e}")))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;
        validation.set_audience(&[MCP]);
        validation.set_issuer(&[PIERRE_MCP_SERVER]);

        let token_data = decode::<Claims>(token, &decoding_key, &validation).map_err(|e| {
            error!("RS256 JWT validation failed: {:?}", e);
            AppError::auth_invalid(format!("JWT validation failed: {e}"))
        })?;

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
            warn!(
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
    fn convert_jwt_error(e: &JwtError) -> JwtValidationError {
        warn!("JWT token validation failed: {:?}", e);

        match e.kind() {
            ErrorKind::InvalidSignature => {
                warn!("JWT token signature verification failed");
                JwtValidationError::TokenInvalid {
                    reason: "Token signature verification failed".to_owned(),
                }
            }
            ErrorKind::InvalidToken => {
                warn!("JWT token format is invalid: {:?}", e);
                JwtValidationError::TokenMalformed {
                    details: "Token format is invalid".to_owned(),
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

    /// Validate a RS256 JWT token with detailed error information
    ///
    /// # Errors
    ///
    /// Returns a [`JwtValidationError`] if:
    /// - Token signature is invalid
    /// - Token has expired
    /// - Token is malformed or not valid JWT format
    /// - Token header doesn't contain kid (key ID)
    /// - JWKS manager doesn't have the specified key
    /// - Token claims cannot be deserialized
    pub fn validate_token_detailed(
        &self,
        token: &str,
        jwks_manager: &JwksManager,
    ) -> Result<Claims, JwtValidationError> {
        debug!("Validating RS256 JWT token (length: {} chars)", token.len());

        let claims = Self::decode_token_claims(token, jwks_manager)?;
        Self::validate_claims_expiry(&claims)?;

        debug!(
            "RS256 JWT token validation successful for user: {}",
            claims.sub
        );
        Ok(claims)
    }

    /// Decode RS256 JWT token claims without expiration validation
    fn decode_token_claims(
        token: &str,
        jwks_manager: &JwksManager,
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
        validation_no_exp.set_audience(&[MCP]);
        validation_no_exp.set_issuer(&[PIERRE_MCP_SERVER]);

        decode::<Claims>(token, &decoding_key, &validation_no_exp)
            .map(|token_data| token_data.claims)
            .map_err(|e| Self::convert_jwt_error(&e))
    }

    /// Validate claims expiration with detailed logging
    fn validate_claims_expiry(claims: &Claims) -> Result<(), JwtValidationError> {
        let current_time = Utc::now();
        let expired_at = DateTime::from_timestamp(claims.exp, 0).unwrap_or_else(Utc::now);

        debug!(
            "Token validation details - User: {}, Issued: {}, Expires: {}, Current: {}",
            claims.sub,
            DateTime::from_timestamp(claims.iat, 0)
                .map_or_else(|| "unknown".into(), |d| d.to_rfc3339()),
            expired_at.to_rfc3339(),
            current_time.to_rfc3339()
        );

        Self::check_token_expiry(claims, current_time, expired_at)
    }

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
        jwks_manager: &JwksManager,
    ) -> AppResult<UserSession> {
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

    /// Validate authentication request using RS256 and return response
    #[must_use]
    pub fn authenticate(&self, request: &AuthRequest, jwks_manager: &JwksManager) -> AuthResponse {
        match self.validate_token_detailed(&request.token, jwks_manager) {
            Ok(claims) => match parse_uuid(&claims.sub) {
                Ok(user_id) => AuthResponse {
                    authenticated: true,
                    user_id: Some(user_id),
                    error: None,
                    available_providers: claims.providers,
                },
                Err(e) => {
                    warn!(
                        sub = %claims.sub,
                        issuer = ?claims.iss,
                        error = %e,
                        "Invalid user ID in authentication token"
                    );
                    AuthResponse {
                        authenticated: false,
                        user_id: None,
                        error: Some("Invalid user ID in token".into()),
                        available_providers: vec![],
                    }
                }
            },
            Err(jwt_error) => AuthResponse {
                authenticated: false,
                user_id: None,
                error: Some(jwt_error.to_string()),
                available_providers: vec![],
            },
        }
    }

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
        jwks_manager: &JwksManager,
    ) -> AppResult<String> {
        // First validate the old token signature (even if expired)
        // This ensures the refresh request is legitimate
        Self::decode_token_claims(old_token, jwks_manager).map_err(|e| -> AppError {
            AppError::auth_invalid(format!("Failed to validate old token for refresh: {e}"))
        })?;

        // Generate new token - atomic counter ensures uniqueness
        self.generate_token(user, jwks_manager)
    }

    /// Check if initial setup is needed by verifying if any admin user exists
    ///
    /// This checks for any user with `is_admin=true` flag, consistent with how
    /// the admin setup endpoint validates existing admin users.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Database connection fails
    /// - Database query execution fails
    /// - User data deserialization fails
    pub async fn check_setup_status(&self, database: &Database) -> AppResult<SetupStatusResponse> {
        // Check for any active user with is_admin=true (consistent with admin setup endpoint)
        match database.get_users_by_status("active", None).await {
            Ok(users) => {
                let admin_exists = users.iter().any(|u| u.is_admin);
                if admin_exists {
                    // Admin user exists, setup is complete
                    Ok(SetupStatusResponse {
                        needs_setup: false,
                        admin_user_exists: true,
                        message: None,
                    })
                } else {
                    // No admin user exists, setup is needed
                    Ok(SetupStatusResponse {
                        needs_setup: true,
                        admin_user_exists: false,
                        message: Some("Run 'cargo run --bin pierre-cli -- user create' to create default admin credentials".into()),
                    })
                }
            }
            Err(e) => {
                // Database error
                error!("Error checking admin user existence: {}", e);
                Ok(SetupStatusResponse {
                    needs_setup: true,
                    admin_user_exists: false,
                    message: Some(
                        "Unable to verify admin user status. Please check database connection."
                            .to_owned(),
                    ),
                })
            }
        }
    }

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
        jwks_manager: &JwksManager,
        user_id: &Uuid,
        scopes: &[String],
        active_tenant_id: Option<String>,
    ) -> AppResult<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(OAUTH_ACCESS_TOKEN_EXPIRY_HOURS);

        let claims = Claims {
            sub: user_id.to_string(),
            email: format!("oauth_{user_id}@system.local"),
            iat: now.timestamp(),
            exp: expiry.timestamp(),
            iss: PIERRE_MCP_SERVER.to_owned(),
            jti: Uuid::new_v4().to_string(),
            providers: scopes.to_vec(),
            aud: MCP.to_owned(),
            active_tenant_id,
            impersonator_id: None,
            impersonation_session_id: None,
        };

        // Get active RSA key from JWKS manager
        let active_key = jwks_manager.get_active_key()?;
        let encoding_key = active_key.encoding_key()?;

        // Create RS256 header with kid
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(active_key.kid.clone());

        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to encode JWT token: {e}")))?;

        Ok(token)
    }

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
        jwks_manager: &JwksManager,
        client_id: &str,
        scopes: &[String],
        active_tenant_id: Option<String>,
    ) -> AppResult<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(1); // 1 hour for client credentials

        let claims = Claims {
            sub: format!("client:{client_id}"),
            email: "client_credentials".to_owned(),
            iat: now.timestamp(),
            exp: expiry.timestamp(),
            iss: PIERRE_MCP_SERVER.to_owned(),
            jti: Uuid::new_v4().to_string(),
            providers: scopes.to_vec(),
            aud: MCP.to_owned(),
            active_tenant_id,
            impersonator_id: None,
            impersonation_session_id: None,
        };

        // Get active RSA key from JWKS manager
        let active_key = jwks_manager.get_active_key()?;
        let encoding_key = active_key.encoding_key()?;

        // Create RS256 header with kid
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(active_key.kid.clone());

        let token = encode(&header, &claims, &encoding_key)
            .map_err(|e| AppError::internal(format!("Failed to encode JWT token: {e}")))?;

        Ok(token)
    }
}

/// Generate a random `JWT` secret
///
/// # Errors
/// Returns an error if system RNG fails - this is a critical security failure
/// and the server cannot operate securely without working RNG
pub fn generate_jwt_secret() -> AppResult<[u8; 64]> {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut secret = [0u8; 64];

    rng.fill(&mut secret).map_err(|e| {
        error!(
            "CRITICAL: Failed to generate cryptographically secure JWT secret: {}",
            e
        );
        AppError::internal("System RNG failure - cannot generate secure JWT secret")
    })?;

    Ok(secret)
}
