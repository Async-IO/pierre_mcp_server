// ABOUTME: Integration tests for the password reset token flow
// ABOUTME: Tests one-time token issuance via admin and redemption via POST /api/auth/complete-reset
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Integration tests for the password reset token flow:
//! 1. Admin issues a reset token (stored as SHA-256 hash in DB)
//! 2. User calls POST /api/auth/complete-reset with the raw token + new password
//! 3. Token is consumed atomically (single-use)
//! 4. User can log in with the new password

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    database_plugins::DatabaseProvider,
    mcp::resources::{ServerResources, ServerResourcesOptions},
    routes::auth::AuthRoutes,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Test setup helper for password reset token testing
struct ResetTokenTestSetup {
    resources: Arc<ServerResources>,
}

impl ResetTokenTestSetup {
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

    /// Create a test user and return their UUID and email
    async fn create_user(&self) -> anyhow::Result<(uuid::Uuid, String)> {
        let (_, user) = common::create_test_user(&self.resources.database).await?;
        Ok((user.id, user.email))
    }

    /// Store a reset token for a user and return the raw token
    async fn issue_reset_token(&self, user_id: uuid::Uuid) -> anyhow::Result<String> {
        use rand::distributions::Alphanumeric;
        use rand::Rng;

        let raw_token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(48)
            .map(char::from)
            .collect();

        let token_hash = format!("{:x}", Sha256::digest(raw_token.as_bytes()));

        self.resources
            .database
            .store_password_reset_token(user_id, &token_hash, "test_admin")
            .await?;

        Ok(raw_token)
    }
}

// ============================================================================
// POST /api/auth/complete-reset - Complete Password Reset Tests
// ============================================================================

#[tokio::test]
async fn test_complete_reset_success() {
    let setup = ResetTokenTestSetup::new().await.expect("Setup failed");
    let (user_id, email) = setup.create_user().await.expect("Failed to create user");
    let raw_token = setup
        .issue_reset_token(user_id)
        .await
        .expect("Failed to issue token");
    let routes = setup.routes();

    // Complete the reset with the token
    let response = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": raw_token,
            "new_password": "BrandNewPassword789"
        }))
        .send(routes.clone())
        .await;

    assert_eq!(
        response.status(),
        200,
        "Complete reset should succeed with valid token"
    );

    let body: serde_json::Value = response.json();
    assert!(
        body["message"].is_string(),
        "Response should contain a message"
    );

    // Verify user can now log in with the new password
    let login_request = [
        ("grant_type", "password"),
        ("username", email.as_str()),
        ("password", "BrandNewPassword789"),
    ];

    let login_response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes.clone())
        .await;

    assert_eq!(
        login_response.status(),
        200,
        "Login with new password should succeed after reset"
    );

    // Verify old password no longer works
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
        "Login with old password should fail after reset"
    );
}

#[tokio::test]
async fn test_complete_reset_token_single_use() {
    let setup = ResetTokenTestSetup::new().await.expect("Setup failed");
    let (user_id, _email) = setup.create_user().await.expect("Failed to create user");
    let raw_token = setup
        .issue_reset_token(user_id)
        .await
        .expect("Failed to issue token");
    let routes = setup.routes();

    // First use should succeed
    let first_response = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": raw_token,
            "new_password": "FirstNewPassword123"
        }))
        .send(routes.clone())
        .await;

    assert_eq!(first_response.status(), 200, "First use should succeed");

    // Second use of the same token should fail (already consumed)
    let second_response = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": raw_token,
            "new_password": "SecondNewPassword456"
        }))
        .send(routes)
        .await;

    assert_eq!(
        second_response.status(),
        404,
        "Second use of same token should fail (already consumed)"
    );
}

#[tokio::test]
async fn test_complete_reset_invalid_token() {
    let setup = ResetTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": "this_token_does_not_exist_in_database",
            "new_password": "NewPassword123"
        }))
        .send(routes)
        .await;

    assert_eq!(response.status(), 404, "Invalid token should return 404");
}

#[tokio::test]
async fn test_complete_reset_weak_password_rejected() {
    let setup = ResetTokenTestSetup::new().await.expect("Setup failed");
    let (user_id, _email) = setup.create_user().await.expect("Failed to create user");
    let raw_token = setup
        .issue_reset_token(user_id)
        .await
        .expect("Failed to issue token");
    let routes = setup.routes();

    let response = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": raw_token,
            "new_password": "weak"
        }))
        .send(routes)
        .await;

    assert!(
        response.status() == 400 || response.status() == 422,
        "Weak password should be rejected, got status {}",
        response.status()
    );
}

#[tokio::test]
async fn test_complete_reset_invalidates_other_tokens() {
    let setup = ResetTokenTestSetup::new().await.expect("Setup failed");
    let (user_id, _email) = setup.create_user().await.expect("Failed to create user");

    // Issue two tokens for the same user
    let token_a = setup
        .issue_reset_token(user_id)
        .await
        .expect("Failed to issue token A");
    let token_b = setup
        .issue_reset_token(user_id)
        .await
        .expect("Failed to issue token B");
    let routes = setup.routes();

    // Use token A
    let response_a = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": token_a,
            "new_password": "ResetViaTokenA123"
        }))
        .send(routes.clone())
        .await;

    assert_eq!(response_a.status(), 200, "Token A should succeed");

    // Token B should now be invalidated
    let response_b = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({
            "reset_token": token_b,
            "new_password": "ResetViaTokenB456"
        }))
        .send(routes)
        .await;

    assert_eq!(
        response_b.status(),
        404,
        "Token B should be invalidated after Token A was used"
    );
}

#[tokio::test]
async fn test_complete_reset_endpoint_registered() {
    let setup = ResetTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::post("/api/auth/complete-reset")
        .json(&json!({}))
        .send(routes)
        .await;

    // Should not be 404 (might be 400 for missing fields, but route exists)
    assert_ne!(
        response.status(),
        404,
        "POST /api/auth/complete-reset should be registered"
    );
}
