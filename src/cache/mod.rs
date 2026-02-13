// ABOUTME: Cache abstraction layer for API response caching with tenant isolation
// ABOUTME: Pluggable backend support (in-memory, Redis) following DatabaseProvider pattern
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Cache factory for creating cache providers
pub mod factory;
/// In-memory cache implementation
pub mod memory;
/// Redis cache implementation
pub mod redis;

use crate::config::admin::service::AdminConfigService;
use crate::config::environment::RedisConnectionConfig;
use crate::constants::cache::{
    DEFAULT_CACHE_MAX_ENTRIES, DEFAULT_CLEANUP_INTERVAL_SECS, TTL_ACTIVITY_LIST_SECS,
    TTL_ACTIVITY_SECS, TTL_PROFILE_SECS, TTL_STATS_SECS,
};
use crate::errors::AppResult;
use pierre_core::models::TenantId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

/// Cache provider trait for pluggable backend implementations
///
/// # Examples
///
/// ```rust,no_run
/// use pierre_mcp_server::cache::{CacheConfig, CacheKey, CacheProvider, CacheResource};
/// use pierre_mcp_server::cache::memory::InMemoryCache;
/// use pierre_mcp_server::models::TenantId;
/// use serde::{Deserialize, Serialize};
/// use std::time::Duration;
/// use uuid::Uuid;
/// # async fn example() -> Result<(), pierre_mcp_server::errors::AppError> {
///
/// #[derive(Serialize, Deserialize)]
/// struct AthleteProfile {
///     name: String,
///     total_activities: u32,
/// }
///
/// // Create cache with default configuration
/// let config = CacheConfig {
///     enable_background_cleanup: false, // Disable for example
///     ..Default::default()
/// };
/// let cache: InMemoryCache = InMemoryCache::new(config).await?;
///
/// // Create a cache key for an athlete profile
/// let key = CacheKey {
///     tenant_id: TenantId::new(),
///     user_id: Uuid::new_v4(),
///     provider: "strava".to_owned(),
///     resource: CacheResource::AthleteProfile,
/// };
///
/// // Store data in cache
/// let profile = AthleteProfile {
///     name: "John Doe".to_owned(),
///     total_activities: 42,
/// };
/// cache.set(&key, &profile, Duration::from_secs(3600)).await?;
///
/// // Retrieve data from cache
/// let cached: Option<AthleteProfile> = cache.get(&key).await?;
/// if let Some(profile) = cached {
///     println!("Found cached profile: {}", profile.name);
/// }
///
/// // Invalidate cache entry
/// cache.invalidate(&key).await?;
/// # Ok(())
/// # }
/// ```
#[async_trait::async_trait]
pub trait CacheProvider: Send + Sync + Clone {
    /// Create new cache instance with configuration
    ///
    /// # Errors
    ///
    /// Returns an error if cache initialization fails
    async fn new(config: CacheConfig) -> AppResult<Self>
    where
        Self: Sized;

