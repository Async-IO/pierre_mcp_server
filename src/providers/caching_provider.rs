// ABOUTME: Caching decorator for FitnessProvider that adds transparent cache-aside caching
// ABOUTME: Supports Redis/in-memory backends via pluggable CacheProvider trait
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Caching Fitness Provider
//!
//! This module provides a caching decorator that wraps any `FitnessProvider` implementation
//! and adds transparent caching using the cache-aside pattern. It reduces API calls to
//! external providers by caching responses with configurable TTLs.
//!
//! ## Cache-Aside Pattern
//!
//! 1. Check cache for requested data
//! 2. If cache hit: return cached data
//! 3. If cache miss: fetch from provider, store in cache, return data
//!
//! ## Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::providers::caching_provider::{CachingFitnessProvider, CachePolicy};
//! use pierre_mcp_server::providers::create_provider;
//! use pierre_mcp_server::providers::core::FitnessProvider;
//! use pierre_mcp_server::cache::{CacheConfig, CacheProvider};
//! use pierre_mcp_server::cache::memory::InMemoryCache;
//! use pierre_mcp_server::models::TenantId;
//! use uuid::Uuid;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create the underlying provider
//! let provider = create_provider("strava")?;
//!
//! // Create cache backend
//! let cache_config = CacheConfig::default();
//! let cache = InMemoryCache::new(cache_config).await?;
//!
//! // Wrap with caching
//! let cached_provider = CachingFitnessProvider::new(
//!     provider,
//!     cache,
//!     TenantId::new(), // tenant_id
//!     Uuid::new_v4(), // user_id
//! );
//!
//! // Use normally - caching is transparent
//! let activities = cached_provider.get_activities(Some(10), None).await?;
//!
//! // Or explicitly control cache behavior
//! let fresh = cached_provider
//!     .get_activities_with_policy(Some(10), None, CachePolicy::Bypass)
//!     .await?;
//! # Ok(())
//! # }
//! ```

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pierre_core::models::TenantId;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};
use uuid::Uuid;

use crate::cache::memory::InMemoryCache;
use crate::cache::{CacheConfig, CacheKey, CacheProvider, CacheResource, CacheTtlConfig};
use crate::errors::AppResult;
use crate::models::{
    Activity, Athlete, HealthMetrics, PersonalRecord, RecoveryMetrics, SleepSession, Stats,
};
use crate::pagination::{CursorPage, PaginationParams};
use crate::providers::core::{
    ActivityQueryParams, FitnessProvider, OAuth2Credentials, ProviderConfig,
};
use crate::providers::errors::ProviderError;

/// Cache policy for controlling caching behavior per-request
///
/// This enum allows callers to explicitly control whether caching should be used
/// for a specific request, overriding the default cache-aside behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CachePolicy {
    /// Use cache if available, fetch and cache on miss (default behavior)
    #[default]
    UseCache,
    /// Bypass cache entirely, always fetch fresh data (do not update cache)
    Bypass,
    /// Invalidate existing cache entry, fetch fresh data, and update cache
    Refresh,
}

/// Caching wrapper for any `FitnessProvider` implementation
///
/// This struct implements the decorator pattern to add transparent caching
/// to any fitness provider. It maintains tenant/user context for proper
/// cache key isolation in multi-tenant environments.
pub struct CachingFitnessProvider<C: CacheProvider> {
    /// The underlying provider being wrapped
    inner: Box<dyn FitnessProvider>,
    /// Cache backend (Redis or in-memory)
    cache: Arc<C>,
    /// Tenant ID for cache key isolation
    tenant_id: TenantId,
    /// User ID for cache key isolation
    user_id: Uuid,
    /// TTL configuration for different resource types
    ttl_config: CacheTtlConfig,
}

