// ABOUTME: HTTP integration tests for MCP (Model Context Protocol) routes
// ABOUTME: Tests all MCP endpoints including tool discovery and JSON-RPC request handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for MCP routes
//!
//! This test suite validates that all MCP endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    constants::tools::PUBLIC_DISCOVERY_TOOLS,
    mcp::resources::{ServerResources, ServerResourcesOptions},
    routes::mcp::McpRoutes,
};
use serde_json::json;
use std::sync::Arc;

/// Test setup helper for MCP route testing
#[allow(dead_code)]
struct McpTestSetup {
    resources: Arc<ServerResources>,
    user_id: uuid::Uuid,
    jwt_token: String,
}

impl McpTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let cache = common::create_test_cache().await?;

        // Create test user
        let (user_id, user) = common::create_test_user(&database).await?;

        // Create ServerResources
        let temp_dir = tempfile::tempdir()?;
        let config = Arc::new(ServerConfig {
            http_port: 8081,
            database: DatabaseConfig {
                url: DatabaseUrl::Memory,
                backup: BackupConfig {
                    directory: temp_dir.path().to_path_buf(),
                    ..Default::default()
                },
                ..Default::default()
            },
            app_behavior: AppBehaviorConfig {
                ci_mode: true,
                auto_approve_users: false,
                ..Default::default()
            },
            security: SecurityConfig {
                headers: SecurityHeadersConfig {
                    environment: Environment::Testing,
                },
                ..Default::default()
            },
            ..Default::default()
        });

        let resources = Arc::new(
            ServerResources::new(
                (*database).clone(),
                (*auth_manager).clone(),
                "test_jwt_secret",
                config,
                cache,
                ServerResourcesOptions {
                    rsa_key_size_bits: Some(2048),
                    jwks_manager: Some(common::get_shared_test_jwks()),
                    llm_provider: None,
                },
            )
            .await,
        );

        // Generate JWT token for the user
        let jwt_token = auth_manager
            .generate_token(&user, &resources.jwks_manager)
            .map_err(|e| anyhow::anyhow!("Failed to generate JWT: {}", e))?;

        Ok(Self {
            resources,
            user_id,
            jwt_token,
        })
    }

    fn routes(&self) -> axum::Router {
        McpRoutes::routes(self.resources.clone())
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.jwt_token)
    }
}

// ============================================================================
// GET /mcp/tools - Tool Discovery Tests
// ============================================================================

#[tokio::test]
async fn test_mcp_tools_discovery_success() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/mcp/tools").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["tools"].is_array());
}

#[tokio::test]
async fn test_mcp_tools_no_auth_required() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Tools endpoint should work without authentication
    let response = AxumTestRequest::get("/mcp/tools").send(routes).await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_mcp_tools_response_structure() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/mcp/tools").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert!(body["tools"].is_array());

    // Verify each tool has required fields
    let tools = body["tools"].as_array().unwrap();
    for tool in tools {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
    }
}

#[tokio::test]
async fn test_mcp_tools_returns_available_tools() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/mcp/tools").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tools = body["tools"].as_array().unwrap();

    // Should have at least some tools registered
    assert!(!tools.is_empty(), "Should return at least one tool");
}

#[tokio::test]
async fn test_mcp_tools_concurrent_requests() {
    let setup = McpTestSetup::new().await.expect("Setup failed");

    // Make multiple tool discovery requests concurrently
    let mut handles = vec![];

    for _ in 0..5 {
        let routes = setup.routes();
        let handle =
            tokio::spawn(async move { AxumTestRequest::get("/mcp/tools").send(routes).await });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status(), 200);
    }
}

// ============================================================================
// POST /mcp - JSON-RPC Request Tests
// ============================================================================

#[tokio::test]
async fn test_mcp_request_with_auth() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .json(&mcp_request)
        .send(routes)
        .await;

    // Should process the request (200) or return accepted (202)
    assert!(response.status() == 200 || response.status() == 202);
}

#[tokio::test]
async fn test_mcp_request_without_auth() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = AxumTestRequest::post("/mcp")
        .json(&mcp_request)
        .send(routes)
        .await;

    // MCP can work without auth for some methods, should not be 401
    // Might be 200, 202, or 400 depending on method requirements
    assert_ne!(response.status(), 500);
}

#[tokio::test]
async fn test_mcp_request_with_session() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id = format!("session_{}", uuid::Uuid::new_v4());

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .header("mcp-session-id", &session_id)
        .json(&mcp_request)
        .send(routes)
        .await;

    // Should process and may return session ID header
    assert!(response.status() == 200 || response.status() == 202);
}

