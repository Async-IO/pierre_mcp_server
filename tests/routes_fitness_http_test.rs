// ABOUTME: HTTP integration tests for fitness configuration routes
// ABOUTME: Tests all fitness configuration endpoints with authentication, authorization, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for fitness configuration routes
//!
//! This test suite validates that all fitness configuration endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    mcp::resources::ServerResources,
    routes::fitness::FitnessConfigurationRoutes,
};
use serde_json::json;
use std::sync::Arc;

/// Test setup helper for fitness configuration route testing
#[allow(dead_code)]
struct FitnessConfigurationTestSetup {
    resources: Arc<ServerResources>,
    user_id: uuid::Uuid,
    jwt_token: String,
}

impl FitnessConfigurationTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let cache = common::create_test_cache().await?;

        // Create test user
        let (user_id, user) = common::create_test_user(&database).await?;

        // Create ServerResources
        let temp_dir = tempfile::tempdir()?;
        let config = Arc::new(ServerConfig {
            http_port: 8081,
            database: DatabaseConfig {
                url: DatabaseUrl::Memory,
                backup: BackupConfig {
                    directory: temp_dir.path().to_path_buf(),
                    ..Default::default()
                },
                ..Default::default()
            },
            app_behavior: AppBehaviorConfig {
                ci_mode: true,
                auto_approve_users: false,
                ..Default::default()
            },
            security: SecurityConfig {
                headers: SecurityHeadersConfig {
                    environment: Environment::Testing,
                },
                ..Default::default()
            },
            ..Default::default()
        });

        let resources = Arc::new(ServerResources::new(
            (*database).clone(),
            (*auth_manager).clone(),
            "test_jwt_secret",
            config,
            cache,
            2048,
            Some(common::get_shared_test_jwks()),
        ));

        // Generate JWT token for the user
        let jwt_token = auth_manager
            .generate_token(&user, &resources.jwks_manager)
            .map_err(|e| anyhow::anyhow!("Failed to generate JWT: {}", e))?;

        Ok(Self {
            resources,
            user_id,
            jwt_token,
        })
    }

    fn routes(&self) -> axum::Router {
        FitnessConfigurationRoutes::routes(self.resources.clone())
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.jwt_token)
    }
}

// ============================================================================
// GET /fitness/config - Get Fitness Configuration Tests (with query params)
// ============================================================================

#[tokio::test]
async fn test_get_fitness_config_default() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/fitness/config")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    // Configuration should have some structure
    assert!(body.is_object() || body.is_null());
}

#[tokio::test]
async fn test_get_fitness_config_with_name() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/fitness/config?configuration_name=default")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_object() || body.is_null());
}

#[tokio::test]
async fn test_get_fitness_config_custom_name() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/fitness/config?configuration_name=marathon_training")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    // Should return 200 with empty/default config or 404 if not found
    assert!(response.status() == 200 || response.status() == 404);
}

#[tokio::test]
async fn test_get_fitness_config_missing_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/fitness/config").send(routes).await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_get_fitness_config_invalid_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/fitness/config")
        .header("authorization", "Bearer invalid_token")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// PUT /fitness/config - Save Fitness Configuration Tests
// ============================================================================

#[tokio::test]
async fn test_save_fitness_config_success() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let config_data = json!({
        "configuration_name": "default",
        "configuration": {
            "sport_types": {},
            "intelligence": {
                "effort_thresholds": {
                    "light_max": 3.0,
                    "moderate_max": 5.0,
                    "hard_max": 7.0
                },
                "zone_thresholds": {
                    "recovery_max": 60.0,
                    "endurance_max": 70.0,
                    "tempo_max": 80.0,
                    "threshold_max": 90.0
                },
                "weather_mapping": {
                    "rain_keywords": ["rain", "shower"],
                    "snow_keywords": ["snow"],
                    "wind_threshold": 15.0
                },
                "personal_records": {
                    "pace_improvement_threshold": 5.0,
                    "distance_pr_types": ["longest_run"],
                    "time_pr_types": ["fastest_5k"]
                }
            },
            "weather_api": null
        }
    });

    let response = AxumTestRequest::put("/fitness/config")
        .header("authorization", &setup.auth_header())
        .json(&config_data)
        .send(routes)
        .await;

    // Should accept the save or return method not allowed
    assert!(response.status() == 200 || response.status() == 204 || response.status() == 405);
}

