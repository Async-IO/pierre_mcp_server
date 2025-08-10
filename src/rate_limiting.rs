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
use crate::models::{Tenant, User, UserTier};
use chrono::{DateTime, Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    /// Tenant ID if applicable
    pub tenant_id: Option<Uuid>,
    /// Tenant-specific rate limit multiplier applied
    pub tenant_multiplier: Option<f32>,
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

pub const TENANT_STARTER_LIMIT: u32 = 10_000;
pub const TENANT_PROFESSIONAL_LIMIT: u32 = 100_000;
pub const TENANT_ENTERPRISE_LIMIT: u32 = 1_000_000;

impl TenantRateLimitTier {
    /// Create tier configuration for starter tenants
    #[must_use]
    pub const fn starter() -> Self {
        Self {
            monthly_limit: TENANT_STARTER_LIMIT,
            burst_limit: 100,
            multiplier: 1.0,
            unlimited: false,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for professional tenants
    #[must_use]
    pub const fn professional() -> Self {
        Self {
            monthly_limit: TENANT_PROFESSIONAL_LIMIT,
            burst_limit: 500,
            multiplier: 1.0,
            unlimited: false,
            custom_reset_period: None,
        }
    }

    /// Create tier configuration for enterprise tenants
    #[must_use]
    pub const fn enterprise() -> Self {
        Self {
            monthly_limit: TENANT_ENTERPRISE_LIMIT,
            burst_limit: 2000,
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
    tenant_configs: HashMap<Uuid, TenantRateLimitTier>,
    /// Default configuration for new tenants
    default_config: TenantRateLimitTier,
}

impl TenantRateLimitConfig {
    /// Create new tenant rate limit configuration manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            tenant_configs: HashMap::new(),
            default_config: TenantRateLimitTier::starter(),
        }
    }

    /// Set rate limit configuration for a tenant
    pub fn set_tenant_config(&mut self, tenant_id: Uuid, config: TenantRateLimitTier) {
        self.tenant_configs.insert(tenant_id, config);
    }

    /// Get rate limit configuration for a tenant
    #[must_use]
    pub fn get_tenant_config(&self, tenant_id: Uuid) -> &TenantRateLimitTier {
        self.tenant_configs
            .get(&tenant_id)
            .unwrap_or(&self.default_config)
    }

    /// Configure tenant based on their plan
    pub fn configure_tenant_by_plan(&mut self, tenant_id: Uuid, plan: &str) {
        let config = match plan.to_lowercase().as_str() {
            "professional" | "pro" => TenantRateLimitTier::professional(),
            "enterprise" | "ent" => TenantRateLimitTier::enterprise(),
            _ => TenantRateLimitTier::starter(),
        };
        self.set_tenant_config(tenant_id, config);
    }

    /// Set custom multiplier for a tenant (for temporary adjustments)
    pub fn set_tenant_multiplier(&mut self, tenant_id: Uuid, multiplier: f32) {
        let mut config = self.get_tenant_config(tenant_id).clone();
        config.multiplier = multiplier;
        self.set_tenant_config(tenant_id, config);
    }

    /// Remove tenant configuration (falls back to default)
    pub fn remove_tenant_config(&mut self, tenant_id: &Uuid) {
        self.tenant_configs.remove(tenant_id);
    }

    /// Get all configured tenant IDs
    #[must_use]
    pub fn get_configured_tenants(&self) -> Vec<Uuid> {
        self.tenant_configs.keys().copied().collect()
    }

    /// Check if tenant is already configured
    #[must_use]
    pub fn is_tenant_configured(&self, tenant_id: Uuid) -> bool {
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
                tier: "enterprise".into(),
                auth_method: "api_key".into(),
                tenant_id: None,
                tenant_multiplier: None,
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
                tenant_id: None,
                tenant_multiplier: None,
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
                tenant_id: None,
                tenant_multiplier: None,
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
                tenant_id: None,
                tenant_multiplier: None,
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
                tenant_id: None,
                tenant_multiplier: None,
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
                tenant_id: None,
                tenant_multiplier: None,
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
                "professional" | "pro" => &TenantRateLimitTier::professional(),
                "enterprise" | "ent" => &TenantRateLimitTier::enterprise(),
                _ => &TenantRateLimitTier::starter(),
            }
        };

        if tenant_config.unlimited {
            UnifiedRateLimitInfo {
                is_rate_limited: false,
                limit: None,
                remaining: None,
                reset_at: None,
                tier: tenant.plan.clone(),
                auth_method: "tenant_token".into(),
                tenant_id: Some(tenant.id),
                tenant_multiplier: Some(tenant_config.multiplier),
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
                tier: tenant.plan.clone(),
                auth_method: "tenant_token".into(),
                tenant_id: Some(tenant.id),
                tenant_multiplier: Some(tenant_config.multiplier),
            }
        }
    }

    /// Calculate tenant-aware API key rate limit (API key + tenant context)
    #[must_use]
    pub fn calculate_tenant_api_key_rate_limit(
        &self,
        api_key: &ApiKey,
        tenant_id: Uuid,
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

        base_info.tenant_id = Some(tenant_id);
        base_info.tenant_multiplier = Some(tenant_config.multiplier);
        base_info
    }

    /// Calculate tenant-aware JWT rate limit (user + tenant context)
    #[must_use]
    pub fn calculate_tenant_jwt_rate_limit(
        &self,
        user: &User,
        tenant_id: Uuid,
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

        base_info.tenant_id = Some(tenant_id);
        base_info.tenant_multiplier = Some(tenant_config.multiplier);
        base_info
    }

    /// Configure tenant rate limits
    pub fn configure_tenant(&mut self, tenant_id: Uuid, config: TenantRateLimitTier) {
        self.tenant_config.set_tenant_config(tenant_id, config);
    }

    /// Configure tenant by plan name
    pub fn configure_tenant_by_plan(&mut self, tenant_id: Uuid, plan: &str) {
        self.tenant_config.configure_tenant_by_plan(tenant_id, plan);
    }

    /// Set tenant rate limit multiplier for temporary adjustments
    pub fn set_tenant_multiplier(&mut self, tenant_id: Uuid, multiplier: f32) {
        self.tenant_config
            .set_tenant_multiplier(tenant_id, multiplier);
    }

    /// Get tenant configuration
    #[must_use]
    pub fn get_tenant_config(&self, tenant_id: Uuid) -> &TenantRateLimitTier {
        self.tenant_config.get_tenant_config(tenant_id)
    }

    /// Get all configured tenants
    #[must_use]
    pub fn get_configured_tenants(&self) -> Vec<Uuid> {
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
