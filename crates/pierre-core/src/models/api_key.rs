// ABOUTME: API key types for authentication and rate limiting
// ABOUTME: ApiKeyTier, ApiKey, usage tracking, and request/response DTOs
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::fmt::{self, Display, Formatter};
use std::result::Result;
use std::str::FromStr;

use crate::constants::{
    system_config::{
        PROFESSIONAL_MONTHLY_LIMIT, RATE_LIMIT_WINDOW_SECONDS, STARTER_MONTHLY_LIMIT,
        TRIAL_MONTHLY_LIMIT, TRIAL_PERIOD_DAYS,
    },
    tiers,
};
use crate::errors::AppError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

/// Simplified API Key creation request
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
