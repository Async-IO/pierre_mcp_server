// ABOUTME: HTTP integration tests for dashboard routes
// ABOUTME: Tests all dashboard endpoints with authentication, authorization, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for dashboard routes
//!
//! This test suite validates that all dashboard endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    mcp::resources::{ServerResources, ServerResourcesOptions},
    routes::dashboard::DashboardRoutes,
};
use std::sync::Arc;

/// Test setup helper for dashboard route testing
struct DashboardTestSetup {
    resources: Arc<ServerResources>,
    user_id: uuid::Uuid,
    jwt_token: String,
}

impl DashboardTestSetup {
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

        let resources = Arc::new(
            ServerResources::new(
                (*database).clone(),
                (*auth_manager).clone(),
                "test_jwt_secret",
                config,
                cache,
                ServerResourcesOptions {
                    rsa_key_size_bits: Some(2048),
                    jwks_manager: Some(common::get_shared_test_jwks()),
                    llm_provider: None,
                },
            )
            .await,
        );

        // Generate JWT token for the user
        let jwt_token = auth_manager
            .generate_token(&user, &resources.jwks_manager)
            .map_err(|e| anyhow::anyhow!("Failed to generate JWT: {}", e))?;

        // Create test API keys for dashboard data
        let _ =
            common::create_and_store_test_api_key(database.as_ref(), user_id, "Test Dashboard Key")
                .await;

        Ok(Self {
            resources,
            user_id,
            jwt_token,
        })
    }

    fn routes(&self) -> axum::Router {
        DashboardRoutes::routes(self.resources.clone())
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.jwt_token)
    }
}

// ============================================================================
// GET /dashboard/status - Dashboard Overview Tests
// ============================================================================

#[tokio::test]
async fn test_dashboard_status_success() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/status")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["total_api_keys"].is_number());
    assert!(body["active_api_keys"].is_number());
    assert!(body["total_requests_today"].is_number());
    assert!(body["total_requests_this_month"].is_number());
    assert!(body["current_month_usage_by_tier"].is_array());
    assert!(body["recent_activity"].is_array());
}

#[tokio::test]
async fn test_dashboard_status_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/status").send(routes).await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_dashboard_status_invalid_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/status")
        .header("authorization", "Bearer invalid_token")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// GET /dashboard/user - User Dashboard Tests
// ============================================================================

#[tokio::test]
async fn test_dashboard_user_success() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/user")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["total_api_keys"].is_number());
}

#[tokio::test]
async fn test_dashboard_user_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/user").send(routes).await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_dashboard_user_invalid_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/user")
        .header("authorization", "Bearer invalid_token")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// GET /dashboard/admin - Admin Dashboard Tests
// ============================================================================

#[tokio::test]
async fn test_dashboard_admin_success() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/admin")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["total_api_keys"].is_number());
}

#[tokio::test]
async fn test_dashboard_admin_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/admin").send(routes).await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// GET /dashboard/detailed - Detailed Stats Tests
// ============================================================================

#[tokio::test]
async fn test_dashboard_detailed_success() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/detailed")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["total_requests"].is_number());
    assert!(body["successful_requests"].is_number());
    assert!(body["failed_requests"].is_number());
    assert!(body["average_response_time"].is_number());
}

#[tokio::test]
async fn test_dashboard_detailed_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/detailed")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// GET /dashboard/usage - Usage Analytics Tests (with query params)
// ============================================================================

#[tokio::test]
async fn test_dashboard_usage_default_days() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/usage")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["time_series"].is_array());
    assert!(body["error_rate"].is_number());
    assert!(body["average_response_time"].is_number());
}

#[tokio::test]
async fn test_dashboard_usage_with_days_param() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/usage?days=7")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["time_series"].is_array());

    // Should have 7 days of data
    let time_series = body["time_series"].as_array().unwrap();
    assert_eq!(time_series.len(), 7);
}

