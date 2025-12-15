// ABOUTME: End-to-end integration test for complete admin setup and user approval workflow
// ABOUTME: Tests server-first admin creation, user registration, and approval process with database cleanup
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;
mod helpers;

use anyhow::Result;
use helpers::axum_test::AxumTestRequest;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    admin::AdminAuthService,
    auth::AuthManager,
    constants::system_config::STARTER_MONTHLY_LIMIT,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{User, UserStatus, UserTier},
    permissions::UserRole,
    routes::admin::{AdminApiContext, AdminRoutes},
};
use serde_json::Value;
use std::{env, fs, sync::Arc};

/// Complete end-to-end test for admin setup and user approval workflow
// Long function: Comprehensive test covering admin setup, user creation, approval, and cleanup
#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn test_complete_admin_user_approval_workflow() -> Result<()> {
    // Initialize test database with cleanup
    let database_url = if env::var("CI").is_ok() {
        "sqlite::memory:".to_owned()
    } else {
        let database_path = "./test_data/admin_approval_e2e_test.db";
        let _ = fs::remove_file(database_path); // Clean up any existing test database
        format!("sqlite:{database_path}")
    };
    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        b"test_encryption_key_32_bytes_long".to_vec(),
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    // Initialize JWT secret
    let jwt_secret = "test_jwt_secret_for_admin_approval_e2e_testing";
    let auth_manager = AuthManager::new(24);

    // Create JWKS manager for RS256 with 2048-bit test keys for faster execution
    let jwks_manager = common::get_shared_test_jwks();

    // Create admin API context
    let admin_api_key_monthly_limit = STARTER_MONTHLY_LIMIT;
    let admin_context = AdminApiContext::new(
        Arc::new(database.clone()),
        jwt_secret,
        Arc::new(auth_manager.clone()),
        jwks_manager.clone(),
        admin_api_key_monthly_limit,
        AdminAuthService::DEFAULT_CACHE_TTL_SECS,
    );

    // Create admin routes
    let admin_routes = AdminRoutes::routes(admin_context);

    println!("Starting complete admin user approval workflow test");

    // Step 1: Create admin user via server-first setup endpoint
    println!("1️⃣ Testing admin setup endpoint...");
    let admin_setup_response = AxumTestRequest::post("/admin/setup")
        .json(&serde_json::json!({
            "email": "test_admin@example.com",
            "password": "admin_password_123",
            "display_name": "Test Admin"
        }))
        .send(admin_routes.clone())
        .await;

    assert_eq!(admin_setup_response.status(), 201);
    let response_bytes = admin_setup_response.bytes();
    let admin_body: Value = serde_json::from_slice(&response_bytes)?;
    let admin_token = admin_body["data"]["admin_token"]
        .as_str()
        .expect("Admin token should be present")
        .to_owned();

    println!(
        " Admin created successfully with token: {}...",
        &admin_token[0..20]
    );

    // Step 2: Register a regular user (this would normally be done via main API)
    println!("2️⃣ Creating test user directly in database...");
    let test_user = User {
        id: uuid::Uuid::new_v4(),
        email: "test_user@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Starter,
        tenant_id: Some("test_tenant".to_owned()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending, // Start as pending
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };

    let user_id = database.create_user(&test_user).await?;
    println!(" Test user created with ID: {user_id}");

    // Step 3: Verify user is in pending status
    println!("3️⃣ Verifying user is in pending status...");
    let pending_users_response = AxumTestRequest::get("/admin/pending-users")
        .header("Authorization", &format!("Bearer {admin_token}"))
        .send(admin_routes.clone())
        .await;

    let status = pending_users_response.status();
    println!("Pending users response status: {status}");
    let response_bytes = pending_users_response.bytes();
    if status != 200 {
        println!(
            "Response body: {}",
            String::from_utf8_lossy(&response_bytes)
        );
    }
    assert_eq!(status, 200);
    let pending_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(pending_body["success"], true);
    assert!(pending_body["data"]["count"].as_u64().unwrap() > 0);
    println!(" User found in pending users list");

    // Step 4: Approve the user using admin token
    println!("4️⃣ Testing user approval...");
    let approval_response = AxumTestRequest::post(&format!("/admin/approve-user/{user_id}"))
        .header("Authorization", &format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "reason": "End-to-end test approval"
        }))
        .send(admin_routes.clone())
        .await;

    assert_eq!(approval_response.status(), 200);
    let response_bytes = approval_response.bytes();
    let approval_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(approval_body["success"], true);
    assert_eq!(approval_body["data"]["user"]["user_status"], "active");
    assert!(approval_body["data"]["user"]["approved_at"].is_string());

    println!(" User approved successfully");

    // Step 5: Verify user is no longer in pending list
    println!("5️⃣ Verifying user is no longer pending...");
    let pending_users_response_after = AxumTestRequest::get("/admin/pending-users")
        .header("Authorization", &format!("Bearer {admin_token}"))
        .send(admin_routes.clone())
        .await;

    assert_eq!(pending_users_response_after.status(), 200);
    let response_bytes = pending_users_response_after.bytes();
    let pending_body_after: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(pending_body_after["success"], true);

    // The user should no longer be in pending status
    let remaining_pending_count = pending_body_after["data"]["count"].as_u64().unwrap();
    assert_eq!(
        remaining_pending_count, 0,
        "No users should be pending after approval"
    );

    println!(" User successfully removed from pending list");

    // Step 6: Test that we can't create another admin (conflict handling)
    println!("6️⃣ Testing admin conflict prevention...");
    let duplicate_admin_response = AxumTestRequest::post("/admin/setup")
        .json(&serde_json::json!({
            "email": "another_admin@example.com",
            "password": "another_password",
            "display_name": "Another Admin"
        }))
        .send(admin_routes.clone())
        .await;

    assert_eq!(duplicate_admin_response.status(), 409); // Conflict
    let response_bytes = duplicate_admin_response.bytes();
    let duplicate_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(duplicate_body["success"], false);
    assert!(duplicate_body["message"]
        .as_str()
        .unwrap()
        .contains("Admin user already exists"));

    println!(" Admin conflict prevention working correctly");

    // Cleanup: Remove test database (only in local environment)
    if env::var("CI").is_err() {
        if let Ok(database_path) = env::var("TEST_DATABASE_PATH") {
            let _ = fs::remove_file(&database_path);
            println!("Test database cleaned up");
        }
    }

    println!("COMPLETE ADMIN USER APPROVAL WORKFLOW TEST PASSED!");
    println!(" Server-first admin setup working");
    println!(" User approval workflow working");
    println!(" Database state transitions correct");
    println!(" Authorization working properly");
    println!(" Conflict handling working");

    Ok(())
}

