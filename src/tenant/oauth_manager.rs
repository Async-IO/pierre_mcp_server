// ABOUTME: Per-tenant OAuth credential management for isolated multi-tenant operation
// ABOUTME: Handles secure storage, encryption, and retrieval of tenant-specific OAuth applications
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::database_plugins::{factory::Database, DatabaseProvider};
use anyhow::Result;
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
}

impl TenantOAuthManager {
    /// Create new OAuth manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            credentials: HashMap::new(),
            usage_tracking: HashMap::new(),
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
    ) -> Result<TenantOAuthCredentials> {
        // Priority 1: Try tenant-specific credentials first (in-memory cache, then database)
        if let Some(credentials) = self
            .try_tenant_specific_credentials(tenant_id, provider, database)
            .await
        {
            return Ok(credentials);
        }

        // Priority 2: Fallback to server-level environment variables
        if let Some(credentials) = Self::try_server_level_credentials(tenant_id, provider) {
            return Ok(credentials);
        }

        // No credentials found - return error
        Err(anyhow::anyhow!(
            "No OAuth credentials configured for tenant {} and provider {}. Configure {}_CLIENT_ID and {}_CLIENT_SECRET environment variables, or provide tenant-specific credentials via the MCP OAuth configuration tool.",
            tenant_id, provider, provider.to_uppercase(), provider.to_uppercase()
        ))
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
    ) -> Result<()> {
        let credentials = TenantOAuthCredentials {
            tenant_id,
            provider: provider.to_string(),
            client_id: config.client_id,
            client_secret: config.client_secret,
            redirect_uri: config.redirect_uri,
            scopes: config.scopes,
            rate_limit_per_day: crate::constants::rate_limits::STRAVA_DEFAULT_DAILY_RATE_LIMIT,
        };

        self.credentials
            .insert((tenant_id, provider.to_string()), credentials);
        Ok(())
    }

    /// Check tenant's daily rate limit usage
    ///
    /// # Errors
    ///
    /// Returns an error if rate limit check fails
    pub fn check_rate_limit(&self, tenant_id: Uuid, provider: &str) -> Result<(u32, u32)> {
        let today = Utc::now().date_naive();
        let usage = self
            .usage_tracking
            .get(&(tenant_id, provider.to_string(), today))
            .copied()
            .unwrap_or(0);

        // Get tenant's rate limit
        let daily_limit = self
            .credentials
            .get(&(tenant_id, provider.to_string()))
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
    ) -> Result<()> {
        let today = Utc::now().date_naive();
        let key = (tenant_id, provider.to_string(), today);
        let current = self.usage_tracking.get(&key).copied().unwrap_or(0);
        self.usage_tracking
            .insert(key, current + successful_requests);
        Ok(())
    }

    /// Try to load server-level OAuth credentials from environment variables
    fn try_server_level_credentials(
        tenant_id: Uuid,
        provider: &str,
    ) -> Option<TenantOAuthCredentials> {
        match provider.to_lowercase().as_str() {
            "strava" => Self::try_strava_env_credentials(tenant_id),
            "fitbit" => Self::try_fitbit_env_credentials(tenant_id),
            _ => {
                tracing::warn!("Unsupported OAuth provider: {}", provider);
                None
            }
        }
    }

    /// Try to load Strava credentials from environment variables
    fn try_strava_env_credentials(tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        if let (Ok(client_id), Ok(client_secret)) = (
            std::env::var("STRAVA_CLIENT_ID"),
            std::env::var("STRAVA_CLIENT_SECRET"),
        ) {
            let redirect_uri = crate::constants::env_config::strava_redirect_uri();
            tracing::info!(
                "Using server-level Strava OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "strava".to_string(),
                client_id,
                client_secret,
                redirect_uri,
                scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_string)
                    .collect(),
                rate_limit_per_day: crate::constants::rate_limits::STRAVA_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No STRAVA_CLIENT_ID/STRAVA_CLIENT_SECRET environment variables set for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
            tenant_id
        );
        None
    }

    /// Try to load Fitbit credentials from environment variables
    fn try_fitbit_env_credentials(tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        if let (Ok(client_id), Ok(client_secret)) = (
            std::env::var("FITBIT_CLIENT_ID"),
            std::env::var("FITBIT_CLIENT_SECRET"),
        ) {
            let redirect_uri = crate::constants::env_config::fitbit_redirect_uri();
            tracing::info!(
                "Using server-level Fitbit OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "fitbit".to_string(),
                client_id,
                client_secret,
                redirect_uri,
                scopes: vec![
                    "activity".to_string(),
                    "heartrate".to_string(),
                    "location".to_string(),
                    "nutrition".to_string(),
                    "profile".to_string(),
                    "settings".to_string(),
                    "sleep".to_string(),
                    "social".to_string(),
                    "weight".to_string(),
                ],
                rate_limit_per_day: crate::constants::rate_limits::FITBIT_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No FITBIT_CLIENT_ID/FITBIT_CLIENT_SECRET environment variables set for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
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
            .get(&(tenant_id, provider.to_string()))
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

impl Default for TenantOAuthManager {
    fn default() -> Self {
        Self::new()
    }
}
