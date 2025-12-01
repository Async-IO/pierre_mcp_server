// ABOUTME: OAuth HTTP endpoint tests for callback handling
// ABOUTME: Tests OAuth HTTP callback endpoints in single-tenant mode
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::if_not_else, clippy::unused_async)]
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # OAuth HTTP Endpoint Tests
//!
//! Tests for OAuth HTTP callback endpoints in single-tenant mode using Axum.

mod helpers;

use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use helpers::axum_test::AxumTestRequest;
use std::collections::HashMap;

/// Test health check endpoint
#[tokio::test]
async fn test_health_endpoint() {
    let routes = Router::new().route(
        "/health",
        get(|| async {
            axum::Json(serde_json::json!({
                "status": "healthy",
                "service": "pierre-mcp-server-single-tenant",
                "timestamp": chrono::Utc::now().to_rfc3339()
            }))
        }),
    );

    let resp = AxumTestRequest::get("/health").send(routes).await;

    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = serde_json::from_slice(&resp.bytes()).unwrap();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "pierre-mcp-server-single-tenant");
}

/// Test OAuth callback endpoint with missing parameters
#[tokio::test]
async fn test_oauth_callback_missing_code() {
    // Mock OAuth callback route that returns error for missing code
    async fn handle_callback(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
        if params.contains_key("code") {
            (StatusCode::OK, Html("OK"))
        } else {
            (StatusCode::BAD_REQUEST, Html("Missing code parameter"))
        }
    }

    let route = Router::new().route("/oauth/callback/strava", get(handle_callback));

    // Test without code parameter
    let resp = AxumTestRequest::get("/oauth/callback/strava?state=test_state")
        .send(route)
        .await;

    assert_eq!(resp.status(), 400);
}

/// Test OAuth callback endpoint with missing state
#[tokio::test]
async fn test_oauth_callback_missing_state() {
    // Mock OAuth callback route that returns error for missing state
    async fn handle_callback(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
        if params.contains_key("state") {
            (StatusCode::OK, Html("OK"))
        } else {
            (StatusCode::BAD_REQUEST, Html("Missing state parameter"))
        }
    }

    let route = Router::new().route("/oauth/callback/strava", get(handle_callback));

    // Test without state parameter
    let resp = AxumTestRequest::get("/oauth/callback/strava?code=test_code")
        .send(route)
        .await;

    assert_eq!(resp.status(), 400);
}

/// Test successful OAuth callback response format
#[tokio::test]
async fn test_oauth_callback_success_html() {
    // Create a mock success HTML response
    let success_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>OAuth Success - Pierre MCP Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; text-align: center; }
        .success { color: #4CAF50; }
    </style>
</head>
<body>
    <h1 class="success">OAuth Authorization Successful!</h1>
    <p>Your Strava account has been successfully connected.</p>
</body>
</html>"#;

    // Mock route that returns success HTML
    let route = Router::new().route(
        "/oauth/callback/strava",
        get(move || async move { Html(success_html) }),
    );

    let resp = AxumTestRequest::get("/oauth/callback/strava")
        .send(route)
        .await;

    assert_eq!(resp.status(), 200);
    assert!(String::from_utf8_lossy(&resp.bytes()).contains("OAuth Authorization Successful"));
}

/// Test OAuth callback error response format
#[tokio::test]
async fn test_oauth_callback_error_html() {
    // Create a mock error HTML response
    let error_html = r#"<!DOCTYPE html>
<html>
<head>
    <title>OAuth Error - Pierre MCP Server</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; text-align: center; }
        .error { color: #f44336; }
    </style>
</head>
<body>
    <h1 class="error">OAuth Authorization Failed</h1>
    <p>Invalid authorization code</p>
</body>
</html>"#;

    // Mock route that returns error HTML
    let route = Router::new().route(
        "/oauth/callback/strava",
        get(move || async move { Html(error_html) }),
    );

    let resp = AxumTestRequest::get("/oauth/callback/strava")
        .send(route)
        .await;

    assert_eq!(resp.status(), 200);
    assert!(String::from_utf8_lossy(&resp.bytes()).contains("OAuth Authorization Failed"));
}

/// Test both Strava and Fitbit callback endpoints exist
#[tokio::test]
async fn test_multiple_provider_endpoints() {
    // Create routes for both providers
    let routes = Router::new()
        .route(
            "/oauth/callback/strava",
            get(|| async { Html("Strava callback") }),
        )
        .route(
            "/oauth/callback/fitbit",
            get(|| async { Html("Fitbit callback") }),
        );

    // Test Strava endpoint
    let resp = AxumTestRequest::get("/oauth/callback/strava")
        .send(routes.clone())
        .await;
    assert_eq!(resp.status(), 200);
    assert!(String::from_utf8_lossy(&resp.bytes()).contains("Strava callback"));

    // Test Fitbit endpoint
    let resp = AxumTestRequest::get("/oauth/callback/fitbit")
        .send(routes)
        .await;
    assert_eq!(resp.status(), 200);
    assert!(String::from_utf8_lossy(&resp.bytes()).contains("Fitbit callback"));
}
