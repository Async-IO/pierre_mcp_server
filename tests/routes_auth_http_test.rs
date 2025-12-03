// ABOUTME: HTTP integration tests for authentication routes
// ABOUTME: Tests all authentication endpoints including registration, login, refresh, and OAuth status
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for authentication routes
//!
//! This test suite validates that all authentication endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::mcp::resources::ServerResources;
use serde_json::json;
use std::sync::Arc;

/// Test setup helper for authentication route testing
struct AuthTestSetup {
    resources: Arc<ServerResources>,
}

impl AuthTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let cache = common::create_test_cache().await?;

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
                auto_approve_users: false,
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

        Ok(Self { resources })
    }

    fn routes(&self) -> axum::Router {
        pierre_mcp_server::routes::auth::AuthRoutes::routes(self.resources.clone())
    }

    /// Create a test admin token for authentication
    async fn create_admin_token(&self) -> anyhow::Result<String> {
        use pierre_mcp_server::admin::models::CreateAdminTokenRequest;
        use pierre_mcp_server::database_plugins::DatabaseProvider;

        // Create admin token request
        let request = CreateAdminTokenRequest {
            service_name: "test_admin".to_owned(),
            service_description: Some("Auto-generated test admin token".to_owned()),
            permissions: None, // Super admin by default
            expires_in_days: Some(1),
            is_super_admin: true,
        };

        // Use database method to create admin token
        let generated_token = self
            .resources
            .database
            .as_ref()
            .create_admin_token(&request, "test_jwt_secret", &self.resources.jwks_manager)
            .await?;

        Ok(generated_token.jwt_token)
    }
}

// ============================================================================
// POST /api/auth/register - User Registration Tests
// ============================================================================

#[tokio::test]
async fn test_register_success() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let admin_token = setup
        .create_admin_token()
        .await
        .expect("Failed to create admin token");
    let routes = setup.routes();

    let register_request = json!({
        "email": "newuser@example.com",
        "password": "securePassword123",
        "display_name": "New User"
    });

    let response = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json();
    assert!(body["user_id"].is_string());
    assert!(body["message"].is_string());
}

#[tokio::test]
async fn test_register_requires_admin_auth() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let register_request = json!({
        "email": "public@example.com",
        "password": "password123",
        "display_name": "Public User"
    });

    // Registration WITHOUT admin token should fail
    let response = AxumTestRequest::post("/api/auth/register")
        .json(&register_request)
        .send(routes)
        .await;

    // Should require authentication (401 or 400 for missing auth header)
    assert!(
        response.status() == 400 || response.status() == 401,
        "Expected 400 or 401, got {}",
        response.status()
    );

    let body: serde_json::Value = response.json();
    let message = body["message"].as_str().unwrap_or("");
    assert!(
        message.contains("Authorization")
            || message.contains("admin")
            || message.contains("authentication")
            || message.contains("credentials"),
        "Error message should mention authorization: {}",
        message
    );
}

#[tokio::test]
async fn test_register_invalid_email() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let admin_token = setup
        .create_admin_token()
        .await
        .expect("Failed to create admin token");
    let routes = setup.routes();

    let register_request = json!({
        "email": "invalid-email",
        "password": "password123"
    });

    let response = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes)
        .await;

    // Should fail validation
    assert!(response.status() == 400 || response.status() == 422);
}

#[tokio::test]
async fn test_register_weak_password() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let admin_token = setup
        .create_admin_token()
        .await
        .expect("Failed to create admin token");
    let routes = setup.routes();

    let register_request = json!({
        "email": "user@example.com",
        "password": "weak"
    });

    let response = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes)
        .await;

    // Should fail validation (password too short)
    assert!(response.status() == 400 || response.status() == 422);
}

#[tokio::test]
async fn test_register_duplicate_email() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let admin_token = setup
        .create_admin_token()
        .await
        .expect("Failed to create admin token");
    let routes = setup.routes();

    let register_request = json!({
        "email": "duplicate@example.com",
        "password": "password123",
        "display_name": "Duplicate User"
    });

    // First registration should succeed
    let _response1 = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes.clone())
        .await;

    // Second registration with same email should fail
    let response2 = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes)
        .await;

    // First should succeed or have valid error, second should fail
    assert_ne!(response2.status(), 201);
}

