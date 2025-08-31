// ABOUTME: End-to-end integration test for complete admin setup and user approval workflow
// ABOUTME: Tests server-first admin creation, user registration, and approval process with database cleanup

use anyhow::Result;
use pierre_mcp_server::{
    admin_routes::AdminApiContext,
    auth::AuthManager,
    database_plugins::{factory::Database, DatabaseProvider},
};
use serde_json::Value;
use warp::test::request;

/// Complete end-to-end test for admin setup and user approval workflow
// Long function: Comprehensive test covering admin setup, user creation, approval, and cleanup
#[allow(clippy::too_many_lines)]
#[tokio::test]
async fn test_complete_admin_user_approval_workflow() -> Result<()> {
    // Initialize test database with cleanup
    let database_url = if std::env::var("CI").is_ok() {
        "sqlite::memory:".to_string()
    } else {
        let database_path = "./test_data/admin_approval_e2e_test.db";
        let _ = std::fs::remove_file(database_path); // Clean up any existing test database
        format!("sqlite:{database_path}")
    };
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    // Initialize JWT secret
    let jwt_secret = "test_jwt_secret_for_admin_approval_e2e_testing";
    let auth_manager = AuthManager::new(jwt_secret.as_bytes().to_vec(), 24);

    // Create admin API context
    let admin_context = AdminApiContext::new(database.clone(), jwt_secret, auth_manager.clone());

    // Create admin routes
    let admin_routes =
        pierre_mcp_server::admin_routes::admin_routes_with_scoped_recovery(admin_context);

    println!("Starting complete admin user approval workflow test");

    // Step 1: Create admin user via server-first setup endpoint
    println!("1️⃣ Testing admin setup endpoint...");
    let admin_setup_response = request()
        .method("POST")
        .path("/admin/setup")
        .json(&serde_json::json!({
            "email": "test_admin@example.com",
            "password": "admin_password_123",
            "display_name": "Test Admin"
        }))
        .reply(&admin_routes)
        .await;

    assert_eq!(admin_setup_response.status(), 201);
    let admin_body: Value = serde_json::from_slice(admin_setup_response.body())?;
    let admin_token = admin_body["admin_token"]
        .as_str()
        .expect("Admin token should be present")
        .to_string();

    println!(
        "✅ Admin created successfully with token: {}...",
        &admin_token[0..20]
    );

    // Step 2: Register a regular user (this would normally be done via main API)
    println!("2️⃣ Creating test user directly in database...");
    let test_user = pierre_mcp_server::models::User {
        id: uuid::Uuid::new_v4(),
        email: "test_user@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        tenant_id: Some("test_tenant".to_string()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Pending, // Start as pending
        is_admin: false,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    let user_id = database.create_user(&test_user).await?;
    println!("✅ Test user created with ID: {user_id}");

    // Step 3: Verify user is in pending status
    println!("3️⃣ Verifying user is in pending status...");
    let pending_users_response = request()
        .method("GET")
        .path("/admin/pending-users")
        .header("Authorization", format!("Bearer {admin_token}"))
        .reply(&admin_routes)
        .await;

    assert_eq!(pending_users_response.status(), 200);
    let pending_body: Value = serde_json::from_slice(pending_users_response.body())?;
    assert_eq!(pending_body["success"], true);
    assert!(pending_body["count"].as_u64().unwrap() > 0);
    println!("✅ User found in pending users list");

    // Step 4: Approve the user using admin token
    println!("4️⃣ Testing user approval...");
    let approval_response = request()
        .method("POST")
        .path(&format!("/admin/approve-user/{user_id}"))
        .header("Authorization", format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "reason": "End-to-end test approval"
        }))
        .reply(&admin_routes)
        .await;

    assert_eq!(approval_response.status(), 200);
    let approval_body: Value = serde_json::from_slice(approval_response.body())?;
    assert_eq!(approval_body["success"], true);
    assert_eq!(approval_body["user"]["user_status"], "active");
    assert!(approval_body["user"]["approved_at"].is_string());

    println!("✅ User approved successfully");

    // Step 5: Verify user is no longer in pending list
    println!("5️⃣ Verifying user is no longer pending...");
    let pending_users_response_after = request()
        .method("GET")
        .path("/admin/pending-users")
        .header("Authorization", format!("Bearer {admin_token}"))
        .reply(&admin_routes)
        .await;

    assert_eq!(pending_users_response_after.status(), 200);
    let pending_body_after: Value = serde_json::from_slice(pending_users_response_after.body())?;
    assert_eq!(pending_body_after["success"], true);

    // The user should no longer be in pending status
    let remaining_pending_count = pending_body_after["count"].as_u64().unwrap();
    assert_eq!(
        remaining_pending_count, 0,
        "No users should be pending after approval"
    );

    println!("✅ User successfully removed from pending list");

    // Step 6: Test that we can't create another admin (conflict handling)
    println!("6️⃣ Testing admin conflict prevention...");
    let duplicate_admin_response = request()
        .method("POST")
        .path("/admin/setup")
        .json(&serde_json::json!({
            "email": "another_admin@example.com",
            "password": "another_password",
            "display_name": "Another Admin"
        }))
        .reply(&admin_routes)
        .await;

    assert_eq!(duplicate_admin_response.status(), 409); // Conflict
    let duplicate_body: Value = serde_json::from_slice(duplicate_admin_response.body())?;
    assert_eq!(duplicate_body["success"], false);
    assert!(duplicate_body["message"]
        .as_str()
        .unwrap()
        .contains("Admin user already exists"));

    println!("✅ Admin conflict prevention working correctly");

    // Cleanup: Remove test database (only in local environment)
    if std::env::var("CI").is_err() {
        if let Ok(database_path) = std::env::var("TEST_DATABASE_PATH") {
            let _ = std::fs::remove_file(&database_path);
            println!("Test database cleaned up");
        }
    }

    println!("COMPLETE ADMIN USER APPROVAL WORKFLOW TEST PASSED!");
    println!("✅ Server-first admin setup working");
    println!("✅ User approval workflow working");
    println!("✅ Database state transitions correct");
    println!("✅ Authorization working properly");
    println!("✅ Conflict handling working");

    Ok(())
}

