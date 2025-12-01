// ABOUTME: Redis cache implementation with connection pooling and TTL support
// ABOUTME: Provides distributed caching for multi-instance deployments
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::{CacheConfig, CacheKey, CacheProvider};
use crate::errors::{AppError, AppResult};
use redis::{aio::ConnectionManager, AsyncCommands};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Redis cache implementation with connection pooling
///
/// Uses Redis `ConnectionManager` for automatic reconnection and connection pooling.
/// All keys are prefixed with `CACHE_KEY_PREFIX` for namespace isolation.
/// Supports TTL-based expiration and pattern-based invalidation using Redis SCAN.
#[derive(Clone)]
pub struct RedisCache {
    manager: ConnectionManager,
}

impl RedisCache {
    /// Create new Redis cache instance
    ///
    /// # Errors
    ///
    /// Returns an error if Redis connection fails
    async fn new_with_config(config: &CacheConfig) -> AppResult<Self> {
        let redis_url = config
            .redis_url
            .as_ref()
            .ok_or_else(|| AppError::config("Redis URL is required for Redis cache backend"))?;

        tracing::info!("Connecting to Redis at {}", redis_url);

        // Create Redis client with connection timeout
        let client = redis::Client::open(redis_url.as_str())
            .map_err(|e| AppError::internal(format!("Failed to create Redis client: {e}")))?;

        // Create connection manager (handles reconnection automatically)
        let manager = ConnectionManager::new(client)
            .await
            .map_err(|e| AppError::internal(format!("Failed to connect to Redis: {e}")))?;

        tracing::info!("Successfully connected to Redis");

        Ok(Self { manager })
    }

    /// Build full Redis key with namespace prefix
    fn build_key(key: &CacheKey) -> String {
        format!("{}{}", crate::constants::cache::CACHE_KEY_PREFIX, key)
    }
}

#[async_trait::async_trait]
impl CacheProvider for RedisCache {
    async fn new(config: CacheConfig) -> AppResult<Self>
    where
        Self: Sized,
    {
        Self::new_with_config(&config).await
    }

    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &CacheKey,
        value: &T,
        ttl: Duration,
    ) -> AppResult<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| AppError::internal(format!("Cache serialization failed: {e}")))?;
        let redis_key = Self::build_key(key);
        let ttl_secs = ttl.as_secs();

        let mut conn = self.manager.clone();

        // Use SETEX to set value with expiration in one atomic operation
        conn.set_ex::<_, _, ()>(&redis_key, serialized, ttl_secs)
            .await
            .map_err(|e| {
                tracing::error!("Redis SET operation failed: {}", e);
                AppError::internal(format!("Cache error: {e}"))
            })?;

        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> AppResult<Option<T>> {
        let redis_key = Self::build_key(key);
        let mut conn = self.manager.clone();

        let data: Option<Vec<u8>> = conn.get(&redis_key).await.map_err(|e| {
            tracing::error!("Redis GET operation failed: {}", e);
            AppError::internal(format!("Cache error: {e}"))
        })?;

        match data {
            Some(bytes) => {
                let value: T = serde_json::from_slice(&bytes).map_err(|e| {
                    AppError::internal(format!("Cache deserialization failed: {e}"))
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn invalidate(&self, key: &CacheKey) -> AppResult<()> {
        let redis_key = Self::build_key(key);
        let mut conn = self.manager.clone();

        let _: () = conn.del(&redis_key).await.map_err(|e| {
            tracing::error!("Redis DEL operation failed: {}", e);
            AppError::internal(format!("Cache error: {e}"))
        })?;

        Ok(())
    }

    async fn invalidate_pattern(&self, pattern: &str) -> AppResult<u64> {
        // Convert glob pattern to Redis pattern (glob and Redis use same wildcard syntax)
        let redis_pattern = format!("{}{}", crate::constants::cache::CACHE_KEY_PREFIX, pattern);

        let mut conn = self.manager.clone();
        let mut count = 0u64;

        // Use SCAN to iterate through keys matching pattern (cursor-based, safe for large datasets)
        let mut cursor = 0u64;
        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&redis_pattern)
                .arg("COUNT")
                .arg(100) // Scan 100 keys per iteration
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    tracing::error!("Redis SCAN failed: {}", e);
                    AppError::internal(format!("Cache error: {e}"))
                })?;

            // Delete matching keys in pipeline for efficiency
            if !keys.is_empty() {
                let deleted: u64 = conn.del(&keys).await.map_err(|e| {
                    tracing::error!("Redis DEL failed: {}", e);
                    AppError::internal(format!("Cache error: {e}"))
                })?;
                count += deleted;
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(count)
    }

    async fn exists(&self, key: &CacheKey) -> AppResult<bool> {
        let redis_key = Self::build_key(key);
        let mut conn = self.manager.clone();

        let exists: bool = conn.exists(&redis_key).await.map_err(|e| {
            tracing::error!("Redis EXISTS operation failed: {}", e);
            AppError::internal(format!("Cache error: {e}"))
        })?;

        Ok(exists)
    }

    async fn ttl(&self, key: &CacheKey) -> AppResult<Option<Duration>> {
        let redis_key = Self::build_key(key);
        let mut conn = self.manager.clone();

        let ttl_secs: i64 = conn.ttl(&redis_key).await.map_err(|e| {
            tracing::error!("Redis TTL operation failed: {}", e);
            AppError::internal(format!("Cache error: {e}"))
        })?;

        // Redis returns -2 if key doesn't exist, -1 if key has no expiration
        match ttl_secs {
            -2 | -1 => Ok(None),
            #[allow(clippy::cast_sign_loss)] // Validated: secs > 0 before cast
            secs if secs > 0 => Ok(Some(Duration::from_secs(secs as u64))),
            _ => Ok(None),
        }
    }

    async fn health_check(&self) -> AppResult<()> {
        let mut conn = self.manager.clone();

        // Use PING to verify Redis connection is healthy
        let response: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| {
                tracing::error!("Redis PING failed: {}", e);
                AppError::internal(format!("Cache error: {e}"))
            })?;

        if response == "PONG" {
            Ok(())
        } else {
            Err(AppError::internal(format!(
                "Cache error: unexpected PING response '{response}'"
            )))
        }
    }

    async fn clear_all(&self) -> AppResult<()> {
        // Clear only keys with our namespace prefix (safe for shared Redis instances)
        let pattern = format!("{}*", crate::constants::cache::CACHE_KEY_PREFIX);

        let mut conn = self.manager.clone();
        let mut cursor = 0u64;

        loop {
            let (new_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                .arg(cursor)
                .arg("MATCH")
                .arg(&pattern)
                .arg("COUNT")
                .arg(100)
                .query_async(&mut conn)
                .await
                .map_err(|e| {
                    tracing::error!("Redis SCAN failed during clear_all: {}", e);
                    AppError::internal(format!("Cache error: {e}"))
                })?;

            if !keys.is_empty() {
                let _: u64 = conn.del(&keys).await.map_err(|e| {
                    tracing::error!("Redis DEL failed during clear_all: {}", e);
                    AppError::internal(format!("Cache error: {e}"))
                })?;
            }

            cursor = new_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(())
    }
}
