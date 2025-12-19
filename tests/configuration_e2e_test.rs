// ABOUTME: End-to-end integration tests for configuration system
// ABOUTME: Tests MCP protocol tools through universal execution to config management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! End-to-End (E2E) integration tests for the configuration system
//!
//! Tests the entire configuration system flow from MCP protocol tools
//! through universal tool execution to configuration management.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use pierre_mcp_server::auth::AuthManager;
use pierre_mcp_server::cache::{factory::Cache, CacheConfig};
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::config::environment::{
    AppBehaviorConfig, BackupConfig, DatabaseConfig, ServerConfig,
};
use pierre_mcp_server::database_plugins::factory::Database;
use pierre_mcp_server::database_plugins::DatabaseProvider;
use pierre_mcp_server::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators,
};
use pierre_mcp_server::mcp::resources::ServerResources;
use pierre_mcp_server::models::User;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

async fn create_test_tool_executor() -> Arc<UniversalToolExecutor> {
    let (executor, _) = create_test_tool_executor_with_user().await;
    executor
}

/// Creates a test executor with a valid user in the database.
/// Returns (executor, `user_id`) where `user_id` can be used for tests that write to the database.
async fn create_test_tool_executor_with_user() -> (Arc<UniversalToolExecutor>, String) {
    // Initialize server config for tests
    common::init_server_config();

    // Create in-memory database for testing
    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            vec![0; 32],
            &PostgresPoolConfig::default(),
        )
        .await
        .expect("Failed to create test database"),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(
        Database::new("sqlite::memory:", vec![0; 32])
            .await
            .expect("Failed to create test database"),
    );

    // Create test intelligence
    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_owned(),
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
    let config = Arc::new(ServerConfig::from_env().unwrap_or_else(|_| create_test_config()));

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

    // Create a test user in the database for FK constraint compliance
    let test_user = User::new(
        "config_test@example.com".to_owned(),
        "hashed_password".to_owned(),
        Some("Config Test User".to_owned()),
    );
    let user_id = database
        .create_user(&test_user)
        .await
        .expect("Failed to create test user");

    let server_resources = Arc::new(
        ServerResources::new(
            (*database).clone(),
            auth_manager,
            "test_secret",
            config,
            cache,
            2048, // Use 2048-bit RSA keys for faster test execution
            Some(common::get_shared_test_jwks()),
        )
        .await,
    );
    (
        Arc::new(UniversalToolExecutor::new(server_resources)),
        user_id.to_string(),
    )
}

fn create_test_config() -> ServerConfig {
    ServerConfig {
        http_port: 4000,
        database: DatabaseConfig {
            backup: BackupConfig {
                directory: PathBuf::from("data/backups"),
                ..Default::default()
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            ci_mode: true,
            auto_approve_users: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[tokio::test]
async fn test_get_configuration_catalog_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_configuration_catalog".to_owned(),
        parameters: json!({}),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("get_configuration_catalog E2E test passed");
}

#[tokio::test]
async fn test_get_configuration_profiles_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_configuration_profiles".to_owned(),
        parameters: json!({}),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("get_configuration_profiles E2E test passed");
}

#[tokio::test]
async fn test_calculate_personalized_zones_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "calculate_personalized_zones".to_owned(),
        parameters: json!({
            "vo2_max": 55.0,
            "resting_hr": 60,
            "max_hr": 190,
            "lactate_threshold": 0.85,
            "sport_efficiency": 1.0
        }),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("calculate_personalized_zones E2E test passed");
}

#[tokio::test]
async fn test_validate_configuration_e2e() {
    let executor = create_test_tool_executor().await;

    // Test valid configuration
    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "validate_configuration".to_owned(),
        parameters: json!({
            "parameters": {
                "fitness.vo2_max_threshold_male_recreational": 45.0,
                "heart_rate.anaerobic_threshold": 85.0,
                "heart_rate.recovery_zone": 65.0
            }
        }),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("validate_configuration E2E test passed");
}

#[tokio::test]
async fn test_update_user_configuration_e2e() {
    // Use executor with a valid user to satisfy FK constraint on user_configurations.user_id
    let (executor, test_user_id) = create_test_tool_executor_with_user().await;

    let request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "update_user_configuration".to_owned(),
        parameters: json!({
            "profile": "default",
            "parameters": {
                "fitness.vo2_max_threshold_male_recreational": 45.0,
                "heart_rate.anaerobic_threshold": 88.0
            }
        }),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("update_user_configuration E2E test passed");
}

#[tokio::test]
async fn test_get_user_configuration_e2e() {
    let executor = create_test_tool_executor().await;

    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_user_configuration".to_owned(),
        parameters: json!({}),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("get_user_configuration E2E test passed");
}

#[tokio::test]
async fn test_configuration_tools_via_different_protocols() {
    let executor = create_test_tool_executor().await;

    // Test same tool via MCP protocol
    let test_user_id = Uuid::new_v4().to_string();
    let mcp_request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "get_configuration_catalog".to_owned(),
        parameters: json!({}),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let mcp_response = executor
        .execute_tool(mcp_request)
        .await
        .expect("MCP tool execution should succeed");
    assert!(mcp_response.success);

    // Test same tool via A2A protocol
    let a2a_request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "get_configuration_catalog".to_owned(),
        parameters: json!({}),
        protocol: "a2a".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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

    println!("Configuration tools work via both MCP and A2A protocols");
}

#[tokio::test]
async fn test_configuration_system_error_handling() {
    let executor = create_test_tool_executor().await;

    // Test invalid tool name
    let test_user_id = Uuid::new_v4().to_string();
    let request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "invalid_configuration_tool".to_owned(),
        parameters: json!({}),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let response = executor.execute_tool(request).await;
    assert!(response.is_err());

    // Test missing required parameter
    let invalid_request = UniversalRequest {
        user_id: test_user_id.clone(),
        tool_name: "calculate_personalized_zones".to_owned(),
        parameters: json!({}), // Missing required vo2_max
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let invalid_response = executor.execute_tool(invalid_request).await;
    assert!(invalid_response.is_err());

    // Test invalid validation parameters
    let validation_request = UniversalRequest {
        user_id: test_user_id,
        tool_name: "validate_configuration".to_owned(),
        parameters: json!({
            "parameters": {
                "invalid.parameter.name": "invalid_value"
            }
        }),
        protocol: "mcp".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let validation_response = executor
        .execute_tool(validation_request)
        .await
        .expect("Should execute but find validation errors");
    assert!(validation_response.success); // Tool executes successfully

    let result = validation_response.result.expect("Should have result");
    assert_eq!(result["validation_passed"], false); // But validation should fail

    println!("Configuration system error handling works correctly");
}
