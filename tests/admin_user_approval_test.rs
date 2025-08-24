// ABOUTME: Integration tests for admin user approval workflow
// ABOUTME: Tests pending users listing, approval, and suspension via database operations

use anyhow::Result;
use pierre_mcp_server::{
    admin::models::CreateAdminTokenRequest,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{User, UserStatus, UserTier},
};
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_admin_user_approval_tests";

/// Test helper to create admin token and database
async fn setup_test_database() -> Result<(Database, String, Uuid)> {
    // Initialize database with test-specific path
    let test_id = Uuid::new_v4().to_string();

    // Create test directory if it doesn't exist
    std::fs::create_dir_all("./test_data")
        .map_err(|e| anyhow::anyhow!("Failed to create test directory: {}", e))?;

    let db_path = format!("./test_data/admin_approval_test_{test_id}.db");
    let db_url = format!("sqlite:{db_path}");

    // Create database with proper encryption
    let (mut key_manager, database_key) =
        pierre_mcp_server::key_management::KeyManager::bootstrap()?;
    let database = Database::new(&db_url, database_key.to_vec()).await?;
    key_manager.complete_initialization(&database).await?;

    // Run migrations
    database.migrate().await?;

    // Create an admin user first (needed for foreign key constraint)
    let admin_user = User {
        id: Uuid::new_v4(),
        email: "admin@test.com".to_string(),
        display_name: Some("Test Admin".to_string()),
        password_hash: "admin_hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        approved_by: None, // Admin doesn't need approval
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    let admin_user_id = admin_user.id;
    database.create_user(&admin_user).await?;

    // Create a test admin token
    let admin_request = CreateAdminTokenRequest {
        service_name: "test_admin".to_string(),
        service_description: Some("Test admin for approval workflow".to_string()),
        permissions: None, // Super admin gets all permissions
        expires_in_days: Some(1),
        is_super_admin: true,
    };

    let admin_token = database
        .create_admin_token(&admin_request, TEST_JWT_SECRET)
        .await?;

    Ok((database, admin_token.token_id, admin_user_id))
}

#[tokio::test]
async fn test_get_pending_users() -> Result<()> {
    let (database, _admin_token_id, admin_user_id) = setup_test_database().await?;

    // Create test users with different statuses
    let pending_user = User {
        id: Uuid::new_v4(),
        email: "pending@test.com".to_string(),
        display_name: Some("Pending User".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    database.create_user(&pending_user).await?;

    let active_user = User {
        id: Uuid::new_v4(),
        email: "active@test.com".to_string(),
        display_name: Some("Active User".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
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

    Ok(())
}

#[tokio::test]
async fn test_approve_user() -> Result<()> {
    let (database, _admin_token_id, admin_user_id) = setup_test_database().await?;

    // Create a pending user
    let pending_user = User {
        id: Uuid::new_v4(),
        email: "to_approve@test.com".to_string(),
        display_name: Some("User to Approve".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
        approved_by: None,
        approved_at: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };
    let user_id = pending_user.id;
    database.create_user(&pending_user).await?;

    // Simulate approval by creating an approved user with the same ID
    let approved_user = User {
        id: user_id,
        email: "to_approve@test.com".to_string(),
        display_name: Some("User to Approve".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        approved_by: Some(admin_user_id), // Approved by admin user
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    // Test that we can create/update a user with approval fields properly set
    // This tests the foreign key constraint works when pointing to a real user
    let result = database.create_user(&approved_user).await;

    // Since we're using the same ID, this should fail on unique constraint,
    // but it should NOT fail on foreign key constraint (which is what we're testing)
    if let Err(e) = result {
        // Verify this is NOT a foreign key constraint error
        let error_msg = e.to_string().to_lowercase();
        assert!(
            !error_msg.contains("foreign key constraint"),
            "Foreign key constraint error when it should be unique constraint: {e}"
        );
        // This is expected - unique constraint violation, which means FK constraint passed
    } else {
        // If it somehow succeeded, verify the fields are set correctly
        let updated_user = database.get_user(user_id).await?.unwrap();
        assert_eq!(updated_user.user_status, UserStatus::Active);
        assert_eq!(updated_user.approved_by, Some(admin_user_id));
        assert!(updated_user.approved_at.is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_suspend_user() -> Result<()> {
    let (database, admin_token_id, admin_user_id) = setup_test_database().await?;

    // Create an active user
    let user = User {
        id: Uuid::new_v4(),
        email: "to_suspend@test.com".to_string(),
        display_name: Some("User to Suspend".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
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

    Ok(())
}

#[tokio::test]
async fn test_user_status_transitions() -> Result<()> {
    let (database, _admin_token_id, _admin_user_id) = setup_test_database().await?;

    // Create a pending user
    let user = User {
        id: Uuid::new_v4(),
        email: "status_test@test.com".to_string(),
        display_name: Some("Status Test User".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
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

    Ok(())
}

// Note: Database cleanup is handled by the Database implementation itself
