// ABOUTME: Comprehensive tests for authentication and OAuth route flows
// ABOUTME: Tests authentication, registration, and OAuth functionality
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Comprehensive tests for routes.rs - Authentication and OAuth flows
//!
//! This test suite aims to improve coverage from 55.09% to 80%+ by testing
//! all critical authentication, registration, and OAuth functionality.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::{
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::ServerResources,
    models::{Tenant, User, UserStatus},
    routes::{
        auth::{AuthService, OAuthService},
        LoginRequest, RefreshTokenRequest, RegisterRequest,
    },
    tenant::TenantOAuthCredentials,
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

mod common;

// === Test Setup Helpers ===

fn create_minimal_test_config(
    temp_dir: &tempfile::TempDir,
) -> Arc<pierre_mcp_server::config::environment::ServerConfig> {
    std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
        http_port: 8081,
        database: pierre_mcp_server::config::environment::DatabaseConfig {
            url: pierre_mcp_server::config::environment::DatabaseUrl::Memory,
            backup: pierre_mcp_server::config::environment::BackupConfig {
                directory: temp_dir.path().to_path_buf(),
                ..Default::default()
            },
            ..Default::default()
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            ci_mode: true,
            ..Default::default()
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            headers: pierre_mcp_server::config::environment::SecurityHeadersConfig {
                environment: pierre_mcp_server::config::environment::Environment::Testing,
            },
            ..Default::default()
        },
        ..Default::default()
    })
}

async fn create_test_auth_routes() -> Result<AuthService> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = create_minimal_test_config(&temp_dir);
    let cache = common::create_test_cache().await.unwrap();

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    Ok(AuthService::new(
        server_context.auth().clone(),
        server_context.data().clone(),
    ))
}

#[allow(clippy::too_many_lines)] // Long function: Complex test setup with full configuration
async fn create_test_oauth_routes() -> Result<(OAuthService, Uuid, Arc<Database>)> {
    let database = common::create_test_database().await?;

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
    let admin_id = database.create_user(&admin_user).await?;

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
    database.create_tenant(&tenant).await?;

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
        .await?;

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
        .await?;

    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/oauth/callback/strava".to_owned()),
                scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/oauth/callback/fitbit".to_owned()),
                scopes: vec!["activity".to_owned(), "profile".to_owned()],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            whoop: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            garmin_api: pierre_mcp_server::config::environment::GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
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

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    Ok((
        OAuthService::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        ),
        tenant_id,
        database,
    ))
}

// === AuthService Registration Tests ===

