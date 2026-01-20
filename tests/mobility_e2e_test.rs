// ABOUTME: End-to-end tests for mobility MCP tools (stretching exercises and yoga poses)
// ABOUTME: Tests full MCP protocol flow from request to response
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::constants::tools::{
    GET_STRETCHING_EXERCISE, GET_YOGA_POSE, LIST_STRETCHING_EXERCISES, LIST_YOGA_POSES,
    SUGGEST_STRETCHES_FOR_ACTIVITY, SUGGEST_YOGA_SEQUENCE,
};
use pierre_mcp_server::mcp::multitenant::McpRequest;
use pierre_mcp_server::mcp::resources::ServerResources;
use pierre_mcp_server::mcp::tool_handlers::ToolHandlers;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

mod common;

/// MCP handler for E2E mobility tool testing
struct MobilityMcpHandler {
    resources: Arc<ServerResources>,
    test_jwt_token: String,
}

impl MobilityMcpHandler {
    /// Create new handler with test resources
    async fn new() -> Result<Self> {
        let resources = common::create_test_server_resources().await?;
        let (_user_id, user) = common::create_test_user(&resources.database).await?;

        // Create a proper JWT token
        let jwt_token = resources
            .auth_manager
            .generate_token(&user, &resources.jwks_manager)?;

        Ok(Self {
            resources,
            test_jwt_token: jwt_token,
        })
    }

    /// Handle MCP tools/call request using actual tool handlers
    async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request = McpRequest {
            jsonrpc: "2.0".to_owned(),
            method: "tools/call".to_owned(),
            params: Some(json!({
                "name": tool_name,
                "arguments": arguments
            })),
            id: Some(json!(1)),
            auth_token: Some(format!("Bearer {}", self.test_jwt_token)),
            headers: Some(HashMap::new()),
            metadata: HashMap::new(),
        };

        let response =
            ToolHandlers::handle_tools_call_with_resources(request, &self.resources).await;

        let json_response = if response.error.is_some() {
            json!({
                "jsonrpc": response.jsonrpc,
                "id": response.id,
                "error": response.error
            })
        } else {
            json!({
                "jsonrpc": response.jsonrpc,
                "id": response.id,
                "result": response.result
            })
        };

        Ok(json_response)
    }
}

// =============================================================================
// E2E Tests for Stretching Exercise Tools
// =============================================================================