/// Test admin token management functionality
#[tokio::test]
async fn test_admin_token_management_workflow() -> Result<()> {
    // Initialize test database
    let database_url = if std::env::var("CI").is_ok() {
        "sqlite::memory:".to_string()
    } else {
        let database_path = "./test_data/admin_token_mgmt_test.db";
        let _ = std::fs::remove_file(database_path);
        format!("sqlite:{database_path}")
    };
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    let jwt_secret = "test_jwt_secret_for_token_management";
    let auth_manager = AuthManager::new(jwt_secret.as_bytes().to_vec(), 24);
    let admin_context = AdminApiContext::new(database.clone(), jwt_secret, auth_manager);
    let admin_routes =
        pierre_mcp_server::admin_routes::admin_routes_with_scoped_recovery(admin_context);

    println!("Starting admin token management workflow test");

    // Step 1: Create initial admin
    let admin_setup_response = request()
        .method("POST")
        .path("/admin/setup")
        .json(&serde_json::json!({
            "email": "token_admin@example.com",
            "password": "admin_pass_123",
            "display_name": "Token Admin"
        }))
        .reply(&admin_routes)
        .await;

    assert_eq!(admin_setup_response.status(), 201);
    let admin_body: Value = serde_json::from_slice(admin_setup_response.body())?;
    let admin_token = admin_body["admin_token"].as_str().unwrap().to_string();

    // Step 2: Create additional admin token
    let create_token_response = request()
        .method("POST")
        .path("/admin/tokens")
        .header("Authorization", format!("Bearer {admin_token}"))
        .json(&serde_json::json!({
            "service_name": "test_service_token",
            "service_description": "Test service token",
            "is_super_admin": false,
            "expires_in_days": 30,
            "permissions": ["manage_users"]
        }))
        .reply(&admin_routes)
        .await;

    assert_eq!(create_token_response.status(), 201);
    let create_body: Value = serde_json::from_slice(create_token_response.body())?;
    assert_eq!(create_body["success"], true);

    let service_token_id = create_body["data"]["token_id"].as_str().unwrap();
    println!("✅ Service token created: {service_token_id}");

    // Step 3: List admin tokens
    let list_tokens_response = request()
        .method("GET")
        .path("/admin/tokens")
        .header("Authorization", format!("Bearer {admin_token}"))
        .reply(&admin_routes)
        .await;

    assert_eq!(list_tokens_response.status(), 200);
    let list_body: Value = serde_json::from_slice(list_tokens_response.body())?;
    assert_eq!(list_body["success"], true);
    assert!(list_body["data"]["count"].as_u64().unwrap() >= 2); // At least initial + service token

    println!("✅ Token listing working");

    // Step 4: Get token details
    let token_details_response = request()
        .method("GET")
        .path(&format!("/admin/tokens/{service_token_id}"))
        .header("Authorization", format!("Bearer {admin_token}"))
        .reply(&admin_routes)
        .await;

    assert_eq!(token_details_response.status(), 200);
    let details_body: Value = serde_json::from_slice(token_details_response.body())?;
    assert_eq!(details_body["success"], true);
    assert_eq!(details_body["data"]["service_name"], "test_service_token");

    println!("✅ Token details retrieval working");

    // Step 5: Revoke the service token
    let revoke_response = request()
        .method("POST")
        .path(&format!("/admin/tokens/{service_token_id}/revoke"))
        .header("Authorization", format!("Bearer {admin_token}"))
        .reply(&admin_routes)
        .await;

    assert_eq!(revoke_response.status(), 200);
    let revoke_body: Value = serde_json::from_slice(revoke_response.body())?;
    assert_eq!(revoke_body["success"], true);

    println!("✅ Token revocation working");

    // Cleanup: Remove test database (only in local environment)
    if std::env::var("CI").is_err() && database_url.starts_with("sqlite:./") {
        let _ = std::fs::remove_file(&database_url[7..]); // Remove "sqlite:" prefix
    }

    println!("ADMIN TOKEN MANAGEMENT WORKFLOW TEST PASSED!");

    Ok(())
}

