// ABOUTME: HTTP integration tests for the change password endpoint
// ABOUTME: Tests password change flow including auth, validation, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Integration tests for PUT /api/user/change-password endpoint

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    mcp::resources::{ServerResources, ServerResourcesOptions},
    routes::auth::AuthRoutes,
};
use serde_json::json;
use std::sync::Arc;

/// Test setup helper for change password testing
struct ChangePasswordTestSetup {
    resources: Arc<ServerResources>,
}

impl ChangePasswordTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let cache = common::create_test_cache().await?;

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

        Ok(Self { resources })
    }

    fn routes(&self) -> axum::Router {
        AuthRoutes::routes(self.resources.clone())
    }

    /// Create a test user and return their JWT token
    async fn create_user_with_token(&self) -> anyhow::Result<(String, String)> {
        let (_, user) = common::create_test_user(&self.resources.database).await?;
        let jwt_token = self
            .resources
            .auth_manager
            .generate_token(&user, &self.resources.jwks_manager)?;
        Ok((jwt_token, user.email))
    }
}

// ============================================================================
// PUT /api/user/change-password - Change Password Tests
// ============================================================================

#[tokio::test]
async fn test_change_password_success() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let (jwt_token, _email) = setup
        .create_user_with_token()
        .await
        .expect("Failed to create user");
    let routes = setup.routes();

    let change_password_request = json!({
        "current_password": "password123",
        "new_password": "NewSecurePass456"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", &format!("Bearer {}", jwt_token))
        .json(&change_password_request)
        .send(routes)
        .await;

    assert_eq!(
        response.status(),
        200,
        "Change password should succeed with valid credentials"
    );

    let body: serde_json::Value = response.json();
    assert!(
        body["message"].is_string(),
        "Response should contain a message"
    );
}

#[tokio::test]
async fn test_change_password_can_login_with_new_password() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let (jwt_token, email) = setup
        .create_user_with_token()
        .await
        .expect("Failed to create user");
    let routes = setup.routes();

    // Change the password
    let change_password_request = json!({
        "current_password": "password123",
        "new_password": "NewSecurePass456"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", &format!("Bearer {}", jwt_token))
        .json(&change_password_request)
        .send(routes.clone())
        .await;

    assert_eq!(response.status(), 200);

    // Login with new password should succeed
    let login_request = [
        ("grant_type", "password"),
        ("username", email.as_str()),
        ("password", "NewSecurePass456"),
    ];

    let login_response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes.clone())
        .await;

    assert_eq!(
        login_response.status(),
        200,
        "Login with new password should succeed"
    );

    // Login with old password should fail
    let old_login_request = [
        ("grant_type", "password"),
        ("username", email.as_str()),
        ("password", "password123"),
    ];

    let old_login_response = AxumTestRequest::post("/oauth/token")
        .form(&old_login_request)
        .send(routes)
        .await;

    assert_eq!(
        old_login_response.status(),
        400,
        "Login with old password should fail after change"
    );
}

#[tokio::test]
async fn test_change_password_wrong_current_password() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let (jwt_token, _email) = setup
        .create_user_with_token()
        .await
        .expect("Failed to create user");
    let routes = setup.routes();

    let change_password_request = json!({
        "current_password": "wrongpassword",
        "new_password": "NewSecurePass456"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", &format!("Bearer {}", jwt_token))
        .json(&change_password_request)
        .send(routes)
        .await;

    assert_eq!(
        response.status(),
        401,
        "Should reject wrong current password"
    );
}

#[tokio::test]
async fn test_change_password_weak_new_password() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let (jwt_token, _email) = setup
        .create_user_with_token()
        .await
        .expect("Failed to create user");
    let routes = setup.routes();

    let change_password_request = json!({
        "current_password": "password123",
        "new_password": "weak"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", &format!("Bearer {}", jwt_token))
        .json(&change_password_request)
        .send(routes)
        .await;

    // Should fail validation (password too short or doesn't meet requirements)
    assert!(
        response.status() == 400 || response.status() == 422,
        "Should reject weak new password, got status {}",
        response.status()
    );
}

#[tokio::test]
async fn test_change_password_missing_auth() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let change_password_request = json!({
        "current_password": "password123",
        "new_password": "NewSecurePass456"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .json(&change_password_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401, "Should require authentication");
}

#[tokio::test]
async fn test_change_password_invalid_auth() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let change_password_request = json!({
        "current_password": "password123",
        "new_password": "NewSecurePass456"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", "Bearer invalid_token")
        .json(&change_password_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401, "Should reject invalid auth token");
}

#[tokio::test]
async fn test_change_password_missing_fields() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let (jwt_token, _email) = setup
        .create_user_with_token()
        .await
        .expect("Failed to create user");
    let routes = setup.routes();

    // Missing new_password
    let request_missing_new = json!({
        "current_password": "password123"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", &format!("Bearer {}", jwt_token))
        .json(&request_missing_new)
        .send(routes.clone())
        .await;

    assert!(
        response.status() == 400 || response.status() == 422,
        "Should reject missing new_password field"
    );

    // Missing current_password
    let request_missing_current = json!({
        "new_password": "NewSecurePass456"
    });

    let response = AxumTestRequest::put("/api/user/change-password")
        .header("Authorization", &format!("Bearer {}", jwt_token))
        .json(&request_missing_current)
        .send(routes)
        .await;

    assert!(
        response.status() == 400 || response.status() == 422,
        "Should reject missing current_password field"
    );
}

#[tokio::test]
async fn test_change_password_endpoint_registered() {
    let setup = ChangePasswordTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::put("/api/user/change-password")
        .json(&json!({}))
        .send(routes)
        .await;

    // Should not be 404 (method not allowed or auth error, but not not-found)
    assert_ne!(
        response.status(),
        404,
        "PUT /api/user/change-password should be registered"
    );
}
