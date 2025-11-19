// ABOUTME: Per-tenant OAuth credential management for isolated multi-tenant operation
// ABOUTME: Handles secure storage, encryption, and retrieval of tenant-specific OAuth applications
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::database_plugins::factory::Database;
use crate::errors::{AppError, AppResult};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Credential configuration for storing OAuth credentials
#[derive(Debug, Clone)]
pub struct CredentialConfig {
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (to be encrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// User who configured these credentials
    pub configured_by: Uuid,
}

/// Per-tenant OAuth credentials with decrypted secret
#[derive(Debug, Clone)]
pub struct TenantOAuthCredentials {
    /// Tenant ID that owns these credentials
    pub tenant_id: Uuid,
    /// OAuth provider name
    pub provider: String,
    /// OAuth client ID (public)
    pub client_id: String,
    /// OAuth client secret (decrypted)
    pub client_secret: String,
    /// OAuth redirect URI
    pub redirect_uri: String,
    /// OAuth scopes
    pub scopes: Vec<String>,
    /// Daily rate limit for this tenant
    pub rate_limit_per_day: u32,
}

/// Manager for tenant-specific OAuth credentials
///
/// Note: This is a simplified implementation for initial multi-tenant support.
/// In production, credentials would be encrypted and stored in the database.
pub struct TenantOAuthManager {
    // In-memory storage for now - would be database-backed in production
    credentials: HashMap<(Uuid, String), TenantOAuthCredentials>,
    usage_tracking: HashMap<(Uuid, String, chrono::NaiveDate), u32>,
    // Server-level OAuth configuration (read once at startup)
    oauth_config: std::sync::Arc<crate::config::environment::OAuthConfig>,
}

impl TenantOAuthManager {
    /// Create new OAuth manager with server-level configuration
    #[must_use]
    pub fn new(oauth_config: std::sync::Arc<crate::config::environment::OAuthConfig>) -> Self {
        Self {
            credentials: HashMap::new(),
            usage_tracking: HashMap::new(),
            oauth_config,
        }
    }

    /// Load OAuth credentials for a specific tenant and provider
    ///
    /// # Errors
    ///
    /// Returns an error if no credentials are found for the tenant/provider combination
    pub async fn get_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
        database: &Database,
    ) -> AppResult<TenantOAuthCredentials> {
        // Priority 1: Try tenant-specific credentials first (in-memory cache, then database)
        if let Some(credentials) = self
            .try_tenant_specific_credentials(tenant_id, provider, database)
            .await
        {
            return Ok(credentials);
        }

        // Priority 2: Fallback to server-level OAuth configuration
        if let Some(credentials) = self.try_server_level_credentials(tenant_id, provider) {
            return Ok(credentials);
        }

        // No credentials found - return error
        Err(AppError::not_found(format!(
            "No OAuth credentials configured for tenant {} and provider {}. Configure {}_CLIENT_ID and {}_CLIENT_SECRET environment variables, or provide tenant-specific credentials via the MCP OAuth configuration tool.",
            tenant_id, provider, provider.to_uppercase(), provider.to_uppercase()
        )))
    }

    /// Store OAuth credentials for a tenant
    ///
    /// # Errors
    ///
    /// Returns an error if credential storage fails
    pub fn store_credentials(
        &mut self,
        tenant_id: Uuid,
        provider: &str,
        config: CredentialConfig,
    ) -> AppResult<()> {
        let credentials = TenantOAuthCredentials {
            tenant_id,
            provider: provider.to_owned(),
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            scopes: config.scopes,
            rate_limit_per_day: crate::constants::rate_limits::STRAVA_DEFAULT_DAILY_RATE_LIMIT,
        };

        self.credentials
            .insert((tenant_id, provider.to_owned()), credentials);
        Ok(())
    }

    /// Check tenant's daily rate limit usage
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit check fails
    pub fn check_rate_limit(&self, tenant_id: Uuid, provider: &str) -> AppResult<(u32, u32)> {
        let today = Utc::now().date_naive();
        let usage = self
            .usage_tracking
            .get(&(tenant_id, provider.to_owned(), today))
            .copied()
            .unwrap_or(0);

        // Get tenant's rate limit
        let daily_limit = self
            .credentials
            .get(&(tenant_id, provider.to_owned()))
            .map_or(
                crate::constants::rate_limits::STRAVA_DEFAULT_DAILY_RATE_LIMIT,
                |c| c.rate_limit_per_day,
            );

        Ok((usage, daily_limit))
    }

