//! Strava Provider Integration Tests
//!
//! Comprehensive tests for the Strava fitness provider implementation
//! covering authentication, API calls, error handling, and data conversion.

use anyhow::Result;
use pierre_mcp_server::providers::{
    strava::{StravaConfig, StravaProvider},
    AuthData, FitnessProvider,
};

/// Create a test Strava provider
fn create_test_provider() -> StravaProvider {
    StravaProvider::new()
}

/// Create a test Strava provider with valid config for auth URL tests
fn create_test_provider_with_config() -> StravaProvider {
    let test_config = StravaConfig {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        base_url: "https://www.strava.com/api/v3".to_string(),
        auth_url: "https://www.strava.com/oauth/authorize".to_string(),
        token_url: "https://www.strava.com/oauth/token".to_string(),
    };
    let static_config: &'static StravaConfig = Box::leak(Box::new(test_config));
    StravaProvider::with_config(static_config)
}

/// Create a test Strava provider with empty client credentials for error testing
fn create_test_provider_no_credentials() -> StravaProvider {
    let empty_config = StravaConfig {
        client_id: String::new(),
        client_secret: String::new(),
        base_url: "https://www.strava.com/api/v3".to_string(),
        auth_url: "https://www.strava.com/oauth/authorize".to_string(),
        token_url: "https://www.strava.com/oauth/token".to_string(),
    };
    let static_config: &'static StravaConfig = Box::leak(Box::new(empty_config));
    StravaProvider::with_config(static_config)
}

/// Create test OAuth2 authentication data
fn create_test_auth_data() -> AuthData {
    AuthData::OAuth2 {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: Some("test_access_token".to_string()),
        refresh_token: Some("test_refresh_token".to_string()),
    }
}

#[tokio::test]
async fn test_strava_provider_creation() -> Result<()> {
    let _provider = create_test_provider();
    // Provider should be created successfully
    Ok(())
}

#[tokio::test]
async fn test_default_creation() -> Result<()> {
    let _provider = StravaProvider::default();
    // Default provider should be created successfully
    Ok(())
}

