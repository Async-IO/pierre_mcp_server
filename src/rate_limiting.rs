// ABOUTME: Rate limiting engine for API request throttling and quota enforcement
// ABOUTME: Implements token bucket algorithm with configurable limits per API key tier
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Unified Rate Limiting System
//!
//! This module provides a unified rate limiting system that works for both
//! API keys and JWT tokens, using the same logic and limits across all
//! authentication methods.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tracing::warn;
use uuid::Uuid;

use crate::api_keys::{ApiKey, ApiKeyTier};
use crate::config::environment::RateLimitConfig;
use crate::constants::tiers;
use crate::models::TenantId;
use crate::models::{Tenant, User, UserTier};

/// JWT token usage record for tracking
#[derive(Debug, Serialize, Deserialize)]
pub struct JwtUsage {
    /// Unique identifier for this usage record
    pub id: Option<i64>,
    /// ID of the user who made the request
    pub user_id: Uuid,
    /// When the request was made
    pub timestamp: DateTime<Utc>,
    /// API endpoint that was accessed
    pub endpoint: String,
    /// HTTP method used (GET, POST, etc.)
    pub method: String,
    /// HTTP status code returned
    pub status_code: u16,
    /// Response time in milliseconds
    pub response_time_ms: Option<u32>,
    /// Request payload size in bytes
    pub request_size_bytes: Option<u32>,
    /// Response payload size in bytes
    pub response_size_bytes: Option<u32>,
    /// Client IP address
    pub ip_address: Option<String>,
    /// Client user agent string
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

/// Tenant-specific rate limit tier configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantRateLimitTier {
    /// Base monthly request limit
    pub monthly_limit: u32,
    /// Requests per minute burst limit
    pub burst_limit: u32,
    /// Rate limit multiplier for this tenant (1.0 = normal, 2.0 = double)
    pub multiplier: f32,
    /// Whether tenant has unlimited requests
    pub unlimited: bool,
    /// Custom reset period in seconds (None = monthly)
    pub custom_reset_period: Option<u64>,
}

/// Monthly request limit for starter tier tenants
pub const TENANT_STARTER_LIMIT: u32 = 10_000;
/// Monthly request limit for professional tier tenants
pub const TENANT_PROFESSIONAL_LIMIT: u32 = 100_000;
/// Monthly request limit for enterprise tier tenants
pub const TENANT_ENTERPRISE_LIMIT: u32 = 1_000_000;

