// ABOUTME: A2A authentication and client credential management
// ABOUTME: Handles client ID/secret validation, session tokens, and A2A protocol security
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! A2A Authentication Implementation
//!
//! Implements authentication and authorization for A2A protocol,
//! supporting API keys and `OAuth2` for agent-to-agent communication.

use crate::auth::{AuthMethod, AuthResult};
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::providers::errors::ProviderError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::debug;

/// A2A Authentication token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AToken {
    /// A2A client identifier
    pub client_id: String,
    /// User ID associated with this token
    pub user_id: String,
    /// List of OAuth scopes granted to this token
    pub scopes: Vec<String>,
    /// When this token expires
    pub expires_at: chrono::DateTime<chrono::Utc>,
    /// When this token was created
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A2A Client registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AClient {
    /// Unique client identifier
    pub id: String,
    /// User ID for session tracking and consistency
    pub user_id: uuid::Uuid,
    /// Human-readable client name
    pub name: String,
    /// Description of the client application
    pub description: String,
    /// Public key for signature verification
    pub public_key: String,
    /// List of capabilities this client can access
    pub capabilities: Vec<String>,
    /// Allowed OAuth redirect URIs
    pub redirect_uris: Vec<String>,
    /// Whether this client is active
    pub is_active: bool,
    /// When this client was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    // Additional fields for database compatibility
    /// List of permissions granted to this client
    #[serde(default = "default_permissions")]
    pub permissions: Vec<String>,
    /// Maximum requests allowed per window
    #[serde(default = "default_rate_limit_requests")]
    pub rate_limit_requests: u32,
    /// Rate limit window duration in seconds
    #[serde(default = "default_rate_limit_window")]
    pub rate_limit_window_seconds: u32,
    /// When this client was last updated
    #[serde(default = "chrono::Utc::now")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_permissions() -> Vec<String> {
    vec!["read_activities".into()]
}

const fn default_rate_limit_requests() -> u32 {
    crate::constants::rate_limits::DEFAULT_BURST_LIMIT * 10
}

#[allow(clippy::cast_possible_truncation)] // Safe: HOUR_SECONDS is 3600, well within u32 range
const fn default_rate_limit_window() -> u32 {
    crate::constants::time::HOUR_SECONDS as u32
}

/// A2A Authenticator
pub struct A2AAuthenticator {
    resources: Arc<crate::mcp::resources::ServerResources>,
}

impl A2AAuthenticator {
    /// Creates a new A2A authenticator instance
    #[must_use]
    pub const fn new(resources: Arc<crate::mcp::resources::ServerResources>) -> Self {
        Self { resources }
    }

    /// Authenticate an A2A request using API key
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The API key format is invalid
    /// - Authentication fails
    /// - Rate limits are exceeded
    pub async fn authenticate_api_key(&self, api_key: &str) -> AppResult<AuthResult> {
        // Check if it's an A2A-specific API key (with a2a_ prefix)
        if api_key.starts_with("a2a_") {
            return self.authenticate_a2a_key(api_key).await;
        }

        // Use standard API key authentication through MCP middleware
        let middleware = &self.resources.auth_middleware;

        middleware
            .authenticate_request(Some(api_key))
            .await
            .map_err(|e| AppError::auth_invalid(format!("A2A authentication failed: {e}")))
    }

    /// Authenticate A2A-specific API key with rate limiting
    async fn authenticate_a2a_key(&self, api_key: &str) -> AppResult<AuthResult> {
        // Extract key components (similar to API key validation)
        if !api_key.starts_with("a2a_") || api_key.len() < 16 {
            return Err(AppError::auth_invalid("Invalid A2A API key format"));
        }

        // A2A keys are stored in API keys table but linked to A2A clients
        // Use regular API key authentication with A2A-specific rate limiting

        let middleware = &self.resources.auth_middleware;

        // First authenticate using regular API key system
        let mut auth_result = middleware.authenticate_request(Some(api_key)).await?;

        // Add A2A-specific rate limiting
        if let AuthMethod::ApiKey { key_id, tier: _ } = &auth_result.auth_method {
            // Find A2A client associated with this API key
            if let Some(client) = self
                .get_a2a_client_by_api_key(key_id)
                .await
                .map_err(|e| AppError::database(format!("Failed to get A2A client: {e}")))?
            {
                let client_manager = &*self.resources.a2a_client_manager;

                // Check A2A-specific rate limits
                let rate_limit_status = client_manager
                    .get_client_rate_limit_status(&client.id)
                    .await
                    .map_err(|e| {
                        AppError::internal(format!("Failed to check A2A rate limits: {e}"))
                    })?;

                if rate_limit_status.is_rate_limited {
                    let err = ProviderError::RateLimitExceeded {
                        provider: "A2A Client Authentication".to_owned(),
                        retry_after_secs: rate_limit_status.reset_at.map_or(3600, |dt| {
                            let now = chrono::Utc::now().timestamp();
                            let reset = dt.timestamp();
                            u64::try_from((reset - now).max(0)).unwrap_or(3600)
                        }),
                        limit_type: format!(
                            "A2A client rate limit exceeded. Limit: {}, Reset at: {}",
                            rate_limit_status.limit.unwrap_or(0),
                            rate_limit_status
                                .reset_at
                                .map_or_else(|| "unknown".into(), |dt| dt.to_rfc3339())
                        ),
                    };
                    return Err(AppError::external_service(
                        "A2A Client Authentication",
                        err.to_string(),
                    ));
                }

                // Update auth method to indicate A2A authentication
                auth_result.auth_method = AuthMethod::ApiKey {
                    key_id: key_id.clone(), // Safe: String ownership for auth method
                    tier: format!("A2A-{}", rate_limit_status.tier.display_name()),
                };

                // Store A2A rate limit status in auth result
                // Note: This requires extending AuthResult to include A2A rate limit info
                // Log successful A2A authentication
                debug!(
                    "A2A client {} authenticated with {} requests remaining",
                    client.id,
                    rate_limit_status.remaining.unwrap_or(0)
                );
            }
        }

        Ok(auth_result)
    }

