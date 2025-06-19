// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Authentication and Session Management
//!
//! This module provides JWT-based authentication and session management
//! for the multi-tenant Pierre MCP Server.

use crate::api_keys::{ApiKeyManager, RateLimitStatus};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::models::{AuthRequest, AuthResponse, User, UserSession};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Convert a duration to a human-readable format
fn humanize_duration(duration: Duration) -> String {
    let total_secs = duration.num_seconds().abs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;

    if hours > 0 {
        format!("{} hours", hours)
    } else if minutes > 0 {
        format!("{} minutes", minutes)
    } else {
        format!("{} seconds", total_secs)
    }
}

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
            JwtValidationError::TokenExpired {
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
            JwtValidationError::TokenInvalid { reason } => {
                write!(f, "JWT token signature is invalid: {}", reason)
            }
            JwtValidationError::TokenMalformed { details } => {
                write!(f, "JWT token is malformed: {}", details)
            }
        }
    }
}

impl std::error::Error for JwtValidationError {}

/// JWT claims for user authentication
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// User ID
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
    /// Authenticated user ID
    pub user_id: Uuid,
    /// Authentication method used
    pub auth_method: AuthMethod,
    /// Rate limit status (only for API keys)
    pub rate_limit: Option<RateLimitStatus>,
}

/// Authentication method used
#[derive(Debug, Clone)]
pub enum AuthMethod {
    /// JWT token authentication
    JwtToken,
    /// API key authentication
    ApiKey {
        /// API key ID
        key_id: String,
        /// API key tier
        tier: String,
    },
}

impl AuthMethod {
    /// Get a human-readable display name for the authentication method
    pub fn display_name(&self) -> &str {
        match self {
            AuthMethod::JwtToken => "JWT Token",
            AuthMethod::ApiKey { .. } => "API Key",
        }
    }

    /// Get detailed information about the authentication method
    pub fn details(&self) -> String {
        match self {
            AuthMethod::JwtToken => "JWT Token".to_string(),
            AuthMethod::ApiKey { key_id, tier } => {
                format!("API Key (tier: {}, id: {})", tier, key_id)
            }
        }
    }
}

/// Authentication manager for JWT tokens and user sessions
#[derive(Clone)]
pub struct AuthManager {
    jwt_secret: Vec<u8>,
    token_expiry_hours: i64,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(jwt_secret: Vec<u8>, token_expiry_hours: i64) -> Self {
        Self {
            jwt_secret,
            token_expiry_hours,
        }
    }

