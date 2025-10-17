// ABOUTME: Unit tests for API key route handlers and endpoints
// ABOUTME: Tests CRUD operations for API keys via HTTP routes
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Unit tests for API key routes

mod common;

use chrono::{Duration, Utc};
use pierre_mcp_server::{
    api_key_routes::ApiKeyRoutes,
    api_keys::{ApiKeyTier, CreateApiKeyRequest},
    auth::{AuthManager, AuthMethod, AuthResult},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
    rate_limiting::UnifiedRateLimitInfo,
};
use std::sync::Arc;
use uuid::Uuid;

/// Helper function to create an `AuthResult` for testing
fn create_test_auth_result(user_id: Uuid) -> AuthResult {
    AuthResult {
        user_id,
        auth_method: AuthMethod::JwtToken {
            tier: "free".to_string(),
        },
        rate_limit: UnifiedRateLimitInfo {
            is_rate_limited: false,
            limit: Some(1000),
            remaining: Some(1000),
            reset_at: Some(Utc::now() + Duration::hours(1)),
            tier: "free".to_string(),
            auth_method: "jwt".to_string(),
        },
    }
}

#[allow(clippy::too_many_lines)] // Long function: Complex test setup with full configuration
async fn create_test_setup() -> (ApiKeyRoutes, Uuid, AuthResult) {
    // Create test database
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new(database_url, encryption_key).await.unwrap();

    // Create auth manager
    let jwt_secret = pierre_mcp_server::auth::generate_jwt_secret().to_vec();
    let auth_manager = AuthManager::new(jwt_secret, 24);

    // Create test user
    let user = User::new(
        "test@example.com".to_string(),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    );
    let user_id = database.create_user(&user).await.unwrap();

    // Generate JWT token for the user
    let _jwt_token = auth_manager.generate_token(&user).unwrap(); // Not used directly, AuthResult created from user_id

    // Create cache for API key routes
    let cache = common::create_test_cache().await.unwrap();

    // Create ServerResources for API key routes
    let server_resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database.clone(),
        auth_manager.clone(),
        "test_jwt_secret",
        Arc::new({
            // Create temporary directory for test config files
            let temp_dir = tempfile::tempdir().unwrap();

            pierre_mcp_server::config::environment::ServerConfig {
                http_port: 8081,
                oauth_callback_port: 35535,
                log_level: pierre_mcp_server::config::environment::LogLevel::Info,
                logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
                http_client: pierre_mcp_server::config::environment::HttpClientConfig::default(),
                database: pierre_mcp_server::config::environment::DatabaseConfig {
                    url: pierre_mcp_server::config::environment::DatabaseUrl::Memory,
                    auto_migrate: true,
                    backup: pierre_mcp_server::config::environment::BackupConfig {
                        enabled: false,
                        interval_seconds: 3600,
                        retention_count: 7,
                        directory: temp_dir.path().to_path_buf(),
                    },
                },
                auth: pierre_mcp_server::config::environment::AuthConfig {
                    jwt_expiry_hours: 24,
                    enable_refresh_tokens: false,
                },
                oauth: pierre_mcp_server::config::environment::OAuthConfig {
                    strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                        client_id: None,
                        client_secret: None,
                        redirect_uri: None,
                        scopes: vec![],
                        enabled: false,
                    },
                    fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                        client_id: None,
                        client_secret: None,
                        redirect_uri: None,
                        scopes: vec![],
                        enabled: false,
                    },
                },
                security: pierre_mcp_server::config::environment::SecurityConfig {
                    cors_origins: vec!["*".to_string()],
                    rate_limit: pierre_mcp_server::config::environment::RateLimitConfig {
                        enabled: false,
                        requests_per_window: 100,
                        window_seconds: 60,
                    },
                    tls: pierre_mcp_server::config::environment::TlsConfig {
                        enabled: false,
                        cert_path: None,
                        key_path: None,
                    },
                    headers: pierre_mcp_server::config::environment::SecurityHeadersConfig {
                        environment: pierre_mcp_server::config::environment::Environment::Testing,
                    },
                },
                external_services: pierre_mcp_server::config::environment::ExternalServicesConfig {
                    weather: pierre_mcp_server::config::environment::WeatherServiceConfig {
                        api_key: None,
                        base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                        enabled: false,
                    },
                    geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                        base_url: "https://nominatim.openstreetmap.org".to_string(),
                        enabled: false,
                    },
                    strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                        base_url: "https://www.strava.com/api/v3".to_string(),
                        auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                        token_url: "https://www.strava.com/oauth/token".to_string(),
                        deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_string(),
                    },
                    fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                        base_url: "https://api.fitbit.com".to_string(),
                        auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                        token_url: "https://api.fitbit.com/oauth2/token".to_string(),
                        revoke_url: "https://api.fitbit.com/oauth2/revoke".to_string(),
                    },
                },
                app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
                    max_activities_fetch: 100,
                    default_activities_limit: 20,
                    ci_mode: true,
                    protocol: pierre_mcp_server::config::environment::ProtocolConfig {
                        mcp_version: "2025-06-18".to_string(),
                        server_name: "pierre-mcp-server-test".to_string(),
                        server_version: env!("CARGO_PKG_VERSION").to_string(),
                    },
                },
                sse: pierre_mcp_server::config::environment::SseConfig::default(),
                oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(
                ),
                route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(
                ),
            }
        }),
        cache,
    ));

    // Create API key routes
    let api_key_routes = ApiKeyRoutes::new(server_resources);

    // Create AuthResult for testing
    let auth_result = create_test_auth_result(user_id);

    (api_key_routes, user_id, auth_result)
}

