// ABOUTME: Tenant-aware OAuth client for multi-tenant fitness platform authentication
// ABOUTME: Provides OAuth flow integration with tenant-specific credentials and rate limiting
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - OAuth credential string ownership transfers (client_id, client_secret, redirect_uri)
// - Tenant context ownership for multi-tenant OAuth flows

use super::oauth_manager::{CredentialConfig, TenantOAuthCredentials, TenantOAuthManager};
use super::TenantContext;
use crate::database_plugins::factory::Database;
use crate::errors::AppError;
use crate::oauth2_client::{OAuth2Client, OAuth2Config, OAuth2Token, PkceParams};
use anyhow::Result;
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
    /// Shared OAuth manager instance for handling tenant-specific OAuth operations
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
    ) -> Result<OAuth2Client> {
        // Check rate limit first
        let manager = self.oauth_manager.lock().await;
        let (current_usage, daily_limit) =
            manager.check_rate_limit(tenant_context.tenant_id, provider)?;

        if current_usage >= daily_limit {
            return Err(AppError::invalid_input(format!(
                "Tenant {} has exceeded daily rate limit for provider {}: {}/{}",
                tenant_context.tenant_id, provider, current_usage, daily_limit
            ))
            .into());
        }

        // Get tenant credentials
        let credentials = manager
            .get_credentials(tenant_context.tenant_id, provider, database)
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
    ) -> Result<String> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        oauth_client.get_authorization_url(state)
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
    ) -> Result<String> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        oauth_client.get_authorization_url_with_pkce(state, pkce)
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
    ) -> Result<OAuth2Token> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        let token = oauth_client.exchange_code(code).await?;

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
    ) -> Result<OAuth2Token> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        let token = oauth_client.exchange_code_with_pkce(code, pkce).await?;

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
    ) -> Result<OAuth2Token> {
        let oauth_client = self
            .get_oauth_client(tenant_context, provider, database)
            .await?;
        let token = oauth_client.refresh_token(refresh_token).await?;

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
    pub async fn check_rate_limit(&self, tenant_id: Uuid, provider: &str) -> Result<(u32, u32)> {
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
    ) -> Result<Option<TenantOAuthCredentials>> {
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
    ) -> Result<()> {
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
    ) -> Result<OAuth2Config> {
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
            _ => {
                warn!("Unknown provider {}, using generic OAuth URLs", provider);
                return Err(AppError::invalid_input(format!(
                    "Unsupported OAuth provider: {provider}"
                ))
                .into());
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
