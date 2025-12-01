// ABOUTME: Cache-related constants for TTL, capacity, and cleanup intervals
// ABOUTME: Supports both in-memory and Redis cache backends with optimal defaults
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

/// Default maximum cache entries for in-memory cache
pub const DEFAULT_CACHE_MAX_ENTRIES: usize = 10_000;

/// Default cleanup interval in seconds for expired entries
pub const DEFAULT_CLEANUP_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Athlete profile cache TTL (24 hours) - profiles change infrequently
pub const TTL_PROFILE_SECS: u64 = 86_400; // 24 hours

/// Activity list cache TTL (15 minutes) - needs to be fresh for new activities
pub const TTL_ACTIVITY_LIST_SECS: u64 = 900; // 15 minutes

/// Individual activity cache TTL (1 hour) - activity details rarely change
pub const TTL_ACTIVITY_SECS: u64 = 3_600; // 1 hour

/// Stats cache TTL (6 hours) - stats aggregate over time windows
pub const TTL_STATS_SECS: u64 = 21_600; // 6 hours

/// Redis connection pool minimum size
pub const REDIS_POOL_MIN_SIZE: usize = 2;

/// Redis connection pool maximum size
pub const REDIS_POOL_MAX_SIZE: usize = 10;

/// Redis connection timeout in seconds
pub const REDIS_CONNECT_TIMEOUT_SECS: u64 = 5;

/// Redis operation timeout in seconds
pub const REDIS_OPERATION_TIMEOUT_SECS: u64 = 3;

/// Cache key prefix for namespacing
pub const CACHE_KEY_PREFIX: &str = "pierre:cache:";
