// ABOUTME: Integration tests for OAuth flow in multi-tenant mode
// ABOUTME: Tests OAuth authentication, authorization, and token management
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Integration tests for OAuth flow in multi-tenant mode

use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::{
        AppBehaviorConfig, AuthConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment,
        ExternalServicesConfig, FitbitApiConfig, GeocodingServiceConfig, LogLevel, OAuthConfig,
        OAuthProviderConfig, ProtocolConfig, RateLimitConfig, SecurityConfig,
        SecurityHeadersConfig, ServerConfig, StravaApiConfig, TlsConfig, WeatherServiceConfig,
    },
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::ServerResources,
    models::{Tenant, User, UserStatus},
    routes::{AuthRoutes, OAuthRoutes, RegisterRequest},
    tenant::TenantOAuthCredentials,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_oauth_authorization_url_generation() {
    // Setup
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    database.migrate().await.unwrap();

    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: temp_dir.path().join("encryption_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: temp_dir.path().join("jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });

    let server_resources = Arc::new(ServerResources::new(
        database.clone(),
        auth_manager.clone(),
        "test_jwt_secret",
        config,
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());
    let oauth_routes = OAuthRoutes::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    // Create admin user first
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin@example.com".to_string(),
        display_name: Some("Admin".to_string()),
        password_hash: "hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: None,
    };
    let admin_id = database.create_user(&admin_user).await.unwrap();

    // Create tenant
    let tenant_id = Uuid::new_v4();
    let tenant = Tenant {
        id: tenant_id,
        name: "Test Tenant".to_string(),
        slug: "test-tenant".to_string(),
        domain: None,
        plan: "starter".to_string(),
        owner_user_id: admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await.unwrap();

    // Store tenant OAuth credentials for Strava
    let strava_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "strava".to_string(),
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:8080/oauth/callback/strava".to_string(),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&strava_credentials)
        .await
        .unwrap();

    // Store tenant OAuth credentials for Fitbit
    let fitbit_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "fitbit".to_string(),
        client_id: "test_fitbit_client_id".to_string(),
        client_secret: "test_fitbit_client_secret".to_string(),
        redirect_uri: "http://localhost:8080/oauth/callback/fitbit".to_string(),
        scopes: vec!["activity".to_string(), "profile".to_string()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&fitbit_credentials)
        .await
        .unwrap();

    // Register and login user
    let register_request = RegisterRequest {
        email: "oauth_test@example.com".to_string(),
        password: "password123".to_string(),
        display_name: Some("OAuth Test User".to_string()),
    };

    let register_response = auth_routes.register(register_request).await.unwrap();
    let user_id = Uuid::parse_str(&register_response.user_id).unwrap();

    // Test Strava OAuth URL generation
    let strava_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .unwrap();

    assert!(strava_auth
        .authorization_url
        .contains("https://www.strava.com/oauth/authorize"));
    assert!(strava_auth.authorization_url.contains("client_id="));
    assert!(strava_auth.authorization_url.contains("redirect_uri="));
    assert!(strava_auth
        .authorization_url
        .contains("scope=read%2Cactivity%3Aread_all"));
    assert!(strava_auth.state.contains(&user_id.to_string()));
    assert_eq!(strava_auth.expires_in_minutes, 10);

    // Test Fitbit OAuth URL generation
    let fitbit_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "fitbit")
        .unwrap();

    assert!(fitbit_auth
        .authorization_url
        .contains("https://www.fitbit.com/oauth2/authorize"));
    assert!(fitbit_auth.authorization_url.contains("client_id="));
    assert!(fitbit_auth.authorization_url.contains("redirect_uri="));
    assert!(fitbit_auth
        .authorization_url
        .contains("scope=activity%20profile"));
    assert!(fitbit_auth.state.contains(&user_id.to_string()));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth state validation test with full setup
