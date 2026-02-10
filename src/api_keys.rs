// ABOUTME: API key management system for authentication and rate limiting
// ABOUTME: Handles creation, validation, storage, and lifecycle of API keys with tier-based limits
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # API Key Management
//!
//! Provides B2B API key generation, validation, and usage tracking
//! for the Pierre MCP Fitness API platform.

use crate::constants::key_prefixes;
use crate::errors::{AppError, AppResult};
use chrono::{Datelike, Duration, Timelike, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use tracing::warn;
use uuid::Uuid;

// Re-export DTOs from pierre-core (canonical definitions)
pub use pierre_core::models::{
    ApiKey, ApiKeyData, ApiKeyResponse, ApiKeyTier, ApiKeyUsage, ApiKeyUsageStats,
    CreateApiKeyRequest, CreateApiKeyRequestSimple, RateLimitStatus,
};

/// API Key Manager
#[derive(Clone)]
pub struct ApiKeyManager {
    key_prefix: &'static str,
}

impl Default for ApiKeyManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiKeyManager {
    /// Create a new API key manager
    #[must_use]
    pub const fn new() -> Self {
        Self {
            key_prefix: key_prefixes::LIVE, // Production keys
        }
    }

    /// Generate a new API key with optional trial prefix
    pub fn generate_api_key(&self, is_trial: bool) -> ApiKeyData {
        // Generate 32 random bytes for the key
        let random_bytes: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        // Full key format: pk_live_<32 random chars> or pk_trial_<32 random chars>
        let prefix = if is_trial {
            "pk_trial_"
        } else {
            self.key_prefix
        };
        let full_key = format!("{prefix}{random_bytes}");

        // Create key prefix for identification (first 12 chars)
        // More efficient: use string slicing instead of collecting chars
        let key_prefix = if full_key.len() >= 12 {
            full_key[..12].to_string()
        } else {
            full_key.clone() // Safe: String ownership for API key display
        };

        // Hash the full key for storage
        let mut hasher = Sha256::new();
        hasher.update(full_key.as_bytes());
        let key_hash = format!("{:x}", hasher.finalize());

        ApiKeyData {
            full_key,
            key_prefix,
            key_hash,
        }
    }

    /// Validate an API key format
    ///
    /// # Errors
    ///
    /// Returns an error if the API key format is invalid or has incorrect length
    pub fn validate_key_format(&self, api_key: &str) -> AppResult<()> {
        if !api_key.starts_with(self.key_prefix) && !api_key.starts_with("pk_trial_") {
            return Err(AppError::invalid_input("Invalid API key format"));
        }

        let expected_len = if api_key.starts_with("pk_trial_") {
            41 // pk_trial_ (9) + 32 chars
        } else {
            40 // pk_live_ (8) + 32 chars
        };

        if api_key.len() != expected_len {
            return Err(AppError::invalid_input("Invalid API key length"));
        }

        Ok(())
    }

    /// Extract key prefix from full key
    #[must_use]
    pub fn extract_key_prefix(&self, api_key: &str) -> String {
        api_key.chars().take(12).collect()
    }

    /// Hash an API key for comparison
    #[must_use]
    pub fn hash_key(&self, api_key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(api_key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check if an API key string is a trial key
    #[must_use]
    pub fn is_trial_key(&self, api_key: &str) -> bool {
        api_key.starts_with("pk_trial_")
    }

    /// Create a new API key with simplified request
    ///
    /// # Errors
    ///
    /// Returns an error if key creation fails
    pub fn create_api_key_simple(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequestSimple,
    ) -> AppResult<(ApiKey, String)> {
        // Determine tier based on rate limit (keep trial functionality but don't expose in UI)
        let tier = if request.rate_limit_requests <= 1_000 {
            ApiKeyTier::Trial
        } else if request.rate_limit_requests <= 10_000 {
            ApiKeyTier::Starter
        } else if request.rate_limit_requests <= 100_000 {
            ApiKeyTier::Professional
        } else {
            ApiKeyTier::Enterprise
        };

        let is_trial = tier.is_trial();

        // Generate the key components
        let api_key_data = self.generate_api_key(is_trial);
        let full_key = api_key_data.full_key;
        let key_prefix = api_key_data.key_prefix;
        let key_hash = api_key_data.key_hash;

        // Calculate expiration
        let expires_at = if is_trial {
            let days = request
                .expires_in_days
                .or_else(|| tier.default_trial_days())
                .unwrap_or(14);
            Some(Utc::now() + Duration::days(days))
        } else {
            request
                .expires_in_days
                .map(|days| Utc::now() + Duration::days(days))
        };

        // Use custom rate limits
        let rate_limit_requests = if request.rate_limit_requests == 0 {
            1_000_000_000 // Effectively unlimited but fits in database constraints
        } else {
            request.rate_limit_requests
        };
        let rate_limit_window = tier.rate_limit_window();

        // Create the API key record
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id,
            name: request.name,
            key_prefix,
            key_hash,
            description: request.description,
            tier,
            rate_limit_requests,
            rate_limit_window_seconds: rate_limit_window,
            is_active: true,
            last_used_at: None,
            expires_at,
            created_at: Utc::now(),
        };

        Ok((api_key, full_key))
    }

    /// Create a new API key with tier-based access
    ///
    /// # Errors
    ///
    /// Returns an error if key creation fails
    pub fn create_api_key(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequest,
    ) -> AppResult<(ApiKey, String)> {
        // Check if this is a trial key
        let is_trial = request.tier.is_trial();

        // Generate the key components
        let api_key_data = self.generate_api_key(is_trial);
        let full_key = api_key_data.full_key;
        let key_prefix = api_key_data.key_prefix;
        let key_hash = api_key_data.key_hash;

        // Calculate expiration
        // For trial keys, use default trial days if not specified
        let expires_at = if is_trial {
            let days = request
                .expires_in_days
                .or_else(|| request.tier.default_trial_days())
                .unwrap_or(14);
            Some(Utc::now() + Duration::days(days))
        } else {
            request
                .expires_in_days
                .map(|days| Utc::now() + Duration::days(days))
        };

        // Get rate limits - use custom if provided, otherwise use tier defaults
        // For enterprise tier, use a high value that fits in database constraints
        let rate_limit_requests = request
            .rate_limit_requests
            .unwrap_or_else(|| request.tier.monthly_limit().unwrap_or(1_000_000_000));
        let rate_limit_window = request.tier.rate_limit_window();

        // Create the API key record
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id,
            name: request.name,
            key_prefix,
            key_hash,
            description: request.description,
            tier: request.tier,
            rate_limit_requests,
            rate_limit_window_seconds: rate_limit_window,
            is_active: true,
            last_used_at: None,
            expires_at,
            created_at: Utc::now(),
        };

        Ok((api_key, full_key))
    }

    /// Create a trial API key with default settings
    ///
    /// # Errors
    ///
    /// Returns an error if key creation fails
    pub fn create_trial_key(
        &self,
        user_id: Uuid,
        name: String,
        description: Option<String>,
    ) -> AppResult<(ApiKey, String)> {
        let request = CreateApiKeyRequest {
            name,
            description,
            tier: ApiKeyTier::Trial,
            rate_limit_requests: None, // Use tier default
            expires_in_days: None,     // Will use default 14 days
        };

        self.create_api_key(user_id, request)
    }

    /// Check if a key is valid and active
    ///
    /// # Errors
    ///
    /// Returns an error if the API key is inactive or expired
    pub fn is_key_valid(&self, api_key: &ApiKey) -> AppResult<()> {
        if !api_key.is_active {
            return Err(AppError::invalid_input("API key is inactive"));
        }

        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                return Err(AppError::invalid_input("API key has expired"));
            }
        }

        Ok(())
    }

    /// Get rate limit status for an API key
    #[must_use]
    pub fn rate_limit_status(&self, api_key: &ApiKey, current_usage: u32) -> RateLimitStatus {
        if api_key.tier == ApiKeyTier::Enterprise {
            RateLimitStatus {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
            }
        } else {
            let limit = api_key.rate_limit_requests;
            let remaining = limit.saturating_sub(current_usage);
            let is_rate_limited = current_usage >= limit;

            // Calculate reset time (beginning of next month)
            // Must set day to 1 BEFORE changing month to avoid invalid dates
            // (e.g., Jan 29 -> Feb 29 fails in non-leap years)
            let now = Utc::now();
            let first_of_current = now
                .with_day(1)
                .and_then(|dt| dt.with_hour(0))
                .and_then(|dt| dt.with_minute(0))
                .and_then(|dt| dt.with_second(0))
                .and_then(|dt| dt.with_nanosecond(0))
                .unwrap_or(now);

            let reset_at = if now.month() == 12 {
                first_of_current
                    .with_year(now.year() + 1)
                    .and_then(|dt| dt.with_month(1))
                    .unwrap_or_else(|| {
                        warn!("Failed to calculate next year/January, using 30-day default");
                        now + chrono::Duration::days(30)
                    })
            } else {
                first_of_current
                    .with_month(now.month() + 1)
                    .unwrap_or_else(|| {
                        warn!("Failed to increment month, using fallback");
                        now + chrono::Duration::days(30)
                    })
            };

            RateLimitStatus {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(reset_at),
            }
        }
    }
}
