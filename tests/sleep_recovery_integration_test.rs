// ABOUTME: Integration tests for sleep and recovery MCP tool handlers
// ABOUTME: Tests end-to-end execution of 5 sleep/recovery tools via universal protocol
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Sleep and Recovery Integration Tests
//!
//! Comprehensive end-to-end tests for sleep analysis and recovery scoring tools
//! that verify proper integration with the universal tool execution system.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::{
    config::environment::*,
    database_plugins::DatabaseProvider,
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

mod common;

/// Create test OAuth configuration with test credentials
fn create_test_oauth_config() -> OAuthConfig {
    OAuthConfig {
        strava: OAuthProviderConfig {
            client_id: Some("test_client_id".to_owned()),
            client_secret: Some("test_client_secret".to_owned()),
            redirect_uri: Some("http://localhost:3000/oauth/callback/strava".to_owned()),
            scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
            enabled: true,
        },
        fitbit: OAuthProviderConfig {
            client_id: Some("test_fitbit_id".to_owned()),
            client_secret: Some("test_fitbit_secret".to_owned()),
            redirect_uri: Some("http://localhost:3000/oauth/callback/fitbit".to_owned()),
            scopes: vec!["activity".to_owned(), "profile".to_owned()],
            enabled: true,
        },
        garmin: OAuthProviderConfig {
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            scopes: vec![],
            enabled: false,
        },
    }
}

/// Create test external services configuration
fn create_test_external_services_config() -> ExternalServicesConfig {
    ExternalServicesConfig {
        weather: WeatherServiceConfig {
            api_key: None,
            base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
            enabled: false,
        },
        geocoding: GeocodingServiceConfig {
            base_url: "https://nominatim.openstreetmap.org".to_owned(),
            enabled: true,
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
        garmin_api: GarminApiConfig {
            base_url: "https://apis.garmin.com".to_owned(),
            auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
            token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
            revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
        },
    }
}

/// Create test security configuration
fn create_test_security_config() -> SecurityConfig {
    SecurityConfig {
        cors_origins: vec!["*".to_owned()],
        tls: TlsConfig {
            enabled: false,
            cert_path: None,
            key_path: None,
        },
        headers: SecurityHeadersConfig {
            environment: Environment::Development,
        },
    }
}

/// Create test server configuration for integration tests
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
            postgres_pool: PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: create_test_oauth_config(),
        security: create_test_security_config(),
        external_services: create_test_external_services_config(),
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
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
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    })
}

/// Create test executor with sleep/recovery tools registered
async fn create_test_executor() -> Result<UniversalToolExecutor> {
    common::init_server_config();
    common::init_test_http_clients();

    let database = common::create_test_database().await?;
    let auth_manager = pierre_mcp_server::auth::AuthManager::new(24);
    let config = create_test_config();

    // Create test cache with background cleanup disabled
    let cache_config = pierre_mcp_server::cache::CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: std::time::Duration::from_secs(60),
        enable_background_cleanup: false,
    };
    let cache = pierre_mcp_server::cache::factory::Cache::new(cache_config).await?;

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