#[tokio::test]
async fn test_e2e_list_stretching_exercises() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(LIST_STRETCHING_EXERCISES, json!({}))
        .await?;

    // Verify MCP protocol compliance
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none(), "Should not have error");

    let result = &response["result"];
    assert!(result.get("content").is_some(), "Should have content");

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_stretching_with_category_filter() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_STRETCHING_EXERCISES,
            json!({
                "category": "static"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_stretching_with_difficulty_filter() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_STRETCHING_EXERCISES,
            json!({
                "difficulty": "beginner"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_stretching_with_pagination() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_STRETCHING_EXERCISES,
            json!({
                "limit": 10,
                "offset": 0
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_get_stretching_exercise_missing_id() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(GET_STRETCHING_EXERCISE, json!({}))
        .await?;

    // Should return error for missing required parameter
    assert_eq!(response["jsonrpc"], "2.0");
    // Either an error in the response or success: false in the content
    let has_error = response.get("error").is_some()
        || response["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("error") || t.contains("required"));
    assert!(has_error, "Should indicate error for missing id");

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_stretches_for_activity() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            SUGGEST_STRETCHES_FOR_ACTIVITY,
            json!({
                "activity_type": "running"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_stretches_with_duration() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            SUGGEST_STRETCHES_FOR_ACTIVITY,
            json!({
                "activity_type": "cycling",
                "duration_minutes": 15
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_stretches_missing_activity_type() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(SUGGEST_STRETCHES_FOR_ACTIVITY, json!({}))
        .await?;

    // Should return error for missing required parameter
    assert_eq!(response["jsonrpc"], "2.0");
    let has_error = response.get("error").is_some()
        || response["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("error") || t.contains("required"));
    assert!(has_error, "Should indicate error for missing activity_type");

    Ok(())
}

// =============================================================================
// E2E Tests for Yoga Pose Tools
// =============================================================================

#[tokio::test]
async fn test_e2e_list_yoga_poses() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler.call_tool(LIST_YOGA_POSES, json!({})).await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    let result = &response["result"];
    assert!(result.get("content").is_some(), "Should have content");

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_yoga_with_category_filter() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_YOGA_POSES,
            json!({
                "category": "standing"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_yoga_with_pose_type_filter() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_YOGA_POSES,
            json!({
                "pose_type": "stretch"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_yoga_with_recovery_context() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_YOGA_POSES,
            json!({
                "recovery_context": "post_cardio"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_list_yoga_with_pagination() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            LIST_YOGA_POSES,
            json!({
                "limit": 10,
                "offset": 5
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_get_yoga_pose_missing_id() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler.call_tool(GET_YOGA_POSE, json!({})).await?;

    // Should return error for missing required parameter
    assert_eq!(response["jsonrpc"], "2.0");
    let has_error = response.get("error").is_some()
        || response["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("error") || t.contains("required"));
    assert!(has_error, "Should indicate error for missing id");

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_yoga_sequence_for_recovery() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            SUGGEST_YOGA_SEQUENCE,
            json!({
                "purpose": "recovery"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_yoga_sequence_with_duration() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            SUGGEST_YOGA_SEQUENCE,
            json!({
                "purpose": "post_run",
                "duration_minutes": 20
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_yoga_sequence_with_difficulty() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler
        .call_tool(
            SUGGEST_YOGA_SEQUENCE,
            json!({
                "purpose": "morning",
                "difficulty": "beginner"
            }),
        )
        .await?;

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response.get("error").is_none());

    Ok(())
}

#[tokio::test]
async fn test_e2e_suggest_yoga_sequence_missing_purpose() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    let response = handler.call_tool(SUGGEST_YOGA_SEQUENCE, json!({})).await?;

    // Should return error for missing required parameter
    assert_eq!(response["jsonrpc"], "2.0");
    let has_error = response.get("error").is_some()
        || response["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|t| t.contains("error") || t.contains("required"));
    assert!(has_error, "Should indicate error for missing purpose");

    Ok(())
}

// =============================================================================
// E2E Tests for Combined Workflows
// =============================================================================

#[tokio::test]
async fn test_e2e_complete_stretching_workflow() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    // Step 1: List all stretching exercises
    let list_response = handler
        .call_tool(LIST_STRETCHING_EXERCISES, json!({}))
        .await?;
    assert!(list_response.get("error").is_none(), "List should succeed");

    // Step 2: Get suggestions for an activity
    let suggest_response = handler
        .call_tool(
            SUGGEST_STRETCHES_FOR_ACTIVITY,
            json!({
                "activity_type": "running",
                "duration_minutes": 10
            }),
        )
        .await?;
    assert!(
        suggest_response.get("error").is_none(),
        "Suggest should succeed"
    );

    Ok(())
}

#[tokio::test]
async fn test_e2e_complete_yoga_workflow() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    // Step 1: List yoga poses with filter
    let list_response = handler
        .call_tool(
            LIST_YOGA_POSES,
            json!({
                "difficulty": "beginner"
            }),
        )
        .await?;
    assert!(list_response.get("error").is_none(), "List should succeed");

    // Step 2: Get a yoga sequence suggestion
    let sequence_response = handler
        .call_tool(
            SUGGEST_YOGA_SEQUENCE,
            json!({
                "purpose": "rest_day",
                "duration_minutes": 15,
                "difficulty": "beginner"
            }),
        )
        .await?;
    assert!(
        sequence_response.get("error").is_none(),
        "Sequence should succeed"
    );

    Ok(())
}

// =============================================================================
// E2E Tests for MCP Protocol Compliance
// =============================================================================

#[tokio::test]
async fn test_e2e_mcp_protocol_compliance() -> Result<()> {
    let handler = MobilityMcpHandler::new().await?;

    // Test all mobility tools respond with valid MCP protocol structure
    let tools = [
        (LIST_STRETCHING_EXERCISES, json!({})),
        (
            SUGGEST_STRETCHES_FOR_ACTIVITY,
            json!({"activity_type": "running"}),
        ),
        (LIST_YOGA_POSES, json!({})),
        (SUGGEST_YOGA_SEQUENCE, json!({"purpose": "recovery"})),
    ];

    for (tool_name, args) in tools {
        let response = handler.call_tool(tool_name, args).await?;

        // Verify MCP JSON-RPC structure
        assert_eq!(
            response["jsonrpc"], "2.0",
            "{tool_name}: Should have jsonrpc 2.0"
        );
        assert!(
            response.get("id").is_some(),
            "{tool_name}: Should have request id"
        );

        // Should have either result or error
        let has_result = response.get("result").is_some();
        let has_error = response.get("error").is_some();
        assert!(
            has_result || has_error,
            "{tool_name}: Should have result or error"
        );
    }

    Ok(())
}
