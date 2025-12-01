// ABOUTME: Tenant-aware OAuth client for multi-tenant fitness platform authentication
// ABOUTME: Provides OAuth flow integration with tenant-specific credentials and rate limiting
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - OAuth credential string ownership transfers (client_id, client_secret, redirect_uri)
// - Tenant context ownership for multi-tenant OAuth flows

use crate::database_plugins::factory::Database;
use crate::errors::{AppError, AppResult};
use crate::oauth2_client::client::{OAuth2Client, OAuth2Config, OAuth2Token, PkceParams};
use crate::tenant::oauth_manager::{CredentialConfig, TenantOAuthCredentials, TenantOAuthManager};
use crate::tenant::TenantContext;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

/// Request for storing tenant OAuth credentials  
#[derive(Debug)]
pub struct StoreCredentialsRequest {
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (will be encrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// User who configured these credentials
    pub configured_by: Uuid,
}

/// Tenant-aware OAuth client with credential isolation and rate limiting
pub struct TenantOAuthClient {
    /// OAuth manager handling tenant-specific credentials
    pub oauth_manager: Arc<Mutex<TenantOAuthManager>>,
}

impl TenantOAuthClient {
    /// Create new tenant OAuth client with provided manager
    #[must_use]
    pub fn new(oauth_manager: TenantOAuthManager) -> Self {
        Self {
            oauth_manager: Arc::new(Mutex::new(oauth_manager)),
        }
    }

    /// Get `OAuth2Client` configured for specific tenant and provider
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tenant exceeds daily rate limit for the provider
    /// - No OAuth credentials configured for tenant and provider
    /// - OAuth configuration creation fails
    pub async fn get_oauth_client(
        &self,
        tenant_context: &TenantContext,
        provider: &str,
        database: &Database,
    ) -> AppResult<OAuth2Client> {
        // Check rate limit first
        let manager = self.oauth_manager.lock().await;
        let (current_usage, daily_limit) =
            manager.check_rate_limit(tenant_context.tenant_id, provider)?;

        if current_usage >= daily_limit {
            return Err(AppError::invalid_input(format!(
                "Tenant {} has exceeded daily rate limit for provider {}: {}/{}",
                tenant_context.tenant_id, provider, current_usage, daily_limit
            )));
        }

        // Get credentials with user-specific priority
        let credentials = manager
            .get_credentials_for_user(
                Some(tenant_context.user_id),
                tenant_context.tenant_id,
                provider,
                database,
            )
            .await?;
        drop(manager);

        // Build OAuth2Config from tenant credentials
        let oauth_config = Self::build_oauth_config(&credentials, provider)?;

        info!(
            "Created OAuth client for tenant={}, provider={}, client_id={}",
            tenant_context.tenant_id, provider, credentials.client_id
        );

        Ok(OAuth2Client::new(oauth_config))
    }

    /// Get authorization URL for tenant-specific OAuth flow
    ///
    /// # Errors
    ///
    /// Returns an error if OAuth client creation or authorization URL generation fails
    pub async fn get_authorization_url(
        &self,
        tenant_context: &TenantContext,
        provider: &str,
        state: &str,
        database: &Database,
    ) -> AppResult<String> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        oauth_client.get_authorization_url(state).map_err(|e| {
            AppError::external_service(
                "oauth2",
                format!("OAuth authorization URL generation failed: {e}"),
            )
        })
    }

