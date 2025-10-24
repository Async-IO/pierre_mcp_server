// ABOUTME: Comprehensive tests for OAuth provider implementations
// ABOUTME: Tests all OAuth provider functionality for Strava and Fitbit
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Comprehensive tests for oauth/providers.rs - OAuth provider implementations
//!
//! This test suite aims to improve coverage from 43.19% to 80%+ by testing
//! all OAuth provider functionality for Strava and Fitbit.

mod common;

use anyhow::Result;
use chrono::{Duration, Utc};
use pierre_mcp_server::{
    config::environment::{FitbitApiConfig, OAuthProviderConfig, StravaApiConfig},
    oauth::{
        providers::{FitbitOAuthProvider, StravaOAuthProvider},
        OAuthError, OAuthProvider, TokenData,
    },
};
use std::sync::Once;
use uuid::Uuid;

/// Ensure `ServerConfig` is initialized only once across all tests
static INIT_SERVER_CONFIG: Once = Once::new();

fn ensure_server_config_initialized() {
    INIT_SERVER_CONFIG.call_once(|| {
        pierre_mcp_server::constants::init_server_config();
    });
}

// === Test Setup Helpers ===

fn create_strava_api_config() -> StravaApiConfig {
    StravaApiConfig {
        base_url: "https://www.strava.com/api/v3".to_string(),
        auth_url: "https://www.strava.com/oauth/authorize".to_string(),
        token_url: "https://www.strava.com/oauth/token".to_string(),
        deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_string(),
    }
}

fn create_fitbit_api_config() -> FitbitApiConfig {
    FitbitApiConfig {
        base_url: "https://api.fitbit.com".to_string(),
        auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
        token_url: "https://api.fitbit.com/oauth2/token".to_string(),
        revoke_url: "https://api.fitbit.com/oauth2/revoke".to_string(),
    }
}

fn create_valid_strava_config() -> OAuthProviderConfig {
    OAuthProviderConfig {
        client_id: Some("test_strava_client_id".to_string()),
        client_secret: Some("test_strava_client_secret".to_string()),
        redirect_uri: Some("http://localhost:4000/oauth/callback/strava".to_string()),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        enabled: true,
    }
}

fn create_valid_fitbit_config() -> OAuthProviderConfig {
    OAuthProviderConfig {
        client_id: Some("test_fitbit_client_id".to_string()),
        client_secret: Some("test_fitbit_client_secret".to_string()),
        redirect_uri: Some("http://localhost:4000/oauth/callback/fitbit".to_string()),
        scopes: vec!["activity".to_string(), "profile".to_string()],
        enabled: true,
    }
}