impl<C: CacheProvider> CachingFitnessProvider<C> {
    /// Create a new caching provider wrapper
    ///
    /// # Arguments
    ///
    /// * `inner` - The underlying fitness provider to wrap
    /// * `cache` - The cache backend to use (implements `CacheProvider`)
    /// * `tenant_id` - Tenant ID for multi-tenant cache isolation
    /// * `user_id` - User ID for per-user cache isolation
    pub fn new(
        inner: Box<dyn FitnessProvider>,
        cache: C,
        tenant_id: TenantId,
        user_id: Uuid,
    ) -> Self {
        Self {
            inner,
            cache: Arc::new(cache),
            tenant_id,
            user_id,
            ttl_config: CacheTtlConfig::default(),
        }
    }

    /// Create a new caching provider with custom TTL configuration
    pub fn with_ttl_config(
        inner: Box<dyn FitnessProvider>,
        cache: C,
        tenant_id: TenantId,
        user_id: Uuid,
        ttl_config: CacheTtlConfig,
    ) -> Self {
        Self {
            inner,
            cache: Arc::new(cache),
            tenant_id,
            user_id,
            ttl_config,
        }
    }

    /// Create a new caching provider from an existing Arc<C> cache
    ///
    /// This is useful when sharing a cache instance across multiple providers.
    pub fn with_shared_cache(
        inner: Box<dyn FitnessProvider>,
        cache: Arc<C>,
        tenant_id: TenantId,
        user_id: Uuid,
    ) -> Self {
        Self {
            inner,
            cache,
            tenant_id,
            user_id,
            ttl_config: CacheTtlConfig::default(),
        }
    }

    /// Get the tenant ID
    #[must_use]
    pub const fn tenant_id(&self) -> TenantId {
        self.tenant_id
    }

    /// Get the user ID
    #[must_use]
    pub const fn user_id(&self) -> Uuid {
        self.user_id
    }

    /// Get a reference to the underlying provider
    #[must_use]
    pub fn inner(&self) -> &dyn FitnessProvider {
        self.inner.as_ref()
    }

    /// Get a reference to the cache
    #[must_use]
    pub fn cache(&self) -> &C {
        &self.cache
    }

    /// Build a cache key for the given resource
    fn cache_key(&self, resource: CacheResource) -> CacheKey {
        CacheKey::new(
            self.tenant_id,
            self.user_id,
            self.inner.name().to_owned(),
            resource,
        )
    }

    /// Try to get a value from cache, returning None on miss or error
    async fn try_get_cached<T>(&self, key: &CacheKey) -> Option<T>
    where
        T: for<'de> Deserialize<'de> + Send + Sync,
    {
        match self.cache.get::<T>(key).await {
            Ok(Some(cached)) => {
                debug!(
                    target: "pierre::cache",
                    cache_hit = true,
                    key = %key,
                    provider = self.inner.name(),
                    "Cache hit"
                );
                Some(cached)
            }
            Ok(None) => {
                debug!(
                    target: "pierre::cache",
                    cache_hit = false,
                    key = %key,
                    provider = self.inner.name(),
                    "Cache miss"
                );
                None
            }
            Err(e) => {
                warn!(
                    target: "pierre::cache",
                    error = %e,
                    key = %key,
                    "Cache read error, falling back to provider"
                );
                None
            }
        }
    }

    /// Store a value in cache (best effort, logs on failure)
    async fn store_in_cache<T>(&self, key: &CacheKey, data: &T, ttl: Duration)
    where
        T: Serialize + Send + Sync,
    {
        if let Err(e) = self.cache.set(key, data, ttl).await {
            warn!(
                target: "pierre::cache",
                error = %e,
                key = %key,
                "Failed to cache response"
            );
        }
    }

    /// Handle cache bypass policy
    async fn handle_bypass<T, F, Fut>(&self, key: &CacheKey, fetch_fn: F) -> AppResult<T>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = AppResult<T>>,
    {
        debug!(
            target: "pierre::cache",
            key = %key,
            provider = self.inner.name(),
            "Cache bypass requested"
        );
        fetch_fn().await
    }

