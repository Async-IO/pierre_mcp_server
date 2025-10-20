// ABOUTME: MCP authentication middleware for request authentication and authorization
// ABOUTME: Handles JWT tokens and API keys with rate limiting and user context extraction
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::api_keys::ApiKeyManager;
use crate::auth::{AuthManager, AuthMethod, AuthResult};
use crate::constants::key_prefixes;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::rate_limiting::UnifiedRateLimitCalculator;
use crate::utils::errors::auth_error;
use anyhow::{Context, Result};

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
    pub fn new(
        auth_manager: AuthManager,
        database: std::sync::Arc<Database>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> Self {
        Self {
            auth_manager,
            api_key_manager: ApiKeyManager::new(),
            rate_limit_calculator: UnifiedRateLimitCalculator::new(),
            database,
            jwks_manager,
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
            tracing::debug!(
                "Auth header content (first 100 chars): {}",
                &header[..std::cmp::min(100, header.len())]
            );
            tracing::debug!("Auth header length: {} characters", header.len());

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
            return Err(auth_error("Missing authorization header").context(
                "Request authentication requires Authorization header with Bearer token or API key",
            ));
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

    /// Authenticate using RS256 JWT token
    async fn authenticate_jwt_token(&self, token: &str) -> Result<AuthResult> {
        let claims = self
            .auth_manager
            .validate_token_detailed(token, &self.jwks_manager)?;

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
    pub fn check_provider_access(&self, token: &str, provider: &str) -> Result<bool> {
        let claims = self
            .auth_manager
            .validate_token(token, &self.jwks_manager)?;
        Ok(claims.providers.contains(&provider.to_string()))
    }

    /// Get reference to the auth manager for testing purposes
    pub const fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }
}
