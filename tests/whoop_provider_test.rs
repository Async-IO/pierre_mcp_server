// ABOUTME: Comprehensive test suite for WHOOP provider implementation
// ABOUTME: Tests provider creation, configuration, OAuth flow, and data conversion
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::config::environment::HttpClientConfig;
use pierre_mcp_server::constants::{init_server_config, oauth_providers};
use pierre_mcp_server::models::SportType;
use pierre_mcp_server::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use pierre_mcp_server::providers::registry::{get_supported_providers, global_registry};
use pierre_mcp_server::providers::whoop_provider::WhoopProvider;
use pierre_mcp_server::utils::http_client::initialize_http_clients;
use std::sync::Once;

/// Ensure HTTP clients and server config are initialized only once across all tests
static INIT_HTTP_CLIENTS: Once = Once::new();
static INIT_SERVER_CONFIG: Once = Once::new();

fn ensure_http_clients_initialized() {
    // Initialize server config first (required for provider defaults)
    INIT_SERVER_CONFIG.call_once(|| {
        let _ = init_server_config();
    });

    INIT_HTTP_CLIENTS.call_once(|| {
        initialize_http_clients(HttpClientConfig::default());
    });
}

#[test]
fn test_whoop_provider_creation() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    assert_eq!(provider.name(), oauth_providers::WHOOP);
    assert_eq!(provider.config().name, oauth_providers::WHOOP);
    assert!(provider.config().auth_url.contains("whoop.com"));
    assert!(provider.config().api_base_url.contains("whoop.com"));
}

#[test]
fn test_whoop_provider_with_custom_config() {
    ensure_http_clients_initialized();
    let custom_config = ProviderConfig {
        name: oauth_providers::WHOOP.to_owned(),
        auth_url: "https://custom.whoop.com/auth".to_owned(),
        token_url: "https://custom.whoop.com/token".to_owned(),
        api_base_url: "https://custom.whoop.com/api".to_owned(),
        revoke_url: Some("https://custom.whoop.com/revoke".to_owned()),
        default_scopes: vec!["custom:scope".to_owned()],
    };

    let provider = WhoopProvider::with_config(custom_config.clone());

    assert_eq!(provider.config().name, custom_config.name);
    assert_eq!(provider.config().auth_url, custom_config.auth_url);
    assert_eq!(provider.config().token_url, custom_config.token_url);
    assert_eq!(provider.config().api_base_url, custom_config.api_base_url);
}

#[tokio::test]
async fn test_whoop_provider_authentication_lifecycle() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Initially not authenticated
    assert!(!provider.is_authenticated().await);

    // Set valid credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:profile".to_owned(), "read:workout".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Now authenticated
    assert!(provider.is_authenticated().await);
}

#[tokio::test]
async fn test_whoop_provider_expired_token() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Set expired credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("expired_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() - chrono::Duration::hours(1)), // Already expired
        scopes: vec!["read:profile".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Not authenticated due to expired token
    assert!(!provider.is_authenticated().await);
}

#[tokio::test]
async fn test_whoop_provider_no_expiry() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Credentials with no expiry time
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: None, // No expiry
        scopes: vec!["read:profile".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Authenticated (no expiry means valid indefinitely)
    assert!(provider.is_authenticated().await);
}

#[test]
fn test_whoop_provider_default() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::default();
    assert_eq!(provider.name(), oauth_providers::WHOOP);
}

#[tokio::test]
async fn test_whoop_provider_disconnect() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:profile".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");
    assert!(provider.is_authenticated().await);

    // Disconnect
    provider.disconnect().await.expect("Failed to disconnect");

    // No longer authenticated
    assert!(!provider.is_authenticated().await);
}

#[test]
fn test_whoop_provider_scopes() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();
    let scopes = &provider.config().default_scopes;

    // WHOOP scopes are space-separated
    assert!(scopes.iter().any(|s| s == "offline"));
    assert!(scopes.iter().any(|s| s == "read:profile"));
    assert!(scopes.iter().any(|s| s == "read:workout"));
    assert!(scopes.iter().any(|s| s == "read:sleep"));
    assert!(scopes.iter().any(|s| s == "read:recovery"));
    assert!(scopes.iter().any(|s| s == "read:cycles"));
}

#[test]
fn test_whoop_provider_endpoints() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();
    let config = provider.config();

    // Verify all required endpoints are configured
    assert!(config.auth_url.starts_with("https://"));
    assert!(config.token_url.starts_with("https://"));
    assert!(config.api_base_url.starts_with("https://"));
    assert!(config.revoke_url.is_some());
    assert!(config.revoke_url.as_ref().unwrap().starts_with("https://"));

    // Verify WHOOP API URLs
    assert!(config.auth_url.contains("api.prod.whoop.com"));
    assert!(config.token_url.contains("api.prod.whoop.com"));
    assert!(config.api_base_url.contains("api.prod.whoop.com"));
}