#[tokio::test]
async fn test_user_registration_success() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "test@example.com".to_owned(),
        password: "securepassword123".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    let response = auth_routes.register(request).await?;

    assert!(!response.user_id.is_empty());
    assert!(response.message.contains("successfully"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_invalid_email() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "invalid-email".to_owned(),
        password: "securepassword123".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    let result = auth_routes.register(request).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email format"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_weak_password() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "test@example.com".to_owned(),
        password: "weak".to_owned(), // Too short
        display_name: Some("Test User".to_owned()),
    };

    let result = auth_routes.register(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("8 characters"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_duplicate_email() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let request = RegisterRequest {
        email: "duplicate@example.com".to_owned(),
        password: "securepassword123".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    // First registration should succeed
    auth_routes.register(request.clone()).await?;

    // Second registration with same email should fail
    let result = auth_routes.register(request).await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    Ok(())
}

#[tokio::test]
async fn test_user_registration_edge_cases() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    // Test with minimal valid input
    let minimal_request = RegisterRequest {
        email: "minimal@example.com".to_owned(),
        password: "12345678".to_owned(), // Exactly 8 characters
        display_name: None,
    };

    let response = auth_routes.register(minimal_request).await?;
    assert!(!response.user_id.is_empty());

    // Test with very long valid email
    let long_email_request = RegisterRequest {
        email: "very.long.email.address.for.testing@example.com".to_owned(),
        password: "securepassword123".to_owned(),
        display_name: Some("Very Long Display Name For Testing Purposes".to_owned()),
    };

    let response = auth_routes.register(long_email_request).await?;
    assert!(!response.user_id.is_empty());

    Ok(())
}

// === AuthService Login Tests ===

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_user_login_success() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            garmin_api: pierre_mcp_server::config::environment::GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
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

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes = pierre_mcp_server::routes::AuthService::new(
        server_context.auth().clone(),
        server_context.data().clone(),
    );

    // First register a user
    let register_request = RegisterRequest {
        email: "login@example.com".to_owned(),
        password: "loginpassword123".to_owned(),
        display_name: Some("Login User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = uuid::Uuid::parse_str(&register_response.user_id)?;

    // Approve the user for testing
    database
        .update_user_status(
            user_id,
            pierre_mcp_server::models::UserStatus::Active,
            "", // Empty string for test admin
        )
        .await?;

    // Now test login
    let login_request = LoginRequest {
        email: "login@example.com".to_owned(),
        password: "loginpassword123".to_owned(),
    };

    let response = auth_routes.login(login_request).await?;

    assert!(response.jwt_token.as_ref().is_some_and(|t| !t.is_empty()));
    assert!(!response.expires_at.is_empty());
    assert_eq!(response.user.email, "login@example.com");
    assert_eq!(response.user.display_name, Some("Login User".to_owned()));

    Ok(())
}

#[tokio::test]
async fn test_user_login_invalid_email() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_owned(),
        password: "anypassword".to_owned(),
    };

    let result = auth_routes.login(login_request).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email or password"));

    Ok(())
}

#[tokio::test]
async fn test_user_login_invalid_password() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    // Register a user first
    let register_request = RegisterRequest {
        email: "password_test@example.com".to_owned(),
        password: "correctpassword123".to_owned(),
        display_name: Some("Password User".to_owned()),
    };

    auth_routes.register(register_request).await?;

    // Try to login with wrong password
    let login_request = LoginRequest {
        email: "password_test@example.com".to_owned(),
        password: "wrongpassword".to_owned(),
    };

    let result = auth_routes.login(login_request).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid credentials provided"));

    Ok(())
}

#[tokio::test]
async fn test_user_login_case_sensitivity() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    // Register with lowercase email
    let register_request = RegisterRequest {
        email: "case@example.com".to_owned(),
        password: "casepassword123".to_owned(),
        display_name: Some("Case User".to_owned()),
    };

    auth_routes.register(register_request).await?;

    // Try to login with uppercase email (should fail for security)
    let login_request = LoginRequest {
        email: "CASE@EXAMPLE.COM".to_owned(),
        password: "casepassword123".to_owned(),
    };

    let result = auth_routes.login(login_request).await;

    // Email should be case-sensitive for security
    assert!(result.is_err());

    Ok(())
}

