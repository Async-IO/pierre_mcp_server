// ABOUTME: Unified provider management for fitness platforms
// ABOUTME: Standardizes provider operations across single-tenant and multi-tenant implementations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::{
    constants::oauth_providers::{FITBIT, STRAVA},
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppError,
    models::TenantId,
    providers::CoreFitnessProvider,
};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
    sync::Arc,
};
use tokio::sync::{OnceCell, RwLock};
use tracing::error;
use uuid::Uuid;

/// Supported fitness providers
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProviderType {
    /// Strava fitness platform
    Strava,
    /// Fitbit fitness platform
    Fitbit,
}

impl Display for ProviderType {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Strava => write!(f, "strava"),
            Self::Fitbit => write!(f, "fitbit"),
        }
    }
}

impl FromStr for ProviderType {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "strava" => Ok(Self::Strava),
            "fitbit" => Ok(Self::Fitbit),
            _ => Err(AppError::invalid_input(format!(
                "Unsupported provider: {s}"
            ))),
        }
    }
}

/// Provider connection status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConnectionStatus {
    /// Provider is connected and tokens are valid
    Connected {
        /// When the access token expires
        expires_at: chrono::DateTime<chrono::Utc>,
        /// OAuth scopes granted
        scopes: Vec<String>,
    },
    /// Provider is connected but tokens need refresh
    TokenExpired {
        /// When the token expired
        expired_at: chrono::DateTime<chrono::Utc>,
    },
    /// Provider is not connected
    Disconnected,
    /// Provider connection failed
    Failed {
        /// Error message describing the failure
        error: String,
        /// When the last connection attempt was made
        last_attempt: chrono::DateTime<chrono::Utc>,
    },
}

/// Provider information for user context
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    /// Type of fitness provider
    pub provider_type: ProviderType,
    /// Current connection status
    pub status: ConnectionStatus,
    /// When data was last synchronized
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether data is available from this provider
    pub data_available: bool,
}

/// Type alias for complex provider cache type
type ProviderCache = RwLock<HashMap<(Uuid, ProviderType), Arc<Box<dyn CoreFitnessProvider>>>>;

/// Unified provider manager
pub struct ProviderManager {
    database: Arc<Database>,
    /// Cache of authenticated providers per user
    provider_cache: ProviderCache,
}

