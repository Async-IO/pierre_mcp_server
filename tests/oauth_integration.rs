// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for OAuth flow in multi-tenant mode

use pierre_mcp_server::{
    routes::{AuthRoutes, OAuthRoutes, RegisterRequest},
    database::{Database, generate_encryption_key},
    auth::AuthManager,
};
use uuid::Uuid;

#[tokio::test]
async fn test_oauth_authorization_url_generation() {
    // Setup
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    
    let auth_routes = AuthRoutes::new(database.clone(), auth_manager.clone());
    let oauth_routes = OAuthRoutes::new(database.clone());
    
    // Register and login user
    let register_request = RegisterRequest {
        email: "oauth_test@example.com".to_string(),
        password: "password123".to_string(),
        display_name: Some("OAuth Test User".to_string()),
    };
    
    let register_response = auth_routes.register(register_request).await.unwrap();
    let user_id = Uuid::parse_str(&register_response.user_id).unwrap();
    
    // Test Strava OAuth URL generation
    let strava_auth = oauth_routes.get_auth_url(user_id, "strava").await.unwrap();
    
    assert!(strava_auth.authorization_url.contains("https://www.strava.com/oauth/authorize"));
    assert!(strava_auth.authorization_url.contains("client_id="));
    assert!(strava_auth.authorization_url.contains("redirect_uri="));
    assert!(strava_auth.authorization_url.contains("scope=read%2Cactivity%3Aread_all"));
    assert!(strava_auth.state.contains(&user_id.to_string()));
    assert_eq!(strava_auth.expires_in_minutes, 10);
    
    // Test Fitbit OAuth URL generation
    let fitbit_auth = oauth_routes.get_auth_url(user_id, "fitbit").await.unwrap();
    
    assert!(fitbit_auth.authorization_url.contains("https://www.fitbit.com/oauth2/authorize"));
    assert!(fitbit_auth.authorization_url.contains("client_id="));
    assert!(fitbit_auth.authorization_url.contains("redirect_uri="));
    assert!(fitbit_auth.authorization_url.contains("scope=activity%20profile"));
    assert!(fitbit_auth.state.contains(&user_id.to_string()));
}

#[tokio::test]
async fn test_oauth_state_validation() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let _oauth_routes = OAuthRoutes::new(database.clone());
    
    // Test valid state format
    let user_id = Uuid::new_v4();
    let state_id = Uuid::new_v4();
    let valid_state = format!("{}:{}", user_id, state_id);
    
    // This should parse correctly (we can't test the full callback without mocking the HTTP client)
    // But we can verify the state format is what we expect
    assert!(valid_state.contains(':'));
    let parts: Vec<&str> = valid_state.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert!(Uuid::parse_str(parts[0]).is_ok());
    assert!(Uuid::parse_str(parts[1]).is_ok());
}

#[tokio::test]
async fn test_connection_status_no_providers() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let oauth_routes = OAuthRoutes::new(database.clone());
    
    let user_id = Uuid::new_v4();
    let statuses = oauth_routes.get_connection_status(user_id).await.unwrap();
    
    assert_eq!(statuses.len(), 2);
    
    let strava_status = statuses.iter().find(|s| s.provider == "strava").unwrap();
    assert!(!strava_status.connected);
    assert!(strava_status.expires_at.is_none());
    assert!(strava_status.scopes.is_none());
    
    let fitbit_status = statuses.iter().find(|s| s.provider == "fitbit").unwrap();
    assert!(!fitbit_status.connected);
    assert!(fitbit_status.expires_at.is_none());
    assert!(fitbit_status.scopes.is_none());
}

#[tokio::test]
async fn test_invalid_provider_error() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let oauth_routes = OAuthRoutes::new(database.clone());
    
    let user_id = Uuid::new_v4();
    let result = oauth_routes.get_auth_url(user_id, "invalid_provider").await;
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported provider"));
}

#[tokio::test]
async fn test_disconnect_provider() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let oauth_routes = OAuthRoutes::new(database.clone());
    
    let user_id = Uuid::new_v4();
    
    // Test disconnecting Strava (should succeed even if not connected)
    let result = oauth_routes.disconnect_provider(user_id, "strava").await;
    assert!(result.is_ok());
    
    // Test disconnecting invalid provider
    let result = oauth_routes.disconnect_provider(user_id, "invalid").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Unsupported provider"));
}

#[tokio::test]
async fn test_oauth_urls_contain_required_parameters() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await.unwrap();
    let oauth_routes = OAuthRoutes::new(database.clone());
    
    let user_id = Uuid::new_v4();
    
    // Test Strava URL parameters
    let strava_auth = oauth_routes.get_auth_url(user_id, "strava").await.unwrap();
    let strava_url = url::Url::parse(&strava_auth.authorization_url).unwrap();
    let strava_params: std::collections::HashMap<_, _> = strava_url.query_pairs().collect();
    
    assert!(strava_params.contains_key("client_id"));
    assert!(strava_params.contains_key("redirect_uri"));
    assert!(strava_params.contains_key("response_type"));
    assert_eq!(strava_params.get("response_type").unwrap(), "code");
    assert!(strava_params.contains_key("scope"));
    assert!(strava_params.contains_key("state"));
    
    // Test Fitbit URL parameters
    let fitbit_auth = oauth_routes.get_auth_url(user_id, "fitbit").await.unwrap();
    let fitbit_url = url::Url::parse(&fitbit_auth.authorization_url).unwrap();
    let fitbit_params: std::collections::HashMap<_, _> = fitbit_url.query_pairs().collect();
    
    assert!(fitbit_params.contains_key("client_id"));
    assert!(fitbit_params.contains_key("redirect_uri"));
    assert!(fitbit_params.contains_key("response_type"));
    assert_eq!(fitbit_params.get("response_type").unwrap(), "code");
    assert!(fitbit_params.contains_key("scope"));
    assert!(fitbit_params.contains_key("state"));
}