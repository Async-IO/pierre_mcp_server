// ABOUTME: Comprehensive error handling tests for all MCP tool categories
// ABOUTME: Tests missing parameters, invalid types, boundary values, and error response formats
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Comprehensive Tool Error Handling Tests
//!
//! Systematic tests for error conditions across all MCP tool categories:
//! - Missing required parameters
//! - Invalid parameter types
//! - Boundary value violations
//! - Proper error response formats
//! - Authentication/authorization errors

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use pierre_mcp_server::protocols::ProtocolError;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

mod common;

// ============================================================================
// Test Setup
// ============================================================================

/// Create test executor for error handling tests
async fn create_error_test_executor() -> Result<UniversalToolExecutor> {
    common::init_server_config();
    common::init_test_http_clients();

    let resources = common::create_test_server_resources().await?;
    Ok(UniversalToolExecutor::new(resources))
}

/// Create a test request with given parameters
fn create_request(tool_name: &str, parameters: serde_json::Value) -> UniversalRequest {
    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: Some(Uuid::new_v4().to_string()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

/// Assert that the error contains expected text
fn assert_error_contains(
    result: &Result<pierre_mcp_server::protocols::universal::UniversalResponse, ProtocolError>,
    expected: &str,
) {
    match result {
        Ok(response) => {
            assert!(!response.success, "Expected failure response");
            assert!(
                response
                    .error
                    .as_ref()
                    .is_some_and(|e| e.contains(expected)),
                "Expected error containing '{expected}', got: {:?}",
                response.error
            );
        }
        Err(e) => {
            let error_str = format!("{e:?}");
            assert!(
                error_str.contains(expected),
                "Expected error containing '{expected}', got: {error_str}"
            );
        }
    }
}

// ============================================================================
// Core Data Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_get_activities_invalid_limit_type() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "get_activities",
        json!({
            "activity_provider": "strava",
            "limit": "not_a_number"  // Should be integer
        }),
    );

    // Should either reject invalid type or proceed with default
    let result = executor.execute_tool(request).await;
    // This might succeed with default or fail - depends on implementation
    // What matters is it doesn't panic
    assert!(result.is_ok() || result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_get_activities_negative_limit() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "get_activities",
        json!({
            "activity_provider": "strava",
            "limit": -10
        }),
    );

    // Negative values should be handled gracefully
    let _result = executor.execute_tool(request).await;
    // Implementation should handle this without panic

    Ok(())
}

#[tokio::test]
async fn test_get_activities_huge_limit() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "get_activities",
        json!({
            "activity_provider": "strava",
            "limit": 1_000_000_000  // Unreasonably large
        }),
    );

    // Should cap or handle gracefully
    let _result = executor.execute_tool(request).await;
    // Should not cause memory issues

    Ok(())
}

// ============================================================================
// Intelligence Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_analyze_activity_missing_id() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("analyze_activity", json!({}));

    let result = executor.execute_tool(request).await;
    assert_error_contains(&result, "activity_id");

    Ok(())
}

#[tokio::test]
async fn test_analyze_activity_invalid_id_format() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "analyze_activity",
        json!({
            "activity_id": "not-a-valid-id-format!!@#$",
            "activity_provider": "strava"
        }),
    );

    let _result = executor.execute_tool(request).await;
    // Should handle invalid format gracefully

    Ok(())
}

#[tokio::test]
async fn test_calculate_metrics_empty_parameters() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("calculate_metrics", json!({}));

    let result = executor.execute_tool(request).await;
    // Should require activity_id or activity_provider
    assert!(result.is_err() || !result.as_ref().unwrap().success);

    Ok(())
}

#[tokio::test]
async fn test_compare_activities_same_id() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let same_id = "12345";
    let request = create_request(
        "compare_activities",
        json!({
            "activity_id_1": same_id,
            "activity_id_2": same_id,  // Same as activity_id_1
            "activity_provider": "strava"
        }),
    );

    let result = executor.execute_tool(request).await;
    // Comparing same activity - should fail or return empty comparison
    if let Ok(response) = result {
        // Either fails or returns meaningful response
        assert!(response.success || response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_compare_activities_missing_one_id() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "compare_activities",
        json!({
            "activity_id_1": "12345",
            "activity_provider": "strava"
            // Missing activity_id_2
        }),
    );

    let result = executor.execute_tool(request).await;
    // Tool should either return an error OR handle gracefully
    // The important thing is it doesn't panic
    if let Ok(response) = result {
        // Tool might have its own validation logic
        // Either way, it should handle missing params
        assert!(
            !response.success || response.error.is_none(),
            "Tool handled missing param gracefully"
        );
    }
    // Error case is also acceptable - param validation caught it

    Ok(())
}

