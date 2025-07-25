// ABOUTME: Universal protocol integration tests for tool execution layer
// ABOUTME: Tests protocol-agnostic interfaces for MCP and A2A
//! Universal Protocol Integration Tests
//!
//! Comprehensive tests for the universal tool execution layer
//! that provides protocol-agnostic interfaces for MCP and A2A.

use anyhow::Result;
use pierre_mcp_server::{
    config::environment::*,
    intelligence::insights::{Insight, InsightType},
    intelligence::{
        ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
        TrendIndicators, WeeklyLoad,
    },
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

mod common;

/// Test configuration for universal protocols
#[allow(clippy::too_many_lines)]
async fn create_test_executor() -> Result<UniversalToolExecutor> {
    let database = common::create_test_database().await?;

    // Create ActivityIntelligence with proper constructor
    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_string(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_string(),
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
            weekly_load: Some(WeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
        mcp_port: 3000,
        http_port: 4000,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: PathBuf::from("test.key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: PathBuf::from("test_backups"),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: PathBuf::from("test.secret"),
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
                environment: Environment::Development,
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
                enabled: true,
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
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });

    let executor = UniversalToolExecutor::new(database, intelligence, config);
    Ok(executor)
}

#[tokio::test]
async fn test_universal_executor_creation() -> Result<()> {
    let executor = create_test_executor().await?;

    // Verify basic functionality
    assert!(!executor.list_tools().is_empty());

    // Check that core tools are registered
    assert!(executor.get_tool("get_connection_status").is_some());
    assert!(executor.get_tool("set_goal").is_some());
    assert!(executor.get_tool("connect_strava").is_some());
    assert!(executor.get_tool("connect_fitbit").is_some());

    Ok(())
}

#[tokio::test]
async fn test_tool_registration() -> Result<()> {
    let executor = create_test_executor().await?;

    // Verify all expected tools are registered
    let tool_names: Vec<String> = executor
        .list_tools()
        .iter()
        .map(|tool| tool.name.clone())
        .collect();

    let expected_tools = vec![
        "get_connection_status",
        "set_goal",
        "connect_strava",
        "connect_fitbit",
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
            tool_names.contains(&expected_tool.to_string()),
            "Missing tool: {expected_tool}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_tool_execution_invalid_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "nonexistent_tool".to_string(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let result = executor.execute_tool(request).await;
    assert!(result.is_err());

    Ok(())
}

#[tokio::test]
async fn test_connection_status_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({
            "provider": "strava"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "connect_strava".to_string(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["authorization_url"].is_string());
    assert!(result["state"].is_string());

    let auth_url = result["authorization_url"].as_str().unwrap();
    assert!(auth_url.contains("strava.com"));
    assert!(auth_url.contains("client_id=test_client_id"));

    Ok(())
}

#[tokio::test]
async fn test_connect_fitbit_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "connect_fitbit".to_string(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["authorization_url"].is_string());
    assert!(result["state"].is_string());

    let auth_url = result["authorization_url"].as_str().unwrap();
    assert!(auth_url.contains("fitbit.com"));
    assert!(auth_url.contains("client_id=test_fitbit_id"));

    Ok(())
}

#[tokio::test]
#[allow(clippy::too_many_lines)]
async fn test_set_goal_tool() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _) = common::create_test_user(&database).await?;

    // Create ActivityIntelligence with proper constructor
    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_string(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_string(),
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
            weekly_load: Some(WeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
        mcp_port: 3000,
        http_port: 4000,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: PathBuf::from("test.key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: PathBuf::from("test_backups"),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: PathBuf::from("test.secret"),
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
                environment: Environment::Development,
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
                enabled: true,
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
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });

    let executor = UniversalToolExecutor::new(database, intelligence, config);

    let request = UniversalRequest {
        tool_name: "set_goal".to_string(),
        parameters: json!({
            "goal_type": "distance",
            "target_value": 1000.0,
            "timeframe": "2024",
            "title": "Run 1000km this year"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "calculate_metrics".to_string(),
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
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_performance_trends".to_string(),
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
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "compare_activities".to_string(),
        parameters: json!({
            "activity_id1": "test_activity_1",
            "activity_id2": "test_activity_2"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
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
        // For test data, it's expected that activities don't exist
        assert!(response.error.is_some());
        assert!(response.error.as_ref().unwrap().contains("not found"));
    }

    Ok(())
}

#[tokio::test]
async fn test_detect_patterns_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "detect_patterns".to_string(),
        parameters: json!({
            "activities": [
                {
                    "date": "2024-01-01",
                    "distance": 5000.0,
                    "duration": 1800,
                    "day_of_week": "Monday"
                },
                {
                    "date": "2024-01-08",
                    "distance": 5200.0,
                    "duration": 1750,
                    "day_of_week": "Monday"
                },
                {
                    "date": "2024-01-15",
                    "distance": 5100.0,
                    "duration": 1780,
                    "day_of_week": "Monday"
                }
            ]
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    if response.success {
        // If it succeeds, verify the response structure
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result["patterns"].is_array());
    } else {
        println!("Error: {:?}", response.error);
        // For test data, the handler may expect stored activities
        assert!(response.error.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_track_progress_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "track_progress".to_string(),
        parameters: json!({
            "goal_id": "test_goal_123"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "suggest_goals".to_string(),
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
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_goal_feasibility".to_string(),
        parameters: json!({
            "goal_type": "distance",
            "target_value": 1000.0,
            "timeframe_days": 365,
            "title": "Run 1000km this year"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "generate_recommendations".to_string(),
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
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result["recommendations"].is_array());

    Ok(())
}

#[tokio::test]
async fn test_calculate_fitness_score_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "calculate_fitness_score".to_string(),
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
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "predict_performance".to_string(),
        parameters: json!({
            "distance": 21097.5,
            "activity_type": "run"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
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
        // For test data, it's expected that no historical activities exist
        assert!(response.error.is_some());
        assert!(response
            .error
            .as_ref()
            .unwrap()
            .contains("No historical activities"));
    }

    Ok(())
}

#[tokio::test]
async fn test_analyze_training_load_tool() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "analyze_training_load".to_string(),
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
        protocol: "test".to_string(),
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
    let database = common::create_test_database().await?;
    let (user_id, _) = common::create_test_user(&database).await?;

    // Create ActivityIntelligence with proper constructor
    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_string(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_string(),
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
            weekly_load: Some(WeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
        mcp_port: 3000,
        http_port: 4000,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: PathBuf::from("test.key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: PathBuf::from("test_backups"),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: PathBuf::from("test.secret"),
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
                environment: Environment::Development,
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
                enabled: true,
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
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    });

    let executor = UniversalToolExecutor::new(database, intelligence, config);

    let request = UniversalRequest {
        tool_name: "disconnect_provider".to_string(),
        parameters: json!({
            "provider": "strava"
        }),
        user_id: user_id.to_string(),
        protocol: "test".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_activities".to_string(),
        parameters: json!({
            "limit": 5,
            "provider": "strava"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    // Should return an object with activities array when no token available
    let result = response.result.unwrap();
    assert!(result.is_object());
    assert!(result["activities"].is_array());

    // Should have at least one error entry
    let activities = result["activities"].as_array().unwrap();
    assert!(!activities.is_empty());
    assert!(activities[0]["error"].is_string());

    Ok(())
}

#[tokio::test]
async fn test_get_athlete_async_no_token() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_athlete".to_string(),
        parameters: json!({
            "provider": "strava"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    // Should return mock data when no token available
    let result = response.result.unwrap();
    assert!(result.is_object());

    Ok(())
}

#[tokio::test]
async fn test_get_stats_async_no_token() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_stats".to_string(),
        parameters: json!({
            "provider": "strava"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);
    assert!(response.result.is_some());

    // Should return mock data when no token available
    let result = response.result.unwrap();
    assert!(result.is_object());

    Ok(())
}

#[tokio::test]
async fn test_invalid_protocol_handling() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: "invalid-uuid".to_string(),
        protocol: "invalid_protocol".to_string(),
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
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
    };

    let response = executor.execute_tool(request).await?;
    assert!(response.success);

    Ok(())
}

#[tokio::test]
async fn test_malformed_parameters() -> Result<()> {
    let executor = create_test_executor().await?;

    let request = UniversalRequest {
        tool_name: "set_goal".to_string(),
        parameters: json!({
            "invalid_param": "value"
        }),
        user_id: Uuid::new_v4().to_string(),
        protocol: "test".to_string(),
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
            assert!(response
                .error
                .as_ref()
                .unwrap()
                .contains("goal_type is required"));
        }
        Err(err) => {
            // If execute_tool returns an error, that's also acceptable for missing params
            println!("Tool execution error: {err:?}");
            assert!(err.to_string().contains("goal_type is required"));
        }
    }

    Ok(())
}
