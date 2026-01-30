// ABOUTME: API key management system for authentication and rate limiting
// ABOUTME: Handles creation, validation, storage, and lifecycle of API keys with tier-based limits
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # API Key Management
//!
//! Provides B2B API key generation, validation, and usage tracking
//! for the Pierre MCP Fitness API platform.

use std::fmt::{self, Display, Formatter};
use std::result::Result;
use std::str::FromStr;

use crate::constants::{
    key_prefixes,
    system_config::{
        PROFESSIONAL_MONTHLY_LIMIT, RATE_LIMIT_WINDOW_SECONDS, STARTER_MONTHLY_LIMIT,
        TRIAL_MONTHLY_LIMIT, TRIAL_PERIOD_DAYS,
    },
    tiers,
};
use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Datelike, Duration, Timelike, Utc};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::warn;
use uuid::Uuid;

/// API Key tiers with rate limits
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ApiKeyTier {
    /// Trial tier - 1,000 requests/month, auto-expires in 14 days
    Trial,
    /// Starter tier - 10,000 requests/month
    Starter,
    /// Professional tier - 100,000 requests/month
    Professional,
    /// Enterprise tier - Unlimited requests
    Enterprise,
}

impl Display for ApiKeyTier {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trial => write!(f, "Trial"),
            Self::Starter => write!(f, "Starter"),
            Self::Professional => write!(f, "Professional"),
            Self::Enterprise => write!(f, "Enterprise"),
        }
    }
}

impl ApiKeyTier {
    /// Returns the monthly API request limit for this tier
    #[must_use]
    pub const fn monthly_limit(&self) -> Option<u32> {
        match self {
            Self::Trial => Some(TRIAL_MONTHLY_LIMIT),
            Self::Starter => Some(STARTER_MONTHLY_LIMIT),
            Self::Professional => Some(PROFESSIONAL_MONTHLY_LIMIT),
            Self::Enterprise => None, // Unlimited
        }
    }

    /// Returns the rate limit window duration in seconds
    #[must_use]
    pub const fn rate_limit_window(&self) -> u32 {
        RATE_LIMIT_WINDOW_SECONDS // 30 days in seconds
    }

    /// Default expiration in days for trial keys
    #[must_use]
    pub const fn default_trial_days(&self) -> Option<i64> {
        match self {
            Self::Trial => Some(TRIAL_PERIOD_DAYS), // Trial period
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

impl FromStr for ApiKeyTier {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            tiers::TRIAL => Ok(Self::Trial),
            tiers::STARTER => Ok(Self::Starter),
            tiers::PROFESSIONAL => Ok(Self::Professional),
            tiers::ENTERPRISE => Ok(Self::Enterprise),
            _ => Err(AppError::invalid_input(format!(
                "Invalid API key tier: {s}"
            ))),
        }
    }
}

/// API Key model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Unique identifier for the API key
    pub id: String,
    /// ID of the user who owns this key
    pub user_id: Uuid,
    /// Human-readable name for the key
    pub name: String,
    /// Visible prefix of the key for identification
    pub key_prefix: String,
    /// SHA-256 hash of the full key for verification
    pub key_hash: String,
    /// Optional description of the key's purpose
    pub description: Option<String>,
    /// Tier level determining rate limits
    pub tier: ApiKeyTier,
    /// Maximum requests allowed in the rate limit window
    pub rate_limit_requests: u32,
    /// Rate limit window duration in seconds
    pub rate_limit_window_seconds: u32,
    /// Whether the key is currently active
    pub is_active: bool,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// When the key expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
}

/// API Key creation request with rate limit
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    /// Human-readable name for the key
    pub name: String,
    /// Optional description of the key's purpose
    pub description: Option<String>,
    /// Tier level for the key
    pub tier: ApiKeyTier,
    /// Maximum requests allowed (0 = unlimited)
    pub rate_limit_requests: Option<u32>,
    /// Number of days until expiration
    pub expires_in_days: Option<i64>,
}

/// New simplified API Key creation request
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequestSimple {
    /// Human-readable name for the key
    pub name: String,
    /// Optional description of the key's purpose
    pub description: Option<String>,
    /// Maximum requests allowed (0 = unlimited)
    pub rate_limit_requests: u32,
    /// Number of days until expiration
    pub expires_in_days: Option<i64>,
}

/// API Key response (includes the actual key only on creation)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    /// Unique identifier for the API key
    pub id: String,
    /// Human-readable name for the key
    pub name: String,
    /// Optional description of the key's purpose
    pub description: Option<String>,
    /// Tier level of the key
    pub tier: ApiKeyTier,
    /// Visible prefix for identification
    pub key_prefix: String,
    /// When the key was created
    pub created_at: DateTime<Utc>,
    /// When the key expires (if set)
    pub expires_at: Option<DateTime<Utc>>,
    /// The actual API key (only included on creation, never shown again)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// Usage record for tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsage {
    /// Unique identifier for this usage record
    pub id: Option<i64>,
    /// ID of the API key that was used
    pub api_key_id: String,
    /// When the request was made
    pub timestamp: DateTime<Utc>,
    /// Name of the tool/endpoint that was called
    pub tool_name: String,
    /// Response time in milliseconds
    pub response_time_ms: Option<u32>,
    /// HTTP status code returned
    pub status_code: u16,
    /// Error message if request failed
    pub error_message: Option<String>,
    /// Size of the request payload in bytes
    pub request_size_bytes: Option<u32>,
    /// Size of the response payload in bytes
    pub response_size_bytes: Option<u32>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent string
    pub user_agent: Option<String>,
}

/// Aggregated usage statistics
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyUsageStats {
    /// ID of the API key
    pub api_key_id: String,
    /// Start of the statistics period
    pub period_start: DateTime<Utc>,
    /// End of the statistics period
    pub period_end: DateTime<Utc>,
    /// Total number of requests made
    pub total_requests: u32,
    /// Number of successful requests (2xx status)
    pub successful_requests: u32,
    /// Number of failed requests (4xx/5xx status)
    pub failed_requests: u32,
    /// Total response time in milliseconds
    pub total_response_time_ms: u64,
    /// JSON object mapping tool names to usage counts
    pub tool_usage: serde_json::Value,
}

/// Rate limit status
#[derive(Debug, Serialize)]
pub struct RateLimitStatus {
    /// Whether the key is currently rate limited
    pub is_rate_limited: bool,
    /// Maximum requests allowed in the window
    pub limit: Option<u32>,
    /// Remaining requests in the current window
    pub remaining: Option<u32>,
    /// When the rate limit window resets
    pub reset_at: Option<DateTime<Utc>>,
}

/// Generated API key data
#[derive(Debug)]
pub struct ApiKeyData {
    /// The full API key (shown only once)
    pub full_key: String,
    /// Visible prefix for identification
    pub key_prefix: String,
    /// SHA-256 hash of the full key
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

    /// Create a new API key (legacy method with tier)
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
