// ABOUTME: MCP authentication middleware for request authentication and authorization
// ABOUTME: Handles JWT tokens and API keys with rate limiting and user context extraction
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::api_keys::ApiKeyManager;
use crate::auth::{AuthManager, AuthMethod, AuthResult};
use crate::constants::key_prefixes;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::providers::errors::ProviderError;
use crate::rate_limiting::UnifiedRateLimitCalculator;
use crate::utils::errors::auth_error;

/// Middleware for `MCP` protocol authentication
#[derive(Clone)]
pub struct McpAuthMiddleware {
    auth_manager: AuthManager,
    api_key_manager: ApiKeyManager,
    rate_limit_calculator: UnifiedRateLimitCalculator,
    database: std::sync::Arc<Database>,
    jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
}

impl McpAuthMiddleware {
    /// Create new `MCP` auth middleware
    #[must_use]
    pub fn new(
        auth_manager: AuthManager,
        database: std::sync::Arc<Database>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
        rate_limit_config: crate::config::environment::RateLimitConfig,
    ) -> Self {
        Self {
            auth_manager,
            api_key_manager: ApiKeyManager::new(),
            rate_limit_calculator: UnifiedRateLimitCalculator::new_with_config(rate_limit_config),
            database,
            jwks_manager,
        }
    }

    /// Authenticate request using headers (supports cookies and Authorization header)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Authentication credentials missing (no cookie or header)
    /// - JWT token validation fails
    /// - API key validation fails
    /// - Database queries fail
    /// - Rate limit calculations fail
    /// - User lookup fails
    #[tracing::instrument(
        skip(self, headers),
        fields(
            auth_method = tracing::field::Empty,
            user_id = tracing::field::Empty,
            tenant_id = tracing::field::Empty,
            success = tracing::field::Empty,
        )
    )]
    pub async fn authenticate_request_with_headers(
        &self,
        headers: &axum::http::HeaderMap,
    ) -> AppResult<AuthResult> {
        tracing::debug!("=== AUTH MIDDLEWARE AUTHENTICATE_REQUEST_WITH_HEADERS START ===");

        // Try cookie authentication first (preferred for web clients)
        if let Some(jwt_token) = crate::security::cookies::get_cookie_value(headers, "auth_token") {
            tracing::debug!("Found JWT in httpOnly cookie, attempting authentication");
            tracing::Span::current().record("auth_method", "JWT_COOKIE");
            match self.authenticate_jwt_token(&jwt_token).await {
                Ok(result) => {
                    tracing::Span::current()
                        .record("user_id", result.user_id.to_string())
                        .record("tenant_id", result.user_id.to_string())
                        .record("success", true);
                    tracing::info!(
                        "JWT cookie authentication successful for user: {}",
                        result.user_id
                    );
                    return Ok(result);
                }
                Err(e) => {
                    tracing::Span::current().record("success", false);
                    tracing::warn!("JWT cookie authentication failed: {}", e);
                    return Err(e);
                }
            }
        }

        // Fall back to Authorization header for API clients
        let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());

        self.authenticate_request(auth_header).await
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
    #[tracing::instrument(
        skip(self, auth_header),
        fields(
            auth_method = tracing::field::Empty,
            user_id = tracing::field::Empty,
            tenant_id = tracing::field::Empty,
            success = tracing::field::Empty,
        )
    )]
    pub async fn authenticate_request(&self, auth_header: Option<&str>) -> AppResult<AuthResult> {
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
            return Err(auth_error("Missing authorization header - Request authentication requires Authorization header with Bearer token or API key"));
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
            Err(AppError::auth_invalid("Invalid authorization header format - must be 'Bearer <token>' or 'pk_live_<api_key>'"))
        }
    }

    /// Authenticate using `API` key
    async fn authenticate_api_key(&self, api_key: &str) -> AppResult<AuthResult> {
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
            .ok_or_else(|| {
                AppError::auth_invalid(format!("API key not found or invalid: {key_prefix}"))
            })?;

        // Validate key status
        self.api_key_manager.is_key_valid(&db_key)?;

        // Get current usage for rate limiting
        let current_usage = self.database.get_api_key_current_usage(&db_key.id).await?;
        let rate_limit = self
            .rate_limit_calculator
            .calculate_api_key_rate_limit(&db_key, current_usage);

        // Check rate limit
        if rate_limit.is_rate_limited {
            let err = ProviderError::RateLimitExceeded {
                provider: "API Key Authentication".to_owned(),
                retry_after_secs: rate_limit.reset_at.map_or(3600, |dt| {
                    let now = chrono::Utc::now().timestamp();
                    let reset = dt.timestamp();
                    u64::try_from((reset - now).max(0)).unwrap_or(3600)
                }),
                limit_type: format!(
                    "Rate limit reached for API key: {}/{} requests",
                    current_usage,
                    rate_limit.limit.unwrap_or(0)
                ),
            };
            return Err(AppError::external_service(
                "API Key Authentication",
                err.to_string(),
            ));
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

    /// Authenticate using RS256 JWT token
    async fn authenticate_jwt_token(&self, token: &str) -> AppResult<AuthResult> {
        let claims = self
            .auth_manager
            .validate_token_detailed(token, &self.jwks_manager)
            .map_err(|e| AppError::auth_invalid(format!("JWT validation failed: {e}")))?;

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
            return Err(auth_error("JWT token rate limit exceeded"));
        }

        Ok(AuthResult {
            user_id,
            auth_method: AuthMethod::JwtToken {
                tier: format!("{:?}", user.tier).to_lowercase(),
            },
            rate_limit,
        })
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
    pub fn check_provider_access(&self, token: &str, provider: &str) -> AppResult<bool> {
        let claims = self
            .auth_manager
            .validate_token(token, &self.jwks_manager)?;
        Ok(claims.providers.contains(&provider.to_owned()))
    }

    /// Get reference to the auth manager for testing purposes
    #[must_use]
    pub const fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }
}