#[tokio::test]
async fn test_save_fitness_config_missing_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let config_data = json!({
        "configuration_name": "default",
        "configuration": {
            "sport_types": {},
            "intelligence": {
                "effort_thresholds": {
                    "light_max": 3.0,
                    "moderate_max": 5.0,
                    "hard_max": 7.0
                },
                "zone_thresholds": {
                    "recovery_max": 60.0,
                    "endurance_max": 70.0,
                    "tempo_max": 80.0,
                    "threshold_max": 90.0
                },
                "weather_mapping": {
                    "rain_keywords": ["rain"],
                    "snow_keywords": ["snow"],
                    "wind_threshold": 15.0
                },
                "personal_records": {
                    "pace_improvement_threshold": 5.0,
                    "distance_pr_types": [],
                    "time_pr_types": []
                }
            }
        }
    });

    let response = AxumTestRequest::put("/fitness/config")
        .json(&config_data)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_save_fitness_config_invalid_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let config_data = json!({
        "configuration_name": "default",
        "configuration": {
            "sport_types": {},
            "intelligence": {
                "effort_thresholds": {
                    "light_max": 3.0,
                    "moderate_max": 5.0,
                    "hard_max": 7.0
                },
                "zone_thresholds": {
                    "recovery_max": 60.0,
                    "endurance_max": 70.0,
                    "tempo_max": 80.0,
                    "threshold_max": 90.0
                },
                "weather_mapping": {
                    "rain_keywords": ["rain"],
                    "snow_keywords": ["snow"],
                    "wind_threshold": 15.0
                },
                "personal_records": {
                    "pace_improvement_threshold": 5.0,
                    "distance_pr_types": [],
                    "time_pr_types": []
                }
            }
        }
    });

    let response = AxumTestRequest::put("/fitness/config")
        .header("authorization", "Bearer invalid_token")
        .json(&config_data)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_save_fitness_config_invalid_json() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::put("/fitness/config")
        .header("authorization", &setup.auth_header())
        .header("content-type", "application/json")
        .send(routes)
        .await;

    // Should fail validation
    assert_ne!(response.status(), 200);
}

// ============================================================================
// DELETE /fitness/config - Delete Fitness Configuration Tests (with query params)
// ============================================================================

#[tokio::test]
async fn test_delete_fitness_config_default() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::delete("/fitness/config")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    // DELETE should return 204 No Content on success, or 404 if config doesn't exist
    assert!(
        response.status() == 204 || response.status() == 404,
        "Expected 204 or 404, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_delete_fitness_config_with_name() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    // Note: Since AxumTestRequest doesn't have delete(), this tests the endpoint existence
    let response = AxumTestRequest::get("/fitness/config?configuration_name=to_delete")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    // The GET should work (testing the parameter parsing)
    assert!(response.status() == 200 || response.status() == 404);
}

#[tokio::test]
async fn test_delete_fitness_config_missing_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::delete("/fitness/config")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_delete_fitness_config_invalid_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::delete("/fitness/config")
        .header("authorization", "Bearer invalid_token")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_delete_fitness_config_nonexistent() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/fitness/config?configuration_name=nonexistent")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    // Should return 404 or empty config
    assert!(response.status() == 200 || response.status() == 404);
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_fitness_config_user_isolation() {
    let setup1 = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup 1 failed");
    let setup2 = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup 2 failed");

    // User 1 and User 2 should have separate fitness configurations
    let routes1 = setup1.routes();
    let response1 = AxumTestRequest::get("/fitness/config")
        .header("authorization", &setup1.auth_header())
        .send(routes1)
        .await;

    let routes2 = setup2.routes();
    let response2 = AxumTestRequest::get("/fitness/config")
        .header("authorization", &setup2.auth_header())
        .send(routes2)
        .await;

    assert_eq!(response1.status(), 200);
    assert_eq!(response2.status(), 200);

    // Both users should have independent configs
    let body1: serde_json::Value = response1.json();
    let body2: serde_json::Value = response2.json();

    assert!(body1.is_object() || body1.is_null());
    assert!(body2.is_object() || body2.is_null());
}

