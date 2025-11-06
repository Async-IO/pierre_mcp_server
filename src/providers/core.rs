// ABOUTME: Core provider traits and interfaces for unified fitness data access
// ABOUTME: Defines the foundational abstractions for all fitness data providers
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::models::{
    Activity, Athlete, HealthMetrics, PersonalRecord, RecoveryMetrics, SleepSession, Stats,
};
use crate::pagination::{CursorPage, PaginationParams};
use crate::providers::errors::ProviderError;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Authentication credentials for `OAuth2` providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Credentials {
    /// OAuth client ID from provider
    pub client_id: String,
    /// OAuth client secret from provider
    pub client_secret: String,
    /// Current access token
    pub access_token: Option<String>,
    /// Refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
    /// When the access token expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Granted OAuth scopes
    pub scopes: Vec<String>,
}

/// Provider configuration containing all necessary endpoints and settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "strava", "fitbit")
    pub name: String,
    /// OAuth authorization endpoint URL
    pub auth_url: String,
    /// OAuth token endpoint URL
    pub token_url: String,
    /// Base URL for provider API calls
    pub api_base_url: String,
    /// Optional token revocation endpoint URL
    pub revoke_url: Option<String>,
    /// Default OAuth scopes to request
    pub default_scopes: Vec<String>,
}

/// Core fitness data provider trait - single interface for all providers
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    /// Get provider name (e.g., "strava", "fitbit")
    fn name(&self) -> &'static str;

    /// Get provider configuration
    fn config(&self) -> &ProviderConfig;

    /// Set OAuth2 credentials for this provider
    async fn set_credentials(&self, credentials: OAuth2Credentials) -> Result<()>;

    /// Check if provider has valid authentication
    async fn is_authenticated(&self) -> bool;

    /// Refresh access token if needed
    async fn refresh_token_if_needed(&self) -> Result<()>;

    /// Get user's athlete profile
    async fn get_athlete(&self) -> Result<Athlete>;

    /// Get user's activities with offset-based pagination (legacy)
    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>>;

    /// Get user's activities with cursor-based pagination (recommended)
    ///
    /// This method provides efficient, consistent pagination using opaque cursors.
    /// Cursors prevent duplicates and missing items when data changes during pagination.
    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> Result<CursorPage<Activity>>;

    /// Get specific activity by ID
    async fn get_activity(&self, id: &str) -> Result<Activity>;

    /// Get user's aggregate statistics
    async fn get_stats(&self) -> Result<Stats>;

    /// Get user's personal records
    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>>;

    /// Get sleep sessions for a date range
    ///
    /// Returns sleep data from providers that support sleep tracking (Fitbit, Garmin).
    /// Providers without sleep data support return `UnsupportedFeature` error.
    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        let date_range = format!(
            "{} to {}",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );
        Err(ProviderError::UnsupportedFeature {
            provider: self.name().to_owned(),
            feature: format!("sleep_sessions (requested: {date_range})"),
        })
    }

    /// Get the most recent sleep session
    ///
    /// Convenience method for providers that support sleep tracking.
    /// Returns `UnsupportedFeature` for providers without sleep data.
    async fn get_latest_sleep_session(&self) -> Result<SleepSession, ProviderError> {
        Err(ProviderError::UnsupportedFeature {
            provider: self.name().to_owned(),
            feature: "latest_sleep_session".to_owned(),
        })
    }

    /// Get recovery and readiness metrics for a date range
    ///
    /// Returns daily recovery scores, HRV status, and training readiness.
    /// Available from providers with recovery tracking (Fitbit, Garmin, Whoop).
    async fn get_recovery_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<RecoveryMetrics>, ProviderError> {
        let date_range = format!(
            "{} to {}",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );
        Err(ProviderError::UnsupportedFeature {
            provider: self.name().to_owned(),
            feature: format!("recovery_metrics (requested: {date_range})"),
        })
    }

    /// Get health metrics for a date range
    ///
    /// Returns comprehensive health data including weight, body composition, vital signs.
    /// Supported by health-focused providers (Fitbit, Garmin, Apple Health).
    async fn get_health_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<HealthMetrics>, ProviderError> {
        let date_range = format!(
            "{} to {}",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );
        Err(ProviderError::UnsupportedFeature {
            provider: self.name().to_owned(),
            feature: format!("health_metrics (requested: {date_range})"),
        })
    }

    /// Revoke access tokens (disconnect)
    async fn disconnect(&self) -> Result<()>;
}

/// Provider factory for creating instances
pub trait ProviderFactory: Send + Sync {
    /// Create a new provider instance with the given configuration
    fn create(&self, config: ProviderConfig) -> Box<dyn FitnessProvider>;

    /// Get supported provider names
    fn supported_providers(&self) -> &'static [&'static str];
}

/// Tenant-aware provider wrapper that handles multi-tenancy
pub struct TenantProvider {
    inner: Box<dyn FitnessProvider>,
    tenant_id: Uuid,
    user_id: Uuid,
}

impl TenantProvider {
    /// Create a new tenant-aware provider
    #[must_use]
    pub fn new(inner: Box<dyn FitnessProvider>, tenant_id: Uuid, user_id: Uuid) -> Self {
        Self {
            inner,
            tenant_id,
            user_id,
        }
    }

    /// Get tenant ID
    #[must_use]
    pub const fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    /// Get user ID
    #[must_use]
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }
}

#[async_trait]
impl FitnessProvider for TenantProvider {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn config(&self) -> &ProviderConfig {
        self.inner.config()
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> Result<()> {
        // Add tenant-specific logging/metrics here
        tracing::info!(
            "Setting credentials for provider {} in tenant {} for user {}",
            self.name(),
            self.tenant_id,
            self.user_id
        );
        self.inner.set_credentials(credentials).await
    }

    async fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated().await
    }

    async fn refresh_token_if_needed(&self) -> Result<()> {
        self.inner.refresh_token_if_needed().await
    }

    async fn get_athlete(&self) -> Result<Athlete> {
        self.inner.get_athlete().await
    }

    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Activity>> {
        self.inner.get_activities(limit, offset).await
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> Result<CursorPage<Activity>> {
        self.inner.get_activities_cursor(params).await
    }

    async fn get_activity(&self, id: &str) -> Result<Activity> {
        self.inner.get_activity(id).await
    }

    async fn get_stats(&self) -> Result<Stats> {
        self.inner.get_stats().await
    }

    async fn get_personal_records(&self) -> Result<Vec<PersonalRecord>> {
        self.inner.get_personal_records().await
    }

    async fn disconnect(&self) -> Result<()> {
        self.inner.disconnect().await
    }
}
