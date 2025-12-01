// ABOUTME: OAuth2 endpoint rate limiting with RFC-compliant headers and rejection handling
// ABOUTME: Implements per-IP token bucket rate limiting for authorization, token, and registration endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use dashmap::DashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `OAuth2` rate limiter with per-IP tracking using sharded concurrent `HashMap`
/// Uses `DashMap` for fine-grained locking instead of global `Mutex` to reduce contention
#[derive(Clone)]
pub struct OAuth2RateLimiter {
    /// Per-IP request tracking: IP -> (`request_count`, `window_start`)
    /// `DashMap` provides lock-free read operations and sharded write operations
    state: Arc<DashMap<IpAddr, (u32, Instant)>>,
    config: crate::rate_limiting::OAuth2RateLimitConfig,
    /// Rate limit configuration for window and cleanup values
    rate_limit_config: crate::config::environment::RateLimitConfig,
}

impl OAuth2RateLimiter {
    /// Create new `OAuth2` rate limiter with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(DashMap::new()),
            config: crate::rate_limiting::OAuth2RateLimitConfig::new(),
            rate_limit_config: crate::config::environment::RateLimitConfig::default(),
        }
    }

    /// Create `OAuth2` rate limiter from `RateLimitConfig`
    #[must_use]
    pub fn from_rate_limit_config(
        rate_config: crate::config::environment::RateLimitConfig,
    ) -> Self {
        Self {
            state: Arc::new(DashMap::new()),
            config: crate::rate_limiting::OAuth2RateLimitConfig::from_rate_limit_config(
                &rate_config,
            ),
            rate_limit_config: rate_config,
        }
    }

    /// Create `OAuth2` rate limiter with custom configuration
    #[must_use]
    pub fn with_config(config: crate::rate_limiting::OAuth2RateLimitConfig) -> Self {
        Self {
            state: Arc::new(DashMap::new()),
            config,
            rate_limit_config: crate::config::environment::RateLimitConfig::default(),
        }
    }

    /// Check rate limit for a specific endpoint and IP
    /// Uses `DashMap` entry API for atomic read-modify-write operations
    #[must_use]
    pub fn check_rate_limit(
        &self,
        endpoint: &str,
        client_ip: IpAddr,
    ) -> crate::rate_limiting::OAuth2RateLimitStatus {
        let limit = self.config.get_limit(endpoint);
        let now = Instant::now();
        let window = Duration::from_secs(self.rate_limit_config.rate_limit_window_secs);

        // Use DashMap entry API for atomic operation without full lock
        let mut entry = self.state.entry(client_ip).or_insert((0, now));
        let (count, window_start) = entry.value_mut();

        // Reset window if expired
        if now.duration_since(*window_start) >= window {
            *count = 0;
            *window_start = now;
        }

        let remaining = limit.saturating_sub(*count);
        let is_limited = *count >= limit;

        // Increment count if not limited
        if !is_limited {
            *count += 1;
        }

        let result_window_start = *window_start;
        drop(entry); // Explicitly drop entry guard to release lock

        // Lazy cleanup: only run if map is growing
        // This avoids holding locks during cleanup on critical path
        if self.state.len() > self.rate_limit_config.cleanup_threshold {
            self.cleanup_old_entries(now);
        }

        // Calculate reset time (convert Instant to Unix timestamp)
        let now_system = std::time::SystemTime::now();
        let elapsed_from_window_start = now.duration_since(result_window_start);
        let reset_system = now_system + (window - elapsed_from_window_start);
        #[allow(clippy::cast_possible_wrap)]
        // Safe: Unix timestamps fit in i64 range for next several centuries
        let reset_at = reset_system
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs() as i64;

        crate::rate_limiting::OAuth2RateLimitStatus {
            is_limited,
            limit,
            remaining,
            reset_at,
            retry_after_seconds: None,
        }
        .with_retry_after()
    }

    /// Remove stale entries older than configured timeout from rate limit state
    /// Called lazily when map size exceeds threshold to avoid contention
    fn cleanup_old_entries(&self, now: Instant) {
        self.state.retain(|_ip, (_count, start)| {
            now.duration_since(*start)
                < Duration::from_secs(self.rate_limit_config.stale_entry_timeout_secs)
        });
    }
}

impl Default for OAuth2RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}
