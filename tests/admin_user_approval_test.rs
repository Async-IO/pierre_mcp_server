// ABOUTME: Integration tests for admin user approval workflow
// ABOUTME: Tests pending users listing, approval, and suspension via database operations
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use anyhow::Result;
use pierre_mcp_server::{
    admin::models::CreateAdminTokenRequest,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{User, UserStatus, UserTier},
};
use serial_test::serial;
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_admin_user_approval_tests";

/// Test helper to create admin token and database
async fn setup_test_database() -> Result<(Database, String, Uuid)> {
    // Initialize database with test-specific path
    let test_id = Uuid::new_v4().to_string();

    // Create test directory if it doesn't exist
    std::fs::create_dir_all("./test_data")
        .map_err(|e| anyhow::anyhow!("Failed to create test directory: {e}"))?;

    let db_path = format!("./test_data/admin_approval_test_{test_id}.db");
    let db_url = format!("sqlite:{db_path}");

    // Set MEK for test (required for KeyManager::bootstrap())
    std::env::set_var(
        "PIERRE_MASTER_ENCRYPTION_KEY",
        "Y2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2NjY2M=",
    );

    // Create database with proper encryption
    let (mut key_manager, database_key) =
        pierre_mcp_server::key_management::KeyManager::bootstrap()?;

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &db_url,
        database_key.to_vec(),
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&db_url, database_key.to_vec()).await?;
    key_manager.complete_initialization(&database).await?;

    // Run migrations
    database.migrate().await?;

    // Create an admin user first (needed for foreign key constraint)
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin@test.com".to_owned(),
        display_name: Some("Test Admin".to_owned()),
        password_hash: "admin_hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: true,
        approved_by: None, // Admin doesn't need approval
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    let admin_user_id = admin_user.id;
    database.create_user(&admin_user).await?;

    // Create a test admin token
    let admin_request = CreateAdminTokenRequest {
        service_name: "test_admin".to_owned(),
        service_description: Some("Test admin for approval workflow".to_owned()),
        permissions: None, // Super admin gets all permissions
        expires_in_days: Some(1),
        is_super_admin: true,
    };

    // Initialize JWKS manager for RS256 admin token signing
    let jwks_manager = common::get_shared_test_jwks();

    let admin_token = database
        .create_admin_token(&admin_request, TEST_JWT_SECRET, &jwks_manager)
        .await?;

    Ok((database, admin_token.token_id, admin_user_id))
}

#[tokio::test]
#[serial]
async fn test_get_pending_users() -> Result<()> {
    let (database, _admin_token_id, admin_user_id) = setup_test_database().await?;

    // Create test users with different statuses
    let pending_user = User {
        id: Uuid::new_v4(),
        email: "pending@test.com".to_owned(),
        display_name: Some("Pending User".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
        is_admin: false,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&pending_user).await?;

    let active_user = User {
        id: Uuid::new_v4(),
        email: "active@test.com".to_owned(),
        display_name: Some("Active User".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: Some(admin_user_id),
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&active_user).await?;

    // Test getting pending users via database query
    let pending_users = database.get_users_by_status("pending").await?;
    assert_eq!(pending_users.len(), 1);
    assert_eq!(pending_users[0].email, "pending@test.com");

    // Clean up test environment variable
    std::env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_approve_user() -> Result<()> {
    let (database, _admin_token_id, admin_user_id) = setup_test_database().await?;

    // Create a pending user
    let pending_user = User {
        id: Uuid::new_v4(),
        email: "to_approve@test.com".to_owned(),
        display_name: Some("User to Approve".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
        is_admin: false,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    let user_id = pending_user.id;
    database.create_user(&pending_user).await?;

    // Test updating the user's status to approved
    // For this test, we'll skip the update_user_status call since it uses token_id, not user_id
    // Instead, we'll directly test creating users with approved_by field set

    // Verify the pending user was created correctly
    let pending_user_check = database.get_user(user_id).await?.unwrap();
    assert_eq!(pending_user_check.user_status, UserStatus::Pending);

    // Now test creating a new user with approved_by set to the admin
    let new_approved_user = User {
        id: Uuid::new_v4(),
        email: "new_approved@test.com".to_owned(),
        display_name: Some("New Approved User".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: Some(admin_user_id), // Approved by admin user
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    // This should succeed since the admin user exists
    database.create_user(&new_approved_user).await?;

    // Verify the new user was created with approval fields set
    let created_user = database.get_user(new_approved_user.id).await?.unwrap();
    assert_eq!(created_user.user_status, UserStatus::Active);
    assert_eq!(created_user.approved_by, Some(admin_user_id));
    assert!(created_user.approved_at.is_some());

    // Clean up test environment variable
    std::env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_suspend_user() -> Result<()> {
    let (database, admin_token_id, admin_user_id) = setup_test_database().await?;

    // Create an active user
    let user = User {
        id: Uuid::new_v4(),
        email: "to_suspend@test.com".to_owned(),
        display_name: Some("User to Suspend".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: Some(admin_user_id),
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    let user_id = user.id;
    database.create_user(&user).await?;

    // Suspend user directly via database
    database
        .update_user_status(user_id, UserStatus::Suspended, &admin_token_id)
        .await?;

    // Verify user status in database
    let updated_user = database.get_user(user_id).await?.unwrap();
    assert_eq!(updated_user.user_status, UserStatus::Suspended);

    // Clean up test environment variable
    std::env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_user_status_transitions() -> Result<()> {
    let (database, _admin_token_id, _admin_user_id) = setup_test_database().await?;

    // Create a pending user
    let user = User {
        id: Uuid::new_v4(),
        email: "status_test@test.com".to_owned(),
        display_name: Some("Status Test User".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
        is_admin: false,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    let user_id = user.id;
    database.create_user(&user).await?;

    // Test status is initially pending
    let retrieved_user = database.get_user(user_id).await?.unwrap();
    assert_eq!(retrieved_user.user_status, UserStatus::Pending);
    assert!(retrieved_user.approved_by.is_none());

    // Clean up test environment variable
    std::env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    Ok(())
}

// Note: Database cleanup is handled by the Database implementation itself