#[tokio::test]
async fn test_dashboard_usage_different_timeframes() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    for days in [1, 7, 14, 30, 90] {
        let response = AxumTestRequest::get(&format!("/dashboard/usage?days={}", days))
            .header("authorization", &setup.auth_header())
            .send(routes.clone())
            .await;

        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json();
        let time_series = body["time_series"].as_array().unwrap();
        assert_eq!(time_series.len(), days);
    }
}

#[tokio::test]
async fn test_dashboard_usage_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/usage?days=7")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// GET /dashboard/rate-limits - Rate Limits Overview Tests
// ============================================================================

#[tokio::test]
async fn test_dashboard_rate_limits_success() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/rate-limits")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());

    // Verify rate limit info structure
    if let Some(first_limit) = body.as_array().and_then(|arr| arr.first()) {
        assert!(first_limit["api_key_id"].is_string());
        assert!(first_limit["api_key_name"].is_string());
        assert!(first_limit["tier"].is_string());
        assert!(first_limit["current_usage"].is_number());
        assert!(first_limit["usage_percentage"].is_number());
    }
}

#[tokio::test]
async fn test_dashboard_rate_limits_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/rate-limits")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

// ============================================================================
// GET /dashboard/logs - Request Logs Tests (with query params)
// ============================================================================

#[tokio::test]
async fn test_dashboard_logs_no_filter() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/logs")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());
}

#[tokio::test]
async fn test_dashboard_logs_with_api_key_filter() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");

    // Create an API key
    let key = common::create_and_store_test_api_key(
        setup.resources.database.as_ref(),
        setup.user_id,
        "Key for Logs",
    )
    .await
    .expect("Failed to create test key");

    let routes = setup.routes();

    let response = AxumTestRequest::get(&format!("/dashboard/logs?api_key={}", key.id))
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());

    // All logs should be for the specified API key
    for log in body.as_array().unwrap() {
        if log["api_key_id"].is_string() {
            assert_eq!(log["api_key_id"].as_str().unwrap(), key.id);
        }
    }
}

#[tokio::test]
async fn test_dashboard_logs_missing_auth() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/logs?api_key=test_key")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_dashboard_logs_invalid_api_key() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/dashboard/logs?api_key=nonexistent_key_id")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    // Should return empty array or error
    assert!(response.status() == 200 || response.status() == 404);
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_dashboard_user_isolation() {
    let setup1 = DashboardTestSetup::new().await.expect("Setup 1 failed");
    let setup2 = DashboardTestSetup::new().await.expect("Setup 2 failed");

    // User 1 creates API keys
    let _ = common::create_and_store_test_api_key(
        setup1.resources.database.as_ref(),
        setup1.user_id,
        "User 1 Key",
    )
    .await
    .expect("Failed to create key for user 1");

    // User 2 views dashboard - should only see their data
    let routes2 = setup2.routes();
    let response = AxumTestRequest::get("/dashboard/status")
        .header("authorization", &setup2.auth_header())
        .send(routes2)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    // User 2 should have their own key count (1 from setup)
    assert_eq!(body["total_api_keys"], 1);
}

#[tokio::test]
async fn test_dashboard_concurrent_requests() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");

    // Make multiple dashboard requests concurrently
    let mut handles = vec![];

    for _ in 0..5 {
        let routes = setup.routes();
        let auth = setup.auth_header();

        let handle = tokio::spawn(async move {
            AxumTestRequest::get("/dashboard/status")
                .header("authorization", &auth)
                .send(routes)
                .await
        });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status(), 200);
    }
}

#[tokio::test]
async fn test_dashboard_all_endpoints_authenticated() {
    let setup = DashboardTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let endpoints = vec![
        "/dashboard/status",
        "/dashboard/user",
        "/dashboard/admin",
        "/dashboard/detailed",
        "/dashboard/usage",
        "/dashboard/rate-limits",
        "/dashboard/logs",
    ];

    for endpoint in endpoints {
        let response = AxumTestRequest::get(endpoint)
            .header("authorization", &setup.auth_header())
            .send(routes.clone())
            .await;

        assert_eq!(
            response.status(),
            200,
            "Endpoint {} should return 200",
            endpoint
        );
    }
}
