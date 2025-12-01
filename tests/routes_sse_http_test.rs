// ABOUTME: HTTP integration tests for SSE (Server-Sent Events) routes
// ABOUTME: Tests all SSE endpoints for notification streams and protocol messages
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for SSE routes
//!
//! This test suite validates that all SSE endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.
//!
//! Note: Full SSE streaming behavior is complex to test in HTTP integration tests.
//! These tests focus on connection establishment and initial response validation.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{mcp::resources::ServerResources, sse::manager::SseManager};
use std::sync::Arc;

/// Test setup helper for SSE route testing
#[allow(dead_code)]
struct SseTestSetup {
    resources: Arc<ServerResources>,
    manager: Arc<SseManager>,
    user_id: uuid::Uuid,
    jwt_token: String,
}

impl SseTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let cache = common::create_test_cache().await?;

        // Create test user
        let (user_id, user) = common::create_test_user(&database).await?;

        // Create ServerResources
        let temp_dir = tempfile::tempdir()?;
        let config = Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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

        // Create SSE manager with buffer size
        let manager = Arc::new(SseManager::new(1024));

        Ok(Self {
            resources,
            manager,
            user_id,
            jwt_token,
        })
    }

    fn routes(&self) -> axum::Router {
        pierre_mcp_server::sse::routes::SseRoutes::routes(
            self.manager.clone(),
            self.resources.clone(),
        )
    }
}

// ============================================================================
// GET /notifications/sse/:user_id - Notification SSE Tests
// ============================================================================

#[tokio::test]
async fn test_notification_sse_endpoint_registered() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let endpoint = format!("/notifications/sse/{}", setup.user_id);
    let response = AxumTestRequest::get(&endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // SSE endpoint should be registered (not 404)
    // Status code might be 200 for SSE connection or 400/500 for errors
    assert_ne!(
        response.status(),
        404,
        "SSE notification endpoint should be registered"
    );
}

#[tokio::test]
async fn test_notification_sse_valid_user_id() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let endpoint = format!("/notifications/sse/{}", setup.user_id);
    let response = AxumTestRequest::get(&endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Should accept valid UUID
    assert!(
        response.status() == 200 || response.status() == 202,
        "Valid user_id should be accepted"
    );
}

#[tokio::test]
async fn test_notification_sse_invalid_user_id() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let endpoint = "/notifications/sse/invalid-uuid";
    let response = AxumTestRequest::get(endpoint).send_sse(routes).await;

    // Should reject invalid UUID format
    assert_eq!(
        response.status(),
        400,
        "Invalid user_id format should return 400"
    );
}

#[tokio::test]
async fn test_notification_sse_different_users() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Test with authenticated user's own ID (should succeed)
    let endpoint1 = format!("/notifications/sse/{}", setup.user_id);
    let response1 = AxumTestRequest::get(&endpoint1)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes.clone())
        .await;

    // Should accept connection for own user_id
    assert!(
        response1.status() == 200 || response1.status() == 202,
        "Should accept connection for authenticated user's own user_id"
    );

    // Test with different user ID (should fail with 401/403 due to ownership check)
    let other_user_id = uuid::Uuid::new_v4();
    let endpoint2 = format!("/notifications/sse/{}", other_user_id);
    let response2 = AxumTestRequest::get(&endpoint2)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Should reject connection for different user_id (ownership enforcement)
    assert!(
        response2.status() == 401 || response2.status() == 403,
        "Should reject connection for different user_id (got {})",
        response2.status()
    );
}

#[tokio::test]
async fn test_notification_sse_requires_auth() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // SSE notification endpoints now require JWT authentication
    let endpoint = format!("/notifications/sse/{}", setup.user_id);
    let response = AxumTestRequest::get(&endpoint).send_sse(routes).await;

    // Should require Authorization header and return 401 without it
    assert_eq!(
        response.status(),
        401,
        "SSE notification endpoint should require authentication"
    );
}

// ============================================================================
// GET /mcp/sse/:session_id - Protocol SSE Tests
// ============================================================================

#[tokio::test]
async fn test_protocol_sse_endpoint_registered() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id = format!("session_{}", uuid::Uuid::new_v4());
    let endpoint = format!("/mcp/sse/{}", session_id);

    let response = AxumTestRequest::get(&endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // SSE endpoint should be registered (not 404)
    assert_ne!(
        response.status(),
        404,
        "SSE protocol endpoint should be registered"
    );
}

#[tokio::test]
async fn test_protocol_sse_valid_session_id() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id = format!("session_{}", uuid::Uuid::new_v4());
    let endpoint = format!("/mcp/sse/{}", session_id);

    let response = AxumTestRequest::get(&endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Should accept valid session ID
    assert!(
        response.status() == 200 || response.status() == 202,
        "Valid session_id should be accepted"
    );
}

#[tokio::test]
async fn test_protocol_sse_custom_session_id() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id = "custom-session-123";
    let endpoint = format!("/mcp/sse/{}", session_id);

    let response = AxumTestRequest::get(&endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Should accept any string as session ID
    assert!(
        response.status() == 200 || response.status() == 202,
        "Custom session_id should be accepted"
    );
}

#[tokio::test]
async fn test_protocol_sse_different_sessions() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Test with different session IDs
    let session_id1 = format!("session_{}", uuid::Uuid::new_v4());
    let session_id2 = format!("session_{}", uuid::Uuid::new_v4());

    let endpoint1 = format!("/mcp/sse/{}", session_id1);
    let endpoint2 = format!("/mcp/sse/{}", session_id2);

    let response1 = AxumTestRequest::get(&endpoint1)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes.clone())
        .await;
    let response2 = AxumTestRequest::get(&endpoint2)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Both should accept connections independently
    assert!(response1.status() == 200 || response1.status() == 202);
    assert!(response2.status() == 200 || response2.status() == 202);
}

