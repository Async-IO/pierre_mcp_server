//! Comprehensive tests for OAuth providers to improve coverage
//!
//! This test suite focuses on oauth/providers.rs which has 43.19% coverage

use anyhow::Result;
use chrono::Utc;
use pierre_mcp_server::{
    config::environment::OAuthProviderConfig,
    database_plugins::DatabaseProvider,
    models::{EncryptedToken, User},
    oauth::{
        manager::OAuthManager,
        providers::{FitbitOAuthProvider, StravaOAuthProvider},
    },
};
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_oauth_provider_creation_success() -> Result<()> {
    // Test successful creation of OAuth providers
    let valid_config = OAuthProviderConfig {
        client_id: Some("test_client_id".to_string()),
        client_secret: Some("test_client_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/callback".to_string()),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        enabled: true,
    };

    // Test Strava provider
    let strava_provider = StravaOAuthProvider::from_config(&valid_config)?;
    // Just verify creation succeeded
    let _ = strava_provider;

    // Test Fitbit provider
    let fitbit_config = OAuthProviderConfig {
        client_id: Some("fitbit_client".to_string()),
        client_secret: Some("fitbit_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/fitbit/callback".to_string()),
        scopes: vec!["activity".to_string(), "profile".to_string()],
        enabled: true,
    };
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config)?;
    // Just verify creation succeeded
    let _ = fitbit_provider;

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_missing_client_id() -> Result<()> {
    // Test error when client_id is missing
    let invalid_config = OAuthProviderConfig {
        client_id: None,
        client_secret: Some("secret".to_string()),
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };

    let strava_result = StravaOAuthProvider::from_config(&invalid_config);
    assert!(strava_result.is_err());
    if let Err(e) = strava_result {
        assert!(e.to_string().contains("client_id"));
    }

    let fitbit_result = FitbitOAuthProvider::from_config(&invalid_config);
    assert!(fitbit_result.is_err());
    if let Err(e) = fitbit_result {
        assert!(e.to_string().contains("client_id"));
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_missing_client_secret() -> Result<()> {
    // Test error when client_secret is missing
    let invalid_config = OAuthProviderConfig {
        client_id: Some("client".to_string()),
        client_secret: None,
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };

    let strava_result = StravaOAuthProvider::from_config(&invalid_config);
    assert!(strava_result.is_err());
    if let Err(e) = strava_result {
        assert!(e.to_string().contains("client_secret"));
    }

    let fitbit_result = FitbitOAuthProvider::from_config(&invalid_config);
    assert!(fitbit_result.is_err());
    if let Err(e) = fitbit_result {
        assert!(e.to_string().contains("client_secret"));
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_missing_redirect_uri() -> Result<()> {
    // Test that providers provide default redirect_uri when None
    let config_no_redirect = OAuthProviderConfig {
        client_id: Some("client".to_string()),
        client_secret: Some("secret".to_string()),
        redirect_uri: None,
        scopes: vec!["read".to_string()],
        enabled: true,
    };

    // Providers should succeed and provide default redirect URIs
    let strava_result = StravaOAuthProvider::from_config(&config_no_redirect);
    assert!(strava_result.is_ok());

    let fitbit_result = FitbitOAuthProvider::from_config(&config_no_redirect);
    assert!(fitbit_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_disabled() -> Result<()> {
    // Test that providers currently ignore the enabled field
    let disabled_config = OAuthProviderConfig {
        client_id: Some("client".to_string()),
        client_secret: Some("secret".to_string()),
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: false,
    };

    // Current implementation ignores the enabled field and succeeds
    let strava_result = StravaOAuthProvider::from_config(&disabled_config);
    assert!(strava_result.is_ok());

    let fitbit_result = FitbitOAuthProvider::from_config(&disabled_config);
    assert!(fitbit_result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_empty_scopes() -> Result<()> {
    // Test provider with empty scopes
    let config = OAuthProviderConfig {
        client_id: Some("client".to_string()),
        client_secret: Some("secret".to_string()),
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec![], // Empty scopes
        enabled: true,
    };

    // Providers should handle empty scopes gracefully
    let strava_provider = StravaOAuthProvider::from_config(&config)?;
    let _ = strava_provider; // Just verify creation succeeded

    let fitbit_provider = FitbitOAuthProvider::from_config(&config)?;
    let _ = fitbit_provider; // Just verify creation succeeded

    Ok(())
}

#[tokio::test]
async fn test_oauth_manager_provider_registration() -> Result<()> {
    let database = common::create_test_database().await?;
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register Strava provider
    let strava_config = OAuthProviderConfig {
        client_id: Some("strava_client".to_string()),
        client_secret: Some("strava_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/strava".to_string()),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        enabled: true,
    };
    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;
    oauth_manager.register_provider(Box::new(strava_provider));

    // Register Fitbit provider
    let fitbit_config = OAuthProviderConfig {
        client_id: Some("fitbit_client".to_string()),
        client_secret: Some("fitbit_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/fitbit".to_string()),
        scopes: vec!["activity".to_string()],
        enabled: true,
    };
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config)?;
    oauth_manager.register_provider(Box::new(fitbit_provider));

    // Test generating auth URLs
    let user = User::new(
        "oauth_manager@example.com".to_string(),
        "password".to_string(),
        Some("OAuth Manager Test".to_string()),
    );
    database.create_user(&user).await?;

    // Test with registered providers
    let strava_auth = oauth_manager.generate_auth_url(user.id, "strava").await;
    assert!(strava_auth.is_ok());
    let strava_response = strava_auth?;
    assert_eq!(strava_response.provider, "strava");
    assert!(!strava_response.state.is_empty());

    let fitbit_auth = oauth_manager.generate_auth_url(user.id, "fitbit").await;
    assert!(fitbit_auth.is_ok());
    let fitbit_response = fitbit_auth?;
    assert_eq!(fitbit_response.provider, "fitbit");

    // Test with unregistered provider
    let unknown_auth = oauth_manager.generate_auth_url(user.id, "unknown").await;
    assert!(unknown_auth.is_err());

    Ok(())
}

#[tokio::test]
async fn test_oauth_manager_connection_status() -> Result<()> {
    let database = common::create_test_database().await?;
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register providers
    let strava_config = OAuthProviderConfig {
        client_id: Some("strava_client".to_string()),
        client_secret: Some("strava_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/strava".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };
    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;
    oauth_manager.register_provider(Box::new(strava_provider));

    // Create users with different token states
    let user_no_tokens = User::new(
        "no_tokens@example.com".to_string(),
        "password".to_string(),
        Some("No Tokens".to_string()),
    );
    database.create_user(&user_no_tokens).await?;

    // Create user with proper encrypted token
    let user_with_token_id = Uuid::new_v4();
    let expires_at = Utc::now() + chrono::Duration::hours(6);

    // First create a user without tokens
    let user_with_token = User {
        id: user_with_token_id,
        email: "with_token@example.com".to_string(),
        display_name: Some("With Token".to_string()),
        password_hash: "hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: Utc::now(),
        last_active: Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user_with_token).await?;

    // Then add tokens using the OAuth manager methods (which handle encryption properly)
    database
        .update_strava_token(
            user_with_token_id,
            "test_access_token",
            "test_refresh_token",
            expires_at,
            "read".to_string(),
        )
        .await?;

    // Test connection status
    let status1 = oauth_manager
        .get_connection_status(user_no_tokens.id)
        .await?;
    assert_eq!(status1.get("strava"), Some(&false));

    let status2 = oauth_manager
        .get_connection_status(user_with_token_id)
        .await?;
    assert_eq!(status2.get("strava"), Some(&true));

    Ok(())
}

#[tokio::test]
async fn test_oauth_manager_ensure_valid_token() -> Result<()> {
    let database = common::create_test_database().await?;
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register provider
    let config = OAuthProviderConfig {
        client_id: Some("test_client".to_string()),
        client_secret: Some("test_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };
    let provider = StravaOAuthProvider::from_config(&config)?;
    oauth_manager.register_provider(Box::new(provider));

    // Test with user without token
    let user = User::new(
        "token_test@example.com".to_string(),
        "password".to_string(),
        Some("Token Test".to_string()),
    );
    database.create_user(&user).await?;

    let token_result = oauth_manager.ensure_valid_token(user.id, "strava").await?;
    assert!(token_result.is_none());

    // Test with expired token - create user first, then add expired token
    let user_expired_id = Uuid::new_v4();
    let user_expired = User {
        id: user_expired_id,
        email: "expired@example.com".to_string(),
        display_name: Some("Expired Token".to_string()),
        password_hash: "hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: Utc::now(),
        last_active: Utc::now(),
        is_active: true,
        strava_token: None,
        fitbit_token: None,
    };
    database.create_user(&user_expired).await?;

    // Add expired token using proper database method
    let expired_time = Utc::now() - chrono::Duration::hours(1);
    database
        .update_strava_token(
            user_expired_id,
            "expired_access_token",
            "expired_refresh_token",
            expired_time,
            "read".to_string(),
        )
        .await?;

    // This should attempt to refresh but will fail due to invalid tokens
    let expired_result = oauth_manager
        .ensure_valid_token(user_expired_id, "strava")
        .await;
    // Token refresh should fail because the tokens are fake/invalid, so ensure_valid_token should return an error
    assert!(expired_result.is_err());
    if let Err(e) = expired_result {
        // Should be a token refresh failure
        assert!(e.to_string().contains("Token refresh failed"));
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth_manager_handle_callback_errors() -> Result<()> {
    let database = common::create_test_database().await?;
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register provider
    let config = OAuthProviderConfig {
        client_id: Some("callback_test".to_string()),
        client_secret: Some("callback_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };
    let provider = StravaOAuthProvider::from_config(&config)?;
    oauth_manager.register_provider(Box::new(provider));

    // Test with invalid state
    let invalid_state_result = oauth_manager
        .handle_callback("code123", "invalid_state", "strava")
        .await;
    assert!(invalid_state_result.is_err());
    assert!(invalid_state_result
        .unwrap_err()
        .to_string()
        .contains("Invalid state"));

    // Test with empty code
    let empty_code_result = oauth_manager
        .handle_callback("", "some_state", "strava")
        .await;
    assert!(empty_code_result.is_err());

    // Test with unknown provider using generate_auth_url which checks provider first
    let test_user = User::new(
        "test_unknown@example.com".to_string(),
        "password".to_string(),
        Some("Test Unknown".to_string()),
    );
    database.create_user(&test_user).await?;

    // Test unknown provider through generate_auth_url
    let unknown_provider_result = oauth_manager
        .generate_auth_url(test_user.id, "unknown")
        .await;
    assert!(unknown_provider_result.is_err());
    assert!(unknown_provider_result
        .unwrap_err()
        .to_string()
        .contains("Provider not supported: unknown"));

    Ok(())
}

#[tokio::test]
async fn test_oauth_disconnect_provider() -> Result<()> {
    let database = common::create_test_database().await?;
    let mut oauth_manager = OAuthManager::new(database.clone());

    // Register providers so disconnect can work
    let strava_config = OAuthProviderConfig {
        client_id: Some("disconnect_client".to_string()),
        client_secret: Some("disconnect_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/strava".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };
    let strava_provider = StravaOAuthProvider::from_config(&strava_config)?;
    oauth_manager.register_provider(Box::new(strava_provider));

    let fitbit_config = OAuthProviderConfig {
        client_id: Some("fitbit_disconnect_client".to_string()),
        client_secret: Some("fitbit_disconnect_secret".to_string()),
        redirect_uri: Some("http://localhost:3000/oauth/fitbit".to_string()),
        scopes: vec!["activity".to_string()],
        enabled: true,
    };
    let fitbit_provider = FitbitOAuthProvider::from_config(&fitbit_config)?;
    oauth_manager.register_provider(Box::new(fitbit_provider));

    // Create user with tokens
    let user = User {
        id: Uuid::new_v4(),
        email: "disconnect@example.com".to_string(),
        display_name: Some("Disconnect Test".to_string()),
        password_hash: "hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        created_at: Utc::now(),
        last_active: Utc::now(),
        is_active: true,
        strava_token: Some(EncryptedToken {
            access_token: "access".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            scope: "read".to_string(),
            nonce: "nonce".to_string(),
        }),
        fitbit_token: Some(EncryptedToken {
            access_token: "fitbit_access".to_string(),
            refresh_token: "fitbit_refresh".to_string(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
            scope: "activity".to_string(),
            nonce: "fitbit_nonce".to_string(),
        }),
    };
    database.create_user(&user).await?;

    // Test disconnecting providers
    let strava_disconnect = oauth_manager.disconnect_provider(user.id, "strava").await;
    assert!(strava_disconnect.is_ok());

    let fitbit_disconnect = oauth_manager.disconnect_provider(user.id, "fitbit").await;
    assert!(fitbit_disconnect.is_ok());

    // Test disconnecting unknown provider
    let unknown_disconnect = oauth_manager.disconnect_provider(user.id, "unknown").await;
    assert!(unknown_disconnect.is_err());

    // Verify tokens are cleared
    let strava_token = database.get_strava_token(user.id).await?;
    assert!(strava_token.is_none());

    let fitbit_token = database.get_fitbit_token(user.id).await?;
    assert!(fitbit_token.is_none());

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_scope_handling() -> Result<()> {
    // Test various scope configurations
    let scope_tests = vec![
        vec!["read".to_string()],
        vec!["read".to_string(), "activity:read_all".to_string()],
        vec![
            "activity".to_string(),
            "profile".to_string(),
            "heartrate".to_string(),
        ],
        vec![], // Empty scopes
        vec!["custom_scope".to_string()],
    ];

    for scopes in scope_tests {
        let config = OAuthProviderConfig {
            client_id: Some("scope_test".to_string()),
            client_secret: Some("scope_secret".to_string()),
            redirect_uri: Some("http://localhost:3000/callback".to_string()),
            scopes: scopes.clone(),
            enabled: true,
        };

        let strava_provider = StravaOAuthProvider::from_config(&config)?;
        let _ = strava_provider; // Just verify creation succeeded

        let fitbit_provider = FitbitOAuthProvider::from_config(&config)?;
        let _ = fitbit_provider; // Just verify creation succeeded
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth_provider_redirect_uri_variations() -> Result<()> {
    // Test various redirect URI formats
    let uri_tests = vec![
        "http://localhost:3000/callback",
        "https://example.com/oauth/callback",
        "http://127.0.0.1:8080/auth/callback",
        "https://app.example.com/oauth/strava/callback",
    ];

    for uri in uri_tests {
        let config = OAuthProviderConfig {
            client_id: Some("uri_test".to_string()),
            client_secret: Some("uri_secret".to_string()),
            redirect_uri: Some(uri.to_string()),
            scopes: vec!["read".to_string()],
            enabled: true,
        };

        let strava_provider = StravaOAuthProvider::from_config(&config)?;
        let _ = strava_provider; // Just verify creation succeeded

        let fitbit_provider = FitbitOAuthProvider::from_config(&config)?;
        let _ = fitbit_provider; // Just verify creation succeeded
    }

    Ok(())
}