impl ProviderManager {
    /// Create a new provider manager
    #[must_use]
    pub fn new(database: Arc<Database>) -> Self {
        Self {
            database,
            provider_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Get all provider information for a user
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_user_providers(&self, user_id: Uuid) -> Result<Vec<ProviderInfo>, AppError> {
        let mut providers = Vec::new();

        // Check Strava
        if let Ok(strava_info) = self.get_provider_info(user_id, ProviderType::Strava).await {
            providers.push(strava_info);
        }

        // Check Fitbit
        if let Ok(fitbit_info) = self.get_provider_info(user_id, ProviderType::Fitbit).await {
            providers.push(fitbit_info);
        }

        Ok(providers)
    }

    /// Get provider information for a specific provider
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_provider_info(
        &self,
        user_id: Uuid,
        provider_type: ProviderType,
    ) -> Result<ProviderInfo, AppError> {
        // Get user's default tenant from tenant_users junction table
        let tenants = self
            .database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;
        let tenant_id: TenantId = tenants
            .first()
            .map(|t| t.id)
            .ok_or_else(|| AppError::invalid_input("User has no tenant"))?;

        let token = match provider_type {
            ProviderType::Strava => self
                .database
                .get_user_oauth_token(user_id, tenant_id, STRAVA)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to get Strava OAuth token: {e}"))
                })?,
            ProviderType::Fitbit => self
                .database
                .get_user_oauth_token(user_id, tenant_id, FITBIT)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to get Fitbit OAuth token: {e}"))
                })?,
        };

        let status = match token {
            Some(token_data) => {
                if let Some(expires_at) = token_data.expires_at {
                    if expires_at > chrono::Utc::now() {
                        ConnectionStatus::Connected {
                            expires_at,
                            scopes: token_data
                                .scope
                                .unwrap_or_default()
                                .split(',')
                                .map(|s| s.trim().to_owned())
                                .collect(),
                        }
                    } else {
                        ConnectionStatus::TokenExpired {
                            expired_at: expires_at,
                        }
                    }
                } else {
                    // Token has no expiration time, treat as disconnected
                    ConnectionStatus::Disconnected
                }
            }
            None => ConnectionStatus::Disconnected,
        };

        // Get last sync timestamp (scoped to tenant for multi-tenant isolation)
        let last_sync = self
            .database
            .get_provider_last_sync(user_id, tenant_id, &provider_type.to_string())
            .await
            .unwrap_or(None);

        let data_available = matches!(status, ConnectionStatus::Connected { .. });

        Ok(ProviderInfo {
            provider_type,
            status,
            last_sync,
            data_available,
        })
    }

    /// Disconnect a provider for a user
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn disconnect_provider(
        &self,
        user_id: Uuid,
        provider_type: ProviderType,
    ) -> Result<(), AppError> {
        // Get user's default tenant from tenant_users junction table
        let tenants = self
            .database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;
        let tenant_id: TenantId = tenants
            .first()
            .map(|t| t.id)
            .ok_or_else(|| AppError::invalid_input("User has no tenant"))?;

        // Remove from database
        match provider_type {
            ProviderType::Strava => {
                self.database
                    .delete_user_oauth_token(user_id, tenant_id, STRAVA)
                    .await
                    .map_err(|e| {
                        AppError::database(format!("Failed to delete Strava OAuth token: {e}"))
                    })?;
            }
            ProviderType::Fitbit => {
                self.database
                    .delete_user_oauth_token(user_id, tenant_id, FITBIT)
                    .await
                    .map_err(|e| {
                        AppError::database(format!("Failed to delete Fitbit OAuth token: {e}"))
                    })?;
            }
        }

        // Remove from cache
        {
            let mut cache = self.provider_cache.write().await;
            cache.remove(&(user_id, provider_type));
        }

        Ok(())
    }

    /// Check connection status for all providers
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn check_all_connections(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ProviderInfo>, AppError> {
        self.get_user_providers(user_id).await
    }

    /// Clear the provider cache for a user (useful for logout)
    pub async fn clear_user_cache(&self, user_id: Uuid) {
        let mut cache = self.provider_cache.write().await;
        cache.retain(|(cached_user_id, _), _| *cached_user_id != user_id);
    }

    /// Clear all cached providers
    pub async fn clear_all_cache(&self) {
        let mut cache = self.provider_cache.write().await;
        cache.clear();
    }

    /// Get connection summary for a user
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn get_connection_summary(
        &self,
        user_id: Uuid,
    ) -> Result<serde_json::Value, AppError> {
        let providers = self.get_user_providers(user_id).await?;

        let connected_count = providers
            .iter()
            .filter(|p| matches!(p.status, ConnectionStatus::Connected { .. }))
            .count();

        let expired_count = providers
            .iter()
            .filter(|p| matches!(p.status, ConnectionStatus::TokenExpired { .. }))
            .count();

        Ok(serde_json::json!({
            "total_providers": providers.len(),
            "connected": connected_count,
            "expired": expired_count,
            "disconnected": providers.len() - connected_count - expired_count,
            "providers": providers,
        }))
    }

    /// Update sync timestamp for a provider after successful data fetch
    ///
    /// Resolves the user's tenant and scopes the update to prevent
    /// cross-tenant sync timestamp collisions.
    ///
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn update_sync_timestamp(
        &self,
        user_id: Uuid,
        provider_type: ProviderType,
    ) -> Result<(), AppError> {
        let tenants = self
            .database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;
        let tenant_id: TenantId = tenants
            .first()
            .map(|t| t.id)
            .ok_or_else(|| AppError::invalid_input("User has no tenant"))?;

        let sync_time = chrono::Utc::now();
        self.database
            .update_provider_last_sync(user_id, tenant_id, &provider_type.to_string(), sync_time)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update sync timestamp: {e}")))?;
        Ok(())
    }
}

/// Global provider manager instance
/// This provides a singleton for use across the application
pub struct GlobalProviderManager {
    inner: OnceCell<ProviderManager>,
}

impl GlobalProviderManager {
    const fn new() -> Self {
        Self {
            inner: OnceCell::const_new(),
        }
    }

    /// Initialize the global provider manager
    /// # Errors
    ///
    /// Returns an error if provider manager is already initialized
    pub fn init(&self, database: Arc<Database>) -> Result<(), AppError> {
        self.inner
            .set(ProviderManager::new(database))
            .map_err(|_| {
                error!(
                    "Attempted to initialize provider manager multiple times (programming error)"
                );
                AppError::internal("Provider manager already initialized")
            })?;
        Ok(())
    }

    /// Get the global provider manager instance
    /// # Errors
    ///
    /// Returns an error if provider manager is not initialized
    pub fn get(&self) -> Result<&ProviderManager, AppError> {
        self.inner
            .get()
            .ok_or_else(|| AppError::internal("Provider manager not initialized"))
    }
}

/// Global provider manager instance
pub static PROVIDER_MANAGER: GlobalProviderManager = GlobalProviderManager::new();