#[tokio::test]
async fn test_mcp_request_invalid_json() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .header("content-type", "application/json")
        .send(routes)
        .await;

    // Should fail with bad request
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_mcp_request_invalid_jsonrpc_format() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let invalid_request = json!({
        "not_jsonrpc": "invalid"
    });

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .json(&invalid_request)
        .send(routes)
        .await;

    // Should fail validation
    assert_eq!(response.status(), 400);
}

#[tokio::test]
async fn test_mcp_request_empty_body() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    // Should handle empty body gracefully
    assert!(response.status() == 400 || response.status() == 200);
}

#[tokio::test]
async fn test_mcp_request_with_notification() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // JSON-RPC notification (no id field)
    let notification_request = json!({
        "jsonrpc": "2.0",
        "method": "notification/test",
        "params": {}
    });

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .json(&notification_request)
        .send(routes)
        .await;

    // Notifications may return 202 Accepted or 200
    assert!(response.status() == 200 || response.status() == 202);
}

#[tokio::test]
async fn test_mcp_request_tools_list_method() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .json(&mcp_request)
        .send(routes)
        .await;

    assert!(response.status() == 200 || response.status() == 202);

    if response.status() == 200 {
        let body: serde_json::Value = response.json();
        // JSON-RPC response should have id and result or error
        assert!(body["id"].is_number() || body["id"].is_string());
    }
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_all_mcp_endpoints_registered() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let endpoints = vec![("/mcp/tools", "GET"), ("/mcp", "POST")];

    for (endpoint, method) in endpoints {
        let response = if method == "GET" {
            AxumTestRequest::get(endpoint).send(routes.clone()).await
        } else {
            AxumTestRequest::post(endpoint)
                .json(&json!({}))
                .send(routes.clone())
                .await
        };

        // Should not be 404 (endpoint not found)
        assert_ne!(
            response.status(),
            404,
            "{} {} should be registered",
            method,
            endpoint
        );
    }
}

#[tokio::test]
async fn test_mcp_request_user_isolation() {
    let setup1 = McpTestSetup::new().await.expect("Setup 1 failed");
    let setup2 = McpTestSetup::new().await.expect("Setup 2 failed");

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    // User 1 makes a request
    let routes1 = setup1.routes();
    let response1 = AxumTestRequest::post("/mcp")
        .header("authorization", &setup1.auth_header())
        .json(&mcp_request)
        .send(routes1)
        .await;

    // User 2 makes a request
    let routes2 = setup2.routes();
    let response2 = AxumTestRequest::post("/mcp")
        .header("authorization", &setup2.auth_header())
        .json(&mcp_request)
        .send(routes2)
        .await;

    // Both should succeed independently
    assert!(response1.status() == 200 || response1.status() == 202);
    assert!(response2.status() == 200 || response2.status() == 202);
}

#[tokio::test]
async fn test_mcp_concurrent_requests() {
    let setup = McpTestSetup::new().await.expect("Setup failed");

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    // Make multiple MCP requests concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let routes = setup.routes();
        let auth = setup.auth_header();
        let mut request = mcp_request.clone();
        request["id"] = json!(i);

        let handle = tokio::spawn(async move {
            AxumTestRequest::post("/mcp")
                .header("authorization", &auth)
                .json(&request)
                .send(routes)
                .await
        });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert!(response.status() == 200 || response.status() == 202);
    }
}

#[tokio::test]
async fn test_mcp_session_persistence() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id = format!("session_{}", uuid::Uuid::new_v4());

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    // First request with auth to establish session
    let response1 = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .header("mcp-session-id", &session_id)
        .json(&mcp_request)
        .send(routes.clone())
        .await;

    assert!(response1.status() == 200 || response1.status() == 202);

    // Second request with same session (no auth header)
    let response2 = AxumTestRequest::post("/mcp")
        .header("mcp-session-id", &session_id)
        .json(&mcp_request)
        .send(routes)
        .await;

    // Session-based auth should work
    assert!(response2.status() == 200 || response2.status() == 202);
}

// ============================================================================
// tools/list Visibility Gating Tests
// ============================================================================

