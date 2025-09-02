// ABOUTME: API key management system for authentication and rate limiting
// ABOUTME: Handles creation, validation, storage, and lifecycle of API keys with tier-based limits
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # API Key Management
//!
//! Provides B2B API key generation, validation, and usage tracking
//! for the Pierre MCP Fitness API platform.

use crate::constants::{
    key_prefixes,
    system_config::{
        PROFESSIONAL_MONTHLY_LIMIT, RATE_LIMIT_WINDOW_SECONDS, STARTER_MONTHLY_LIMIT,
        TRIAL_MONTHLY_LIMIT, TRIAL_PERIOD_DAYS,
    },
    tiers,
};
use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// API Key tiers with rate limits
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyTier {
    Trial,        // 1,000 requests/month, auto-expires in 14 days
    Starter,      // 10,000 requests/month
    Professional, // 100,000 requests/month
    Enterprise,   // Unlimited
}

impl ApiKeyTier {
    #[must_use]
    pub const fn monthly_limit(&self) -> Option<u32> {
        match self {
            Self::Trial => Some(TRIAL_MONTHLY_LIMIT),
            Self::Starter => Some(STARTER_MONTHLY_LIMIT),
            Self::Professional => Some(PROFESSIONAL_MONTHLY_LIMIT),
            Self::Enterprise => None, // Unlimited
        }
    }

    #[must_use]
    pub const fn rate_limit_window(&self) -> u32 {
        RATE_LIMIT_WINDOW_SECONDS // 30 days in seconds
    }

    /// Default expiration in days for trial keys
    #[must_use]
    pub const fn default_trial_days(&self) -> Option<i64> {
        match self {
            Self::Trial => Some(TRIAL_PERIOD_DAYS as i64), // Trial period
            _ => None,
        }
    }

    /// Check if this is a trial tier
    #[must_use]
    pub const fn is_trial(&self) -> bool {
        matches!(self, Self::Trial)
    }

    /// Get string representation for database storage
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Trial => tiers::TRIAL,
            Self::Starter => tiers::STARTER,
            Self::Professional => tiers::PROFESSIONAL,
            Self::Enterprise => tiers::ENTERPRISE,
        }
    }
}

impl std::str::FromStr for ApiKeyTier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            tiers::TRIAL => Ok(Self::Trial),
            tiers::STARTER => Ok(Self::Starter),
            tiers::PROFESSIONAL => Ok(Self::Professional),
            tiers::ENTERPRISE => Ok(Self::Enterprise),
            _ => Err(anyhow::anyhow!("Invalid API key tier: {}", s)),
        }
    }
}

/// API Key model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub user_id: Uuid,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u32,
    pub is_active: bool,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// API Key creation request with rate limit
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub rate_limit_requests: Option<u32>, // 0 = unlimited
    pub expires_in_days: Option<i64>,
}

/// New simplified API Key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequestSimple {
    pub name: String,
    pub description: Option<String>,
    pub rate_limit_requests: u32, // 0 = unlimited
    pub expires_in_days: Option<i64>,
}

/// API Key response (includes the actual key only on creation)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub tier: ApiKeyTier,
    pub key_prefix: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>, // Only included on creation
}

/// Usage record for tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsage {
    pub id: Option<i64>,
    pub api_key_id: String,
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub response_time_ms: Option<u32>,
    pub status_code: u16,
    pub error_message: Option<String>,
    pub request_size_bytes: Option<u32>,
    pub response_size_bytes: Option<u32>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Aggregated usage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsageStats {
    pub api_key_id: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_requests: u32,
    pub successful_requests: u32,
    pub failed_requests: u32,
    pub total_response_time_ms: u64,
    pub tool_usage: serde_json::Value, // JSON object with tool counts
}

/// Rate limit status
#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    pub is_rate_limited: bool,
    pub limit: Option<u32>,
    pub remaining: Option<u32>,
    pub reset_at: Option<DateTime<Utc>>,
}

/// Generated API key data
#[derive(Debug)]
pub struct ApiKeyData {
    pub full_key: String,
    pub key_prefix: String,
    pub key_hash: String,
}

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
            key_prefix: key_prefixes::API_KEY_LIVE, // Production keys
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
            full_key.clone()
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
    pub fn validate_key_format(&self, api_key: &str) -> Result<()> {
        if !api_key.starts_with(self.key_prefix) && !api_key.starts_with("pk_trial_") {
            anyhow::bail!("Invalid API key format");
        }

        let expected_len = if api_key.starts_with("pk_trial_") {
            41 // pk_trial_ (9) + 32 chars
        } else {
            40 // pk_live_ (8) + 32 chars
        };

        if api_key.len() != expected_len {
            anyhow::bail!("Invalid API key length");
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
    ) -> Result<(ApiKey, String)> {
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

    /// Create a new API key (legacy method with tier)
    ///
    /// # Errors
    ///
    /// Returns an error if key creation fails
    pub fn create_api_key(
        &self,
        user_id: Uuid,
        request: CreateApiKeyRequest,
    ) -> Result<(ApiKey, String)> {
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
    ) -> Result<(ApiKey, String)> {
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
    pub fn is_key_valid(&self, api_key: &ApiKey) -> Result<()> {
        if !api_key.is_active {
            anyhow::bail!("API key is inactive");
        }

        if let Some(expires_at) = api_key.expires_at {
            if Utc::now() > expires_at {
                anyhow::bail!("API key has expired");
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
            let now = Utc::now();
            let next_month = if now.month() == 12 {
                now.with_year(now.year() + 1)
                    .and_then(|dt| dt.with_month(1))
                    .unwrap_or_else(|| {
                        tracing::warn!(
                            "Failed to calculate next year/January, using 30-day default"
                        );
                        now + chrono::Duration::days(30)
                    })
            } else {
                now.with_month(now.month() + 1).unwrap_or_else(|| {
                    tracing::warn!("Failed to increment month, using fallback");
                    now + chrono::Duration::days(30)
                })
            };

            let reset_at = next_month
                .with_day(1)
                .and_then(|dt| dt.with_hour(0))
                .and_then(|dt| dt.with_minute(0))
                .and_then(|dt| dt.with_second(0))
                .unwrap_or_else(|| {
                    tracing::warn!(
                        "Failed to set reset time components, using beginning of next month"
                    );
                    next_month
                });

            RateLimitStatus {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(reset_at),
            }
        }
    }
}
