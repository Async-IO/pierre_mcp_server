// ABOUTME: In-memory cache for friend activity summaries in social features
// ABOUTME: Privacy-preserving cache that stores only aggregated activity data
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Friend Activity Cache
//!
//! This module provides an in-memory cache for friend activity summaries.
//! It stores only aggregated, privacy-safe information about friend activities
//! to enable quick social feed generation without accessing raw activity data.
//!
//! Key privacy guarantees:
//! - No GPS coordinates stored
//! - No exact paces or times stored
//! - Only aggregated summaries cached
//! - Cache entries expire automatically

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

// ============================================================================
// Constants
// ============================================================================

/// Default cache TTL in minutes
const DEFAULT_CACHE_TTL_MINUTES: i64 = 15;

/// Maximum entries per user
const MAX_ENTRIES_PER_USER: usize = 50;

/// Maximum total cache entries
const MAX_TOTAL_ENTRIES: usize = 10000;

// ============================================================================
// Activity Summary
// ============================================================================

/// Privacy-safe summary of a friend's recent activity
///
/// Contains only aggregated information suitable for social display.
/// No exact times, GPS coordinates, or sensitive metrics are stored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FriendActivitySummary {
    /// User ID of the friend
    pub user_id: Uuid,
    /// Display name (if shared)
    pub display_name: Option<String>,
    /// Sport type (run, ride, swim, etc.)
    pub sport_type: String,
    /// Relative time description ("this morning", "yesterday", etc.)
    pub relative_time: String,
    /// Activity duration category (short, medium, long)
    pub duration_category: DurationCategory,
    /// Effort level (easy, moderate, hard)
    pub effort_level: EffortLevel,
    /// Optional achievement badge
    pub achievement: Option<String>,
    /// When this summary was created
    pub created_at: DateTime<Utc>,
    /// When this cache entry expires
    pub expires_at: DateTime<Utc>,
}

/// Duration category for privacy-safe display
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DurationCategory {
    /// Under 30 minutes
    Short,
    /// 30-60 minutes
    Medium,
    /// Over 60 minutes
    Long,
    /// Over 2 hours
    Epic,
}

impl DurationCategory {
    /// Create from duration in minutes
    #[must_use]
    pub const fn from_minutes(minutes: u32) -> Self {
        match minutes {
            0..=29 => Self::Short,
            30..=59 => Self::Medium,
            60..=119 => Self::Long,
            _ => Self::Epic,
        }
    }

    /// Human-readable description
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Short => "quick session",
            Self::Medium => "solid workout",
            Self::Long => "long session",
            Self::Epic => "epic adventure",
        }
    }
}

/// Effort level for privacy-safe display
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffortLevel {
    /// Easy/recovery effort
    Easy,
    /// Moderate effort
    Moderate,
    /// Hard effort
    Hard,
    /// All-out effort
    Max,
}

impl EffortLevel {
    /// Create from heart rate percentage of max
    #[must_use]
    pub const fn from_hr_percentage(pct: u8) -> Self {
        match pct {
            0..=59 => Self::Easy,
            60..=75 => Self::Moderate,
            76..=89 => Self::Hard,
            _ => Self::Max,
        }
    }

    /// Human-readable description
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Easy => "easy effort",
            Self::Moderate => "moderate effort",
            Self::Hard => "hard effort",
            Self::Max => "all-out effort",
        }
    }

    /// Emoji representation
    #[must_use]
    pub const fn emoji(&self) -> &'static str {
        match self {
            Self::Easy => "😊",
            Self::Moderate => "💪",
            Self::Hard => "🔥",
            Self::Max => "⚡",
        }
    }
}

impl FriendActivitySummary {
    /// Create a new activity summary
    #[must_use]
    pub fn new(
        user_id: Uuid,
        sport_type: String,
        relative_time: String,
        duration_category: DurationCategory,
        effort_level: EffortLevel,
    ) -> Self {
        let now = Utc::now();
        Self {
            user_id,
            display_name: None,
            sport_type,
            relative_time,
            duration_category,
            effort_level,
            achievement: None,
            created_at: now,
            expires_at: now + Duration::minutes(DEFAULT_CACHE_TTL_MINUTES),
        }
    }

    /// Set display name
    #[must_use]
    pub fn with_display_name(mut self, name: String) -> Self {
        self.display_name = Some(name);
        self
    }

