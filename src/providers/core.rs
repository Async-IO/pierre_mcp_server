// ABOUTME: Core provider traits and interfaces for unified fitness data access
// ABOUTME: Defines the foundational abstractions for all fitness data providers
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Pluggable Provider Architecture - Shared Request/Response Traits (Phase 1)
//!
//! This module defines the shared request/response contract that all fitness providers
//! must implement. The `FitnessProvider` trait serves as the unified interface for
//! accessing fitness data from multiple providers (Strava, Garmin, Fitbit, Synthetic).
//!
//! ## Shared Request/Response Pattern
//!
//! ### Request Side (Shared Method Parameters)
//!
//! All providers accept standardized request parameters:
//! - **IDs**: String identifiers for resources (`activity_id`, etc.)
//! - **Pagination**: `PaginationParams` for cursor-based or offset-based paging
//! - **Date Ranges**: `DateTime<Utc>` for time-based queries
//! - **Options**: `Option<T>` for optional filtering/limiting
//!
//! ### Response Side (Shared Domain Models)
//!
//! All providers return standardized domain models:
//! - **Activity**: Unified workout/activity representation
//! - **Athlete**: User profile information
//! - **Stats**: Aggregate performance statistics
//! - **PersonalRecord**: Best performance achievements
//! - **SleepSession**, **RecoveryMetrics**, **HealthMetrics**: Health data
//!
//! ### Error Handling (Shared Result Type)
//!
//! All providers use `AppResult<T>` for consistent error handling across:
//! - Authentication failures
//! - API rate limiting
//! - Network errors
//! - Data validation errors
//!
//! ## Architecture Benefits
//!
//! 1. **Provider Interchangeability**: Swap providers without changing application code
//! 2. **Type Safety**: Compile-time guarantees for request/response contracts
//! 3. **Extensibility**: Add new providers by implementing `FitnessProvider` trait
//! 4. **Consistency**: Uniform error handling and data models across all providers
//!
//! ## Example: Adding a New Provider
//!
//! ```rust,no_run
//! use pierre_mcp_server::providers::core::{FitnessProvider, ProviderConfig, OAuth2Credentials};
//! use pierre_mcp_server::models::{Activity, Athlete, Stats};
//! use pierre_mcp_server::errors::AppResult;
//! use async_trait::async_trait;
//!
//! // Step 1: Define provider struct
//! pub struct CustomProvider {
//!     config: ProviderConfig,
//!     // ... provider-specific fields (e.g., HTTP client, tokens)
//! }
//!
//! // Step 2: Implement shared FitnessProvider trait
//! #[async_trait]
//! impl FitnessProvider for CustomProvider {
//!     fn name(&self) -> &'static str {
//!         "custom"
//!     }
//!
//!     fn config(&self) -> &ProviderConfig {
//!         &self.config
//!     }
//!
//!     async fn set_credentials(&self, _credentials: OAuth2Credentials) -> AppResult<()> {
//!         // Provider-specific OAuth handling (store tokens, configure client)
//!         Ok(())
//!     }
//!
//!     async fn get_athlete(&self) -> AppResult<Athlete> {
//!         // Fetch from provider API and convert to shared Athlete model
//!         Ok(Athlete {
//!             id: "12345".to_owned(),
//!             username: "athlete".to_owned(),
//!             firstname: Some("John".to_owned()),
//!             lastname: Some("Doe".to_owned()),
//!             profile_picture: None,
//!             provider: "custom".to_owned(),
//!         })
//!     }
//!
//!     async fn get_activities_with_params(
//!         &self,
//!         _params: &pierre_mcp_server::providers::core::ActivityQueryParams,
//!     ) -> AppResult<Vec<Activity>> {
//!         // Fetch from provider API and map to shared Activity models
//!         Ok(vec![])
//!     }
//!
//!     // ... implement remaining trait methods
//! #   async fn is_authenticated(&self) -> bool { true }
//! #   async fn refresh_token_if_needed(&self) -> AppResult<()> { Ok(()) }
//! #   async fn get_activities_cursor(&self, _params: &pierre_mcp_server::pagination::PaginationParams) -> AppResult<pierre_mcp_server::pagination::CursorPage<Activity>> {
//! #       Ok(pierre_mcp_server::pagination::CursorPage::new(vec![], None, None, false))
//! #   }
//! #   async fn get_activity(&self, _id: &str) -> AppResult<Activity> {
//! #       Err(pierre_mcp_server::errors::AppError::not_found("Activity not found"))
//! #   }
//! #   async fn get_stats(&self) -> AppResult<Stats> {
//! #       Ok(Stats { total_activities: 0, total_distance: 0.0, total_duration: 0, total_elevation_gain: 0.0 })
//! #   }
//! #   async fn get_personal_records(&self) -> AppResult<Vec<pierre_mcp_server::models::PersonalRecord>> {
//! #       Ok(vec![])
//! #   }
//! #   async fn disconnect(&self) -> AppResult<()> { Ok(()) }
//! }
//! ```
//!
//! ## Provider-Specific Details vs Shared Interface
//!
//! - **Internal**: Providers use custom DTOs (e.g., `StravaActivityResponse`)
//! - **External**: Providers expose shared models (e.g., `Activity`)
//! - **Conversion**: Providers implement mapping logic internally
//!
//! This separation allows providers to adapt their specific API formats while
//! maintaining a consistent interface for the rest of the application.

