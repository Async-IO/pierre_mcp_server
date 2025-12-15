// ABOUTME: HTTP integration tests for WebSocket routes
// ABOUTME: Tests WebSocket endpoint registration and upgrade handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for WebSocket routes
//!
//! This test suite validates that WebSocket endpoints are correctly registered
//! in the router and handle upgrade requests appropriately.
//!
//! Note: Full WebSocket protocol communication testing requires specialized
//! WebSocket client libraries. These tests focus on HTTP-level upgrade validation.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::RateLimitConfig, routes::websocket::WebSocketRoutes,
    websocket::WebSocketManager,
};
use std::sync::Arc;

/// Test setup helper for WebSocket route testing
struct WebSocketTestSetup {
    manager: Arc<WebSocketManager>,
}

impl WebSocketTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let jwks_manager = common::get_shared_test_jwks();

        // Create rate limit config using defaults
        let rate_limit_config = RateLimitConfig::default();

        // Create WebSocket manager
        let manager = Arc::new(WebSocketManager::new(
            database,
            &auth_manager,
            &jwks_manager,
            rate_limit_config,
        ));

        Ok(Self { manager })
    }

    fn routes(&self) -> axum::Router {
        WebSocketRoutes::routes(self.manager.clone())
    }
}

// ============================================================================
// GET /ws - WebSocket Upgrade Tests
// ============================================================================

#[tokio::test]
async fn test_websocket_endpoint_registered() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/ws").send(routes).await;

    // WebSocket endpoint should be registered (not 404)
    // Will return 426 Upgrade Required or other status, but not 404
    assert_ne!(
        response.status(),
        404,
        "WebSocket endpoint should be registered"
    );
}

#[tokio::test]
async fn test_websocket_without_upgrade_header() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Request without WebSocket upgrade headers
    let response = AxumTestRequest::get("/ws").send(routes).await;

    // Should require WebSocket upgrade (not 404)
    assert_ne!(response.status(), 404);
}

#[tokio::test]
async fn test_websocket_endpoint_accessible() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // WebSocket endpoint should be accessible even without proper upgrade headers
    // It should not return 404 (not found) or 500 (server error)
    let response = AxumTestRequest::get("/ws").send(routes).await;

    // Should be registered and respond (even if rejecting upgrade)
    assert!(
        response.status() != 404 && response.status() != 500,
        "WebSocket endpoint should be accessible"
    );
}

#[tokio::test]
async fn test_websocket_no_auth_required_for_connection() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // WebSocket connection attempt without authentication
    let response = AxumTestRequest::get("/ws").send(routes).await;

    // Should not require auth to attempt connection (not 401)
    // Authentication happens after WebSocket upgrade
    assert_ne!(response.status(), 401);
}

#[tokio::test]
async fn test_websocket_get_method_only() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // WebSocket upgrades only work with GET method
    let post_response = AxumTestRequest::post("/ws").send(routes).await;

    // POST should not be allowed for WebSocket endpoint
    assert_eq!(post_response.status(), 405); // Method Not Allowed
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_websocket_concurrent_connection_attempts() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");

    // Make multiple WebSocket connection attempts concurrently
    let mut handles = vec![];

    for _ in 0..5 {
        let routes = setup.routes();

        let handle = tokio::spawn(async move { AxumTestRequest::get("/ws").send(routes).await });

        handles.push(handle);
    }

    // All connection attempts should be handled
    for handle in handles {
        let response = handle.await.expect("Task panicked");

        // Should handle request (not crash)
        assert_ne!(response.status(), 500);
    }
}

#[tokio::test]
async fn test_websocket_endpoint_idempotency() {
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Make multiple requests to verify endpoint is stable
    let responses = vec![
        AxumTestRequest::get("/ws").send(routes.clone()).await,
        AxumTestRequest::get("/ws").send(routes.clone()).await,
        AxumTestRequest::get("/ws").send(routes).await,
    ];

    // All should return consistent status
    let first_status = responses[0].status();
    for response in &responses {
        assert_eq!(response.status(), first_status);
    }
}

#[tokio::test]
async fn test_websocket_route_isolation() {
    // Create multiple independent WebSocket setups
    let setup1 = WebSocketTestSetup::new().await.expect("Setup 1 failed");
    let setup2 = WebSocketTestSetup::new().await.expect("Setup 2 failed");

    let routes1 = setup1.routes();
    let routes2 = setup2.routes();

    let response1 = AxumTestRequest::get("/ws").send(routes1).await;
    let response2 = AxumTestRequest::get("/ws").send(routes2).await;

    // Both should handle requests independently
    assert_ne!(response1.status(), 500);
    assert_ne!(response2.status(), 500);
}

#[tokio::test]
async fn test_websocket_manager_initialization() {
    // Verify that WebSocketManager can be initialized successfully
    let setup = WebSocketTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/ws").send(routes).await;

    // Should not crash with manager initialization
    assert_ne!(response.status(), 500);
}

#[tokio::test]
async fn test_websocket_multiple_managers() {
    // Create routes with different WebSocket managers
    let setup1 = WebSocketTestSetup::new().await.expect("Setup 1 failed");
    let setup2 = WebSocketTestSetup::new().await.expect("Setup 2 failed");

    let routes1 = setup1.routes();
    let routes2 = setup2.routes();

    let response1 = AxumTestRequest::get("/ws").send(routes1).await;
    let response2 = AxumTestRequest::get("/ws").send(routes2).await;

    // Both managers should work independently
    assert_ne!(response1.status(), 500);
    assert_ne!(response2.status(), 500);
}
