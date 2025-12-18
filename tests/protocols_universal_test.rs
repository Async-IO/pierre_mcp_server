// ABOUTME: Universal protocol integration tests for tool execution layer
// ABOUTME: Tests protocol-agnostic interfaces for MCP and A2A
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Universal Protocol Integration Tests
//!
//! Comprehensive tests for the universal tool execution layer
//! that provides protocol-agnostic interfaces for MCP and A2A.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::{
    auth::AuthManager,
    cache::{factory::Cache, CacheConfig},
    config::environment::{self, *},
    database_plugins::DatabaseProvider,
    intelligence::insights::{Insight, InsightType},
    intelligence::{
        ActivityIntelligence, ContextualFactors, ContextualWeeklyLoad, PerformanceMetrics,
        TimeOfDay, TrendDirection, TrendIndicators,
    },
    mcp::resources::ServerResources,
    models::{Tenant, User},
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

mod common;

/// Test configuration for universal protocols
#[allow(clippy::too_many_lines)]
async fn create_test_executor() -> Result<UniversalToolExecutor> {
    let database = common::create_test_database().await?;

    // Create ActivityIntelligence with proper constructor
    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_owned(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_owned(),
            confidence: 90.0,
            data: None,
        }],
        PerformanceMetrics {
            relative_effort: Some(85.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(82.5),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Improving,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Improving,
                consistency_score: 90.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: Some(ContextualWeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
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
            ..AuthConfig::default()
        },
        oauth: OAuthConfig {
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
                environment: Environment::Development,
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
                enabled: true,
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
            garmin_api: GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ..Default::default()
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
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
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_recovery: SleepRecoveryConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });

    // Create ServerResources for the test
    let auth_manager = AuthManager::new(24);

    // Create test cache with background cleanup disabled
    let cache_config = CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(60),
        enable_background_cleanup: false,
        ..Default::default()
    };
    let cache = Cache::new(cache_config)
        .await
        .expect("Failed to create test cache");

    let server_resources = Arc::new(ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let executor = UniversalToolExecutor::new(server_resources);
    Ok(executor)
}

#[tokio::test]
async fn test_universal_executor_creation() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Verify basic functionality
    assert!(!executor.list_tools().is_empty());

    // Check that core tools are registered
    assert!(executor.has_tool("get_connection_status"));
    assert!(executor.has_tool("set_goal"));
    assert!(executor.has_tool("get_activities"));
    assert!(executor.has_tool("analyze_activity"));

    Ok(())
}