use crate::errors::AppResult;
use crate::models::{
    Activity, Athlete, HealthMetrics, PersonalRecord, RecoveryMetrics, SleepSession, Stats,
};
use crate::pagination::{CursorPage, PaginationParams};
use crate::providers::errors::ProviderError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

/// Authentication credentials for `OAuth2` providers (Shared Request Type)
///
/// This struct serves as the standardized authentication request/response format
/// across all fitness providers. All providers accept and return credentials in
/// this unified format, regardless of their internal OAuth implementation details.
///
/// # Usage Pattern
///
/// Providers receive credentials via `set_credentials()` and use them internally
/// for API authentication. The credential lifecycle is managed by the auth layer,
/// which handles token refresh and expiration automatically.
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

/// Provider configuration containing all necessary endpoints and settings (Shared Request Type)
///
/// This struct defines the standardized configuration format for all fitness providers.
/// Configurations can be loaded from environment variables (via `load_provider_env_config`)
/// or provided directly when creating provider instances.
///
/// # Configuration Sources
///
/// 1. **Environment Variables**: `PIERRE_<PROVIDER>_*` (recommended for cloud deployment)
/// 2. **Direct Instantiation**: Programmatically set for testing/advanced scenarios
/// 3. **Registry Defaults**: Loaded automatically by `ProviderRegistry::new()`
///
/// # Example
///
/// ```rust
/// use pierre_mcp_server::providers::core::ProviderConfig;
///
/// let config = ProviderConfig {
///     name: "strava".to_owned(),
///     auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
///     token_url: "https://www.strava.com/oauth/token".to_owned(),
///     api_base_url: "https://www.strava.com/api/v3".to_owned(),
///     revoke_url: Some("https://www.strava.com/oauth/deauthorize".to_owned()),
///     default_scopes: vec!["activity:read_all".to_owned()],
///};
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name (e.g., "strava", "fitbit", "garmin", "synthetic")
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

/// Query parameters for fetching activities with time-based filtering
///
/// This struct provides flexible activity querying with optional pagination
/// and timestamp-based filtering. When `before` and `after` are specified,
/// they enable efficient date range queries without fetching all activities.
///
/// # Strava API Mapping
///
/// - `before`: Maps to Strava's `before` parameter (activities before this epoch timestamp)
/// - `after`: Maps to Strava's `after` parameter (activities after this epoch timestamp)
/// - `limit`: Maps to Strava's `per_page` parameter
/// - `offset`: Converted to page number for Strava's pagination
#[derive(Debug, Clone, Default)]
pub struct ActivityQueryParams {
    /// Maximum number of activities to return
    pub limit: Option<usize>,
    /// Number of activities to skip (for offset-based pagination)
    pub offset: Option<usize>,
    /// Unix timestamp (seconds) - return activities before this time
    pub before: Option<i64>,
    /// Unix timestamp (seconds) - return activities after this time
    pub after: Option<i64>,
}

impl ActivityQueryParams {
    /// Create new query params with just limit and offset (backward compatible)
    #[must_use]
    pub const fn with_pagination(limit: Option<usize>, offset: Option<usize>) -> Self {
        Self {
            limit,
            offset,
            before: None,
            after: None,
        }
    }

    /// Create new query params with timestamp filtering
    #[must_use]
    pub const fn with_time_range(before: Option<i64>, after: Option<i64>) -> Self {
        Self {
            limit: None,
            offset: None,
            before,
            after,
        }
    }
}

/// Core fitness data provider trait - Shared Request/Response Interface for all providers
///
/// This trait defines the complete contract for fitness data providers. All providers
/// (Strava, Garmin, Fitbit, Synthetic) implement this trait, ensuring consistent
/// request/response patterns across the entire system.
///
/// # Shared Request/Response Contract
///
/// - **Request Parameters**: Standardized method signatures (IDs, pagination, dates)
/// - **Response Types**: Unified domain models (Activity, Athlete, Stats, etc.)
/// - **Error Handling**: Consistent `AppResult<T>` for all operations
/// - **Authentication**: Uniform `OAuth2Credentials` flow
///
/// # Provider Responsibilities
///
/// Implementors must:
/// 1. Convert provider-specific API responses to shared domain models
/// 2. Handle provider-specific authentication flows
/// 3. Implement rate limiting and retry logic
/// 4. Map provider errors to standard `AppError` types
///
/// # Thread Safety
///
/// All implementations must be `Send + Sync` for concurrent access across async tasks.
#[async_trait]
pub trait FitnessProvider: Send + Sync {
    /// Get provider name (e.g., "strava", "fitbit", "garmin", "synthetic")
    fn name(&self) -> &'static str;