// === AuthService Token Refresh Tests ===

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_token_refresh_success() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            garmin_api: pierre_mcp_server::config::environment::GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
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

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes = pierre_mcp_server::routes::AuthService::new(
        server_context.auth().clone(),
        server_context.data().clone(),
    );

    // Register and login to get initial token
    let register_request = RegisterRequest {
        email: "refresh@example.com".to_owned(),
        password: "refreshpassword123".to_owned(),
        display_name: Some("Refresh User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = register_response.user_id;
    let user_uuid = uuid::Uuid::parse_str(&user_id)?;

    // Approve the user for testing
    database
        .update_user_status(
            user_uuid,
            pierre_mcp_server::models::UserStatus::Active,
            "", // Empty string for test admin
        )
        .await?;

    let login_request = LoginRequest {
        email: "refresh@example.com".to_owned(),
        password: "refreshpassword123".to_owned(),
    };

    let login_response = auth_routes.login(login_request).await?;
    let original_token = login_response
        .jwt_token
        .ok_or_else(|| anyhow::anyhow!("JWT token not found in login response"))?;

    // Test token refresh
    let refresh_request = RefreshTokenRequest {
        token: original_token.clone(),
        user_id: user_id.clone(),
    };

    let refresh_response = auth_routes.refresh_token(refresh_request).await?;

    // Token refresh should return a valid token (may be same or different depending on implementation)
    assert!(refresh_response
        .jwt_token
        .as_ref()
        .is_some_and(|t| !t.is_empty()));
    assert_eq!(refresh_response.user.email, "refresh@example.com");

    Ok(())
}

#[tokio::test]
async fn test_token_refresh_invalid_token() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let refresh_request = RefreshTokenRequest {
        token: "invalid.jwt.token".to_owned(),
        user_id: Uuid::new_v4().to_string(),
    };

    let result = auth_routes.refresh_token(refresh_request).await;

    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_token_refresh_mismatched_user() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            garmin_api: pierre_mcp_server::config::environment::GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
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

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes = pierre_mcp_server::routes::AuthService::new(
        server_context.auth().clone(),
        server_context.data().clone(),
    );

    // Register and login to get a valid token
    let register_request = RegisterRequest {
        email: "mismatch@example.com".to_owned(),
        password: "mismatchpassword123".to_owned(),
        display_name: Some("Mismatch User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = uuid::Uuid::parse_str(&register_response.user_id)?;

    // Approve the user for testing
    database
        .update_user_status(
            user_id,
            pierre_mcp_server::models::UserStatus::Active,
            "", // Empty string for test admin
        )
        .await?;

    let login_request = LoginRequest {
        email: "mismatch@example.com".to_owned(),
        password: "mismatchpassword123".to_owned(),
    };

    let login_response = auth_routes.login(login_request).await?;

    // Try to refresh with different user ID
    let refresh_request = RefreshTokenRequest {
        token: login_response
            .jwt_token
            .ok_or_else(|| anyhow::anyhow!("JWT token not found in login response"))?,
        user_id: Uuid::new_v4().to_string(), // Different user ID
    };

    let result = auth_routes.refresh_token(refresh_request).await;

    assert!(result.is_err());

    Ok(())
}

// === OAuthService Tests ===

#[tokio::test]
async fn test_oauth_get_auth_url_strava() -> Result<()> {
    common::init_server_config();
    let (oauth_routes, tenant_id, _database) = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let response = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .await?;

    assert!(response.authorization_url.contains("strava.com"));
    assert!(response.authorization_url.contains("authorize"));
    assert!(!response.state.is_empty());
    assert!(!response.instructions.is_empty());
    assert!(response.expires_in_minutes > 0);

    Ok(())
}

#[tokio::test]
async fn test_oauth_get_auth_url_fitbit() -> Result<()> {
    common::init_server_config();
    let (oauth_routes, tenant_id, _database) = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let response = oauth_routes
        .get_auth_url(user_id, tenant_id, "fitbit")
        .await?;

    // Fitbit is now fully implemented with provider-fitbit feature
    assert!(response.authorization_url.contains("fitbit.com"));
    assert!(response.authorization_url.contains("authorize"));
    assert!(!response.state.is_empty());
    assert!(!response.instructions.is_empty());
    assert!(response.expires_in_minutes > 0);

    Ok(())
}

#[tokio::test]
async fn test_oauth_get_auth_url_unsupported_provider() -> Result<()> {
    common::init_server_config();
    let (oauth_routes, tenant_id, _database) = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    let result = oauth_routes
        .get_auth_url(user_id, tenant_id, "unsupported_provider")
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));

    Ok(())
}

#[tokio::test]
async fn test_oauth_connection_status_no_connections() -> Result<()> {
    common::init_server_config();
    let (oauth_routes, _tenant_id, database) = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    // Create the user in the database first
    let user = pierre_mcp_server::models::User {
        id: user_id,
        email: format!("test_{user_id}@example.com"),
        display_name: Some("Test User".to_owned()),
        password_hash: "test_hash".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: None,
    };
    database.create_user(&user).await?;

    let status = oauth_routes.get_connection_status(user_id).await?;

    // Should return status for all providers
    assert!(status.len() >= 2); // At least Strava and Fitbit

    // All should be disconnected initially
    for connection in status {
        assert!(!connection.connected);
        assert!(connection.expires_at.is_none());
    }

    Ok(())
}

