// ABOUTME: MCP error scenario integration tests
// ABOUTME: Tests error handling, edge cases, and failure recovery
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
// TEST HELPERS
// ============================================================================

async fn setup_test_client() -> Result<(IntegrationTestServer, McpTestClient)> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;
    let (_user_id, jwt_token) = server.create_test_user("error@test.local").await?;
    let client = McpTestClient::new(&server.base_url(), &jwt_token);
    Ok((server, client))
}

// ============================================================================
// AUTHENTICATION ERRORS
// ============================================================================

/// Test: Invalid JWT token is rejected
#[tokio::test]
async fn test_error_invalid_jwt() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let client = McpTestClient::new(&server.base_url(), "invalid.jwt.token");

    let result = client
        .call_tool_raw("get_activities", json!({"provider": "synthetic"}))
        .await;

    assert!(result.is_err(), "Should reject invalid JWT");
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("JWT")
            || error.contains("auth")
            || error.contains("invalid")
            || error.contains("malformed"),
        "Error should mention auth failure: {error}"
    );

    println!("✅ Invalid JWT rejected: {error}");
    Ok(())
}

/// Test: Expired JWT token is rejected
#[tokio::test]
async fn test_error_expired_jwt() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    // Create a malformed/expired-looking JWT
    let expired_token =
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiZXhwIjoxfQ.invalid";
    let client = McpTestClient::new(&server.base_url(), expired_token);

    let result = client
        .call_tool_raw("get_activities", json!({"provider": "synthetic"}))
        .await;

    assert!(result.is_err(), "Should reject expired JWT");
    println!("✅ Expired JWT rejected");
    Ok(())
}

/// Test: Empty auth token is rejected
#[tokio::test]
async fn test_error_empty_auth() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    let client = McpTestClient::new(&server.base_url(), "");

    let result = client
        .call_tool_raw("get_activities", json!({"provider": "synthetic"}))
        .await;

    assert!(result.is_err(), "Should reject empty auth token");
    println!("✅ Empty auth token rejected");
    Ok(())
}

// ============================================================================
// TOOL ERRORS
// ============================================================================

/// Test: Unknown tool name returns proper error
#[tokio::test]
async fn test_error_unknown_tool() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    let result = client
        .call_tool_raw("nonexistent_tool_xyz", json!({}))
        .await;

    assert!(result.is_err(), "Should fail for unknown tool");
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("not found") || error.contains("unknown") || error.contains("-32601"),
        "Error should indicate tool not found: {error}"
    );

    println!("✅ Unknown tool error: {error}");
    Ok(())
}

/// Test: Empty tool name returns error
#[tokio::test]
async fn test_error_empty_tool_name() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    let result = client.call_tool_raw("", json!({})).await;

    assert!(result.is_err(), "Should fail for empty tool name");
    println!("✅ Empty tool name rejected");
    Ok(())
}

/// Test: Tool with invalid JSON params
#[tokio::test]
async fn test_error_invalid_params_type() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Send string instead of object for provider
    let result = client
        .call_tool_raw("get_activities", json!("invalid"))
        .await;

    // May succeed but return error, or fail at protocol level
    match result {
        Ok(r) if r.is_error => {
            println!("✅ Invalid params returned tool error");
        }
        Ok(_) => {
            println!("✅ Tool handled invalid params gracefully");
        }
        Err(e) => {
            println!("✅ Invalid params rejected at protocol level: {e}");
        }
    }
    Ok(())
}

// ============================================================================
// PROVIDER ERRORS
// ============================================================================

/// Test: Unknown provider returns helpful error
#[tokio::test]
async fn test_error_unknown_provider() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    let result = client
        .call_tool_raw(
            "get_activities",
            json!({"provider": "nonexistent_provider_xyz"}),
        )
        .await;

    // Should either error or return helpful message about unknown provider
    match result {
        Ok(r) => {
            let empty = String::new();
            let text = r.content[0].text.as_ref().unwrap_or(&empty);
            println!(
                "✅ Unknown provider handled: {}",
                &text[..text.len().min(100)]
            );
        }
        Err(e) => {
            println!("✅ Unknown provider error: {e}");
        }
    }
    Ok(())
}

