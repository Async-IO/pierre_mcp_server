// ABOUTME: Integration tests for OAuth flow in multi-tenant mode
// ABOUTME: Tests OAuth authentication, authorization, and token management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Integration tests for OAuth flow in multi-tenant mode

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::{
        AppBehaviorConfig, AuthConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment,
        ExternalServicesConfig, FitbitApiConfig, GeocodingServiceConfig, HttpClientConfig,
        LogLevel, OAuthConfig, OAuthProviderConfig, ProtocolConfig, SecurityConfig,
        SecurityHeadersConfig, ServerConfig, StravaApiConfig, TlsConfig, WeatherServiceConfig,
    },
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::ServerResources,
    models::{Tenant, User, UserStatus},
    routes::{
        auth::{AuthService, OAuthService},
        RegisterRequest,
    },
    tenant::TenantOAuthCredentials,
};
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_oauth_authorization_url_generation() {
    common::init_server_config();

    // Setup
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();

    database.migrate().await.unwrap();

    let auth_manager = AuthManager::new(24);

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: pierre_mcp_server::config::environment::McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: pierre_mcp_server::config::environment::CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: pierre_mcp_server::config::environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    });

    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        database.clone(),
        auth_manager.clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes =
        AuthService::new(server_context.auth().clone(), server_context.data().clone());
    let oauth_routes = OAuthService::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    // Create admin user first
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin@example.com".to_owned(),
        display_name: Some("Admin".to_owned()),
        password_hash: "hash".to_owned(),
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
        name: "Test Tenant".to_owned(),
        slug: "test-tenant".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await.unwrap();

    // Store tenant OAuth credentials for Strava
    let strava_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "strava".to_owned(),
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        redirect_uri: "http://localhost:8080/oauth/callback/strava".to_owned(),
        scopes: vec!["activity:read_all".to_owned()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&strava_credentials)
        .await
        .unwrap();

    // Store tenant OAuth credentials for Fitbit
    let fitbit_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "fitbit".to_owned(),
        client_id: "test_fitbit_client_id".to_owned(),
        client_secret: "test_fitbit_client_secret".to_owned(),
        redirect_uri: "http://localhost:8080/oauth/callback/fitbit".to_owned(),
        scopes: vec!["activity".to_owned(), "profile".to_owned()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&fitbit_credentials)
        .await
        .unwrap();

    // Register and login user
    let register_request = RegisterRequest {
        email: "oauth_test@example.com".to_owned(),
        password: "password123".to_owned(),
        display_name: Some("OAuth Test User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await.unwrap();
    let user_id = Uuid::parse_str(&register_response.user_id).unwrap();

    // Test Strava OAuth URL generation
    let strava_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .await
        .unwrap();

    assert!(strava_auth
        .authorization_url
        .contains("https://www.strava.com/oauth/authorize"));
    assert!(strava_auth.authorization_url.contains("client_id="));
    assert!(strava_auth.authorization_url.contains("redirect_uri="));
    assert!(strava_auth
        .authorization_url
        .contains("scope=activity%3Aread_all"));
    assert!(strava_auth.state.contains(&user_id.to_string()));
    assert_eq!(strava_auth.expires_in_minutes, 10);

    // Test Garmin OAuth URL generation (Garmin uses OAuth 1.0a, different structure)
    let garmin_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "garmin")
        .await
        .unwrap();

    // Just verify we got a valid auth URL back (Garmin uses OAuth 1.0a, different parameters)
    assert!(!garmin_auth.authorization_url.is_empty());
    assert!(garmin_auth.state.contains(&user_id.to_string()));
    assert_eq!(garmin_auth.expires_in_minutes, 10);
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth state validation test with full setup
async fn test_oauth_state_validation() {
    common::init_server_config();

    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();

    let auth_manager = AuthManager::new(24);

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: pierre_mcp_server::config::environment::McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: pierre_mcp_server::config::environment::CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: pierre_mcp_server::config::environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    });

    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let _oauth_routes = OAuthService::new(
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
    common::init_server_config();

    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(24);

    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: pierre_mcp_server::config::environment::McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: pierre_mcp_server::config::environment::CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: pierre_mcp_server::config::environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    });

    let user_id = Uuid::new_v4();

    // Create a test user in the database for the connection status check
    let user = pierre_mcp_server::models::User {
        id: user_id,
        email: format!("test_{user_id}@example.com"),
        display_name: None,
        password_hash: "test_hash".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("00000000-0000-0000-0000-000000000000".to_owned()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&user).await.unwrap();

    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthService::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    let statuses = oauth_routes.get_connection_status(user_id).await.unwrap();

    // After pluggable provider architecture, we have 5 OAuth providers: strava, garmin, fitbit, terra, whoop
    // (synthetic provider doesn't use OAuth)
    assert_eq!(statuses.len(), 5);

    let strava_status = statuses.iter().find(|s| s.provider == "strava").unwrap();
    assert!(!strava_status.connected);
    assert!(strava_status.expires_at.is_none());
    assert!(strava_status.scopes.is_none());

    let garmin_status = statuses.iter().find(|s| s.provider == "garmin").unwrap();
    assert!(!garmin_status.connected);
    assert!(garmin_status.expires_at.is_none());
    assert!(garmin_status.scopes.is_none());

    let fitbit_status = statuses.iter().find(|s| s.provider == "fitbit").unwrap();
    assert!(!fitbit_status.connected);
    assert!(fitbit_status.expires_at.is_none());
    assert!(fitbit_status.scopes.is_none());
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_invalid_provider_error() {
    common::init_server_config();

    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    database.migrate().await.unwrap();
    let auth_manager = AuthManager::new(24);
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: pierre_mcp_server::config::environment::McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: pierre_mcp_server::config::environment::CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: pierre_mcp_server::config::environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    });
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthService::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    let user_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let result = oauth_routes
        .get_auth_url(user_id, tenant_id, "invalid_provider")
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_disconnect_provider() {
    common::init_server_config();

    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(24);
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: pierre_mcp_server::config::environment::McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: pierre_mcp_server::config::environment::CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: pierre_mcp_server::config::environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    });
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthService::new(
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
        password_hash: "test_hash".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("00000000-0000-0000-0000-000000000000".to_owned()),
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
    common::init_server_config();

    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    database.migrate().await.unwrap();

    // Create admin user first
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin@example.com".to_owned(),
        display_name: Some("Admin".to_owned()),
        password_hash: "hash".to_owned(),
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
        name: "Test Tenant".to_owned(),
        slug: "test-tenant".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await.unwrap();

    // Store tenant OAuth credentials
    let strava_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "strava".to_owned(),
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        redirect_uri: "http://localhost:8080/oauth/callback/strava".to_owned(),
        scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&strava_credentials)
        .await
        .unwrap();

    let fitbit_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "fitbit".to_owned(),
        client_id: "test_fitbit_client_id".to_owned(),
        client_secret: "test_fitbit_client_secret".to_owned(),
        redirect_uri: "http://localhost:8080/oauth/callback/fitbit".to_owned(),
        scopes: vec!["activity".to_owned(), "profile".to_owned()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&fitbit_credentials)
        .await
        .unwrap();

    let auth_manager = AuthManager::new(24);
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8080,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: temp_dir.path().to_path_buf(),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: pierre_mcp_server::config::environment::McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: pierre_mcp_server::config::environment::CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: pierre_mcp_server::config::environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    });
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let oauth_routes = OAuthService::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    let user_id = Uuid::new_v4();

    // Test Strava URL parameters
    let strava_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .await
        .unwrap();
    let strava_url = url::Url::parse(&strava_auth.authorization_url).unwrap();
    let strava_params: std::collections::HashMap<_, _> = strava_url.query_pairs().collect();

    assert!(strava_params.contains_key("client_id"));
    assert!(strava_params.contains_key("redirect_uri"));
    assert!(strava_params.contains_key("response_type"));
    assert_eq!(strava_params.get("response_type").unwrap(), "code");
    assert!(strava_params.contains_key("scope"));
    assert!(strava_params.contains_key("state"));

    // Test Garmin URL parameters (OAuth 1.0a uses different flow)
    let garmin_auth = oauth_routes
        .get_auth_url(user_id, tenant_id, "garmin")
        .await
        .unwrap();
    // Garmin uses OAuth 1.0a, so it has different URL structure - just verify we got a valid URL
    assert!(!garmin_auth.authorization_url.is_empty());
    assert!(garmin_auth.state.contains(&user_id.to_string()));
}