    /// Store value in cache with TTL
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or storage fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::cache::{CacheConfig, CacheKey, CacheProvider, CacheResource};
    /// # use pierre_mcp_server::cache::memory::InMemoryCache;
    /// # use pierre_mcp_server::models::TenantId;
    /// # use std::time::Duration;
    /// # use uuid::Uuid;
    /// # async fn example() -> Result<(), pierre_mcp_server::errors::AppError> {
    /// # let cache: InMemoryCache = InMemoryCache::new(CacheConfig { enable_background_cleanup: false, ..Default::default() }).await?;
    /// # let key = CacheKey { tenant_id: TenantId::new(), user_id: Uuid::new_v4(), provider: "strava".to_owned(), resource: CacheResource::AthleteProfile };
    /// // Store a string value with 1 hour TTL
    /// cache.set(&key, &"cached_value", Duration::from_secs(3600)).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &CacheKey,
        value: &T,
        ttl: Duration,
    ) -> AppResult<()>;

    /// Retrieve value from cache
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::cache::{CacheConfig, CacheKey, CacheProvider, CacheResource};
    /// # use pierre_mcp_server::cache::memory::InMemoryCache;
    /// # use pierre_mcp_server::models::TenantId;
    /// # use uuid::Uuid;
    /// # async fn example() -> Result<(), pierre_mcp_server::errors::AppError> {
    /// # let cache: InMemoryCache = InMemoryCache::new(CacheConfig { enable_background_cleanup: false, ..Default::default() }).await?;
    /// # let key = CacheKey { tenant_id: TenantId::new(), user_id: Uuid::new_v4(), provider: "strava".to_owned(), resource: CacheResource::AthleteProfile };
    /// // Retrieve a cached value (returns None if not found or expired)
    /// let value: Option<String> = cache.get(&key).await?;
    /// match value {
    ///     Some(data) => println!("Cache hit: {}", data),
    ///     None => println!("Cache miss"),
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> AppResult<Option<T>>;

    /// Remove single cache entry
    ///
    /// # Errors
    ///
    /// Returns an error if invalidation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use pierre_mcp_server::cache::{CacheConfig, CacheKey, CacheProvider, CacheResource};
    /// # use pierre_mcp_server::cache::memory::InMemoryCache;
    /// # use pierre_mcp_server::models::TenantId;
    /// # use uuid::Uuid;
    /// # async fn example() -> Result<(), pierre_mcp_server::errors::AppError> {
    /// # let cache: InMemoryCache = InMemoryCache::new(CacheConfig { enable_background_cleanup: false, ..Default::default() }).await?;
    /// # let key = CacheKey { tenant_id: TenantId::new(), user_id: Uuid::new_v4(), provider: "strava".to_owned(), resource: CacheResource::AthleteProfile };
    /// // Invalidate a specific cache entry (e.g., after user updates their profile)
    /// cache.invalidate(&key).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn invalidate(&self, key: &CacheKey) -> AppResult<()>;

    /// Remove all cache entries matching pattern (e.g., "tenant:*:strava:*")
    ///
    /// # Errors
    ///
    /// Returns an error if pattern invalidation fails
    async fn invalidate_pattern(&self, pattern: &str) -> AppResult<u64>;

    /// Check if key exists in cache
    ///
    /// # Errors
    ///
    /// Returns an error if existence check fails
    async fn exists(&self, key: &CacheKey) -> AppResult<bool>;

    /// Get remaining TTL for key
    ///
    /// # Errors
    ///
    /// Returns an error if TTL check fails
    async fn ttl(&self, key: &CacheKey) -> AppResult<Option<Duration>>;

    /// Verify cache backend is healthy
    ///
    /// # Errors
    ///
    /// Returns an error if health check fails
    async fn health_check(&self) -> AppResult<()>;

    /// Clear all cache entries (for testing/admin)
    ///
    /// # Errors
    ///
    /// Returns an error if clear operation fails
    async fn clear_all(&self) -> AppResult<()>;
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries (for in-memory cache)
    pub max_entries: usize,
    /// Redis connection URL (for Redis cache)
    pub redis_url: Option<String>,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
    /// Enable background cleanup task (should be false in tests to avoid runtime conflicts)
    pub enable_background_cleanup: bool,
    /// Redis connection and retry configuration
    pub redis_connection: RedisConnectionConfig,
    /// Cache TTL configuration
    pub ttl: CacheTtlConfig,
}

/// Cache TTL configuration for different resource types
#[derive(Debug, Clone)]
pub struct CacheTtlConfig {
    /// Athlete profile cache TTL in seconds (default: 24 hours)
    pub profile_secs: u64,
    /// Activity list cache TTL in seconds (default: 15 minutes)
    pub activity_list_secs: u64,
    /// Individual activity cache TTL in seconds (default: 1 hour)
    pub activity_secs: u64,
    /// Stats cache TTL in seconds (default: 6 hours)
    pub stats_secs: u64,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            profile_secs: TTL_PROFILE_SECS,
            activity_list_secs: TTL_ACTIVITY_LIST_SECS,
            activity_secs: TTL_ACTIVITY_SECS,
            stats_secs: TTL_STATS_SECS,
        }
    }
}

impl CacheTtlConfig {
    /// Get TTL duration for a specific cache resource type
    #[must_use]
    pub const fn ttl_for_resource(&self, resource: &CacheResource) -> Duration {
        match resource {
            CacheResource::AthleteProfile => Duration::from_secs(self.profile_secs),
            CacheResource::ActivityList { .. } => Duration::from_secs(self.activity_list_secs),
            CacheResource::Activity { .. } | CacheResource::DetailedActivity { .. } => {
                Duration::from_secs(self.activity_secs)
            }
            CacheResource::Stats { .. } => Duration::from_secs(self.stats_secs),
        }
    }

    /// Create TTL config from admin configuration service
    ///
    /// Loads TTL values from the admin config service, falling back to defaults
    /// if values are not configured or retrieval fails.
    ///
    /// # Arguments
    ///
    /// * `admin_config` - Reference to the admin configuration service
    /// * `tenant_id` - Optional tenant ID for tenant-specific configuration
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::cache::CacheTtlConfig;
    /// use pierre_mcp_server::config::admin::service::AdminConfigService;
    ///
    /// # async fn example(admin_config: &AdminConfigService) {
    /// let ttl_config = CacheTtlConfig::from_admin_config(admin_config, None).await;
    /// # }
    /// ```
    pub async fn from_admin_config(
        admin_config: &AdminConfigService,
        tenant_id: Option<&str>,
    ) -> Self {
        let defaults = Self::default();

        let profile_secs = Self::get_ttl_value(
            admin_config,
            "cache.profile_ttl_secs",
            tenant_id,
            defaults.profile_secs,
        )
        .await;

        let activity_list_secs = Self::get_ttl_value(
            admin_config,
            "cache.activity_list_ttl_secs",
            tenant_id,
            defaults.activity_list_secs,
        )
        .await;

        let activity_secs = Self::get_ttl_value(
            admin_config,
            "cache.activity_ttl_secs",
            tenant_id,
            defaults.activity_secs,
        )
        .await;

        let stats_secs = Self::get_ttl_value(
            admin_config,
            "cache.stats_ttl_secs",
            tenant_id,
            defaults.stats_secs,
        )
        .await;

        Self {
            profile_secs,
            activity_list_secs,
            activity_secs,
            stats_secs,
        }
    }