#[tokio::test]
async fn test_oauth_disconnect_provider_success() -> Result<()> {
    common::init_server_config();
    let (oauth_routes, _tenant_id, database) = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    // Create a test user in the database
    let user = User {
        id: user_id,
        email: format!("test_{user_id}@example.com"),
        display_name: None,
        password_hash: "test_hash".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("00000000-0000-0000-0000-000000000000".to_owned()),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&user).await?;

    // Disconnecting a provider that wasn't connected should succeed (idempotent)
    let result = oauth_routes.disconnect_provider(user_id, "strava").await;

    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_oauth_disconnect_invalid_provider() -> Result<()> {
    common::init_server_config();
    let (oauth_routes, _tenant_id, database) = create_test_oauth_routes().await?;
    let user_id = Uuid::new_v4();

    // Create a test user in the database
    let user = User {
        id: user_id,
        email: format!("test_{user_id}@example.com"),
        display_name: None,
        password_hash: "test_hash".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("00000000-0000-0000-0000-000000000000".to_owned()),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&user).await?;

    let result = oauth_routes
        .disconnect_provider(user_id, "invalid_provider")
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));

    Ok(())
}

// === Email and Password Validation Tests ===

#[tokio::test]
async fn test_email_validation_comprehensive() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    // Test obviously invalid email formats that should definitely fail
    let invalid_emails = ["invalid-email", "@example.com", "user@", ""];

    for email in invalid_emails {
        let request = RegisterRequest {
            email: email.to_owned(),
            password: "validpassword123".to_owned(),
            display_name: Some("Test User".to_owned()),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Email '{email}' should be invalid");
    }

    // Test valid email formats
    let valid_emails = [
        "user@example.com",
        "test.user@example.com",
        "user+tag@example.com",
        "user123@example123.com",
        "a@b.co",
    ];

    for (i, email) in valid_emails.iter().enumerate() {
        let request = RegisterRequest {
            email: (*email).to_owned(),
            password: "validpassword123".to_owned(),
            display_name: Some(format!("Test User {i}")),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_ok(), "Email '{email}' should be valid");
    }

    Ok(())
}

#[tokio::test]
async fn test_password_validation_comprehensive() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    // Test invalid passwords (too short)
    let invalid_passwords = [
        "", "1", "12", "123", "1234", "12345", "123456",
        "1234567", // 7 characters - should fail
    ];

    for (i, password) in invalid_passwords.iter().enumerate() {
        let request = RegisterRequest {
            email: format!("test{i}@example.com"),
            password: (*password).to_owned(),
            display_name: Some("Test User".to_owned()),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_err(), "Password '{password}' should be invalid");
    }

    // Test valid passwords (8+ characters)
    let valid_passwords = [
        "12345678", // Exactly 8 characters
        "validpassword",
        "ValidPassword123",
        "very_long_password_that_exceeds_minimum_requirements",
        "P@ssw0rd!",
        "简单密码", // Unicode characters
    ];

    for (i, password) in valid_passwords.iter().enumerate() {
        let request = RegisterRequest {
            email: format!("valid{i}@example.com"),
            password: (*password).to_owned(),
            display_name: Some("Test User".to_owned()),
        };

        let result = auth_routes.register(request).await;
        assert!(result.is_ok(), "Password should be valid");
    }

    Ok(())
}

// === Integration Tests ===

