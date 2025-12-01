// ABOUTME: Terra provider implementing FitnessProvider trait
// ABOUTME: Reads from webhook-populated cache to serve activities, sleep, recovery, and health data
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Terra `FitnessProvider` implementation
//!
//! This module implements the `FitnessProvider` trait for Terra, enabling
//! Pierre's unified provider interface to work with Terra's webhook-based
//! data delivery model.
//!
//! The provider reads from a local cache populated by the webhook handler,
//! effectively bridging Terra's push model to Pierre's pull model.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::errors::AppResult;
use crate::models::{
    Activity, Athlete, HealthMetrics, PersonalRecord, RecoveryMetrics, SleepSession, Stats,
};
use crate::pagination::{Cursor, CursorPage, PaginationParams};
use crate::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig, ProviderFactory};
use crate::providers::errors::ProviderError;
use crate::providers::spi::{
    OAuthEndpoints, OAuthParams, ProviderCapabilities, ProviderDescriptor,
};

use super::api_client::{TerraApiClient, TerraApiConfig};
use super::cache::TerraDataCache;
use super::constants::{
    TERRA_API_BASE_URL, TERRA_DEAUTH_URL, TERRA_TOKEN_URL, TERRA_WIDGET_SESSION_URL,
};

/// Terra provider for accessing fitness data from 150+ wearables
///
/// Unlike direct providers (Strava, Garmin), Terra uses a webhook-based model.
/// This provider reads from a local cache populated by webhook events.
pub struct TerraProvider {
    config: ProviderConfig,
    credentials: RwLock<Option<OAuth2Credentials>>,
    cache: Arc<TerraDataCache>,
    api_client: Option<TerraApiClient>,
    /// Terra user ID for this provider instance
    terra_user_id: RwLock<Option<String>>,
}

impl TerraProvider {
    /// Create a new Terra provider with default configuration
    #[must_use]
    pub fn new(cache: Arc<TerraDataCache>) -> Self {
        Self {
            config: ProviderConfig {
                name: "terra".to_owned(),
                auth_url: TERRA_WIDGET_SESSION_URL.to_owned(),
                token_url: TERRA_TOKEN_URL.to_owned(),
                api_base_url: TERRA_API_BASE_URL.to_owned(),
                revoke_url: Some(TERRA_DEAUTH_URL.to_owned()),
                default_scopes: vec![
                    "activity".to_owned(),
                    "sleep".to_owned(),
                    "body".to_owned(),
                    "daily".to_owned(),
                    "nutrition".to_owned(),
                ],
            },
            credentials: RwLock::new(None),
            cache,
            api_client: None,
            terra_user_id: RwLock::new(None),
        }
    }

    /// Create a Terra provider with custom configuration
    #[must_use]
    pub fn with_config(config: ProviderConfig, cache: Arc<TerraDataCache>) -> Self {
        Self {
            config,
            credentials: RwLock::new(None),
            cache,
            api_client: None,
            terra_user_id: RwLock::new(None),
        }
    }

    /// Create a Terra provider with API client for REST operations
    #[must_use]
    pub fn with_api_client(cache: Arc<TerraDataCache>, api_config: TerraApiConfig) -> Self {
        let api_client = TerraApiClient::new(api_config);
        Self {
            config: ProviderConfig {
                name: "terra".to_owned(),
                auth_url: TERRA_WIDGET_SESSION_URL.to_owned(),
                token_url: TERRA_TOKEN_URL.to_owned(),
                api_base_url: TERRA_API_BASE_URL.to_owned(),
                revoke_url: Some(TERRA_DEAUTH_URL.to_owned()),
                default_scopes: vec![
                    "activity".to_owned(),
                    "sleep".to_owned(),
                    "body".to_owned(),
                    "daily".to_owned(),
                    "nutrition".to_owned(),
                ],
            },
            credentials: RwLock::new(None),
            cache,
            api_client: Some(api_client),
            terra_user_id: RwLock::new(None),
        }
    }