    /// Set achievement
    #[must_use]
    pub fn with_achievement(mut self, achievement: String) -> Self {
        self.achievement = Some(achievement);
        self
    }

    /// Set custom TTL
    #[must_use]
    pub fn with_ttl_minutes(mut self, minutes: i64) -> Self {
        self.expires_at = self.created_at + Duration::minutes(minutes);
        self
    }

    /// Check if this entry has expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Generate a privacy-safe display string
    #[must_use]
    pub fn display_string(&self) -> String {
        let name = self.display_name.as_deref().unwrap_or("A friend");

        let achievement_text = self
            .achievement
            .as_ref()
            .map_or(String::new(), |a| format!(" - {a}"));

        format!(
            "{name} completed a {duration} {sport} {effort} {emoji}{achievement}",
            duration = self.duration_category.description(),
            sport = self.sport_type,
            effort = self.effort_level.description(),
            emoji = self.effort_level.emoji(),
            achievement = achievement_text,
        )
    }
}

// ============================================================================
// Cache Entry
// ============================================================================

/// Internal cache entry wrapping a summary with metadata
#[derive(Debug, Clone)]
struct CacheEntry {
    summary: FriendActivitySummary,
    access_count: u32,
    last_accessed: DateTime<Utc>,
}

impl CacheEntry {
    fn new(summary: FriendActivitySummary) -> Self {
        Self {
            summary,
            access_count: 0,
            last_accessed: Utc::now(),
        }
    }

    fn touch(&mut self) {
        self.access_count = self.access_count.saturating_add(1);
        self.last_accessed = Utc::now();
    }

    fn is_expired(&self) -> bool {
        self.summary.is_expired()
    }
}

// ============================================================================
// Friend Activity Cache
// ============================================================================

/// Thread-safe in-memory cache for friend activity summaries
///
/// This cache provides quick access to friend activity summaries for social feeds.
/// It automatically expires old entries and limits total cache size for memory safety.
///
/// # Thread Safety
///
/// The cache uses `RwLock` internally and is safe to share across threads via `Arc`.
#[derive(Debug)]
pub struct FriendActivityCache {
    /// Internal cache storage
    cache: RwLock<HashMap<Uuid, Vec<CacheEntry>>>,
    /// Cache configuration
    config: CacheConfig,
}

/// Configuration for the friend activity cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// TTL for cache entries in minutes
    pub ttl_minutes: i64,
    /// Maximum entries per user
    pub max_entries_per_user: usize,
    /// Maximum total entries
    pub max_total_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_minutes: DEFAULT_CACHE_TTL_MINUTES,
            max_entries_per_user: MAX_ENTRIES_PER_USER,
            max_total_entries: MAX_TOTAL_ENTRIES,
        }
    }
}