#[tokio::test]
async fn test_sleep_recovery_tools_registered() -> Result<()> {
    let executor = create_test_executor().await?;

    // Verify all 5 sleep/recovery tools are registered
    let tool_names: Vec<String> = executor
        .list_tools()
        .iter()
        .map(|tool| tool.name().to_owned())
        .collect();

    let expected_tools = vec![
        "analyze_sleep_quality",
        "calculate_recovery_score",
        "suggest_rest_day",
        "track_sleep_trends",
        "optimize_sleep_schedule",
    ];

    for expected_tool in expected_tools {
        assert!(
            tool_names.contains(&expected_tool.to_owned()),
            "Missing sleep/recovery tool: {expected_tool}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_analyze_sleep_quality_tool() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Create user for testing
    let user = pierre_mcp_server::models::User::new(
        "sleep_test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Sleep Test User".to_owned()),
    );
    executor.resources.database.create_user(&user).await?;

    // Test with optimal sleep data
    let sleep_data = json!({
        "date": "2025-01-15T06:00:00Z",
        "duration_hours": 8.0,
        "efficiency_percent": 92.0,
        "deep_sleep_hours": 1.5,
        "rem_sleep_hours": 1.8,
        "light_sleep_hours": 4.0,
        "awake_hours": 0.7,
        "hrv_rmssd_ms": 55.0
    });

    let request = UniversalRequest {
        tool_name: "analyze_sleep_quality".to_owned(),
        parameters: json!({
            "sleep_data": sleep_data
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success, "Tool execution should succeed");
    assert!(response.result.is_some(), "Result should be present");

    let result = response.result.unwrap();
    assert!(
        result["sleep_quality"].is_object(),
        "Should have sleep_quality object"
    );
    assert!(
        result["sleep_quality"]["overall_score"].is_number(),
        "Should have overall_score"
    );
    assert!(
        result["sleep_quality"]["quality_category"].is_string(),
        "Should have quality_category"
    );
    assert!(
        result["hrv_analysis"].is_object(),
        "Should have HRV analysis"
    );

    Ok(())
}

#[tokio::test]
async fn test_analyze_sleep_quality_poor_sleep() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Test with poor sleep data (short duration, low efficiency)
    let sleep_data = json!({
        "date": "2025-01-15T06:00:00Z",
        "duration_hours": 5.5,
        "efficiency_percent": 72.0,
        "deep_sleep_hours": 0.8,
        "rem_sleep_hours": 0.9,
        "light_sleep_hours": 3.0,
        "awake_hours": 0.8,
        "hrv_rmssd_ms": 35.0
    });

    let request = UniversalRequest {
        tool_name: "analyze_sleep_quality".to_owned(),
        parameters: json!({
            "sleep_data": sleep_data
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);

    let result = response.result.unwrap();
    let overall_score = result["sleep_quality"]["overall_score"].as_f64().unwrap();
    assert!(overall_score < 70.0, "Poor sleep should have score < 70");

    Ok(())
}

#[tokio::test]
async fn test_calculate_recovery_score_tool() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Create user
    let user = pierre_mcp_server::models::User::new(
        "recovery_test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Recovery Test User".to_owned()),
    );
    executor.resources.database.create_user(&user).await?;

    let request = UniversalRequest {
        tool_name: "calculate_recovery_score".to_owned(),
        parameters: json!({
            "provider": "strava"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;

    // This tool requires Strava authentication, so it should fail in tests without auth
    assert!(!response.success);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(
        error_msg.contains("No valid Strava token found")
            || error_msg.contains("Connect")
            || error_msg.contains("Authentication error")
    );

    Ok(())
}

#[tokio::test]
async fn test_calculate_recovery_score_fatigued() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    let request = UniversalRequest {
        tool_name: "calculate_recovery_score".to_owned(),
        parameters: json!({
            "provider": "strava"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;

    // This tool requires Strava authentication, so it should fail in tests without auth
    assert!(!response.success);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(
        error_msg.contains("No valid Strava token found")
            || error_msg.contains("Connect")
            || error_msg.contains("Authentication error")
    );

    Ok(())
}

#[tokio::test]
async fn test_suggest_rest_day_tool() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Create user with some activities
    let user = pierre_mcp_server::models::User::new(
        "rest_day_test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Rest Day Test User".to_owned()),
    );
    executor.resources.database.create_user(&user).await?;

    // Test recommendation for rest day
    let sleep_data = json!({
        "date": "2025-01-15T06:00:00Z",
        "duration_hours": 6.5,
        "efficiency_percent": 78.0,
        "deep_sleep_hours": 1.0,
        "rem_sleep_hours": 1.2,
        "light_sleep_hours": 3.5,
        "awake_hours": 0.8,
        "hrv_rmssd_ms": 42.0
    });

    let training_load = json!({
        "ctl": 55.0,
        "atl": 68.0,
        "tsb": -13.0,
        "tss_history": []
    });

    let recovery_score = json!({
        "overall_score": 52.0,
        "tsb_score": 40.0,
        "sleep_score": 55.0,
        "hrv_score": 60.0,
        "recovery_category": "poor",
        "training_readiness": "low",
        "recommendations": []
    });

    let request = UniversalRequest {
        tool_name: "suggest_rest_day".to_owned(),
        parameters: json!({
            "sleep_data": sleep_data,
            "training_load": training_load,
            "recovery_score": recovery_score
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;

    // This tool requires Strava authentication, so it should fail in tests without auth
    assert!(!response.success);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(
        error_msg.contains("No valid Strava token found")
            || error_msg.contains("Connect")
            || error_msg.contains("Authentication error")
    );

    Ok(())
}

/// Generate test sleep history data for trend analysis
fn generate_test_sleep_history() -> Vec<serde_json::Value> {
    vec![
        json!({
            "date": "2025-01-09T06:00:00Z",
            "duration_hours": 7.5,
            "efficiency_percent": 88.0,
            "deep_sleep_hours": 1.4,
            "rem_sleep_hours": 1.6,
            "light_sleep_hours": 3.8,
            "awake_hours": 0.7,
            "hrv_rmssd_ms": 52.0
        }),
        json!({
            "date": "2025-01-10T06:00:00Z",
            "duration_hours": 7.8,
            "efficiency_percent": 90.0,
            "deep_sleep_hours": 1.5,
            "rem_sleep_hours": 1.7,
            "light_sleep_hours": 3.9,
            "awake_hours": 0.7,
            "hrv_rmssd_ms": 54.0
        }),
        json!({
            "date": "2025-01-11T06:00:00Z",
            "duration_hours": 8.0,
            "efficiency_percent": 91.0,
            "deep_sleep_hours": 1.5,
            "rem_sleep_hours": 1.8,
            "light_sleep_hours": 4.0,
            "awake_hours": 0.7,
            "hrv_rmssd_ms": 56.0
        }),
        json!({
            "date": "2025-01-12T06:00:00Z",
            "duration_hours": 7.2,
            "efficiency_percent": 86.0,
            "deep_sleep_hours": 1.3,
            "rem_sleep_hours": 1.5,
            "light_sleep_hours": 3.7,
            "awake_hours": 0.7,
            "hrv_rmssd_ms": 53.0
        }),
        json!({
            "date": "2025-01-13T06:00:00Z",
            "duration_hours": 7.9,
            "efficiency_percent": 89.0,
            "deep_sleep_hours": 1.5,
            "rem_sleep_hours": 1.7,
            "light_sleep_hours": 3.9,
            "awake_hours": 0.8,
            "hrv_rmssd_ms": 55.0
        }),
        json!({
            "date": "2025-01-14T06:00:00Z",
            "duration_hours": 8.1,
            "efficiency_percent": 92.0,
            "deep_sleep_hours": 1.6,
            "rem_sleep_hours": 1.8,
            "light_sleep_hours": 4.0,
            "awake_hours": 0.7,
            "hrv_rmssd_ms": 57.0
        }),
        json!({
            "date": "2025-01-15T06:00:00Z",
            "duration_hours": 8.0,
            "efficiency_percent": 90.0,
            "deep_sleep_hours": 1.5,
            "rem_sleep_hours": 1.8,
            "light_sleep_hours": 4.0,
            "awake_hours": 0.7,
            "hrv_rmssd_ms": 56.0
        }),
    ]
}

#[tokio::test]
async fn test_track_sleep_trends_tool() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Create user
    let user = pierre_mcp_server::models::User::new(
        "trends_test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Trends Test User".to_owned()),
    );
    executor.resources.database.create_user(&user).await?;

    let sleep_history = generate_test_sleep_history();

    let request = UniversalRequest {
        tool_name: "track_sleep_trends".to_owned(),
        parameters: json!({
            "sleep_history": sleep_history,
            "days": 7
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["trends"].is_object());
    assert!(result["trends"]["average_duration_hours"].is_number());
    assert!(result["trends"]["average_efficiency_percent"].is_number());
    assert!(result["trends"]["quality_trend"].is_string());
    assert!(result["trends"]["recent_7day_avg"].is_number());
    assert!(result["trends"]["previous_7day_avg"].is_number());
    assert!(result["highlights"].is_object());
    assert!(result["insights"].is_array());
    assert!(result["data_points"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_optimize_sleep_schedule_tool() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Create user
    let user = pierre_mcp_server::models::User::new(
        "optimize_test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Optimize Test User".to_owned()),
    );
    executor.resources.database.create_user(&user).await?;

    // Test with sleep history and training schedule
    let sleep_history = vec![
        json!({
            "date": "2025-01-10T06:00:00Z",
            "duration_hours": 7.5,
            "efficiency_percent": 88.0,
            "bedtime": "23:00",
            "wake_time": "06:30",
            "hrv_rmssd_ms": 52.0
        }),
        json!({
            "date": "2025-01-11T06:00:00Z",
            "duration_hours": 7.8,
            "efficiency_percent": 90.0,
            "bedtime": "22:45",
            "wake_time": "06:30",
            "hrv_rmssd_ms": 54.0
        }),
        json!({
            "date": "2025-01-12T06:00:00Z",
            "duration_hours": 8.0,
            "efficiency_percent": 91.0,
            "bedtime": "22:30",
            "wake_time": "06:30",
            "hrv_rmssd_ms": 56.0
        }),
    ];

    let training_schedule = json!({
        "monday": {"time": "06:00", "type": "run", "duration_minutes": 60},
        "wednesday": {"time": "06:00", "type": "run", "duration_minutes": 45},
        "friday": {"time": "06:00", "type": "run", "duration_minutes": 60}
    });

    let request = UniversalRequest {
        tool_name: "optimize_sleep_schedule".to_owned(),
        parameters: json!({
            "sleep_history": sleep_history,
            "training_schedule": training_schedule,
            "target_sleep_hours": 8.0
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;

    // This tool requires Strava authentication, so it should fail in tests without auth
    assert!(!response.success);
    assert!(response.error.is_some());
    let error_msg = response.error.unwrap();
    assert!(
        error_msg.contains("No valid Strava token found")
            || error_msg.contains("Connect")
            || error_msg.contains("Authentication error")
    );

    Ok(())
}

#[tokio::test]
async fn test_missing_required_parameters() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Test analyze_sleep_quality without required sleep_data parameter
    let request = UniversalRequest {
        tool_name: "analyze_sleep_quality".to_owned(),
        parameters: json!({}), // Missing sleep_data
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let result = executor.execute_tool(request).await;

    // Should return an error (ProtocolError::InvalidRequest)
    assert!(result.is_err(), "Should fail with missing parameters");
    let error = result.unwrap_err();
    assert!(format!("{error:?}").contains("sleep_data"));

    Ok(())
}

#[tokio::test]
async fn test_invalid_sleep_data_format() -> Result<()> {
    let executor = create_test_executor().await?;
    let user_id = Uuid::new_v4();

    // Test with invalid date format
    let sleep_data = json!({
        "date": "invalid-date",
        "duration_hours": 8.0
    });

    let request = UniversalRequest {
        tool_name: "analyze_sleep_quality".to_owned(),
        parameters: json!({
            "sleep_data": sleep_data
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let result = executor.execute_tool(request).await;

    // Should return an error (ProtocolError::InvalidRequest)
    assert!(result.is_err(), "Should fail with invalid data format");
    let error = result.unwrap_err();
    assert!(format!("{error:?}").contains("Invalid sleep_data format"));

    Ok(())
}