    /// Helper to get a TTL value from admin config with fallback
    async fn get_ttl_value(
        admin_config: &AdminConfigService,
        key: &str,
        tenant_id: Option<&str>,
        default: u64,
    ) -> u64 {
        match admin_config.get_value(key, tenant_id).await {
            Ok(Some(value)) => value.as_u64().unwrap_or(default),
            Ok(None) | Err(_) => default,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: DEFAULT_CACHE_MAX_ENTRIES,
            redis_url: None,
            cleanup_interval: Duration::from_secs(DEFAULT_CLEANUP_INTERVAL_SECS),
            // Default to enabled - production code should use background cleanup
            // Tests can explicitly disable by setting to false
            enable_background_cleanup: true,
            redis_connection: RedisConnectionConfig::default(),
            ttl: CacheTtlConfig::default(),
        }
    }
}

/// Structured cache key with tenant and user isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Tenant ID for multi-tenant isolation
    pub tenant_id: TenantId,
    /// User ID for per-user isolation
    pub user_id: Uuid,
    /// OAuth provider name
    pub provider: String,
    /// Specific resource being cached
    pub resource: CacheResource,
}

impl CacheKey {
    /// Create new cache key
    #[must_use]
    pub const fn new(
        tenant_id: TenantId,
        user_id: Uuid,
        provider: String,
        resource: CacheResource,
    ) -> Self {
        Self {
            tenant_id,
            user_id,
            provider,
            resource,
        }
    }

    /// Create pattern for invalidating all entries for a user
    #[must_use]
    pub fn user_pattern(tenant_id: TenantId, user_id: Uuid, provider: &str) -> String {
        format!("tenant:{tenant_id}:user:{user_id}:provider:{provider}:*")
    }

    /// Create pattern for invalidating all entries for a tenant
    #[must_use]
    pub fn tenant_pattern(tenant_id: TenantId, provider: &str) -> String {
        format!("tenant:{tenant_id}:*:provider:{provider}:*")
    }
}

impl fmt::Display for CacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "tenant:{}:user:{}:provider:{}:{}",
            self.tenant_id, self.user_id, self.provider, self.resource
        )
    }
}

/// Cache resource types with specific parameters
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CacheResource {
    /// Athlete profile (24h TTL)
    AthleteProfile,
    /// Activity list with pagination and optional time/sport filters (15min TTL)
    ActivityList {
        /// Page number for pagination
        page: u32,
        /// Items per page
        per_page: u32,
        /// Optional Unix timestamp (seconds) - return activities before this time
        before: Option<i64>,
        /// Optional Unix timestamp (seconds) - return activities after this time
        after: Option<i64>,
        /// Optional sport type filter (e.g., "run", "ride") for server-side filtering
        sport_type: Option<String>,
    },
    /// Single activity summary (1h TTL)
    Activity {
        /// Activity ID
        activity_id: u64,
    },
    /// Athlete statistics (6h TTL)
    Stats {
        /// Athlete ID
        athlete_id: u64,
    },
    /// Detailed activity with streams (1h TTL)
    DetailedActivity {
        /// Activity ID
        activity_id: u64,
    },
}

impl CacheResource {
    /// Get recommended TTL for this resource type
    #[must_use]
    pub const fn recommended_ttl(&self) -> Duration {
        match self {
            Self::AthleteProfile => Duration::from_secs(TTL_PROFILE_SECS),
            Self::ActivityList { .. } => Duration::from_secs(TTL_ACTIVITY_LIST_SECS),
            Self::Activity { .. } | Self::DetailedActivity { .. } => {
                Duration::from_secs(TTL_ACTIVITY_SECS)
            }
            Self::Stats { .. } => Duration::from_secs(TTL_STATS_SECS),
        }
    }
}

impl fmt::Display for CacheResource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AthleteProfile => write!(f, "athlete_profile"),
            Self::ActivityList {
                page,
                per_page,
                before,
                after,
                sport_type,
            } => {
                let before_str = before.map_or(String::new(), |t| format!(":before:{t}"));
                let after_str = after.map_or(String::new(), |t| format!(":after:{t}"));
                let sport_str = sport_type
                    .as_ref()
                    .map_or(String::new(), |s| format!(":sport:{s}"));
                write!(
                    f,
                    "activity_list:page:{page}:per_page:{per_page}{before_str}{after_str}{sport_str}"
                )
            }
            Self::Activity { activity_id } => write!(f, "activity:{activity_id}"),
            Self::Stats { athlete_id } => write!(f, "stats:{athlete_id}"),
            Self::DetailedActivity { activity_id } => {
                write!(f, "detailed_activity:{activity_id}")
            }
        }
    }
}
