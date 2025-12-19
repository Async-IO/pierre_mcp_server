// ABOUTME: HTTP integration tests for user MCP token routes
// ABOUTME: Tests token creation, listing, revocation, and validation for user self-service token management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Integration tests for user MCP token management
//!
//! This test suite validates the user MCP token routes for creating, listing,
//! and revoking tokens for AI client authentication.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{
    config::environment::{
        AppBehaviorConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment, SecurityConfig,
        SecurityHeadersConfig, ServerConfig,
    },
    database::CreateUserMcpTokenRequest,
    database_plugins::DatabaseProvider,
    mcp::resources::ServerResources,
    routes::user_mcp_tokens::UserMcpTokenRoutes,
};
use serde_json::json;
use std::sync::Arc;

/// Test setup helper for user MCP token route testing
struct UserMcpTokenTestSetup {
    resources: Arc<ServerResources>,
    user_jwt: String,
}

impl UserMcpTokenTestSetup {
    async fn new() -> anyhow::Result<Self> {
        common::init_server_config();
        let database = common::create_test_database().await?;
        let auth_manager = common::create_test_auth_manager();
        let cache = common::create_test_cache().await?;

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
                2048,
                Some(common::get_shared_test_jwks()),
            )
            .await,
        );

        // Create test user
        let (user_id, user) = common::create_test_user(&database).await?;
        // `user_id` is used indirectly via JWT claims (embedded in token)
        let _ = user_id;

        // Generate user JWT token using auth_manager
        let user_jwt = resources
            .auth_manager
            .generate_token(&user, &resources.jwks_manager)
            .map_err(|e| anyhow::Error::msg(format!("Failed to generate JWT: {}", e)))?;

        Ok(Self {
            resources,
            user_jwt,
        })
    }

    fn routes(&self) -> axum::Router {
        UserMcpTokenRoutes::routes(self.resources.clone())
    }
}

// ============================================================================
// POST /api/user/mcp-tokens - Token Creation Tests
// ============================================================================

#[tokio::test]
async fn test_create_mcp_token_success() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let create_request = json!({
        "name": "Claude Desktop",
        "expires_in_days": 30
    });

    let response = AxumTestRequest::post("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .json(&create_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json();
    assert!(body["id"].is_string(), "Expected id to be a string");
    assert_eq!(body["name"], "Claude Desktop");
    assert!(
        body["token_prefix"].is_string(),
        "Expected token_prefix to be a string"
    );
    assert!(
        body["token_value"].is_string(),
        "Expected token_value to be a string"
    );

    // Token should start with "pmcp_" prefix
    let token_value = body["token_value"].as_str().unwrap();
    assert!(
        token_value.starts_with("pmcp_"),
        "Token should start with pmcp_ prefix"
    );
}

#[tokio::test]
async fn test_create_mcp_token_no_expiry() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let create_request = json!({
        "name": "Cursor IDE"
        // No expires_in_days means never expires
    });

    let response = AxumTestRequest::post("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .json(&create_request)
        .send(routes)
        .await;

    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json();
    assert_eq!(body["name"], "Cursor IDE");
    assert!(
        body["expires_at"].is_null(),
        "Token should not have expiration date"
    );
}

#[tokio::test]
async fn test_create_mcp_token_requires_auth() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let create_request = json!({
        "name": "Unauthorized Token"
    });

    // Request without Authorization header should fail
    let response = AxumTestRequest::post("/api/user/mcp-tokens")
        .json(&create_request)
        .send(routes)
        .await;

    assert!(
        response.status() == 400 || response.status() == 401,
        "Expected 400 or 401, got {}",
        response.status()
    );
}

// ============================================================================
// GET /api/user/mcp-tokens - Token Listing Tests
// ============================================================================

