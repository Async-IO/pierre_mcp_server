//! End-to-End (E2E) integration tests for the configuration system
//!
//! Tests the entire configuration system flow from MCP protocol tools
//! through universal tool execution to configuration management.

use pierre_mcp_server::database_plugins::factory::Database;
use pierre_mcp_server::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators,
};
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn create_test_tool_executor() -> Arc<UniversalToolExecutor> {
    // Create in-memory database for testing
    let database = Arc::new(
        Database::new("sqlite::memory:", vec![0; 32])
            .await
            .expect("Failed to create test database"),
    );

    // Create test intelligence
    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(7.5),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(85.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Improving,
                distance_trend: TrendDirection::Stable,
                consistency_score: 88.0,
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

    // Create minimal config for testing
    let config = Arc::new(
        pierre_mcp_server::config::environment::ServerConfig::from_env()
            .unwrap_or_else(|_| create_test_config()),
    );

    Arc::new(UniversalToolExecutor::new(database, intelligence, config))
}

fn create_test_config() -> pierre_mcp_server::config::environment::ServerConfig {
    pierre_mcp_server::config::environment::ServerConfig {
        mcp_port: 3000,
        http_port: 4000,
        log_level: pierre_mcp_server::config::environment::LogLevel::Info,
        database: pierre_mcp_server::config::environment::DatabaseConfig {
            url: pierre_mcp_server::config::environment::DatabaseUrl::default(),
            encryption_key_path: std::path::PathBuf::from("data/encryption.key"),
            auto_migrate: true,
            backup: pierre_mcp_server::config::environment::BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: std::path::PathBuf::from("data/backups"),
            },
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_secret_path: std::path::PathBuf::from("data/jwt.secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec!["read".to_string()],
                enabled: false,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec!["activity".to_string()],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: pierre_mcp_server::config::environment::RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: pierre_mcp_server::config::environment::TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: pierre_mcp_server::config::environment::SecurityHeadersConfig {
                environment: pierre_mcp_server::config::environment::Environment::Development,
            },
        },
        external_services: pierre_mcp_server::config::environment::ExternalServicesConfig {
            weather: pierre_mcp_server::config::environment::WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true, // Set to CI mode for testing
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    }
}

#[tokio::test]
async fn test_get_configuration_catalog_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_configuration_catalog".to_string(),
        parameters: json!({}),
        protocol: "mcp".to_string(),
    };

    let response = executor
        .execute_tool(request)
        .await
        .expect("Tool execution should succeed");

    assert!(response.success);

    let result = response.result.expect("Should have result");

    assert!(result["catalog"].is_object());
    assert!(result["catalog"]["categories"].is_array());
    assert!(result["catalog"]["total_parameters"].is_number());
    assert!(result["catalog"]["version"].is_string());

    // Verify we have the expected parameter categories
    let categories = result["catalog"]["categories"]
        .as_array()
        .expect("Categories should be array");
    assert!(!categories.is_empty());

    // Check that we have some expected category names
    let category_names: Vec<&str> = categories
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();

    // Check that we have the expected category names based on actual implementation
    assert!(category_names.contains(&"physiological_zones"));
    assert!(category_names.contains(&"performance_calculation"));
    assert!(category_names.contains(&"sport_specific"));
    assert!(category_names.contains(&"analysis_settings"));
    assert!(category_names.contains(&"safety_constraints"));

    println!("✅ get_configuration_catalog E2E test passed");
}

#[tokio::test]
async fn test_get_configuration_profiles_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_configuration_profiles".to_string(),
        parameters: json!({}),
        protocol: "mcp".to_string(),
    };

    let response = executor
        .execute_tool(request)
        .await
        .expect("Tool execution should succeed");

    assert!(response.success);

    let result = response.result.expect("Should have result");

    assert!(result["profiles"].is_array());
    assert!(result["total_count"].is_number());

    let profiles = result["profiles"]
        .as_array()
        .expect("Profiles should be array");
    assert!(!profiles.is_empty());

    // Check that we have the expected profile types
    let profile_names: Vec<&str> = profiles
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    // Check that we have the expected profile names based on actual implementation
    assert!(profile_names.contains(&"Default"));
    assert!(profile_names.contains(&"Research"));
    assert!(profile_names.contains(&"Elite Athlete"));
    assert!(profile_names.contains(&"Recreational Athlete"));
    assert!(profile_names.contains(&"Beginner"));

    println!("✅ get_configuration_profiles E2E test passed");
}

#[tokio::test]
async fn test_calculate_personalized_zones_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "calculate_personalized_zones".to_string(),
        parameters: json!({
            "vo2_max": 55.0,
            "resting_hr": 60,
            "max_hr": 190,
            "lactate_threshold": 0.85,
            "sport_efficiency": 1.0
        }),
        protocol: "mcp".to_string(),
    };

    let response = executor
        .execute_tool(request)
        .await
        .expect("Tool execution should succeed");

    assert!(response.success);

    let result = response.result.expect("Should have result");
    assert!(result["user_profile"].is_object());
    assert!(result["personalized_zones"].is_object());
    assert!(result["zone_calculations"].is_object());

    // Verify user profile
    let user_profile = &result["user_profile"];
    assert_eq!(user_profile["vo2_max"], 55.0);
    assert_eq!(user_profile["resting_hr"], 60);
    assert_eq!(user_profile["max_hr"], 190);

    // Verify personalized zones
    let zones = &result["personalized_zones"];
    assert!(zones["heart_rate_zones"].is_object());
    assert!(zones["pace_zones"].is_object());
    assert!(zones["power_zones"].is_object());
    assert!(zones["estimated_ftp"].is_number());

    // Verify zone calculations metadata
    let calculations = &result["zone_calculations"];
    assert!(calculations["method"].is_string());
    assert!(calculations["pace_formula"].is_string());
    assert!(calculations["power_estimation"].is_string());

    println!("✅ calculate_personalized_zones E2E test passed");
}