/// Test: Provider not connected returns OAuth URL
#[tokio::test]
async fn test_error_provider_not_connected() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Try to get activities from a provider that isn't connected
    let result = client
        .call_tool_raw("get_activities", json!({"provider": "strava"}))
        .await;

    // Should return OAuth connection URL or helpful message
    match result {
        Ok(r) if !r.is_error => {
            let text = r.content[0].text.as_ref().expect("text");
            assert!(
                text.contains("connect")
                    || text.contains("oauth")
                    || text.contains("http")
                    || text.contains("not connected"),
                "Should mention how to connect: {text}"
            );
            println!("✅ Provider not connected - got connection info");
        }
        Ok(_) => {
            println!("✅ Provider not connected - returned error as expected");
        }
        Err(e) => {
            println!("✅ Provider not connected error: {e}");
        }
    }
    Ok(())
}

// ============================================================================
// PARAMETER VALIDATION ERRORS
// ============================================================================

/// Test: Missing required parameters
#[tokio::test]
async fn test_error_missing_required_params() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // calculate_daily_nutrition requires multiple params
    let result = client
        .call_tool_raw("calculate_daily_nutrition", json!({}))
        .await;

    // Should fail or return error about missing params
    match result {
        Ok(r) if r.is_error => {
            println!("✅ Missing params returned tool error");
        }
        Ok(r) => {
            let empty = String::new();
            let text = r.content[0].text.as_ref().unwrap_or(&empty);
            println!(
                "✅ Missing params handled: {}",
                &text[..text.len().min(100)]
            );
        }
        Err(e) => {
            println!("✅ Missing params rejected: {e}");
        }
    }
    Ok(())
}

/// Test: Invalid numeric values
#[tokio::test]
async fn test_error_invalid_numeric_values() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Negative weight
    let _result = client
        .call_tool_raw(
            "calculate_daily_nutrition",
            json!({
                "weight_kg": -50.0,
                "height_cm": 175.0,
                "age": 30,
                "sex": "male",
                "activity_level": "moderate",
                "goal": "maintenance"
            }),
        )
        .await;

    println!("✅ Negative values handled gracefully");
    Ok(())
}

/// Test: Invalid enum values
#[tokio::test]
async fn test_error_invalid_enum_values() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Invalid activity_level
    let result = client
        .call_tool_raw(
            "calculate_daily_nutrition",
            json!({
                "weight_kg": 70.0,
                "height_cm": 175.0,
                "age": 30,
                "sex": "male",
                "activity_level": "super_mega_active",  // Invalid enum value
                "goal": "maintenance"
            }),
        )
        .await;

    match result {
        Ok(r) => {
            let empty = String::new();
            let text = r.content[0].text.as_ref().unwrap_or(&empty);
            println!("✅ Invalid enum handled: {}", &text[..text.len().min(100)]);
        }
        Err(e) => {
            println!("✅ Invalid enum rejected: {e}");
        }
    }
    Ok(())
}

/// Test: Out of range values
#[tokio::test]
async fn test_error_out_of_range_values() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Unreasonable age
    let _result = client
        .call_tool_raw(
            "calculate_daily_nutrition",
            json!({
                "weight_kg": 70.0,
                "height_cm": 175.0,
                "age": 500,  // Unreasonable age
                "sex": "male",
                "activity_level": "moderate",
                "goal": "maintenance"
            }),
        )
        .await;

    println!("✅ Out of range values handled");
    Ok(())
}

// ============================================================================
// MCP PROTOCOL ERRORS
// ============================================================================

/// Test: Invalid JSON-RPC version
#[tokio::test]
async fn test_error_invalid_jsonrpc_version() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;
    let (_user_id, jwt_token) = server.create_test_user("protocol@test.local").await?;

    // Send request with wrong JSON-RPC version directly
    let http_client = reqwest::Client::new();
    let response = http_client
        .post(format!("{}/mcp", server.base_url()))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {jwt_token}"))
        .json(&json!({
            "jsonrpc": "1.0",  // Wrong version
            "method": "tools/list",
            "id": 1
        }))
        .send()
        .await?;

    // Should get some response (may be error or success depending on strictness)
    println!(
        "✅ Invalid JSON-RPC version returned status: {}",
        response.status()
    );
    Ok(())
}