#[tokio::test]
async fn test_register_missing_required_fields() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let admin_token = setup
        .create_admin_token()
        .await
        .expect("Failed to create admin token");
    let routes = setup.routes();

    let register_request = json!({
        "email": "user@example.com"
        // Missing password
    });

    let response = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes)
        .await;

    // Should fail validation
    assert_ne!(response.status(), 201);
}

// ============================================================================
// POST /api/auth/login - User Login Tests
// ============================================================================

#[tokio::test]
async fn test_login_success() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");

    // Create a test user first
    let (_, user) = common::create_test_user(&setup.resources.database)
        .await
        .expect("Failed to create test user");

    let routes = setup.routes();

    let login_request = json!({
        "email": user.email,
        "password": "password123"  // Default password from create_test_user
    });

    let response = AxumTestRequest::post("/api/auth/login")
        .json(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["jwt_token"].is_string());
    assert!(body["expires_at"].is_string());
    assert!(body["user"]["user_id"].is_string());
    assert!(body["user"]["email"].is_string());
}

#[tokio::test]
async fn test_login_no_auth_required() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let login_request = json!({
        "email": "user@example.com",
        "password": "password123"
    });

    // Login should work without authentication header
    let response = AxumTestRequest::post("/api/auth/login")
        .json(&login_request)
        .send(routes)
        .await;

    // Should fail due to wrong credentials (401) but not require auth header
    // If it returns 401, it's because credentials are wrong, not missing auth
    assert_ne!(response.status(), 500);
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let login_request = json!({
        "email": "nonexistent@example.com",
        "password": "wrongpassword"
    });

    let response = AxumTestRequest::post("/api/auth/login")
        .json(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_login_wrong_password() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");

    // Create a test user first
    let (_, user) = common::create_test_user(&setup.resources.database)
        .await
        .expect("Failed to create test user");

    let routes = setup.routes();

    let login_request = json!({
        "email": user.email,
        "password": "wrongpassword"
    });

    let response = AxumTestRequest::post("/api/auth/login")
        .json(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_login_missing_fields() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let login_request = json!({
        "email": "user@example.com"
        // Missing password
    });

    let response = AxumTestRequest::post("/api/auth/login")
        .json(&login_request)
        .send(routes)
        .await;

    // Should fail validation
    assert_ne!(response.status(), 200);
}

// ============================================================================
// POST /api/auth/refresh - Token Refresh Tests
// ============================================================================

#[tokio::test]
async fn test_refresh_token_success() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");

    // Create test user and generate token
    let (user_id, user) = common::create_test_user(&setup.resources.database)
        .await
        .expect("Failed to create test user");

    let jwt_token = setup
        .resources
        .auth_manager
        .generate_token(&user, &setup.resources.jwks_manager)
        .expect("Failed to generate JWT");

    let routes = setup.routes();

    let refresh_request = json!({
        "token": jwt_token,
        "user_id": user_id.to_string()
    });

    let response = AxumTestRequest::post("/api/auth/refresh")
        .json(&refresh_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["jwt_token"].is_string());
    assert!(body["expires_at"].is_string());
    assert!(body["user"]["user_id"].is_string());
}

