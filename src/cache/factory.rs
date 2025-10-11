// ABOUTME: Cache factory for environment-based backend selection
// ABOUTME: Follows DatabaseProvider pattern for pluggable cache backends
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::{memory::InMemoryCache, CacheConfig, CacheProvider};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Unified cache interface
/// Currently only supports in-memory backend. Redis backend will be added when needed.
#[derive(Clone)]
pub struct Cache {
    inner: InMemoryCache,
}

impl Cache {
    /// Create new cache instance based on configuration
    ///
    /// # Errors
    ///
    /// Returns an error if cache initialization fails
    pub async fn new(config: CacheConfig) -> Result<Self> {
        if config.redis_url.is_some() {
            tracing::warn!(
                "Redis cache requested but not yet implemented. Using in-memory cache instead."
            );
        }

        tracing::info!(
            "Initializing in-memory cache (max entries: {})",
            config.max_entries
        );
        let inner = InMemoryCache::new(config).await?;
        Ok(Self { inner })
    }

    /// Create cache from environment variables
    ///
    /// Currently uses in-memory cache. Redis support will be added in future.
    ///
    /// # Errors
    ///
    /// Returns an error if cache initialization fails
    pub async fn from_env() -> Result<Self> {
        let config = CacheConfig {
            max_entries: std::env::var("CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(crate::constants::cache::DEFAULT_CACHE_MAX_ENTRIES),
            redis_url: std::env::var("REDIS_URL").ok(),
            cleanup_interval: std::env::var("CACHE_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .map_or_else(
                    || Duration::from_secs(crate::constants::cache::DEFAULT_CLEANUP_INTERVAL_SECS),
                    Duration::from_secs,
                ),
            // Enable background cleanup for production use
            enable_background_cleanup: true,
        };

        Self::new(config).await
    }

    /// Store value in cache with TTL
    ///
    /// # Errors
    ///
    /// Returns an error if serialization or storage fails
    pub async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &super::CacheKey,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        self.inner.set(key, value, ttl).await
    }

    /// Retrieve value from cache
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails
    pub async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        key: &super::CacheKey,
    ) -> Result<Option<T>> {
        self.inner.get(key).await
    }

    /// Remove single cache entry
    ///
    /// # Errors
    ///
    /// Returns an error if invalidation fails
    pub async fn invalidate(&self, key: &super::CacheKey) -> Result<()> {
        self.inner.invalidate(key).await
    }

    /// Remove all cache entries matching pattern
    ///
    /// # Errors
    ///
    /// Returns an error if pattern invalidation fails
    pub async fn invalidate_pattern(&self, pattern: &str) -> Result<u64> {
        self.inner.invalidate_pattern(pattern).await
    }

    /// Check if key exists in cache
    ///
    /// # Errors
    ///
    /// Returns an error if existence check fails
    pub async fn exists(&self, key: &super::CacheKey) -> Result<bool> {
        self.inner.exists(key).await
    }

    /// Get remaining TTL for key
    ///
    /// # Errors
    ///
    /// Returns an error if TTL check fails
    pub async fn ttl(&self, key: &super::CacheKey) -> Result<Option<Duration>> {
        self.inner.ttl(key).await
    }

    /// Verify cache backend is healthy
    ///
    /// # Errors
    ///
    /// Returns an error if health check fails
    pub async fn health_check(&self) -> Result<()> {
        self.inner.health_check().await
    }

    /// Clear all cache entries
    ///
    /// # Errors
    ///
    /// Returns an error if clear operation fails
    pub async fn clear_all(&self) -> Result<()> {
        self.inner.clear_all().await
    }
}
