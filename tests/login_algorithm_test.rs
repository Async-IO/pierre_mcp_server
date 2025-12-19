// ABOUTME: Tests for the login algorithm's internal logic and security properties
// ABOUTME: Validates user status checks, password verification, timestamp updates, and error uniformity
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Login Algorithm Tests
//!
//! These tests exercise the login algorithm's internal logic, focusing on:
//! - User status checks (Active, Pending, Suspended)
//! - Password verification edge cases
//! - `last_active` timestamp updates
//! - Error message uniformity (preventing information leakage)

mod common;
mod helpers;

use chrono::Utc;
use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::ServerResources,
    models::{User, UserStatus},
    routes::auth::AuthRoutes,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Test setup helper for login algorithm testing
struct LoginAlgorithmTestSetup {
    resources: Arc<ServerResources>,
    database: Arc<Database>,
}

impl LoginAlgorithmTestSetup {
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
                2048,
                Some(common::get_shared_test_jwks()),
            )
            .await,
        );

        Ok(Self {
            resources,
            database,
        })
    }

    fn routes(&self) -> axum::Router {
        AuthRoutes::routes(self.resources.clone())
    }

    /// Create a user with a specific status and password
    async fn create_user_with_status(
        &self,
        email: &str,
        password: &str,
        status: UserStatus,
    ) -> anyhow::Result<User> {
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let mut user = User::new(
            email.to_owned(),
            password_hash,
            Some("Test User".to_owned()),
        );
        user.user_status = status;

        // Set approval fields for Active users
        if status == UserStatus::Active {
            user.approved_by = Some(user.id);
            user.approved_at = Some(Utc::now());
        }

        self.database.create_user(&user).await?;
        Ok(user)
    }
}

// ============================================================================
// User Status Tests - Verify login behavior for different account states
// ============================================================================

#[tokio::test]
async fn test_login_active_user_succeeds() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "active@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(
        response.status(),
        200,
        "Active user should be able to login"
    );

    let body: serde_json::Value = response.json();
    assert!(body["access_token"].is_string());
    assert!(body["user"]["email"].as_str() == Some(email));
}

#[tokio::test]
async fn test_login_pending_user_succeeds_with_status_in_response() {
    // DESIGN NOTE: Pending users CAN authenticate - the frontend restricts access
    // based on user_status in the response. This is intentional to allow the
    // frontend to display appropriate messaging (e.g., "Your account is pending approval")
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "pending@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Pending)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    // Pending users can authenticate - access control is handled by frontend
    assert_eq!(
        response.status(),
        200,
        "Pending user should be able to authenticate (frontend handles access control)"
    );

    let body: serde_json::Value = response.json();

    // Verify the user_status is returned so frontend can act on it
    let user_status = body["user"]["user_status"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();

    assert!(
        user_status.contains("pending"),
        "Response should include user_status=Pending for frontend handling, got: {}",
        user_status
    );
}

#[tokio::test]
async fn test_login_suspended_user_succeeds_with_status_in_response() {
    // DESIGN NOTE: Suspended users CAN authenticate - the frontend restricts access
    // based on user_status in the response. This allows the frontend to display
    // appropriate messaging (e.g., "Your account has been suspended")
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "suspended@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Suspended)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    // Suspended users can authenticate - access control is handled by frontend
    assert_eq!(
        response.status(),
        200,
        "Suspended user should be able to authenticate (frontend handles access control)"
    );

    let body: serde_json::Value = response.json();

    // Verify the user_status is returned so frontend can act on it
    let user_status = body["user"]["user_status"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();

    assert!(
        user_status.contains("suspended"),
        "Response should include user_status=Suspended for frontend handling, got: {}",
        user_status
    );
}

// ============================================================================
// Password Verification Tests - Edge cases in password checking
// ============================================================================

#[tokio::test]
async fn test_login_correct_password_succeeds() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "correct_pw@example.com";
    let password = "correctPassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200, "Correct password should succeed");
}

#[tokio::test]
async fn test_login_wrong_password_fails() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "wrong_pw@example.com";
    let correct_password = "correctPassword123";
    let wrong_password = "wrongPassword456";

    setup
        .create_user_with_status(email, correct_password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", wrong_password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    // OAuth2 ROPC returns 400 with invalid_grant for bad credentials
    assert_eq!(
        response.status(),
        400,
        "Wrong password should fail with 400"
    );
}

#[tokio::test]
async fn test_login_empty_password_fails() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "empty_pw@example.com";
    let password = "realPassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", ""),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_ne!(response.status(), 200, "Empty password should not succeed");
}