#[tokio::test]
async fn test_list_mcp_tokens_empty() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["tokens"].is_array());
    assert_eq!(body["tokens"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_list_mcp_tokens_with_tokens() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Create a token first
    let create_request = json!({
        "name": "Test Token"
    });

    AxumTestRequest::post("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .json(&create_request)
        .send(routes.clone())
        .await;

    // Now list tokens
    let response = AxumTestRequest::get("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tokens = body["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0]["name"], "Test Token");
    assert!(!tokens[0]["is_revoked"].as_bool().unwrap());
}

#[tokio::test]
async fn test_list_mcp_tokens_requires_auth() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/api/user/mcp-tokens")
        .send(routes)
        .await;

    assert!(
        response.status() == 400 || response.status() == 401,
        "Expected 400 or 401, got {}",
        response.status()
    );
}

// ============================================================================
// DELETE /api/user/mcp-tokens/:token_id - Token Revocation Tests
// ============================================================================

#[tokio::test]
async fn test_revoke_mcp_token_success() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Create a token first
    let create_request = json!({
        "name": "Token to Revoke"
    });

    let create_response = AxumTestRequest::post("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .json(&create_request)
        .send(routes.clone())
        .await;

    let created_body: serde_json::Value = create_response.json();
    let token_id = created_body["id"].as_str().unwrap();

    // Revoke the token
    let response = AxumTestRequest::delete(&format!("/api/user/mcp-tokens/{}", token_id))
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .send(routes.clone())
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["success"].as_bool().unwrap());

    // Verify token is revoked in the list
    let list_response = AxumTestRequest::get("/api/user/mcp-tokens")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .send(routes)
        .await;

    let list_body: serde_json::Value = list_response.json();
    let tokens = list_body["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 1);
    assert!(tokens[0]["is_revoked"].as_bool().unwrap());
}

#[tokio::test]
async fn test_revoke_mcp_token_not_found() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::delete("/api/user/mcp-tokens/nonexistent-token-id")
        .header("Authorization", &format!("Bearer {}", setup.user_jwt))
        .send(routes)
        .await;

    assert_eq!(response.status(), 404);
}

#[tokio::test]
async fn test_revoke_mcp_token_requires_auth() {
    let setup = UserMcpTokenTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::delete("/api/user/mcp-tokens/some-token-id")
        .send(routes)
        .await;

    assert!(
        response.status() == 400 || response.status() == 401,
        "Expected 400 or 401, got {}",
        response.status()
    );
}

// ============================================================================
// Database Layer Tests
// ============================================================================

#[tokio::test]
async fn test_database_create_user_mcp_token() {
    common::init_server_config();
    let database = common::create_test_database()
        .await
        .expect("Failed to create database");
    let (user_id, _user) = common::create_test_user(&database)
        .await
        .expect("Failed to create user");

    let request = CreateUserMcpTokenRequest {
        name: "Database Test Token".to_owned(),
        expires_in_days: Some(30),
    };

    let result = database.create_user_mcp_token(user_id, &request).await;
    assert!(result.is_ok(), "Token creation should succeed");

    let created = result.unwrap();
    assert_eq!(created.token.name, "Database Test Token");
    assert!(created.token_value.starts_with("pmcp_"));
    assert!(!created.token.is_revoked);
    assert_eq!(created.token.usage_count, 0);
}

#[tokio::test]
async fn test_database_validate_user_mcp_token() {
    common::init_server_config();
    let database = common::create_test_database()
        .await
        .expect("Failed to create database");
    let (user_id, _user) = common::create_test_user(&database)
        .await
        .expect("Failed to create user");

    // Create token
    let request = CreateUserMcpTokenRequest {
        name: "Validation Test Token".to_owned(),
        expires_in_days: None,
    };

    let created = database
        .create_user_mcp_token(user_id, &request)
        .await
        .unwrap();

    // Validate token
    let validated_user_id = database.validate_user_mcp_token(&created.token_value).await;
    assert!(validated_user_id.is_ok());
    assert_eq!(validated_user_id.unwrap(), user_id);
}