    /// Increment tenant's usage counter
    ///
    /// # Errors
    ///
    /// Returns an error if usage increment fails
    pub fn increment_usage(
        &mut self,
        tenant_id: Uuid,
        provider: &str,
        successful_requests: u32,
        _failed_requests: u32,
    ) -> AppResult<()> {
        let today = Utc::now().date_naive();
        let key = (tenant_id, provider.to_owned(), today);
        let current = self.usage_tracking.get(&key).copied().unwrap_or(0);
        self.usage_tracking
            .insert(key, current + successful_requests);
        Ok(())
    }

    /// Try to load server-level OAuth credentials from `ServerConfig`
    fn try_server_level_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
    ) -> Option<TenantOAuthCredentials> {
        match provider.to_lowercase().as_str() {
            "strava" => self.try_strava_config_credentials(tenant_id),
            "fitbit" => self.try_fitbit_config_credentials(tenant_id),
            _ => {
                tracing::warn!("Unsupported OAuth provider: {}", provider);
                None
            }
        }
    }

    /// Try to load Strava credentials from `ServerConfig`
    fn try_strava_config_credentials(&self, tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        let strava_config = &self.oauth_config.strava;

        if let (Some(client_id), Some(client_secret)) =
            (&strava_config.client_id, &strava_config.client_secret)
        {
            let redirect_uri = strava_config
                .redirect_uri
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/api/oauth/callback/strava".to_owned());
            tracing::info!(
                "Using server-level Strava OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "strava".to_owned(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                redirect_uri,
                scopes: if strava_config.scopes.is_empty() {
                    crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                        .split(',')
                        .map(str::to_owned)
                        .collect()
                } else {
                    strava_config.scopes.clone()
                },
                rate_limit_per_day: crate::constants::rate_limits::STRAVA_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No Strava OAuth credentials in ServerConfig for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
            tenant_id
        );
        None
    }

    /// Try to load Fitbit credentials from `ServerConfig`
    fn try_fitbit_config_credentials(&self, tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        let fitbit_config = &self.oauth_config.fitbit;

        if let (Some(client_id), Some(client_secret)) =
            (&fitbit_config.client_id, &fitbit_config.client_secret)
        {
            let redirect_uri = fitbit_config
                .redirect_uri
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/api/oauth/callback/fitbit".to_owned());
            tracing::info!(
                "Using server-level Fitbit OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "fitbit".to_owned(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                redirect_uri,
                scopes: if fitbit_config.scopes.is_empty() {
                    vec![
                        "activity".to_owned(),
                        "heartrate".to_owned(),
                        "location".to_owned(),
                        "nutrition".to_owned(),
                        "profile".to_owned(),
                        "settings".to_owned(),
                        "sleep".to_owned(),
                        "social".to_owned(),
                        "weight".to_owned(),
                    ]
                } else {
                    fitbit_config.scopes.clone()
                },
                rate_limit_per_day: crate::constants::rate_limits::FITBIT_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No Fitbit OAuth credentials in ServerConfig for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
            tenant_id
        );
        None
    }

    /// Try to load tenant-specific OAuth credentials from memory cache and database
    async fn try_tenant_specific_credentials(
        &self,
        tenant_id: Uuid,
        provider: &str,
        database: &Database,
    ) -> Option<TenantOAuthCredentials> {
        // First check in-memory cache
        if let Some(credentials) = self
            .credentials
            .get(&(tenant_id, provider.to_owned()))
            .cloned()
        {
            tracing::info!(
                "Using cached tenant-specific {} OAuth credentials for tenant {}",
                provider,
                tenant_id
            );
            return Some(credentials);
        }

        // Then check database
        if let Ok(Some(db_credentials)) = database
            .get_tenant_oauth_credentials(tenant_id, provider)
            .await
        {
            tracing::info!(
                "Using database-stored tenant-specific {} OAuth credentials for tenant {}",
                provider,
                tenant_id
            );
            return Some(db_credentials);
        }

        tracing::debug!(
            "No tenant-specific {} OAuth credentials found for tenant {}",
            provider,
            tenant_id
        );
        None
    }
}
