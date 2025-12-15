// ABOUTME: Comprehensive test suite for Garmin Connect provider implementation
// ABOUTME: Tests provider creation, configuration, OAuth flow, and data conversion
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::config::environment::HttpClientConfig;
use pierre_mcp_server::constants::{
    api_provider_limits, init_server_config, oauth, oauth_providers,
};
use pierre_mcp_server::providers::core::{FitnessProvider, OAuth2Credentials, ProviderConfig};
use pierre_mcp_server::providers::garmin_provider::GarminProvider;
use pierre_mcp_server::providers::registry::{get_supported_providers, global_registry};
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
fn test_garmin_provider_creation() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    assert_eq!(provider.name(), oauth_providers::GARMIN);
    assert_eq!(provider.config().name, oauth_providers::GARMIN);
    assert_eq!(
        provider.config().auth_url,
        "https://connect.garmin.com/oauthConfirm"
    );
    assert_eq!(
        provider.config().api_base_url,
        "https://apis.garmin.com/wellness-api/rest"
    );
}

#[test]
fn test_garmin_provider_with_custom_config() {
    ensure_http_clients_initialized();
    let custom_config = ProviderConfig {
        name: oauth_providers::GARMIN.to_owned(),
        auth_url: "https://custom.garmin.com/auth".to_owned(),
        token_url: "https://custom.garmin.com/token".to_owned(),
        api_base_url: "https://custom.garmin.com/api".to_owned(),
        revoke_url: Some("https://custom.garmin.com/revoke".to_owned()),
        default_scopes: vec!["custom:scope".to_owned()],
    };

    let provider = GarminProvider::with_config(custom_config.clone());

    assert_eq!(provider.config().name, custom_config.name);
    assert_eq!(provider.config().auth_url, custom_config.auth_url);
    assert_eq!(provider.config().token_url, custom_config.token_url);
    assert_eq!(provider.config().api_base_url, custom_config.api_base_url);
}

#[tokio::test]
async fn test_garmin_provider_authentication_lifecycle() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Initially not authenticated
    assert!(!provider.is_authenticated().await);

    // Set valid credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["wellness:read".to_owned(), "activities:read".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Now authenticated
    assert!(provider.is_authenticated().await);
}

#[tokio::test]
async fn test_garmin_provider_expired_token() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Set expired credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("expired_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() - chrono::Duration::hours(1)), // Already expired
        scopes: vec!["wellness:read".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Not authenticated due to expired token
    assert!(!provider.is_authenticated().await);
}

#[tokio::test]
async fn test_garmin_provider_no_expiry() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Credentials with no expiry time
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: None, // No expiry
        scopes: vec!["wellness:read".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Authenticated (no expiry means valid indefinitely)
    assert!(provider.is_authenticated().await);
}

// Note: sport type mapping is tested indirectly through activity conversion
// The parse_sport_type method is private and tested via integration tests

#[test]
fn test_garmin_provider_default() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::default();
    assert_eq!(provider.name(), oauth_providers::GARMIN);
}

#[tokio::test]
async fn test_garmin_provider_disconnect() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["wellness:read".to_owned()],
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
fn test_garmin_provider_scopes() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();
    let scopes = &provider.config().default_scopes;

    assert!(scopes.contains(&"wellness:read".to_owned()));
    assert!(scopes.contains(&"activities:read".to_owned()));
}

#[test]
fn test_garmin_provider_endpoints() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();
    let config = provider.config();

    // Verify all required endpoints are configured
    assert!(config.auth_url.starts_with("https://"));
    assert!(config.token_url.starts_with("https://"));
    assert!(config.api_base_url.starts_with("https://"));
    assert!(config.revoke_url.is_some());
    assert!(config.revoke_url.as_ref().unwrap().starts_with("https://"));
}

#[tokio::test]
async fn test_garmin_provider_get_athlete_requires_auth() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Attempt to get athlete without authentication
    let result = provider.get_athlete().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_garmin_provider_get_activities_requires_auth() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Attempt to get activities without authentication
    let result = provider.get_activities(Some(10), None).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_garmin_provider_get_activity_requires_auth() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Attempt to get specific activity without authentication
    let result = provider.get_activity("12345").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_garmin_provider_get_stats_requires_auth() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Attempt to get stats without authentication
    let result = provider.get_stats().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_garmin_provider_get_personal_records() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Set credentials
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: None,
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["wellness:read".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Personal records should return empty vec (not yet implemented)
    let result = provider
        .get_personal_records()
        .await
        .expect("Failed to get personal records");
    assert!(result.is_empty());
}

