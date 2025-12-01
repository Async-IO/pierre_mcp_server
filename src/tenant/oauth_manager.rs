// ABOUTME: Per-tenant OAuth credential management for isolated multi-tenant operation
// ABOUTME: Handles secure storage, encryption, and retrieval of tenant-specific OAuth applications
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::database_plugins::{factory::Database, DatabaseProvider};
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
        self.get_credentials_for_user(None, tenant_id, provider, database)
            .await
    }

    /// Load OAuth credentials with user-specific priority
    ///
    /// Resolution order:
    /// 1. User-specific credentials (from `user_oauth_app_credentials` table)
    /// 2. Tenant-specific credentials (in-memory cache, then database)
    /// 3. Server-level OAuth configuration (environment variables)
    ///
    /// # Errors
    ///
    /// Returns an error if no credentials are found for the user/tenant/provider combination
    pub async fn get_credentials_for_user(
        &self,
        user_id: Option<Uuid>,
        tenant_id: Uuid,
        provider: &str,
        database: &Database,
    ) -> AppResult<TenantOAuthCredentials> {
        // Priority 1: Try user-specific credentials first (per-user OAuth app)
        if let Some(uid) = user_id {
            if let Some(credentials) = self
                .try_user_specific_credentials(uid, tenant_id, provider, database)
                .await
            {
                return Ok(credentials);
            }
        }

        // Priority 2: Try tenant-specific credentials (in-memory cache, then database)
        if let Some(credentials) = self
            .try_tenant_specific_credentials(tenant_id, provider, database)
            .await
        {
            return Ok(credentials);
        }

        // Priority 3: Fallback to server-level OAuth configuration
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
            "garmin" => self.try_garmin_config_credentials(tenant_id),
            "whoop" => self.try_whoop_config_credentials(tenant_id),
            "terra" => self.try_terra_config_credentials(tenant_id),
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

    /// Try to load Garmin credentials from `ServerConfig`
    fn try_garmin_config_credentials(&self, tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        let garmin_config = &self.oauth_config.garmin;

        if let (Some(client_id), Some(client_secret)) =
            (&garmin_config.client_id, &garmin_config.client_secret)
        {
            let redirect_uri = garmin_config
                .redirect_uri
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/api/oauth/callback/garmin".to_owned());
            tracing::info!(
                "Using server-level Garmin OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "garmin".to_owned(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                redirect_uri,
                scopes: if garmin_config.scopes.is_empty() {
                    vec!["wellness:read".to_owned(), "activities:read".to_owned()]
                } else {
                    garmin_config.scopes.clone()
                },
                rate_limit_per_day: crate::constants::rate_limits::GARMIN_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No Garmin OAuth credentials in ServerConfig for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
            tenant_id
        );
        None
    }

    /// Try to load WHOOP credentials from `ServerConfig`
    fn try_whoop_config_credentials(&self, tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        let whoop_config = &self.oauth_config.whoop;

        if let (Some(client_id), Some(client_secret)) =
            (&whoop_config.client_id, &whoop_config.client_secret)
        {
            let redirect_uri = whoop_config
                .redirect_uri
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/api/oauth/callback/whoop".to_owned());
            tracing::info!(
                "Using server-level WHOOP OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "whoop".to_owned(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                redirect_uri,
                scopes: if whoop_config.scopes.is_empty() {
                    vec![
                        "offline".to_owned(),
                        "read:profile".to_owned(),
                        "read:body_measurement".to_owned(),
                        "read:workout".to_owned(),
                        "read:sleep".to_owned(),
                        "read:recovery".to_owned(),
                        "read:cycles".to_owned(),
                    ]
                } else {
                    whoop_config.scopes.clone()
                },
                rate_limit_per_day: crate::constants::rate_limits::WHOOP_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No WHOOP OAuth credentials in ServerConfig for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
            tenant_id
        );
        None
    }

    /// Try to load Terra credentials from `ServerConfig`
    fn try_terra_config_credentials(&self, tenant_id: Uuid) -> Option<TenantOAuthCredentials> {
        let terra_config = &self.oauth_config.terra;

        if let (Some(client_id), Some(client_secret)) =
            (&terra_config.client_id, &terra_config.client_secret)
        {
            let redirect_uri = terra_config
                .redirect_uri
                .clone()
                .unwrap_or_else(|| "http://localhost:8080/api/oauth/callback/terra".to_owned());
            tracing::info!(
                "Using server-level Terra OAuth credentials for tenant {}",
                tenant_id
            );
            return Some(TenantOAuthCredentials {
                tenant_id,
                provider: "terra".to_owned(),
                client_id: client_id.clone(),
                client_secret: client_secret.clone(),
                redirect_uri,
                scopes: if terra_config.scopes.is_empty() {
                    vec![
                        "activity".to_owned(),
                        "sleep".to_owned(),
                        "body".to_owned(),
                        "daily".to_owned(),
                        "nutrition".to_owned(),
                    ]
                } else {
                    terra_config.scopes.clone()
                },
                rate_limit_per_day: crate::constants::rate_limits::TERRA_DEFAULT_DAILY_RATE_LIMIT,
            });
        }
        tracing::warn!(
            "No Terra OAuth credentials in ServerConfig for tenant {}. MCP client should provide these credentials via OAuth configuration tool.",
            tenant_id
        );
        None
    }

    /// Try to load user-specific OAuth credentials from database
    ///
    /// This allows individual users to configure their own OAuth application
    /// credentials for a provider, avoiding rate limits on shared apps.
    async fn try_user_specific_credentials(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        provider: &str,
        database: &Database,
    ) -> Option<TenantOAuthCredentials> {
        match database.get_user_oauth_app(user_id, provider).await {
            Ok(Some(user_app)) => {
                tracing::info!(
                    "Using user-specific {} OAuth credentials for user {} in tenant {}",
                    provider,
                    user_id,
                    tenant_id
                );
                Some(TenantOAuthCredentials {
                    tenant_id,
                    provider: provider.to_owned(),
                    client_id: user_app.client_id,
                    client_secret: user_app.client_secret,
                    redirect_uri: user_app.redirect_uri,
                    scopes: Self::default_scopes_for_provider(provider),
                    rate_limit_per_day: Self::default_rate_limit_for_provider(provider),
                })
            }
            Ok(None) => {
                tracing::debug!(
                    "No user-specific {} OAuth credentials found for user {} in tenant {}",
                    provider,
                    user_id,
                    tenant_id
                );
                None
            }
            Err(e) => {
                tracing::warn!(
                    "Error fetching user-specific {} OAuth credentials for user {}: {}",
                    provider,
                    user_id,
                    e
                );
                None
            }
        }
    }

    /// Get default scopes for a provider
    fn default_scopes_for_provider(provider: &str) -> Vec<String> {
        match provider.to_lowercase().as_str() {
            "strava" => crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                .split(',')
                .map(str::to_owned)
                .collect(),
            "fitbit" => vec![
                "activity".to_owned(),
                "heartrate".to_owned(),
                "location".to_owned(),
                "nutrition".to_owned(),
                "profile".to_owned(),
                "settings".to_owned(),
                "sleep".to_owned(),
                "social".to_owned(),
                "weight".to_owned(),
            ],
            "garmin" => vec!["wellness:read".to_owned(), "activities:read".to_owned()],
            "whoop" => vec![
                "offline".to_owned(),
                "read:profile".to_owned(),
                "read:body_measurement".to_owned(),
                "read:workout".to_owned(),
                "read:sleep".to_owned(),
                "read:recovery".to_owned(),
                "read:cycles".to_owned(),
            ],
            "terra" => vec![
                "activity".to_owned(),
                "sleep".to_owned(),
                "body".to_owned(),
                "daily".to_owned(),
                "nutrition".to_owned(),
            ],
            _ => vec![],
        }
    }

    /// Get default rate limit for a provider
    fn default_rate_limit_for_provider(provider: &str) -> u32 {
        match provider.to_lowercase().as_str() {
            "strava" => crate::constants::rate_limits::STRAVA_DEFAULT_DAILY_RATE_LIMIT,
            "fitbit" => crate::constants::rate_limits::FITBIT_DEFAULT_DAILY_RATE_LIMIT,
            "garmin" => crate::constants::rate_limits::GARMIN_DEFAULT_DAILY_RATE_LIMIT,
            "whoop" => crate::constants::rate_limits::WHOOP_DEFAULT_DAILY_RATE_LIMIT,
            "terra" => crate::constants::rate_limits::TERRA_DEFAULT_DAILY_RATE_LIMIT,
            _ => 1000, // Default fallback
        }
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
