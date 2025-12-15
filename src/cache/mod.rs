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

use crate::config::environment::RedisConnectionConfig;
use crate::constants::cache::{
    DEFAULT_CACHE_MAX_ENTRIES, DEFAULT_CLEANUP_INTERVAL_SECS, TTL_ACTIVITY_LIST_SECS,
    TTL_ACTIVITY_SECS, TTL_PROFILE_SECS, TTL_STATS_SECS,
};
use crate::errors::AppResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;
use uuid::Uuid;

/// Cache provider trait for pluggable backend implementations
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
    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> AppResult<Option<T>>;

    /// Remove single cache entry
    ///
    /// # Errors
    ///
    /// Returns an error if invalidation fails
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
        }
    }
}

/// Structured cache key with tenant and user isolation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Tenant ID for multi-tenant isolation
    pub tenant_id: Uuid,
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
        tenant_id: Uuid,
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
    pub fn user_pattern(tenant_id: Uuid, user_id: Uuid, provider: &str) -> String {
        format!("tenant:{tenant_id}:user:{user_id}:provider:{provider}:*")
    }

    /// Create pattern for invalidating all entries for a tenant
    #[must_use]
    pub fn tenant_pattern(tenant_id: Uuid, provider: &str) -> String {
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
    /// Activity list with pagination and optional time filters (15min TTL)
    ActivityList {
        /// Page number for pagination
        page: u32,
        /// Items per page
        per_page: u32,
        /// Optional Unix timestamp (seconds) - return activities before this time
        before: Option<i64>,
        /// Optional Unix timestamp (seconds) - return activities after this time
        after: Option<i64>,
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
            } => {
                let before_str = before.map_or(String::new(), |t| format!(":before:{t}"));
                let after_str = after.map_or(String::new(), |t| format!(":after:{t}"));
                write!(
                    f,
                    "activity_list:page:{page}:per_page:{per_page}{before_str}{after_str}"
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