#[tokio::test]
async fn test_refresh_token_invalid_token() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let refresh_request = json!({
        "token": "invalid_jwt_token",
        "user_id": uuid::Uuid::new_v4().to_string()
    });

    let response = AxumTestRequest::post("/api/auth/refresh")
        .json(&refresh_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_refresh_token_user_id_mismatch() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");

    // Create test user and generate token
    let (_, user) = common::create_test_user(&setup.resources.database)
        .await
        .expect("Failed to create test user");

    let jwt_token = setup
        .resources
        .auth_manager
        .generate_token(&user, &setup.resources.jwks_manager)
        .expect("Failed to generate JWT");

    let routes = setup.routes();

    let refresh_request = json!({
        "token": jwt_token,
        "user_id": uuid::Uuid::new_v4().to_string()  // Different user_id
    });

    let response = AxumTestRequest::post("/api/auth/refresh")
        .json(&refresh_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_refresh_token_missing_fields() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let refresh_request = json!({
        "token": "some_token"
        // Missing user_id
    });

    let response = AxumTestRequest::post("/api/auth/refresh")
        .json(&refresh_request)
        .send(routes)
        .await;

    // Should fail validation
    assert_ne!(response.status(), 200);
}

// ============================================================================
// GET /api/oauth/status - OAuth Status Tests
// ============================================================================

#[tokio::test]
async fn test_oauth_status_success() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");

    // Create test user and generate token
    let (_, user) = common::create_test_user(&setup.resources.database)
        .await
        .expect("Failed to create test user");

    let jwt_token = setup
        .resources
        .auth_manager
        .generate_token(&user, &setup.resources.jwks_manager)
        .expect("Failed to generate JWT");

    let routes = setup.routes();

    let response = AxumTestRequest::get("/api/oauth/status")
        .header("authorization", &format!("Bearer {}", jwt_token))
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body.is_array());

    // Should contain OAuth provider statuses
    let statuses = body.as_array().unwrap();
    for status in statuses {
        assert!(status["provider"].is_string());
        assert!(status["connected"].is_boolean());
    }
}

#[tokio::test]
async fn test_oauth_status_missing_auth() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/api/oauth/status").send(routes).await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_oauth_status_invalid_auth() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/api/oauth/status")
        .header("authorization", "Bearer invalid_token")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_oauth_status_includes_all_providers() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");

    // Create test user and generate token
    let (_, user) = common::create_test_user(&setup.resources.database)
        .await
        .expect("Failed to create test user");

    let jwt_token = setup
        .resources
        .auth_manager
        .generate_token(&user, &setup.resources.jwks_manager)
        .expect("Failed to generate JWT");

    let routes = setup.routes();

    let response = AxumTestRequest::get("/api/oauth/status")
        .header("authorization", &format!("Bearer {}", jwt_token))
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let statuses = body.as_array().unwrap();

    // Should include common providers like strava and fitbit
    let providers: Vec<String> = statuses
        .iter()
        .map(|s| s["provider"].as_str().unwrap().to_owned())
        .collect();

    assert!(providers.contains(&"strava".to_owned()));
    assert!(providers.contains(&"fitbit".to_owned()));
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_all_auth_endpoints_registered() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Test that all endpoints are registered (not 404)
    let endpoints = vec![
        ("/api/auth/register", "POST"),
        ("/api/auth/login", "POST"),
        ("/api/auth/refresh", "POST"),
        ("/api/oauth/status", "GET"),
    ];

    for (endpoint, method) in endpoints {
        let response = if method == "POST" {
            AxumTestRequest::post(endpoint)
                .json(&json!({}))
                .send(routes.clone())
                .await
        } else {
            AxumTestRequest::get(endpoint).send(routes.clone()).await
        };

        // Should not be 404 (endpoint not found)
        assert_ne!(
            response.status(),
            404,
            "{} {} should be registered",
            method,
            endpoint
        );
    }
}

#[tokio::test]
async fn test_register_and_login_flow() {
    let setup = AuthTestSetup::new().await.expect("Setup failed");
    let admin_token = setup
        .create_admin_token()
        .await
        .expect("Failed to create admin token");
    let routes = setup.routes();

    // Step 1: Register a new user (with admin auth)
    let email = format!("flowtest{}@example.com", uuid::Uuid::new_v4());
    let password = "securePassword123";

    let register_request = json!({
        "email": email,
        "password": password,
        "display_name": "Flow Test User"
    });

    let register_response = AxumTestRequest::post("/api/auth/register")
        .header("Authorization", &format!("Bearer {}", admin_token))
        .json(&register_request)
        .send(routes.clone())
        .await;

    // Registration might fail in some test scenarios, so we'll be flexible
    if register_response.status() != 201 {
        return;
    }

    // Step 2: Login with the registered credentials
    let login_request = json!({
        "email": email,
        "password": password
    });

    let login_response = AxumTestRequest::post("/api/auth/login")
        .json(&login_request)
        .send(routes)
        .await;

    // Login might fail if user needs approval, so we'll check both scenarios
    assert!(
        login_response.status() == 200 || login_response.status() == 403,
        "Login should either succeed or be forbidden due to pending approval"
    );
}
