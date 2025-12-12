// ABOUTME: Redis cache implementation with connection pooling and TTL support
// ABOUTME: Provides distributed caching for multi-instance deployments
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::{CacheConfig, CacheKey, CacheProvider};
use crate::config::environment::RedisConnectionConfig;
use crate::errors::{AppError, AppResult};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{error, info, warn};

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

        let conn_config = &config.redis_connection;

        info!(
            "Connecting to Redis at {} (timeout={}s, response_timeout={}s, retries={})",
            redis_url,
            conn_config.connection_timeout_secs,
            conn_config.response_timeout_secs,
            conn_config.initial_connection_retries
        );

        // Create Redis client
        let client = redis::Client::open(redis_url.as_str())
            .map_err(|e| AppError::internal(format!("Failed to create Redis client: {e}")))?;

        // Connect with retry logic
        let manager = Self::connect_with_retry(&client, conn_config).await?;

        info!("Successfully connected to Redis");

        Ok(Self { manager })
    }

    /// Connect to Redis with exponential backoff retry on failure
    ///
    /// Uses `ConnectionManagerConfig` to configure timeouts and reconnection behavior.
    async fn connect_with_retry(
        client: &redis::Client,
        conn_config: &RedisConnectionConfig,
    ) -> AppResult<ConnectionManager> {
        // Configure connection manager with timeout and reconnection settings
        let manager_config = ConnectionManagerConfig::new()
            .set_connection_timeout(Duration::from_secs(conn_config.connection_timeout_secs))
            .set_response_timeout(Duration::from_secs(conn_config.response_timeout_secs))
            .set_number_of_retries(conn_config.reconnection_retries)
            .set_exponent_base(conn_config.retry_exponent_base)
            .set_max_delay(conn_config.max_retry_delay_ms);

        let max_retries = conn_config.initial_connection_retries;
        let initial_delay_ms = conn_config.initial_retry_delay_ms;
        let max_delay_ms = conn_config.max_retry_delay_ms;

        let mut last_error = None;
        let mut delay_ms = initial_delay_ms;

        for attempt in 0..=max_retries {
            match ConnectionManager::new_with_config(client.clone(), manager_config.clone()).await {
                Ok(manager) => {
                    if attempt > 0 {
                        info!("Redis connection established after {} retries", attempt);
                    }
                    return Ok(manager);
                }
                Err(e) => {
                    last_error = Some(e);

                    if attempt < max_retries {
                        warn!(
                            "Redis connection attempt {}/{} failed, retrying in {}ms: {}",
                            attempt + 1,
                            max_retries + 1,
                            delay_ms,
                            last_error
                                .as_ref()
                                .map_or_else(|| "unknown".to_owned(), ToString::to_string)
                        );
                        tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        // Exponential backoff with cap
                        delay_ms = (delay_ms * 2).min(max_delay_ms);
                    }
                }
            }
        }

        // All retries exhausted
        Err(AppError::internal(format!(
            "Failed to connect to Redis after {} retries: {}",
            max_retries + 1,
            last_error.map_or_else(|| "unknown error".to_owned(), |e| e.to_string())
        )))
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
                error!("Redis SET operation failed: {}", e);
                AppError::internal(format!("Cache error: {e}"))
            })?;

        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> AppResult<Option<T>> {
        let redis_key = Self::build_key(key);
        let mut conn = self.manager.clone();

        let data: Option<Vec<u8>> = conn.get(&redis_key).await.map_err(|e| {
            error!("Redis GET operation failed: {}", e);
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
            error!("Redis DEL operation failed: {}", e);
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
                    error!("Redis SCAN failed: {}", e);
                    AppError::internal(format!("Cache error: {e}"))
                })?;

            // Delete matching keys in pipeline for efficiency
            if !keys.is_empty() {
                let deleted: u64 = conn.del(&keys).await.map_err(|e| {
                    error!("Redis DEL failed: {}", e);
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
            error!("Redis EXISTS operation failed: {}", e);
            AppError::internal(format!("Cache error: {e}"))
        })?;

        Ok(exists)
    }

    async fn ttl(&self, key: &CacheKey) -> AppResult<Option<Duration>> {
        let redis_key = Self::build_key(key);
        let mut conn = self.manager.clone();

        let ttl_secs: i64 = conn.ttl(&redis_key).await.map_err(|e| {
            error!("Redis TTL operation failed: {}", e);
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
                error!("Redis PING failed: {}", e);
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
                    error!("Redis SCAN failed during clear_all: {}", e);
                    AppError::internal(format!("Cache error: {e}"))
                })?;

            if !keys.is_empty() {
                let _: u64 = conn.del(&keys).await.map_err(|e| {
                    error!("Redis DEL failed during clear_all: {}", e);
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