#[tokio::test]
async fn test_oauth2_authentication_success() -> Result<()> {
    let mut provider = create_test_provider();
    let auth_data = create_test_auth_data();

    let result = provider.authenticate(auth_data).await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_oauth2_authentication_partial_data() -> Result<()> {
    let mut provider = create_test_provider();

    // Test with minimal OAuth2 data
    let auth_data = AuthData::OAuth2 {
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        access_token: None,
        refresh_token: None,
    };

    let result = provider.authenticate(auth_data).await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_invalid_authentication_type() -> Result<()> {
    let mut provider = create_test_provider();

    // Try to authenticate with API key instead of OAuth2
    let invalid_auth = AuthData::ApiKey("invalid_key".to_string());

    let result = provider.authenticate(invalid_auth).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("OAuth2 authentication"));

    Ok(())
}

#[tokio::test]
async fn test_get_athlete_not_authenticated() -> Result<()> {
    let provider = create_test_provider();

    let result = provider.get_athlete().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not authenticated"));

    Ok(())
}

#[tokio::test]
async fn test_get_activities_not_authenticated() -> Result<()> {
    let provider = create_test_provider();

    let result = provider.get_activities(None, None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not authenticated"));

    Ok(())
}

#[tokio::test]
async fn test_get_stats_not_authenticated() -> Result<()> {
    let provider = create_test_provider();

    let result = provider.get_stats().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not authenticated"));

    Ok(())
}

#[tokio::test]
async fn test_auth_url_generation_no_client_id() -> Result<()> {
    let provider = create_test_provider_no_credentials();

    let result = provider.get_auth_url("http://localhost:3000/callback", "test_state");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Client ID not configured"));

    Ok(())
}

#[tokio::test]
async fn test_auth_url_generation_with_client_id() -> Result<()> {
    let mut provider = create_test_provider_with_config();

    // Set client_id for URL generation
    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    let auth_url = provider.get_auth_url("http://localhost:3000/callback", "test_state")?;

    // Print the URL for debugging
    println!("Generated auth URL: {}", auth_url);

    assert!(auth_url.contains("client_id=test_client_id"));
    assert!(auth_url.contains("redirect_uri="));
    assert!(auth_url.contains("localhost%3A3000")); // Check for localhost encoding
    assert!(auth_url.contains("response_type=code"));
    assert!(auth_url.contains("scope=read"));
    assert!(auth_url.contains("state=test_state"));
    assert!(auth_url.contains("strava.com"));

    Ok(())
}

#[tokio::test]
async fn test_pkce_auth_url_generation() -> Result<()> {
    let mut provider = create_test_provider_with_config();

    // Set client_id for URL generation
    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    // Create test PKCE parameters
    let pkce = pierre_mcp_server::oauth2_client::PkceParams {
        code_verifier: "test_verifier".to_string(),
        code_challenge: "test_challenge".to_string(),
        code_challenge_method: "S256".to_string(),
    };

    let auth_url =
        provider.get_auth_url_with_pkce("http://localhost:3000/callback", "test_state", &pkce)?;

    assert!(auth_url.contains("client_id=test_client_id"));
    assert!(auth_url.contains("code_challenge=test_challenge"));
    assert!(auth_url.contains("code_challenge_method=S256"));
    assert!(auth_url.contains("strava.com"));

    Ok(())
}

#[tokio::test]
async fn test_multiple_authentication_calls() -> Result<()> {
    let mut provider = create_test_provider();

    // First authentication
    let auth_data1 = AuthData::OAuth2 {
        client_id: "client1".to_string(),
        client_secret: "secret1".to_string(),
        access_token: Some("token1".to_string()),
        refresh_token: Some("refresh1".to_string()),
    };

    let result1 = provider.authenticate(auth_data1).await;
    assert!(result1.is_ok());

    // Second authentication should overwrite the first
    let auth_data2 = AuthData::OAuth2 {
        client_id: "client2".to_string(),
        client_secret: "secret2".to_string(),
        access_token: Some("token2".to_string()),
        refresh_token: Some("refresh2".to_string()),
    };

    let result2 = provider.authenticate(auth_data2).await;
    assert!(result2.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_concurrent_provider_usage() -> Result<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Create multiple providers for concurrent testing
    let provider1 = Arc::new(Mutex::new(create_test_provider()));
    let provider2 = Arc::new(Mutex::new(create_test_provider()));

    // Authenticate both providers concurrently
    let auth_task1 = {
        let provider = provider1.clone();
        tokio::spawn(async move {
            let auth_data = AuthData::OAuth2 {
                client_id: "client1".to_string(),
                client_secret: "secret1".to_string(),
                access_token: Some("token1".to_string()),
                refresh_token: Some("refresh1".to_string()),
            };
            provider.lock().await.authenticate(auth_data).await
        })
    };

    let auth_task2 = {
        let provider = provider2.clone();
        tokio::spawn(async move {
            let auth_data = AuthData::OAuth2 {
                client_id: "client2".to_string(),
                client_secret: "secret2".to_string(),
                access_token: Some("token2".to_string()),
                refresh_token: Some("refresh2".to_string()),
            };
            provider.lock().await.authenticate(auth_data).await
        })
    };

    // Wait for both tasks to complete
    let (result1, result2) = tokio::try_join!(auth_task1, auth_task2)?;

    // Verify both authentications succeeded
    assert!(result1.is_ok());
    assert!(result2.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_empty_authentication_fields() -> Result<()> {
    let mut provider = create_test_provider();

    // Test with empty client credentials
    let auth_data = AuthData::OAuth2 {
        client_id: "".to_string(),
        client_secret: "".to_string(),
        access_token: None,
        refresh_token: None,
    };

    let result = provider.authenticate(auth_data).await;
    assert!(result.is_ok()); // Should succeed but may fail later when making API calls

    Ok(())
}

#[tokio::test]
async fn test_url_generation_edge_cases() -> Result<()> {
    let mut provider = create_test_provider_with_config();

    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    // Test with special characters in redirect URI
    let auth_url = provider.get_auth_url(
        "http://localhost:3000/callback?param=value&other=test",
        "state_with_special_chars!@#$%",
    )?;

    assert!(auth_url.contains("client_id=test_client_id"));
    assert!(auth_url.contains("strava.com"));

    // Test with empty state
    let auth_url_empty_state = provider.get_auth_url("http://localhost:3000/callback", "")?;

    assert!(auth_url_empty_state.contains("state="));

    Ok(())
}

#[tokio::test]
async fn test_provider_state_isolation() -> Result<()> {
    // Create two separate providers
    let mut provider1 = create_test_provider();
    let provider2 = create_test_provider();

    // Authenticate first provider
    provider1
        .authenticate(AuthData::OAuth2 {
            client_id: "client1".to_string(),
            client_secret: "secret1".to_string(),
            access_token: Some("token1".to_string()),
            refresh_token: Some("refresh1".to_string()),
        })
        .await?;

    // Second provider should still be unauthenticated
    let result = provider2.get_athlete().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Not authenticated"));

    Ok(())
}
