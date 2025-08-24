// Integration tests for routes.rs module
// Tests for authentication routes, OAuth routes, and A2A routes

use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::{
        AppBehaviorConfig, AuthConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment,
        ExternalServicesConfig, FitbitApiConfig, GeocodingServiceConfig, LogLevel, OAuthConfig,
        OAuthProviderConfig, ProtocolConfig, RateLimitConfig, SecurityConfig,
        SecurityHeadersConfig, ServerConfig, StravaApiConfig, TlsConfig, WeatherServiceConfig,
    },
    database_plugins::factory::Database,
    mcp::multitenant::ServerResources,
    routes::{AuthRoutes, RegisterRequest},
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_email_validation() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    tracing::trace!("Created test database: {:?}", std::ptr::addr_of!(database));
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);
    tracing::trace!(
        "Created test auth manager: {:?}",
        std::ptr::addr_of!(auth_manager)
    );
    // Email and password validation functions are now static, no need for routes instance
    assert!(AuthRoutes::is_valid_email("test@example.com"));
    assert!(AuthRoutes::is_valid_email("user.name+tag@domain.co.uk"));
    assert!(!AuthRoutes::is_valid_email("invalid-email"));
    assert!(!AuthRoutes::is_valid_email("@domain.com"));
    assert!(!AuthRoutes::is_valid_email("user@"));
}

#[tokio::test]
async fn test_password_validation() {
    // Password validation function is now static, no need for database setup
    assert!(AuthRoutes::is_valid_password("password123"));
    assert!(AuthRoutes::is_valid_password("verylongpassword"));
    assert!(!AuthRoutes::is_valid_password("short"));
    assert!(!AuthRoutes::is_valid_password("1234567"));
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_register_user() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    // Create ServerResources for auth routes
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        mcp_port: 8080,
        http_port: 8081,
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

    let routes = AuthRoutes::new(server_resources);

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
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.display();
    let database = Database::new(&format!("sqlite:{db_path_str}"), vec![0u8; 32])
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    // Create ServerResources for auth routes
    let temp_dir = tempfile::tempdir().unwrap();
    let config = Arc::new(ServerConfig {
        mcp_port: 8080,
        http_port: 8081,
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

    let routes = AuthRoutes::new(server_resources);

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
