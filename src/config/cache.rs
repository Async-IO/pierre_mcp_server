// ABOUTME: Cache and rate limiting configuration types
// ABOUTME: Handles Redis connections, cache TTLs, and rate limiting settings
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::{cache, oauth_rate_limiting, rate_limiting_bursts, redis, system_config};
use serde::{Deserialize, Serialize};
use std::env;

/// Cache configuration for Redis and in-memory caching
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    /// Redis URL for distributed caching (optional)
    #[serde(default)]
    pub redis_url: Option<String>,
    /// Maximum number of entries in local cache
    #[serde(default)]
    pub max_entries: usize,
    /// Cache cleanup interval in seconds
    #[serde(default)]
    pub cleanup_interval_secs: u64,
    /// Redis connection configuration
    #[serde(default)]
    pub redis_connection: RedisConnectionConfig,
    /// Cache TTL configuration
    #[serde(default)]
    pub ttl: CacheTtlConfig,
}

impl CacheConfig {
    /// Load cache configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            redis_url: env::var("REDIS_URL").ok(),
            max_entries: env::var("CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1000),
            cleanup_interval_secs: env::var("CACHE_CLEANUP_INTERVAL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            redis_connection: RedisConnectionConfig::from_env(),
            ttl: CacheTtlConfig::from_env(),
        }
    }
}

/// Cache TTL configuration for different resource types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTtlConfig {
    /// Athlete profile cache TTL in seconds (default: 24 hours)
    pub profile_secs: u64,
    /// Activity list cache TTL in seconds (default: 15 minutes)
    pub activity_list_secs: u64,
    /// Individual activity cache TTL in seconds (default: 1 hour)
    pub activity_secs: u64,
    /// Stats cache TTL in seconds (default: 6 hours)
    pub stats_secs: u64,
}

impl Default for CacheTtlConfig {
    fn default() -> Self {
        Self {
            profile_secs: cache::TTL_PROFILE_SECS,
            activity_list_secs: cache::TTL_ACTIVITY_LIST_SECS,
            activity_secs: cache::TTL_ACTIVITY_SECS,
            stats_secs: cache::TTL_STATS_SECS,
        }
    }
}

impl CacheTtlConfig {
    /// Load cache TTL configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            profile_secs: env::var("CACHE_TTL_PROFILE_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(cache::TTL_PROFILE_SECS),
            activity_list_secs: env::var("CACHE_TTL_ACTIVITY_LIST_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(cache::TTL_ACTIVITY_LIST_SECS),
            activity_secs: env::var("CACHE_TTL_ACTIVITY_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(cache::TTL_ACTIVITY_SECS),
            stats_secs: env::var("CACHE_TTL_STATS_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(cache::TTL_STATS_SECS),
        }
    }
}

/// Redis connection and retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConnectionConfig {
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Response/command timeout in seconds
    pub response_timeout_secs: u64,
    /// Number of reconnection retries after connection drop
    pub reconnection_retries: usize,
    /// Exponential backoff base for retry delays
    pub retry_exponent_base: u64,
    /// Maximum retry delay in milliseconds
    pub max_retry_delay_ms: u64,
    /// Number of retries for initial connection at startup
    pub initial_connection_retries: u32,
    /// Initial retry delay in milliseconds (doubles with exponential backoff)
    pub initial_retry_delay_ms: u64,
}

impl Default for RedisConnectionConfig {
    fn default() -> Self {
        Self {
            connection_timeout_secs: redis::CONNECTION_TIMEOUT_SECS,
            response_timeout_secs: redis::RESPONSE_TIMEOUT_SECS,
            reconnection_retries: redis::RECONNECTION_RETRIES,
            retry_exponent_base: redis::RETRY_EXPONENT_BASE,
            max_retry_delay_ms: redis::MAX_RETRY_DELAY_MS,
            initial_connection_retries: redis::INITIAL_CONNECTION_RETRIES,
            initial_retry_delay_ms: 500, // Same as database default
        }
    }
}

