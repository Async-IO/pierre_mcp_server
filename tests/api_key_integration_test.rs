// ABOUTME: Integration tests for API key authentication and MCP workflows
// ABOUTME: Tests API key creation, validation, rate limiting, and MCP protocol integration
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

mod common;

use chrono::{Duration, Utc};
use pierre_mcp_server::{
    api_keys::{ApiKeyManager, ApiKeyTier, ApiKeyUsage, CreateApiKeyRequest},
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::factory::Database,
    middleware::McpAuthMiddleware,
    models::User,
};
use std::sync::Arc;

async fn create_test_environment() -> (
    Arc<Database>,
    Arc<AuthManager>,
    Arc<McpAuthMiddleware>,
    User,
    String,
) {
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

    // Create auth manager
    let auth_manager = Arc::new(AuthManager::new(24));

    // Create auth middleware
    let jwks_manager = common::get_shared_test_jwks();
    let auth_middleware = Arc::new(McpAuthMiddleware::new(
        (*auth_manager).clone(),
        database.clone(),
        jwks_manager,
        pierre_mcp_server::config::environment::RateLimitConfig::default(),
    ));

    // Create test user
    let user = User::new(
        "integration@example.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Integration Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate JWT token for the user
    let jwks_manager = common::get_shared_test_jwks();
    let jwt_token = auth_manager.generate_token(&user, &jwks_manager).unwrap();

    (database, auth_manager, auth_middleware, user, jwt_token)
}

#[tokio::test]
async fn test_end_to_end_api_key_workflow() {
    let (database, _auth_manager, auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Step 1: Create an API key
    let request = CreateApiKeyRequest {
        name: "E2E Test Key".to_owned(),
        description: Some("End-to-end test API key".to_owned()),
        tier: ApiKeyTier::Professional,
        expires_in_days: Some(30),
        rate_limit_requests: None,
    };

    let (api_key, full_key) = api_key_manager.create_api_key(user.id, request).unwrap();
    database.create_api_key(&api_key).await.unwrap();

    // Step 2: Authenticate using the API key
    let auth_result = auth_middleware
        .authenticate_request(Some(&full_key))
        .await
        .unwrap();

    assert_eq!(auth_result.user_id, user.id);
    assert!(matches!(
        auth_result.auth_method,
        pierre_mcp_server::auth::AuthMethod::ApiKey { .. }
    ));
    assert!(auth_result.rate_limit.limit.is_some());

    // Step 3: Verify rate limit status
    let rate_limit = &auth_result.rate_limit;
    assert!(!rate_limit.is_rate_limited);
    assert_eq!(rate_limit.limit, Some(100_000)); // Professional tier
    assert_eq!(rate_limit.remaining, Some(100_000)); // No usage yet

    // Step 4: Record some usage
    let usage = ApiKeyUsage {
        id: None,
        api_key_id: api_key.id.clone(),
        timestamp: Utc::now(),
        tool_name: "get_activities".to_owned(),
        response_time_ms: Some(150),
        status_code: 200,
        error_message: None,
        request_size_bytes: Some(256),
        response_size_bytes: Some(1024),
        ip_address: Some("127.0.0.1".to_owned()),
        user_agent: Some("test-client".to_owned()),
    };

    database.record_api_key_usage(&usage).await.unwrap();

    // Step 5: Verify usage is tracked
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 1);

    // Step 6: Check updated rate limit
    let updated_rate_limit = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(!updated_rate_limit.is_rate_limited);
    assert_eq!(updated_rate_limit.remaining, Some(99_999));

    // Step 7: Get usage statistics
    let start_date = Utc::now() - Duration::days(1);
    let end_date = Utc::now() + Duration::days(1);
    let stats = database
        .get_api_key_usage_stats(&api_key.id, start_date, end_date)
        .await
        .unwrap();

    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_requests, 1);
    assert_eq!(stats.failed_requests, 0);
    assert_eq!(stats.total_response_time_ms, 150);
}