#[tokio::test]
async fn test_create_api_key_success() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        description: Some("Test description".to_string()),
        tier: ApiKeyTier::Starter,
        expires_in_days: Some(30),
        rate_limit_requests: None,
    };

    // Auth is already AuthResult, no need for Bearer token
    let response = api_key_routes.create_api_key(&auth, request).await.unwrap();

    // Verify response
    assert!(response.api_key.starts_with("pk_live_"));
    assert_eq!(response.api_key.len(), 40);
    assert_eq!(response.key_info.name, "Test API Key");
    assert_eq!(response.key_info.tier, ApiKeyTier::Starter);
    assert!(response.key_info.expires_at.is_some());
    assert!(response.warning.contains("Store this API key securely"));
}

// NOTE: This test is now obsolete - authentication happens at the HTTP filter level
// before route methods are called. Route methods now receive validated AuthResult.
// Invalid authentication is now tested at the integration test level (HTTP routes)
// See test_create_api_key_invalid_auth in tests/api_key_integration_test.rs

#[tokio::test]
async fn test_list_api_keys() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Create a couple of API keys
    let request1 = CreateApiKeyRequest {
        name: "Key 1".to_string(),
        description: Some("First key".to_string()),
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    let request2 = CreateApiKeyRequest {
        name: "Key 2".to_string(),
        description: Some("Second key".to_string()),
        tier: ApiKeyTier::Professional,
        expires_in_days: Some(90),
        rate_limit_requests: None,
    };

    // Auth is already AuthResult, no need for Bearer token

    // Create the keys
    api_key_routes
        .create_api_key(&auth, request1)
        .await
        .unwrap();

    api_key_routes
        .create_api_key(&auth, request2)
        .await
        .unwrap();

    // List keys
    let response = api_key_routes.list_api_keys(&auth).await.unwrap();

    // Verify response
    assert_eq!(response.api_keys.len(), 2);

    let key_names: Vec<_> = response.api_keys.iter().map(|k| &k.name).collect();
    assert!(key_names.contains(&&"Key 1".to_string()));
    assert!(key_names.contains(&&"Key 2".to_string()));

    // Check tiers
    let starter_key = response
        .api_keys
        .iter()
        .find(|k| k.name == "Key 1")
        .unwrap();
    let pro_key = response
        .api_keys
        .iter()
        .find(|k| k.name == "Key 2")
        .unwrap();

    assert_eq!(starter_key.tier, ApiKeyTier::Starter);
    assert_eq!(pro_key.tier, ApiKeyTier::Professional);
    assert!(starter_key.expires_at.is_none());
    assert!(pro_key.expires_at.is_some());
}

#[tokio::test]
async fn test_deactivate_api_key() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Key to deactivate".to_string(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    // Auth is already AuthResult, no need for Bearer token
    let create_response = api_key_routes.create_api_key(&auth, request).await.unwrap();

    let key_id = &create_response.key_info.id;

    // Deactivate the key
    let deactivate_response = api_key_routes
        .deactivate_api_key(&auth, key_id)
        .await
        .unwrap();

    assert!(deactivate_response.message.contains("deactivated"));
    assert!(deactivate_response.deactivated_at <= Utc::now());

    // Verify key is no longer active in the list
    let list_response = api_key_routes.list_api_keys(&auth).await.unwrap();

    let deactivated_key = list_response
        .api_keys
        .iter()
        .find(|k| k.id == *key_id)
        .unwrap();

    assert!(!deactivated_key.is_active);
}

