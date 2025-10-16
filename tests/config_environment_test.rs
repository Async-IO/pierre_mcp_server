// ABOUTME: Unit tests for config environment functionality
// ABOUTME: Validates config environment behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use pierre_mcp_server::config::environment::{
    AppBehaviorConfig, AuthConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment,
    ExternalServicesConfig, FitbitApiConfig, GeocodingServiceConfig, LogLevel, OAuthConfig,
    OAuthProviderConfig, ProtocolConfig, RateLimitConfig, SecurityConfig, SecurityHeadersConfig,
    ServerConfig, StravaApiConfig, TlsConfig, WeatherServiceConfig,
};

// Tests for public configuration types

#[test]
fn test_log_level_parsing() {
    assert_eq!(LogLevel::from_str_or_default("error"), LogLevel::Error);
    assert_eq!(LogLevel::from_str_or_default("WARN"), LogLevel::Warn);
    assert_eq!(LogLevel::from_str_or_default("info"), LogLevel::Info);
    assert_eq!(LogLevel::from_str_or_default("Debug"), LogLevel::Debug);
    assert_eq!(LogLevel::from_str_or_default("trace"), LogLevel::Trace);
    assert_eq!(LogLevel::from_str_or_default("invalid"), LogLevel::Info); // Default fallback
}

#[test]
fn test_environment_parsing() {
    assert_eq!(
        Environment::from_str_or_default("production"),
        Environment::Production
    );
    assert_eq!(
        Environment::from_str_or_default("PROD"),
        Environment::Production
    );
    assert_eq!(
        Environment::from_str_or_default("development"),
        Environment::Development
    );
    assert_eq!(
        Environment::from_str_or_default("dev"),
        Environment::Development
    );
    assert_eq!(
        Environment::from_str_or_default("testing"),
        Environment::Testing
    );
    assert_eq!(
        Environment::from_str_or_default("test"),
        Environment::Testing
    );
    assert_eq!(
        Environment::from_str_or_default("invalid"),
        Environment::Development
    ); // Default fallback
}

#[test]
fn test_database_url_parsing() {
    // SQLite URLs
    let sqlite_url = DatabaseUrl::parse_url("sqlite:./test.db").unwrap();
    assert!(sqlite_url.is_sqlite());
    assert!(!sqlite_url.is_postgresql());
    assert_eq!(sqlite_url.to_connection_string(), "sqlite:./test.db");

    // Memory database
    let memory_url = DatabaseUrl::parse_url("sqlite::memory:").unwrap();
    assert!(memory_url.is_memory());
    assert!(memory_url.is_sqlite());

    // PostgreSQL URLs
    let pg_url = DatabaseUrl::parse_url("postgresql://user:pass@localhost/db").unwrap();
    assert!(pg_url.is_postgresql());
    assert!(!pg_url.is_sqlite());

    // Fallback to SQLite
    let fallback_url = DatabaseUrl::parse_url("./some/path.db").unwrap();
    assert!(fallback_url.is_sqlite());
}

#[test]
fn test_config_validation() {
    // Test valid configuration with single-port architecture
    let config = ServerConfig {
        http_port: 3000, // Single unified port for all protocols
        oauth_callback_port: 35535,
        log_level: LogLevel::default(),
        http_client: pierre_mcp_server::config::environment::HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::SQLite {
                path: "./test.db".into(),
            },
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: "./backups".into(),
            },
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_id".into()),
                client_secret: Some("test_secret".into()),
                redirect_uri: Some("http://localhost/callback".into()),
                scopes: vec!["read".into()],
                enabled: true,
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
            cors_origins: vec!["*".into()],
            rate_limit: RateLimitConfig {
                enabled: true,
                requests_per_window: 60,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Development,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".into(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".into(),
                enabled: true,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".into(),
                auth_url: "https://www.strava.com/oauth/authorize".into(),
                token_url: "https://www.strava.com/oauth/token".into(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".into(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".into(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".into(),
                token_url: "https://api.fitbit.com/oauth2/token".into(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".into(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: false,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".into(),
                server_name: "pierre-mcp-server".into(),
                server_version: "test".into(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
    };

    // With single-port architecture, validation should pass
    assert!(config.validate().is_ok());
}
