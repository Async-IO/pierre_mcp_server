// ABOUTME: Authentication service for universal protocol handlers
// ABOUTME: Handles OAuth token management and provider creation with tenant support
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::environment::get_oauth_config;
use crate::constants::oauth_providers;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::mcp::resources::ServerResources;
use crate::models::{TenantId, UserOAuthToken};
use crate::oauth2_client::client::fitbit::refresh_fitbit_token;
use crate::oauth2_client::client::strava::refresh_strava_token;
use crate::protocols::universal::UniversalResponse;
use crate::providers::synthetic_provider::SyntheticProvider;
use crate::providers::{CoreFitnessProvider, OAuth2Credentials};
use crate::tenant::{TenantContext, TenantRole};
use crate::utils::http_client::api_client;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// OAuth token data structure
#[derive(Debug, Clone)]
pub struct TokenData {
    /// OAuth access token
    pub access_token: String,
    /// OAuth refresh token
    pub refresh_token: String,
    /// When the access token expires
    pub expires_at: DateTime<Utc>,
    /// OAuth scopes as comma-separated string
    pub scopes: String,
    /// Provider name (e.g., "strava", "fitbit")
    pub provider: String,
}

/// OAuth error types
#[derive(Debug, thiserror::Error)]
pub enum OAuthError {
    /// Failed to exchange authorization code for tokens
    #[error("Token exchange failed: {0}")]
    TokenExchangeFailed(String),

    /// Failed to refresh expired access token
    #[error("Token refresh failed: {0}")]
    TokenRefreshFailed(String),

    /// Database operation failed
    #[error("Database error: {0}")]
    DatabaseError(String),
}

/// Service responsible for authentication and provider creation
/// Centralizes OAuth token management and reduces duplication across handlers
pub struct AuthService {
    resources: Arc<ServerResources>,
}

impl AuthService {
    /// Create new authentication service
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Get valid token for a provider, automatically refreshing if needed
    /// Returns None if no token exists or is expired, Error if token operations fail
    ///
    /// # Errors
    /// Returns `OAuthError` if token refresh fails or database operations fail
    pub async fn get_valid_token(
        &self,
        user_id: Uuid,
        provider: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<TokenData>, OAuthError> {
        debug!(
            "get_valid_token called for user={}, provider={}, tenant={:?}",
            user_id, provider, tenant_id
        );

        // If we have tenant context, initialize tenant-specific OAuth credentials
        if let Some(tenant_id_str) = tenant_id {
            self.initialize_tenant_oauth_context(user_id, tenant_id_str, provider)
                .await;
        }

        // Look up token from database with tenant context
        let Some(tenant_id_str) = tenant_id else {
            debug!("No tenant_id provided, returning Ok(None)");
            return Ok(None);
        };

        // Direct database lookup with tenant_id
        let tenant_id_parsed: TenantId = tenant_id_str.parse().map_err(|_| {
            OAuthError::DatabaseError(format!("Invalid tenant_id format: {tenant_id_str}"))
        })?;
        let token_result = (*self.resources.database)
            .get_user_oauth_token(user_id, tenant_id_parsed, provider)
            .await;

        Self::log_token_lookup_result(&token_result, user_id, tenant_id_str, provider);

        let Ok(Some(oauth_token)) = token_result else {
            return Ok(None);
        };

        // Process the token - validate expiration and refresh if needed
        self.process_oauth_token(user_id, tenant_id_str, provider, oauth_token)
            .await
    }

    /// Initialize tenant-specific OAuth context if available
    async fn initialize_tenant_oauth_context(
        &self,
        user_id: Uuid,
        tenant_id_str: &str,
        provider: &str,
    ) {
        let Ok(tenant_uuid) = tenant_id_str.parse::<TenantId>() else {
            return;
        };

        let Ok(tenant) = (*self.resources.database)
            .get_tenant_by_id(tenant_uuid)
            .await
        else {
            return;
        };

        let tenant_context = TenantContext {
            tenant_id: tenant_uuid,
            tenant_name: tenant.name.clone(), // Safe: String ownership needed for tenant context
            user_id,
            user_role: TenantRole::Member,
        };

        // Get tenant-specific OAuth credentials - result is unused but initializes context
        let _ = self
            .resources
            .tenant_oauth_client
            .get_oauth_client(&tenant_context, provider, &self.resources.database)
            .await;
    }

    /// Process OAuth token - validate expiration and refresh if needed
    async fn process_oauth_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        oauth_token: UserOAuthToken,
    ) -> Result<Option<TokenData>, OAuthError> {
        // Check if token is expired (with 5-minute buffer)
        if let Some(expires_at) = oauth_token.expires_at {
            if Self::is_token_expired(expires_at) {
                return self
                    .handle_expired_token(user_id, tenant_id, provider, &oauth_token)
                    .await;
            }
        }

        // Token is valid, return it
        Ok(Some(TokenData {
            provider: provider.to_owned(),
            access_token: oauth_token.access_token,
            refresh_token: oauth_token.refresh_token.unwrap_or_default(),
            expires_at: oauth_token.expires_at.unwrap_or_else(chrono::Utc::now),
            scopes: oauth_token.scope.unwrap_or_default(),
        }))
    }