    /// Handle cache refresh policy
    async fn handle_refresh<T, F, Fut>(
        &self,
        key: &CacheKey,
        ttl: Duration,
        fetch_fn: F,
    ) -> AppResult<T>
    where
        T: Serialize + Send + Sync,
        F: FnOnce() -> Fut,
        Fut: Future<Output = AppResult<T>>,
    {
        // Invalidate existing cache entry
        if let Err(e) = self.cache.invalidate(key).await {
            warn!(
                target: "pierre::cache",
                error = %e,
                key = %key,
                "Failed to invalidate cache entry"
            );
        }

        // Fetch fresh data
        let data = fetch_fn().await?;

        // Store in cache
        self.store_in_cache(key, &data, ttl).await;

        info!(
            target: "pierre::cache",
            key = %key,
            provider = self.inner.name(),
            "Cache refreshed"
        );

        Ok(data)
    }

    /// Get cached value or fetch from provider
    ///
    /// Implements the cache-aside pattern with policy support.
    async fn get_or_fetch<T, F, Fut>(
        &self,
        key: &CacheKey,
        policy: CachePolicy,
        fetch_fn: F,
    ) -> AppResult<T>
    where
        T: Serialize + for<'de> Deserialize<'de> + Send + Sync,
        F: FnOnce() -> Fut,
        Fut: Future<Output = AppResult<T>>,
    {
        let ttl = self.ttl_config.ttl_for_resource(&key.resource);

        match policy {
            CachePolicy::UseCache => {
                // Try cache first
                if let Some(cached) = self.try_get_cached(key).await {
                    return Ok(cached);
                }

                // Fetch from provider
                let data = fetch_fn().await?;

                // Store in cache (best effort)
                self.store_in_cache(key, &data, ttl).await;

                Ok(data)
            }
            CachePolicy::Bypass => self.handle_bypass(key, fetch_fn).await,
            CachePolicy::Refresh => self.handle_refresh(key, ttl, fetch_fn).await,
        }
    }

    // =========================================================================
    // Public methods with explicit cache policy control
    // =========================================================================

    /// Get athlete profile with explicit cache policy
    ///
    /// # Errors
    ///
    /// Returns an error if the provider API call fails or cache operations fail.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id))]
    pub async fn get_athlete_with_policy(&self, policy: CachePolicy) -> AppResult<Athlete> {
        let key = self.cache_key(CacheResource::AthleteProfile);
        self.get_or_fetch(&key, policy, || self.inner.get_athlete())
            .await
    }

    /// Get activities with explicit cache policy
    ///
    /// # Errors
    ///
    /// Returns an error if the provider API call fails or cache operations fail.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id))]
    pub async fn get_activities_with_policy(
        &self,
        limit: Option<usize>,
        offset: Option<usize>,
        policy: CachePolicy,
    ) -> AppResult<Vec<Activity>> {
        let params = ActivityQueryParams::with_pagination(limit, offset);
        self.get_activities_with_params_and_policy(&params, policy)
            .await
    }

    /// Get activities with full query parameters and explicit cache policy
    ///
    /// # Errors
    ///
    /// Returns an error if the provider API call fails or cache operations fail.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id))]
    pub async fn get_activities_with_params_and_policy(
        &self,
        params: &ActivityQueryParams,
        policy: CachePolicy,
    ) -> AppResult<Vec<Activity>> {
        // Calculate page and per_page from limit/offset for cache key
        // Clamp per_page to at least 1 to prevent division by zero
        let per_page = params.limit.unwrap_or(50).max(1);
        let page = params.offset.map_or(1, |off| (off / per_page) + 1);

        let key = self.cache_key(CacheResource::ActivityList {
            page: u32::try_from(page).unwrap_or(1),
            per_page: u32::try_from(per_page).unwrap_or(50),
            before: params.before,
            after: params.after,
            sport_type: None,
        });

        // Clone params for the closure
        let params_clone = params.clone();
        self.get_or_fetch(&key, policy, || {
            self.inner.get_activities_with_params(&params_clone)
        })
        .await
    }