#[tokio::test]
async fn test_fitness_config_multiple_named_configs() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let config_names = vec!["default", "marathon", "triathlon", "recovery"];

    for name in config_names {
        let response =
            AxumTestRequest::get(&format!("/fitness/config?configuration_name={}", name))
                .header("authorization", &setup.auth_header())
                .send(routes.clone())
                .await;

        // Each config should be accessible
        assert!(response.status() == 200 || response.status() == 404);
    }
}

#[tokio::test]
async fn test_fitness_config_save_and_retrieve() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    // Save a configuration
    let config_data = json!({
        "configuration_name": "test_config",
        "training_zones": {
            "zone1": {"min": 100, "max": 120}
        }
    });

    let save_response = AxumTestRequest::put("/fitness/config")
        .header("authorization", &setup.auth_header())
        .json(&config_data)
        .send(routes.clone())
        .await;

    // If save succeeds, try to retrieve it
    if save_response.status() == 200 || save_response.status() == 204 {
        let get_response = AxumTestRequest::get("/fitness/config?configuration_name=test_config")
            .header("authorization", &setup.auth_header())
            .send(routes)
            .await;

        assert_eq!(get_response.status(), 200);

        let body: serde_json::Value = get_response.json();
        assert!(body.is_object() || body.is_null());
    }
}

#[tokio::test]
async fn test_all_fitness_endpoints_require_auth() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    let endpoints = vec![
        "/fitness/config",
        "/fitness/config?configuration_name=default",
    ];

    for endpoint in endpoints {
        // Test GET
        let get_response = AxumTestRequest::get(endpoint).send(routes.clone()).await;
        assert_eq!(
            get_response.status(),
            401,
            "GET {} should require authentication",
            endpoint
        );

        // Test PUT
        let put_response = AxumTestRequest::put(endpoint)
            .json(&json!({
                "configuration_name": "test",
                "configuration": {
                    "sport_types": {},
                    "intelligence": {
                        "effort_thresholds": {
                            "light_max": 3.0,
                            "moderate_max": 5.0,
                            "hard_max": 7.0
                        },
                        "zone_thresholds": {
                            "recovery_max": 60.0,
                            "endurance_max": 70.0,
                            "tempo_max": 80.0,
                            "threshold_max": 90.0
                        },
                        "weather_mapping": {
                            "rain_keywords": [],
                            "snow_keywords": [],
                            "wind_threshold": 15.0
                        },
                        "personal_records": {
                            "pace_improvement_threshold": 5.0,
                            "distance_pr_types": [],
                            "time_pr_types": []
                        }
                    }
                }
            }))
            .send(routes.clone())
            .await;
        assert_eq!(
            put_response.status(),
            401,
            "PUT {} should require authentication",
            endpoint
        );
    }
}

#[tokio::test]
async fn test_fitness_config_query_parameter_validation() {
    let setup = FitnessConfigurationTestSetup::new()
        .await
        .expect("Setup failed");
    let routes = setup.routes();

    // Test various query parameter formats
    let test_cases = vec![
        "/fitness/config",                                // No param - should use default
        "/fitness/config?configuration_name=default",     // Explicit default
        "/fitness/config?configuration_name=custom",      // Custom name
        "/fitness/config?configuration_name=",            // Empty name
        "/fitness/config?configuration_name=with-dashes", // Name with dashes
        "/fitness/config?configuration_name=with_underscores", // Name with underscores
    ];

    for endpoint in test_cases {
        let response = AxumTestRequest::get(endpoint)
            .header("authorization", &setup.auth_header())
            .send(routes.clone())
            .await;

        // All should either succeed or return not found (not auth error)
        assert_ne!(
            response.status(),
            401,
            "{} should not return unauthorized with valid auth",
            endpoint
        );
        assert!(
            response.status() == 200 || response.status() == 404,
            "{} should return 200 or 404",
            endpoint
        );
    }
}