    /// Generate a JWT token for a user
    pub fn generate_token(&self, user: &User) -> Result<String> {
        let now = Utc::now();
        let expiry = now + Duration::hours(self.token_expiry_hours);

        let claims = Claims {
            sub: user.id.to_string(),
            email: user.email.clone(),
            iat: now.timestamp(),
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

    /// Validate a JWT token and extract claims
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

    /// Validate a JWT token with detailed error information
    pub fn validate_token_detailed(&self, token: &str) -> Result<Claims, JwtValidationError> {
        tracing::debug!("Validating JWT token (length: {} chars)", token.len());

        // First, try to decode without expiration validation to get claims for error details
        let mut validation_no_exp = Validation::new(Algorithm::HS256);
        validation_no_exp.validate_exp = false;

        // Check if the token is properly formatted first
        let claims_result = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation_no_exp,
        );

        match claims_result {
            Ok(token_data) => {
                let claims = token_data.claims;
                let current_time = Utc::now();
                let exp_timestamp = claims.exp;
                let expired_at =
                    DateTime::from_timestamp(exp_timestamp, 0).unwrap_or_else(Utc::now);

                tracing::debug!(
                    "Token validation details - User: {}, Issued: {}, Expires: {}, Current: {}",
                    claims.sub,
                    DateTime::from_timestamp(claims.iat, 0)
                        .map(|d| d.to_rfc3339())
                        .unwrap_or_else(|| "unknown".to_string()),
                    expired_at.to_rfc3339(),
                    current_time.to_rfc3339()
                );

                // Check if token is expired
                if current_time.timestamp() > exp_timestamp {
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

                tracing::debug!("JWT token validation successful for user: {}", claims.sub);
                // Token is valid and not expired
                Ok(claims)
            }
            Err(e) => {
                // Check the specific type of JWT error
                use jsonwebtoken::errors::ErrorKind;
                tracing::warn!("JWT token validation failed: {:?}", e);

                match e.kind() {
                    ErrorKind::InvalidSignature => {
                        tracing::warn!("JWT token signature verification failed");
                        Err(JwtValidationError::TokenInvalid {
                            reason: "Token signature verification failed".to_string(),
                        })
                    }
                    ErrorKind::InvalidToken => {
                        tracing::warn!("JWT token format is invalid: {:?}", e);
                        Err(JwtValidationError::TokenMalformed {
                            details: "Token format is invalid".to_string(),
                        })
                    }
                    ErrorKind::Base64(base64_err) => Err(JwtValidationError::TokenMalformed {
                        details: format!("Token contains invalid base64: {}", base64_err),
                    }),
                    ErrorKind::Json(json_err) => Err(JwtValidationError::TokenMalformed {
                        details: format!("Token contains invalid JSON: {}", json_err),
                    }),
                    ErrorKind::Utf8(utf8_err) => Err(JwtValidationError::TokenMalformed {
                        details: format!("Token contains invalid UTF-8: {}", utf8_err),
                    }),
                    _ => Err(JwtValidationError::TokenInvalid {
                        reason: format!("Token validation failed: {}", e),
                    }),
                }
            }
        }
    }

    /// Create a user session from a valid user
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
    pub fn authenticate(&self, request: AuthRequest) -> AuthResponse {
        match self.validate_token_detailed(&request.token) {
            Ok(claims) => match Uuid::parse_str(&claims.sub) {
                Ok(user_id) => AuthResponse {
                    authenticated: true,
                    user_id: Some(user_id),
                    error: None,
                    available_providers: claims.providers,
                },
                Err(_) => AuthResponse {
                    authenticated: false,
                    user_id: None,
                    error: Some("Invalid user ID in token".to_string()),
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
    pub fn refresh_token(&self, old_token: &str, user: &User) -> Result<String> {
        // First validate the old token (even if expired, we want to check signature)
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false; // Allow expired tokens for refresh

        let _token_data = decode::<Claims>(
            old_token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation,
        )?;

        // Wait to ensure different timestamp
        std::thread::sleep(std::time::Duration::from_millis(1));

        // Generate new token
        self.generate_token(user)
    }

    /// Extract user ID from token without full validation
    /// Used for database lookups when token might be expired
    pub fn extract_user_id(&self, token: &str) -> Result<Uuid> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = false;
        validation.validate_aud = false;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(&self.jwt_secret),
            &validation,
        )?;

        Uuid::parse_str(&token_data.claims.sub)
            .map_err(|e| anyhow::anyhow!("Invalid user ID in token: {}", e))
    }
}

/// Generate a random JWT secret
pub fn generate_jwt_secret() -> [u8; 64] {
    use ring::rand::{SecureRandom, SystemRandom};

    let rng = SystemRandom::new();
    let mut secret = [0u8; 64];
    rng.fill(&mut secret)
        .expect("Failed to generate JWT secret");
    secret
}

/// Middleware for MCP protocol authentication
#[derive(Clone)]
pub struct McpAuthMiddleware {
    auth_manager: AuthManager,
    api_key_manager: ApiKeyManager,
    database: std::sync::Arc<Database>,
}

impl McpAuthMiddleware {
    /// Create new MCP auth middleware
    pub fn new(auth_manager: AuthManager, database: std::sync::Arc<Database>) -> Self {
        Self {
            auth_manager,
            api_key_manager: ApiKeyManager::new(),
            database,
        }
    }

    /// Authenticate MCP request and extract user context with rate limiting
    pub async fn authenticate_request(&self, auth_header: Option<&str>) -> Result<AuthResult> {
        let auth_str = match auth_header {
            Some(header) => {
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
            }
            None => {
                tracing::warn!("Authentication failed: Missing authorization header");
                return Err(anyhow::anyhow!("Missing authorization header"));
            }
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
            Err(anyhow::anyhow!("Invalid authorization header format"))
        }
    }

    /// Authenticate using API key
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
            .ok_or_else(|| anyhow::anyhow!("Invalid API key"))?;

        // Validate key status
        self.api_key_manager.is_key_valid(&db_key)?;

        // Get current usage for rate limiting
        let current_usage = self.database.get_api_key_current_usage(&db_key.id).await?;
        let rate_limit = self
            .api_key_manager
            .calculate_rate_limit_status(&db_key, current_usage);

        // Check rate limit
        if rate_limit.is_rate_limited {
            return Err(anyhow::anyhow!("API key rate limit exceeded"));
        }

        // Update last used timestamp
        self.database.update_api_key_last_used(&db_key.id).await?;

        Ok(AuthResult {
            user_id: db_key.user_id,
            auth_method: AuthMethod::ApiKey {
                key_id: db_key.id,
                tier: format!("{:?}", db_key.tier).to_lowercase(),
            },
            rate_limit: Some(rate_limit),
        })
    }

    /// Authenticate using JWT token
    async fn authenticate_jwt_token(&self, token: &str) -> Result<AuthResult> {
        match self.auth_manager.validate_token_detailed(token) {
            Ok(claims) => {
                let user_id = Uuid::parse_str(&claims.sub)
                    .map_err(|_| anyhow::anyhow!("Invalid user ID in token"))?;

                Ok(AuthResult {
                    user_id,
                    auth_method: AuthMethod::JwtToken,
                    rate_limit: None,
                })
            }
            Err(jwt_error) => Err(anyhow::anyhow!("{}", jwt_error)),
        }
    }

    /// Legacy method for backward compatibility - authenticate and return just user ID
    pub async fn authenticate_request_legacy(&self, auth_header: Option<&str>) -> Result<Uuid> {
        let auth_result = self.authenticate_request(auth_header).await?;
        Ok(auth_result.user_id)
    }

    /// Check if user has access to specific provider
    pub fn check_provider_access(&self, token: &str, provider: &str) -> Result<bool> {
        let claims = self.auth_manager.validate_token(token)?;
        Ok(claims.providers.contains(&provider.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::User;

    fn create_test_user() -> User {
        User::new(
            "test@example.com".to_string(),
            "hashed_password_123".to_string(),
            Some("Test User".to_string()),
        )
    }

    fn create_auth_manager() -> AuthManager {
        let secret = generate_jwt_secret().to_vec();
        AuthManager::new(secret, 24) // 24 hour expiry
    }

    #[test]
    fn test_generate_and_validate_token() {
        let auth_manager = create_auth_manager();
        let user = create_test_user();

        // Generate token
        let token = auth_manager.generate_token(&user).unwrap();
        assert!(!token.is_empty());

        // Validate token
        let claims = auth_manager.validate_token(&token).unwrap();
        assert_eq!(claims.email, "test@example.com");
        assert_eq!(claims.sub, user.id.to_string());
        assert!(claims.exp > Utc::now().timestamp());
    }

    #[test]
    fn test_create_session() {
        let auth_manager = create_auth_manager();
        let user = create_test_user();

        let session = auth_manager.create_session(&user).unwrap();
        assert_eq!(session.user_id, user.id);
        assert_eq!(session.email, "test@example.com");
        assert!(!session.jwt_token.is_empty());
        assert!(session.expires_at > Utc::now());
    }

    #[test]
    fn test_authenticate_request() {
        let auth_manager = create_auth_manager();
        let user = create_test_user();

        let token = auth_manager.generate_token(&user).unwrap();
        let auth_request = AuthRequest { token };

        let response = auth_manager.authenticate(auth_request);
        assert!(response.authenticated);
        assert_eq!(response.user_id, Some(user.id));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_authenticate_invalid_token() {
        let auth_manager = create_auth_manager();
        let auth_request = AuthRequest {
            token: "invalid.jwt.token".to_string(),
        };

        let response = auth_manager.authenticate(auth_request);
        assert!(!response.authenticated);
        assert!(response.user_id.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_refresh_token() {
        let auth_manager = create_auth_manager();
        let user = create_test_user();

        let original_token = auth_manager.generate_token(&user).unwrap();
        let refreshed_token = auth_manager.refresh_token(&original_token, &user).unwrap();

        // Both tokens should be valid (tokens might be identical if generated within same second)

        let original_claims = auth_manager.validate_token(&original_token).unwrap();
        let refreshed_claims = auth_manager.validate_token(&refreshed_token).unwrap();

        assert_eq!(original_claims.sub, refreshed_claims.sub);
        assert_eq!(original_claims.email, refreshed_claims.email);
        // Note: expiry times might be the same if generated within the same second
    }

    #[test]
    fn test_extract_user_id() {
        let auth_manager = create_auth_manager();
        let user = create_test_user();

        let token = auth_manager.generate_token(&user).unwrap();
        let extracted_id = auth_manager.extract_user_id(&token).unwrap();

        assert_eq!(extracted_id, user.id);
    }

    #[tokio::test]
    async fn test_mcp_auth_middleware() {
        use crate::database::generate_encryption_key;
        use crate::database_plugins::factory::Database;
        use std::sync::Arc;

        let auth_manager = create_auth_manager();
        let user = create_test_user();

        // Create in-memory database for testing
        let database_url = "sqlite::memory:";
        let encryption_key = generate_encryption_key().to_vec();
        let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

        let middleware = McpAuthMiddleware::new(auth_manager, database);

        let token = middleware.auth_manager.generate_token(&user).unwrap();
        let auth_header = format!("Bearer {}", token);

        let auth_result = middleware
            .authenticate_request(Some(&auth_header))
            .await
            .unwrap();
        assert_eq!(auth_result.user_id, user.id);
        assert!(matches!(
            auth_result.auth_method,
            crate::auth::AuthMethod::JwtToken
        ));
    }

    #[tokio::test]
    async fn test_mcp_auth_middleware_invalid_header() {
        use crate::database::generate_encryption_key;
        use crate::database_plugins::factory::Database;
        use std::sync::Arc;

        let auth_manager = create_auth_manager();

        // Create in-memory database for testing
        let database_url = "sqlite::memory:";
        let encryption_key = generate_encryption_key().to_vec();
        let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

        let middleware = McpAuthMiddleware::new(auth_manager, database);

        // Test missing header
        let result = middleware.authenticate_request(None).await;
        assert!(result.is_err());

        // Test invalid format
        let result = middleware
            .authenticate_request(Some("Invalid header"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_provider_access_check() {
        use crate::database::generate_encryption_key;
        use crate::database_plugins::factory::Database;
        use std::sync::Arc;

        let auth_manager = create_auth_manager();
        let user = create_test_user();

        // Create in-memory database for testing
        let database_url = "sqlite::memory:";
        let encryption_key = generate_encryption_key().to_vec();
        let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

        let middleware = McpAuthMiddleware::new(auth_manager, database);

        // User has no providers initially
        let token = middleware.auth_manager.generate_token(&user).unwrap();

        let has_strava = middleware.check_provider_access(&token, "strava").unwrap();
        assert!(!has_strava);
    }

    #[test]
    fn test_jwt_detailed_validation_invalid_token() {
        let auth_manager = create_auth_manager();

        // Test with malformed token
        let result = auth_manager.validate_token_detailed("invalid.jwt.token");
        assert!(result.is_err());

        match result.unwrap_err() {
            JwtValidationError::TokenMalformed { details } => {
                assert!(details.contains("Token"));
            }
            _ => panic!("Expected TokenMalformed error"),
        }
    }

    #[test]
    fn test_jwt_detailed_validation_expired_token() {
        let user = create_test_user();

        // Create an auth manager with very short expiry for testing
        let secret = generate_jwt_secret().to_vec();
        let short_expiry_auth_manager = AuthManager::new(secret.clone(), -1); // Expired 1 hour ago

        let expired_token = short_expiry_auth_manager.generate_token(&user).unwrap();

        // Wait a moment to ensure expiration check works
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Use the same auth manager to validate the token (same secret)
        let result = short_expiry_auth_manager.validate_token_detailed(&expired_token);
        assert!(result.is_err());

        match result.unwrap_err() {
            JwtValidationError::TokenExpired {
                expired_at,
                current_time,
            } => {
                assert!(current_time > expired_at);
            }
            other => panic!("Expected TokenExpired error, got: {:?}", other),
        }
    }

    #[test]
    fn test_jwt_detailed_validation_invalid_signature() {
        let auth_manager1 = create_auth_manager();
        let auth_manager2 = create_auth_manager(); // Different secret
        let user = create_test_user();

        // Create token with one auth manager, validate with another
        let token = auth_manager1.generate_token(&user).unwrap();

        let result = auth_manager2.validate_token_detailed(&token);
        assert!(result.is_err());

        match result.unwrap_err() {
            JwtValidationError::TokenInvalid { reason } => {
                assert!(reason.contains("signature"));
            }
            _ => panic!("Expected TokenInvalid error"),
        }
    }

    #[test]
    fn test_enhanced_authenticate_response() {
        let user = create_test_user();

        // Test with expired token - use same auth manager for validation
        let secret = generate_jwt_secret().to_vec();
        let expired_auth_manager = AuthManager::new(secret, -1);
        let expired_token = expired_auth_manager.generate_token(&user).unwrap();

        let auth_request = AuthRequest {
            token: expired_token,
        };
        let response = expired_auth_manager.authenticate(auth_request);

        assert!(!response.authenticated);
        assert!(response.error.is_some());
        let error_msg = response.error.unwrap();
        assert!(error_msg.contains("JWT token expired"));
    }
}
