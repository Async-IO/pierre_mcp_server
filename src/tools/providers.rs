// ABOUTME: Unified provider management for fitness platforms
// ABOUTME: Standardizes provider operations across single-tenant and multi-tenant implementations

use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::providers::FitnessProvider;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Supported fitness providers
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProviderType {
    Strava,
    Fitbit,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Strava => write!(f, "strava"),
            Self::Fitbit => write!(f, "fitbit"),
        }
    }
}

impl std::str::FromStr for ProviderType {
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
        expires_at: chrono::DateTime<chrono::Utc>,
        scopes: Vec<String>,
    },
    /// Provider is connected but tokens need refresh
    TokenExpired {
        expired_at: chrono::DateTime<chrono::Utc>,
    },
    /// Provider is not connected
    Disconnected,
    /// Provider connection failed
    Failed {
        error: String,
        last_attempt: chrono::DateTime<chrono::Utc>,
    },
}

/// Provider information for user context
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProviderInfo {
    pub provider_type: ProviderType,
    pub status: ConnectionStatus,
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub data_available: bool,
}

/// Type alias for complex provider cache type
type ProviderCache =
    tokio::sync::RwLock<HashMap<(Uuid, ProviderType), Arc<Box<dyn FitnessProvider>>>>;

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
            provider_cache: tokio::sync::RwLock::new(HashMap::new()),
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
        let token = match provider_type {
            ProviderType::Strava => self.database.get_strava_token(user_id).await?,
            ProviderType::Fitbit => self.database.get_fitbit_token(user_id).await?,
        };

        let status = match token {
            Some(token_data) => {
                if token_data.expires_at > chrono::Utc::now() {
                    ConnectionStatus::Connected {
                        expires_at: token_data.expires_at,
                        scopes: token_data
                            .scope
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect(),
                    }
                } else {
                    ConnectionStatus::TokenExpired {
                        expired_at: token_data.expires_at,
                    }
                }
            }
            None => ConnectionStatus::Disconnected,
        };

        // Get last sync timestamp
        let last_sync = self
            .database
            .get_provider_last_sync(user_id, &provider_type.to_string())
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
        // Remove from database
        match provider_type {
            ProviderType::Strava => self.database.clear_strava_token(user_id).await?,
            ProviderType::Fitbit => self.database.clear_fitbit_token(user_id).await?,
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
    /// # Errors
    ///
    /// Returns an error if database operations fail
    pub async fn update_sync_timestamp(
        &self,
        user_id: Uuid,
        provider_type: ProviderType,
    ) -> Result<(), AppError> {
        let sync_time = chrono::Utc::now();
        self.database
            .update_provider_last_sync(user_id, &provider_type.to_string(), sync_time)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update sync timestamp: {e}")))?;
        Ok(())
    }
}

/// Global provider manager instance
/// This provides a singleton for use across the application
pub struct GlobalProviderManager {
    inner: tokio::sync::OnceCell<ProviderManager>,
}

impl GlobalProviderManager {
    const fn new() -> Self {
        Self {
            inner: tokio::sync::OnceCell::const_new(),
        }
    }

    /// Initialize the global provider manager
    /// # Errors
    ///
    /// Returns an error if provider manager is already initialized
    pub fn init(&self, database: Arc<Database>) -> Result<(), AppError> {
        self.inner
            .set(ProviderManager::new(database))
            .map_err(|_| AppError::internal("Provider manager already initialized"))?;
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