impl RedisConnectionConfig {
    /// Load Redis connection configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            connection_timeout_secs: env::var("REDIS_CONNECTION_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(redis::CONNECTION_TIMEOUT_SECS),
            response_timeout_secs: env::var("REDIS_RESPONSE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(redis::RESPONSE_TIMEOUT_SECS),
            reconnection_retries: env::var("REDIS_RECONNECTION_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(redis::RECONNECTION_RETRIES),
            retry_exponent_base: env::var("REDIS_RETRY_EXPONENT_BASE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(redis::RETRY_EXPONENT_BASE),
            max_retry_delay_ms: env::var("REDIS_MAX_RETRY_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(redis::MAX_RETRY_DELAY_MS),
            initial_connection_retries: env::var("REDIS_INITIAL_CONNECTION_RETRIES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(redis::INITIAL_CONNECTION_RETRIES),
            initial_retry_delay_ms: env::var("REDIS_INITIAL_RETRY_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500),
        }
    }
}

/// Rate limiting configuration for tier-based request throttling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Free tier burst limit
    pub free_tier_burst: u32,
    /// Professional tier burst limit
    pub professional_burst: u32,
    /// Enterprise tier burst limit
    pub enterprise_burst: u32,
    /// OAuth authorize endpoint rate limit (requests per minute)
    pub oauth_authorize_rpm: u32,
    /// OAuth token endpoint rate limit (requests per minute)
    pub oauth_token_rpm: u32,
    /// OAuth register endpoint rate limit (requests per minute)
    pub oauth_register_rpm: u32,
    /// Rate limit window duration in seconds
    pub rate_limit_window_secs: u64,
    /// Rate limiter cleanup threshold
    pub cleanup_threshold: usize,
    /// Stale entry timeout in seconds
    pub stale_entry_timeout_secs: u64,
    /// Admin-provisioned API key default monthly request limit
    pub admin_provisioned_api_key_monthly_limit: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            free_tier_burst: rate_limiting_bursts::FREE_TIER_BURST,
            professional_burst: rate_limiting_bursts::PROFESSIONAL_BURST,
            enterprise_burst: rate_limiting_bursts::ENTERPRISE_BURST,
            oauth_authorize_rpm: oauth_rate_limiting::AUTHORIZE_RPM,
            oauth_token_rpm: oauth_rate_limiting::TOKEN_RPM,
            oauth_register_rpm: oauth_rate_limiting::REGISTER_RPM,
            rate_limit_window_secs: oauth_rate_limiting::WINDOW_SECS,
            cleanup_threshold: oauth_rate_limiting::CLEANUP_THRESHOLD,
            stale_entry_timeout_secs: oauth_rate_limiting::STALE_ENTRY_TIMEOUT_SECS,
            admin_provisioned_api_key_monthly_limit: system_config::STARTER_MONTHLY_LIMIT,
        }
    }
}

impl RateLimitConfig {
    /// Load rate limiting configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            free_tier_burst: env::var("RATE_LIMIT_FREE_TIER_BURST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(rate_limiting_bursts::FREE_TIER_BURST),
            professional_burst: env::var("RATE_LIMIT_PROFESSIONAL_BURST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(rate_limiting_bursts::PROFESSIONAL_BURST),
            enterprise_burst: env::var("RATE_LIMIT_ENTERPRISE_BURST")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(rate_limiting_bursts::ENTERPRISE_BURST),
            oauth_authorize_rpm: env::var("OAUTH_AUTHORIZE_RATE_LIMIT_RPM")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(oauth_rate_limiting::AUTHORIZE_RPM),
            oauth_token_rpm: env::var("OAUTH_TOKEN_RATE_LIMIT_RPM")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(oauth_rate_limiting::TOKEN_RPM),
            oauth_register_rpm: env::var("OAUTH_REGISTER_RATE_LIMIT_RPM")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(oauth_rate_limiting::REGISTER_RPM),
            rate_limit_window_secs: env::var("OAUTH2_RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(oauth_rate_limiting::WINDOW_SECS),
            cleanup_threshold: env::var("RATE_LIMITER_CLEANUP_THRESHOLD")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(oauth_rate_limiting::CLEANUP_THRESHOLD),
            stale_entry_timeout_secs: env::var("RATE_LIMITER_STALE_ENTRY_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(oauth_rate_limiting::STALE_ENTRY_TIMEOUT_SECS),
            admin_provisioned_api_key_monthly_limit: env::var("PIERRE_ADMIN_API_KEY_MONTHLY_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(system_config::STARTER_MONTHLY_LIMIT),
        }
    }
}
