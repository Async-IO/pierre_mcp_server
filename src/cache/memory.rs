// ABOUTME: In-memory cache implementation with LRU eviction and TTL support
// ABOUTME: Includes background cleanup task for expired entries
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::{CacheConfig, CacheKey, CacheProvider};
use crate::errors::AppError;
use anyhow::Result;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// In-memory cache entry with expiration
#[derive(Debug, Clone)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: Instant,
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    fn remaining_ttl(&self) -> Option<Duration> {
        self.expires_at.checked_duration_since(Instant::now())
    }
}

/// In-memory cache with LRU eviction and background cleanup
///
/// Uses `Arc<RwLock<LruCache>>` for shared state between cache operations and background cleanup task.
/// The Arc is required because the cleanup task (spawned in `new_with_config`) needs shared
/// ownership of the store to remove expired entries concurrently.
/// `LruCache` provides O(1) eviction by automatically removing least-recently-used entries.
#[derive(Clone)]
pub struct InMemoryCache {
    store: Arc<RwLock<LruCache<String, CacheEntry>>>,
    shutdown_tx: Option<Arc<tokio::sync::mpsc::Sender<()>>>,
}

impl InMemoryCache {
    /// Default cache capacity when config specifies zero entries
    /// Note: `unwrap()` on compile-time constant is verified at compile time
    const DEFAULT_CACHE_CAPACITY: NonZeroUsize = match NonZeroUsize::new(1000) {
        Some(n) => n,
        None => unreachable!(),
    };

    /// Create new in-memory cache with optional background cleanup task
    fn new_with_config(config: &CacheConfig) -> Self {
        // LruCache requires NonZeroUsize for capacity
        let capacity =
            NonZeroUsize::new(config.max_entries).unwrap_or(Self::DEFAULT_CACHE_CAPACITY);

        let store = Arc::new(RwLock::new(LruCache::new(capacity)));

        let shutdown_tx = if config.enable_background_cleanup {
            let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
            let store_clone = store.clone();
            let cleanup_interval = config.cleanup_interval;

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(cleanup_interval);
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            Self::cleanup_expired(&store_clone).await;
                        }
                        _ = shutdown_rx.recv() => {
                            tracing::debug!("Cache cleanup task received shutdown signal");
                            break;
                        }
                    }
                }
            });

            Some(Arc::new(shutdown_tx))
        } else {
            None
        };

        Self { store, shutdown_tx }
    }

    /// Remove all expired entries from cache
    async fn cleanup_expired(store: &Arc<RwLock<LruCache<String, CacheEntry>>>) {
        let mut store_guard = store.write().await;

        // Collect expired keys first (can't modify while iterating)
        let expired_keys: Vec<String> = store_guard
            .iter()
            .filter_map(|(k, v)| {
                if v.is_expired() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove expired entries
        for key in &expired_keys {
            store_guard.pop(key);
        }

        let removed = expired_keys.len();
        drop(store_guard);
        if removed > 0 {
            tracing::debug!("Cleaned up {} expired cache entries", removed);
        }
    }
}

#[async_trait::async_trait]
impl CacheProvider for InMemoryCache {
    async fn new(config: CacheConfig) -> Result<Self> {
        Ok(Self::new_with_config(&config))
    }

    async fn set<T: Serialize + Send + Sync>(
        &self,
        key: &CacheKey,
        value: &T,
        ttl: Duration,
    ) -> Result<()> {
        let serialized = serde_json::to_vec(value)?;
        let entry = CacheEntry::new(serialized, ttl);

        // LruCache handles eviction automatically on push
        self.store.write().await.push(key.to_string(), entry);

        Ok(())
    }

    async fn get<T: for<'de> Deserialize<'de>>(&self, key: &CacheKey) -> Result<Option<T>> {
        let mut store = self.store.write().await;

        // LruCache::get is mutable (updates access order for LRU)
        if let Some(entry) = store.get(&key.to_string()) {
            if entry.is_expired() {
                // Remove expired entry
                store.pop(&key.to_string());
                drop(store);
                return Ok(None);
            }

            let value: T = serde_json::from_slice(&entry.data)?;
            drop(store);
            return Ok(Some(value));
        }
        drop(store);

        Ok(None)
    }

    async fn invalidate(&self, key: &CacheKey) -> Result<()> {
        self.store.write().await.pop(&key.to_string());
        Ok(())
    }

    async fn invalidate_pattern(&self, pattern: &str) -> Result<u64> {
        let mut store = self.store.write().await;

        // Use proper glob matching for cache key patterns
        // Patterns like "tenant:*:provider:strava:*" will correctly match wildcards
        let glob_pattern = glob::Pattern::new(pattern).map_err(|e| -> anyhow::Error {
            AppError::internal(format!("Invalid glob pattern '{pattern}': {e}")).into()
        })?;

        // Collect keys to remove (can't modify while iterating)
        let keys_to_remove: Vec<String> = store
            .iter()
            .filter_map(|(k, _)| {
                if glob_pattern.matches(k) {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove matching keys
        for key in &keys_to_remove {
            store.pop(key);
        }

        let removed = keys_to_remove.len() as u64;
        drop(store);
        Ok(removed)
    }

    async fn exists(&self, key: &CacheKey) -> Result<bool> {
        let mut store = self.store.write().await;

        // LruCache::get is mutable, need write lock
        if let Some(entry) = store.get(&key.to_string()) {
            if entry.is_expired() {
                // Remove expired entry
                store.pop(&key.to_string());
                drop(store);
                return Ok(false);
            }
            drop(store);
            return Ok(true);
        }
        drop(store);

        Ok(false)
    }

    async fn ttl(&self, key: &CacheKey) -> Result<Option<Duration>> {
        let store = self.store.write().await;

        // Use peek to avoid updating LRU order
        if let Some(entry) = store.peek(&key.to_string()) {
            if entry.is_expired() {
                return Ok(None);
            }
            let ttl = entry.remaining_ttl();
            drop(store);
            return Ok(ttl);
        }

        Ok(None)
    }

    async fn health_check(&self) -> Result<()> {
        // In-memory cache is always healthy
        Ok(())
    }

    async fn clear_all(&self) -> Result<()> {
        self.store.write().await.clear();
        Ok(())
    }
}

impl Drop for InMemoryCache {
    fn drop(&mut self) {
        // Signal background cleanup task to shutdown on drop
        // Note: This only works if the Sender is fully dropped (all Arc clones released)
        // The task will exit when all senders are dropped and recv() returns None
        if let Some(tx) = &self.shutdown_tx {
            // Try to send shutdown signal, errors are expected if channel is already closed
            if let Err(e) = tx.try_send(()) {
                tracing::debug!(error = ?e, "Cache shutdown signal send failed (channel likely closed)");
            }
        }
    }
}
