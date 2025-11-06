// ABOUTME: Unit tests for OAuth routes module
// ABOUTME: Tests OAuth route handlers and endpoint functionality
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Unit tests for OAuth routes module

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
    routes::{AuthRoutes, LoginRequest, RegisterRequest},
};
use std::sync::Arc;

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_email_validation() {
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
        http_port: 8081,
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
            ..Default::default()
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());

    // Valid emails
    let valid_emails = vec![
        "test@example.com",
        "user.name@domain.com",
        "user+tag@example.co.uk",
        "123@numbers.com",
    ];

    for email in valid_emails {
        let request = RegisterRequest {
            email: email.to_owned(),
            password: "password123".to_owned(),
            display_name: None,
        };

        // Should not fail on email validation
        let result = auth_routes.register(request).await;
        // May fail on duplicate email, but not on validation
        if result.is_err() {
            let err = result.unwrap_err().to_string();
            assert!(
                !err.contains("Invalid email format"),
                "Email {email} should be valid"
            );
        }
    }

    // Invalid emails
    let invalid_emails = vec![
        "@domain.com",
        "user@",
        "nodomain",
        "missing@dotcom",
        "",
        "a@b",
    ];

    for email in invalid_emails {
        let request = RegisterRequest {
            email: email.to_owned(),
            password: "password123".to_owned(),
            display_name: None,
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Email {email} should be invalid");
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid email format"));
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_password_validation() {
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
        http_port: 8081,
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
            ..Default::default()
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());

    // Test short password
    let request = RegisterRequest {
        email: "test@example.com".to_owned(),
        password: "short".to_owned(),
        display_name: None,
    };

    let result = auth_routes.register(request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Password must be at least 8 characters"));

    // Test valid password
    let request = RegisterRequest {
        email: "test2@example.com".to_owned(),
        password: "validpassword123".to_owned(),
        display_name: None,
    };

    let result = auth_routes.register(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_duplicate_user_registration() {
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
        http_port: 8081,
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
            ..Default::default()
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());

    let request = RegisterRequest {
        email: "duplicate@example.com".to_owned(),
        password: "password123".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    // First registration should succeed
    let result1 = auth_routes.register(request.clone()).await;
    assert!(result1.is_ok());

    // Second registration with same email should fail
    let result2 = auth_routes.register(request).await;
    assert!(result2.is_err());
    assert!(result2.unwrap_err().to_string().contains("already exists"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_login_with_correct_credentials() {
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
        http_port: 8081,
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
            ..Default::default()
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());

    // Register user
    let register_request = RegisterRequest {
        email: "login_test@example.com".to_owned(),
        password: "password123".to_owned(),
        display_name: Some("Login Test".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await.unwrap();

    // Create admin user and approve the registered user for testing
    let user_id = uuid::Uuid::parse_str(&register_response.user_id).unwrap();
    let admin_id = uuid::Uuid::new_v4();
    let admin_user = pierre_mcp_server::models::User {
        id: admin_id,
        email: "admin@test.com".to_owned(),
        display_name: Some("Test Admin".to_owned()),
        password_hash: "$2b$10$hashedpassword".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Enterprise,
        tenant_id: Some("test-tenant".to_owned()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    server_resources
        .database
        .create_user(&admin_user)
        .await
        .unwrap();

    server_resources
        .database
        .update_user_status(
            user_id,
            pierre_mcp_server::models::UserStatus::Active,
            &admin_id.to_string(),
        )
        .await
        .unwrap();

    // Login with correct credentials
    let login_request = LoginRequest {
        email: "login_test@example.com".to_owned(),
        password: "password123".to_owned(),
    };

    let result = auth_routes.login(login_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(!response.jwt_token.is_empty());
    assert_eq!(response.user.email, "login_test@example.com");
    assert_eq!(response.user.display_name, Some("Login Test".to_owned()));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_login_with_wrong_password() {
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
        http_port: 8081,
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
            ..Default::default()
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());

    // Register user
    let register_request = RegisterRequest {
        email: "wrong_pass@example.com".to_owned(),
        password: "correctpassword".to_owned(),
        display_name: None,
    };

    auth_routes.register(register_request).await.unwrap();

    // Login with wrong password
    let login_request = LoginRequest {
        email: "wrong_pass@example.com".to_owned(),
        password: "wrongpassword".to_owned(),
    };

    let result = auth_routes.login(login_request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid credentials provided"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex OAuth integration test with full setup
async fn test_login_with_non_existent_user() {
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
        http_port: 8081,
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
            ..Default::default()
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
    let auth_routes = AuthRoutes::new(server_context.auth().clone(), server_context.data().clone());

    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_owned(),
        password: "password123".to_owned(),
    };

    let result = auth_routes.login(login_request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email or password"));
}