#[tokio::test]
#[serial]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_complete_auth_flow() -> Result<()> {
    common::init_server_config();
    // Set required environment variables for OAuth
    std::env::set_var("STRAVA_CLIENT_ID", "test_client_id");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_client_secret");
    std::env::set_var("FITBIT_CLIENT_ID", "test_fitbit_client_id");
    std::env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_client_secret");

    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            garmin_api: pierre_mcp_server::config::environment::GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
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

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes = pierre_mcp_server::routes::AuthService::new(
        server_context.auth().clone(),
        server_context.data().clone(),
    );
    let oauth_routes = pierre_mcp_server::routes::OAuthService::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    // 1. Register user
    let register_request = RegisterRequest {
        email: "integration@example.com".to_owned(),
        password: "integrationpass123".to_owned(),
        display_name: Some("Integration User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = Uuid::parse_str(&register_response.user_id)?;

    // Approve the user for testing
    database
        .update_user_status(
            user_id,
            pierre_mcp_server::models::UserStatus::Active,
            "", // Empty string for test admin
        )
        .await?;

    // 2. Login
    let login_request = LoginRequest {
        email: "integration@example.com".to_owned(),
        password: "integrationpass123".to_owned(),
    };

    let login_response = auth_routes.login(login_request).await?;

    // 3. Refresh token
    let refresh_request = RefreshTokenRequest {
        token: login_response
            .jwt_token
            .ok_or_else(|| anyhow::anyhow!("JWT token not found in login response"))?,
        user_id: user_id.to_string(),
    };

    let refresh_response = auth_routes.refresh_token(refresh_request).await?;

    // 4. Check OAuth connection status
    let connections = oauth_routes.get_connection_status(user_id).await?;

    // 5. Get OAuth authorization URL (need tenant credentials first)
    // Create admin user and tenant for OAuth
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin_integration@example.com".to_owned(),
        display_name: Some("Admin".to_owned()),
        password_hash: "hash".to_owned(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: true,
        approved_by: None,
        approved_at: None,
    };
    let admin_id = database.create_user(&admin_user).await?;

    let tenant_id = Uuid::new_v4();
    let tenant = pierre_mcp_server::models::Tenant {
        id: tenant_id,
        name: "Integration Test Tenant".to_owned(),
        slug: "integration-tenant".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await?;

    let strava_credentials = pierre_mcp_server::tenant::TenantOAuthCredentials {
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
        .await?;

    let auth_url = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .await?;

    // Verify everything worked
    assert!(!register_response.user_id.is_empty());
    assert!(refresh_response
        .jwt_token
        .as_ref()
        .is_some_and(|t| !t.is_empty()));
    assert!(!connections.is_empty());
    assert!(!auth_url.authorization_url.is_empty());

    Ok(())
}

// === Concurrency Tests ===

#[tokio::test]
#[serial]
async fn test_concurrent_registrations() -> Result<()> {
    common::init_server_config();
    let auth_routes = create_test_auth_routes().await?;

    let mut handles = vec![];

    for i in 0..5 {
        let routes = auth_routes.clone();
        handles.push(tokio::spawn(async move {
            let request = RegisterRequest {
                email: format!("concurrent{i}@example.com"),
                password: "concurrentpass123".to_owned(),
                display_name: Some(format!("Concurrent User {i}")),
            };

            routes.register(request).await
        }));
    }

    // All registrations should succeed
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    Ok(())
}

#[tokio::test]
#[serial]
#[allow(clippy::too_many_lines)] // Long function: Complex test with full setup
async fn test_concurrent_logins() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();
    let temp_dir = tempfile::tempdir()?;
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_strava_client_id".to_owned()),
                client_secret: Some("test_strava_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/fitbit".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_garmin_client_id".to_owned()),
                client_secret: Some("test_garmin_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8080/api/oauth/callback/garmin".to_owned()),
                scopes: vec![],
                enabled: true,
            },
            whoop: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            garmin_api: pierre_mcp_server::config::environment::GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
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

    let server_resources = std::sync::Arc::new(ServerResources::new(
        (*database).clone(),
        (*auth_manager).clone(),
        "test_jwt_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    let server_context = pierre_mcp_server::context::ServerContext::from(server_resources.as_ref());
    let auth_routes = pierre_mcp_server::routes::AuthService::new(
        server_context.auth().clone(),
        server_context.data().clone(),
    );

    // First register and approve users
    for i in 0..3 {
        let request = RegisterRequest {
            email: format!("login_concurrent{i}@example.com"),
            password: "loginpass123".to_owned(),
            display_name: Some(format!("Login User {i}")),
        };
        let register_response = auth_routes.register(request).await?;
        let user_id = uuid::Uuid::parse_str(&register_response.user_id)?;

        // Approve the user
        database
            .update_user_status(
                user_id,
                pierre_mcp_server::models::UserStatus::Active,
                "", // Empty string for test admin
            )
            .await?;
    }

    let mut handles = vec![];

    for i in 0..3 {
        let routes = auth_routes.clone();
        handles.push(tokio::spawn(async move {
            let request = LoginRequest {
                email: format!("login_concurrent{i}@example.com"),
                password: "loginpass123".to_owned(),
            };

            routes.login(request).await
        }));
    }

    // All logins should succeed
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    Ok(())
}
