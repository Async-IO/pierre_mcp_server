// ABOUTME: Integration tests for tenant-specific rate limiting functionality
// ABOUTME: Tests tenant rate limit configuration, multipliers, and enforcement
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::{
    api_keys::{ApiKey, ApiKeyTier},
    models::{Tenant, TenantId, User, UserStatus, UserTier},
    permissions::UserRole,
    rate_limiting::{
        TenantRateLimitConfig, TenantRateLimitTier, UnifiedRateLimitCalculator,
        TENANT_ENTERPRISE_LIMIT, TENANT_PROFESSIONAL_LIMIT, TENANT_STARTER_LIMIT,
    },
};
use uuid::Uuid;

fn create_test_tenant(plan: &str) -> Tenant {
    Tenant {
        id: TenantId::new(),
        name: "Test Tenant".to_owned(),
        slug: "test-tenant".to_owned(),
        domain: None,
        plan: plan.to_owned(),
        owner_user_id: Uuid::new_v4(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

fn create_test_api_key(tier: &ApiKeyTier) -> ApiKey {
    ApiKey {
        id: Uuid::new_v4().to_string(),
        name: "Test API Key".to_owned(),
        key_prefix: "pk_test_".to_owned(),
        key_hash: "test_hash".to_owned(),
        description: Some("Test API Key for rate limiting".to_owned()),
        user_id: Uuid::new_v4(),
        tier: tier.clone(),
        rate_limit_requests: match *tier {
            ApiKeyTier::Trial => 1_000,
            ApiKeyTier::Starter => 10_000,
            ApiKeyTier::Professional => 100_000,
            // Enterprise and any future tiers default to unlimited
            _ => u32::MAX,
        },
        rate_limit_window_seconds: 2_592_000, // 30 days
        created_at: Utc::now(),
        last_used_at: None,
        is_active: true,
        expires_at: None,
    }
}

fn create_test_user(tier: UserTier) -> User {
    User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: "test_hash".to_owned(),
        tier,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    }
}

#[test]
fn test_tenant_rate_limit_tier_creation() {
    let starter = TenantRateLimitTier::starter();
    assert_eq!(starter.monthly_limit, TENANT_STARTER_LIMIT);
    assert_eq!(starter.burst_limit, 100);
    assert!((starter.multiplier - 1.0).abs() < f32::EPSILON);
    assert!(!starter.unlimited);

    let professional = TenantRateLimitTier::professional();
    assert_eq!(professional.monthly_limit, TENANT_PROFESSIONAL_LIMIT);
    assert_eq!(professional.burst_limit, 500);
    assert!(!professional.unlimited);

    let enterprise = TenantRateLimitTier::enterprise();
    assert_eq!(enterprise.monthly_limit, TENANT_ENTERPRISE_LIMIT);
    assert_eq!(enterprise.burst_limit, 2000);
    assert!(enterprise.unlimited);
}

#[test]
fn test_tenant_rate_limit_tier_effective_limits() {
    let mut config = TenantRateLimitTier::professional();
    config.multiplier = 2.0;

    assert_eq!(
        config.effective_monthly_limit(),
        TENANT_PROFESSIONAL_LIMIT * 2
    );
    assert_eq!(config.effective_burst_limit(), 1000); // 500 * 2.0

    // Test unlimited behavior
    config.unlimited = true;
    assert_eq!(config.effective_monthly_limit(), u32::MAX);
}

#[test]
fn test_tenant_config_management() {
    let mut config = TenantRateLimitConfig::new();
    let tenant_id = TenantId::new();

    // Test default configuration
    let default_config = config.get_tenant_config(tenant_id);
    assert_eq!(default_config.monthly_limit, TENANT_STARTER_LIMIT);

    // Test setting custom configuration
    let custom_config = TenantRateLimitTier::professional();
    config.set_tenant_config(tenant_id, custom_config);

    let retrieved_config = config.get_tenant_config(tenant_id);
    assert_eq!(retrieved_config.monthly_limit, TENANT_PROFESSIONAL_LIMIT);

    // Test configure by plan
    config.configure_tenant_by_plan(tenant_id, "enterprise");
    let enterprise_config = config.get_tenant_config(tenant_id);
    assert!(enterprise_config.unlimited);
}

#[test]
fn test_tenant_specific_rate_limiting() {
    let calculator = UnifiedRateLimitCalculator::new();
    let tenant = create_test_tenant("professional");
    let current_usage = 50_000;

    let rate_limit_info = calculator.calculate_tenant_rate_limit(&tenant, current_usage);

    assert!(!rate_limit_info.is_rate_limited);
    assert_eq!(rate_limit_info.limit, Some(TENANT_PROFESSIONAL_LIMIT));
    assert_eq!(
        rate_limit_info.remaining,
        Some(TENANT_PROFESSIONAL_LIMIT - current_usage)
    );
    assert_eq!(rate_limit_info.tier, "professional");
    assert_eq!(rate_limit_info.auth_method, "tenant_token");
}

#[test]
fn test_tenant_rate_limiting_with_multiplier() {
    let mut calculator = UnifiedRateLimitCalculator::new();
    let tenant = create_test_tenant("professional");
    let tenant_id = tenant.id;

    // Configure tenant based on plan first, then set multiplier
    calculator.configure_tenant_by_plan(tenant_id, &tenant.plan);
    calculator.set_tenant_multiplier(tenant_id, 2.0);

    let current_usage = 150_000; // Would exceed normal professional limit
    let rate_limit_info = calculator.calculate_tenant_rate_limit(&tenant, current_usage);

    // Should not be rate limited due to 2x multiplier
    assert!(!rate_limit_info.is_rate_limited);
    assert_eq!(rate_limit_info.limit, Some(TENANT_PROFESSIONAL_LIMIT * 2));
    assert_eq!(
        rate_limit_info.remaining,
        Some((TENANT_PROFESSIONAL_LIMIT * 2) - current_usage)
    );
}

#[test]
fn test_tenant_aware_api_key_rate_limiting() {
    let mut calculator = UnifiedRateLimitCalculator::new();
    let api_key = create_test_api_key(&ApiKeyTier::Professional);
    let tenant_id = TenantId::new();

    // Configure tenant with 1.5x multiplier
    calculator.set_tenant_multiplier(tenant_id, 1.5);

    let current_usage = 80_000;
    let rate_limit_info =
        calculator.calculate_tenant_api_key_rate_limit(&api_key, tenant_id, current_usage);

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    // Safe: API key limits are reasonable values, multiplier is controlled
    let expected_limit = (api_key.rate_limit_requests as f32 * 1.5) as u32;
    assert_eq!(rate_limit_info.limit, Some(expected_limit));
}

#[test]
fn test_tenant_aware_jwt_rate_limiting() {
    let mut calculator = UnifiedRateLimitCalculator::new();
    let user = create_test_user(UserTier::Professional);
    let tenant_id = TenantId::new();

    // Configure tenant with 0.5x multiplier (reduced limits)
    calculator.set_tenant_multiplier(tenant_id, 0.5);

    let current_usage = 30_000;
    let rate_limit_info =
        calculator.calculate_tenant_jwt_rate_limit(&user, tenant_id, current_usage);

    let user_limit = user.tier.monthly_limit().unwrap();
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    // Safe: User limits are reasonable values, multiplier is controlled
    let expected_limit = (user_limit as f32 * 0.5) as u32;
    assert_eq!(rate_limit_info.limit, Some(expected_limit));
}

#[test]
fn test_enterprise_tenant_unlimited() {
    let calculator = UnifiedRateLimitCalculator::new();
    let tenant = create_test_tenant("enterprise");

    // Very high usage that would normally trigger rate limiting
    let current_usage = 5_000_000;
    let rate_limit_info = calculator.calculate_tenant_rate_limit(&tenant, current_usage);

    assert!(!rate_limit_info.is_rate_limited);
    assert!(rate_limit_info.limit.is_none());
    assert!(rate_limit_info.remaining.is_none());
    assert_eq!(rate_limit_info.tier, "enterprise");
}

#[test]
fn test_tenant_config_by_plan() {
    let mut calculator = UnifiedRateLimitCalculator::new();
    let tenant_id = TenantId::new();

    // Test different plan configurations
    calculator.configure_tenant_by_plan(tenant_id, "starter");
    let config = calculator.get_tenant_config(tenant_id);
    assert_eq!(config.monthly_limit, TENANT_STARTER_LIMIT);

    calculator.configure_tenant_by_plan(tenant_id, "professional");
    let config = calculator.get_tenant_config(tenant_id);
    assert_eq!(config.monthly_limit, TENANT_PROFESSIONAL_LIMIT);

    calculator.configure_tenant_by_plan(tenant_id, "enterprise");
    let config = calculator.get_tenant_config(tenant_id);
    assert_eq!(config.monthly_limit, TENANT_ENTERPRISE_LIMIT);
    assert!(config.unlimited);

    // Test case insensitive
    calculator.configure_tenant_by_plan(tenant_id, "ENTERPRISE");
    let config = calculator.get_tenant_config(tenant_id);
    assert!(config.unlimited);
}

#[test]
fn test_tenant_rate_limiting_edge_cases() {
    let calculator = UnifiedRateLimitCalculator::new();
    let tenant = create_test_tenant("starter");

    // Test exactly at limit
    let rate_limit_info = calculator.calculate_tenant_rate_limit(&tenant, TENANT_STARTER_LIMIT);
    assert!(rate_limit_info.is_rate_limited);
    assert_eq!(rate_limit_info.remaining, Some(0));

    // Test over limit
    let rate_limit_info = calculator.calculate_tenant_rate_limit(&tenant, TENANT_STARTER_LIMIT + 1);
    assert!(rate_limit_info.is_rate_limited);
    assert_eq!(rate_limit_info.remaining, Some(0)); // Should not underflow
}

#[test]
fn test_unified_rate_limit_info_serialization() {
    use pierre_mcp_server::rate_limiting::UnifiedRateLimitInfo;

    let info = UnifiedRateLimitInfo {
        is_rate_limited: true,
        limit: Some(10_000),
        remaining: Some(5_000),
        reset_at: Some(Utc::now()),
        tier: "professional".to_owned(),
        auth_method: "tenant_token".to_owned(),
    };

    // Test serialization
    let serialized = serde_json::to_string(&info).expect("Should serialize");
    assert!(serialized.contains("is_rate_limited"));
    assert!(serialized.contains("limit"));
    assert!(serialized.contains("10000"));
}