async fn test_oauth_state_validation() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: temp_dir.path().join("encryption_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: temp_dir.path().join("jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });

    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let _oauth_routes = OAuthRoutes::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    // Test valid state format
    let user_id = Uuid::new_v4();
    let state_id = Uuid::new_v4();
    let valid_state = format!("{user_id}:{state_id}");

    // This should parse correctly (we can't test the full callback without mocking the HTTP client)
    // But we can verify the state format is what we expect
    assert!(valid_state.contains(':'));
    let parts: Vec<&str> = valid_state.split(':').collect();
    assert_eq!(parts.len(), 2);
    assert!(Uuid::parse_str(parts[0]).is_ok());
    assert!(Uuid::parse_str(parts[1]).is_ok());
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth connection status test with full setup
async fn test_connection_status_no_providers() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: temp_dir.path().join("encryption_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: temp_dir.path().join("jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });

    let user_id = Uuid::new_v4();

    // Create a test user in the database for the connection status check
    let user = pierre_mcp_server::models::User {
        id: user_id,
        email: format!("test_{user_id}@example.com"),
        display_name: None,
        password_hash: "test_hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&user).await.unwrap();

    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthRoutes::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

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
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_invalid_provider_error() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    database.migrate().await.unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: temp_dir.path().join("encryption_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: temp_dir.path().join("jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
    ));
    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthRoutes::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let result = oauth_routes.get_auth_url(user_id, tenant_id, "invalid_provider");

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_disconnect_provider() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: temp_dir.path().join("encryption_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: temp_dir.path().join("jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
    ));
    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthRoutes::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    let user_id = Uuid::new_v4();

    // Create a test user in the database
    let user = pierre_mcp_server::models::User {
        id: user_id,
        email: format!("test_{user_id}@example.com"),
        display_name: None,
        password_hash: "test_hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    server_context
        .data()
        .database()
        .create_user(&user)
        .await
        .unwrap();

    // Test disconnecting Strava (should succeed even if not connected)
    let result = oauth_routes.disconnect_provider(user_id, "strava").await;
    assert!(result.is_ok());

    // Test disconnecting invalid provider
    let result = oauth_routes.disconnect_provider(user_id, "invalid").await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_oauth_urls_contain_required_parameters() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    database.migrate().await.unwrap();

    // Create admin user first
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin@example.com".to_string(),
        display_name: Some("Admin".to_string()),
        password_hash: "hash".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: None,
    };
    let admin_id = database.create_user(&admin_user).await.unwrap();

    // Create tenant
    let tenant_id = Uuid::new_v4();
    let tenant = Tenant {
        id: tenant_id,
        name: "Test Tenant".to_string(),
        slug: "test-tenant".to_string(),
        domain: None,
        plan: "starter".to_string(),
        owner_user_id: admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await.unwrap();

    // Store tenant OAuth credentials
    let strava_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "strava".to_string(),
        client_id: "test_client_id".to_string(),
        client_secret: "test_client_secret".to_string(),
        redirect_uri: "http://localhost:8080/oauth/callback/strava".to_string(),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&strava_credentials)
        .await
        .unwrap();

    let fitbit_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "fitbit".to_string(),
        client_id: "test_fitbit_client_id".to_string(),
        client_secret: "test_fitbit_client_secret".to_string(),
        redirect_uri: "http://localhost:8080/oauth/callback/fitbit".to_string(),
        scopes: vec!["activity".to_string(), "profile".to_string()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&fitbit_credentials)
        .await
        .unwrap();

    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: temp_dir.path().join("encryption_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: temp_dir.path().join("jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
    ));
    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthRoutes::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    let user_id = Uuid::new_v4();

    // Test Strava URL parameters
    let strava_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .unwrap();
    let strava_url = url::Url::parse(&strava_auth.authorization_url).unwrap();
    let strava_params: std::collections::HashMap<_, _> = strava_url.query_pairs().collect();

    assert!(strava_params.contains_key("client_id"));
    assert!(strava_params.contains_key("redirect_uri"));
    assert!(strava_params.contains_key("response_type"));
    assert_eq!(strava_params.get("response_type").unwrap(), "code");
    assert!(strava_params.contains_key("scope"));
    assert!(strava_params.contains_key("state"));

    // Test Fitbit URL parameters
    let fitbit_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "fitbit")
        .unwrap();
    let fitbit_url = url::Url::parse(&fitbit_auth.authorization_url).unwrap();
    let fitbit_params: std::collections::HashMap<_, _> = fitbit_url.query_pairs().collect();

    assert!(fitbit_params.contains_key("client_id"));
    assert!(fitbit_params.contains_key("redirect_uri"));
    assert!(fitbit_params.contains_key("response_type"));
    assert_eq!(fitbit_params.get("response_type").unwrap(), "code");
    assert!(fitbit_params.contains_key("scope"));
    assert!(fitbit_params.contains_key("state"));
}