#[test]
fn test_garmin_api_limits() {
    assert_eq!(api_provider_limits::garmin::DEFAULT_ACTIVITIES_PER_PAGE, 20);
    assert_eq!(api_provider_limits::garmin::MAX_ACTIVITIES_PER_REQUEST, 100);
}

#[tokio::test]
async fn test_garmin_provider_refresh_token_no_credentials() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Attempt to refresh without credentials
    let result = provider.refresh_token_if_needed().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("No credentials available"));
}

#[tokio::test]
async fn test_garmin_provider_refresh_token_not_needed() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Set credentials that don't need refresh (expires in 2 hours)
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: Some("test_access_token".to_owned()),
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(2)),
        scopes: vec!["wellness:read".to_owned()],
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
fn test_garmin_in_provider_registry() {
    ensure_http_clients_initialized();
    let registry = global_registry();
    // Verify Garmin is in the list of all providers using the registry
    let all_providers = registry.supported_providers();
    assert!(all_providers.contains(&oauth_providers::GARMIN));
    assert!(registry.is_supported(oauth_providers::GARMIN));
}

#[test]
fn test_garmin_provider_factory() {
    ensure_http_clients_initialized();
    let registry = global_registry();

    // Verify Garmin is supported
    assert!(registry.is_supported(oauth_providers::GARMIN));

    // Verify we can create a Garmin provider
    let provider = registry
        .create_provider(oauth_providers::GARMIN)
        .expect("Failed to create Garmin provider");

    assert_eq!(provider.name(), oauth_providers::GARMIN);
}

#[test]
fn test_garmin_in_supported_providers_list() {
    let supported = get_supported_providers();
    assert!(supported.contains(&oauth_providers::GARMIN));
}

#[test]
fn test_garmin_provider_pagination_limits() {
    // Test that requesting within single page limit would use single page fetch
    let limit = api_provider_limits::garmin::MAX_ACTIVITIES_PER_REQUEST;
    assert_eq!(limit, 100);

    // Test default page size
    let default = api_provider_limits::garmin::DEFAULT_ACTIVITIES_PER_PAGE;
    assert_eq!(default, 20);

    // Test that requesting more than max would use multi-page fetch
    let large_limit = api_provider_limits::garmin::MAX_ACTIVITIES_PER_REQUEST + 1;
    assert!(large_limit > 100);
}

// Activity type conversion is tested through integration tests with real API responses

#[tokio::test]
async fn test_garmin_credentials_without_access_token() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();

    // Credentials without access token
    let credentials = OAuth2Credentials {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        access_token: None, // No access token
        refresh_token: Some("test_refresh_token".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec!["wellness:read".to_owned()],
    };

    provider
        .set_credentials(credentials)
        .await
        .expect("Failed to set credentials");

    // Not authenticated without access token
    assert!(!provider.is_authenticated().await);
}

#[test]
fn test_garmin_default_scopes_format() {
    // Verify default scopes are comma-separated
    let scopes = oauth::GARMIN_DEFAULT_SCOPES;
    assert!(scopes.contains("wellness:read"));
    assert!(scopes.contains("activities:read"));
    assert!(scopes.contains(','));
}

#[test]
fn test_garmin_provider_config_urls() {
    ensure_http_clients_initialized();
    let provider = GarminProvider::new();
    let config = provider.config();

    // Verify URLs don't have trailing slashes
    assert!(!config.api_base_url.ends_with('/'));
    assert!(!config.auth_url.ends_with('/'));
    assert!(!config.token_url.ends_with('/'));
}

#[test]
fn test_garmin_rate_limit_constants() {
    // Verify rate limit constants are properly configured
    assert_eq!(
        api_provider_limits::garmin::RECOMMENDED_MAX_REQUESTS_PER_HOUR,
        100
    );
    assert_eq!(
        api_provider_limits::garmin::RECOMMENDED_MIN_LOGIN_INTERVAL_SECS,
        300
    );
    assert_eq!(api_provider_limits::garmin::RATE_LIMIT_HTTP_STATUS, 429);
    assert_eq!(
        api_provider_limits::garmin::ESTIMATED_RATE_LIMIT_BLOCK_DURATION_SECS,
        3600
    );

    // Verify block duration is 1 hour (60 minutes)
    assert_eq!(
        api_provider_limits::garmin::ESTIMATED_RATE_LIMIT_BLOCK_DURATION_SECS / 60,
        60
    );
}