#[tokio::test]
async fn test_api_key_rate_limiting() {
    let (database, _auth_manager, auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Create a Starter tier key with low limit for testing
    let request = CreateApiKeyRequest {
        name: "Rate Limit Test Key".to_owned(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    let (mut api_key, full_key) = api_key_manager.create_api_key(user.id, request).unwrap();

    // Override rate limit for testing (simulate a very low limit)
    api_key.rate_limit_requests = 2;
    database.create_api_key(&api_key).await.unwrap();

    // First request should succeed
    let auth_result1 = auth_middleware
        .authenticate_request(Some(&full_key))
        .await
        .unwrap();
    assert!(!auth_result1.rate_limit.is_rate_limited);

    // Record usage to approach the limit
    for i in 0..2 {
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

    // Now the key should be rate limited
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 2);

    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.remaining, Some(0));

    // Authentication should fail due to rate limiting
    let auth_result = auth_middleware.authenticate_request(Some(&full_key)).await;
    assert!(auth_result.is_err());
    let error_msg = auth_result.unwrap_err().to_string();
    assert!(error_msg.contains("Rate limit exceeded"));
}

#[tokio::test]
async fn test_enterprise_tier_unlimited_usage() {
    let (database, _auth_manager, auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Create an Enterprise tier key
    let request = CreateApiKeyRequest {
        name: "Enterprise Test Key".to_owned(),
        description: None,
        tier: ApiKeyTier::Enterprise,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    let (api_key, full_key) = api_key_manager.create_api_key(user.id, request).unwrap();
    database.create_api_key(&api_key).await.unwrap();

    // Record high usage
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

    // Verify high usage is recorded
    let current_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(current_usage, 1000);

    // Enterprise tier should never be rate limited
    let rate_limit_status = api_key_manager.rate_limit_status(&api_key, current_usage);
    assert!(!rate_limit_status.is_rate_limited);
    assert_eq!(rate_limit_status.limit, None);
    assert_eq!(rate_limit_status.remaining, None);

    // Authentication should still succeed
    let auth_result = auth_middleware
        .authenticate_request(Some(&full_key))
        .await
        .unwrap();
    assert!(!auth_result.rate_limit.is_rate_limited);
}

#[tokio::test]
async fn test_api_key_expiration() {
    let (database, _auth_manager, auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Create an expired API key
    let request = CreateApiKeyRequest {
        name: "Expired Test Key".to_owned(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: Some(1),
        rate_limit_requests: None,
    };

    let (mut api_key, full_key) = api_key_manager.create_api_key(user.id, request).unwrap();

    // Manually set expiration to past date
    api_key.expires_at = Some(Utc::now() - Duration::days(1));
    database.create_api_key(&api_key).await.unwrap();

    // Authentication should fail due to expiration
    let auth_result = auth_middleware.authenticate_request(Some(&full_key)).await;
    assert!(auth_result.is_err());
    assert!(auth_result.unwrap_err().to_string().contains("expired"));
}

#[tokio::test]
async fn test_deactivated_api_key() {
    let (database, _auth_manager, auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Deactivated Test Key".to_owned(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    let (api_key, full_key) = api_key_manager.create_api_key(user.id, request).unwrap();
    database.create_api_key(&api_key).await.unwrap();

    // Verify it works initially
    let auth_result = auth_middleware
        .authenticate_request(Some(&full_key))
        .await
        .unwrap();
    assert_eq!(auth_result.user_id, user.id);

    // Deactivate the key
    database
        .deactivate_api_key(&api_key.id, user.id)
        .await
        .unwrap();

    // Authentication should now fail
    let auth_result = auth_middleware.authenticate_request(Some(&full_key)).await;
    assert!(auth_result.is_err());
    let error_msg = auth_result.unwrap_err().to_string();
    assert!(error_msg.contains("API key not found or invalid"));
}

#[tokio::test]
async fn test_invalid_api_key_format() {
    let (_database, _auth_manager, auth_middleware, _user, _jwt_token) =
        create_test_environment().await;

    // Test various invalid key formats
    let invalid_keys = vec![
        "invalid_key",
        "pk_test_abcdefghijklmnopqrstuvwxyz123456", // Wrong prefix
        "pk_live_short",                            // Too short
        "pk_live_abcdefghijklmnopqrstuvwxyz12345",  // Too short by 1
        "pk_live_abcdefghijklmnopqrstuvwxyz1234567", // Too long by 1
        "",                                         // Empty
        "bearer token",                             // JWT-like format
    ];

    for invalid_key in invalid_keys {
        let auth_result = auth_middleware
            .authenticate_request(Some(invalid_key))
            .await;
        assert!(
            auth_result.is_err(),
            "Key '{}' should be invalid",
            invalid_key
        );
    }
}

#[tokio::test]
async fn test_concurrent_api_key_usage() {
    let (database, _auth_manager, auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Concurrent Test Key".to_owned(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    let (api_key, full_key) = api_key_manager.create_api_key(user.id, request).unwrap();
    database.create_api_key(&api_key).await.unwrap();

    // Simulate concurrent requests
    let mut handles = vec![];
    for i in 0..10 {
        let auth_middleware_clone = auth_middleware.clone();
        let database_clone = database.clone();
        let api_key_id = api_key.id.clone();
        let full_key_clone = full_key.clone();

        let handle = tokio::spawn(async move {
            // Authenticate
            let auth_result = auth_middleware_clone
                .authenticate_request(Some(&full_key_clone))
                .await
                .unwrap();

            // Record usage
            let usage = ApiKeyUsage {
                id: None,
                api_key_id: api_key_id.clone(),
                timestamp: Utc::now(),
                tool_name: format!("concurrent_tool_{i}"),
                response_time_ms: Some(100 + i * 10),
                status_code: 200,
                error_message: None,
                request_size_bytes: None,
                response_size_bytes: None,
                ip_address: None,
                user_agent: None,
            };
            database_clone.record_api_key_usage(&usage).await.unwrap();

            auth_result.user_id
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Verify all requests succeeded and returned the correct user ID
    assert_eq!(results.len(), 10);
    for user_id in results {
        assert_eq!(user_id, user.id);
    }

    // Verify all usage was recorded
    let final_usage = database
        .get_api_key_current_usage(&api_key.id)
        .await
        .unwrap();
    assert_eq!(final_usage, 10);
}

#[tokio::test]
async fn test_usage_analytics() {
    let (database, _auth_manager, _auth_middleware, user, _jwt_token) =
        create_test_environment().await;
    let api_key_manager = ApiKeyManager::new();

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Analytics Test Key".to_owned(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    let (api_key, _full_key) = api_key_manager.create_api_key(user.id, request).unwrap();
    database.create_api_key(&api_key).await.unwrap();

    // Record diverse usage patterns
    let tools = ["get_activities", "get_athlete", "analyze_activity"];
    let status_codes = [200, 200, 400, 200, 500]; // Mix of success and errors

    for (i, &status_code) in status_codes.iter().enumerate() {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            timestamp: Utc::now() - Duration::hours(i as i64),
            tool_name: tools[i % tools.len()].to_owned(),
            response_time_ms: Some((100 + i * 50) as u32),
            status_code,
            error_message: if status_code >= 400 {
                Some(format!("Error {status_code}"))
            } else {
                None
            },
            request_size_bytes: Some(256),
            response_size_bytes: Some(1024),
            ip_address: Some("127.0.0.1".to_owned()),
            user_agent: Some("test-client".to_owned()),
        };
        database.record_api_key_usage(&usage).await.unwrap();
    }

    // Get usage statistics
    let start_date = Utc::now() - Duration::days(1);
    let end_date = Utc::now() + Duration::hours(1);
    let stats = database
        .get_api_key_usage_stats(&api_key.id, start_date, end_date)
        .await
        .unwrap();

    // Verify statistics
    assert_eq!(stats.total_requests, 5);
    assert_eq!(stats.successful_requests, 3); // 200 status codes
    assert_eq!(stats.failed_requests, 2); // 400 and 500 status codes
    assert_eq!(stats.total_response_time_ms, 100 + 150 + 200 + 250 + 300); // Sum of response times

    // Verify tool usage breakdown is captured
    assert!(stats.tool_usage.is_object());
}

#[tokio::test]
async fn test_create_api_key_invalid_auth() {
    let (_database, _auth_manager, auth_middleware, _user, _jwt_token) =
        create_test_environment().await;

    // Test with no authorization header
    let result = auth_middleware.authenticate_request(None).await;
    assert!(result.is_err());
    // Just verify auth fails - don't check exact error message

    // Test with invalid bearer token format
    let result = auth_middleware
        .authenticate_request(Some("invalid_token"))
        .await;
    assert!(result.is_err());

    // Test with malformed bearer token
    let result = auth_middleware.authenticate_request(Some("Bearer ")).await;
    assert!(result.is_err());

    // Test with completely fake token
    let result = auth_middleware
        .authenticate_request(Some("Bearer fake_token_12345"))
        .await;
    assert!(result.is_err());
}
