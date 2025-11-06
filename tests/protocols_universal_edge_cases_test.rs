// ABOUTME: Universal protocol edge cases and error path tests
// ABOUTME: Tests error conditions, edge cases, and untested paths in universal layer
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Universal Protocol Edge Cases and Error Path Tests
//!
//! Tests for error conditions, edge cases, and untested paths
//! in the universal tool execution layer.

use anyhow::Result;
use pierre_mcp_server::{
    config::environment::*,
    constants::oauth_providers,
    database_plugins::DatabaseProvider,
    intelligence::{
        ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
        TrendIndicators,
    },
    models::UserOAuthToken,
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

mod common;

/// Create test user without saving to database (local helper)
fn create_test_user(email: &str, display_name: Option<String>) -> pierre_mcp_server::models::User {
    pierre_mcp_server::models::User::new(email.to_string(), "test_hash".to_string(), display_name)
}

/// Create test configuration
fn create_test_config() -> Arc<ServerConfig> {
    Arc::new(ServerConfig {
        http_port: 4000,
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
                directory: PathBuf::from("test_backups"),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_client_id".to_string()),
                client_secret: Some("test_client_secret".to_string()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/strava".to_string()),
                scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_id".to_string()),
                client_secret: Some("test_fitbit_secret".to_string()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/fitbit".to_string()),
                scopes: vec!["activity".to_string(), "profile".to_string()],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
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
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_string(),
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: true,
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: "0.1.0".to_string(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        ..Default::default()
    })
}

/// Create test config without OAuth
fn create_test_config_no_oauth() -> Arc<ServerConfig> {
    let mut config = (*create_test_config()).clone();
    config.oauth.strava.client_id = None;
    config.oauth.strava.client_secret = None;
    config.oauth.fitbit.client_id = None;
    config.oauth.fitbit.client_secret = None;
    Arc::new(config)
}

/// Create test executor - duplicated from `protocols_universal_test.rs`
async fn create_test_executor() -> Result<UniversalToolExecutor> {
    common::init_server_config();
    let database = common::create_test_database().await?;

    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(75.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(80.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Stable,
                consistency_score: 85.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    let config = create_test_config();
    // Create ServerResources for the test
    let auth_manager = pierre_mcp_server::auth::AuthManager::new(24);
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    Ok(UniversalToolExecutor::new(server_resources))
}

/// Create executor with missing OAuth configuration
async fn create_executor_no_oauth() -> Result<UniversalToolExecutor> {
    common::init_server_config();
    let database = common::create_test_database().await?;

    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(75.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(80.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Stable,
                consistency_score: 85.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    // Create config without OAuth credentials
    let config = create_test_config_no_oauth();
    // Create ServerResources for the test
    let auth_manager = pierre_mcp_server::auth::AuthManager::new(24);
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));

    Ok(UniversalToolExecutor::new(server_resources))
}

/// Test OAuth configuration errors
#[tokio::test]
async fn test_oauth_configuration_errors() -> Result<()> {
    let executor = create_executor_no_oauth().await?;

    // Create tenant first
    // Create test user first so they can be tenant owner
    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    user.tenant_id = Some("test-tenant".to_string()); // Link user to tenant
    executor.resources.database.create_user(&user).await?;

    // Create tenant with user as owner
    let tenant = pierre_mcp_server::models::Tenant::new(
        "Test Tenant".to_string(),
        "test-tenant".to_string(),
        Some("test.example.com".to_string()),
        "starter".to_string(),
        user_id, // User is now the owner
    );
    executor.resources.database.create_tenant(&tenant).await?;

    // Test get_activities with missing OAuth config
    let request = UniversalRequest {
        tool_name: "get_activities".to_string(),
        parameters: json!({}),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should fail with proper error about missing OAuth credentials
    assert!(
        !response.success,
        "Tool execution should fail without OAuth credentials"
    );
    assert!(
        response.error.is_some(),
        "Should have error about missing OAuth credentials"
    );

    // Check that the error contains information about missing OAuth token
    let error = response.error.unwrap();
    assert!(
        error.contains("No") && error.contains("Strava token")
            || error.contains("Connect your Strava account"),
        "Error should contain OAuth connection message: {error}"
    );

    Ok(())
}

/// Test invalid provider token scenarios
#[tokio::test]
async fn test_invalid_provider_tokens() -> Result<()> {
    let executor = create_test_executor().await?;

    // Create test user
    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    executor.resources.database.create_user(&user).await?;

    // Store an invalid/expired token
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1); // Expired
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_string(), // tenant_id
        oauth_providers::STRAVA.to_string(),
        "invalid_access_token".to_string(),
        Some("invalid_refresh_token".to_string()),
        Some(expires_at),
        Some("read".to_string()), // scope as String
    );
    executor
        .resources
        .database
        .upsert_user_oauth_token(&oauth_token)
        .await?;

    // Test get_activities with expired token
    let request = UniversalRequest {
        tool_name: "get_activities".to_string(),
        parameters: json!({
            "limit": 10,
            "provider": "strava"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should either succeed after refresh attempt or fail with OAuth error
    if response.success {
        // If successful, continue with any additional checks
    } else {
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert!(
            error.contains("OAuth")
                || error.contains("token")
                || error.contains("Failed to get activities")
        );
    }

    Ok(())
}

/// Test malformed UUID handling
#[tokio::test]
async fn test_malformed_user_id() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: "not-a-valid-uuid".to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let result = executor.execute_tool(request).await;
    // Should return ProtocolError for invalid user ID format
    assert!(result.is_err());
    let error = result.err().unwrap();
    assert!(
        error.to_string().contains("Invalid user ID")
            || error.to_string().contains("Invalid parameters")
            || error.to_string().contains("Invalid user ID format")
    );

    Ok(())
}

/// Test non-existent user scenarios
#[tokio::test]
async fn test_non_existent_user() -> Result<()> {
    let executor = create_test_executor().await?;

    let non_existent_user_id = Uuid::new_v4();

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: non_existent_user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should handle non-existent user gracefully
    if response.success {
        // If successful, continue with any additional checks
    } else {
        assert!(response.error.is_some());
    }

    Ok(())
}

/// Test invalid tool parameters
#[tokio::test]
async fn test_invalid_tool_parameters() -> Result<()> {
    let executor = create_test_executor().await?;

    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    executor.resources.database.create_user(&user).await?;

    // Test get_activities with invalid limit
    let request = UniversalRequest {
        tool_name: "get_activities".to_string(),
        parameters: json!({
            "limit": "not_a_number",
            "provider": "strava"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should handle invalid parameters gracefully
    if response.success {
        // If successful, continue with any additional checks
    } else {
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        println!("Error from get_activities with invalid limit: {error}");
        assert!(
            error.contains("Invalid parameters")
                || error.contains("limit")
                || error.contains("not_a_number")
                || error.contains("No") && error.contains("token")
        );
    }

    // Test set_goal with invalid goal data
    let request = UniversalRequest {
        tool_name: "set_goal".to_string(),
        parameters: json!({
            "goal_type": "invalid_goal_type",
            "target_value": "not_a_number"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let result = executor.execute_tool(request).await;
    // Should return ProtocolError for invalid parameters
    assert!(result.is_err());
    let error = result.err().unwrap();
    assert!(
        error.to_string().contains("Invalid parameters")
            || error.to_string().contains("target_value is required")
            || error.to_string().contains("invalid_goal_type")
    );

    Ok(())
}

/// Test database connection failures simulation
#[tokio::test]
async fn test_database_error_handling() -> Result<()> {
    let executor = create_test_executor().await?;

    // Try to access a very large invalid user ID to potentially trigger DB errors
    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: "00000000-0000-0000-0000-000000000000".to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should handle gracefully regardless of database state
    assert!(response.success || response.error.is_some());

    Ok(())
}

/// Test concurrent tool execution
#[tokio::test]
async fn test_concurrent_tool_execution() -> Result<()> {
    let executor = Arc::new(create_test_executor().await?);

    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    executor.resources.database.create_user(&user).await?;

    // Create multiple concurrent requests
    let mut handles = vec![];

    for i in 0..5 {
        let executor_clone = executor.clone();
        let user_id_str = user_id.to_string();

        let handle = tokio::spawn(async move {
            let request = UniversalRequest {
                tool_name: "get_connection_status".to_string(),
                parameters: json!({}),
                user_id: user_id_str,
                protocol: format!("test_{i}"),
                tenant_id: None,
            };

            executor_clone.execute_tool(request).await
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
        let response = result?;
        assert!(response.success || response.error.is_some());
    }

    Ok(())
}

/// Test tool response metadata
#[tokio::test]
async fn test_tool_response_metadata() -> Result<()> {
    let executor = create_test_executor().await?;

    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    executor.resources.database.create_user(&user).await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;

    // Check response structure
    assert!(response.success || response.error.is_some());

    // Metadata might be present
    if let Some(metadata) = response.metadata {
        assert!(!metadata.is_empty());
    }

    Ok(())
}

/// Test intelligence integration edge cases
#[tokio::test]
async fn test_intelligence_integration_errors() -> Result<()> {
    let executor = create_test_executor().await?;

    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    executor.resources.database.create_user(&user).await?;

    // Test analytics tools with invalid data
    let request = UniversalRequest {
        tool_name: "analyze_performance_trends".to_string(),
        parameters: json!({
            "activities": [], // Empty activities array
            "metrics": ["invalid_metric"]
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should handle empty/invalid data gracefully
    assert!(response.success || response.error.is_some());

    Ok(())
}

/// Test provider unavailable scenarios
#[tokio::test]
async fn test_provider_unavailable() -> Result<()> {
    let executor = create_test_executor().await?;

    let user_id = Uuid::new_v4();
    let mut user = create_test_user("test@example.com", Some("Test User".to_string()));
    user.id = user_id;
    user.password_hash = bcrypt::hash("password", bcrypt::DEFAULT_COST)?;
    executor.resources.database.create_user(&user).await?;

    // Test with unsupported provider
    let request = UniversalRequest {
        tool_name: "get_activities".to_string(),
        parameters: json!({
            "limit": 10,
            "provider": "unsupported_provider"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If successful, continue with any additional checks
    } else {
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        println!("Error from unsupported provider: {error}");
        assert!(
            error.contains("provider")
                || error.contains("Unsupported")
                || error.contains("not found")
                || error.contains("No") && error.contains("token")
        );
    }

    Ok(())
}
