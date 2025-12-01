// ABOUTME: In-memory cache for Terra webhook data
// ABOUTME: Stores activities, sleep, health metrics, and nutrition from webhooks for FitnessProvider access
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Terra data cache
//!
//! This module provides an in-memory cache layer for storing data received from Terra webhooks.
//! The cache enables the `FitnessProvider` trait implementation to serve data using
//! Pierre's pull-based model while Terra uses a push-based webhook model.
//!
//! ## Storage Strategy
//!
//! Data is stored in-memory with per-user partitioning:
//! - Activities: Cached workout/activity data
//! - Sleep sessions: Cached sleep data with stages
//! - Health metrics: Cached body measurements
//! - Recovery metrics: Cached recovery data from daily/sleep readiness
//! - Nutrition logs: Cached food/nutrition data
//!
//! ## Data Lifecycle
//!
//! - **Insert/Update**: Webhook handler stores new data via `store_*` methods
//! - **Query**: Provider reads data via `get_*` methods
//! - **Expiry**: Old data is automatically cleaned up (configurable TTL, default 7 days)

use crate::models::{Activity, HealthMetrics, NutritionLog, RecoveryMetrics, SleepSession};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Cache configuration
#[derive(Debug, Clone)]
pub struct TerraDataCacheConfig {
    /// Time-to-live for cached data (default: 7 days)
    pub ttl_days: i64,
    /// Maximum items per user per data type (default: 1000)
    pub max_items_per_type: usize,
}

impl Default for TerraDataCacheConfig {
    fn default() -> Self {
        Self {
            ttl_days: 7,
            max_items_per_type: 1000,
        }
    }
}

/// In-memory cache entry with expiry tracking
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    cached_at: DateTime<Utc>,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            cached_at: Utc::now(),
        }
    }

    fn is_expired(&self, ttl_days: i64) -> bool {
        Utc::now() > self.cached_at + Duration::days(ttl_days)
    }
}

/// User's cached data
#[derive(Debug, Default)]
struct UserCache {
    activities: Vec<CacheEntry<Activity>>,
    sleep_sessions: Vec<CacheEntry<SleepSession>>,
    health_metrics: Vec<CacheEntry<HealthMetrics>>,
    recovery_metrics: Vec<CacheEntry<RecoveryMetrics>>,
    nutrition_logs: Vec<CacheEntry<NutritionLog>>,
}

impl UserCache {
    /// Add an activity to the cache, avoiding duplicates and enforcing limits
    fn add_activity(&mut self, activity: Activity, max_items: usize) {
        // Check for duplicate by ID
        if !self.activities.iter().any(|e| e.data.id == activity.id) {
            self.activities.push(CacheEntry::new(activity));

            // Enforce max items limit
            if self.activities.len() > max_items {
                // Remove oldest entries
                self.activities
                    .sort_by(|a, b| b.data.start_date.cmp(&a.data.start_date));
                self.activities.truncate(max_items);
            }
        }
    }

    /// Add a sleep session to the cache, avoiding duplicates and enforcing limits
    fn add_sleep_session(&mut self, sleep: SleepSession, max_items: usize) {
        // Check for duplicate by ID
        if !self.sleep_sessions.iter().any(|e| e.data.id == sleep.id) {
            self.sleep_sessions.push(CacheEntry::new(sleep));

            // Enforce max items limit
            if self.sleep_sessions.len() > max_items {
                self.sleep_sessions
                    .sort_by(|a, b| b.data.start_time.cmp(&a.data.start_time));
                self.sleep_sessions.truncate(max_items);
            }
        }
    }

    /// Add health metrics to the cache, replacing existing for same date
    fn add_health_metrics(&mut self, health: HealthMetrics, max_items: usize) {
        // Replace or add based on date (one entry per day)
        let date_key = health.date.date_naive();
        self.health_metrics
            .retain(|e| e.data.date.date_naive() != date_key);
        self.health_metrics.push(CacheEntry::new(health));

        // Enforce max items limit
        if self.health_metrics.len() > max_items {
            self.health_metrics
                .sort_by(|a, b| b.data.date.cmp(&a.data.date));
            self.health_metrics.truncate(max_items);
        }
    }

    /// Add recovery metrics to the cache, replacing existing for same date
    fn add_recovery_metrics(&mut self, recovery: RecoveryMetrics, max_items: usize) {
        // Replace or add based on date
        let date_key = recovery.date.date_naive();
        self.recovery_metrics
            .retain(|e| e.data.date.date_naive() != date_key);
        self.recovery_metrics.push(CacheEntry::new(recovery));

        // Enforce max items limit
        if self.recovery_metrics.len() > max_items {
            self.recovery_metrics
                .sort_by(|a, b| b.data.date.cmp(&a.data.date));
            self.recovery_metrics.truncate(max_items);
        }
    }