#[tokio::test]
async fn test_validate_configuration_e2e() {
    let executor = create_test_tool_executor().await;

    // Test valid configuration
    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "validate_configuration".to_string(),
        parameters: json!({
            "parameters": {
                "fitness.vo2_max_threshold_male_recreational": 45.0,
                "heart_rate.anaerobic_threshold": 85.0,
                "heart_rate.recovery_zone": 65.0
            }
        }),
        protocol: "mcp".to_string(),
    };

    let response = executor
        .execute_tool(request)
        .await
        .expect("Tool execution should succeed");

    assert!(response.success);

    let result = response.result.expect("Should have result");

    assert!(result["validation_passed"].is_boolean());
    assert!(result["parameters_validated"].is_number());

    // Should pass validation for valid parameters
    assert_eq!(result["validation_passed"], true);

    println!("✅ validate_configuration E2E test passed");
}

#[tokio::test]
async fn test_update_user_configuration_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "update_user_configuration".to_string(),
        parameters: json!({
            "profile": "default",
            "parameters": {
                "fitness.vo2_max_threshold_male_recreational": 45.0,
                "heart_rate.anaerobic_threshold": 88.0
            }
        }),
        protocol: "mcp".to_string(),
    };

    let response = executor
        .execute_tool(request)
        .await
        .expect("Tool execution should succeed");

    assert!(response.success);

    let result = response.result.expect("Should have result");
    assert!(result["user_id"].is_string());
    assert!(result["updated_configuration"].is_object());
    assert!(result["changes_applied"].is_number());

    let updated_config = &result["updated_configuration"];
    assert_eq!(updated_config["active_profile"], "default");
    assert!(updated_config["applied_overrides"].is_number());
    assert!(updated_config["last_modified"].is_string());

    // Should have applied both profile change and parameter overrides
    let changes_applied = result["changes_applied"].as_u64().unwrap();
    assert!(changes_applied >= 1); // At least the profile change

    println!("✅ update_user_configuration E2E test passed");
}

#[tokio::test]
async fn test_get_user_configuration_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_user_configuration".to_string(),
        parameters: json!({}),
        protocol: "mcp".to_string(),
    };

    let response = executor
        .execute_tool(request)
        .await
        .expect("Tool execution should succeed");

    assert!(response.success);

    let result = response.result.expect("Should have result");
    assert!(result["user_id"].is_string());
    assert!(result["active_profile"].is_string());
    assert!(result["configuration"].is_object());
    assert!(result["available_parameters"].is_number());

    let configuration = &result["configuration"];
    assert!(configuration["profile"].is_object());
    assert!(configuration["session_overrides"].is_object());
    assert!(configuration["last_modified"].is_string());

    // Should have Default profile by default
    assert_eq!(result["active_profile"], "default");

    println!("✅ get_user_configuration E2E test passed");
}

#[tokio::test]
async fn test_configuration_tools_via_different_protocols() {
    let executor = create_test_tool_executor().await;

    // Test same tool via MCP protocol
    let test_user_id = Uuid::new_v4().to_string();
    let mcp_request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "get_configuration_catalog".to_string(),
        parameters: json!({}),
        protocol: "mcp".to_string(),
    };

    let mcp_response = executor
        .execute_tool(mcp_request)
        .await
        .expect("MCP tool execution should succeed");
    assert!(mcp_response.success);

    // Test same tool via A2A protocol
    let a2a_request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_configuration_catalog".to_string(),
        parameters: json!({}),
        protocol: "a2a".to_string(),
    };

    let a2a_response = executor
        .execute_tool(a2a_request)
        .await
        .expect("A2A tool execution should succeed");
    assert!(a2a_response.success);

    // Both protocols should return similar structured results
    let mcp_result = mcp_response.result.expect("MCP should have result");
    let a2a_result = a2a_response.result.expect("A2A should have result");

    assert_eq!(
        mcp_result["catalog"]["total_parameters"],
        a2a_result["catalog"]["total_parameters"]
    );

    println!("✅ Configuration tools work via both MCP and A2A protocols");
}

#[tokio::test]
async fn test_configuration_system_error_handling() {
    let executor = create_test_tool_executor().await;

    // Test invalid tool name
    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "invalid_configuration_tool".to_string(),
        parameters: json!({}),
        protocol: "mcp".to_string(),
    };

    let response = executor.execute_tool(request).await;
    assert!(response.is_err());

    // Test missing required parameter
    let invalid_request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "calculate_personalized_zones".to_string(),
        parameters: json!({}), // Missing required vo2_max
        protocol: "mcp".to_string(),
    };

    let invalid_response = executor.execute_tool(invalid_request).await;
    assert!(invalid_response.is_err());

    // Test invalid validation parameters
    let validation_request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "validate_configuration".to_string(),
        parameters: json!({
            "parameters": {
                "invalid.parameter.name": "invalid_value"
            }
        }),
        protocol: "mcp".to_string(),
    };

    let validation_response = executor
        .execute_tool(validation_request)
        .await
        .expect("Should execute but find validation errors");
    assert!(validation_response.success); // Tool executes successfully

    let result = validation_response.result.expect("Should have result");
    assert_eq!(result["validation_passed"], false); // But validation should fail

    println!("✅ Configuration system error handling works correctly");
}