    /// Get provider configuration (endpoints, scopes, etc.)
    fn config(&self) -> &ProviderConfig;

    /// Set `OAuth2` credentials for this provider
    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()>;

    /// Check if provider has valid authentication
    async fn is_authenticated(&self) -> bool;

    /// Refresh access token if needed
    async fn refresh_token_if_needed(&self) -> AppResult<()>;

    /// Get user's athlete profile
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::core::FitnessProvider;
    /// # async fn example(provider: &impl FitnessProvider) -> Result<(), pierre_mcp_server::errors::AppError> {
    /// let athlete = provider.get_athlete().await?;
    /// println!("Athlete: {} (ID: {})", athlete.username, athlete.id);
    /// if let Some(first) = &athlete.firstname {
    ///     println!("Name: {}", first);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn get_athlete(&self) -> AppResult<Athlete>;

    /// Get user's activities with offset-based pagination (legacy)
    ///
    /// For time-based filtering, use `get_activities_with_params` instead.
    async fn get_activities(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> AppResult<Vec<Activity>> {
        self.get_activities_with_params(&ActivityQueryParams::with_pagination(limit, offset))
            .await
    }

    /// Get user's activities with full query parameters including time filtering
    ///
    /// This method supports:
    /// - `limit`: Maximum number of activities to return
    /// - `offset`: Skip this many activities (for offset-based pagination)
    /// - `before`: Unix timestamp - return activities before this time
    /// - `after`: Unix timestamp - return activities after this time
    ///
    /// For Strava, `before` and `after` map directly to API parameters, enabling
    /// efficient date range queries without fetching all activities first.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::core::{FitnessProvider, ActivityQueryParams};
    /// # use chrono::{Duration, Utc};
    /// # async fn example(provider: &impl FitnessProvider) -> Result<(), pierre_mcp_server::errors::AppError> {
    /// // Get last 10 activities
    /// let params = ActivityQueryParams::with_pagination(Some(10), None);
    /// let activities = provider.get_activities_with_params(&params).await?;
    ///
    /// // Get activities from the last 30 days
    /// let thirty_days_ago = (Utc::now() - Duration::days(30)).timestamp();
    /// let time_params = ActivityQueryParams::with_time_range(None, Some(thirty_days_ago));
    /// let recent = provider.get_activities_with_params(&time_params).await?;
    ///
    /// for activity in &recent {
    ///     println!("{}: {:.1} km", activity.name(), activity.distance_meters().unwrap_or(0.0) / 1000.0);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>>;

    /// Get user's activities with cursor-based pagination (recommended)
    ///
    /// This method provides efficient, consistent pagination using opaque cursors.
    /// Cursors prevent duplicates and missing items when data changes during pagination.
    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>>;

    /// Get specific activity by ID
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::core::FitnessProvider;
    /// # async fn example(provider: &impl FitnessProvider) -> Result<(), pierre_mcp_server::errors::AppError> {
    /// let activity = provider.get_activity("12345678").await?;
    /// println!("Activity: {}", activity.name());
    /// println!("Type: {:?}", activity.sport_type());
    /// if let Some(distance) = activity.distance_meters() {
    ///     println!("Distance: {:.2} km", distance / 1000.0);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn get_activity(&self, id: &str) -> AppResult<Activity>;

    /// Get user's aggregate statistics
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::providers::core::FitnessProvider;
    /// # async fn example(provider: &impl FitnessProvider) -> Result<(), pierre_mcp_server::errors::AppError> {
    /// let stats = provider.get_stats().await?;
    /// println!("Total activities: {}", stats.total_activities);
    /// println!("Total distance: {:.1} km", stats.total_distance / 1000.0);
    /// println!("Total duration: {} hours", stats.total_duration / 3600);
    /// println!("Total elevation: {:.0} m", stats.total_elevation_gain);
    /// # Ok(())
    /// # }
    /// ```
    async fn get_stats(&self) -> AppResult<Stats>;

    /// Get user's personal records
    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>>;

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
    async fn disconnect(&self) -> AppResult<()>;
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

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        // Add tenant-specific logging/metrics here
        info!(
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

    async fn refresh_token_if_needed(&self) -> AppResult<()> {
        self.inner.refresh_token_if_needed().await
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        self.inner.get_athlete().await
    }

    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>> {
        self.inner.get_activities_with_params(params).await
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        self.inner.get_activities_cursor(params).await
    }

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        self.inner.get_activity(id).await
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        self.inner.get_stats().await
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        self.inner.get_personal_records().await
    }

    async fn disconnect(&self) -> AppResult<()> {
        self.inner.disconnect().await
    }
}