fn create_incomplete_config() -> OAuthProviderConfig {
    OAuthProviderConfig {
        client_id: None, // Missing client_id
        client_secret: Some("test_secret".to_string()),
        redirect_uri: Some("http://localhost:4000/oauth/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    }
}

fn create_test_token_data(provider: &str) -> TokenData {
    TokenData {
        access_token: "test_access_token".to_string(),
        refresh_token: "test_refresh_token".to_string(),
        expires_at: Utc::now() + Duration::hours(1),
        scopes: "read,activity:read_all".to_string(),
        provider: provider.to_string(),
    }
}

fn create_expired_token_data(provider: &str) -> TokenData {
    TokenData {
        access_token: "expired_access_token".to_string(),
        refresh_token: "expired_refresh_token".to_string(),
        expires_at: Utc::now() - Duration::hours(1), // Expired
        scopes: "read,activity:read_all".to_string(),
        provider: provider.to_string(),
    }
}

// === Strava OAuth Provider Tests ===

#[tokio::test]
async fn test_strava_provider_from_config_success() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    assert_eq!(provider.name(), "strava");

    Ok(())
}

#[tokio::test]
async fn test_strava_provider_from_config_missing_client_id() -> Result<()> {
    let config = create_incomplete_config();
    let api_config = create_strava_api_config();

    let result = StravaOAuthProvider::from_config(&config, &api_config);

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("client_id not configured"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
async fn test_strava_provider_from_config_missing_client_secret() -> Result<()> {
    let mut config = create_valid_strava_config();
    config.client_secret = None;
    let api_config = create_strava_api_config();

    let result = StravaOAuthProvider::from_config(&config, &api_config);

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("client_secret not configured"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
async fn test_strava_provider_from_config_default_redirect_uri() -> Result<()> {
    ensure_server_config_initialized();
    let mut config = create_valid_strava_config();
    config.redirect_uri = None;
    let api_config = create_strava_api_config();

    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    // Should succeed and use default redirect URI
    assert_eq!(provider.name(), "strava");

    Ok(())
}

#[tokio::test]
async fn test_strava_generate_auth_url() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;
    let user_id = Uuid::new_v4();
    let state = "test_state";

    let response = provider
        .generate_auth_url(user_id, state.to_string())
        .await?;

    assert!(response.authorization_url.contains("strava.com"));
    assert!(response.authorization_url.contains("authorize"));
    assert!(response.authorization_url.contains("test_strava_client_id"));
    assert!(response.authorization_url.contains("test_state"));
    assert_eq!(response.state, state);
    assert_eq!(response.provider, "strava");
    assert!(!response.instructions.is_empty());
    assert!(response.expires_in_minutes > 0);

    Ok(())
}

#[tokio::test]
async fn test_strava_exchange_code_invalid_code() -> Result<()> {
    common::init_test_http_clients();
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    // Test with invalid code (will fail because no real OAuth server)
    let result = provider.exchange_code("invalid_code", "test_state").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strava_refresh_token_invalid() -> Result<()> {
    common::init_test_http_clients();
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    // Test with invalid refresh token (will fail because no real OAuth server)
    let result = provider.refresh_token("invalid_refresh_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strava_revoke_token_invalid() -> Result<()> {
    common::init_test_http_clients();
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    // Test with invalid access token (will fail because no real OAuth server)
    let result = provider.revoke_token("invalid_access_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strava_validate_token_expired() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;
    let expired_token = create_expired_token_data("strava");

    let is_valid = provider.validate_token(&expired_token).await?;

    assert!(!is_valid);

    Ok(())
}

#[tokio::test]
async fn test_strava_validate_token_valid() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;
    let valid_token = create_test_token_data("strava");

    // Token validation might fail due to network, we just test it doesn't panic
    let _is_valid = provider.validate_token(&valid_token).await;

    Ok(())
}

// === Fitbit OAuth Provider Tests ===

#[tokio::test]
async fn test_fitbit_provider_from_config_success() -> Result<()> {
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;

    assert_eq!(provider.name(), "fitbit");

    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_from_config_missing_client_id() -> Result<()> {
    let config = create_incomplete_config();
    let api_config = create_fitbit_api_config();

    let result = FitbitOAuthProvider::from_config(&config, &api_config);

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("client_id not configured"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_from_config_missing_client_secret() -> Result<()> {
    let mut config = create_valid_fitbit_config();
    config.client_secret = None;
    let api_config = create_fitbit_api_config();

    let result = FitbitOAuthProvider::from_config(&config, &api_config);

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("client_secret not configured"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_from_config_default_redirect_uri() -> Result<()> {
    ensure_server_config_initialized();
    let mut config = create_valid_fitbit_config();
    config.redirect_uri = None;
    let api_config = create_fitbit_api_config();

    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;

    // Should succeed and use default redirect URI
    assert_eq!(provider.name(), "fitbit");

    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_missing_config() -> Result<()> {
    // Test missing client_id in config
    let config = OAuthProviderConfig {
        client_id: None,
        client_secret: Some("test_secret".to_string()),
        redirect_uri: Some("http://test.example.com/callback".to_string()),
        scopes: vec!["activity".to_string()],
        enabled: true,
    };
    let api_config = create_fitbit_api_config();

    let result = FitbitOAuthProvider::from_config(&config, &api_config);

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("client_id not configured"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
async fn test_fitbit_generate_auth_url() -> Result<()> {
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;
    let user_id = Uuid::new_v4();
    let state = "test_fitbit_state";

    let response = provider
        .generate_auth_url(user_id, state.to_string())
        .await?;

    assert!(response.authorization_url.contains("fitbit.com"));
    assert!(response.authorization_url.contains("authorize"));
    assert!(response.authorization_url.contains("test_fitbit_client_id"));
    assert!(response.authorization_url.contains("test_fitbit_state"));
    assert_eq!(response.state, state);
    assert_eq!(response.provider, "fitbit");
    assert!(!response.instructions.is_empty());
    assert!(response.expires_in_minutes > 0);

    Ok(())
}

#[tokio::test]
async fn test_fitbit_exchange_code_invalid_code() -> Result<()> {
    common::init_test_http_clients();
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;

    // Test with invalid code (will fail because no real OAuth server)
    let result = provider
        .exchange_code("invalid_fitbit_code", "test_state")
        .await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_refresh_token_invalid() -> Result<()> {
    common::init_test_http_clients();
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;

    // Test with invalid refresh token (will fail because no real OAuth server)
    let result = provider.refresh_token("invalid_fitbit_refresh_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_revoke_token_invalid() -> Result<()> {
    common::init_test_http_clients();
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;

    // Test with invalid access token (will fail because no real OAuth server)
    let result = provider.revoke_token("invalid_fitbit_access_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_validate_token_expired() -> Result<()> {
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;
    let expired_token = create_expired_token_data("fitbit");

    let is_valid = provider.validate_token(&expired_token).await?;

    assert!(!is_valid);

    Ok(())
}

#[tokio::test]
async fn test_fitbit_validate_token_valid() -> Result<()> {
    let config = create_valid_fitbit_config();
    let api_config = create_fitbit_api_config();
    let provider = FitbitOAuthProvider::from_config(&config, &api_config)?;
    let valid_token = create_test_token_data("fitbit");

    // Token validation might fail due to network, we just test it doesn't panic
    let _is_valid = provider.validate_token(&valid_token).await;

    Ok(())
}

// === Provider Comparison Tests ===

#[tokio::test]
async fn test_provider_names() -> Result<()> {
    let strava_config = create_valid_strava_config();
    let fitbit_config = create_valid_fitbit_config();
    let strava_api_config = create_strava_api_config();
    let fitbit_api_config = create_fitbit_api_config();

    let strava_provider = StravaOAuthProvider::from_config(&strava_config, &strava_api_config)?;
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config, &fitbit_api_config)?;

    assert_eq!(strava_provider.name(), "strava");
    assert_eq!(fitbit_provider.name(), "fitbit");
    assert_ne!(strava_provider.name(), fitbit_provider.name());

    Ok(())
}

#[tokio::test]
async fn test_auth_urls_different() -> Result<()> {
    let strava_config = create_valid_strava_config();
    let fitbit_config = create_valid_fitbit_config();
    let strava_api_config = create_strava_api_config();
    let fitbit_api_config = create_fitbit_api_config();

    let strava_provider = StravaOAuthProvider::from_config(&strava_config, &strava_api_config)?;
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config, &fitbit_api_config)?;

    let user_id = Uuid::new_v4();
    let state = "comparison_test_state";

    let strava_response = strava_provider
        .generate_auth_url(user_id, state.to_string())
        .await?;
    let fitbit_response = fitbit_provider
        .generate_auth_url(user_id, state.to_string())
        .await?;

    assert_ne!(
        strava_response.authorization_url,
        fitbit_response.authorization_url
    );
    assert!(strava_response.authorization_url.contains("strava.com"));
    assert!(fitbit_response.authorization_url.contains("fitbit.com"));

    Ok(())
}

// === Configuration Edge Cases ===

#[tokio::test]
async fn test_config_with_empty_scopes() -> Result<()> {
    let mut config = create_valid_strava_config();
    config.scopes = vec![]; // Empty scopes
    let api_config = create_strava_api_config();

    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;
    let user_id = Uuid::new_v4();
    let state = "empty_scopes_test";

    let response = provider
        .generate_auth_url(user_id, state.to_string())
        .await?;

    // Should still work with empty scopes
    assert!(!response.authorization_url.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_config_with_custom_scopes() -> Result<()> {
    let mut config = create_valid_strava_config();
    config.scopes = vec!["custom_scope1".to_string(), "custom_scope2".to_string()];
    let api_config = create_strava_api_config();

    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;
    let user_id = Uuid::new_v4();
    let state = "custom_scopes_test";

    let response = provider
        .generate_auth_url(user_id, state.to_string())
        .await?;

    // Should work with custom scopes
    assert!(!response.authorization_url.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_config_disabled_provider() -> Result<()> {
    let mut config = create_valid_strava_config();
    config.enabled = false;
    let api_config = create_strava_api_config();

    // Provider should still be creatable even if disabled
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    assert_eq!(provider.name(), "strava");

    Ok(())
}

// === Token Data Edge Cases ===

#[tokio::test]
async fn test_token_validation_with_wrong_provider() -> Result<()> {
    let strava_config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let strava_provider = StravaOAuthProvider::from_config(&strava_config, &api_config)?;

    // Create token data for fitbit but validate with strava provider
    let fitbit_token = create_test_token_data("fitbit");

    let _is_valid = strava_provider.validate_token(&fitbit_token).await;

    Ok(())
}

#[tokio::test]
async fn test_token_validation_edge_case_expires_exactly_now() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();
    let provider = StravaOAuthProvider::from_config(&config, &api_config)?;

    let mut token = create_test_token_data("strava");
    token.expires_at = Utc::now(); // Expires exactly now

    let is_valid = provider.validate_token(&token).await?;

    // Should be considered invalid (expired)
    assert!(!is_valid);

    Ok(())
}

// === Integration Tests ===

#[tokio::test]
async fn test_complete_oauth_flow_simulation() -> Result<()> {
    common::init_test_http_clients();
    let strava_config = create_valid_strava_config();
    let fitbit_config = create_valid_fitbit_config();
    let strava_api_config = create_strava_api_config();
    let fitbit_api_config = create_fitbit_api_config();

    let strava_provider = StravaOAuthProvider::from_config(&strava_config, &strava_api_config)?;
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config, &fitbit_api_config)?;

    let user_id = Uuid::new_v4();
    let state = "integration_test_state";

    // 1. Generate auth URLs for both providers
    let strava_auth = strava_provider
        .generate_auth_url(user_id, state.to_string())
        .await?;
    let fitbit_auth = fitbit_provider
        .generate_auth_url(user_id, state.to_string())
        .await?;

    // 2. Verify URLs are different and valid
    assert_ne!(strava_auth.authorization_url, fitbit_auth.authorization_url);
    assert!(strava_auth.authorization_url.contains("strava.com"));
    assert!(fitbit_auth.authorization_url.contains("fitbit.com"));

    // 3. Test token validation with various token states
    let valid_strava_token = create_test_token_data("strava");
    let expired_strava_token = create_expired_token_data("strava");

    let _valid_result = strava_provider.validate_token(&valid_strava_token).await;
    let expired_result = strava_provider
        .validate_token(&expired_strava_token)
        .await?;

    // Expired should definitely fail
    assert!(!expired_result);

    // 4. Test error handling for invalid operations
    let exchange_result = strava_provider.exchange_code("invalid_code", state).await;
    let refresh_result = strava_provider.refresh_token("invalid_refresh").await;
    let revoke_result = strava_provider.revoke_token("invalid_access").await;

    // All should fail gracefully
    assert!(exchange_result.is_err());
    assert!(refresh_result.is_err());
    assert!(revoke_result.is_err());

    Ok(())
}

// === Concurrency Tests ===

#[tokio::test]
async fn test_concurrent_auth_url_generation() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();

    let mut handles = vec![];

    for i in 0..5 {
        let config_clone = config.clone();
        let api_config_clone = api_config.clone();
        handles.push(tokio::spawn(async move {
            let provider = StravaOAuthProvider::from_config(&config_clone, &api_config_clone)?;
            let user_id = Uuid::new_v4();
            let state = format!("concurrent_test_{i}");
            provider.generate_auth_url(user_id, state).await
        }));
    }

    // All should succeed
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_token_validation() -> Result<()> {
    let config = create_valid_strava_config();
    let api_config = create_strava_api_config();

    let mut handles = vec![];

    for i in 0..3 {
        let config_clone = config.clone();
        let api_config_clone = api_config.clone();
        handles.push(tokio::spawn(async move {
            let provider = StravaOAuthProvider::from_config(&config_clone, &api_config_clone)?;
            let token = if i % 2 == 0 {
                create_test_token_data("strava")
            } else {
                create_expired_token_data("strava")
            };
            provider.validate_token(&token).await
        }));
    }

    // All should complete without panicking
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    Ok(())
}
