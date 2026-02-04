// ABOUTME: HTTP integration tests for A2A (Agent-to-Agent) protocol routes
// ABOUTME: Tests all A2A endpoints without authentication requirements
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for A2A protocol routes
//!
//! This test suite validates that all A2A endpoints are correctly registered
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
    routes::a2a::A2ARoutes,
};
use std::sync::Arc;

/// Create test resources for A2A route testing
async fn create_a2a_test_resources() -> Arc<ServerResources> {
    common::init_server_config();
    let database = common::create_test_database().await.unwrap();
    let auth_manager = common::create_test_auth_manager();
    let cache = common::create_test_cache().await.unwrap();
    let temp_dir = tempfile::tempdir().unwrap();

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

    Arc::new(
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
    )
}

/// Get A2A routes for testing
async fn a2a_routes() -> axum::Router {
    let resources = create_a2a_test_resources().await;
    A2ARoutes::routes(resources)
}

// ============================================================================
// GET /a2a/status - A2A Status Tests
// ============================================================================

#[tokio::test]
async fn test_a2a_status_success() {
    let routes = a2a_routes().await;

    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn test_a2a_status_no_auth_required() {
    let routes = a2a_routes().await;

    // A2A status endpoint should work without any authentication
    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_a2a_status_response_structure() {
    let routes = a2a_routes().await;

    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_object());
    assert!(body["status"].is_string());
    assert_eq!(body["status"], "active");
}

#[tokio::test]
async fn test_a2a_status_content_type() {
    let routes = a2a_routes().await;

    let response = AxumTestRequest::get("/a2a/status").send(routes).await;

    assert_eq!(response.status(), 200);

    // Response should be valid JSON
    let body: serde_json::Value = response.json();
    assert!(body.is_object());
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_a2a_status_concurrent_requests() {
    // Make multiple A2A status requests concurrently
    let mut handles = vec![];

    for _ in 0..10 {
        let handle = tokio::spawn(async {
            let routes = a2a_routes().await;
            AxumTestRequest::get("/a2a/status").send(routes).await
        });

        handles.push(handle);
    }

    // All requests should succeed
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "active");
    }
}

#[tokio::test]
async fn test_a2a_status_idempotency() {
    let routes = a2a_routes().await;

    // Make multiple requests and verify they all return the same result
    let responses = vec![
        AxumTestRequest::get("/a2a/status")
            .send(routes.clone())
            .await,
        AxumTestRequest::get("/a2a/status")
            .send(routes.clone())
            .await,
        AxumTestRequest::get("/a2a/status").send(routes).await,
    ];

    for response in responses {
        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "active");
    }
}

#[tokio::test]
async fn test_a2a_status_always_active() {
    let routes = a2a_routes().await;

    // Verify that status is always "active"
    for _ in 0..5 {
        let response = AxumTestRequest::get("/a2a/status")
            .send(routes.clone())
            .await;

        assert_eq!(response.status(), 200);

        let body: serde_json::Value = response.json();
        assert_eq!(body["status"], "active");
    }
}
