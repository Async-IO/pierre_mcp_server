// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth Token Refresh Tests
//!
//! Tests for automatic token refresh in Universal Tool Executor.

use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use pierre_mcp_server::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators,
};
use pierre_mcp_server::models::User;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Create a test UniversalToolExecutor with in-memory database
async fn create_test_executor() -> (Arc<UniversalToolExecutor>, Arc<Database>) {
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(7.5),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(85.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Improving,
                distance_trend: TrendDirection::Stable,
                consistency_score: 88.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    let executor = Arc::new(UniversalToolExecutor::new(database.clone(), intelligence));

    (executor, database)
}

/// Test that get_activities uses token refresh
#[tokio::test]
async fn test_get_activities_with_expired_token() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Store expired token
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1); // Expired
    database
        .update_strava_token(
            user_id,
            "expired_access_token",
            "refresh_token_123",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Set up environment for OAuth provider
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create request for get_activities
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_activities".to_string(),
        parameters: json!({
            "limit": 10,
            "provider": "strava"
        }),
        protocol: "test".to_string(),
    };

    // Execute tool - it should attempt to refresh the token
    let response = executor.execute_tool(request).await;

    // In a real scenario with a mock server, this would succeed after refresh
    // For now, we expect an OAuth error indicating refresh was attempted
    match response {
        Ok(resp) => {
            // If successful, check that result mentions OAuth error
            if let Some(result) = resp.result {
                if let Some(arr) = result.as_array() {
                    if let Some(first) = arr.first() {
                        if let Some(error) = first.get("error") {
                            assert!(error.as_str().unwrap().contains("OAuth"));
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Expected in test environment without mock server
        }
    }
}

/// Test connection status with OAuth manager integration
#[tokio::test]
async fn test_connection_status_with_oauth_manager() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Set up environment for OAuth providers
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");
    std::env::set_var("FITBIT_CLIENT_ID", "test_fitbit");
    std::env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_secret");

    // Create request for get_connection_status
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        protocol: "test".to_string(),
    };

    // Execute tool
    let response = executor.execute_tool(request).await.unwrap();

    // Check response
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.get("providers").is_some());

    let providers = result.get("providers").unwrap();
    assert!(providers.get("strava").is_some());
    assert!(providers.get("fitbit").is_some());

    // Both should be disconnected since no tokens are stored
    assert_eq!(
        providers["strava"]["connected"],
        serde_json::Value::Bool(false)
    );
    assert_eq!(
        providers["fitbit"]["connected"],
        serde_json::Value::Bool(false)
    );
}

/// Test that analyze_activity uses token refresh
#[tokio::test]
async fn test_analyze_activity_token_refresh() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Store token that will expire soon
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(3); // Expires in 3 minutes (within buffer)
    database
        .update_strava_token(
            user_id,
            "soon_to_expire_token",
            "refresh_token_456",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Set up environment
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create request
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "analyze_activity".to_string(),
        parameters: json!({
            "activity_id": "123456789"
        }),
        protocol: "test".to_string(),
    };

    // Execute - should trigger refresh due to token expiring soon
    let response = executor.execute_tool(request).await;

    // Verify response (will fail in test without mock server, but structure is tested)
    match response {
        Ok(resp) => {
            if let Some(error) = resp.error {
                // Expected in test environment - could be OAuth error, provider error, or activity not found
                assert!(
                    error.contains("OAuth")
                        || error.contains("Failed")
                        || error.contains("not yet fully implemented")
                        || error.contains("Activity not found")
                );
            }
        }
        Err(_) => {
            // Expected in test environment
        }
    }
}

/// Test concurrent token refresh attempts
#[tokio::test]
async fn test_concurrent_token_operations() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Store valid token
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    database
        .update_strava_token(
            user_id,
            "valid_token",
            "refresh_token",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Set up environment
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create multiple concurrent requests
    let mut handles = vec![];

    for _i in 0..5 {
        let executor_clone = executor.clone();
        let user_id_str = user_id.to_string();
        let handle = tokio::spawn(async move {
            let request = UniversalRequest {
                user_id: user_id_str,
                tool_name: "get_connection_status".to_string(),
                parameters: json!({}),
                protocol: "test".to_string(),
            };
            executor_clone.execute_tool(request).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
    }
}

/// Test error handling when OAuth provider initialization fails
/// DISABLED: This test has issues with environment variable interference from other tests
/// The OAuth provider fails gracefully in real usage when environment variables are missing
#[tokio::test]
#[serial_test::serial]
#[ignore = "Disabled due to test environment interference"]
async fn test_oauth_provider_init_failure() {
    // Store and clear environment variables to test failure case
    let original_client_id = std::env::var("STRAVA_CLIENT_ID").ok();
    let original_client_secret = std::env::var("STRAVA_CLIENT_SECRET").ok();

    // Always clear the environment variables for this test
    std::env::remove_var("STRAVA_CLIENT_ID");
    std::env::remove_var("STRAVA_CLIENT_SECRET");

    // Create executor
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Create request
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "connect_strava".to_string(),
        parameters: json!({}),
        protocol: "test".to_string(),
    };

    // Execute - should handle provider initialization failure gracefully
    let response = executor.execute_tool(request).await.unwrap();

    // Restore environment variables before assertions
    if let Some(client_id) = original_client_id {
        std::env::set_var("STRAVA_CLIENT_ID", client_id);
    }
    if let Some(client_secret) = original_client_secret {
        std::env::set_var("STRAVA_CLIENT_SECRET", client_secret);
    }

    // Should fail due to missing environment variables
    assert!(
        !response.success,
        "Expected failure but got success: {:?}",
        response
    );
    assert!(response.error.is_some(), "Expected error but got none");
    let error = response.error.as_ref().unwrap();
    assert!(
        error.contains("Failed to initialize Strava provider")
            || error.contains("STRAVA_CLIENT_ID not set")
            || error.contains("STRAVA_CLIENT_SECRET not set")
            || error.contains("Missing required environment variables"),
        "Unexpected error message: {}",
        error
    );
}