    /// Set the Terra user ID for this provider instance
    pub async fn set_terra_user_id(&self, user_id: &str) {
        let mut id = self.terra_user_id.write().await;
        *id = Some(user_id.to_owned());
    }

    /// Get the current Terra user ID
    async fn get_user_id(&self) -> Result<String, ProviderError> {
        let id = self.terra_user_id.read().await;
        id.clone()
            .ok_or_else(|| ProviderError::AuthenticationFailed {
                provider: "terra".to_owned(),
                reason: "Terra user ID not set. Call set_terra_user_id() first.".to_owned(),
            })
    }
}

#[async_trait]
impl FitnessProvider for TerraProvider {
    fn name(&self) -> &'static str {
        "terra"
    }

    fn config(&self) -> &ProviderConfig {
        &self.config
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        // For Terra, credentials contain the API key and dev ID
        // The access_token field stores the Terra user ID
        if let Some(ref token) = credentials.access_token {
            self.set_terra_user_id(token).await;
        }
        *self.credentials.write().await = Some(credentials);
        Ok(())
    }

    async fn is_authenticated(&self) -> bool {
        let id = self.terra_user_id.read().await;
        id.is_some()
    }

    async fn refresh_token_if_needed(&self) -> AppResult<()> {
        // Terra uses API keys, not OAuth tokens that need refreshing
        // The webhook connection stays active as long as the API key is valid
        Ok(())
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        let user_id = self.get_user_id().await?;

        // If we have an API client, fetch user info
        if let Some(ref client) = self.api_client {
            let user_info = client.get_user_info(&user_id).await?;
            if let Some(user) = user_info.user {
                return Ok(Athlete {
                    id: user.user_id,
                    username: user.reference_id.unwrap_or_default(),
                    firstname: None,
                    lastname: None,
                    profile_picture: None,
                    provider: format!("terra:{}", user.provider.to_lowercase()),
                });
            }
        }

        // Return basic athlete info from user ID
        Ok(Athlete {
            id: user_id.clone(),
            username: user_id,
            firstname: None,
            lastname: None,
            profile_picture: None,
            provider: "terra".to_owned(),
        })
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        let user_id = self.get_user_id().await?;
        let activities = self.cache.get_activities(&user_id, limit, offset).await;
        Ok(activities)
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        let user_id = self.get_user_id().await?;

        // Get all activities and sort by start_date descending
        let mut activities = self.cache.get_activities(&user_id, None, None).await;
        activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        // Find starting position based on cursor
        let start_index = params.cursor.as_ref().map_or(0, |cursor| {
            cursor.decode().map_or(0, |(_timestamp, id)| {
                activities
                    .iter()
                    .position(|a| a.id == id)
                    .map_or(0, |pos| pos + 1)
            })
        });

        let limit = params.limit.min(100);
        let items: Vec<Activity> = activities
            .iter()
            .skip(start_index)
            .take(limit)
            .cloned()
            .collect();

        let activities_len = activities.len();
        let has_more = start_index + items.len() < activities_len;

        // Create next cursor using the last item's timestamp and ID
        let next_cursor = if has_more && !items.is_empty() {
            let last_item = &items[items.len() - 1];
            Some(Cursor::new(last_item.start_date, &last_item.id))
        } else {
            None
        };

        Ok(CursorPage::new(items, next_cursor, None, has_more))
    }

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        let user_id = self.get_user_id().await?;

        self.cache.get_activity(&user_id, id).await.ok_or_else(|| {
            ProviderError::NotFound {
                provider: "terra".to_owned(),
                resource_type: "activity".to_owned(),
                resource_id: id.to_owned(),
            }
            .into()
        })
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        let user_id = self.get_user_id().await?;

        // Calculate stats from cached activities
        let activities = self.cache.get_activities(&user_id, None, None).await;

        let total_activities = activities.len() as u64;
        let total_distance: f64 = activities.iter().filter_map(|a| a.distance_meters).sum();
        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();
        let total_elevation_gain: f64 = activities.iter().filter_map(|a| a.elevation_gain).sum();

        Ok(Stats {
            total_activities,
            total_distance,
            total_duration,
            total_elevation_gain,
        })
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Terra doesn't provide personal records directly
        // Return empty vec for now - could calculate from activities if needed
        Ok(Vec::new())
    }

    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        let user_id = self.get_user_id().await?;
        let sessions = self
            .cache
            .get_sleep_sessions(&user_id, start_date, end_date)
            .await;
        Ok(sessions)
    }

    async fn get_latest_sleep_session(&self) -> Result<SleepSession, ProviderError> {
        let user_id = self.get_user_id().await?;

        self.cache
            .get_latest_sleep_session(&user_id)
            .await
            .ok_or_else(|| ProviderError::NotFound {
                provider: "terra".to_owned(),
                resource_type: "sleep_session".to_owned(),
                resource_id: "latest".to_owned(),
            })
    }

    async fn get_recovery_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<RecoveryMetrics>, ProviderError> {
        let user_id = self.get_user_id().await?;
        let metrics = self
            .cache
            .get_recovery_metrics(&user_id, start_date, end_date)
            .await;
        Ok(metrics)
    }

    async fn get_health_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<HealthMetrics>, ProviderError> {
        let user_id = self.get_user_id().await?;
        let metrics = self
            .cache
            .get_health_metrics(&user_id, start_date, end_date)
            .await;
        Ok(metrics)
    }

    async fn disconnect(&self) -> AppResult<()> {
        let user_id = self.get_user_id().await?;

        // Deauthenticate via API if client is available
        if let Some(ref client) = self.api_client {
            client.deauthenticate_user(&user_id).await?;
        }

        // Clear credentials and user ID
        *self.credentials.write().await = None;
        *self.terra_user_id.write().await = None;

        Ok(())
    }
}