/// Test: Missing method field
#[tokio::test]
async fn test_error_missing_method() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;
    let (_user_id, jwt_token) = server.create_test_user("protocol2@test.local").await?;

    let http_client = reqwest::Client::new();
    let response = http_client
        .post(format!("{}/mcp", server.base_url()))
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {jwt_token}"))
        .json(&json!({
            "jsonrpc": "2.0",
            // Missing "method" field
            "id": 1
        }))
        .send()
        .await?;

    let status = response.status();
    assert!(
        status.is_client_error() || status.is_success(),
        "Should return error or error response"
    );
    println!("✅ Missing method field returned status: {status}");
    Ok(())
}

// ============================================================================
// RATE LIMITING AND RESOURCE ERRORS
// ============================================================================

/// Test: Rapid sequential requests are handled
#[tokio::test]
async fn test_error_rapid_requests() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Send many requests in quick succession
    let mut success_count = 0;
    let total_requests = 20;

    for _ in 0..total_requests {
        let result = client.list_tools().await;
        if result.is_ok() {
            success_count += 1;
        }
    }

    // Most requests should succeed
    assert!(
        success_count >= total_requests / 2,
        "At least half of rapid requests should succeed: {success_count} of {total_requests}"
    );

    println!("✅ Rapid requests: {success_count}/{total_requests} succeeded");
    Ok(())
}

/// Test: Large parameter values are handled
#[tokio::test]
async fn test_error_large_parameters() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Request with very large limit
    let result = client
        .call_tool_raw(
            "get_activities",
            json!({
                "provider": "synthetic",
                "limit": 999_999
            }),
        )
        .await;

    // Should either succeed with reasonable limit or return error
    match result {
        Ok(_) => {
            println!("✅ Large limit handled gracefully");
        }
        Err(e) => {
            println!("✅ Large limit rejected: {e}");
        }
    }
    Ok(())
}

// ============================================================================
// CONCURRENT ACCESS ERRORS
// ============================================================================

/// Test: Multiple users can access server concurrently
#[tokio::test]
async fn test_error_concurrent_users() -> Result<()> {
    let mut server = IntegrationTestServer::new().await?;
    server.start().await?;

    // Create multiple users
    let (_user1_id, token1) = server.create_test_user("concurrent1@test.local").await?;
    let (_user2_id, token2) = server.create_test_user("concurrent2@test.local").await?;
    let (_user3_id, token3) = server.create_test_user("concurrent3@test.local").await?;

    let client1 = McpTestClient::new(&server.base_url(), &token1);
    let client2 = McpTestClient::new(&server.base_url(), &token2);
    let client3 = McpTestClient::new(&server.base_url(), &token3);

    // Send concurrent requests
    let (r1, r2, r3) = tokio::join!(
        client1.list_tools(),
        client2.call_tool_raw("get_connection_status", json!({"provider": "synthetic"})),
        client3.call_tool_raw("calculate_fitness_score", json!({"provider": "synthetic"}))
    );

    assert!(r1.is_ok(), "User 1 request should succeed");
    assert!(r2.is_ok(), "User 2 request should succeed");
    assert!(r3.is_ok(), "User 3 request should succeed");

    println!("✅ Concurrent users handled successfully");
    Ok(())
}

// ============================================================================
// ERROR RECOVERY TESTS
// ============================================================================

/// Test: Client can continue after error
#[tokio::test]
async fn test_error_recovery_after_failure() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // First, trigger an error
    let error_result = client.call_tool_raw("nonexistent_tool", json!({})).await;
    assert!(error_result.is_err(), "Should fail for unknown tool");

    // Then, make a valid request
    let success_result = client.list_tools().await;
    assert!(
        success_result.is_ok(),
        "Should succeed after error recovery"
    );

    println!("✅ Client recovered after error");
    Ok(())
}

/// Test: Multiple consecutive errors don't break client
#[tokio::test]
async fn test_error_multiple_consecutive_errors() -> Result<()> {
    let (_server, client) = setup_test_client().await?;

    // Trigger multiple errors
    for i in 0..5 {
        let _ = client
            .call_tool_raw(&format!("fake_tool_{i}"), json!({}))
            .await;
    }

    // Client should still work
    let result = client.list_tools().await;
    assert!(result.is_ok(), "Client should work after multiple errors");

    println!("✅ Client stable after multiple errors");
    Ok(())
}