    /// Get a single activity by ID with explicit cache policy
    ///
    /// # Errors
    ///
    /// Returns an error if the provider API call fails or cache operations fail.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id, activity_id = %id))]
    pub async fn get_activity_with_policy(
        &self,
        id: &str,
        policy: CachePolicy,
    ) -> AppResult<Activity> {
        let activity_id = id.parse::<u64>().unwrap_or(0);
        let key = self.cache_key(CacheResource::Activity { activity_id });

        let id_owned = id.to_owned();
        self.get_or_fetch(&key, policy, || self.inner.get_activity(&id_owned))
            .await
    }

    /// Get stats with explicit cache policy
    ///
    /// # Errors
    ///
    /// Returns an error if the provider API call fails or cache operations fail.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id))]
    #[allow(clippy::cast_possible_truncation)]
    pub async fn get_stats_with_policy(&self, policy: CachePolicy) -> AppResult<Stats> {
        // Use lower 64 bits of user_id as athlete_id for stats cache key
        // Truncation is intentional: we only need a unique key, not the full UUID
        let athlete_id = self.user_id.as_u128() as u64;
        let key = self.cache_key(CacheResource::Stats { athlete_id });

        self.get_or_fetch(&key, policy, || self.inner.get_stats())
            .await
    }

    /// Invalidate all cache entries for this user
    ///
    /// Use this when the user disconnects from the provider or when
    /// a significant data change is detected (e.g., via webhook).
    ///
    /// # Errors
    ///
    /// Returns an error if the cache invalidation operation fails.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id))]
    pub async fn invalidate_user_cache(&self) -> AppResult<u64> {
        let pattern = CacheKey::user_pattern(self.tenant_id, self.user_id, self.inner.name());
        let count = self.cache.invalidate_pattern(&pattern).await?;
        info!(
            target: "pierre::cache",
            pattern = %pattern,
            invalidated_count = count,
            "User cache invalidated"
        );
        Ok(count)
    }

    /// Invalidate activity list cache entries for this user
    ///
    /// Use this when new activities are detected (e.g., via webhook).
    ///
    /// # Errors
    ///
    /// Returns an error if the cache invalidation operation fails.
    #[instrument(skip(self), fields(provider = self.inner.name(), tenant_id = %self.tenant_id))]
    pub async fn invalidate_activity_list_cache(&self) -> AppResult<u64> {
        let pattern = format!(
            "tenant:{}:user:{}:provider:{}:activity_list:*",
            self.tenant_id,
            self.user_id,
            self.inner.name()
        );
        let count = self.cache.invalidate_pattern(&pattern).await?;
        info!(
            target: "pierre::cache",
            pattern = %pattern,
            invalidated_count = count,
            "Activity list cache invalidated"
        );
        Ok(count)
    }
}

// =============================================================================
// FitnessProvider trait implementation (default cache behavior)
// =============================================================================