    /// Get authorization URL with PKCE for tenant-specific OAuth flow
    ///
    /// # Errors
    ///
    /// Returns an error if OAuth client creation or authorization URL generation fails
    pub async fn get_authorization_url_with_pkce(
        &self,
        tenant_context: &TenantContext,
        provider: &str,
        state: &str,
        pkce: &PkceParams,
        database: &Database,
    ) -> AppResult<String> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        oauth_client
            .get_authorization_url_with_pkce(state, pkce)
            .map_err(|e| {
                AppError::external_service(
                    "oauth2",
                    format!("OAuth authorization URL with PKCE generation failed: {e}"),
                )
            })
    }

    /// Exchange authorization code for access token
    ///
    /// # Errors
    ///
    /// Returns an error if OAuth client creation or token exchange fails
    pub async fn exchange_code(
        &self,
        tenant_context: &TenantContext,
        provider: &str,
        code: &str,
        database: &Database,
    ) -> AppResult<OAuth2Token> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        let token = oauth_client.exchange_code(code).await.map_err(|e| {
            AppError::external_service("oauth2", format!("OAuth code exchange failed: {e}"))
        })?;

        // Increment usage counter
        self.oauth_manager.lock().await.increment_usage(
            tenant_context.tenant_id,
            provider,
            1,
            0,
        )?;

        info!(
            "Successfully exchanged OAuth code for tenant={}, provider={}",
            tenant_context.tenant_id, provider
        );

        Ok(token)
    }

    /// Exchange authorization code with PKCE for access token
    ///
    /// # Errors
    ///
    /// Returns an error if OAuth client creation or token exchange fails
    pub async fn exchange_code_with_pkce(
        &self,
        tenant_context: &TenantContext,
        provider: &str,
        code: &str,
        pkce: &PkceParams,
        database: &Database,
    ) -> AppResult<OAuth2Token> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        let token = oauth_client
            .exchange_code_with_pkce(code, pkce)
            .await
            .map_err(|e| {
                AppError::external_service(
                    "oauth2",
                    format!("OAuth code exchange with PKCE failed: {e}"),
                )
            })?;

        // Increment usage counter
        self.oauth_manager.lock().await.increment_usage(
            tenant_context.tenant_id,
            provider,
            1,
            0,
        )?;

        info!(
            "Successfully exchanged OAuth code with PKCE for tenant={}, provider={}",
            tenant_context.tenant_id, provider
        );

        Ok(token)
    }

    /// Refresh access token
    ///
    /// # Errors
    ///
    /// Returns an error if OAuth client creation or token refresh fails
    pub async fn refresh_token(
        &self,
        tenant_context: &TenantContext,
        provider: &str,
        refresh_token: &str,
        database: &Database,
    ) -> AppResult<OAuth2Token> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        let token = oauth_client
            .refresh_token(refresh_token)
            .await
            .map_err(|e| {
                AppError::external_service("oauth2", format!("OAuth token refresh failed: {e}"))
            })?;

        // Increment usage counter
        self.oauth_manager.lock().await.increment_usage(
            tenant_context.tenant_id,
            provider,
            1,
            0,
        )?;

        info!(
            "Successfully refreshed OAuth token for tenant={}, provider={}",
            tenant_context.tenant_id, provider
        );

        Ok(token)
    }

    /// Check if tenant can make OAuth requests (rate limit check)
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit checking fails
    pub async fn check_rate_limit(&self, tenant_id: Uuid, provider: &str) -> AppResult<(u32, u32)> {
        let manager = self.oauth_manager.lock().await;
        manager.check_rate_limit(tenant_id, provider)
    }

    /// Get tenant's OAuth credentials (without decrypted secret)
    ///
    /// # Errors
    ///
    /// Returns an error if credential retrieval fails
    pub async fn get_tenant_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
        database: &Database,
    ) -> AppResult<Option<TenantOAuthCredentials>> {
        let manager = self.oauth_manager.lock().await;
        manager
            .get_credentials(tenant_id, provider, database)
            .await
            .map(Some)
    }

    /// Store OAuth credentials for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if credential storage fails
    pub async fn store_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
        request: StoreCredentialsRequest,
    ) -> AppResult<()> {
        let config = CredentialConfig {
            client_id: request.client_id,
            client_secret: request.client_secret,
            redirect_uri: request.redirect_uri,
            scopes: request.scopes,
            configured_by: request.configured_by,
        };

        let mut manager = self.oauth_manager.lock().await;
        manager.store_credentials(tenant_id, provider, config)
    }

    /// Build `OAuth2Config` from tenant credentials
    fn build_oauth_config(
        credentials: &TenantOAuthCredentials,
        provider: &str,
    ) -> AppResult<OAuth2Config> {
        let (auth_url, token_url, use_pkce) = match provider {
            "strava" => (
                "https://www.strava.com/oauth/authorize".to_owned(),
                "https://www.strava.com/oauth/token".to_owned(),
                true,
            ),
            "fitbit" => (
                "https://www.fitbit.com/oauth2/authorize".to_owned(),
                "https://api.fitbit.com/oauth2/token".to_owned(),
                true,
            ),
            "garmin" => (
                "https://connect.garmin.com/oauthConfirm".to_owned(),
                "https://connectapi.garmin.com/oauth-service/oauth/access_token".to_owned(),
                false, // Garmin uses OAuth 1.0a, no PKCE
            ),
            "whoop" => (
                "https://api.prod.whoop.com/oauth/oauth2/auth".to_owned(),
                "https://api.prod.whoop.com/oauth/oauth2/token".to_owned(),
                true,
            ),
            "terra" => (
                "https://widget.tryterra.co/session".to_owned(),
                "https://api.tryterra.co/v2/auth/token".to_owned(),
                false, // Terra uses API key auth, not standard OAuth
            ),
            _ => {
                warn!("Unknown provider {}, using generic OAuth URLs", provider);
                return Err(AppError::invalid_input(format!(
                    "Unsupported OAuth provider: {provider}"
                )));
            }
        };

        Ok(OAuth2Config {
            client_id: credentials.client_id.clone(),
            client_secret: credentials.client_secret.clone(),
            auth_url,
            token_url,
            redirect_uri: credentials.redirect_uri.clone(),
            scopes: credentials.scopes.clone(), // Safe: Option<String> ownership for OAuth config
            use_pkce,
        })
    }
}