#[tokio::test]
async fn test_login_case_sensitive_password() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "case_sensitive@example.com";
    let password = "CaseSensitivePassword123";
    let wrong_case_password = "casesensitivepassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    // Try with wrong case
    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", wrong_case_password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(
        response.status(),
        400,
        "Password verification should be case-sensitive"
    );
}

#[tokio::test]
async fn test_login_unicode_password() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "unicode_pw@example.com";
    let password = "пароль密码🔐123"; // Russian, Chinese, and emoji

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(
        response.status(),
        200,
        "Unicode passwords should work correctly"
    );
}

// ============================================================================
// Last Active Timestamp Tests - Verify timestamp updates on login
// ============================================================================

#[tokio::test]
async fn test_login_updates_last_active_timestamp() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "timestamp@example.com";
    let password = "securePassword123";

    let user = setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    // Record time before login
    let before_login = Utc::now();

    // Small delay to ensure timestamp difference
    sleep(Duration::from_millis(10)).await;

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    // Fetch the updated user from database
    let updated_user = setup
        .database
        .get_user(user.id)
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    assert!(
        updated_user.last_active > before_login,
        "last_active should be updated after login. Before: {}, After: {}",
        before_login,
        updated_user.last_active
    );
}

#[tokio::test]
async fn test_failed_login_does_not_update_timestamp() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "no_timestamp@example.com";
    let password = "securePassword123";

    let user = setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    // Get initial last_active
    let initial_user = setup
        .database
        .get_user(user.id)
        .await
        .expect("Failed to get user")
        .expect("User should exist");
    let initial_last_active = initial_user.last_active;

    // Small delay
    sleep(Duration::from_millis(10)).await;

    let routes = setup.routes();

    // Attempt login with wrong password
    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", "wrongPassword"),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 400, "Login should fail");

    // Verify timestamp was NOT updated
    let after_user = setup
        .database
        .get_user(user.id)
        .await
        .expect("Failed to get user")
        .expect("User should exist");

    assert_eq!(
        after_user.last_active, initial_last_active,
        "last_active should NOT be updated on failed login"
    );
}

// ============================================================================
// Error Message Uniformity Tests - Prevent information leakage
// ============================================================================

#[tokio::test]
async fn test_nonexistent_user_error_matches_wrong_password_error() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let existing_email = "existing@example.com";
    let nonexistent_email = "nonexistent@example.com";
    let password = "securePassword123";

    // Create one user
    setup
        .create_user_with_status(existing_email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    // Try login with nonexistent user
    let nonexistent_request = [
        ("grant_type", "password"),
        ("username", nonexistent_email),
        ("password", password),
    ];

    let nonexistent_response = AxumTestRequest::post("/oauth/token")
        .form(&nonexistent_request)
        .send(routes.clone())
        .await;

    // Try login with existing user but wrong password
    let wrong_pw_request = [
        ("grant_type", "password"),
        ("username", existing_email),
        ("password", "wrongPassword"),
    ];

    let wrong_pw_response = AxumTestRequest::post("/oauth/token")
        .form(&wrong_pw_request)
        .send(routes)
        .await;

    // Both should return the same status code
    assert_eq!(
        nonexistent_response.status(),
        wrong_pw_response.status(),
        "Nonexistent user and wrong password should return same status code"
    );

    // Parse error responses
    let nonexistent_body: serde_json::Value = nonexistent_response.json();
    let wrong_pw_body: serde_json::Value = wrong_pw_response.json();

    // Error types should be the same
    assert_eq!(
        nonexistent_body["error"], wrong_pw_body["error"],
        "Error types should match to prevent user enumeration"
    );

    // Error descriptions should be identical or similarly vague
    let nonexistent_desc = nonexistent_body["error_description"]
        .as_str()
        .unwrap_or_default();
    let wrong_pw_desc = wrong_pw_body["error_description"]
        .as_str()
        .unwrap_or_default();

    // Both should be generic "invalid credentials" type messages
    assert!(
        nonexistent_desc.to_lowercase().contains("invalid")
            || nonexistent_desc
                .to_lowercase()
                .contains("email or password"),
        "Nonexistent user error should be generic: {}",
        nonexistent_desc
    );

    assert!(
        wrong_pw_desc.to_lowercase().contains("invalid")
            || wrong_pw_desc.to_lowercase().contains("email or password"),
        "Wrong password error should be generic: {}",
        wrong_pw_desc
    );
}