#[async_trait]
impl<C: CacheProvider + 'static> FitnessProvider for CachingFitnessProvider<C> {
    fn name(&self) -> &'static str {
        self.inner.name()
    }

    fn config(&self) -> &ProviderConfig {
        self.inner.config()
    }

    async fn set_credentials(&self, credentials: OAuth2Credentials) -> AppResult<()> {
        self.inner.set_credentials(credentials).await
    }

    async fn is_authenticated(&self) -> bool {
        self.inner.is_authenticated().await
    }

    async fn refresh_token_if_needed(&self) -> AppResult<()> {
        self.inner.refresh_token_if_needed().await
    }

    async fn get_athlete(&self) -> AppResult<Athlete> {
        self.get_athlete_with_policy(CachePolicy::UseCache).await
    }

    async fn get_activities_with_params(
        &self,
        params: &ActivityQueryParams,
    ) -> AppResult<Vec<Activity>> {
        self.get_activities_with_params_and_policy(params, CachePolicy::UseCache)
            .await
    }

    async fn get_activities_cursor(
        &self,
        params: &PaginationParams,
    ) -> AppResult<CursorPage<Activity>> {
        // Cursor-based pagination is not cached because cursors are opaque
        // and may change between requests. Caching could return stale cursors.
        self.inner.get_activities_cursor(params).await
    }

    async fn get_activity(&self, id: &str) -> AppResult<Activity> {
        self.get_activity_with_policy(id, CachePolicy::UseCache)
            .await
    }

    async fn get_stats(&self) -> AppResult<Stats> {
        self.get_stats_with_policy(CachePolicy::UseCache).await
    }

    async fn get_personal_records(&self) -> AppResult<Vec<PersonalRecord>> {
        // Personal records change infrequently, but we don't have a dedicated
        // cache resource type for them. Use stats TTL as a reasonable default.
        // For now, pass through without caching - can be added later if needed.
        self.inner.get_personal_records().await
    }

    async fn get_sleep_sessions(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<SleepSession>, ProviderError> {
        // Sleep sessions are time-range based; caching requires careful key design.
        // Pass through for now - can be added with proper cache key strategy.
        self.inner.get_sleep_sessions(start_date, end_date).await
    }

    async fn get_latest_sleep_session(&self) -> Result<SleepSession, ProviderError> {
        // Latest sleep changes frequently; pass through without caching.
        self.inner.get_latest_sleep_session().await
    }

    async fn get_recovery_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<RecoveryMetrics>, ProviderError> {
        // Recovery metrics are time-range based; pass through for now.
        self.inner.get_recovery_metrics(start_date, end_date).await
    }

    async fn get_health_metrics(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<Vec<HealthMetrics>, ProviderError> {
        // Health metrics are time-range based; pass through for now.
        self.inner.get_health_metrics(start_date, end_date).await
    }

    async fn disconnect(&self) -> AppResult<()> {
        // Invalidate user cache on disconnect
        if let Err(e) = self.invalidate_user_cache().await {
            warn!(
                target: "pierre::cache",
                error = %e,
                "Failed to invalidate cache on disconnect"
            );
        }
        self.inner.disconnect().await
    }
}

// =============================================================================
// Factory function for creating cached providers
// =============================================================================

/// Create a caching provider from cache configuration
///
/// This is a convenience function that creates the appropriate cache backend
/// based on configuration (Redis if URL provided, otherwise in-memory).
///
/// # Errors
///
/// Returns an error if cache initialization fails.
pub async fn create_caching_provider(
    inner: Box<dyn FitnessProvider>,
    cache_config: CacheConfig,
    tenant_id: TenantId,
    user_id: Uuid,
) -> AppResult<CachingFitnessProvider<InMemoryCache>> {
    // For now, always use in-memory cache. Redis support can be added via
    // the factory pattern when Redis backend is needed.
    let cache = InMemoryCache::new(cache_config.clone()).await?;

    Ok(CachingFitnessProvider::with_ttl_config(
        inner,
        cache,
        tenant_id,
        user_id,
        cache_config.ttl,
    ))
}

/// Create a caching provider with explicit TTL configuration
///
/// This factory function allows specifying TTL configuration separately from
/// the cache config, enabling loading TTLs from admin configuration while
/// using the cache config for capacity and cleanup settings.
///
/// # Arguments
///
/// * `inner` - The underlying fitness provider to wrap
/// * `cache_config` - Cache configuration for capacity, cleanup, etc.
/// * `tenant_id` - Tenant ID for multi-tenant cache isolation
/// * `user_id` - User ID for per-user cache isolation
/// * `ttl_config` - TTL configuration (e.g., from admin config service)
///
/// # Errors
///
/// Returns an error if cache initialization fails.
pub async fn create_caching_provider_with_ttl(
    inner: Box<dyn FitnessProvider>,
    cache_config: CacheConfig,
    tenant_id: TenantId,
    user_id: Uuid,
    ttl_config: CacheTtlConfig,
) -> AppResult<CachingFitnessProvider<InMemoryCache>> {
    let cache = InMemoryCache::new(cache_config).await?;

    Ok(CachingFitnessProvider::with_ttl_config(
        inner, cache, tenant_id, user_id, ttl_config,
    ))
}
