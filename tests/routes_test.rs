// ABOUTME: Unit tests for routes functionality
// ABOUTME: Validates routes behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Integration tests for routes.rs module
// Tests for authentication routes, OAuth routes, and A2A routes

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::{
        AppBehaviorConfig, AuthConfig, BackupConfig, CacheConfig as EnvCacheConfig, CorsConfig,
        DatabaseConfig, DatabaseUrl, Environment, ExternalServicesConfig, FirebaseConfig,
        FitbitApiConfig, GeocodingServiceConfig, GoalManagementConfig, HttpClientConfig, LogLevel,
        LoggingConfig, McpConfig, MonitoringConfig, OAuth2ServerConfig, OAuthConfig,
        OAuthProviderConfig, PostgresPoolConfig, ProtocolConfig, RateLimitConfig,
        RouteTimeoutConfig, SecurityConfig, SecurityHeadersConfig, ServerConfig,
        SleepToolParamsConfig, SqlxConfig, SseConfig, StravaApiConfig, TlsConfig,
        TokioRuntimeConfig, TrainingZonesConfig, WeatherServiceConfig,
    },
    context::ServerContext,
    database_plugins::factory::Database,
    mcp::resources::ServerResources,
    routes::{auth::AuthService, RegisterRequest},
};
use std::{ptr, sync::Arc};
use tempfile::TempDir;

#[tokio::test]
async fn test_email_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &format!("sqlite:{db_path_str}"),
        vec![0u8; 32],
        &PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();

    tracing::trace!("Created test database: {:?}", ptr::addr_of!(database));
    let auth_manager = AuthManager::new(24);
    tracing::trace!(
        "Created test auth manager: {:?}",
        ptr::addr_of!(auth_manager)
    );
    // Email and password validation functions are now static, no need for routes instance
    assert!(AuthService::is_valid_email("test@example.com"));
    assert!(AuthService::is_valid_email("user.name+tag@domain.co.uk"));
    assert!(!AuthService::is_valid_email("invalid-email"));
    assert!(!AuthService::is_valid_email("@domain.com"));
    assert!(!AuthService::is_valid_email("user@"));
}

#[tokio::test]
async fn test_password_validation() {
    // Password validation function is now static, no need for database setup
    assert!(AuthService::is_valid_password("password123"));
    assert!(AuthService::is_valid_password("verylongpassword"));
    assert!(!AuthService::is_valid_password("short"));
    assert!(!AuthService::is_valid_password("1234567"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_register_user() {
    common::init_server_config();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &format!("sqlite:{db_path_str}"),
        vec![0u8; 32],
        &PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    let auth_manager = AuthManager::new(24);

    // Create ServerResources for auth routes
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8081,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: LoggingConfig::default(),
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
            postgres_pool: PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..AuthConfig::default()
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
                ..Default::default()
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: SseConfig::default(),
        oauth2_server: OAuth2ServerConfig::default(),
        route_timeouts: RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: EnvCacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_tool_params: SleepToolParamsConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(
        ServerResources::new(
            database.clone(),
            auth_manager.clone(),
            "test_jwt_secret",
            config,
            cache,
            2048, // Use 2048-bit RSA keys for faster test execution
            Some(common::get_shared_test_jwks()),
        )
        .await,
    );

    let server_context = ServerContext::from(server_resources.as_ref());
    let routes = AuthService::new(
        server_context.auth().clone(),
        server_context.config().clone(),
        server_context.data().clone(),
    );

    let request = RegisterRequest {
        email: "test@example.com".into(),
        password: "password123".into(),
        display_name: Some("Test User".into()),
    };

    let response = routes.register(request).await.unwrap();
    assert!(!response.user_id.is_empty());
    assert_eq!(
        response.message,
        "User registered successfully. Your account is pending admin approval."
    );
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_register_duplicate_user() {
    common::init_server_config();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &format!("sqlite:{db_path_str}"),
        vec![0u8; 32],
        &PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    let auth_manager = AuthManager::new(24);

    // Create ServerResources for auth routes
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        http_port: 8081,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: LoggingConfig::default(),
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
            postgres_pool: PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..AuthConfig::default()
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
                ..Default::default()
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
                ..Default::default()
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: SseConfig::default(),
        oauth2_server: OAuth2ServerConfig::default(),
        route_timeouts: RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: EnvCacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_tool_params: SleepToolParamsConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(
        ServerResources::new(
            database.clone(),
            auth_manager.clone(),
            "test_jwt_secret",
            config,
            cache,
            2048, // Use 2048-bit RSA keys for faster test execution
            Some(common::get_shared_test_jwks()),
        )
        .await,
    );

    let server_context = ServerContext::from(server_resources.as_ref());
    let routes = AuthService::new(
        server_context.auth().clone(),
        server_context.config().clone(),
        server_context.data().clone(),
    );

    let request = RegisterRequest {
        email: "test@example.com".into(),
        password: "password123".into(),
        display_name: Some("Test User".into()),
    };

    // First registration should succeed
    routes.register(request.clone()).await.unwrap();

    // Second registration should fail
    let result = routes.register(request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));
}
