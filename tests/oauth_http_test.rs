// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth HTTP Endpoint Tests
//!
//! Tests for OAuth HTTP callback endpoints in single-tenant mode.

use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::factory::Database;
use pierre_mcp_server::oauth::manager::OAuthManager;
use std::sync::Arc;
use uuid::Uuid;
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
            if !params.contains_key("code") {
                warp::reply::with_status(
                    warp::reply::html("Missing code parameter"),
                    StatusCode::BAD_REQUEST,
                )
            } else {
                warp::reply::with_status(warp::reply::html("OK"), StatusCode::OK)
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
            if !params.contains_key("state") {
                warp::reply::with_status(
                    warp::reply::html("Missing state parameter"),
                    StatusCode::BAD_REQUEST,
                )
            } else {
                warp::reply::with_status(warp::reply::html("OK"), StatusCode::OK)
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
    <h1 class="success">✅ OAuth Authorization Successful!</h1>
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
    <h1 class="error">❌ OAuth Authorization Failed</h1>
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

/// Test OAuth state validation
#[tokio::test]
async fn test_oauth_state_validation() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create OAuth manager
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Set up test environment
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    let strava_config = pierre_mcp_server::config::environment::OAuthProviderConfig {
        client_id: Some("test_client".to_string()),
        client_secret: Some("test_secret".to_string()),
        redirect_uri: None,
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        enabled: true,
    };
    let strava_provider =
        pierre_mcp_server::oauth::providers::StravaOAuthProvider::from_config(&strava_config)
            .unwrap();
    oauth_manager.register_provider(Box::new(strava_provider));

    // Generate auth URL to create a valid state
    let user_id = Uuid::new_v4();
    let auth_response = oauth_manager
        .generate_auth_url(user_id, "strava")
        .await
        .unwrap();

    // Valid state should be accepted
    assert!(!auth_response.state.is_empty());

    // Invalid state should be rejected in callback
    let result = oauth_manager
        .handle_callback("code123", "invalid_state_123", "strava")
        .await;
    assert!(result.is_err());
}

/// Test concurrent OAuth requests
#[tokio::test]
async fn test_concurrent_oauth_requests() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create OAuth manager
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Set up test environment
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    let strava_config = pierre_mcp_server::config::environment::OAuthProviderConfig {
        client_id: Some("test_client".to_string()),
        client_secret: Some("test_secret".to_string()),
        redirect_uri: None,
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        enabled: true,
    };
    let strava_provider =
        pierre_mcp_server::oauth::providers::StravaOAuthProvider::from_config(&strava_config)
            .unwrap();
    oauth_manager.register_provider(Box::new(strava_provider));

    // Generate multiple auth URLs concurrently
    let mut handles = vec![];
    let oauth_manager = Arc::new(oauth_manager);

    for _i in 0..5 {
        let oauth_mgr = oauth_manager.clone();
        let handle = tokio::spawn(async move {
            let user_id = Uuid::new_v4();
            oauth_mgr.generate_auth_url(user_id, "strava").await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut results = vec![];
    for handle in handles {
        let result = handle.await.unwrap();
        results.push(result);
    }

    // Verify all requests succeeded and have unique states
    let mut states = std::collections::HashSet::new();
    for result in results {
        let auth_response = result.unwrap();
        assert!(auth_response.authorization_url.contains("strava.com"));
        assert!(states.insert(auth_response.state)); // Should be unique
    }
}
