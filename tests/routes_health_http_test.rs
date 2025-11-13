// ABOUTME: HTTP integration tests for health check routes
// ABOUTME: Tests all health check endpoints without authentication requirements
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for health check routes
//!
//! This test suite validates that all health check endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod helpers;

use helpers::axum_test::AxumTestRequest;

/// Get health routes for testing
fn health_routes() -> axum::Router {
    pierre_mcp_server::routes::health::HealthRoutes::routes()
}

// ============================================================================
// GET /health - Health Check Tests
// ============================================================================

#[tokio::test]
async fn test_health_endpoint_success() {
    let routes = health_routes();

    let response = AxumTestRequest::get("/health").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "healthy");
    assert!(body["timestamp"].is_string());
}

#[tokio::test]
async fn test_health_endpoint_no_auth_required() {
    let routes = health_routes();

    // Health endpoint should work without any authentication
    let response = AxumTestRequest::get("/health").send(routes).await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_health_endpoint_response_structure() {
    let routes = health_routes();

    let response = AxumTestRequest::get("/health").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert!(body["status"].is_string());
    assert!(body["timestamp"].is_string());

    // Verify timestamp is in ISO 8601 format
    let timestamp_str = body["timestamp"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(timestamp_str).is_ok());
}

// ============================================================================
// GET /ready - Readiness Check Tests
// ============================================================================

#[tokio::test]
async fn test_ready_endpoint_success() {
    let routes = health_routes();

    let response = AxumTestRequest::get("/ready").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "ready");
    assert!(body["timestamp"].is_string());
}

#[tokio::test]
async fn test_ready_endpoint_no_auth_required() {
    let routes = health_routes();

    // Ready endpoint should work without any authentication
    let response = AxumTestRequest::get("/ready").send(routes).await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_ready_endpoint_response_structure() {
    let routes = health_routes();

    let response = AxumTestRequest::get("/ready").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert!(body["status"].is_string());
    assert!(body["timestamp"].is_string());

    // Verify timestamp is in ISO 8601 format
    let timestamp_str = body["timestamp"].as_str().unwrap();
    assert!(chrono::DateTime::parse_from_rfc3339(timestamp_str).is_ok());
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_all_health_endpoints_accessible() {
    let routes = health_routes();

    let endpoints = vec!["/health", "/ready"];

    for endpoint in endpoints {
        let response = AxumTestRequest::get(endpoint).send(routes.clone()).await;

        assert_eq!(
            response.status(),
            200,
            "Endpoint {} should return 200",
            endpoint
        );
    }
}

#[tokio::test]
async fn test_health_endpoints_concurrent_requests() {
    // Make multiple health check requests concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async {
            let routes = health_routes();
            AxumTestRequest::get("/health").send(routes).await
        });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status(), 200);
    }
}

#[tokio::test]
async fn test_ready_endpoint_concurrent_requests() {
    // Make multiple ready check requests concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async {
            let routes = health_routes();
            AxumTestRequest::get("/ready").send(routes).await
        });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status(), 200);
    }
}

#[tokio::test]
async fn test_health_and_ready_return_different_status() {
    let routes = health_routes();

    let health_response = AxumTestRequest::get("/health").send(routes.clone()).await;
    let health_body: serde_json::Value = health_response.json();

    let ready_response = AxumTestRequest::get("/ready").send(routes).await;
    let ready_body: serde_json::Value = ready_response.json();

    // They should have different status values
    assert_eq!(health_body["status"], "healthy");
    assert_eq!(ready_body["status"], "ready");
}