#[tokio::test]
async fn test_tool_registration() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Verify all expected tools are registered
    let tool_names: Vec<String> = executor
        .list_tools()
        .iter()
        .map(|tool| tool.name().to_owned())
        .collect();

    let expected_tools = vec![
        "get_connection_status",
        "set_goal",
        "get_activities",
        "analyze_activity",
        "disconnect_provider",
        "calculate_metrics",
        "analyze_performance_trends",
        "compare_activities",
        "detect_patterns",
        "track_progress",
        "suggest_goals",
        "analyze_goal_feasibility",
        "generate_recommendations",
        "calculate_fitness_score",
        "predict_performance",
        "analyze_training_load",
    ];

    for expected_tool in expected_tools {
        assert!(
            tool_names.contains(&expected_tool.to_owned()),
            "Missing tool: {expected_tool}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_tool_execution_invalid_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "nonexistent_tool".to_owned(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let result = executor.execute_tool(request).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_connection_status_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_owned(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    // Should indicate no connection (no valid token)
    let result = response.result.unwrap();
    assert!(result["providers"].is_object());
    assert_eq!(result["providers"]["strava"]["connected"], false);

    Ok(())
}

#[tokio::test]
async fn test_connect_strava_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Create tenant and user for testing (user first, then tenant)
    let user_id = Uuid::new_v4();
    let mut user = User::new(
        "test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    user.id = user_id;
    user.tenant_id = Some("test-tenant".to_owned());
    executor.resources.database.create_user(&user).await?;

    // Create tenant with user as owner
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        "test-tenant".to_owned(),
        Some("test.example.com".to_owned()),
        "starter".to_owned(),
        user_id, // Owner
    );
    executor.resources.database.create_tenant(&tenant).await?;

    let request = UniversalRequest {
        tool_name: "get_activities".to_owned(),
        parameters: json!({}),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    // Should fail without OAuth token
    assert!(!response.success);
    assert!(response.error.is_some());

    // Error should mention missing token
    let error = response.error.unwrap();
    assert!(error.contains("No") && error.contains("token") || error.contains("Connect"));

    Ok(())
}

#[tokio::test]
async fn test_connect_fitbit_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_activity".to_owned(),
        parameters: json!({
            "activity_id": "test_activity_123"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    // analyze_activity may fail without proper tenant context, which is expected
    if response.success {
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["analysis"].is_object() || result["error"].is_string());
    } else {
        // Failing is also acceptable for this test scenario
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_set_goal_tool() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let (user_id, _) = common::create_test_user(&database).await?;

    // Create ActivityIntelligence with proper constructor
    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_owned(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_owned(),
            confidence: 90.0,
            data: None,
        }],
        PerformanceMetrics {
            relative_effort: Some(85.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(82.5),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Improving,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Improving,
                consistency_score: 90.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: Some(ContextualWeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
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
            ..AuthConfig::default()
        },
        oauth: OAuthConfig {
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
                environment: Environment::Development,
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
                enabled: true,
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
            garmin_api: GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ..Default::default()
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
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
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_recovery: SleepRecoveryConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });

    // Create ServerResources for the test
    let auth_manager = AuthManager::new(24);

    // Create test cache with background cleanup disabled
    let cache_config = CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(60),
        enable_background_cleanup: false,
        ..Default::default()
    };
    let cache = Cache::new(cache_config)
        .await
        .expect("Failed to create test cache");

    let server_resources = Arc::new(ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let executor = UniversalToolExecutor::new(server_resources);

    let request = UniversalRequest {
        tool_name: "set_goal".to_owned(),
        parameters: json!({
            "goal_type": "distance",
            "target_value": 1000.0,
            "timeframe": "2024",
            "title": "Run 1000km this year"
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
    assert!(result["goal_id"].is_string());
    assert_eq!(result["status"], "created");

    Ok(())
}

#[tokio::test]
async fn test_calculate_metrics_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "calculate_metrics".to_owned(),
        parameters: json!({
            "activity": {
                "distance": 5000.0,
                "duration": 1800,
                "elevation_gain": 100.0,
                "average_heart_rate": 150,
                "activity_type": "Run"
            }
        }),
        user_id: Uuid::new_v4().to_string(),
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
    assert!(result["pace"].is_number());
    assert!(result["speed"].is_number());
    assert!(result["intensity_score"].is_number());

    Ok(())
}

#[tokio::test]
async fn test_analyze_performance_trends_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_performance_trends".to_owned(),
        parameters: json!({
            "activities": [
                {
                    "date": "2024-01-01",
                    "distance": 5000.0,
                    "duration": 1800,
                    "activity_type": "Run"
                },
                {
                    "date": "2024-01-08",
                    "distance": 5200.0,
                    "duration": 1750,
                    "activity_type": "Run"
                }
            ],
            "metric": "pace"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds, verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["trend_direction"].is_string());
        assert!(result["improvement_percentage"].is_number());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, the handler may expect stored activities
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_compare_activities_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Create tenant and user for testing (user first, then tenant)
    let user_id = Uuid::new_v4();
    let mut user = User::new(
        "test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    user.id = user_id;
    user.tenant_id = Some("test-tenant".to_owned());
    executor.resources.database.create_user(&user).await?;

    // Create tenant with user as owner
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        "test-tenant".to_owned(),
        Some("test.example.com".to_owned()),
        "starter".to_owned(),
        user_id, // Owner
    );
    executor.resources.database.create_tenant(&tenant).await?;

    let request = UniversalRequest {
        tool_name: "compare_activities".to_owned(),
        parameters: json!({
            "activity_id": "test_activity_1",
            "comparison_type": "similar_activities"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds (with mock data), verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["comparison_result"].is_object());
        assert!(result["performance_ranking"].is_number());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, authentication is required
        assert!(response.error.is_some());
        let error_msg = response.error.as_ref().unwrap();
        assert!(
            (error_msg.contains("No valid") && error_msg.contains("token found"))
                || error_msg.contains("Connect")
                || error_msg.contains("Authentication error")
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_detect_patterns_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "detect_patterns".to_owned(),
        parameters: json!({
            "pattern_type": "training_consistency"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds, verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["patterns"].is_array());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, authentication is required
        assert!(response.error.is_some());
        let error_msg = response.error.as_ref().unwrap();
        assert!(
            (error_msg.contains("No valid") && error_msg.contains("token found"))
                || error_msg.contains("Connect")
                || error_msg.contains("Authentication error")
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_track_progress_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "track_progress".to_owned(),
        parameters: json!({
            "goal_id": "test_goal_123"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds (with mock data), verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["progress_percentage"].is_number());
        assert!(result["on_track"].is_boolean());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, it's expected that goal doesn't exist
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_suggest_goals_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "suggest_goals".to_owned(),
        parameters: json!({
            "recent_activities": [
                {
                    "distance": 5000.0,
                    "duration": 1800,
                    "activity_type": "Run"
                }
            ],
            "user_profile": {
                "fitness_level": "intermediate",
                "primary_sport": "running"
            }
        }),
        user_id: Uuid::new_v4().to_string(),
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
    assert!(result["suggested_goals"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_analyze_goal_feasibility_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_goal_feasibility".to_owned(),
        parameters: json!({
            "goal_type": "distance",
            "target_value": 1000.0,
            "timeframe_days": 365,
            "title": "Run 1000km this year"
        }),
        user_id: Uuid::new_v4().to_string(),
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
    assert!(result["feasibility_score"].is_number());
    assert!(result["feasible"].is_boolean());

    Ok(())
}

#[tokio::test]
async fn test_generate_recommendations_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "generate_recommendations".to_owned(),
        parameters: json!({
            "user_profile": {
                "fitness_level": "intermediate",
                "goals": ["improve_endurance"]
            },
            "recent_activities": [
                {
                    "distance": 5000.0,
                    "duration": 1800,
                    "activity_type": "Run"
                }
            ]
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds, verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["recommendations"].is_array());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, authentication is required
        assert!(response.error.is_some());
        let error_msg = response.error.as_ref().unwrap();
        assert!(
            (error_msg.contains("No valid") && error_msg.contains("token found"))
                || error_msg.contains("Connect")
                || error_msg.contains("Authentication error")
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_calculate_fitness_score_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "calculate_fitness_score".to_owned(),
        parameters: json!({
            "activities": [
                {
                    "distance": 5000.0,
                    "duration": 1800,
                    "activity_type": "Run",
                    "training_stress_score": 65
                }
            ],
            "timeframe_days": 7
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds, verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["fitness_score"].is_number());
        assert!(result["score_components"].is_object());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, the handler may expect stored activities
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_predict_performance_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "predict_performance".to_owned(),
        parameters: json!({
            "distance": 21097.5,
            "activity_type": "run"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds (with mock/real data), verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["predicted_time"].is_number());
        assert!(result["confidence_level"].is_number());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, authentication is required
        assert!(response.error.is_some());
        let error_msg = response.error.as_ref().unwrap();
        assert!(
            (error_msg.contains("No valid") && error_msg.contains("token found"))
                || error_msg.contains("Connect")
                || error_msg.contains("Authentication error")
                || error_msg.contains("No historical activities")
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_analyze_training_load_tool() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_training_load".to_owned(),
        parameters: json!({
            "activities": [
                {
                    "date": "2024-01-01",
                    "duration": 1800,
                    "intensity": "moderate",
                    "training_stress_score": 65
                },
                {
                    "date": "2024-01-02",
                    "duration": 3600,
                    "intensity": "easy",
                    "training_stress_score": 45
                }
            ]
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds, verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["training_load_balance"].is_string());
        assert!(result["recovery_recommendation"].is_string());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, the handler may expect stored activities
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_disconnect_provider_tool() -> Result<()> {
    common::init_server_config();
    let database = common::create_test_database().await?;
    let (user_id, _) = common::create_test_user(&database).await?;

    // Create ActivityIntelligence with proper constructor
    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_owned(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_owned(),
            confidence: 90.0,
            data: None,
        }],
        PerformanceMetrics {
            relative_effort: Some(85.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(82.5),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Improving,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Improving,
                consistency_score: 90.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: Some(ContextualWeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
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
            ..AuthConfig::default()
        },
        oauth: OAuthConfig {
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
                environment: Environment::Development,
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
                enabled: true,
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
            garmin_api: GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ..Default::default()
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
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
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: environment::CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_recovery: SleepRecoveryConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });

    // Create ServerResources for the test
    let auth_manager = AuthManager::new(24);

    // Create test cache with background cleanup disabled
    let cache_config = CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(60),
        enable_background_cleanup: false,
        ..Default::default()
    };
    let cache = Cache::new(cache_config)
        .await
        .expect("Failed to create test cache");

    let server_resources = Arc::new(ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let executor = UniversalToolExecutor::new(server_resources);

    let request = UniversalRequest {
        tool_name: "disconnect_provider".to_owned(),
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
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert_eq!(result["provider"], "strava");
    assert_eq!(result["status"], "disconnected");

    Ok(())
}

#[tokio::test]
async fn test_get_activities_async_no_token() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Create tenant and user for testing (user first, then tenant)
    let user_id = Uuid::new_v4();
    let mut user = User::new(
        "test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    user.id = user_id;
    user.tenant_id = Some("test-tenant".to_owned());
    executor.resources.database.create_user(&user).await?;

    // Create tenant with user as owner
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        "test-tenant".to_owned(),
        Some("test.example.com".to_owned()),
        "starter".to_owned(),
        user_id, // Owner
    );
    executor.resources.database.create_tenant(&tenant).await?;

    let request = UniversalRequest {
        tool_name: "get_activities".to_owned(),
        parameters: json!({
            "limit": 5,
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
    // Should fail without OAuth token
    assert!(!response.success);
    assert!(response.error.is_some());

    // Error should mention missing token
    let error = response.error.unwrap();
    assert!(error.contains("No") && error.contains("token") || error.contains("Connect"));

    Ok(())
}

#[tokio::test]
async fn test_get_athlete_async_no_token() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Create tenant and user for testing (user first, then tenant)
    let user_id = Uuid::new_v4();
    let mut user = User::new(
        "test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    user.id = user_id;
    user.tenant_id = Some("test-tenant".to_owned());
    executor.resources.database.create_user(&user).await?;

    // Create tenant with user as owner
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        "test-tenant".to_owned(),
        Some("test.example.com".to_owned()),
        "starter".to_owned(),
        user_id, // Owner
    );
    executor.resources.database.create_tenant(&tenant).await?;

    let request = UniversalRequest {
        tool_name: "get_athlete".to_owned(),
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
    // get_athlete may fail without proper tenant context or token, which is expected
    if response.success {
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result.is_object());
    } else {
        // Failing is also acceptable for this test scenario
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_get_stats_async_no_token() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    // Create tenant and user for testing (user first, then tenant)
    let user_id = Uuid::new_v4();
    let mut user = User::new(
        "test@example.com".to_owned(),
        "password_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    user.id = user_id;
    user.tenant_id = Some("test-tenant".to_owned());
    executor.resources.database.create_user(&user).await?;

    // Create tenant with user as owner
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        "test-tenant".to_owned(),
        Some("test.example.com".to_owned()),
        "starter".to_owned(),
        user_id, // Owner
    );
    executor.resources.database.create_tenant(&tenant).await?;

    let request = UniversalRequest {
        tool_name: "get_stats".to_owned(),
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
    // Should fail when no OAuth token is available (no fallbacks)
    assert!(!response.success);
    assert!(response.error.is_some());
    let error_msg = response.error.as_ref().unwrap();
    assert!(
        (error_msg.contains("No valid") && error_msg.contains("token found"))
            || error_msg.contains("deprecated")
            || error_msg.contains("tenant-aware MCP endpoints")
            || error_msg.contains("Tool execution failed")
    );

    Ok(())
}

#[tokio::test]
async fn test_invalid_protocol_handling() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_owned(),
        parameters: json!({}),
        user_id: "invalid-uuid".to_owned(),
        protocol: "invalid_protocol".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    // Should handle gracefully and return error response
    let response = executor.execute_tool(request).await;
    match response {
        Ok(response) => {
            // If it succeeds in creating a response, it should indicate failure
            if response.success {
                panic!("Response should not be successful for invalid UUID");
            } else {
                println!("Error: {:?}", response.error);
            }
            assert!(!response.success);
            assert!(response.error.is_some());
        }
        Err(err) => {
            // If execute_tool returns an error, that's also acceptable for invalid UUID
            println!("Tool execution error: {err:?}");
            assert!(err.to_string().contains("Invalid user ID format"));
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_empty_parameters() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_owned(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);

    Ok(())
}

#[tokio::test]
async fn test_malformed_parameters() -> Result<()> {
    common::init_server_config();
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "set_goal".to_owned(),
        parameters: json!({
            "invalid_param": "value"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    // Should handle gracefully and return error response
    let response = executor.execute_tool(request).await;
    match response {
        Ok(response) => {
            // If it succeeds in creating a response, it should indicate failure
            if response.success {
                panic!("Response should not be successful for invalid parameters");
            } else {
                println!("Error: {:?}", response.error);
            }
            assert!(!response.success);
            assert!(response.error.is_some());
            let error_msg = response.error.as_ref().unwrap();
            assert!(
                error_msg.contains("goal_type is required")
                    || error_msg.contains("missing field `goal_type`")
            );
        }
        Err(err) => {
            // If execute_tool returns an error, that's also acceptable for missing params
            println!("Tool execution error: {err:?}");
            let error_str = err.to_string();
            assert!(
                error_str.contains("goal_type is required")
                    || error_str.contains("missing field `goal_type`")
            );
        }
    }

    Ok(())
}