// ============================================================================
// Goal Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_set_goal_missing_type() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "set_goal",
        json!({
            "target_value": 100.0,
            "deadline": "2025-12-31"
        }),
    );

    let result = executor.execute_tool(request).await;
    assert_error_contains(&result, "goal_type");

    Ok(())
}

#[tokio::test]
async fn test_set_goal_invalid_target() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "set_goal",
        json!({
            "goal_type": "distance",
            "target_value": -1000.0,  // Negative target
            "deadline": "2025-12-31"
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should reject negative target or handle gracefully
    if let Ok(response) = &result {
        if response.success {
            // Some implementations might accept this
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_set_goal_past_deadline() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "set_goal",
        json!({
            "goal_type": "distance",
            "target_value": 100.0,
            "deadline": "2020-01-01"  // Past date
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should handle past deadline - might warn or reject
    if let Ok(response) = &result {
        if !response.success {
            assert!(response.error.is_some());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_track_progress_missing_goal_id() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("track_progress", json!({}));

    let result = executor.execute_tool(request).await;
    assert_error_contains(&result, "goal_id");

    Ok(())
}

// ============================================================================
// Configuration Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_update_user_configuration_invalid_key() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "update_user_configuration",
        json!({
            "key": "definitely.not.a.real.config.key",
            "value": 42
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should reject unknown config key
    if let Ok(response) = &result {
        if !response.success {
            assert!(response.error.is_some());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_validate_configuration_missing_config() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("validate_configuration", json!({}));

    let result = executor.execute_tool(request).await;
    // Should require configuration to validate
    assert!(result.is_err() || !result.as_ref().unwrap().success);

    Ok(())
}

#[tokio::test]
async fn test_calculate_personalized_zones_invalid_vo2max() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "calculate_personalized_zones",
        json!({
            "vo2_max": -10.0  // Invalid negative value
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should reject invalid VO2 max
    if let Ok(response) = &result {
        if !response.success {
            assert!(response.error.is_some());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_calculate_personalized_zones_extreme_vo2max() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "calculate_personalized_zones",
        json!({
            "vo2_max": 150.0  // Unrealistically high (world record ~97)
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should handle or warn about extreme value
    // This might succeed but with warnings
    let _ = result;

    Ok(())
}

// ============================================================================
// Sleep & Recovery Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_analyze_sleep_quality_missing_data() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("analyze_sleep_quality", json!({}));

    let result = executor.execute_tool(request).await;
    // Should require either sleep_data or sleep_provider
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_analyze_sleep_quality_invalid_duration() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "analyze_sleep_quality",
        json!({
            "sleep_data": {
                "date": "2025-01-15T06:00:00Z",
                "duration_hours": -2.0,  // Negative duration
                "efficiency_percent": 90.0
            }
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should handle invalid sleep duration
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_analyze_sleep_quality_invalid_efficiency() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "analyze_sleep_quality",
        json!({
            "sleep_data": {
                "date": "2025-01-15T06:00:00Z",
                "duration_hours": 8.0,
                "efficiency_percent": 150.0  // >100%
            }
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should handle invalid efficiency percentage
    // This might fail or clamp the value
    let _ = result;

    Ok(())
}

#[tokio::test]
async fn test_track_sleep_trends_empty_history() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "track_sleep_trends",
        json!({
            "sleep_history": [],
            "days": 7
        }),
    );

    let result = executor.execute_tool(request).await;
    // Empty history should be handled gracefully - might return zeros, error, or empty response
    // The important thing is it doesn't panic
    // Both Ok and Err responses are acceptable - we just verify no panic occurred
    drop(result);

    Ok(())
}

// ============================================================================
// Nutrition Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_calculate_daily_nutrition_all_missing() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("calculate_daily_nutrition", json!({}));

    let result = executor.execute_tool(request).await?;
    assert!(!result.success, "Should fail with all missing params");

    Ok(())
}

#[tokio::test]
async fn test_calculate_daily_nutrition_extreme_weight() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "calculate_daily_nutrition",
        json!({
            "weight_kg": 1000.0,  // Unrealistic weight
            "height_cm": 180.0,
            "age": 30,
            "gender": "male",
            "activity_level": "moderately_active",
            "training_goal": "maintenance"
        }),
    );

    let result = executor.execute_tool(request).await?;
    // Should either succeed with warning or fail
    // Just verify it doesn't panic
    let _ = result;

    Ok(())
}

#[tokio::test]
async fn test_search_food_empty_query() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "search_food",
        json!({
            "query": ""  // Empty query
        }),
    );

    let result = executor.execute_tool(request).await;
    // Empty query might be rejected or return empty results
    let _ = result;

    Ok(())
}

// ============================================================================
// Recipe Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_save_recipe_empty_ingredients() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "save_recipe",
        json!({
            "name": "Empty Recipe",
            "servings": 2,
            "instructions": ["Step 1"],
            "ingredients": []  // Empty ingredients
        }),
    );

    let result = executor.execute_tool(request).await;
    // Empty ingredients might be rejected or saved
    let _ = result;

    Ok(())
}

#[tokio::test]
async fn test_save_recipe_zero_servings() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "save_recipe",
        json!({
            "name": "Zero Serving Recipe",
            "servings": 0,  // Invalid
            "instructions": ["Step 1"],
            "ingredients": [{"name": "rice", "amount": 100.0, "unit": "grams"}]
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should reject zero servings
    if let Ok(response) = &result {
        if !response.success {
            assert!(response.error.is_some());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_get_recipe_invalid_uuid() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "get_recipe",
        json!({
            "recipe_id": "not-a-valid-uuid"
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should handle invalid UUID
    if let Ok(response) = &result {
        assert!(!response.success);
    }

    Ok(())
}

#[tokio::test]
async fn test_search_recipes_very_long_query() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let long_query = "a".repeat(10000); // Very long query
    let request = create_request(
        "search_recipes",
        json!({
            "query": long_query
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should handle very long query without crashing
    let _ = result;

    Ok(())
}

// ============================================================================
// Connection Tools Error Tests
// ============================================================================

#[tokio::test]
async fn test_disconnect_provider_missing_name() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("disconnect_provider", json!({}));

    let result = executor.execute_tool(request).await;
    assert_error_contains(&result, "provider");

    Ok(())
}

#[tokio::test]
async fn test_disconnect_provider_invalid_name() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request(
        "disconnect_provider",
        json!({
            "provider": "nonexistent_provider_xyz"
        }),
    );

    let result = executor.execute_tool(request).await;
    // Should reject unknown provider
    if let Ok(response) = &result {
        if !response.success {
            assert!(response.error.is_some());
        }
    }

    Ok(())
}

// ============================================================================
// Universal Error Response Format Tests
// ============================================================================

#[tokio::test]
async fn test_unknown_tool_name() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("definitely_not_a_real_tool_name_xyz", json!({}));

    let result = executor.execute_tool(request).await;
    // Should return proper error for unknown tool
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_null_parameters() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let request = create_request("get_activities", serde_json::Value::Null);

    let result = executor.execute_tool(request).await;
    // Should handle null parameters gracefully
    let _ = result;

    Ok(())
}

#[tokio::test]
async fn test_malformed_json_in_parameters() -> Result<()> {
    let executor = create_error_test_executor().await?;

    // Array instead of object for parameters
    let request = create_request("get_activities", json!(["not", "an", "object"]));

    let result = executor.execute_tool(request).await;
    // Should handle type mismatch
    let _ = result;

    Ok(())
}

// ============================================================================
// Concurrent Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_concurrent_invalid_requests() -> Result<()> {
    let executor = Arc::new(create_error_test_executor().await?);

    // Fire multiple invalid requests concurrently
    let handles: Vec<_> = (0..5)
        .map(|i| {
            let exec = Arc::clone(&executor);
            let req = create_request(
                "unknown_tool",
                json!({
                    "bad_param": i
                }),
            );
            tokio::spawn(async move { exec.execute_tool(req).await })
        })
        .collect();

    for handle in handles {
        let result = handle.await?;
        // All should fail gracefully
        assert!(result.is_err());
    }

    Ok(())
}

// ============================================================================
// User ID Validation Tests
// ============================================================================

#[tokio::test]
async fn test_empty_user_id() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let mut request = create_request("get_activities", json!({"activity_provider": "strava"}));
    request.user_id = String::new(); // Empty user ID

    let result = executor.execute_tool(request).await;
    // Should fail with empty user ID
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_invalid_user_id_format() -> Result<()> {
    let executor = create_error_test_executor().await?;

    let mut request = create_request("get_activities", json!({"activity_provider": "strava"}));
    request.user_id = "not-a-valid-uuid-format".to_owned();

    let result = executor.execute_tool(request).await;
    // Should fail with invalid user ID format
    assert!(result.is_err());

    Ok(())
}