#[tokio::test]
async fn test_tools_list_unauthenticated_returns_public_tools_only() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    // Send request WITHOUT auth header
    let response = AxumTestRequest::post("/mcp")
        .json(&mcp_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    // Verify only public discovery tools are returned
    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Every returned tool must be in PUBLIC_DISCOVERY_TOOLS
    for name in &tool_names {
        assert!(
            PUBLIC_DISCOVERY_TOOLS.contains(name),
            "Unauthenticated tools/list returned non-public tool: {name}"
        );
    }

    // Sensitive tools must NOT appear
    assert!(
        !tool_names.contains(&"connect_provider"),
        "connect_provider should not appear in unauthenticated tools/list"
    );
    assert!(
        !tool_names.contains(&"disconnect_provider"),
        "disconnect_provider should not appear in unauthenticated tools/list"
    );
    assert!(
        !tool_names.iter().any(|n| n.starts_with("admin_")),
        "Admin tools should not appear in unauthenticated tools/list"
    );
    assert!(
        !tool_names.contains(&"set_goal"),
        "Write tool set_goal should not appear in unauthenticated tools/list"
    );
    assert!(
        !tool_names.contains(&"save_recipe"),
        "Write tool save_recipe should not appear in unauthenticated tools/list"
    );

    // At least some public tools should be present
    assert!(
        !tool_names.is_empty(),
        "Public tool list should not be empty"
    );
    assert!(
        tool_names.contains(&"get_activities"),
        "get_activities should be in public tools"
    );
}

#[tokio::test]
async fn test_tools_list_authenticated_returns_more_tools() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes_unauth = setup.routes();
    let routes_auth = setup.routes();

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    // Unauthenticated request
    let response_unauth = AxumTestRequest::post("/mcp")
        .json(&mcp_request)
        .send(routes_unauth)
        .await;

    assert_eq!(response_unauth.status(), 200);
    let body_unauth: serde_json::Value = response_unauth.json();
    let tools_unauth = body_unauth["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    // Authenticated request
    let response_auth = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .json(&mcp_request)
        .send(routes_auth)
        .await;

    assert_eq!(response_auth.status(), 200);
    let body_auth: serde_json::Value = response_auth.json();
    let tools_auth = body_auth["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    // Authenticated user should see at least as many tools as unauthenticated
    assert!(
        tools_auth.len() >= tools_unauth.len(),
        "Authenticated should see >= tools than unauthenticated ({} vs {})",
        tools_auth.len(),
        tools_unauth.len()
    );
}

#[tokio::test]
async fn test_tools_list_invalid_token_falls_back_to_public() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    // Send request with invalid auth token
    let response = AxumTestRequest::post("/mcp")
        .header("authorization", "Bearer invalid_token_12345")
        .json(&mcp_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Should fall back to public tools (not error out)
    for name in &tool_names {
        assert!(
            PUBLIC_DISCOVERY_TOOLS.contains(name),
            "Invalid-token tools/list returned non-public tool: {name}"
        );
    }

    // Should not contain sensitive tools
    assert!(
        !tool_names.contains(&"connect_provider"),
        "connect_provider should not appear with invalid token"
    );
}

#[tokio::test]
async fn test_tools_list_admin_user_sees_all_tools_including_admin() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // McpTestSetup creates an owner user, which has admin privileges
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = AxumTestRequest::post("/mcp")
        .header("authorization", &setup.auth_header())
        .json(&mcp_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // Admin user should see admin tools
    assert!(
        tool_names.iter().any(|n| n.starts_with("admin_")),
        "Admin user should see admin tools in tools/list, got: {:?}",
        tool_names
    );

    // Admin user should also see all non-admin tools
    assert!(
        tool_names.contains(&"get_activities"),
        "Admin should see get_activities"
    );
    assert!(
        tool_names.contains(&"connect_provider"),
        "Admin should see connect_provider"
    );

    // Admin should see more tools than the public set
    assert!(
        tool_names.len() > PUBLIC_DISCOVERY_TOOLS.len(),
        "Admin should see more tools ({}) than public discovery ({})",
        tool_names.len(),
        PUBLIC_DISCOVERY_TOOLS.len()
    );
}

#[tokio::test]
async fn test_tools_list_params_token_auth_path() {
    let setup = McpTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Send token via params.token instead of Authorization header
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {
            "token": setup.jwt_token
        }
    });

    // No Authorization header - token is in params
    let response = AxumTestRequest::post("/mcp")
        .json(&mcp_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tools = body["result"]["tools"]
        .as_array()
        .expect("tools should be an array");

    let tool_names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();

    // params.token auth should return authenticated tool set (more than public)
    assert!(
        tool_names.len() > PUBLIC_DISCOVERY_TOOLS.len(),
        "params.token auth should return more tools ({}) than public discovery ({})",
        tool_names.len(),
        PUBLIC_DISCOVERY_TOOLS.len()
    );

    // Should include tools that are NOT in the public set (e.g. connection management)
    assert!(
        tool_names.contains(&"connect_provider"),
        "params.token auth should include connect_provider"
    );
}
