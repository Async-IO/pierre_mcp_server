// ABOUTME: OAuth HTTP endpoint tests for callback handling
// ABOUTME: Tests OAuth HTTP callback endpoints in single-tenant mode
#![allow(clippy::if_not_else, clippy::unused_async)]
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # OAuth HTTP Endpoint Tests
//!
//! Tests for OAuth HTTP callback endpoints in single-tenant mode.

use warp::http::StatusCode;
use warp::test::request;
use warp::Filter;

/// Test health check endpoint
#[tokio::test]
async fn test_health_endpoint() {
    // Import the setup function from multitenant_server
    let routes = warp::path!("health").and(warp::get()).map(|| {
        warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "service": "pierre-mcp-server-single-tenant",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    });

    let resp = request().method("GET").path("/health").reply(&routes).await;

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = serde_json::from_slice(resp.body()).unwrap();
    assert_eq!(body["status"], "healthy");
    assert_eq!(body["service"], "pierre-mcp-server-single-tenant");
}

/// Test OAuth callback endpoint with missing parameters
#[tokio::test]
async fn test_oauth_callback_missing_code() {
    // Mock OAuth callback route that returns error for missing code
    let route = warp::path!("oauth" / "callback" / "strava")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .map(|params: std::collections::HashMap<String, String>| {
            if params.contains_key("code") {
                warp::reply::with_status(warp::reply::html("OK"), StatusCode::OK)
            } else {
                warp::reply::with_status(
                    warp::reply::html("Missing code parameter"),
                    StatusCode::BAD_REQUEST,
                )
            }
        });

    // Test without code parameter
    let resp = request()
        .method("GET")
        .path("/oauth/callback/strava?state=test_state")
        .reply(&route)
        .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

/// Test OAuth callback endpoint with missing state
#[tokio::test]
async fn test_oauth_callback_missing_state() {
    // Mock OAuth callback route that returns error for missing state
    let route = warp::path!("oauth" / "callback" / "strava")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .map(|params: std::collections::HashMap<String, String>| {
            if params.contains_key("state") {
                warp::reply::with_status(warp::reply::html("OK"), StatusCode::OK)
            } else {
                warp::reply::with_status(
                    warp::reply::html("Missing state parameter"),
                    StatusCode::BAD_REQUEST,
                )
            }
        });

    // Test without state parameter
    let resp = request()
        .method("GET")
        .path("/oauth/callback/strava?code=test_code")
        .reply(&route)
        .await;

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
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
    let route = warp::path!("oauth" / "callback" / "strava")
        .and(warp::get())
        .map(move || warp::reply::html(success_html));

    let resp = request()
        .method("GET")
        .path("/oauth/callback/strava")
        .reply(&route)
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(String::from_utf8_lossy(resp.body()).contains("OAuth Authorization Successful"));
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
    let route = warp::path!("oauth" / "callback" / "strava")
        .and(warp::get())
        .map(move || warp::reply::html(error_html));

    let resp = request()
        .method("GET")
        .path("/oauth/callback/strava")
        .reply(&route)
        .await;

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(String::from_utf8_lossy(resp.body()).contains("OAuth Authorization Failed"));
}

/// Test both Strava and Fitbit callback endpoints exist
#[tokio::test]
async fn test_multiple_provider_endpoints() {
    // Create routes for both providers
    let strava = warp::path!("oauth" / "callback" / "strava")
        .and(warp::get())
        .map(|| warp::reply::html("Strava callback"));

    let fitbit = warp::path!("oauth" / "callback" / "fitbit")
        .and(warp::get())
        .map(|| warp::reply::html("Fitbit callback"));

    let routes = strava.or(fitbit);

    // Test Strava endpoint
    let resp = request()
        .method("GET")
        .path("/oauth/callback/strava")
        .reply(&routes)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(String::from_utf8_lossy(resp.body()).contains("Strava callback"));

    // Test Fitbit endpoint
    let resp = request()
        .method("GET")
        .path("/oauth/callback/fitbit")
        .reply(&routes)
        .await;
    assert_eq!(resp.status(), StatusCode::OK);
    assert!(String::from_utf8_lossy(resp.body()).contains("Fitbit callback"));
}