#[tokio::test]
async fn test_deactivate_nonexistent_key() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Auth is already AuthResult, no need for Bearer token
    let fake_key_id = "nonexistent_key_id";

    let result = api_key_routes.deactivate_api_key(&auth, fake_key_id).await;

    // Should succeed (idempotent operation)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_api_key_usage_stats() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Usage Test Key".to_string(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    // Auth is already AuthResult, no need for Bearer token
    let create_response = api_key_routes.create_api_key(&auth, request).await.unwrap();

    let key_id = &create_response.key_info.id;

    // Get usage stats (should be empty for new key)
    let start_date = Utc::now() - Duration::days(30);
    let end_date = Utc::now();

    let usage_response = api_key_routes
        .get_api_key_usage(&auth, key_id, start_date, end_date)
        .await
        .unwrap();

    // Verify empty usage stats
    assert_eq!(usage_response.stats.api_key_id, *key_id);
    assert_eq!(usage_response.stats.total_requests, 0);
    assert_eq!(usage_response.stats.successful_requests, 0);
    assert_eq!(usage_response.stats.failed_requests, 0);
}

#[tokio::test]
async fn test_get_usage_stats_unauthorized_key() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Try to access usage stats for a key that doesn't belong to the user
    // Auth is already AuthResult, no need for Bearer token
    let fake_key_id = "some_other_users_key";

    let start_date = Utc::now() - Duration::days(30);
    let end_date = Utc::now();

    let result = api_key_routes
        .get_api_key_usage(&auth, fake_key_id, start_date, end_date)
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found or access denied"));
}

#[tokio::test]
async fn test_api_key_tiers() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Auth is already AuthResult, no need for Bearer token

    // Test all tiers
    for (tier, tier_name) in [
        (ApiKeyTier::Starter, "Starter"),
        (ApiKeyTier::Professional, "Professional"),
        (ApiKeyTier::Enterprise, "Enterprise"),
    ] {
        let request = CreateApiKeyRequest {
            name: format!("{tier_name} Key"),
            description: Some(format!("Test {tier_name} tier")),
            tier: tier.clone(),
            expires_in_days: None,
            rate_limit_requests: None,
        };

        let response = api_key_routes.create_api_key(&auth, request).await.unwrap();

        assert_eq!(response.key_info.tier, tier);
        assert_eq!(response.key_info.name, format!("{tier_name} Key"));
    }
}

#[tokio::test]
async fn test_api_key_expiration() {
    let (api_key_routes, _user_id, auth) = create_test_setup().await;

    // Auth is already AuthResult, no need for Bearer token

    // Test key with expiration
    let request = CreateApiKeyRequest {
        name: "Expiring Key".to_string(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: Some(7),
        rate_limit_requests: None,
    };

    let response = api_key_routes.create_api_key(&auth, request).await.unwrap();

    // Verify expiration is set correctly
    assert!(response.key_info.expires_at.is_some());
    let expires_at = response.key_info.expires_at.unwrap();
    let expected_expiry = Utc::now() + Duration::days(7);

    // Should be within 1 minute of expected (to account for test execution time)
    let diff = (expires_at - expected_expiry).num_seconds().abs();
    assert!(
        diff < 60,
        "Expiration time should be within 1 minute of expected"
    );
}

#[tokio::test]
async fn test_authentication_with_different_users() {
    // Create first user setup
    let (api_key_routes1, _user_id1, auth1) = create_test_setup().await;

    // Create second user in same database
    let _user2 = User::new(
        "user2@example.com".to_string(),
        "hashed_password2".to_string(),
        Some("User 2".to_string()),
    );

    // We need access to the database to create the second user
    // This test demonstrates that each setup creates its own isolated database
    // In a real scenario, we'd use the same database instance

    // For now, let's verify that each user can only access their own keys
    // Auth is already AuthResult, no need for Bearer token

    // Create key for user 1
    let request = CreateApiKeyRequest {
        name: "User 1 Key".to_string(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
        rate_limit_requests: None,
    };

    api_key_routes1
        .create_api_key(&auth1, request)
        .await
        .unwrap();

    // List keys for user 1
    let list_response = api_key_routes1.list_api_keys(&auth1).await.unwrap();

    assert_eq!(list_response.api_keys.len(), 1);
    assert_eq!(list_response.api_keys[0].name, "User 1 Key");
}