/// Test admin token management functionality
#[tokio::test]
async fn test_admin_token_management_workflow() -> Result<()> {
    // Initialize test database
    let database_url = if env::var("CI").is_ok() {
        "sqlite::memory:".to_owned()
    } else {
        let database_path = "./test_data/admin_token_mgmt_test.db";
        let _ = fs::remove_file(database_path);
        format!("sqlite:{database_path}")
    };
    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        b"test_encryption_key_32_bytes_long".to_vec(),
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    let jwt_secret = "test_jwt_secret_for_token_management";
    let auth_manager = AuthManager::new(24);

    // Create JWKS manager for RS256 with 2048-bit test keys for faster execution
    let jwks_manager = common::get_shared_test_jwks();

    let admin_api_key_monthly_limit = STARTER_MONTHLY_LIMIT;
    let admin_context = AdminApiContext::new(
        Arc::new(database.clone()),
        jwt_secret,
        Arc::new(auth_manager),
        jwks_manager.clone(),
        admin_api_key_monthly_limit,
        AdminAuthService::DEFAULT_CACHE_TTL_SECS,
    );
    let admin_routes = AdminRoutes::routes(admin_context);

    println!("Starting admin token management workflow test");

    // Step 1: Create initial admin
    let admin_setup_response = AxumTestRequest::post("/admin/setup")
        .json(&serde_json::json!({
            "email": "token_admin@example.com",
            "password": "admin_pass_123",
            "display_name": "Token Admin"
        }))
        .send(admin_routes.clone())
        .await;

    assert_eq!(admin_setup_response.status(), 201);
    let response_bytes = admin_setup_response.bytes();
    let admin_body: Value = serde_json::from_slice(&response_bytes)?;
    let admin_token = admin_body["data"]["admin_token"]
        .as_str()
        .unwrap()
        .to_owned();

    // Step 2: Create additional admin token
    let create_token_response = AxumTestRequest::post("/admin/tokens")
        .header("Authorization", &format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "service_name": "test_service_token",
            "service_description": "Test service token",
            "is_super_admin": false,
            "expires_in_days": 30,
            "permissions": ["manage_users"]
        }))
        .send(admin_routes.clone())
        .await;

    assert_eq!(create_token_response.status(), 201);
    let response_bytes = create_token_response.bytes();
    let create_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(create_body["success"], true);

    let service_token_id = create_body["data"]["token_id"].as_str().unwrap();
    println!(" Service token created: {service_token_id}");

    // Step 3: List admin tokens
    let list_tokens_response = AxumTestRequest::get("/admin/tokens")
        .header("Authorization", &format!("Bearer {admin_token}"))
        .send(admin_routes.clone())
        .await;

    assert_eq!(list_tokens_response.status(), 200);
    let response_bytes = list_tokens_response.bytes();
    let list_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(list_body["success"], true);
    assert!(list_body["data"]["count"].as_u64().unwrap() >= 2); // At least initial + service token

    println!(" Token listing working");

    // Step 4: Get token details
    let token_details_response = AxumTestRequest::get(&format!("/admin/tokens/{service_token_id}"))
        .header("Authorization", &format!("Bearer {admin_token}"))
        .send(admin_routes.clone())
        .await;

    assert_eq!(token_details_response.status(), 200);
    let response_bytes = token_details_response.bytes();
    let details_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(details_body["success"], true);
    assert_eq!(details_body["data"]["service_name"], "test_service_token");

    println!(" Token details retrieval working");

    // Step 5: Revoke the service token
    let revoke_response =
        AxumTestRequest::post(&format!("/admin/tokens/{service_token_id}/revoke"))
            .header("Authorization", &format!("Bearer {admin_token}"))
            .send(admin_routes.clone())
            .await;

    assert_eq!(revoke_response.status(), 200);
    let response_bytes = revoke_response.bytes();
    let revoke_body: Value = serde_json::from_slice(&response_bytes)?;
    assert_eq!(revoke_body["success"], true);

    println!(" Token revocation working");

    // Cleanup: Remove test database (only in local environment)
    if env::var("CI").is_err() && database_url.starts_with("sqlite:./") {
        let _ = fs::remove_file(&database_url[7..]); // Remove "sqlite:" prefix
    }

    println!("ADMIN TOKEN MANAGEMENT WORKFLOW TEST PASSED!");

    Ok(())
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_admin_workflow_error_handling() -> Result<()> {
    let database_url = if env::var("CI").is_ok() {
        "sqlite::memory:".to_owned()
    } else {
        let database_path = "./test_data/admin_error_handling_test.db";
        let _ = fs::remove_file(database_path);
        format!("sqlite:{database_path}")
    };
    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        b"test_encryption_key_32_bytes_long".to_vec(),
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    let jwt_secret = "test_jwt_secret_for_error_handling";
    let auth_manager = AuthManager::new(24);

    // Create JWKS manager for RS256 with 2048-bit test keys for faster execution
    let jwks_manager = common::get_shared_test_jwks();

    let admin_api_key_monthly_limit = STARTER_MONTHLY_LIMIT;
    let admin_context = AdminApiContext::new(
        Arc::new(database),
        jwt_secret,
        Arc::new(auth_manager),
        jwks_manager,
        admin_api_key_monthly_limit,
        AdminAuthService::DEFAULT_CACHE_TTL_SECS,
    );
    let admin_routes = AdminRoutes::routes(admin_context);

    println!("Starting admin workflow error handling test");

    // Test 1: Try to approve non-existent user
    let fake_admin_token = "fake_token_12345";
    let fake_user_id = uuid::Uuid::new_v4();

    let approve_fake_user_response =
        AxumTestRequest::post(&format!("/admin/approve-user/{fake_user_id}"))
            .header("Authorization", &format!("Bearer {fake_admin_token}"))
            .json(&serde_json::json!({"reason": "Test"}))
            .send(admin_routes.clone())
            .await;

    // Should fail with unauthorized due to invalid token
    assert_eq!(approve_fake_user_response.status(), 401);
    println!(" Invalid token properly rejected");

    // Test 2: Try to access admin endpoints without token
    let no_auth_response = AxumTestRequest::get("/admin/pending-users")
        .send(admin_routes.clone())
        .await;

    assert_eq!(no_auth_response.status(), 400); // Missing auth header
    println!(" Missing authorization properly rejected");

    // Test 3: Try to approve with malformed user ID
    let admin_setup_response = AxumTestRequest::post("/admin/setup")
        .json(&serde_json::json!({
            "email": "error_admin@example.com",
            "password": "admin_pass_123",
            "display_name": "Error Test Admin"
        }))
        .send(admin_routes.clone())
        .await;

    let response_bytes = admin_setup_response.bytes();
    let admin_body: Value = serde_json::from_slice(&response_bytes)?;
    let admin_token = admin_body["data"]["admin_token"].as_str().unwrap();

    let malformed_id_response = AxumTestRequest::post("/admin/approve-user/not-a-uuid")
        .header("Authorization", &format!("Bearer {admin_token}"))
        .json(&serde_json::json!({"reason": "Test"}))
        .send(admin_routes.clone())
        .await;

    assert_eq!(malformed_id_response.status(), 400); // Bad request for malformed UUID
    println!(" Malformed UUID properly rejected");

    // Cleanup: Remove test database (only in local environment)
    if env::var("CI").is_err() && database_url.starts_with("sqlite:./") {
        let _ = fs::remove_file(&database_url[7..]); // Remove "sqlite:" prefix
    }

    println!("ADMIN WORKFLOW ERROR HANDLING TEST PASSED!");

    Ok(())
}

