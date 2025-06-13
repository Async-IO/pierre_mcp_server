// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Authentication and Session Management
//!
//! This module provides JWT-based authentication and session management
//! for the multi-tenant Pierre MCP Server.

use crate::models::{User, UserSession, AuthRequest, AuthResponse};
use crate::api_keys::{ApiKeyManager, RateLimitStatus};
use crate::database::Database;
use anyhow::Result;
use chrono::{Utc, Duration};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
        match self.validate_token(&request.token) {
            Ok(claims) => {
                match Uuid::parse_str(&claims.sub) {
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
                }
            }
            Err(e) => AuthResponse {
                authenticated: false,
                user_id: None,
                error: Some(format!("Token validation failed: {}", e)),
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
    rng.fill(&mut secret).expect("Failed to generate JWT secret");
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
        let auth_str = auth_header
            .ok_or_else(|| anyhow::anyhow!("Missing authorization header"))?;

        // Try API key authentication first (starts with pk_live_)
        if auth_str.starts_with("pk_live_") {
            self.authenticate_api_key(auth_str).await
        }
        // Then try Bearer token authentication
        else if let Some(token) = auth_str.strip_prefix("Bearer ") {
            self.authenticate_jwt_token(token).await
        }
        else {
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
        let db_key = self.database.get_api_key_by_prefix(&key_prefix, &key_hash).await?
            .ok_or_else(|| anyhow::anyhow!("Invalid API key"))?;

        // Validate key status
        self.api_key_manager.is_key_valid(&db_key)?;

        // Get current usage for rate limiting
        let current_usage = self.database.get_api_key_current_usage(&db_key.id).await?;
        let rate_limit = self.api_key_manager.calculate_rate_limit_status(&db_key, current_usage);

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
        let claims = self.auth_manager.validate_token(token)?;
        let user_id = Uuid::parse_str(&claims.sub)?;
        
        Ok(AuthResult {
            user_id,
            auth_method: AuthMethod::JwtToken,
            rate_limit: None,
        })
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
            Some("Test User".to_string())
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
            token: "invalid.jwt.token".to_string() 
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
        use crate::database::{Database, generate_encryption_key};
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

        let auth_result = middleware.authenticate_request(Some(&auth_header)).await.unwrap();
        assert_eq!(auth_result.user_id, user.id);
        assert!(matches!(auth_result.auth_method, crate::auth::AuthMethod::JwtToken));
    }

    #[tokio::test]
    async fn test_mcp_auth_middleware_invalid_header() {
        use crate::database::{Database, generate_encryption_key};
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
        let result = middleware.authenticate_request(Some("Invalid header")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_provider_access_check() {
        use crate::database::{Database, generate_encryption_key};
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
}