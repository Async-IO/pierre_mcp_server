// ABOUTME: Integration tests for mobility MCP tools (stretching exercises and yoga poses)
// ABOUTME: Tests tool registration, database operations, and activity-based recommendations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Mobility Tool Handler Integration Tests
//!
//! Tests the 6 mobility MCP tools via the `UniversalToolExecutor`:
//! - `list_stretching_exercises`: List stretching exercises with optional filters
//! - `get_stretching_exercise`: Get a specific stretching exercise by ID
//! - `suggest_stretches_for_activity`: Get activity-specific stretch recommendations
//! - `list_yoga_poses`: List yoga poses with optional filters
//! - `get_yoga_pose`: Get a specific yoga pose by ID
//! - `suggest_yoga_sequence`: Generate a yoga sequence for recovery

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use serde_json::json;
use uuid::Uuid;

mod common;

// ============================================================================
// Test Setup
// ============================================================================

/// Create test executor for mobility tool tests
async fn create_mobility_test_executor() -> Result<UniversalToolExecutor> {
    common::init_server_config();
    common::init_test_http_clients();

    let resources = common::create_test_server_resources().await?;
    Ok(UniversalToolExecutor::new(resources))
}

/// Create a test request with user ID
fn create_test_request(
    tool_name: &str,
    parameters: serde_json::Value,
    user_id: Uuid,
) -> UniversalRequest {
    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: Some(user_id.to_string()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

// ============================================================================
// Tool Registration Tests
// ============================================================================

#[tokio::test]
async fn test_mobility_tools_registered() -> Result<()> {
    let executor = create_mobility_test_executor().await?;

    let tool_names: Vec<String> = executor
        .list_tools()
        .iter()
        .map(|tool| tool.name().to_owned())
        .collect();

    let expected_tools = vec![
        "list_stretching_exercises",
        "get_stretching_exercise",
        "suggest_stretches_for_activity",
        "list_yoga_poses",
        "get_yoga_pose",
        "suggest_yoga_sequence",
    ];

    for expected_tool in expected_tools {
        assert!(
            tool_names.contains(&expected_tool.to_owned()),
            "Missing mobility tool: {expected_tool}"
        );
    }

    Ok(())
}

// ============================================================================
// list_stretching_exercises Tests
// ============================================================================

#[tokio::test]
async fn test_list_stretching_exercises_default() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request("list_stretching_exercises", json!({}), user_id);

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Tool should succeed: {:?}",
        response.error
    );
    let result = response.result.unwrap();

    // Result should have exercises array
    assert!(
        result["exercises"].is_array(),
        "Should have exercises array"
    );

    Ok(())
}

