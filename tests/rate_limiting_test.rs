// ABOUTME: Rate limiting integration tests for API throttling
// ABOUTME: Tests rate limiting functionality and quota enforcement
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Rate limiting integration tests

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use chrono::{Datelike, Duration, TimeZone, Timelike, Utc};
use pierre_mcp_server::{
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, ApiKeyUsage},
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    middleware::McpAuthMiddleware,
    models::User,
};
use std::sync::Arc;
use uuid::Uuid;

async fn create_test_setup() -> (Arc<Database>, ApiKeyManager, Arc<McpAuthMiddleware>, User) {
    // Create test database
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    // Create auth manager and middleware
    let auth_manager = AuthManager::new(24);
    let jwks_manager = common::get_shared_test_jwks();
    let auth_middleware = Arc::new(McpAuthMiddleware::new(
        auth_manager,
        database.clone(),
        jwks_manager,
        pierre_mcp_server::config::environment::RateLimitConfig::default(),
    ));

    // Create API key manager
    let api_key_manager = ApiKeyManager::new();

    // Create test user with unique email
    let unique_id = Uuid::new_v4();
    let user = User::new(
        format!("ratelimit+{unique_id}@example.com"),
        "hashed_password".to_owned(),
        Some("Rate Limit Test User".to_owned()),
    );

    (database, api_key_manager, auth_middleware, user)
}