impl TenantRateLimitTier {
    /// Create tier configuration for starter tenants
    #[must_use]
    pub const fn starter() -> Self {
        use crate::constants::rate_limiting_bursts;
        Self {
            monthly_limit: TENANT_STARTER_LIMIT,
            burst_limit: rate_limiting_bursts::FREE_TIER_BURST,
            multiplier: 1.0,
            unlimited: false,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for professional tenants
    #[must_use]
    pub const fn professional() -> Self {
        use crate::constants::rate_limiting_bursts;
        Self {
            monthly_limit: TENANT_PROFESSIONAL_LIMIT,
            burst_limit: rate_limiting_bursts::PROFESSIONAL_BURST,
            multiplier: 1.0,
            unlimited: false,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for enterprise tenants
    #[must_use]
    pub const fn enterprise() -> Self {
        use crate::constants::rate_limiting_bursts;
        Self {
            monthly_limit: TENANT_ENTERPRISE_LIMIT,
            burst_limit: rate_limiting_bursts::ENTERPRISE_BURST,
            multiplier: 1.0,
            unlimited: true,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for starter tenants from config
    #[must_use]
    pub const fn starter_from_config(config: &RateLimitConfig) -> Self {
        Self {
            monthly_limit: TENANT_STARTER_LIMIT,
            burst_limit: config.free_tier_burst,
            multiplier: 1.0,
            unlimited: false,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for professional tenants from config
    #[must_use]
    pub const fn professional_from_config(config: &RateLimitConfig) -> Self {
        Self {
            monthly_limit: TENANT_PROFESSIONAL_LIMIT,
            burst_limit: config.professional_burst,
            multiplier: 1.0,
            unlimited: false,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for enterprise tenants from config
    #[must_use]
    pub const fn enterprise_from_config(config: &RateLimitConfig) -> Self {
        Self {
            monthly_limit: TENANT_ENTERPRISE_LIMIT,
            burst_limit: config.enterprise_burst,
            multiplier: 1.0,
            unlimited: true,
            custom_reset_period: None,
        }
    }

    /// Create custom tier configuration
    #[must_use]
    pub const fn custom(
        monthly_limit: u32,
        burst_limit: u32,
        multiplier: f32,
        unlimited: bool,
    ) -> Self {
        Self {
            monthly_limit,
            burst_limit,
            multiplier,
            unlimited,
            custom_reset_period: None,
        }
    }

    /// Apply multiplier to get effective monthly limit
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    /// Returns the effective monthly limit after applying the tier multiplier
    // Safe: multiplier values are controlled and positive, result fits in u32 range
    pub fn effective_monthly_limit(&self) -> u32 {
        if self.unlimited {
            u32::MAX
        } else {
            (self.monthly_limit as f32 * self.multiplier) as u32
        }
    }

    /// Apply multiplier to get effective burst limit
    #[must_use]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    /// Returns the effective burst limit after applying the tier multiplier
    // Safe: multiplier values are controlled and positive, result fits in u32 range
    pub fn effective_burst_limit(&self) -> u32 {
        (self.burst_limit as f32 * self.multiplier) as u32
    }
}

impl Default for TenantRateLimitTier {
    fn default() -> Self {
        Self::starter()
    }
}

/// Tenant rate limit configuration manager
#[derive(Debug, Clone)]
pub struct TenantRateLimitConfig {
    /// Per-tenant rate limit configurations
    tenant_configs: HashMap<TenantId, TenantRateLimitTier>,
    /// Default configuration for new tenants
    default_config: TenantRateLimitTier,
    /// Rate limit configuration source (optional)
    rate_limit_config: Option<RateLimitConfig>,
}

impl TenantRateLimitConfig {
    /// Create new tenant rate limit configuration manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            tenant_configs: HashMap::new(),
            default_config: TenantRateLimitTier::starter(),
            rate_limit_config: None,
        }
    }

    /// Create new tenant rate limit configuration manager with config
    #[must_use]
    pub fn new_with_config(config: RateLimitConfig) -> Self {
        Self {
            tenant_configs: HashMap::new(),
            default_config: TenantRateLimitTier::starter_from_config(&config),
            rate_limit_config: Some(config),
        }
    }

    /// Set rate limit configuration for a tenant
    pub fn set_tenant_config(&mut self, tenant_id: TenantId, config: TenantRateLimitTier) {
        self.tenant_configs.insert(tenant_id, config);
    }

    /// Get rate limit configuration for a tenant
    #[must_use]
    pub fn get_tenant_config(&self, tenant_id: TenantId) -> &TenantRateLimitTier {
        self.tenant_configs
            .get(&tenant_id)
            .unwrap_or(&self.default_config)
    }

    /// Configure tenant based on their plan
    pub fn configure_tenant_by_plan(&mut self, tenant_id: TenantId, plan: &str) {
        let config = self.rate_limit_config.as_ref().map_or_else(
            || {
                // Fall back to constant-based constructors
                match plan.to_lowercase().as_str() {
                    tiers::PROFESSIONAL => TenantRateLimitTier::professional(),
                    tiers::ENTERPRISE => TenantRateLimitTier::enterprise(),
                    _ => TenantRateLimitTier::starter(),
                }
            },
            |rate_config| {
                // Use config-based constructors when config is available
                match plan.to_lowercase().as_str() {
                    tiers::PROFESSIONAL => {
                        TenantRateLimitTier::professional_from_config(rate_config)
                    }
                    tiers::ENTERPRISE => TenantRateLimitTier::enterprise_from_config(rate_config),
                    _ => TenantRateLimitTier::starter_from_config(rate_config),
                }
            },
        );
        self.set_tenant_config(tenant_id, config);
    }

    /// Set custom multiplier for a tenant (for temporary adjustments)
    pub fn set_tenant_multiplier(&mut self, tenant_id: TenantId, multiplier: f32) {
        let mut config = self.get_tenant_config(tenant_id).clone(); // Safe: TenantConfig ownership for modification
        config.multiplier = multiplier;
        self.set_tenant_config(tenant_id, config);
    }

    /// Remove tenant configuration (falls back to default)
    pub fn remove_tenant_config(&mut self, tenant_id: &TenantId) {
        self.tenant_configs.remove(tenant_id);
    }

    /// Get all configured tenant IDs
    #[must_use]
    pub fn get_configured_tenants(&self) -> Vec<TenantId> {
        self.tenant_configs.keys().copied().collect()
    }

    /// Check if tenant is already configured
    #[must_use]
    pub fn is_tenant_configured(&self, tenant_id: TenantId) -> bool {
        self.tenant_configs.contains_key(&tenant_id)
    }
}

impl Default for TenantRateLimitConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified rate limit calculator with tenant-aware capabilities
#[derive(Clone)]
pub struct UnifiedRateLimitCalculator {
    /// Tenant-specific rate limit configurations
    tenant_config: TenantRateLimitConfig,
}

impl UnifiedRateLimitCalculator {
    /// Create a new unified rate limit calculator
    #[must_use]
    pub fn new() -> Self {
        Self {
            tenant_config: TenantRateLimitConfig::new(),
        }
    }

    /// Create a new unified rate limit calculator with config
    #[must_use]
    pub fn new_with_config(config: RateLimitConfig) -> Self {
        Self {
            tenant_config: TenantRateLimitConfig::new_with_config(config),
        }
    }

    /// Create calculator with custom tenant configuration
    #[must_use]
    pub const fn with_tenant_config(tenant_config: TenantRateLimitConfig) -> Self {
        Self { tenant_config }
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
                tier: tiers::ENTERPRISE.into(),
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
                tier: tiers::ENTERPRISE.into(),
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
                tier: tiers::ENTERPRISE.into(),
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

    /// Calculate tenant-specific rate limit status
    ///
    /// # Errors
    ///
    /// Returns an error if tenant configuration cannot be retrieved
    #[must_use]
    pub fn calculate_tenant_rate_limit(
        &self,
        tenant: &Tenant,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        // Get tenant config, auto-configuring based on plan if not already configured
        let tenant_config = if self.tenant_config.is_tenant_configured(tenant.id) {
            // Use existing configuration
            self.tenant_config.get_tenant_config(tenant.id)
        } else {
            // Auto-configure based on plan
            match tenant.plan.to_lowercase().as_str() {
                tiers::PROFESSIONAL => &TenantRateLimitTier::professional(),
                tiers::ENTERPRISE => &TenantRateLimitTier::enterprise(),
                _ => &TenantRateLimitTier::starter(),
            }
        };

        if tenant_config.unlimited {
            UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: tenant.plan.clone(), // Safe: String ownership for rate limit status
                auth_method: "tenant_token".into(),
            }
        } else {
            let limit = tenant_config.effective_monthly_limit();
            let remaining = limit.saturating_sub(current_usage);
            let is_rate_limited = current_usage >= limit;

            UnifiedRateLimitInfo {
                is_rate_limited,
                limit: Some(limit),
                remaining: Some(remaining),
                reset_at: Some(Self::calculate_monthly_reset()),
                tier: tenant.plan.clone(), // Safe: String ownership for rate limit status
                auth_method: "tenant_token".into(),
            }
        }
    }

    /// Calculate tenant-aware API key rate limit (API key + tenant context)
    #[must_use]
    pub fn calculate_tenant_api_key_rate_limit(
        &self,
        api_key: &ApiKey,
        tenant_id: TenantId,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        let mut base_info = self.calculate_api_key_rate_limit(api_key, current_usage);
        let tenant_config = self.tenant_config.get_tenant_config(tenant_id);

        // Apply tenant multiplier to API key limits
        if let (Some(limit), Some(_remaining)) = (base_info.limit, base_info.remaining) {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            // Safe: limit values are from API tiers, multiplier is controlled and positive
            let effective_limit = (limit as f32 * tenant_config.multiplier) as u32;
            let effective_remaining = effective_limit.saturating_sub(current_usage);

            base_info.limit = Some(effective_limit);
            base_info.remaining = Some(effective_remaining);
            base_info.is_rate_limited = current_usage >= effective_limit;
        }

        base_info
    }

    /// Calculate tenant-aware JWT rate limit (user + tenant context)
    #[must_use]
    pub fn calculate_tenant_jwt_rate_limit(
        &self,
        user: &User,
        tenant_id: TenantId,
        current_usage: u32,
    ) -> UnifiedRateLimitInfo {
        let mut base_info = self.calculate_jwt_rate_limit(user, current_usage);
        let tenant_config = self.tenant_config.get_tenant_config(tenant_id);

        // Apply tenant multiplier to user limits
        if let (Some(limit), Some(_remaining)) = (base_info.limit, base_info.remaining) {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss
            )]
            // Safe: limit values are from API tiers, multiplier is controlled and positive
            let effective_limit = (limit as f32 * tenant_config.multiplier) as u32;
            let effective_remaining = effective_limit.saturating_sub(current_usage);

            base_info.limit = Some(effective_limit);
            base_info.remaining = Some(effective_remaining);
            base_info.is_rate_limited = current_usage >= effective_limit;
        }

        base_info
    }

    /// Configure tenant rate limits
    pub fn configure_tenant(&mut self, tenant_id: TenantId, config: TenantRateLimitTier) {
        self.tenant_config.set_tenant_config(tenant_id, config);
    }

    /// Configure tenant by plan name
    pub fn configure_tenant_by_plan(&mut self, tenant_id: TenantId, plan: &str) {
        self.tenant_config.configure_tenant_by_plan(tenant_id, plan);
    }

    /// Set tenant rate limit multiplier for temporary adjustments
    pub fn set_tenant_multiplier(&mut self, tenant_id: TenantId, multiplier: f32) {
        self.tenant_config
            .set_tenant_multiplier(tenant_id, multiplier);
    }

    /// Get tenant configuration
    #[must_use]
    pub fn get_tenant_config(&self, tenant_id: TenantId) -> &TenantRateLimitTier {
        self.tenant_config.get_tenant_config(tenant_id)
    }

    /// Get all configured tenants
    #[must_use]
    pub fn get_configured_tenants(&self) -> Vec<TenantId> {
        self.tenant_config.get_configured_tenants()
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
            // Enterprise and any future tiers default to Enterprise
            _ => UserTier::Enterprise,
        }
    }

    /// Calculate when the monthly rate limit resets (beginning of next month)
    #[must_use]
    pub fn calculate_monthly_reset() -> DateTime<Utc> {
        let now = Utc::now();
        let next_month = if now.month() == 12 {
            now.with_year(now.year() + 1)
                .and_then(|dt| dt.with_month(1))
                .unwrap_or_else(|| {
                    warn!("Failed to calculate next year/January, using fallback");
                    now + chrono::Duration::days(31)
                })
        } else {
            now.with_month(now.month() + 1).unwrap_or_else(|| {
                warn!("Failed to increment month, using fallback");
                now + chrono::Duration::days(31)
            })
        };

        next_month
            .with_day(1)
            .and_then(|dt| dt.with_hour(0))
            .and_then(|dt| dt.with_minute(0))
            .and_then(|dt| dt.with_second(0))
            .unwrap_or_else(|| {
                warn!("Failed to set reset time components, using next month");
                next_month
            })
    }
}

impl Default for UnifiedRateLimitCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// `OAuth2`-specific rate limit configuration
#[derive(Debug, Clone)]
pub struct OAuth2RateLimitConfig {
    /// Requests per minute for authorization endpoint
    pub authorize_rpm: u32,
    /// Requests per minute for token endpoint
    pub token_rpm: u32,
    /// Requests per minute for registration endpoint
    pub register_rpm: u32,
}

impl OAuth2RateLimitConfig {
    /// Create new `OAuth2` rate limit configuration with defaults
    #[must_use]
    pub const fn new() -> Self {
        use crate::constants::oauth_rate_limiting;
        Self {
            authorize_rpm: oauth_rate_limiting::AUTHORIZE_RPM, // 1 per second
            token_rpm: oauth_rate_limiting::TOKEN_RPM,         // 1 per 2 seconds
            register_rpm: oauth_rate_limiting::REGISTER_RPM,   // 1 per 6 seconds
        }
    }

    /// Create `OAuth2` rate limit configuration from `RateLimitConfig`
    #[must_use]
    pub const fn from_rate_limit_config(config: &RateLimitConfig) -> Self {
        Self {
            authorize_rpm: config.oauth_authorize_rpm,
            token_rpm: config.oauth_token_rpm,
            register_rpm: config.oauth_register_rpm,
        }
    }

    /// Create custom `OAuth2` rate limit configuration
    #[must_use]
    pub const fn custom(authorize_rpm: u32, token_rpm: u32, register_rpm: u32) -> Self {
        Self {
            authorize_rpm,
            token_rpm,
            register_rpm,
        }
    }

    /// Get rate limit for specific `OAuth2` endpoint
    #[must_use]
    pub fn get_limit(&self, endpoint: &str) -> u32 {
        match endpoint {
            "authorize" => self.authorize_rpm,
            "token" => self.token_rpm,
            "register" => self.register_rpm,
            _ => 60,
        }
    }
}

impl Default for OAuth2RateLimitConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// `OAuth2` rate limit status including retry information
#[derive(Debug, Clone, Serialize)]
pub struct OAuth2RateLimitStatus {
    /// Whether the request is rate limited
    pub is_limited: bool,
    /// Maximum requests allowed per minute
    pub limit: u32,
    /// Remaining requests in the current minute
    pub remaining: u32,
    /// When the rate limit resets (Unix timestamp)
    pub reset_at: i64,
    /// Seconds until rate limit resets (for Retry-After header)
    pub retry_after_seconds: Option<u32>,
}

impl OAuth2RateLimitStatus {
    /// Calculate retry-after seconds from reset timestamp
    #[must_use]
    pub fn with_retry_after(mut self) -> Self {
        if self.is_limited {
            let now = Utc::now().timestamp();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            // Safe: retry_after is always positive (max(0)) and bounded by 60 seconds (rate limit window)
            let retry_after = ((self.reset_at - now).max(0)) as u32;
            self.retry_after_seconds = Some(retry_after);
        }
        self
    }

    /// Get next reset time (start of next minute)
    #[must_use]
    pub fn calculate_reset() -> DateTime<Utc> {
        let now = Utc::now();
        now.with_second(0)
            .and_then(|dt| dt.with_nanosecond(0))
            .unwrap_or(now)
            + chrono::Duration::minutes(1)
    }
}
