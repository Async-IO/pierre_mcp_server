// ABOUTME: Authentication service for universal protocol handlers
// ABOUTME: Handles OAuth token management and provider creation with tenant support
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::constants::oauth_providers;
use crate::database_plugins::DatabaseProvider;
use crate::mcp::resources::ServerResources;
use crate::oauth::{OAuthError, TokenData};
use crate::protocols::universal::UniversalResponse;
use crate::providers::{CoreFitnessProvider, OAuth2Credentials};
use std::sync::Arc;
use uuid::Uuid;

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
    /// Returns None if no token exists, Error if token operations fail
    ///
    /// # Errors
    /// Returns `OAuthError` if token refresh fails or database operations fail
    pub async fn get_valid_token(
        &self,
        user_id: Uuid,
        provider: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<TokenData>, OAuthError> {
        // If we have tenant context, use tenant-specific OAuth credentials
        if let Some(tenant_id_str) = tenant_id {
            // Convert string tenant ID to UUID and look up full tenant context
            if let Ok(tenant_uuid) = Uuid::parse_str(tenant_id_str) {
                // Look up tenant information from database to create proper TenantContext
                if let Ok(tenant) = (*self.resources.database)
                    .get_tenant_by_id(tenant_uuid)
                    .await
                {
                    let tenant_context = crate::tenant::TenantContext {
                        tenant_id: tenant_uuid,
                        tenant_name: tenant.name.clone(), // Safe: String ownership needed for tenant context
                        user_id,
                        user_role: crate::tenant::TenantRole::Member,
                    };

                    // Get tenant-specific OAuth credentials using proper TenantContext
                    match self
                        .resources
                        .tenant_oauth_client
                        .get_oauth_client(&tenant_context, provider, &self.resources.database)
                        .await
                    {
                        Ok(_oauth_client) => {
                            // Tenant-specific OAuth client found
                            // Continue to database lookup below which will find the token
                        }
                        Err(_e) => {
                            // No tenant-specific client, will use global config
                        }
                    }
                }
            }
        }

        // Use pre-registered global config
        // If tenant_id was provided, look up token directly from database with tenant context
        if let Some(tenant_id_str) = tenant_id {
            // Direct database lookup with tenant_id
            match (*self.resources.database)
                .get_user_oauth_token(user_id, tenant_id_str, provider)
                .await
            {
                Ok(Some(oauth_token)) => {
                    let token_data = TokenData {
                        provider: provider.to_string(),
                        access_token: oauth_token.access_token,
                        refresh_token: oauth_token.refresh_token.unwrap_or_default(),
                        expires_at: oauth_token.expires_at.unwrap_or_else(chrono::Utc::now),
                        scopes: oauth_token.scope.unwrap_or_default(),
                    };
                    return Ok(Some(token_data));
                }
                Ok(None) => return Ok(None),
                Err(_) => {
                    // Continue to global manager as fallback
                }
            }
        }

        // If no token found in database, return None
        Ok(None)
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
        // Only support Strava for now
        if provider_name != oauth_providers::STRAVA {
            return Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Unsupported provider: {provider_name}")),
                metadata: None,
            });
        }

        // Get valid Strava token (with automatic refresh if needed)
        match self
            .get_valid_token(user_id, oauth_providers::STRAVA, tenant_id)
            .await
        {
            Ok(Some(token_data)) => {
                self.create_strava_provider_with_token(token_data, tenant_id)
                    .await
            }
            Ok(None) => Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(
                    "No valid Strava token found. Please connect your Strava account.".to_string(),
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

    /// Create Strava provider with token and tenant-aware credentials
    async fn create_strava_provider_with_token(
        &self,
        token_data: TokenData,
        tenant_id: Option<&str>,
    ) -> Result<Box<dyn CoreFitnessProvider>, UniversalResponse> {
        // Get tenant-aware OAuth credentials or fall back to environment
        let (client_id, client_secret) = if let Some(tenant_id_str) = tenant_id {
            self.get_tenant_oauth_credentials(tenant_id_str).await?
        } else {
            Self::get_default_oauth_credentials()?
        };

        // Create Strava provider using the factory function
        match crate::providers::create_provider(oauth_providers::STRAVA) {
            Ok(mut provider) => {
                // Prepare credentials in the correct format
                let credentials = OAuth2Credentials {
                    client_id,
                    client_secret,
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_string)
                        .collect(),
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

    /// Get OAuth credentials for a specific tenant
    async fn get_tenant_oauth_credentials(
        &self,
        tenant_id_str: &str,
    ) -> Result<(String, String), UniversalResponse> {
        let tenant_uuid = Uuid::parse_str(tenant_id_str).map_err(|_| UniversalResponse {
            success: false,
            result: None,
            error: Some("Invalid tenant ID format".to_string()),
            metadata: None,
        })?;

        // Get tenant OAuth credentials from database
        match (*self.resources.database)
            .get_tenant_oauth_credentials(tenant_uuid, oauth_providers::STRAVA)
            .await
        {
            Ok(Some(creds)) => Ok((creds.client_id, creds.client_secret)),
            Ok(None) => {
                // Fall back to default credentials if tenant doesn't have custom ones
                Self::get_default_oauth_credentials()
            }
            Err(e) => Err(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to get tenant OAuth credentials: {e}")),
                metadata: None,
            }),
        }
    }

    /// Get default OAuth credentials from environment variables
    ///
    /// # Errors
    /// Returns `UniversalResponse` error if environment variables are missing
    fn get_default_oauth_credentials() -> Result<(String, String), UniversalResponse> {
        let client_id = std::env::var("STRAVA_CLIENT_ID").map_err(|_| UniversalResponse {
            success: false,
            result: None,
            error: Some("STRAVA_CLIENT_ID environment variable required".to_string()),
            metadata: None,
        })?;

        let client_secret =
            std::env::var("STRAVA_CLIENT_SECRET").map_err(|_| UniversalResponse {
                success: false,
                result: None,
                error: Some("STRAVA_CLIENT_SECRET environment variable required".to_string()),
                metadata: None,
            })?;

        Ok((client_id, client_secret))
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
        let tenant_id_str = tenant_id.unwrap_or("default");
        (*self.resources.database)
            .delete_user_oauth_token(user_id, tenant_id_str, provider)
            .await
            .map_err(|e| OAuthError::DatabaseError(format!("Failed to delete token: {e}")))
    }
}