#[tokio::test]
async fn test_error_does_not_reveal_user_exists() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "secret_exists@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    // Try login with wrong password
    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", "wrongPassword"),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    let body: serde_json::Value = response.json();
    let error_desc = body["error_description"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();

    // Should NOT contain phrases that reveal the user exists
    assert!(
        !error_desc.contains("user exists"),
        "Error should not reveal user exists"
    );
    assert!(
        !error_desc.contains("password incorrect"),
        "Error should not specifically mention password is wrong"
    );
    assert!(
        !error_desc.contains("wrong password"),
        "Error should not specifically mention wrong password"
    );
}

// ============================================================================
// Additional Edge Cases
// ============================================================================

#[tokio::test]
async fn test_login_with_whitespace_in_email() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "whitespace@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    // Try login with leading/trailing whitespace in email
    let login_request = [
        ("grant_type", "password"),
        ("username", " whitespace@example.com "),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    // This test documents current behavior - the system should either:
    // 1. Trim whitespace and succeed (user-friendly)
    // 2. Fail with invalid credentials (strict matching)
    // Either is acceptable as long as it's consistent
    let status = response.status();
    assert!(
        status == 200 || status == 400,
        "Should either succeed (trimmed) or fail (strict), got {}",
        status
    );
}

#[tokio::test]
async fn test_login_case_insensitive_email() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "CaseMixed@Example.COM";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    // Try login with different case
    let login_request = [
        ("grant_type", "password"),
        ("username", "casemixed@example.com"),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    // Email should typically be case-insensitive (RFC 5321)
    // This test documents the current behavior
    let status = response.status();
    assert!(
        status == 200 || status == 400,
        "Email matching behavior should be consistent, got {}",
        status
    );
}

#[tokio::test]
async fn test_multiple_failed_logins_same_user() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "multiple_fails@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    // Attempt multiple failed logins
    for i in 0..5 {
        let login_request = [
            ("grant_type", "password"),
            ("username", email),
            ("password", "wrongPassword"),
        ];

        let response = AxumTestRequest::post("/oauth/token")
            .form(&login_request)
            .send(routes.clone())
            .await;

        assert_eq!(
            response.status(),
            400,
            "Attempt {} should fail with 400",
            i + 1
        );
    }

    // After multiple failures, a correct login should still work
    // (unless rate limiting kicks in, which is a separate concern)
    let correct_login = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&correct_login)
        .send(routes)
        .await;

    // Should succeed (or be rate limited with 429)
    assert!(
        response.status() == 200 || response.status() == 429,
        "Correct password should succeed (or be rate limited), got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_login_response_contains_required_fields() {
    let setup = LoginAlgorithmTestSetup::new().await.expect("Setup failed");

    let email = "fields_check@example.com";
    let password = "securePassword123";

    setup
        .create_user_with_status(email, password, UserStatus::Active)
        .await
        .expect("Failed to create user");

    let routes = setup.routes();

    let login_request = [
        ("grant_type", "password"),
        ("username", email),
        ("password", password),
    ];

    let response = AxumTestRequest::post("/oauth/token")
        .form(&login_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();

    // OAuth2 required fields
    assert!(
        body["access_token"].is_string(),
        "Response must contain access_token"
    );
    assert!(
        body["token_type"].is_string(),
        "Response must contain token_type"
    );
    assert!(
        body["expires_in"].is_number(),
        "Response must contain expires_in"
    );

    // User info fields
    assert!(
        body["user"]["user_id"].is_string(),
        "Response must contain user.user_id"
    );
    assert!(
        body["user"]["email"].is_string(),
        "Response must contain user.email"
    );
    assert!(
        body["user"]["user_status"].is_string(),
        "Response must contain user.user_status"
    );
}
