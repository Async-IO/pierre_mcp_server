//! Comprehensive tests for oauth/providers.rs - OAuth provider implementations
//!
//! This test suite aims to improve coverage from 43.19% to 80%+ by testing
//! all OAuth provider functionality for Strava and Fitbit.

use anyhow::Result;
use chrono::{Duration, Utc};
use pierre_mcp_server::{
    config::environment::OAuthProviderConfig,
    oauth::{
        providers::{FitbitOAuthProvider, StravaOAuthProvider},
        OAuthError, OAuthProvider, TokenData,
    },
};
use uuid::Uuid;

// === Test Setup Helpers ===

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
    let provider = StravaOAuthProvider::from_config(&config)?;

    assert_eq!(provider.name(), "strava");

    Ok(())
}

#[tokio::test]
async fn test_strava_provider_from_config_missing_client_id() -> Result<()> {
    let config = create_incomplete_config();

    let result = StravaOAuthProvider::from_config(&config);

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

    let result = StravaOAuthProvider::from_config(&config);

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
    let mut config = create_valid_strava_config();
    config.redirect_uri = None;

    let provider = StravaOAuthProvider::from_config(&config)?;

    // Should succeed and use default redirect URI
    assert_eq!(provider.name(), "strava");

    Ok(())
}

#[tokio::test]
#[allow(deprecated)]
async fn test_strava_provider_legacy_constructor_missing_env() -> Result<()> {
    // Clear environment variables to test missing config
    std::env::remove_var("STRAVA_CLIENT_ID");
    std::env::remove_var("STRAVA_CLIENT_SECRET");

    let result = StravaOAuthProvider::new();

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("STRAVA_CLIENT_ID not set"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
#[allow(deprecated)]
async fn test_strava_provider_legacy_constructor_success() -> Result<()> {
    // Set environment variables
    std::env::set_var("STRAVA_CLIENT_ID", "test_env_client_id");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_env_client_secret");
    std::env::set_var("STRAVA_REDIRECT_URI", "http://test.example.com/callback");

    let provider = StravaOAuthProvider::new()?;

    assert_eq!(provider.name(), "strava");

    // Clean up
    std::env::remove_var("STRAVA_CLIENT_ID");
    std::env::remove_var("STRAVA_CLIENT_SECRET");
    std::env::remove_var("STRAVA_REDIRECT_URI");

    Ok(())
}

#[tokio::test]
async fn test_strava_generate_auth_url() -> Result<()> {
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;
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
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;

    // Test with invalid code (will fail because no real OAuth server)
    let result = provider.exchange_code("invalid_code", "test_state").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strava_refresh_token_invalid() -> Result<()> {
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;

    // Test with invalid refresh token (will fail because no real OAuth server)
    let result = provider.refresh_token("invalid_refresh_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strava_revoke_token_invalid() -> Result<()> {
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;

    // Test with invalid access token (will fail because no real OAuth server)
    let result = provider.revoke_token("invalid_access_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_strava_validate_token_expired() -> Result<()> {
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;
    let expired_token = create_expired_token_data("strava");

    let is_valid = provider.validate_token(&expired_token).await?;

    assert!(!is_valid);

    Ok(())
}

#[tokio::test]
async fn test_strava_validate_token_valid() -> Result<()> {
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;
    let valid_token = create_test_token_data("strava");

    // Token validation might fail due to network, we just test it doesn't panic
    let _is_valid = provider.validate_token(&valid_token).await;

    Ok(())
}

// === Fitbit OAuth Provider Tests ===

#[tokio::test]
async fn test_fitbit_provider_from_config_success() -> Result<()> {
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;

    assert_eq!(provider.name(), "fitbit");

    Ok(())
}

#[tokio::test]
async fn test_fitbit_provider_from_config_missing_client_id() -> Result<()> {
    let config = create_incomplete_config();

    let result = FitbitOAuthProvider::from_config(&config);

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

    let result = FitbitOAuthProvider::from_config(&config);

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
    let mut config = create_valid_fitbit_config();
    config.redirect_uri = None;

    let provider = FitbitOAuthProvider::from_config(&config)?;

    // Should succeed and use default redirect URI
    assert_eq!(provider.name(), "fitbit");

    Ok(())
}

#[tokio::test]
#[allow(deprecated)]
async fn test_fitbit_provider_legacy_constructor_missing_env() -> Result<()> {
    // Clear environment variables to test missing config
    std::env::remove_var("FITBIT_CLIENT_ID");
    std::env::remove_var("FITBIT_CLIENT_SECRET");

    let result = FitbitOAuthProvider::new();

    assert!(result.is_err());
    if let Err(OAuthError::ConfigurationError(msg)) = result {
        assert!(msg.contains("FITBIT_CLIENT_ID not set"));
    } else {
        panic!("Expected ConfigurationError");
    }

    Ok(())
}

#[tokio::test]
#[allow(deprecated)]
async fn test_fitbit_provider_legacy_constructor_success() -> Result<()> {
    // Set environment variables
    std::env::set_var("FITBIT_CLIENT_ID", "test_fitbit_env_client_id");
    std::env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_env_client_secret");
    std::env::set_var(
        "FITBIT_REDIRECT_URI",
        "http://test.example.com/fitbit/callback",
    );

    let provider = FitbitOAuthProvider::new()?;

    assert_eq!(provider.name(), "fitbit");

    // Clean up
    std::env::remove_var("FITBIT_CLIENT_ID");
    std::env::remove_var("FITBIT_CLIENT_SECRET");
    std::env::remove_var("FITBIT_REDIRECT_URI");

    Ok(())
}

#[tokio::test]
async fn test_fitbit_generate_auth_url() -> Result<()> {
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;
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
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;

    // Test with invalid code (will fail because no real OAuth server)
    let result = provider
        .exchange_code("invalid_fitbit_code", "test_state")
        .await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_refresh_token_invalid() -> Result<()> {
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;

    // Test with invalid refresh token (will fail because no real OAuth server)
    let result = provider.refresh_token("invalid_fitbit_refresh_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_revoke_token_invalid() -> Result<()> {
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;

    // Test with invalid access token (will fail because no real OAuth server)
    let result = provider.revoke_token("invalid_fitbit_access_token").await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_validate_token_expired() -> Result<()> {
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;
    let expired_token = create_expired_token_data("fitbit");

    let is_valid = provider.validate_token(&expired_token).await?;

    assert!(!is_valid);

    Ok(())
}

#[tokio::test]
async fn test_fitbit_validate_token_valid() -> Result<()> {
    let config = create_valid_fitbit_config();
    let provider = FitbitOAuthProvider::from_config(&config)?;
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

    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config)?;

    assert_eq!(strava_provider.name(), "strava");
    assert_eq!(fitbit_provider.name(), "fitbit");
    assert_ne!(strava_provider.name(), fitbit_provider.name());

    Ok(())
}

#[tokio::test]
async fn test_auth_urls_different() -> Result<()> {
    let strava_config = create_valid_strava_config();
    let fitbit_config = create_valid_fitbit_config();

    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config)?;

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

    let provider = StravaOAuthProvider::from_config(&config)?;
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

    let provider = StravaOAuthProvider::from_config(&config)?;
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

    // Provider should still be creatable even if disabled
    let provider = StravaOAuthProvider::from_config(&config)?;

    assert_eq!(provider.name(), "strava");

    Ok(())
}

// === Token Data Edge Cases ===

#[tokio::test]
async fn test_token_validation_with_wrong_provider() -> Result<()> {
    let strava_config = create_valid_strava_config();
    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;

    // Create token data for fitbit but validate with strava provider
    let fitbit_token = create_test_token_data("fitbit");

    let _is_valid = strava_provider.validate_token(&fitbit_token).await;

    Ok(())
}

#[tokio::test]
async fn test_token_validation_edge_case_expires_exactly_now() -> Result<()> {
    let config = create_valid_strava_config();
    let provider = StravaOAuthProvider::from_config(&config)?;

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
    let strava_config = create_valid_strava_config();
    let fitbit_config = create_valid_fitbit_config();

    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config)?;

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

    let mut handles = vec![];

    for i in 0..5 {
        let config_clone = config.clone();
        handles.push(tokio::spawn(async move {
            let provider = StravaOAuthProvider::from_config(&config_clone)?;
            let user_id = Uuid::new_v4();
            let state = format!("concurrent_test_{}", i);
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

    let mut handles = vec![];

    for i in 0..3 {
        let config_clone = config.clone();
        handles.push(tokio::spawn(async move {
            let provider = StravaOAuthProvider::from_config(&config_clone)?;
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
