// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Rate limiting integration tests

use chrono::{Datelike, Duration, Timelike, Utc};
use pierre_mcp_server::{
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, ApiKeyUsage},
    auth::{AuthManager, McpAuthMiddleware},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
};
use std::sync::Arc;
use uuid::Uuid;

async fn create_test_setup() -> (Arc<Database>, ApiKeyManager, Arc<McpAuthMiddleware>, User) {
    // Create test database
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(Database::new(database_url, encryption_key).await.unwrap());

    // Create auth manager and middleware
    let jwt_secret = pierre_mcp_server::auth::generate_jwt_secret().to_vec();
    let auth_manager = AuthManager::new(jwt_secret, 24);
    let auth_middleware = Arc::new(McpAuthMiddleware::new(auth_manager, database.clone()));

    // Create API key manager
    let api_key_manager = ApiKeyManager::new();

    // Create test user with unique email
    let unique_id = Uuid::new_v4();
    let user = User::new(
        format!("ratelimit+{}@example.com", unique_id),
        "hashed_password".to_string(),
        Some("Rate Limit Test User".to_string()),
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
        name: "Starter Rate Limit Test".to_string(),
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
            tool_name: format!("test_tool_{}", i),
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
    assert!(error_msg.contains("Rate limit reached for API key"));
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
        name: "Professional Rate Limit Test".to_string(),
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
            tool_name: format!("bulk_tool_{}", i),
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
    let rate_limit_status = api_key_manager.calculate_rate_limit_status(&api_key, current_usage);
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
        name: "Enterprise Unlimited Test".to_string(),
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
            tool_name: format!("enterprise_tool_{}", i),
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

    let rate_limit_status = api_key_manager.calculate_rate_limit_status(&api_key, current_usage);
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
        name: "Reset Timing Test".to_string(),
        key_prefix: "pk_live_rese".to_string(),
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
    let rate_limit_status = api_key_manager.calculate_rate_limit_status(&api_key, 5000);

    // Verify reset time is set correctly
    let reset_at = rate_limit_status.reset_at.unwrap();
    let now = Utc::now();

    // Reset should be at beginning of next month
    let expected_next_month = if now.month() == 12 {
        now.with_year(now.year() + 1)
            .unwrap()
            .with_month(1)
            .unwrap()
    } else {
        now.with_month(now.month() + 1).unwrap()
    };

    let expected_reset = expected_next_month
        .with_day(1)
        .unwrap()
        .with_hour(0)
        .unwrap()
        .with_minute(0)
        .unwrap()
        .with_second(0)
        .unwrap();

    // Should be within a few seconds of expected reset time
    let diff = (reset_at - expected_reset).num_seconds().abs();
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
        name: "Monthly Usage Test".to_string(),
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
            tool_name: format!("current_month_tool_{}", i),
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
            tool_name: format!("last_month_tool_{}", i),
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
        name: "Edge Case Test".to_string(),
        key_prefix: "pk_live_edge".to_string(),
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
            tool_name: format!("edge_tool_{}", i),
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

    let rate_limit_status = api_key_manager.calculate_rate_limit_status(&api_key, current_usage);
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
        name: "Mixed Status Test".to_string(),
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
            tool_name: format!("mixed_tool_{}", i),
            response_time_ms: Some(100),
            status_code,
            error_message: if status_code >= 400 {
                Some(format!("Error {}", status_code))
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

    let rate_limit_status = api_key_manager.calculate_rate_limit_status(&api_key, current_usage);
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