    /// Add nutrition log to the cache, replacing existing for same date
    fn add_nutrition_log(&mut self, nutrition: NutritionLog, max_items: usize) {
        // Replace or add based on date
        let date_key = nutrition.date.date_naive();
        self.nutrition_logs
            .retain(|e| e.data.date.date_naive() != date_key);
        self.nutrition_logs.push(CacheEntry::new(nutrition));

        // Enforce max items limit
        if self.nutrition_logs.len() > max_items {
            self.nutrition_logs
                .sort_by(|a, b| b.data.date.cmp(&a.data.date));
            self.nutrition_logs.truncate(max_items);
        }
    }
}

/// Terra data cache for webhook data storage
///
/// This cache stores data received from Terra webhooks and makes it available
/// to the `TerraProvider` for `FitnessProvider` trait implementation.
pub struct TerraDataCache {
    config: TerraDataCacheConfig,
    /// User data cache keyed by Terra user ID
    users: Arc<RwLock<HashMap<String, UserCache>>>,
    /// Mapping from `reference_id` to `terra_user_id`
    reference_map: Arc<RwLock<HashMap<String, String>>>,
}

impl TerraDataCache {
    /// Create a new in-memory Terra data cache
    #[must_use]
    pub fn new_in_memory() -> Self {
        Self {
            config: TerraDataCacheConfig::default(),
            users: Arc::new(RwLock::new(HashMap::new())),
            reference_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new cache with custom configuration
    #[must_use]
    pub fn with_config(config: TerraDataCacheConfig) -> Self {
        Self {
            config,
            users: Arc::new(RwLock::new(HashMap::new())),
            reference_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a mapping from `reference_id` to `terra_user_id`
    pub async fn register_user_mapping(&self, reference_id: &str, terra_user_id: &str) {
        let mut map = self.reference_map.write().await;
        map.insert(reference_id.to_owned(), terra_user_id.to_owned());
    }

    /// Get Terra user ID from reference ID
    pub async fn get_terra_user_id(&self, reference_id: &str) -> Option<String> {
        let map = self.reference_map.read().await;
        map.get(reference_id).cloned()
    }

    /// Store an activity in the cache
    pub async fn store_activity(&self, terra_user_id: &str, activity: Activity) {
        let max_items = self.config.max_items_per_type;
        self.users
            .write()
            .await
            .entry(terra_user_id.to_owned())
            .or_default()
            .add_activity(activity, max_items);
    }

    /// Store multiple activities in the cache
    pub async fn store_activities(&self, terra_user_id: &str, activities: Vec<Activity>) {
        for activity in activities {
            self.store_activity(terra_user_id, activity).await;
        }
    }

    /// Get activities for a user
    pub async fn get_activities(
        &self,
        terra_user_id: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> Vec<Activity> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        let Some(mut activities) = self.users.read().await.get(terra_user_id).map(|cache| {
            cache
                .activities
                .iter()
                .filter(|e| !e.is_expired(ttl))
                .map(|e| e.data.clone())
                .collect::<Vec<_>>()
        }) else {
            return Vec::new();
        };

        activities.sort_by(|a, b| b.start_date.cmp(&a.start_date));

        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(usize::MAX);
        activities.into_iter().skip(offset).take(limit).collect()
    }

    /// Get a specific activity by ID
    pub async fn get_activity(&self, terra_user_id: &str, activity_id: &str) -> Option<Activity> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        self.users
            .read()
            .await
            .get(terra_user_id)?
            .activities
            .iter()
            .find(|e| e.data.id == activity_id && !e.is_expired(ttl))
            .map(|e| e.data.clone())
    }

    /// Store a sleep session in the cache
    pub async fn store_sleep_session(&self, terra_user_id: &str, sleep: SleepSession) {
        let max_items = self.config.max_items_per_type;
        self.users
            .write()
            .await
            .entry(terra_user_id.to_owned())
            .or_default()
            .add_sleep_session(sleep, max_items);
    }

    /// Get sleep sessions for a date range
    pub async fn get_sleep_sessions(
        &self,
        terra_user_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Vec<SleepSession> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        let Some(mut sessions) = self.users.read().await.get(terra_user_id).map(|cache| {
            cache
                .sleep_sessions
                .iter()
                .filter(|e| {
                    !e.is_expired(ttl)
                        && e.data.start_time >= start_date
                        && e.data.start_time <= end_date
                })
                .map(|e| e.data.clone())
                .collect::<Vec<_>>()
        }) else {
            return Vec::new();
        };

        sessions.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        sessions
    }

    /// Get the latest sleep session
    pub async fn get_latest_sleep_session(&self, terra_user_id: &str) -> Option<SleepSession> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        self.users
            .read()
            .await
            .get(terra_user_id)?
            .sleep_sessions
            .iter()
            .filter(|e| !e.is_expired(ttl))
            .max_by_key(|e| e.data.start_time)
            .map(|e| e.data.clone())
    }

    /// Store health metrics in the cache
    pub async fn store_health_metrics(&self, terra_user_id: &str, health: HealthMetrics) {
        let max_items = self.config.max_items_per_type;
        self.users
            .write()
            .await
            .entry(terra_user_id.to_owned())
            .or_default()
            .add_health_metrics(health, max_items);
    }

    /// Get health metrics for a date range
    pub async fn get_health_metrics(
        &self,
        terra_user_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Vec<HealthMetrics> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        let Some(mut metrics) = self.users.read().await.get(terra_user_id).map(|cache| {
            cache
                .health_metrics
                .iter()
                .filter(|e| {
                    !e.is_expired(ttl) && e.data.date >= start_date && e.data.date <= end_date
                })
                .map(|e| e.data.clone())
                .collect::<Vec<_>>()
        }) else {
            return Vec::new();
        };

        metrics.sort_by(|a, b| b.date.cmp(&a.date));
        metrics
    }

    /// Store recovery metrics in the cache
    pub async fn store_recovery_metrics(&self, terra_user_id: &str, recovery: RecoveryMetrics) {
        let max_items = self.config.max_items_per_type;
        self.users
            .write()
            .await
            .entry(terra_user_id.to_owned())
            .or_default()
            .add_recovery_metrics(recovery, max_items);
    }

    /// Get recovery metrics for a date range
    pub async fn get_recovery_metrics(
        &self,
        terra_user_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Vec<RecoveryMetrics> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        let Some(mut metrics) = self.users.read().await.get(terra_user_id).map(|cache| {
            cache
                .recovery_metrics
                .iter()
                .filter(|e| {
                    !e.is_expired(ttl) && e.data.date >= start_date && e.data.date <= end_date
                })
                .map(|e| e.data.clone())
                .collect::<Vec<_>>()
        }) else {
            return Vec::new();
        };

        metrics.sort_by(|a, b| b.date.cmp(&a.date));
        metrics
    }

    /// Store nutrition log in the cache
    pub async fn store_nutrition_log(&self, terra_user_id: &str, nutrition: NutritionLog) {
        let max_items = self.config.max_items_per_type;
        self.users
            .write()
            .await
            .entry(terra_user_id.to_owned())
            .or_default()
            .add_nutrition_log(nutrition, max_items);
    }

    /// Get nutrition logs for a date range
    pub async fn get_nutrition_logs(
        &self,
        terra_user_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Vec<NutritionLog> {
        let ttl = self.config.ttl_days;

        // Chain directly to avoid holding lock guard in named variable
        let Some(mut logs) = self.users.read().await.get(terra_user_id).map(|cache| {
            cache
                .nutrition_logs
                .iter()
                .filter(|e| {
                    !e.is_expired(ttl) && e.data.date >= start_date && e.data.date <= end_date
                })
                .map(|e| e.data.clone())
                .collect::<Vec<_>>()
        }) else {
            return Vec::new();
        };

        logs.sort_by(|a, b| b.date.cmp(&a.date));
        logs
    }

    /// Clean up expired entries from the cache
    pub async fn cleanup_expired(&self) {
        let mut users = self.users.write().await;
        let ttl = self.config.ttl_days;

        for user_cache in users.values_mut() {
            user_cache.activities.retain(|e| !e.is_expired(ttl));
            user_cache.sleep_sessions.retain(|e| !e.is_expired(ttl));
            user_cache.health_metrics.retain(|e| !e.is_expired(ttl));
            user_cache.recovery_metrics.retain(|e| !e.is_expired(ttl));
            user_cache.nutrition_logs.retain(|e| !e.is_expired(ttl));
        }

        // Remove empty user caches
        users.retain(|_, cache| {
            !cache.activities.is_empty()
                || !cache.sleep_sessions.is_empty()
                || !cache.health_metrics.is_empty()
                || !cache.recovery_metrics.is_empty()
                || !cache.nutrition_logs.is_empty()
        });
    }

    /// Get cache statistics for monitoring
    pub async fn get_stats(&self) -> CacheStats {
        let users = self.users.read().await;

        let mut total_activities = 0;
        let mut total_sleep_sessions = 0;
        let mut total_health_metrics = 0;
        let mut total_recovery_metrics = 0;
        let mut total_nutrition_logs = 0;

        for cache in users.values() {
            total_activities += cache.activities.len();
            total_sleep_sessions += cache.sleep_sessions.len();
            total_health_metrics += cache.health_metrics.len();
            total_recovery_metrics += cache.recovery_metrics.len();
            total_nutrition_logs += cache.nutrition_logs.len();
        }

        CacheStats {
            user_count: users.len(),
            total_activities,
            total_sleep_sessions,
            total_health_metrics,
            total_recovery_metrics,
            total_nutrition_logs,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of users with cached data
    pub user_count: usize,
    /// Total cached activities across all users
    pub total_activities: usize,
    /// Total cached sleep sessions
    pub total_sleep_sessions: usize,
    /// Total cached health metrics
    pub total_health_metrics: usize,
    /// Total cached recovery metrics
    pub total_recovery_metrics: usize,
    /// Total cached nutrition logs
    pub total_nutrition_logs: usize,
}