#[tokio::test]
async fn test_database_validate_revoked_token_fails() {
    common::init_server_config();
    let database = common::create_test_database()
        .await
        .expect("Failed to create database");
    let (user_id, _user) = common::create_test_user(&database)
        .await
        .expect("Failed to create user");

    // Create token
    let request = CreateUserMcpTokenRequest {
        name: "Revocation Test Token".to_owned(),
        expires_in_days: None,
    };

    let created = database
        .create_user_mcp_token(user_id, &request)
        .await
        .unwrap();

    // Revoke token
    database
        .revoke_user_mcp_token(&created.token.id, user_id)
        .await
        .unwrap();

    // Validation should fail
    let result = database.validate_user_mcp_token(&created.token_value).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_database_list_user_mcp_tokens() {
    common::init_server_config();
    let database = common::create_test_database()
        .await
        .expect("Failed to create database");
    let (user_id, _user) = common::create_test_user(&database)
        .await
        .expect("Failed to create user");

    // Create multiple tokens
    for i in 1..=3 {
        let request = CreateUserMcpTokenRequest {
            name: format!("Token {}", i),
            expires_in_days: None,
        };
        database
            .create_user_mcp_token(user_id, &request)
            .await
            .unwrap();
    }

    // List tokens
    let tokens = database.list_user_mcp_tokens(user_id).await.unwrap();
    assert_eq!(tokens.len(), 3);
}

#[tokio::test]
async fn test_database_usage_count_increment() {
    common::init_server_config();
    let database = common::create_test_database()
        .await
        .expect("Failed to create database");
    let (user_id, _user) = common::create_test_user(&database)
        .await
        .expect("Failed to create user");

    // Create token
    let request = CreateUserMcpTokenRequest {
        name: "Usage Count Token".to_owned(),
        expires_in_days: None,
    };

    let created = database
        .create_user_mcp_token(user_id, &request)
        .await
        .unwrap();
    assert_eq!(created.token.usage_count, 0);

    // Validate token multiple times (should increment usage count)
    for _ in 0..3 {
        database
            .validate_user_mcp_token(&created.token_value)
            .await
            .unwrap();
    }

    // Check usage count
    let tokens = database.list_user_mcp_tokens(user_id).await.unwrap();
    assert_eq!(tokens[0].usage_count, 3);
}

#[tokio::test]
async fn test_database_token_isolation_between_users() {
    common::init_server_config();
    let database = common::create_test_database()
        .await
        .expect("Failed to create database");

    // Create two users
    let (user1_id, _user1) = common::create_test_user_with_email(&database, "user1@example.com")
        .await
        .expect("Failed to create user1");
    let (user2_id, _user2) = common::create_test_user_with_email(&database, "user2@example.com")
        .await
        .expect("Failed to create user2");

    // Create tokens for each user
    let request1 = CreateUserMcpTokenRequest {
        name: "User1 Token".to_owned(),
        expires_in_days: None,
    };
    let request2 = CreateUserMcpTokenRequest {
        name: "User2 Token".to_owned(),
        expires_in_days: None,
    };

    let token1 = database
        .create_user_mcp_token(user1_id, &request1)
        .await
        .unwrap();
    database
        .create_user_mcp_token(user2_id, &request2)
        .await
        .unwrap();

    // User1 should only see their own token
    let user1_tokens = database.list_user_mcp_tokens(user1_id).await.unwrap();
    assert_eq!(user1_tokens.len(), 1);
    assert_eq!(user1_tokens[0].name, "User1 Token");

    // User2 should only see their own token
    let user2_tokens = database.list_user_mcp_tokens(user2_id).await.unwrap();
    assert_eq!(user2_tokens.len(), 1);
    assert_eq!(user2_tokens[0].name, "User2 Token");

    // User2 cannot revoke User1's token
    let revoke_result = database
        .revoke_user_mcp_token(&token1.token.id, user2_id)
        .await;
    assert!(revoke_result.is_err());
}