    /// Check if token is expired or expiring within 5 minutes
    fn is_token_expired(expires_at: DateTime<Utc>) -> bool {
        let now = chrono::Utc::now();
        expires_at <= now + chrono::Duration::minutes(5)
    }

    /// Log the result of a token lookup operation
    fn log_token_lookup_result(
        token_result: &Result<Option<UserOAuthToken>, AppError>,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
    ) {
        match token_result {
            Ok(Some(token)) => debug!(
                "Found OAuth token for user={}, provider={}, expires_at={:?}",
                user_id, provider, token.expires_at
            ),
            Ok(None) => debug!(
                "No OAuth token found for user={}, tenant={}, provider={}",
                user_id, tenant_id, provider
            ),
            Err(e) => warn!(
                "Error retrieving OAuth token for user={}, tenant={}, provider={}: {}",
                user_id, tenant_id, provider, e
            ),
        }
    }

    /// Handle expired token by attempting refresh
    async fn handle_expired_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        oauth_token: &UserOAuthToken,
    ) -> Result<Option<TokenData>, OAuthError> {
        // Check if we have a valid refresh token
        let Some(ref refresh_token) = oauth_token.refresh_token else {
            return Ok(None);
        };

        if refresh_token.is_empty() {
            return Ok(None);
        }

        info!(
            "Token expired for user {} provider {}, attempting refresh",
            user_id, provider
        );

        // Attempt to refresh the token
        match self
            .refresh_provider_token(user_id, tenant_id, provider, refresh_token)
            .await
        {
            Ok(refreshed_token) => {
                info!(
                    "Token refreshed successfully for user {} provider {}",
                    user_id, provider
                );
                Ok(Some(refreshed_token))
            }
            Err(e) => {
                warn!(
                    "Token refresh failed for user {} provider {}: {}",
                    user_id, provider, e
                );
                Ok(None)
            }
        }
    }

    /// Create authenticated provider with proper tenant-aware credentials
    /// Returns configured provider ready for API calls
    ///
    /// # Errors
    /// Returns `UniversalResponse` error if provider is unsupported or authentication fails
    pub async fn create_authenticated_provider(
        &self,
        provider_name: &str,
        user_id: Uuid,
        tenant_id: Option<&str>,
    ) -> Result<Box<dyn CoreFitnessProvider>, UniversalResponse> {
        // Check if provider is supported by the registry
        if !self.resources.provider_registry.is_supported(provider_name) {
            return Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Unsupported provider: {provider_name}")),
                metadata: None,
            });
        }

        // Synthetic provider doesn't use OAuth - create directly with user context
        if provider_name == oauth_providers::SYNTHETIC {
            debug!(
                user_id = %user_id,
                provider = provider_name,
                "Creating synthetic provider with user context (no OAuth required)"
            );
            let mut provider = SyntheticProvider::new();
            provider.set_user_id(user_id);
            return Ok(Box::new(provider));
        }

        // Get valid token for the provider (with automatic refresh if needed)
        match self
            .get_valid_token(user_id, provider_name, tenant_id)
            .await
        {
            Ok(Some(token_data)) => {
                self.create_provider_with_token(provider_name, token_data, tenant_id)
                    .await
            }
            Ok(None) => Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    format!("No valid {provider_name} token found. Please connect your {provider_name} account."),
                ),
                metadata: None,
            }),
            Err(e) => Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Authentication error: {e}")),
                metadata: None,
            }),
        }
    }

    /// Create provider with token and tenant-aware credentials
    async fn create_provider_with_token(
        &self,
        provider_name: &str,
        token_data: TokenData,
        tenant_id: Option<&str>,
    ) -> Result<Box<dyn CoreFitnessProvider>, UniversalResponse> {
        // Get tenant-aware OAuth credentials or fall back to environment
        let (client_id, client_secret) = if let Some(tenant_id_str) = tenant_id {
            self.get_tenant_oauth_credentials(tenant_id_str, provider_name)
                .await
                .map_err(|e| *e)?
        } else {
            Self::get_default_oauth_credentials(provider_name).map_err(|e| *e)?
        };

        // Get provider-specific scopes
        let scopes = token_data
            .scopes
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>();

        // Create provider using the factory function
        match self
            .resources
            .provider_registry
            .create_provider(provider_name)
        {
            Ok(provider) => {
                // Prepare credentials in the correct format
                let credentials = OAuth2Credentials {
                    client_id,
                    client_secret,
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes,
                };

                // Set credentials asynchronously
                match provider.set_credentials(credentials).await {
                    Ok(()) => Ok(provider),
                    Err(e) => Err(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to set provider credentials: {e}")),
                        metadata: None,
                    }),
                }
            }
            Err(e) => Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to create provider: {e}")),
                metadata: None,
            }),
        }
    }

    /// Get OAuth credentials for a specific tenant and provider
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id_str: &str,
        provider_name: &str,
    ) -> Result<(String, String), Box<UniversalResponse>> {
        let tenant_uuid: TenantId = tenant_id_str.parse().map_err(|e| {
            warn!(tenant_id = %tenant_id_str, error = %e, "Invalid tenant ID format in OAuth credentials request");
            Box::new(UniversalResponse {
                success: false,
                result: None,
                error: Some("Invalid tenant ID format".to_owned()),
                metadata: None,
            })
        })?;

        // Get tenant OAuth credentials from database for the specific provider
        match (*self.resources.database)
            .get_tenant_oauth_credentials(tenant_uuid, provider_name)
            .await
        {
            Ok(Some(creds)) => Ok((creds.client_id, creds.client_secret)),
            Ok(None) => {
                // Fall back to default credentials if tenant doesn't have custom ones
                Self::get_default_oauth_credentials(provider_name)
            }
            Err(e) => Err(Box::new(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to get tenant OAuth credentials: {e}")),
                metadata: None,
            })),
        }
    }

    /// Get default OAuth credentials from `ServerConfig` or environment for a provider
    ///
    /// # Errors
    /// Returns boxed `UniversalResponse` error if credentials are not configured
    fn get_default_oauth_credentials(
        provider_name: &str,
    ) -> Result<(String, String), Box<UniversalResponse>> {
        // Get OAuth config from environment (PIERRE_<PROVIDER>_* env vars)
        let oauth_config = get_oauth_config(provider_name);

        let client_id = oauth_config.client_id.as_ref().ok_or_else(|| {
            Box::new(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "{}_CLIENT_ID not configured for provider {}",
                    provider_name.to_uppercase(),
                    provider_name
                )),
                metadata: None,
            })
        })?;

        let client_secret = oauth_config.client_secret.as_ref().ok_or_else(|| {
            Box::new(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "{}_CLIENT_SECRET not configured for provider {}",
                    provider_name.to_uppercase(),
                    provider_name
                )),
                metadata: None,
            })
        })?;

        Ok((client_id.clone(), client_secret.clone()))
    }

    /// Refresh an expired OAuth token for a provider
    ///
    /// Calls the provider's token refresh endpoint and stores the new token in the database.
    ///
    /// # Errors
    /// Returns `OAuthError` if token refresh or database operations fail
    async fn refresh_provider_token(
        &self,
        user_id: Uuid,
        tenant_id: &str,
        provider: &str,
        refresh_token: &str,
    ) -> Result<TokenData, OAuthError> {
        // Get tenant-specific OAuth credentials, falling back to defaults
        let (client_id, client_secret) = if tenant_id.is_empty() {
            Self::get_default_oauth_credentials(provider)
        } else {
            self.get_tenant_oauth_credentials(tenant_id, provider).await
        }
        .map_err(|e| OAuthError::TokenRefreshFailed(e.error.unwrap_or_default()))?;

        // Call provider-specific token refresh
        let new_token = match provider.to_lowercase().as_str() {
            "strava" => {
                let http_client = api_client();
                refresh_strava_token(&http_client, &client_id, &client_secret, refresh_token)
                    .await
                    .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?
            }
            "fitbit" => {
                let http_client = api_client();
                refresh_fitbit_token(&http_client, &client_id, &client_secret, refresh_token)
                    .await
                    .map_err(|e| OAuthError::TokenRefreshFailed(e.to_string()))?
            }
            other => {
                return Err(OAuthError::TokenRefreshFailed(format!(
                    "Token refresh not supported for provider: {other}"
                )));
            }
        };

        // Prepare token data for database update
        let new_access_token = new_token.access_token.clone();
        let new_refresh_token = new_token.refresh_token.clone();
        let new_expires_at = new_token.expires_at;

        // Update the token in the database
        let tenant_id_parsed: TenantId = tenant_id.parse().map_err(|_| {
            OAuthError::DatabaseError(format!("Invalid tenant_id format: {tenant_id}"))
        })?;
        (*self.resources.database)
            .refresh_user_oauth_token(
                user_id,
                tenant_id_parsed,
                provider,
                &new_access_token,
                new_refresh_token.as_deref(),
                new_expires_at,
            )
            .await
            .map_err(|e| OAuthError::DatabaseError(e.to_string()))?;

        // Return the refreshed token data
        Ok(TokenData {
            provider: provider.to_owned(),
            access_token: new_access_token,
            refresh_token: new_refresh_token.unwrap_or_default(),
            expires_at: new_expires_at.unwrap_or_else(chrono::Utc::now),
            scopes: new_token.scope.unwrap_or_default(),
        })
    }

    /// Check if user has valid authentication for a provider
    pub async fn has_valid_auth(
        &self,
        user_id: Uuid,
        provider: &str,
        tenant_id: Option<&str>,
    ) -> bool {
        matches!(
            self.get_valid_token(user_id, provider, tenant_id).await,
            Ok(Some(_))
        )
    }

    /// Disconnect user from a provider
    ///
    /// # Errors
    /// Returns `OAuthError` if database operations fail
    pub async fn disconnect_provider(
        &self,
        user_id: Uuid,
        provider: &str,
        tenant_id: Option<&str>,
    ) -> Result<(), OAuthError> {
        // Use database to delete tokens directly (like original implementation)
        let tenant_id_str = tenant_id.ok_or_else(|| {
            OAuthError::DatabaseError("tenant_id is required to disconnect a provider".to_owned())
        })?;
        let tenant_id_parsed: TenantId = tenant_id_str.parse().map_err(|_| {
            OAuthError::DatabaseError(format!("Invalid tenant_id format: {tenant_id_str}"))
        })?;
        (*self.resources.database)
            .delete_user_oauth_token(user_id, tenant_id_parsed, provider)
            .await
            .map_err(|e| OAuthError::DatabaseError(format!("Failed to delete token: {e}")))
    }
}
