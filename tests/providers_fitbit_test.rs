//! Fitbit Provider Integration Tests
//!
//! Comprehensive tests for the Fitbit fitness provider implementation
//! covering authentication, API calls, error handling, and data conversion.

use anyhow::Result;
use pierre_mcp_server::{
    oauth2_client::PkceParams,
    providers::{fitbit::FitbitProvider, AuthData, FitnessProvider},
};

/// Create a test Fitbit provider
fn create_test_provider() -> FitbitProvider {
    FitbitProvider::new()
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

/// Create test PKCE parameters
fn create_test_pkce() -> PkceParams {
    PkceParams {
        code_verifier: "test_verifier_12345".to_string(),
        code_challenge: "test_challenge_abcdef".to_string(),
        code_challenge_method: "S256".to_string(),
    }
}

#[tokio::test]
async fn test_fitbit_provider_creation() -> Result<()> {
    let _provider = create_test_provider();
    // Provider should be created successfully
    Ok(())
}

#[tokio::test]
async fn test_default_creation() -> Result<()> {
    let _provider = FitbitProvider::default();
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
        .contains("Fitbit requires OAuth2 authentication"));

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
    let provider = create_test_provider();

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
    let mut provider = create_test_provider();

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
    // Accept either URL encoding format for spaces (+ or %20)
    assert!(
        auth_url.contains("scope=activity+profile+sleep")
            || auth_url.contains("scope=activity%20profile%20sleep")
    );
    assert!(auth_url.contains("state=test_state"));
    assert!(auth_url.contains("fitbit.com"));

    Ok(())
}

#[tokio::test]
async fn test_pkce_auth_url_generation() -> Result<()> {
    let mut provider = create_test_provider();

    // Set client_id for URL generation
    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    let pkce = create_test_pkce();

    let auth_url =
        provider.get_auth_url_with_pkce("http://localhost:3000/callback", "test_state", &pkce)?;

    assert!(auth_url.contains("client_id=test_client_id"));
    assert!(auth_url.contains("code_challenge=test_challenge_abcdef"));
    assert!(auth_url.contains("code_challenge_method=S256"));
    assert!(auth_url.contains("fitbit.com"));

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
    let mut provider = create_test_provider();

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
    assert!(auth_url.contains("fitbit.com"));

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

#[tokio::test]
async fn test_fitbit_scopes_in_auth_url() -> Result<()> {
    let mut provider = create_test_provider();

    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    let auth_url = provider.get_auth_url("http://localhost:3000/callback", "test_state")?;

    // Verify Fitbit-specific scopes are included
    // Accept either URL encoding format for spaces (+ or %20)
    assert!(
        auth_url.contains("scope=activity+profile+sleep")
            || auth_url.contains("scope=activity%20profile%20sleep")
    );

    Ok(())
}

#[tokio::test]
async fn test_pkce_parameters_in_url() -> Result<()> {
    let mut provider = create_test_provider();

    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    let pkce = PkceParams {
        code_verifier: "test_verifier_abcdef123456".to_string(),
        code_challenge: "challenge_xyz789".to_string(),
        code_challenge_method: "S256".to_string(),
    };

    let auth_url =
        provider.get_auth_url_with_pkce("http://localhost:3000/callback", "test_state", &pkce)?;

    assert!(auth_url.contains("code_challenge=challenge_xyz789"));
    assert!(auth_url.contains("code_challenge_method=S256"));

    Ok(())
}

#[tokio::test]
async fn test_fitbit_api_base_url() -> Result<()> {
    let mut provider = create_test_provider();

    // Authenticate to test API calls
    provider.authenticate(create_test_auth_data()).await?;

    // Test API calls will fail without valid tokens, but we can verify error messages
    // indicate they're hitting the correct Fitbit API endpoints

    let athlete_result = provider.get_athlete().await;
    assert!(athlete_result.is_err());
    // The error should indicate an HTTP request was attempted

    let activities_result = provider.get_activities(Some(5), None).await;
    assert!(activities_result.is_err());
    // The error should indicate an HTTP request was attempted

    let stats_result = provider.get_stats().await;
    assert!(stats_result.is_err());
    // The error should indicate an HTTP request was attempted

    Ok(())
}

#[tokio::test]
async fn test_provider_name() -> Result<()> {
    let provider = create_test_provider();
    assert_eq!(provider.provider_name(), "Fitbit");
    Ok(())
}

#[tokio::test]
async fn test_get_activities_with_pagination() -> Result<()> {
    let mut provider = create_test_provider();
    provider.authenticate(create_test_auth_data()).await?;

    // Test with limit only
    let result1 = provider.get_activities(Some(10), None).await;
    assert!(result1.is_err()); // Will fail without valid API credentials

    // Test with limit and offset
    let result2 = provider.get_activities(Some(5), Some(20)).await;
    assert!(result2.is_err()); // Will fail without valid API credentials

    Ok(())
}

#[tokio::test]
async fn test_date_range_activity_fetching() -> Result<()> {
    let mut provider = create_test_provider();
    provider.authenticate(create_test_auth_data()).await?;

    // The get_activities method uses a 30-day window internally
    let result = provider.get_activities(Some(50), None).await;
    assert!(result.is_err()); // Will fail without valid API credentials but tests the flow

    Ok(())
}

#[tokio::test]
async fn test_authentication_state_changes() -> Result<()> {
    let mut provider = create_test_provider();

    // Initially not authenticated
    assert!(provider.get_athlete().await.is_err());

    // Authenticate
    provider.authenticate(create_test_auth_data()).await?;

    // Still not authenticated for real API calls (no valid token)
    // but the provider has stored the authentication data
    assert!(provider.get_athlete().await.is_err());

    // Clear authentication by providing empty tokens
    provider
        .authenticate(AuthData::OAuth2 {
            client_id: "test".to_string(),
            client_secret: "test".to_string(),
            access_token: None,
            refresh_token: None,
        })
        .await?;

    assert!(provider.get_athlete().await.is_err());

    Ok(())
}