impl FriendActivityCache {
    /// Create a new cache with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            config: CacheConfig::default(),
        }
    }

    /// Create a cache with custom configuration
    #[must_use]
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Insert an activity summary into the cache
    ///
    /// If the lock is poisoned, the insertion is silently skipped.
    pub fn insert(&self, summary: FriendActivitySummary) {
        let user_id = summary.user_id;
        let entry = CacheEntry::new(summary);

        let Ok(mut cache) = self.cache.write() else {
            // Lock poisoned - skip insertion, cache is non-critical
            return;
        };

        // Get or create user's entry list
        let entries = cache.entry(user_id).or_default();

        // Remove expired entries first
        entries.retain(|e| !e.is_expired());

        // Enforce per-user limit
        while entries.len() >= self.config.max_entries_per_user {
            // Remove oldest entry
            if !entries.is_empty() {
                entries.remove(0);
            }
        }

        entries.push(entry);

        // Check total cache size
        self.enforce_total_limit(&mut cache);
    }

    /// Get all activity summaries for a friend
    ///
    /// Returns only non-expired entries, sorted by creation time (newest first).
    /// Returns empty if the lock is poisoned.
    #[must_use]
    pub fn get_friend_activities(&self, friend_id: Uuid) -> Vec<FriendActivitySummary> {
        let Ok(mut cache) = self.cache.write() else {
            return Vec::new();
        };

        let Some(entries) = cache.get_mut(&friend_id) else {
            return Vec::new();
        };

        // Update access stats and filter expired
        let mut summaries: Vec<FriendActivitySummary> = entries
            .iter_mut()
            .filter(|e| !e.is_expired())
            .map(|e| {
                e.touch();
                e.summary.clone()
            })
            .collect();

        // Sort by creation time, newest first
        summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        summaries
    }

    /// Get recent activities from multiple friends
    ///
    /// Returns a combined feed of activities from all specified friends,
    /// sorted by creation time (newest first), limited to `max_items`.
    /// Returns empty if the lock is poisoned.
    #[must_use]
    pub fn get_friends_feed(
        &self,
        friend_ids: &[Uuid],
        max_items: usize,
    ) -> Vec<FriendActivitySummary> {
        let Ok(cache) = self.cache.read() else {
            return Vec::new();
        };

        let mut all_summaries: Vec<FriendActivitySummary> = friend_ids
            .iter()
            .filter_map(|id| cache.get(id))
            .flatten()
            .filter(|e| !e.is_expired())
            .map(|e| e.summary.clone())
            .collect();

        // Sort by creation time, newest first
        all_summaries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Limit results
        all_summaries.truncate(max_items);

        all_summaries
    }

    /// Remove all cached activities for a specific user
    ///
    /// Silently skipped if the lock is poisoned.
    pub fn invalidate_user(&self, user_id: Uuid) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(&user_id);
        }
    }

    /// Clear all expired entries from the cache
    ///
    /// Silently skipped if the lock is poisoned.
    pub fn cleanup_expired(&self) {
        let Ok(mut cache) = self.cache.write() else {
            return;
        };

        for entries in cache.values_mut() {
            entries.retain(|e| !e.is_expired());
        }

        // Remove empty user entries
        cache.retain(|_, entries| !entries.is_empty());
    }

    /// Get cache statistics
    ///
    /// Returns zeroed stats if the lock is poisoned.
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        let Ok(cache) = self.cache.read() else {
            return CacheStats {
                total_entries: 0,
                user_count: 0,
                expired_count: 0,
                max_entries: self.config.max_total_entries,
            };
        };

        let total_entries: usize = cache.values().map(Vec::len).sum();
        let user_count = cache.len();

        let expired_count: usize = cache
            .values()
            .flat_map(|entries| entries.iter())
            .filter(|e| e.is_expired())
            .count();

        CacheStats {
            total_entries,
            user_count,
            expired_count,
            max_entries: self.config.max_total_entries,
        }
    }

    /// Clear the entire cache
    ///
    /// Silently skipped if the lock is poisoned.
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Enforce total cache size limit
    fn enforce_total_limit(&self, cache: &mut HashMap<Uuid, Vec<CacheEntry>>) {
        let total_entries: usize = cache.values().map(Vec::len).sum();

        if total_entries > self.config.max_total_entries {
            // Find and remove oldest entries globally
            let mut all_entries: Vec<(Uuid, usize, DateTime<Utc>)> = cache
                .iter()
                .flat_map(|(user_id, entries)| {
                    entries
                        .iter()
                        .enumerate()
                        .map(|(idx, e)| (*user_id, idx, e.summary.created_at))
                })
                .collect();

            // Sort by creation time, oldest first
            all_entries.sort_by_key(|(_, _, created)| *created);

            // Remove oldest entries until under limit
            let to_remove = total_entries - self.config.max_total_entries;
            for (user_id, idx, _) in all_entries.into_iter().take(to_remove) {
                if let Some(entries) = cache.get_mut(&user_id) {
                    if idx < entries.len() {
                        entries.remove(idx);
                    }
                }
            }
        }
    }
}

impl Default for FriendActivityCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of entries
    pub total_entries: usize,
    /// Number of unique users
    pub user_count: usize,
    /// Number of expired entries (pending cleanup)
    pub expired_count: usize,
    /// Maximum allowed entries
    pub max_entries: usize,
}

// ============================================================================
// Thread-Safe Cache Handle
// ============================================================================

/// Thread-safe handle to a friend activity cache
///
/// This type alias provides a convenient way to share the cache across threads.
pub type SharedFriendActivityCache = Arc<FriendActivityCache>;

/// Create a new shared cache
#[must_use]
pub fn create_shared_cache() -> SharedFriendActivityCache {
    Arc::new(FriendActivityCache::new())
}

/// Create a shared cache with custom configuration
#[must_use]
pub fn create_shared_cache_with_config(config: CacheConfig) -> SharedFriendActivityCache {
    Arc::new(FriendActivityCache::with_config(config))
}
