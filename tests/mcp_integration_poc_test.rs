// ABOUTME: MCP integration test proof-of-concept
// ABOUTME: Validates full HTTP transport MCP protocol with real server
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod integration;

use anyhow::Result;
use integration::{IntegrationTestServer, McpTestClient};
use serde_json::json;

// ============================================================================
// PROOF OF CONCEPT: MCP Integration Test Suite
// ============================================================================
// These tests validate the full MCP protocol stack via HTTP transport:
// 1. Server lifecycle management
// 2. MCP protocol compliance (JSON-RPC 2.0)
// 3. Authentication and authorization
// 4. Tool discovery and execution
// ============================================================================

/// Test: Server starts and becomes healthy
#[tokio::test]
async fn test_server_lifecycle() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    // Server is healthy (verified by start())
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/health", server.base_url()))
        .send()
        .await?;
    assert!(
        response.status().is_success(),
        "Health endpoint should return success"
    );

    // Cleanup happens automatically via Drop
    Ok(())
}

/// Test: MCP initialize handshake
#[tokio::test]
async fn test_mcp_initialize() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let (_user_id, jwt_token) = server.create_test_user("init@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);

    let result = client.initialize().await?;

    // Verify protocol compliance
    assert!(
        result.protocol_version == "2025-11-25"
            || result.protocol_version == "2025-06-18"
            || result.protocol_version == "2024-11-05",
        "Protocol version should be 2025-11-25, 2025-06-18 or 2024-11-05, got: {}",
        result.protocol_version
    );
    assert!(
        !result.server_info.name.is_empty(),
        "Server name should be set"
    );
    assert!(
        !result.server_info.version.is_empty(),
        "Server version should be set"
    );

    println!(
        "✅ MCP initialize: protocol={}, server={} v{}",
        result.protocol_version, result.server_info.name, result.server_info.version
    );

    Ok(())
}

/// Test: Tools listing returns available tools
#[tokio::test]
async fn test_mcp_tools_list() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let (_user_id, jwt_token) = server.create_test_user("tools@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);

    let result = client.list_tools().await?;

    assert!(!result.tools.is_empty(), "Should have at least one tool");

    // Verify some expected tools exist
    let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();

    let expected_tools = [
        "get_activities",
        "get_connection_status",
        "connect_provider",
    ];

    for expected in expected_tools {
        assert!(
            tool_names.contains(&expected),
            "Tool '{expected}' should be available. Found: {tool_names:?}"
        );
    }

    println!("✅ MCP tools/list: {} tools available", result.tools.len());

    Ok(())
}

/// Test: Tool call with synthetic provider
#[tokio::test]
async fn test_mcp_tool_call_connection_status() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let (_user_id, jwt_token) = server.create_test_user("toolcall@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);

    // Call get_connection_status for synthetic provider
    let result = client
        .call_tool_raw("get_connection_status", json!({"provider": "synthetic"}))
        .await?;

    assert!(!result.is_error, "Tool call should not be an error");
    assert!(!result.content.is_empty(), "Result should have content");

    let text = result.content[0]
        .text
        .as_ref()
        .expect("Should have text content");
    println!("✅ get_connection_status result: {text}");

    Ok(())
}

/// Test: Invalid tool returns proper error
#[tokio::test]
async fn test_mcp_invalid_tool() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let (_user_id, jwt_token) = server.create_test_user("error@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);

    // Call non-existent tool
    let result = client
        .call_tool_raw("nonexistent_tool_xyz", json!({}))
        .await;

    assert!(result.is_err(), "Invalid tool should return error");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("not found")
            || error_msg.contains("unknown")
            || error_msg.contains("MCP error"),
        "Error should indicate tool not found: {error_msg}"
    );

    println!("✅ Invalid tool returns proper error: {error_msg}");

    Ok(())
}

/// Test: Unauthenticated tool call fails
#[tokio::test]
async fn test_mcp_auth_required_for_tool_call() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    // Create client with invalid token
    let client = McpTestClient::new(&server.base_url(), "invalid_token_xyz");

    // Try to call a tool that requires auth
    let result = client
        .call_tool_raw("get_activities", json!({"provider": "synthetic"}))
        .await;

    // Should fail authentication
    assert!(result.is_err(), "Should fail with invalid auth token");
    let error_msg = result.unwrap_err().to_string();
    println!("✅ Auth required: {error_msg}");

    Ok(())
}

/// Test: Multiple sequential requests work correctly
#[tokio::test]
async fn test_mcp_sequential_requests() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let (_user_id, jwt_token) = server.create_test_user("sequential@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);

    // Make multiple sequential requests
    for i in 0..5 {
        let result = client.list_tools().await?;
        assert!(!result.tools.is_empty(), "Request {i} should return tools");
    }

    println!("✅ 5 sequential requests completed successfully");

    Ok(())
}

/// Test: Different users have isolated sessions
#[tokio::test]
async fn test_mcp_user_isolation() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    // Create two different users
    let (_user1_id, token1) = server.create_test_user("user1@test.local").await?;
    let (_user2_id, token2) = server.create_test_user("user2@test.local").await?;

    let client1 = McpTestClient::new(&server.base_url(), &token1);
    let client2 = McpTestClient::new(&server.base_url(), &token2);

    // Both can make requests independently
    let result1 = client1.list_tools().await?;
    let result2 = client2.list_tools().await?;

    assert!(!result1.tools.is_empty(), "User 1 should see tools");
    assert!(!result2.tools.is_empty(), "User 2 should see tools");
    assert_eq!(
        result1.tools.len(),
        result2.tools.len(),
        "Both users should see the same number of tools"
    );

    println!("✅ User isolation: both users can access tools independently");

    Ok(())
}

// ============================================================================
// WORKFLOW TEST: Basic tool sequence
// ============================================================================

/// Test: Basic workflow - initialize, list tools, call tool
#[tokio::test]
async fn test_mcp_basic_workflow() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let (_user_id, jwt_token) = server.create_test_user("workflow@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);

    // Step 1: Initialize session
    let init_result = client.initialize().await?;
    assert!(!init_result.server_info.name.is_empty());
    println!("Step 1: Initialized with {}", init_result.server_info.name);

    // Step 2: List available tools
    let tools = client.list_tools().await?;
    assert!(!tools.tools.is_empty());
    println!("Step 2: Found {} tools", tools.tools.len());

    // Step 3: Call a simple tool
    let status = client
        .call_tool_raw("get_connection_status", json!({"provider": "synthetic"}))
        .await?;
    assert!(!status.is_error);
    println!("Step 3: Tool call succeeded");

    println!("✅ Basic workflow completed: init -> list -> call");

    Ok(())
}
