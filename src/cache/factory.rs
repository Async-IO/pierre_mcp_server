// ABOUTME: Cache factory for environment-based backend selection
// ABOUTME: Follows DatabaseProvider pattern for pluggable cache backends
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::info;

use super::{memory::InMemoryCache, redis::RedisCache, CacheConfig, CacheProvider};
use crate::config::environment::RedisConnectionConfig;
use crate::constants::get_server_config;
use crate::errors::AppResult;
use crate::middleware::redaction::redact_url;

/// Cache backend enum for pluggable implementations
#[non_exhaustive]
#[derive(Clone)]
enum CacheBackend {
    InMemory(InMemoryCache),
    Redis(Box<RedisCache>),
}

/// Unified cache interface supporting both in-memory and Redis backends
#[derive(Clone)]
pub struct Cache {
    inner: CacheBackend,
}

impl Cache {
    /// Create new cache instance based on configuration
    ///
    /// # Errors
    ///
    /// Returns an error if cache initialization fails
    pub async fn new(config: CacheConfig) -> AppResult<Self> {
        let inner = if let Some(ref redis_url) = config.redis_url {
            info!("Initializing Redis cache (url: {})", redact_url(redis_url));
            let redis = RedisCache::new(config).await?;
            CacheBackend::Redis(Box::new(redis))
        } else {
            info!(
                "Initializing in-memory cache (max entries: {})",
                config.max_entries
            );
            let memory = InMemoryCache::new(config).await?;
            CacheBackend::InMemory(memory)
        };

        Ok(Self { inner })
    }

    /// Create cache from environment variables
    ///
    /// Supports both in-memory and Redis backends based on `REDIS_URL` environment variable.
    /// Uses sensible defaults if server configuration is not yet initialized.
    ///
    /// # Errors
    ///
    /// Returns an error if cache initialization fails
    pub async fn from_env() -> AppResult<Self> {
        let config = get_server_config().map_or_else(
            || CacheConfig {
                max_entries: 1000,
                redis_url: None,
                cleanup_interval: Duration::from_secs(300),
                enable_background_cleanup: true,
                redis_connection: RedisConnectionConfig::default(),
                ttl: super::CacheTtlConfig::default(),
            },
            |server_config| CacheConfig {
                max_entries: server_config.cache.max_entries,
                redis_url: server_config.cache.redis_url.clone(),
                cleanup_interval: Duration::from_secs(server_config.cache.cleanup_interval_secs),
                enable_background_cleanup: true,
                redis_connection: server_config.cache.redis_connection.clone(),
                ttl: super::CacheTtlConfig {
                    profile_secs: server_config.cache.ttl.profile_secs,
                    activity_list_secs: server_config.cache.ttl.activity_list_secs,
                    activity_secs: server_config.cache.ttl.activity_secs,
                    stats_secs: server_config.cache.ttl.stats_secs,
                },
            },
        );

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
    ) -> AppResult<()> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.set(key, value, ttl).await,
            CacheBackend::Redis(cache) => cache.set(key, value, ttl).await,
        }
    }

    /// Retrieve value from cache
    ///
    /// # Errors
    ///
    /// Returns an error if deserialization fails
    pub async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        key: &super::CacheKey,
    ) -> AppResult<Option<T>> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.get(key).await,
            CacheBackend::Redis(cache) => cache.get(key).await,
        }
    }

    /// Remove single cache entry
    ///
    /// # Errors
    ///
    /// Returns an error if invalidation fails
    pub async fn invalidate(&self, key: &super::CacheKey) -> AppResult<()> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.invalidate(key).await,
            CacheBackend::Redis(cache) => cache.invalidate(key).await,
        }
    }

    /// Remove all cache entries matching pattern
    ///
    /// # Errors
    ///
    /// Returns an error if pattern invalidation fails
    pub async fn invalidate_pattern(&self, pattern: &str) -> AppResult<u64> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.invalidate_pattern(pattern).await,
            CacheBackend::Redis(cache) => cache.invalidate_pattern(pattern).await,
        }
    }

    /// Check if key exists in cache
    ///
    /// # Errors
    ///
    /// Returns an error if existence check fails
    pub async fn exists(&self, key: &super::CacheKey) -> AppResult<bool> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.exists(key).await,
            CacheBackend::Redis(cache) => cache.exists(key).await,
        }
    }

    /// Get remaining TTL for key
    ///
    /// # Errors
    ///
    /// Returns an error if TTL check fails
    pub async fn ttl(&self, key: &super::CacheKey) -> AppResult<Option<Duration>> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.ttl(key).await,
            CacheBackend::Redis(cache) => cache.ttl(key).await,
        }
    }

    /// Verify cache backend is healthy
    ///
    /// # Errors
    ///
    /// Returns an error if health check fails
    pub async fn health_check(&self) -> AppResult<()> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.health_check().await,
            CacheBackend::Redis(cache) => cache.health_check().await,
        }
    }

    /// Clear all cache entries
    ///
    /// # Errors
    ///
    /// Returns an error if clear operation fails
    pub async fn clear_all(&self) -> AppResult<()> {
        match &self.inner {
            CacheBackend::InMemory(cache) => cache.clear_all().await,
            CacheBackend::Redis(cache) => cache.clear_all().await,
        }
    }
}