#[tokio::test]
async fn test_starter_tier_rate_limiting() {
    let (database, api_key_manager, auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create Starter tier API key with low limit for testing
    let full_key = "pk_live_ratelimitstarterkey1234567890123"; // 40 chars total
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Starter Rate Limit Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 5, // Very low limit for testing
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Test requests within limit
    for i in 0..5 {
        let auth_result = auth_middleware
            .authenticate_request(Some(full_key))
            .await
            .unwrap();

        let rate_limit = &auth_result.rate_limit;
        assert!(!rate_limit.is_rate_limited);
        assert_eq!(rate_limit.limit, Some(5));
        assert_eq!(rate_limit.remaining, Some(5 - i)); // Remaining should decrease

        // Record usage
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now(),
            tool_name: format!("test_tool_{i}"),
            response_time_ms: Some(100),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Next request should be rate limited
    let auth_result = auth_middleware.authenticate_request(Some(full_key)).await;
    assert!(auth_result.is_err());
    let error_msg = auth_result.unwrap_err().to_string();
    assert!(error_msg.contains("Rate limit exceeded"));
}

#[tokio::test]
async fn test_professional_tier_rate_limiting() {
    let (database, api_key_manager, auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create Professional tier API key
    let full_key = "pk_live_professionallimitkey123456789012"; // 40 chars total
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Professional Rate Limit Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 100_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Test that professional tier has higher limits
    let auth_result = auth_middleware
        .authenticate_request(Some(full_key))
        .await
        .unwrap();

    let rate_limit = &auth_result.rate_limit;
    assert!(!rate_limit.is_rate_limited);
    assert_eq!(rate_limit.limit, Some(100_000));
    assert_eq!(rate_limit.remaining, Some(100_000));

    // Record some usage
    for i in 0..1000 {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now(),
            tool_name: format!("bulk_tool_{i}"),
            response_time_ms: Some(50),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Should still be under limit
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(!rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.remaining, Some(99_000));
}

#[tokio::test]
async fn test_enterprise_tier_unlimited() {
    let (database, api_key_manager, auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create Enterprise tier API key
    let full_key = "pk_live_enterpriseunlimitedkey1234567890"; // 40 chars total
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Enterprise Unlimited Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Enterprise,
        rate_limit_requests: u32::MAX,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Test unlimited usage
    let auth_result = auth_middleware
        .authenticate_request(Some(full_key))
        .await
        .unwrap();

    let rate_limit = &auth_result.rate_limit;
    assert!(!rate_limit.is_rate_limited);
    assert_eq!(rate_limit.limit, None); // Unlimited
    assert_eq!(rate_limit.remaining, None);
    assert_eq!(rate_limit.reset_at, None);

    // Record massive usage
    for i in 0..10000 {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now(),
            tool_name: format!("enterprise_tool_{i}"),
            response_time_ms: Some(25),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Should still be unlimited
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 10_000);

    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(!rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.limit, None);
    assert_eq!(rate_limit_status.remaining, None);

    // Authentication should still work
    let auth_result2 = auth_middleware
        .authenticate_request(Some(full_key))
        .await
        .unwrap();
    assert!(!auth_result2.rate_limit.is_rate_limited);
}

#[tokio::test]
async fn test_rate_limit_reset_timing() {
    let (database, api_key_manager, _auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create test API key
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Reset Timing Test".to_owned(),
        key_prefix: "pk_live_rese".to_owned(),
        key_hash: api_key_manager.hash_key("pk_live_resettimingkey12345678901234567"),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 10_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Calculate rate limit status
    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, 5000);

    // Verify reset time is set correctly
    let reset_at = rate_limit_status.reset_at.unwrap();
    let now = Utc::now();

    // Reset should be at beginning of next month
    // Use chrono's built-in date arithmetic to avoid edge cases
    let next_month_start = if now.month() == 12 {
        Utc.with_ymd_and_hms(now.year() + 1, 1, 1, 0, 0, 0)
    } else {
        Utc.with_ymd_and_hms(now.year(), now.month() + 1, 1, 0, 0, 0)
    };

    let expected_next_month = next_month_start
        .single()
        .expect("Failed to create valid date for next month");

    let expected_reset = expected_next_month;

    // Should be within a few seconds of expected reset time
    let duration_diff = reset_at - expected_reset;
    let diff = duration_diff.num_seconds().abs();
    assert!(diff < 5, "Reset time should be at beginning of next month");

    // Reset should be in the future
    assert!(reset_at > now, "Reset time should be in the future");

    // Reset should be at exact beginning of day
    assert_eq!(reset_at.hour(), 0);
    assert_eq!(reset_at.minute(), 0);
    assert_eq!(reset_at.second(), 0);
    assert_eq!(reset_at.day(), 1);
}

#[tokio::test]
async fn test_monthly_usage_calculation() {
    let (database, api_key_manager, _auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create an API key first
    let full_key = "pk_live_monthlyusagecalckey123456789012"; // 40 chars total
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Monthly Usage Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 100_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();
    let api_key_id = api_key.id;

    // Record usage across different months
    let current_month = Utc::now();
    let last_month = current_month - Duration::days(40);

    // Current month usage
    for i in 0..5 {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key_id.clone(),
            timestamp: current_month - Duration::hours(i),
            tool_name: format!("current_month_tool_{i}"),
            response_time_ms: Some(100),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Last month usage (should not count)
    for i in 0..3 {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key_id.clone(),
            timestamp: last_month - Duration::hours(i),
            tool_name: format!("last_month_tool_{i}"),
            response_time_ms: Some(100),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Get current month usage
    let current_usage = database
        .get_api_key_current_usage(&api_key_id)
        .await
        .unwrap();

    // Should only count current month (5 requests)
    assert_eq!(current_usage, 5, "Should only count current month usage");
}

#[tokio::test]
async fn test_rate_limit_edge_cases() {
    let (database, api_key_manager, auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Test rate limit at exactly the limit
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Edge Case Test".to_owned(),
        key_prefix: "pk_live_edge".to_owned(),
        key_hash: api_key_manager.hash_key("pk_live_edgecaselimitkey123456789012345"),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 10,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();
    let full_key = "pk_live_edgecaselimitkey123456789012345";

    // Use exactly up to the limit
    for i in 0..10 {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now(),
            tool_name: format!("edge_tool_{i}"),
            response_time_ms: Some(100),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // At the limit, should be rate limited
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 10);

    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.remaining, Some(0));

    // Authentication should fail
    let auth_result = auth_middleware.authenticate_request(Some(full_key)).await;
    assert!(auth_result.is_err());
}

#[tokio::test]
async fn test_rate_limit_with_mixed_status_codes() {
    let (database, api_key_manager, _auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    let full_key = "pk_live_mixedstatuskey123456789012345678"; // 40 chars total
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Mixed Status Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 100_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Record usage with different status codes
    let status_codes = [200, 201, 400, 401, 403, 404, 500, 502];
    for (i, &status_code) in status_codes.iter().enumerate() {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now(),
            tool_name: format!("mixed_tool_{i}"),
            response_time_ms: Some(100),
            status_code,
            error_message: if status_code >= 400 {
                Some(format!("Error {status_code}"))
            } else {
                None
            },
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // All requests should count toward rate limit, regardless of status code
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 8);

    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(!rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.remaining, Some(99_992));

    // Verify statistics capture different status codes correctly
    let start_date = Utc::now() - Duration::hours(1);
    let end_date = Utc::now() + Duration::hours(1);
    let stats = database
        .get_api_key_usage_stats(&api_key.id, start_date, end_date)
        .await
        .unwrap();

    assert_eq!(stats.total_requests, 8);
    assert_eq!(stats.successful_requests, 2); // 200, 201
    assert_eq!(stats.failed_requests, 6); // 400, 401, 403, 404, 500, 502
}

#[tokio::test]
async fn test_trial_tier_rate_limiting() {
    let (database, api_key_manager, auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create Trial tier API key with default settings
    let full_key = "pk_live_trialtierlimitkey123456789012345"; // 40 chars total (use pk_live_ for consistency)
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Trial Rate Limit Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Trial,
        rate_limit_requests: 1_000, // Trial tier limit
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: Some(Utc::now() + Duration::days(14)), // Auto-expires in 14 days
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Test that trial tier has lowest limits
    let auth_result = auth_middleware
        .authenticate_request(Some(full_key))
        .await
        .unwrap();

    let rate_limit = &auth_result.rate_limit;
    assert!(!rate_limit.is_rate_limited);
    assert_eq!(rate_limit.limit, Some(1_000));
    assert_eq!(rate_limit.remaining, Some(1_000));

    // Record usage up to limit
    for i in 0..1_000 {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now(),
            tool_name: format!("trial_tool_{i}"),
            response_time_ms: Some(50),
            status_code: 200,
            error_message: None,
            request_size_bytes: None,
            response_size_bytes: None,
            ip_address: None,
            user_agent: None,
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Should be at limit
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 1_000);

    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.remaining, Some(0));

    // Next request should be rate limited
    let auth_result = auth_middleware.authenticate_request(Some(full_key)).await;
    assert!(auth_result.is_err());
}

#[tokio::test]
async fn test_tier_conversion_scenarios() {
    let (database, api_key_manager, _auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Test conversion from Trial to Starter
    let trial_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Trial to Starter Conversion".to_owned(),
        key_prefix: "pk_trial_con".to_owned(),
        key_hash: api_key_manager.hash_key("pk_trial_conversionkey123456789012345"),
        description: None,
        tier: ApiKeyTier::Trial,
        rate_limit_requests: 1_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: Some(Utc::now() + Duration::days(14)),
        created_at: Utc::now(),
    };

    database.create_api_key(&trial_key).await.unwrap();

    // Verify trial tier properties
    assert_eq!(trial_key.tier, ApiKeyTier::Trial);
    assert_eq!(trial_key.rate_limit_requests, 1_000);
    assert!(trial_key.expires_at.is_some());

    // Test conversion to Starter by creating new key with same user
    let starter_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Upgraded Starter Key".to_owned(),
        key_prefix: "pk_live_upg".to_owned(),
        key_hash: api_key_manager.hash_key("pk_live_upgradedstarterkey123456789012"),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 10_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None, // No expiration for non-trial keys
        created_at: Utc::now(),
    };

    database.create_api_key(&starter_key).await.unwrap();

    // Verify starter tier properties
    assert_eq!(starter_key.tier, ApiKeyTier::Starter);
    assert_eq!(starter_key.rate_limit_requests, 10_000);
    assert!(starter_key.expires_at.is_none());

    // Test conversion to Professional
    let professional_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Professional Key".to_owned(),
        key_prefix: "pk_live_pro".to_owned(),
        key_hash: api_key_manager.hash_key("pk_live_professionalkey123456789012345"),
        description: None,
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 100_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&professional_key).await.unwrap();

    // Verify professional tier properties
    assert_eq!(professional_key.tier, ApiKeyTier::Professional);
    assert_eq!(professional_key.rate_limit_requests, 100_000);
    assert!(professional_key.expires_at.is_none());

    // Test conversion to Enterprise
    let enterprise_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Enterprise Key".to_owned(),
        key_prefix: "pk_live_ent".to_owned(),
        key_hash: api_key_manager.hash_key("pk_live_enterprisekey123456789012345678"),
        description: None,
        tier: ApiKeyTier::Enterprise,
        rate_limit_requests: 1_000_000_000, // Effectively unlimited
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&enterprise_key).await.unwrap();

    // Verify enterprise tier properties
    assert_eq!(enterprise_key.tier, ApiKeyTier::Enterprise);
    assert_eq!(enterprise_key.rate_limit_requests, 1_000_000_000);
    assert!(enterprise_key.expires_at.is_none());

    // Test rate limit status for each tier
    let trial_status = api_key_manager.rate_limit_status(&trial_key, 500);
    assert!(!trial_status.is_rate_limited);
    assert_eq!(trial_status.limit, Some(1_000));
    assert_eq!(trial_status.remaining, Some(500));

    let starter_status = api_key_manager.rate_limit_status(&starter_key, 5_000);
    assert!(!starter_status.is_rate_limited);
    assert_eq!(starter_status.limit, Some(10_000));
    assert_eq!(starter_status.remaining, Some(5_000));

    let professional_status = api_key_manager.rate_limit_status(&professional_key, 50_000);
    assert!(!professional_status.is_rate_limited);
    assert_eq!(professional_status.limit, Some(100_000));
    assert_eq!(professional_status.remaining, Some(50_000));

    let enterprise_status = api_key_manager.rate_limit_status(&enterprise_key, 1_000_000);
    assert!(!enterprise_status.is_rate_limited);
    assert_eq!(enterprise_status.limit, None); // Unlimited
    assert_eq!(enterprise_status.remaining, None);
}

#[tokio::test]
async fn test_legacy_conversion_functionality() {
    let (database, api_key_manager, _auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Test legacy API key creation using the old CreateApiKeyRequest format
    let legacy_request = pierre_mcp_server::api_keys::CreateApiKeyRequest {
        name: "Legacy API Key".to_owned(),
        description: Some("Created using legacy format".to_owned()),
        tier: ApiKeyTier::Professional,
        rate_limit_requests: Some(50_000), // Custom limit
        expires_in_days: Some(365),        // Custom expiration
    };

    let (legacy_key, legacy_full_key) = api_key_manager
        .create_api_key(user.id, legacy_request)
        .unwrap();

    // Verify legacy key properties
    assert_eq!(legacy_key.tier, ApiKeyTier::Professional);
    assert_eq!(legacy_key.rate_limit_requests, 50_000);
    assert!(legacy_key.expires_at.is_some());
    assert_eq!(
        legacy_key.description,
        Some("Created using legacy format".to_owned())
    );

    // Test new simplified API key creation
    let simple_request = pierre_mcp_server::api_keys::CreateApiKeyRequestSimple {
        name: "Simple API Key".to_owned(),
        description: Some("Created using simplified format".to_owned()),
        rate_limit_requests: 25_000, // Maps to Professional tier
        expires_in_days: None,
    };

    let (simple_key, simple_full_key) = api_key_manager
        .create_api_key_simple(user.id, simple_request)
        .unwrap();

    // Verify simple key properties (tier is automatically determined)
    assert_eq!(simple_key.tier, ApiKeyTier::Professional);
    assert_eq!(simple_key.rate_limit_requests, 25_000);
    assert!(simple_key.expires_at.is_none());

    // Test trial key creation using legacy method
    let trial_key_result = api_key_manager.create_trial_key(
        user.id,
        "Legacy Trial Key".to_owned(),
        Some("Auto-generated trial key".to_owned()),
    );

    let (trial_key, trial_full_key) = trial_key_result.unwrap();

    // Verify trial key properties
    assert_eq!(trial_key.tier, ApiKeyTier::Trial);
    assert_eq!(trial_key.rate_limit_requests, 1_000);
    assert!(trial_key.expires_at.is_some());
    assert!(trial_full_key.starts_with("pk_trial_"));

    // Test key format validation
    assert!(api_key_manager
        .validate_key_format(&legacy_full_key)
        .is_ok());
    assert!(api_key_manager
        .validate_key_format(&simple_full_key)
        .is_ok());
    assert!(api_key_manager.validate_key_format(&trial_full_key).is_ok());

    // Test key type detection
    assert!(!api_key_manager.is_trial_key(&legacy_full_key));
    assert!(!api_key_manager.is_trial_key(&simple_full_key));
    assert!(api_key_manager.is_trial_key(&trial_full_key));

    // Store all keys in database
    database.create_api_key(&legacy_key).await.unwrap();
    database.create_api_key(&simple_key).await.unwrap();
    database.create_api_key(&trial_key).await.unwrap();

    // Test that all keys are valid
    assert!(api_key_manager.is_key_valid(&legacy_key).is_ok());
    assert!(api_key_manager.is_key_valid(&simple_key).is_ok());
    assert!(api_key_manager.is_key_valid(&trial_key).is_ok());
}

#[tokio::test]
async fn test_monthly_reset_calculations() {
    let (database, api_key_manager, _auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Test reset calculations for different times of the month
    let test_dates = [
        // Beginning of month
        Utc::now()
            .with_day(1)
            .unwrap()
            .with_hour(0)
            .unwrap()
            .with_minute(0)
            .unwrap()
            .with_second(0)
            .unwrap(),
        // Middle of month
        Utc::now()
            .with_day(15)
            .unwrap()
            .with_hour(12)
            .unwrap()
            .with_minute(30)
            .unwrap()
            .with_second(45)
            .unwrap(),
        // End of month
        Utc::now()
            .with_day(28)
            .unwrap()
            .with_hour(23)
            .unwrap()
            .with_minute(59)
            .unwrap()
            .with_second(59)
            .unwrap(),
    ];

    for (i, test_date) in test_dates.iter().enumerate() {
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id: user.id,
            name: format!("Monthly Reset Test {i}"),
            key_prefix: format!("pk_live_res{i}"),
            key_hash: api_key_manager.hash_key(&format!("pk_live_resettest{i}key123456789012345")),
            description: None,
            tier: ApiKeyTier::Starter,
            rate_limit_requests: 10_000,
            rate_limit_window_seconds: 30 * 24 * 60 * 60,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: *test_date,
        };

        database.create_api_key(&api_key).await.unwrap();

        // Test rate limit status at different usage levels
        let usage_levels = [0, 5_000, 9_999, 10_000];

        for usage in usage_levels {
            let rate_limit_status = api_key_manager.rate_limit_status(&api_key, usage);

            // Verify reset time is always at beginning of next month
            let reset_at = rate_limit_status.reset_at.unwrap();
            assert_eq!(reset_at.day(), 1);
            assert_eq!(reset_at.hour(), 0);
            assert_eq!(reset_at.minute(), 0);
            assert_eq!(reset_at.second(), 0);

            // Reset should be in the future
            assert!(reset_at > Utc::now());

            // Verify rate limiting behavior
            if usage >= 10_000 {
                assert!(rate_limit_status.is_rate_limited);
                assert_eq!(rate_limit_status.remaining, Some(0));
            } else {
                assert!(!rate_limit_status.is_rate_limited);
                assert_eq!(rate_limit_status.remaining, Some(10_000 - usage));
            }
        }
    }
}

#[tokio::test]
async fn test_enterprise_unlimited_comprehensive() {
    let (database, api_key_manager, auth_middleware, user) = create_test_setup().await;

    // Store the user in the database first
    database.create_user(&user).await.unwrap();

    // Create Enterprise tier API key
    let full_key = "pk_live_enterprisecomprehensivekey123456"; // 40 chars total
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id: user.id,
        name: "Enterprise Comprehensive Test".to_owned(),
        key_prefix: api_key_manager.extract_key_prefix(full_key),
        key_hash: api_key_manager.hash_key(full_key),
        description: None,
        tier: ApiKeyTier::Enterprise,
        rate_limit_requests: 1_000_000_000, // Effectively unlimited
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    database.create_api_key(&api_key).await.unwrap();

    // Test with extremely high usage levels that would break other tiers
    let extreme_usage_levels = [0, 1_000, 10_000, 50_000]; // Reduced for test performance

    for usage in extreme_usage_levels {
        // Record usage to database (sample only, not full count for performance)
        let sample_count = std::cmp::min(usage, 100); // Record max 100 samples
        for i in 0..sample_count {
            let usage_record = ApiKeyUsage {
                id: None,
                api_key_id: api_key.id.clone(),
                timestamp: Utc::now() - Duration::seconds(i64::from(i)),
                tool_name: format!("enterprise_extreme_tool_{i}"),
                response_time_ms: Some(10),
                status_code: 200,
                error_message: None,
                request_size_bytes: None,
                response_size_bytes: None,
                ip_address: None,
                user_agent: None,
            };
            database.record_api_key_usage(&usage_record).await.unwrap();
        }

        // Test rate limit status
        let rate_limit_status = api_key_manager.rate_limit_status(&api_key, usage);
        assert!(!rate_limit_status.is_rate_limited);
        assert_eq!(rate_limit_status.limit, None); // Unlimited
        assert_eq!(rate_limit_status.remaining, None);
        assert_eq!(rate_limit_status.reset_at, None);

        // Test authentication still works
        let auth_result = auth_middleware
            .authenticate_request(Some(full_key))
            .await
            .unwrap();
        assert!(!auth_result.rate_limit.is_rate_limited);
        assert_eq!(auth_result.rate_limit.limit, None);
        assert_eq!(auth_result.rate_limit.remaining, None);
        assert_eq!(auth_result.rate_limit.reset_at, None);
    }

    // Test tier methods
    assert_eq!(api_key.tier.monthly_limit(), None);
    assert!(!api_key.tier.is_trial());
    assert_eq!(api_key.tier.as_str(), "enterprise");
    assert_eq!(api_key.tier.default_trial_days(), None);

    // Verify usage statistics still work with enterprise keys
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert!(current_usage > 0); // Should have recorded usage

    let start_date = Utc::now() - Duration::hours(24);
    let end_date = Utc::now() + Duration::hours(1);
    let stats = database
        .get_api_key_usage_stats(&api_key.id, start_date, end_date)
        .await
        .unwrap();

    assert!(stats.total_requests > 0);
    assert_eq!(stats.successful_requests, stats.total_requests); // All were successful
    assert_eq!(stats.failed_requests, 0);
}