/// Test error handling and edge cases
#[tokio::test]
async fn test_admin_workflow_error_handling() -> Result<()> {
    let database_url = if std::env::var("CI").is_ok() {
        "sqlite::memory:".to_string()
    } else {
        let database_path = "./test_data/admin_error_handling_test.db";
        let _ = std::fs::remove_file(database_path);
        format!("sqlite:{database_path}")
    };
    let database =
        Database::new(&database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    let jwt_secret = "test_jwt_secret_for_error_handling";
    let auth_manager = AuthManager::new(jwt_secret.as_bytes().to_vec(), 24);
    let admin_context = AdminApiContext::new(database, jwt_secret, auth_manager);
    let admin_routes =
        pierre_mcp_server::admin_routes::admin_routes_with_scoped_recovery(admin_context);

    println!("Starting admin workflow error handling test");

    // Test 1: Try to approve non-existent user
    let fake_admin_token = "fake_token_12345";
    let fake_user_id = uuid::Uuid::new_v4();

    let approve_fake_user_response = request()
        .method("POST")
        .path(&format!("/admin/approve-user/{fake_user_id}"))
        .header("Authorization", format!("Bearer {fake_admin_token}"))
        .json(&serde_json::json!({"reason": "Test"}))
        .reply(&admin_routes)
        .await;

    // Should fail with unauthorized due to invalid token
    assert_eq!(approve_fake_user_response.status(), 401);
    println!("✅ Invalid token properly rejected");

    // Test 2: Try to access admin endpoints without token
    let no_auth_response = request()
        .method("GET")
        .path("/admin/pending-users")
        .reply(&admin_routes)
        .await;

    assert_eq!(no_auth_response.status(), 400); // Missing auth header
    println!("✅ Missing authorization properly rejected");

    // Test 3: Try to approve with malformed user ID
    let admin_setup_response = request()
        .method("POST")
        .path("/admin/setup")
        .json(&serde_json::json!({
            "email": "error_admin@example.com",
            "password": "admin_pass_123",
            "display_name": "Error Test Admin"
        }))
        .reply(&admin_routes)
        .await;

    let admin_body: Value = serde_json::from_slice(admin_setup_response.body())?;
    let admin_token = admin_body["admin_token"].as_str().unwrap();

    let malformed_id_response = request()
        .method("POST")
        .path("/admin/approve-user/not-a-uuid")
        .header("Authorization", format!("Bearer {admin_token}"))
        .json(&serde_json::json!({"reason": "Test"}))
        .reply(&admin_routes)
        .await;

    assert_eq!(malformed_id_response.status(), 400); // Bad request for malformed UUID
    println!("✅ Malformed UUID properly rejected");

    // Cleanup: Remove test database (only in local environment)
    if std::env::var("CI").is_err() && database_url.starts_with("sqlite:./") {
        let _ = std::fs::remove_file(&database_url[7..]); // Remove "sqlite:" prefix
    }

    println!("ADMIN WORKFLOW ERROR HANDLING TEST PASSED!");

    Ok(())
}