#[tokio::test]
async fn test_whoop_provider_get_athlete_requires_auth() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Attempt to get athlete without authentication
    let result = provider.get_athlete().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_whoop_provider_get_activities_requires_auth() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Attempt to get activities without authentication
    let result = provider.get_activities(Some(10), None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_whoop_provider_get_activity_requires_auth() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Attempt to get specific activity without authentication
    let result = provider.get_activity("test-uuid").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_whoop_provider_get_personal_records() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: None,
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:profile".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Personal records should return empty vec (WHOOP doesn't track PRs)
    let result = provider
        .get_personal_records()
        .await
        .expect("Failed to get personal records");
    assert!(result.is_empty());
}

#[tokio::test]
async fn test_whoop_provider_refresh_token_no_credentials() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Attempt to refresh without credentials
    let result = provider.refresh_token_if_needed().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_whoop_provider_refresh_token_not_needed() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Set credentials that don't need refresh (expires in 2 hours)
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(2)),
        scopes: vec!["read:profile".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Refresh should succeed without actually refreshing
    let result = provider.refresh_token_if_needed().await;
    assert!(result.is_ok());
}

#[test]
fn test_whoop_in_provider_registry() {
    ensure_http_clients_initialized();
    let registry = global_registry();
    // Verify WHOOP is in the list of all providers using the registry
    let all_providers = registry.supported_providers();
    assert!(all_providers.contains(&oauth_providers::WHOOP));
    assert!(registry.is_supported(oauth_providers::WHOOP));
}

#[test]
fn test_whoop_provider_factory() {
    ensure_http_clients_initialized();
    let registry = global_registry();

    // Verify WHOOP is supported
    assert!(registry.is_supported(oauth_providers::WHOOP));

    // Verify we can create a WHOOP provider
    let provider = registry
        .create_provider(oauth_providers::WHOOP)
        .expect("Failed to create WHOOP provider");

    assert_eq!(provider.name(), oauth_providers::WHOOP);
}

#[test]
fn test_whoop_in_supported_providers_list() {
    let supported = get_supported_providers();
    assert!(supported.contains(&oauth_providers::WHOOP));
}

#[tokio::test]
async fn test_whoop_credentials_without_access_token() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();

    // Credentials without access token
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: None, // No access token
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["read:profile".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Not authenticated without access token
    assert!(!provider.is_authenticated().await);
}

#[test]
fn test_whoop_provider_config_urls() {
    ensure_http_clients_initialized();
    let provider = WhoopProvider::new();
    let config = provider.config();

    // Verify URLs don't have trailing slashes
    assert!(!config.api_base_url.ends_with('/'));
    assert!(!config.auth_url.ends_with('/'));
    assert!(!config.token_url.ends_with('/'));
}

/// Test that the `SportType` enum can parse various sport types
///
/// Note: WHOOP provider's internal sport ID mapping is tested indirectly through
/// activity conversion. This test verifies the public `SportType` API works correctly.
#[test]
fn test_sport_type_conversion_basic() {
    // Test a sample of common sport types
    assert!(matches!(
        SportType::from_internal_string("run"),
        SportType::Run
    ));
    assert!(matches!(
        SportType::from_internal_string("bike_ride"),
        SportType::Ride
    ));
    assert!(matches!(
        SportType::from_internal_string("swim"),
        SportType::Swim
    ));
    assert!(matches!(
        SportType::from_internal_string("yoga"),
        SportType::Yoga
    ));

    // Unknown sport should return Other
    if let SportType::Other(name) = SportType::from_internal_string("unknown_sport_xyz") {
        assert_eq!(name, "unknown_sport_xyz");
    } else {
        panic!("Expected SportType::Other for unknown sport");
    }
}

#[test]
fn test_whoop_provider_has_full_health_capabilities() {
    ensure_http_clients_initialized();
    let registry = global_registry();

    // WHOOP should have full health capabilities
    assert!(registry.supports_sleep(oauth_providers::WHOOP));
    assert!(registry.supports_recovery(oauth_providers::WHOOP));

    // Get capabilities
    let capabilities = registry.get_capabilities(oauth_providers::WHOOP);
    assert!(capabilities.is_some());

    let caps = capabilities.unwrap();
    assert!(caps.requires_oauth());
    assert!(caps.supports_activities());
    assert!(caps.supports_sleep());
    assert!(caps.supports_recovery());
    assert!(caps.supports_health());
}
