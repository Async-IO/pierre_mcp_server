// ABOUTME: HTTP integration tests for A2A (Agent-to-Agent) protocol routes
// ABOUTME: Tests all A2A endpoints without authentication requirements
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for A2A protocol routes
//!
//! This test suite validates that all A2A endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod helpers;

use helpers::axum_test::AxumTestRequest;

/// Get A2A routes for testing
fn a2a_routes() -> axum::Router {
    pierre_mcp_server::routes::a2a::A2ARoutes::routes()
}

// ============================================================================
// GET /a2a/status - A2A Status Tests
// ============================================================================

#[tokio::test]
async fn test_a2a_status_success() {
    let routes = a2a_routes();

    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn test_a2a_status_no_auth_required() {
    let routes = a2a_routes();

    // A2A status endpoint should work without any authentication
    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_a2a_status_response_structure() {
    let routes = a2a_routes();

    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert!(body["status"].is_string());
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn test_a2a_status_content_type() {
    let routes = a2a_routes();

    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);

    // Response should be valid JSON
    let body: serde_json::Value = response.json();
    assert!(body.is_object());
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_a2a_status_concurrent_requests() {
    // Make multiple A2A status requests concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async {
            let routes = a2a_routes();
            AxumTestRequest::get("/a2a/status").send(routes).await
        });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "active");
    }
}

#[tokio::test]
async fn test_a2a_status_idempotency() {
    let routes = a2a_routes();

    // Make multiple requests and verify they all return the same result
    let responses = vec![
        AxumTestRequest::get("/a2a/status")
            .send(routes.clone())
            .await,
        AxumTestRequest::get("/a2a/status")
            .send(routes.clone())
            .await,
        AxumTestRequest::get("/a2a/status").send(routes).await,
    ];

    for response in responses {
        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "active");
    }
}

#[tokio::test]
async fn test_a2a_status_always_active() {
    let routes = a2a_routes();

    // Verify that status is always "active"
    for _ in 0..5 {
        let response = AxumTestRequest::get("/a2a/status")
            .send(routes.clone())
            .await;

        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "active");
    }
}