/// Helper: Setup admin and get token
async fn setup_admin_and_get_token(admin_routes: axum::Router) -> Result<String> {
    let admin_setup_response = AxumTestRequest::post("/admin/setup")
        .json(&serde_json::json!({
            "email": "admin@example.com",
            "password": "admin_pass",
            "display_name": "Admin"
        }))
        .send(admin_routes)
        .await;

    assert_eq!(admin_setup_response.status(), 201);
    let admin_body: Value = serde_json::from_slice(&admin_setup_response.bytes())?;
    Ok(admin_body["data"]["admin_token"]
        .as_str()
        .unwrap()
        .to_owned())
}

/// Helper: Create pending user for testing
async fn create_test_pending_user(database: &Database) -> Result<uuid::Uuid> {
    let test_user_id = uuid::Uuid::new_v4();
    let test_user = User {
        id: test_user_id,
        email: "user@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: "dummy_hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: UserStatus::Pending,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: None,
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&test_user).await?;
    Ok(test_user_id)
}

/// Helper: Verify tenant and user linkage
async fn verify_tenant_user_linkage(
    database: &Database,
    tenant_id: uuid::Uuid,
    test_user_id: uuid::Uuid,
    expected_tenant_name: &str,
    expected_tenant_slug: &str,
) -> Result<()> {
    let created_tenant = database.get_tenant_by_id(tenant_id).await?;
    assert_eq!(created_tenant.name, expected_tenant_name);
    assert_eq!(created_tenant.slug, expected_tenant_slug);
    assert_eq!(created_tenant.plan, "starter");
    assert_eq!(created_tenant.owner_user_id, test_user_id);

    let updated_user = database.get_user(test_user_id).await?.unwrap();
    assert_eq!(updated_user.tenant_id, Some(tenant_id.to_string()));
    Ok(())
}

