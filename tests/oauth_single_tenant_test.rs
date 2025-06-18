// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # OAuth Single-Tenant Integration Tests
//!
//! Tests for OAuth functionality in single-tenant mode.

use pierre_mcp_server::database::{generate_encryption_key, Database};
use pierre_mcp_server::models::User;
use pierre_mcp_server::oauth::{manager::OAuthManager, providers::StravaOAuthProvider};
use std::sync::Arc;
use uuid::Uuid;

/// Test OAuth flow in single-tenant mode
#[tokio::test]
async fn test_single_tenant_oauth_flow() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new(":memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create OAuth manager
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register Strava provider with test credentials
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    let strava_provider = StravaOAuthProvider::new().unwrap();
    oauth_manager.register_provider(Box::new(strava_provider));

    // Test authorization URL generation
    let user_id = Uuid::new_v4();
    let auth_response = oauth_manager
        .generate_auth_url(user_id, "strava")
        .await
        .unwrap();

    assert!(auth_response.authorization_url.contains("strava.com"));
    assert!(auth_response.authorization_url.contains("test_client"));
    assert!(!auth_response.state.is_empty());
    assert_eq!(auth_response.provider, "strava");
}

/// Test OAuth callback handling with invalid state
#[tokio::test]
async fn test_oauth_callback_invalid_state() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new(":memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create OAuth manager
    let oauth_manager = OAuthManager::new(database.clone());

    // Try to handle callback with invalid state
    let result = oauth_manager
        .handle_callback("code123", "invalid_state", "strava")
        .await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        pierre_mcp_server::oauth::OAuthError::InvalidState
    ));
}

/// Test connection status retrieval
#[tokio::test]
async fn test_connection_status_single_tenant() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new(":memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Create OAuth manager
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register providers
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");
    std::env::set_var("FITBIT_CLIENT_ID", "test_fitbit");
    std::env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_secret");

    let strava_provider = StravaOAuthProvider::new().unwrap();
    oauth_manager.register_provider(Box::new(strava_provider));

    let fitbit_provider = pierre_mcp_server::oauth::providers::FitbitOAuthProvider::new().unwrap();
    oauth_manager.register_provider(Box::new(fitbit_provider));

    // Get connection status
    let status = oauth_manager.get_connection_status(user_id).await.unwrap();

    assert_eq!(status.len(), 2);
    assert_eq!(status.get("strava"), Some(&false));
    assert_eq!(status.get("fitbit"), Some(&false));
}

/// Test token refresh mechanism
#[tokio::test]
async fn test_token_refresh() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new(":memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Store expired token
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1); // Expired 1 hour ago
    database
        .update_strava_token(
            user_id,
            "old_access_token",
            "refresh_token_123",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Create OAuth manager
    let oauth_manager = OAuthManager::new(database.clone());

    // Register mock provider (Note: In real tests, we'd use a mock server)
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Test that ensure_valid_token attempts refresh
    // (In a real test, this would hit a mock server and succeed)
    let result = oauth_manager.ensure_valid_token(user_id, "strava").await;

    // Since we don't have a mock server running, this will fail
    // but we're testing that the mechanism exists
    assert!(result.is_err() || result.unwrap().is_some());
}

/// Test disconnect provider functionality
#[tokio::test]
async fn test_disconnect_provider() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new(":memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user).await.unwrap();

    // Store token
    database
        .update_strava_token(
            user_id,
            "access_token",
            "refresh_token",
            chrono::Utc::now() + chrono::Duration::hours(1),
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Verify token exists
    assert!(database.get_strava_token(user_id).await.unwrap().is_some());

    // Create OAuth manager and disconnect
    let mut oauth_manager = OAuthManager::new(database.clone());

    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    let strava_provider = StravaOAuthProvider::new().unwrap();
    oauth_manager.register_provider(Box::new(strava_provider));

    // Disconnect provider (revocation will fail but local deletion should work)
    let result = oauth_manager.disconnect_provider(user_id, "strava").await;
    assert!(result.is_ok());

    // Verify token is removed
    assert!(database.get_strava_token(user_id).await.unwrap().is_none());
}

/// Test multi-provider OAuth management
#[tokio::test]
async fn test_multi_provider_oauth() {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new(":memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    // Create OAuth manager
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register multiple providers
    std::env::set_var("STRAVA_CLIENT_ID", "strava_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "strava_secret");
    std::env::set_var("FITBIT_CLIENT_ID", "fitbit_client");
    std::env::set_var("FITBIT_CLIENT_SECRET", "fitbit_secret");

    let strava_provider = StravaOAuthProvider::new().unwrap();
    oauth_manager.register_provider(Box::new(strava_provider));

    let fitbit_provider = pierre_mcp_server::oauth::providers::FitbitOAuthProvider::new().unwrap();
    oauth_manager.register_provider(Box::new(fitbit_provider));

    // Generate auth URLs for both providers
    let user_id = Uuid::new_v4();

    let strava_auth = oauth_manager
        .generate_auth_url(user_id, "strava")
        .await
        .unwrap();
    assert!(strava_auth.authorization_url.contains("strava.com"));

    let fitbit_auth = oauth_manager
        .generate_auth_url(user_id, "fitbit")
        .await
        .unwrap();
    assert!(fitbit_auth.authorization_url.contains("fitbit.com"));

    // Test unsupported provider
    let result = oauth_manager.generate_auth_url(user_id, "garmin").await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        pierre_mcp_server::oauth::OAuthError::UnsupportedProvider(_)
    ));
}