#[tokio::test]
async fn test_list_stretching_exercises_with_category_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_stretching_exercises",
        json!({
            "category": "dynamic"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success, "Tool should succeed");
    let result = response.result.unwrap();

    assert!(result["exercises"].is_array());

    // If there are exercises, verify they have the correct category
    if let Some(exercises) = result["exercises"].as_array() {
        for exercise in exercises {
            assert_eq!(
                exercise["category"].as_str(),
                Some("dynamic"),
                "Filtered exercises should be dynamic category"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_list_stretching_exercises_with_difficulty_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_stretching_exercises",
        json!({
            "difficulty": "beginner"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["exercises"].is_array());

    // If there are exercises, verify they have the correct difficulty
    if let Some(exercises) = result["exercises"].as_array() {
        for exercise in exercises {
            assert_eq!(
                exercise["difficulty"].as_str(),
                Some("beginner"),
                "Filtered exercises should be beginner difficulty"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_list_stretching_exercises_with_muscle_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_stretching_exercises",
        json!({
            "muscle_group": "hamstrings"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["exercises"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_list_stretching_exercises_with_pagination() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_stretching_exercises",
        json!({
            "limit": 5,
            "offset": 0
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    if let Some(exercises) = result["exercises"].as_array() {
        assert!(exercises.len() <= 5, "Should respect limit parameter");
    }

    Ok(())
}

// ============================================================================
// get_stretching_exercise Tests
// ============================================================================

#[tokio::test]
async fn test_get_stretching_exercise_not_found() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "get_stretching_exercise",
        json!({
            "id": "nonexistent-id"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    // Should fail for nonexistent ID
    assert!(!response.success, "Should fail for nonexistent exercise");
    assert!(response.error.is_some());

    Ok(())
}

#[tokio::test]
async fn test_get_stretching_exercise_missing_id() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request("get_stretching_exercise", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    // Should error on missing required parameter
    assert!(result.is_err() || !result.unwrap().success);

    Ok(())
}

// ============================================================================
// suggest_stretches_for_activity Tests
// ============================================================================

#[tokio::test]
async fn test_suggest_stretches_for_running() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_stretches_for_activity",
        json!({
            "activity_type": "running"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Tool should succeed: {:?}",
        response.error
    );
    let result = response.result.unwrap();

    // Should have suggested exercises
    assert!(result["exercises"].is_array() || result["suggestions"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_suggest_stretches_for_cycling() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_stretches_for_activity",
        json!({
            "activity_type": "cycling"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Verify activity type is echoed back
    assert!(
        result["activity_type"].as_str().is_some()
            || result["exercises"].is_array()
            || result["suggestions"].is_array()
    );

    Ok(())
}

#[tokio::test]
async fn test_suggest_stretches_with_difficulty() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_stretches_for_activity",
        json!({
            "activity_type": "running",
            "difficulty": "intermediate"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);

    Ok(())
}

#[tokio::test]
async fn test_suggest_stretches_missing_activity() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request("suggest_stretches_for_activity", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    // Should error on missing required parameter
    assert!(result.is_err() || !result.unwrap().success);

    Ok(())
}

// ============================================================================
// list_yoga_poses Tests
// ============================================================================

#[tokio::test]
async fn test_list_yoga_poses_default() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request("list_yoga_poses", json!({}), user_id);

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Tool should succeed: {:?}",
        response.error
    );
    let result = response.result.unwrap();

    // Result should have poses array
    assert!(result["poses"].is_array(), "Should have poses array");

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_poses_with_category_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "category": "standing"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["poses"].is_array());

    // If there are poses, verify they have the correct category
    if let Some(poses) = result["poses"].as_array() {
        for pose in poses {
            assert_eq!(
                pose["category"].as_str(),
                Some("standing"),
                "Filtered poses should be standing category"
            );
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_poses_with_difficulty_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "difficulty": "advanced"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["poses"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_poses_with_pose_type_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "pose_type": "stretch"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["poses"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_poses_with_recovery_context_filter() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "recovery_context": "post_cardio"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["poses"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_poses_with_pagination() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "limit": 3,
            "offset": 0
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    if let Some(poses) = result["poses"].as_array() {
        assert!(poses.len() <= 3, "Should respect limit parameter");
    }

    Ok(())
}

// ============================================================================
// get_yoga_pose Tests
// ============================================================================

#[tokio::test]
async fn test_get_yoga_pose_not_found() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "get_yoga_pose",
        json!({
            "id": "nonexistent-pose-id"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    // Should fail for nonexistent ID
    assert!(!response.success, "Should fail for nonexistent pose");
    assert!(response.error.is_some());

    Ok(())
}

#[tokio::test]
async fn test_get_yoga_pose_missing_id() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request("get_yoga_pose", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    // Should error on missing required parameter
    assert!(result.is_err() || !result.unwrap().success);

    Ok(())
}

// ============================================================================
// suggest_yoga_sequence Tests
// ============================================================================

#[tokio::test]
async fn test_suggest_yoga_sequence_for_recovery() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_yoga_sequence",
        json!({
            "purpose": "recovery"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(
        response.success,
        "Tool should succeed: {:?}",
        response.error
    );
    let result = response.result.unwrap();

    // Should have sequence or poses
    assert!(
        result["sequence"].is_array() || result["poses"].is_array(),
        "Should have sequence of poses"
    );

    Ok(())
}

#[tokio::test]
async fn test_suggest_yoga_sequence_for_post_run() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_yoga_sequence",
        json!({
            "purpose": "post_run"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Verify purpose is echoed back or sequence is present
    assert!(
        result["purpose"].as_str().is_some()
            || result["sequence"].is_array()
            || result["poses"].is_array()
    );

    Ok(())
}

#[tokio::test]
async fn test_suggest_yoga_sequence_with_duration() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_yoga_sequence",
        json!({
            "purpose": "recovery",
            "duration_minutes": 15
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    // Should have sequence metadata including duration
    assert!(
        result["sequence"].is_array()
            || result["poses"].is_array()
            || result["total_duration_seconds"].is_number()
    );

    Ok(())
}

#[tokio::test]
async fn test_suggest_yoga_sequence_with_difficulty() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_yoga_sequence",
        json!({
            "purpose": "recovery",
            "difficulty": "beginner"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);

    Ok(())
}

#[tokio::test]
async fn test_suggest_yoga_sequence_missing_purpose() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request("suggest_yoga_sequence", json!({}), user_id);

    let result = executor.execute_tool(request).await;

    // Should error on missing required parameter
    assert!(result.is_err() || !result.unwrap().success);

    Ok(())
}

// ============================================================================
// Combined Filter Tests
// ============================================================================

#[tokio::test]
async fn test_list_stretching_with_multiple_filters() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_stretching_exercises",
        json!({
            "category": "static",
            "difficulty": "beginner",
            "limit": 10
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["exercises"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_with_multiple_filters() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "category": "standing",
            "difficulty": "intermediate",
            "pose_type": "strength",
            "limit": 5
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    assert!(response.success);
    let result = response.result.unwrap();

    assert!(result["poses"].is_array());

    Ok(())
}

// ============================================================================
// Edge Cases
// ============================================================================

#[tokio::test]
async fn test_suggest_stretches_unknown_activity() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "suggest_stretches_for_activity",
        json!({
            "activity_type": "unknown_sport_xyz"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    // Should either succeed with empty results or provide fallback
    // Behavior depends on implementation - both are acceptable
    if response.success {
        let result = response.result.unwrap();
        assert!(
            result["exercises"].is_array() || result["suggestions"].is_array(),
            "Should return array even for unknown activity"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_list_stretching_invalid_category() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_stretching_exercises",
        json!({
            "category": "invalid_category_xyz"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    // Should handle gracefully - either succeed with empty results or default category
    // Both behaviors are acceptable
    if response.success {
        let result = response.result.unwrap();
        assert!(result["exercises"].is_array());
    }

    Ok(())
}

#[tokio::test]
async fn test_list_yoga_invalid_difficulty() -> Result<()> {
    let executor = create_mobility_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = create_test_request(
        "list_yoga_poses",
        json!({
            "difficulty": "impossible"
        }),
        user_id,
    );

    let response = executor.execute_tool(request).await?;

    // Should handle gracefully - either succeed with empty results or default difficulty
    if response.success {
        let result = response.result.unwrap();
        assert!(result["poses"].is_array());
    }

    Ok(())
}
