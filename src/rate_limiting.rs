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
    pub fn new() -> Self {
        Self
    }

    /// Calculate rate limit status for an API key
    pub fn calculate_api_key_rate_limit(
        &self,
        api_key: &ApiKey,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        match api_key.tier {
            ApiKeyTier::Enterprise => UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: "enterprise".to_string(),
                auth_method: "api_key".to_string(),
            },
            _ => {
                let limit = api_key.rate_limit_requests;
                let remaining = limit.saturating_sub(current_usage);
                let is_rate_limited = current_usage >= limit;

                UnifiedRateLimitInfo {
                    is_rate_limited,
                    limit: Some(limit),
                    remaining: Some(remaining),
                    reset_at: Some(self.calculate_monthly_reset()),
                    tier: format!("{:?}", api_key.tier).to_lowercase(),
                    auth_method: "api_key".to_string(),
                }
            }
        }
    }

    /// Calculate rate limit status for a JWT token (user)
    pub fn calculate_jwt_rate_limit(
        &self,
        user: &User,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        match user.tier {
            UserTier::Enterprise => UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: "enterprise".to_string(),
                auth_method: "jwt_token".to_string(),
            },
            _ => {
                let limit = user.tier.monthly_limit().unwrap_or(u32::MAX);
                let remaining = limit.saturating_sub(current_usage);
                let is_rate_limited = current_usage >= limit;

                UnifiedRateLimitInfo {
                    is_rate_limited,
                    limit: Some(limit),
                    remaining: Some(remaining),
                    reset_at: Some(self.calculate_monthly_reset()),
                    tier: format!("{:?}", user.tier).to_lowercase(),
                    auth_method: "jwt_token".to_string(),
                }
            }
        }
    }

    /// Calculate rate limit status for a user tier (used for JWT tokens)
    pub fn calculate_user_tier_rate_limit(
        &self,
        tier: &UserTier,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        match tier {
            UserTier::Enterprise => UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: "enterprise".to_string(),
                auth_method: "jwt_token".to_string(),
            },
            _ => {
                let limit = tier.monthly_limit().unwrap_or(u32::MAX);
                let remaining = limit.saturating_sub(current_usage);
                let is_rate_limited = current_usage >= limit;

                UnifiedRateLimitInfo {
                    is_rate_limited,
                    limit: Some(limit),
                    remaining: Some(remaining),
                    reset_at: Some(self.calculate_monthly_reset()),
                    tier: format!("{:?}", tier).to_lowercase(),
                    auth_method: "jwt_token".to_string(),
                }
            }
        }
    }

    /// Convert UserTier to equivalent ApiKeyTier for compatibility
    pub fn user_tier_to_api_key_tier(user_tier: &UserTier) -> ApiKeyTier {
        match user_tier {
            UserTier::Starter => ApiKeyTier::Starter,
            UserTier::Professional => ApiKeyTier::Professional,
            UserTier::Enterprise => ApiKeyTier::Enterprise,
        }
    }

    /// Convert ApiKeyTier to equivalent UserTier for compatibility
    pub fn api_key_tier_to_user_tier(api_key_tier: &ApiKeyTier) -> UserTier {
        match api_key_tier {
            ApiKeyTier::Trial => UserTier::Starter, // Trial maps to Starter for users
            ApiKeyTier::Starter => UserTier::Starter,
            ApiKeyTier::Professional => UserTier::Professional,
            ApiKeyTier::Enterprise => UserTier::Enterprise,
        }
    }

    /// Calculate when the monthly rate limit resets (beginning of next month)
    fn calculate_monthly_reset(&self) -> DateTime<Utc> {
        let now = Utc::now();
        let next_month = if now.month() == 12 {
            now.with_year(now.year() + 1)
                .expect("Failed to set year for next month calculation")
                .with_month(1)
                .expect("Failed to set month to January")
        } else {
            now.with_month(now.month() + 1)
                .expect("Failed to increment month")
        };

        next_month
            .with_day(1)
            .expect("Failed to set day to 1st of month")
            .with_hour(0)
            .expect("Failed to set hour to 0")
            .with_minute(0)
            .expect("Failed to set minute to 0")
            .with_second(0)
            .expect("Failed to set second to 0")
    }

    /// Convert to legacy RateLimitStatus for backward compatibility
    pub fn to_legacy_rate_limit_status(info: &UnifiedRateLimitInfo) -> RateLimitStatus {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api_keys::ApiKeyTier;
    use crate::models::UserTier;
    use chrono::Utc;

    #[test]
    fn test_enterprise_unlimited_api_key() {
        let calculator = UnifiedRateLimitCalculator::new();

        let api_key = ApiKey {
            id: "test".to_string(),
            user_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            key_prefix: "pk_live_test".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Enterprise,
            rate_limit_requests: u32::MAX,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        let info = calculator.calculate_api_key_rate_limit(&api_key, 1_000_000);

        assert!(!info.is_rate_limited);
        assert_eq!(info.limit, None);
        assert_eq!(info.remaining, None);
        assert_eq!(info.tier, "enterprise");
        assert_eq!(info.auth_method, "api_key");
    }

    #[test]
    fn test_starter_tier_rate_limit() {
        let calculator = UnifiedRateLimitCalculator::new();

        let api_key = ApiKey {
            id: "test".to_string(),
            user_id: Uuid::new_v4(),
            name: "Test Key".to_string(),
            key_prefix: "pk_live_test".to_string(),
            key_hash: "hash".to_string(),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        // Under limit
        let info = calculator.calculate_api_key_rate_limit(&api_key, 5_000);
        assert!(!info.is_rate_limited);
        assert_eq!(info.limit, Some(10_000));
        assert_eq!(info.remaining, Some(5_000));
        assert_eq!(info.tier, "starter");

        // At limit
        let info = calculator.calculate_api_key_rate_limit(&api_key, 10_000);
        assert!(info.is_rate_limited);
        assert_eq!(info.remaining, Some(0));

        // Over limit
        let info = calculator.calculate_api_key_rate_limit(&api_key, 15_000);
        assert!(info.is_rate_limited);
        assert_eq!(info.remaining, Some(0)); // Should be 0, not negative
    }

    #[test]
    fn test_user_tier_rate_limit() {
        let calculator = UnifiedRateLimitCalculator::new();

        let user = User {
            id: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            display_name: Some("Test User".to_string()),
            password_hash: "hash".to_string(),
            tier: UserTier::Professional,
            strava_token: None,
            fitbit_token: None,
            created_at: Utc::now(),
            last_active: Utc::now(),
            is_active: true,
        };

        let info = calculator.calculate_jwt_rate_limit(&user, 50_000);

        assert!(!info.is_rate_limited);
        assert_eq!(info.limit, Some(100_000));
        assert_eq!(info.remaining, Some(50_000));
        assert_eq!(info.tier, "professional");
        assert_eq!(info.auth_method, "jwt_token");
    }

    #[test]
    fn test_tier_conversion() {
        // Test UserTier to ApiKeyTier conversion
        assert_eq!(
            UnifiedRateLimitCalculator::user_tier_to_api_key_tier(&UserTier::Starter),
            ApiKeyTier::Starter
        );
        assert_eq!(
            UnifiedRateLimitCalculator::user_tier_to_api_key_tier(&UserTier::Professional),
            ApiKeyTier::Professional
        );
        assert_eq!(
            UnifiedRateLimitCalculator::user_tier_to_api_key_tier(&UserTier::Enterprise),
            ApiKeyTier::Enterprise
        );

        // Test ApiKeyTier to UserTier conversion
        assert_eq!(
            UnifiedRateLimitCalculator::api_key_tier_to_user_tier(&ApiKeyTier::Trial),
            UserTier::Starter
        );
        assert_eq!(
            UnifiedRateLimitCalculator::api_key_tier_to_user_tier(&ApiKeyTier::Starter),
            UserTier::Starter
        );
        assert_eq!(
            UnifiedRateLimitCalculator::api_key_tier_to_user_tier(&ApiKeyTier::Professional),
            UserTier::Professional
        );
        assert_eq!(
            UnifiedRateLimitCalculator::api_key_tier_to_user_tier(&ApiKeyTier::Enterprise),
            UserTier::Enterprise
        );
    }

    #[test]
    fn test_monthly_reset_calculation() {
        let calculator = UnifiedRateLimitCalculator::new();
        let reset_at = calculator.calculate_monthly_reset();
        let now = Utc::now();

        // Reset should be in the future
        assert!(reset_at > now);

        // Reset should be at beginning of day (hour 0, minute 0, second 0)
        assert_eq!(reset_at.hour(), 0);
        assert_eq!(reset_at.minute(), 0);
        assert_eq!(reset_at.second(), 0);

        // Reset should be on the 1st day of some month
        assert_eq!(reset_at.day(), 1);
    }

    #[test]
    fn test_legacy_conversion() {
        let info = UnifiedRateLimitInfo {
            is_rate_limited: true,
            limit: Some(10_000),
            remaining: Some(5_000),
            reset_at: Some(Utc::now()),
            tier: "starter".to_string(),
            auth_method: "api_key".to_string(),
        };

        let legacy = UnifiedRateLimitCalculator::to_legacy_rate_limit_status(&info);

        assert_eq!(legacy.is_rate_limited, info.is_rate_limited);
        assert_eq!(legacy.limit, info.limit);
        assert_eq!(legacy.remaining, info.remaining);
        assert_eq!(legacy.reset_at, info.reset_at);
    }
}