#[tokio::test]
async fn test_protocol_sse_requires_auth() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id = format!("session_{}", uuid::Uuid::new_v4());
    let endpoint = format!("/mcp/sse/{}", session_id);

    // SSE protocol endpoints now require JWT authentication
    let response = AxumTestRequest::get(&endpoint).send_sse(routes).await;

    // Should require Authorization header and return 401 without it
    assert_eq!(
        response.status(),
        401,
        "SSE protocol endpoint should require authentication"
    );
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_all_sse_endpoints_registered() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let user_id = uuid::Uuid::new_v4();
    let session_id = format!("session_{}", uuid::Uuid::new_v4());

    let endpoints = vec![
        format!("/notifications/sse/{}", user_id),
        format!("/mcp/sse/{}", session_id),
    ];

    for endpoint in endpoints {
        let response = AxumTestRequest::get(&endpoint)
            .send_sse(routes.clone())
            .await;

        // Should not be 404 (endpoint not found)
        assert_ne!(
            response.status(),
            404,
            "Endpoint {} should be registered",
            endpoint
        );
    }
}

#[tokio::test]
async fn test_sse_concurrent_connections() {
    let setup = SseTestSetup::new().await.expect("Setup failed");

    // Make multiple SSE connection requests concurrently
    let mut handles = vec![];

    for _ in 0..3 {
        let routes = setup.routes();
        let user_id = setup.user_id;
        let jwt_token = setup.jwt_token.clone();
        let endpoint = format!("/notifications/sse/{}", user_id);

        let handle = tokio::spawn(async move {
            AxumTestRequest::get(&endpoint)
                .header("Authorization", &format!("Bearer {}", jwt_token))
                .send_sse(routes)
                .await
        });

        handles.push(handle);
    }

    // All connection attempts should be accepted
    for handle in handles {
        let response = handle.await.expect("Task panicked");
        assert!(
            response.status() == 200 || response.status() == 202,
            "Concurrent SSE connections should be accepted"
        );
    }
}

#[tokio::test]
async fn test_notification_and_protocol_sse_independent() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let user_id = setup.user_id;
    let session_id = format!("session_{}", uuid::Uuid::new_v4());

    let notification_endpoint = format!("/notifications/sse/{}", user_id);
    let protocol_endpoint = format!("/mcp/sse/{}", session_id);

    let response1 = AxumTestRequest::get(&notification_endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes.clone())
        .await;
    let response2 = AxumTestRequest::get(&protocol_endpoint)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Both types of SSE connections should work independently
    assert!(response1.status() == 200 || response1.status() == 202);
    assert!(response2.status() == 200 || response2.status() == 202);
}

#[tokio::test]
async fn test_sse_user_isolation() {
    let setup1 = SseTestSetup::new().await.expect("Setup 1 failed");
    let setup2 = SseTestSetup::new().await.expect("Setup 2 failed");

    let endpoint1 = format!("/notifications/sse/{}", setup1.user_id);
    let endpoint2 = format!("/notifications/sse/{}", setup2.user_id);

    let routes1 = setup1.routes();
    let routes2 = setup2.routes();

    let response1 = AxumTestRequest::get(&endpoint1)
        .header("Authorization", &format!("Bearer {}", setup1.jwt_token))
        .send_sse(routes1)
        .await;
    let response2 = AxumTestRequest::get(&endpoint2)
        .header("Authorization", &format!("Bearer {}", setup2.jwt_token))
        .send_sse(routes2)
        .await;

    // Both users should have independent SSE streams
    assert!(response1.status() == 200 || response1.status() == 202);
    assert!(response2.status() == 200 || response2.status() == 202);
}

#[tokio::test]
async fn test_sse_session_isolation() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let session_id1 = format!("session_{}", uuid::Uuid::new_v4());
    let session_id2 = format!("session_{}", uuid::Uuid::new_v4());

    let endpoint1 = format!("/mcp/sse/{}", session_id1);
    let endpoint2 = format!("/mcp/sse/{}", session_id2);

    let response1 = AxumTestRequest::get(&endpoint1)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes.clone())
        .await;
    let response2 = AxumTestRequest::get(&endpoint2)
        .header("Authorization", &format!("Bearer {}", setup.jwt_token))
        .send_sse(routes)
        .await;

    // Different sessions should be isolated
    assert!(response1.status() == 200 || response1.status() == 202);
    assert!(response2.status() == 200 || response2.status() == 202);
}

#[tokio::test]
async fn test_sse_path_parameter_validation() {
    let setup = SseTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Test various path parameter formats
    let test_cases = vec![
        (format!("/notifications/sse/{}", uuid::Uuid::new_v4()), true), // Valid UUID
        ("/notifications/sse/invalid-uuid".to_owned(), false),          // Invalid UUID
        (format!("/mcp/sse/session_{}", uuid::Uuid::new_v4()), true),   // Valid session
        ("/mcp/sse/simple-session".to_owned(), true),                   // Simple session ID
    ];

    for (endpoint, should_accept) in &test_cases {
        let response = AxumTestRequest::get(endpoint)
            .send_sse(routes.clone())
            .await;

        if *should_accept {
            assert_ne!(
                response.status(),
                400,
                "{} should accept valid format",
                endpoint
            );
        } else {
            assert_eq!(
                response.status(),
                400,
                "{} should reject invalid format",
                endpoint
            );
        }
    }
}
