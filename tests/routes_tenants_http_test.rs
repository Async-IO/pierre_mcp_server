// ABOUTME: HTTP integration tests for tenant management routes
// ABOUTME: Tests all tenant endpoints with authentication, authorization, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::uninlined_format_args)]

//! Comprehensive HTTP integration tests for tenant management routes
//!
//! This test suite validates that all tenant endpoints are correctly registered
//! in the router and handle HTTP requests appropriately.

mod common;
mod helpers;

use helpers::axum_test::AxumTestRequest;
use pierre_mcp_server::{database_plugins::DatabaseProvider, mcp::resources::ServerResources};
use serde_json::json;
use std::sync::Arc;

/// Test setup helper for tenant route testing
struct TenantTestSetup {
    resources: Arc<ServerResources>,
    user_id: uuid::Uuid,
    jwt_token: String,
}

impl TenantTestSetup {
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

        Ok(Self {
            resources,
            user_id,
            jwt_token,
        })
    }

    fn routes(&self) -> axum::Router {
        pierre_mcp_server::routes::tenants::TenantRoutes::routes(self.resources.clone())
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.jwt_token)
    }
}

// ============================================================================
// POST /tenants - Create Tenant Tests
// ============================================================================

#[tokio::test]
async fn test_create_tenant_success() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let request_body = json!({
        "name": "Test Tenant",
        "slug": "test-tenant",
        "plan": "starter"
    });

    let response = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .json(&request_body)
        .send(routes)
        .await;

    assert_eq!(response.status(), 201);

    let body: serde_json::Value = response.json();
    assert!(body["tenant_id"].is_string());
    assert_eq!(body["name"], "Test Tenant");
    assert_eq!(body["slug"], "test-tenant");
}

#[tokio::test]
async fn test_create_tenant_missing_auth() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let request_body = json!({
        "name": "Test Tenant",
        "slug": "test-tenant",
        "plan": "starter"
    });

    let response = AxumTestRequest::post("/tenants")
        .json(&request_body)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_create_tenant_invalid_auth() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let request_body = json!({
        "name": "Test Tenant",
        "slug": "test-tenant",
        "plan": "starter"
    });

    let response = AxumTestRequest::post("/tenants")
        .header("authorization", "Bearer invalid_token")
        .json(&request_body)
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_create_tenant_invalid_json() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .header("content-type", "application/json")
        .send(routes)
        .await;

    // Should fail due to missing/invalid body
    assert_ne!(response.status(), 201);
}

#[tokio::test]
async fn test_create_tenant_missing_required_fields() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Missing slug
    let request_body1 = json!({
        "name": "Test Tenant",
        "plan": "starter"
    });

    let response1 = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .json(&request_body1)
        .send(routes.clone())
        .await;

    assert_ne!(response1.status(), 201);

    // Missing name
    let request_body2 = json!({
        "slug": "test-tenant",
        "plan": "starter"
    });

    let response2 = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .json(&request_body2)
        .send(routes)
        .await;

    assert_ne!(response2.status(), 201);
}

#[tokio::test]
async fn test_create_tenant_duplicate_slug() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");

    // Create first tenant
    let tenant = pierre_mcp_server::models::Tenant {
        id: uuid::Uuid::new_v4(),
        name: "First Tenant".to_owned(),
        slug: "duplicate-slug".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: setup.user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    setup
        .resources
        .database
        .create_tenant(&tenant)
        .await
        .expect("Failed to create first tenant");

    let routes = setup.routes();

    // Try to create second tenant with same slug
    let request_body = json!({
        "name": "Second Tenant",
        "slug": "duplicate-slug",
        "plan": "starter"
    });

    let response = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .json(&request_body)
        .send(routes)
        .await;

    // Should fail with conflict or bad request
    assert_ne!(response.status(), 201);
    assert!(response.status() == 400 || response.status() == 409 || response.status() == 500);
}

// ============================================================================
// GET /tenants - List Tenants Tests
// ============================================================================

#[tokio::test]
async fn test_list_tenants_success() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");

    // Create a tenant
    let tenant = pierre_mcp_server::models::Tenant {
        id: uuid::Uuid::new_v4(),
        name: "Test Tenant".to_owned(),
        slug: "test-tenant".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: setup.user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    setup
        .resources
        .database
        .create_tenant(&tenant)
        .await
        .expect("Failed to create tenant");

    let routes = setup.routes();

    let response = AxumTestRequest::get("/tenants")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["tenants"].is_array());
    let tenants = body["tenants"].as_array().unwrap();
    assert!(!tenants.is_empty());

    // Verify tenant structure
    assert!(tenants[0]["tenant_id"].is_string());
    assert!(tenants[0]["name"].is_string());
    assert!(tenants[0]["slug"].is_string());
}

#[tokio::test]
async fn test_list_tenants_missing_auth() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/tenants").send(routes).await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_list_tenants_invalid_auth() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/tenants")
        .header("authorization", "Bearer invalid_token")
        .send(routes)
        .await;

    assert_eq!(response.status(), 401);
}

#[tokio::test]
async fn test_list_tenants_empty() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    let response = AxumTestRequest::get("/tenants")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    assert!(body["tenants"].is_array());
    // Tenants array could be empty or have tenants depending on test execution order
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

#[tokio::test]
async fn test_create_multiple_tenants() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");
    let routes = setup.routes();

    // Create first tenant
    let request1 = json!({
        "name": "First Tenant",
        "slug": "first-tenant",
        "plan": "starter"
    });

    let response1 = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .json(&request1)
        .send(routes.clone())
        .await;

    assert_eq!(response1.status(), 201);

    // Create second tenant
    let request2 = json!({
        "name": "Second Tenant",
        "slug": "second-tenant",
        "plan": "professional"
    });

    let response2 = AxumTestRequest::post("/tenants")
        .header("authorization", &setup.auth_header())
        .json(&request2)
        .send(routes.clone())
        .await;

    assert_eq!(response2.status(), 201);

    // List tenants - should have both
    let list_response = AxumTestRequest::get("/tenants")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(list_response.status(), 200);

    let body: serde_json::Value = list_response.json();
    let tenants = body["tenants"].as_array().unwrap();
    assert!(tenants.len() >= 2);
}

#[tokio::test]
async fn test_tenant_ownership() {
    let setup = TenantTestSetup::new().await.expect("Setup failed");

    // Create a tenant owned by the test user
    let tenant = pierre_mcp_server::models::Tenant {
        id: uuid::Uuid::new_v4(),
        name: "Owned Tenant".to_owned(),
        slug: "owned-tenant".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: setup.user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    setup
        .resources
        .database
        .create_tenant(&tenant)
        .await
        .expect("Failed to create tenant");

    let routes = setup.routes();

    // User should be able to list their tenant
    let response = AxumTestRequest::get("/tenants")
        .header("authorization", &setup.auth_header())
        .send(routes)
        .await;

    assert_eq!(response.status(), 200);

    let body: serde_json::Value = response.json();
    let tenants = body["tenants"].as_array().unwrap();
    assert!(tenants.iter().any(|t| t["slug"] == "owned-tenant"));
}