/// Test user approval with automatic tenant creation
#[tokio::test]
async fn test_user_approval_with_tenant_creation() -> Result<()> {
    let database_url = if env::var("CI").is_ok() {
        "sqlite::memory:".to_owned()
    } else {
        let database_path = "./test_data/admin_approval_with_tenant_test.db";
        let _ = fs::remove_file(database_path);
        format!("sqlite:{database_path}")
    };

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        b"test_encryption_key_32_bytes_long".to_vec(),
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    let jwt_secret = "test_jwt_secret_for_tenant_creation";
    let auth_manager = AuthManager::new(24);
    let jwks_manager = common::get_shared_test_jwks();

    let admin_context = AdminApiContext::new(
        Arc::new(database.clone()),
        jwt_secret,
        Arc::new(auth_manager.clone()),
        jwks_manager.clone(),
        STARTER_MONTHLY_LIMIT,
        AdminAuthService::DEFAULT_CACHE_TTL_SECS,
    );

    let admin_routes = AdminRoutes::routes(admin_context);

    println!("Testing user approval with tenant creation");

    // Create admin and get token
    let admin_token = setup_admin_and_get_token(admin_routes.clone()).await?;

    // Create pending user
    let test_user_id = create_test_pending_user(&database).await?;
    println!(" Pending user created");

    // Approve user WITH tenant creation
    let approve_response = AxumTestRequest::post(&format!("/admin/approve-user/{test_user_id}"))
        .header("Authorization", &format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "reason": "Approved for testing",
            "create_default_tenant": true,
            "tenant_name": "Test Organization",
            "tenant_slug": "test-org"
        }))
        .send(admin_routes.clone())
        .await;

    assert_eq!(approve_response.status(), 200);
    let approve_body: Value = serde_json::from_slice(&approve_response.bytes())?;

    println!(
        "Approval response: {}",
        serde_json::to_string_pretty(&approve_body)?
    );

    // Verify response contains tenant_created
    assert!(approve_body["success"].as_bool().unwrap());
    assert!(approve_body["data"]["tenant_created"].is_object());

    let tenant_created = &approve_body["data"]["tenant_created"];
    assert_eq!(
        tenant_created["name"].as_str().unwrap(),
        "Test Organization"
    );
    assert_eq!(tenant_created["slug"].as_str().unwrap(), "test-org");
    assert_eq!(tenant_created["plan"].as_str().unwrap(), "starter");

    let tenant_id_str = tenant_created["tenant_id"].as_str().unwrap();
    let tenant_id = uuid::Uuid::parse_str(tenant_id_str)?;

    println!(
        " Tenant created: {} ({})",
        tenant_created["name"], tenant_id
    );

    // Verify tenant and user linkage
    verify_tenant_user_linkage(
        &database,
        tenant_id,
        test_user_id,
        "Test Organization",
        "test-org",
    )
    .await?;
    println!(" Tenant and user linkage verified");

    // Cleanup
    if env::var("CI").is_err() && database_url.starts_with("sqlite:./") {
        let _ = fs::remove_file(&database_url[7..]);
    }

    println!("USER APPROVAL WITH TENANT CREATION TEST PASSED!");

    Ok(())
}