/// Terra provider descriptor for SPI
pub struct TerraDescriptor;

impl ProviderDescriptor for TerraDescriptor {
    fn name(&self) -> &'static str {
        "terra"
    }

    fn display_name(&self) -> &'static str {
        "Terra (150+ Wearables)"
    }

    fn capabilities(&self) -> ProviderCapabilities {
        // Terra supports all data types through its unified API
        ProviderCapabilities::full_health()
    }

    fn oauth_endpoints(&self) -> Option<OAuthEndpoints> {
        // Terra uses API key auth + widget sessions, not traditional OAuth
        // Returning endpoints for widget session generation
        Some(OAuthEndpoints {
            auth_url: TERRA_WIDGET_SESSION_URL,
            token_url: TERRA_TOKEN_URL,
            revoke_url: Some(TERRA_DEAUTH_URL),
        })
    }

    fn oauth_params(&self) -> Option<OAuthParams> {
        Some(OAuthParams {
            scope_separator: ",",
            use_pkce: false, // Terra uses API keys
            additional_auth_params: &[],
        })
    }

    fn api_base_url(&self) -> &'static str {
        TERRA_API_BASE_URL
    }

    fn default_scopes(&self) -> &'static [&'static str] {
        &["activity", "sleep", "body", "daily", "nutrition"]
    }
}

/// Factory for creating Terra provider instances
pub struct TerraProviderFactory {
    cache: Arc<TerraDataCache>,
}

impl TerraProviderFactory {
    /// Create a new factory with a shared cache
    #[must_use]
    pub const fn new(cache: Arc<TerraDataCache>) -> Self {
        Self { cache }
    }
}

impl ProviderFactory for TerraProviderFactory {
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider> {
        Box::new(TerraProvider::with_config(config, Arc::clone(&self.cache)))
    }

    fn supported_providers(&self) -> &'static [&'static str] {
        &["terra"]
    }
}