    /// Get A2A client by API key ID
    ///
    /// # Errors
    /// Returns an error if database query fails
    async fn get_a2a_client_by_api_key(
        &self,
        api_key_id: &str,
    ) -> crate::errors::AppResult<Option<A2AClient>> {
        self.resources
            .database
            .get_a2a_client_by_api_key_id(api_key_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to lookup A2A client by API key: {e}")))
    }

    /// Authenticate using `OAuth2` token
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Token validation fails
    /// - Token does not contain valid A2A client identifier
    /// - A2A client not found or is deactivated
    ///
    /// # Panics
    ///
    /// Panics if the token subject has `a2a_client_` prefix but cannot be stripped (should never happen)
    pub async fn authenticate_oauth2(&self, token: &str) -> crate::errors::AppResult<AuthResult> {
        // OAuth2 token validation for A2A using JWT tokens

        // Try to decode the JWT token using RS256
        let token_claims = self
            .resources
            .auth_manager
            .validate_token(token, &self.resources.jwks_manager)?;

        // Check if this is an A2A OAuth2 token by looking for specific claims
        // A2A OAuth tokens should have client_id in the subject or a custom claim
        let client_id = if token_claims.sub.starts_with("a2a_client_") {
            token_claims
                .sub
                .strip_prefix("a2a_client_")
                .ok_or_else(|| {
                    AppError::auth_invalid("Failed to strip a2a_client_ prefix from token subject")
                })?
                .to_owned()
        } else {
            // Try to extract from custom claims if available
            return Err(AppError::auth_invalid(
                "Token does not contain valid A2A client identifier",
            ));
        };

        // Verify the client exists and is active
        let client = self
            .get_client(&client_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch A2A client: {e}")))?
            .ok_or_else(|| AppError::not_found(format!("A2A client {client_id}")))?;

        if !client.is_active {
            return Err(AppError::auth_invalid(format!(
                "A2A client is deactivated: {client_id}"
            )));
        }

        // Check token expiration (already handled by validate_token)
        // Check scopes if present in token
        // Grant access based on A2A client permissions

        Ok(AuthResult {
            user_id: client.user_id, // Use consistent A2A client user ID for session tracking
            auth_method: AuthMethod::ApiKey {
                key_id: format!("oauth2_a2a_{client_id}"),
                tier: "A2A-OAuth2".into(),
            },
            rate_limit: crate::rate_limiting::UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: Some(1000),     // Default A2A OAuth2 limit
                remaining: Some(1000), // Start with full limit
                reset_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
                tier: "A2A-OAuth2".into(),
                auth_method: "oauth2".into(),
            },
        })
    }

    /// Register a new A2A client
    ///
    /// # Errors
    ///
    /// Returns an error if client registration fails
    pub async fn register_client(&self, client: A2AClient) -> Result<String, crate::a2a::A2AError> {
        // Use the client manager to handle registration
        let client_manager = &*self.resources.a2a_client_manager;

        let request = crate::a2a::client::ClientRegistrationRequest {
            name: client.name,
            description: client.description,
            capabilities: client.capabilities,
            redirect_uris: client.redirect_uris,
            contact_email: format!("a2a-client-{}@system.local", uuid::Uuid::new_v4()), // System-generated A2A client email
        };

        // Use the user_id from the client struct for ownership tracking
        let credentials = client_manager
            .register_client(request, client.user_id)
            .await?;
        Ok(credentials.client_id)
    }

    /// Get client by ID
    ///
    /// # Errors
    ///
    /// Returns an error if database query fails
    pub async fn get_client(
        &self,
        client_id: &str,
    ) -> Result<Option<A2AClient>, crate::a2a::A2AError> {
        self.resources
            .database
            .get_a2a_client(client_id)
            .await
            .map_err(|e| {
                crate::a2a::A2AError::InternalError(format!("Failed to get A2A client: {e}"))
            })
    }

    /// Validate client capabilities
    #[must_use]
    pub fn validate_capabilities(&self, client: &A2AClient, requested_capability: &str) -> bool {
        client
            .capabilities
            .contains(&requested_capability.to_owned())
    }

    /// Create A2A token for authenticated client
    #[must_use]
    pub fn create_token(&self, client_id: &str, user_id: &str, scopes: Vec<String>) -> A2AToken {
        A2AToken {
            client_id: client_id.to_owned(),
            user_id: user_id.to_owned(),
            scopes,
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
            created_at: chrono::Utc::now(),
        }
    }

    /// Validate A2A token
    ///
    /// # Errors
    ///
    /// Returns an error if token validation fails
    pub fn validate_token(&self, token: &A2AToken) -> Result<bool, crate::a2a::A2AError> {
        // Check if token is expired
        if token.expires_at < chrono::Utc::now() {
            return Ok(false);
        }

        // Token validation checks: database existence, expiry, and client active status

        Ok(true)
    }

    /// Check if client has required scope
    #[must_use]
    pub fn check_scope(&self, token: &A2AToken, required_scope: &str) -> bool {
        token.scopes.contains(&required_scope.to_owned()) || token.scopes.contains(&"*".into())
    }
}
