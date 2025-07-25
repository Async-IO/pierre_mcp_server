// ABOUTME: Rate limiting engine for API request throttling and quota enforcement
// ABOUTME: Implements token bucket algorithm with configurable limits per API key tier
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Unified Rate Limiting System
//!
//! This module provides a unified rate limiting system that works for both
//! API keys and JWT tokens, using the same logic and limits across all
//! authentication methods.

use crate::api_keys::{ApiKey, ApiKeyTier, RateLimitStatus};
use crate::models::{User, UserTier};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// JWT token usage record for tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtUsage {
    pub id: Option<i64>,
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub endpoint: String,
    pub method: String,
    pub status_code: u16,
    pub response_time_ms: Option<u32>,
    pub request_size_bytes: Option<u32>,
    pub response_size_bytes: Option<u32>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

/// Rate limit information for any authentication method
#[derive(Debug, Clone, Serialize)]
pub struct UnifiedRateLimitInfo {
    /// Whether the request is rate limited
    pub is_rate_limited: bool,
    /// Maximum requests allowed in the current period
    pub limit: Option<u32>,
    /// Remaining requests in the current period
    pub remaining: Option<u32>,
    /// When the current rate limit period resets
    pub reset_at: Option<DateTime<Utc>>,
    /// The tier associated with this rate limit
    pub tier: String,
    /// The authentication method used
    pub auth_method: String,
}

/// Unified rate limit calculator
#[derive(Clone)]
pub struct UnifiedRateLimitCalculator;

impl UnifiedRateLimitCalculator {
    /// Create a new unified rate limit calculator
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Calculate rate limit status for an API key
    #[must_use]
    pub fn calculate_api_key_rate_limit(
        &self,
        api_key: &ApiKey,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        if api_key.tier == ApiKeyTier::Enterprise {
            UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: "enterprise".into(),
                auth_method: "api_key".into(),
            }
        } else {
            let limit = api_key.rate_limit_requests;
            let remaining = limit.saturating_sub(current_usage);
            let is_rate_limited = current_usage >= limit;

            UnifiedRateLimitInfo {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(Self::calculate_monthly_reset()),
                tier: format!("{:?}", api_key.tier).to_lowercase(),
                auth_method: "api_key".into(),
            }
        }
    }

    /// Calculate rate limit status for a JWT token (user)
    #[must_use]
    pub fn calculate_jwt_rate_limit(
        &self,
        user: &User,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        if user.tier == UserTier::Enterprise {
            UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: "enterprise".into(),
                auth_method: "jwt_token".into(),
            }
        } else {
            let limit = user.tier.monthly_limit().unwrap_or(u32::MAX);
            let remaining = limit.saturating_sub(current_usage);
            let is_rate_limited = current_usage >= limit;

            UnifiedRateLimitInfo {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(Self::calculate_monthly_reset()),
                tier: format!("{:?}", user.tier).to_lowercase(),
                auth_method: "jwt_token".into(),
            }
        }
    }

    /// Calculate rate limit status for a user tier (used for JWT tokens)
    #[must_use]
    pub fn calculate_user_tier_rate_limit(
        &self,
        tier: &UserTier,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        if *tier == UserTier::Enterprise {
            UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: "enterprise".into(),
                auth_method: "jwt_token".into(),
            }
        } else {
            let limit = tier.monthly_limit().unwrap_or(u32::MAX);
            let remaining = limit.saturating_sub(current_usage);
            let is_rate_limited = current_usage >= limit;

            UnifiedRateLimitInfo {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(Self::calculate_monthly_reset()),
                tier: format!("{tier:?}").to_lowercase(),
                auth_method: "jwt_token".into(),
            }
        }
    }

    /// Convert `UserTier` to equivalent `ApiKeyTier` for compatibility
    #[must_use]
    pub const fn user_tier_to_api_key_tier(user_tier: &UserTier) -> ApiKeyTier {
        match user_tier {
            UserTier::Starter => ApiKeyTier::Starter,
            UserTier::Professional => ApiKeyTier::Professional,
            UserTier::Enterprise => ApiKeyTier::Enterprise,
        }
    }

    /// Convert `ApiKeyTier` to equivalent `UserTier` for compatibility
    #[must_use]
    pub const fn api_key_tier_to_user_tier(api_key_tier: &ApiKeyTier) -> UserTier {
        match api_key_tier {
            ApiKeyTier::Trial | ApiKeyTier::Starter => UserTier::Starter, // Trial maps to Starter for users
            ApiKeyTier::Professional => UserTier::Professional,
            ApiKeyTier::Enterprise => UserTier::Enterprise,
        }
    }

    /// Calculate when the monthly rate limit resets (beginning of next month)
    fn calculate_monthly_reset() -> DateTime<Utc> {
        let now = Utc::now();
        let next_month = if now.month() == 12 {
            now.with_year(now.year() + 1)
                .and_then(|dt| dt.with_month(1))
                .unwrap_or_else(|| {
                    tracing::warn!("Failed to calculate next year/January, using fallback");
                    now + chrono::Duration::days(31)
                })
        } else {
            now.with_month(now.month() + 1).unwrap_or_else(|| {
                tracing::warn!("Failed to increment month, using fallback");
                now + chrono::Duration::days(31)
            })
        };

        next_month
            .with_day(1)
            .and_then(|dt| dt.with_hour(0))
            .and_then(|dt| dt.with_minute(0))
            .and_then(|dt| dt.with_second(0))
            .unwrap_or_else(|| {
                tracing::warn!("Failed to set reset time components, using next month");
                next_month
            })
    }

    /// Convert to legacy `RateLimitStatus` for backward compatibility
    #[must_use]
    pub const fn to_legacy_rate_limit_status(info: &UnifiedRateLimitInfo) -> RateLimitStatus {
        RateLimitStatus {
            is_rate_limited: info.is_rate_limited,
            limit: info.limit,
            remaining: info.remaining,
            reset_at: info.reset_at,
        }
    }
}

impl Default for UnifiedRateLimitCalculator {
    fn default() -> Self {
        Self::new()
    }
}
