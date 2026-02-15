// ABOUTME: Unit tests for database users functionality
// ABOUTME: Validates database users behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::{
    database::Database,
    models::{TenantId, User, UserStatus, UserTier},
    permissions::UserRole,
};
use uuid::Uuid;

#[tokio::test]
async fn test_create_and_get_user() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    let user = User {
        id: Uuid::new_v4(),
        email: format!("test_{}@example.com", Uuid::new_v4()),
        display_name: Some("Test User".into()),
        password_hash: "hashed_password".into(),
        tier: UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };

    // Create user
    let user_id = db.create_user(&user).await.expect("Failed to create user");
    assert_eq!(user_id, user.id);

    // Get user by ID
    let retrieved = db
        .get_user(user.id)
        .await
        .expect("Failed to get user")
        .expect("User not found");
    assert_eq!(retrieved.email, user.email);
    assert_eq!(retrieved.display_name, user.display_name);
    assert_eq!(retrieved.tier, user.tier);

    // Get user by email
    let retrieved_by_email = db
        .get_user_by_email(&user.email)
        .await
        .expect("Failed to get user by email")
        .expect("User not found");
    assert_eq!(retrieved_by_email.id, user.id);
}

#[tokio::test]
async fn test_last_active_update() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: format!("active_{user_id}@example.com"),
        display_name: None,
        password_hash: "hashed".into(),
        tier: UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now() - chrono::Duration::hours(1),
        firebase_uid: None,
        auth_provider: String::new(),
    };

    db.create_user(&user).await.expect("Failed to create user");

    // Update last active
    db.update_last_active(user.id)
        .await
        .expect("Failed to update last active");

    // Verify update
    let updated = db
        .get_user(user.id)
        .await
        .expect("Failed to get user")
        .expect("User not found");

    assert!(updated.last_active > user.last_active);
}

// Comprehensive Database User Tests

async fn create_test_database() -> Database {
    Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database")
}

fn create_test_user(email: &str, display_name: Option<String>) -> User {
    let now = Utc::now();
    User {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        display_name,
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(now),
        created_at: now,
        last_active: now,
        firebase_uid: None,
        auth_provider: String::new(),
    }
}

fn create_test_admin_user(email: &str, display_name: Option<String>) -> User {
    let mut user = create_test_user(email, display_name);
    user.is_admin = true;
    user.user_status = UserStatus::Active;
    user
}

#[tokio::test]
async fn test_create_user_success() {
    let db = create_test_database().await;
    let user = create_test_user("test@example.com", Some("Test User".to_owned()));

    let result = db.create_user(&user).await;
    assert!(result.is_ok());

    let created_user_id = result.unwrap();
    assert_eq!(created_user_id, user.id);
}

#[tokio::test]
async fn test_create_user_duplicate_email() {
    let db = create_test_database().await;
    let user1 = create_test_user("duplicate@example.com", Some("User 1".to_owned()));
    let user2 = create_test_user("duplicate@example.com", Some("User 2".to_owned()));

    // First user should succeed
    let result1 = db.create_user(&user1).await;
    assert!(result1.is_ok());

    // Second user with same email should fail
    let result2 = db.create_user(&user2).await;
    assert!(result2.is_err());
}

#[tokio::test]
async fn test_get_user_by_id_existing() {
    let db = create_test_database().await;
    let user = create_test_user("get_test@example.com", Some("Get Test User".to_owned()));

    db.create_user(&user).await.unwrap();

    let retrieved_user = db.get_user_by_id(user.id).await.unwrap();
    assert!(retrieved_user.is_some());

    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.id, user.id);
    assert_eq!(retrieved_user.email, user.email);
    assert_eq!(retrieved_user.display_name, user.display_name);
}

#[tokio::test]
async fn test_get_user_by_id_nonexistent() {
    let db = create_test_database().await;
    let non_existent_id = Uuid::new_v4();

    let result = db.get_user_by_id(non_existent_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_user_by_email_existing() {
    let db = create_test_database().await;
    let email = "email_test@example.com";
    let user = create_test_user(email, Some("Email Test User".to_owned()));

    db.create_user(&user).await.unwrap();

    let retrieved_user = db.get_user_by_email(email).await.unwrap();
    assert!(retrieved_user.is_some());

    let retrieved_user = retrieved_user.unwrap();
    assert_eq!(retrieved_user.email, email);
    assert_eq!(retrieved_user.id, user.id);
}

#[tokio::test]
async fn test_get_user_by_email_nonexistent() {
    let db = create_test_database().await;

    let result = db
        .get_user_by_email("nonexistent@example.com")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_get_user_by_email_required_existing() {
    let db = create_test_database().await;
    let email = "required_test@example.com";
    let user = create_test_user(email, Some("Required Test User".to_owned()));

    db.create_user(&user).await.unwrap();

    let result = db.get_user_by_email_required(email).await;
    assert!(result.is_ok());

    let retrieved_user = result.unwrap();
    assert_eq!(retrieved_user.email, email);
    assert_eq!(retrieved_user.id, user.id);
}

#[tokio::test]
async fn test_get_user_by_email_required_nonexistent() {
    let db = create_test_database().await;

    let result = db
        .get_user_by_email_required("nonexistent@example.com")
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_update_last_active_success() {
    let db = create_test_database().await;
    let user = create_test_user(
        "active_test@example.com",
        Some("Active Test User".to_owned()),
    );

    db.create_user(&user).await.unwrap();

    let result = db.update_last_active(user.id).await;
    assert!(result.is_ok());

    // Verify the user still exists and can be retrieved
    let updated_user = db.get_user_by_id(user.id).await.unwrap();
    assert!(updated_user.is_some());
}

#[tokio::test]
async fn test_update_last_active_nonexistent() {
    let db = create_test_database().await;
    let non_existent_id = Uuid::new_v4();

    let result = db.update_last_active(non_existent_id).await;
    // Should not error for non-existent user (UPDATE with no matches)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_user_count() {
    let db = create_test_database().await;

    // Initially should be 0
    let count = db.get_user_count().await.unwrap();
    assert_eq!(count, 0);

    // Add a user
    let user1 = create_test_user("count_test1@example.com", Some("Count Test 1".to_owned()));
    db.create_user(&user1).await.unwrap();

    let count = db.get_user_count().await.unwrap();
    assert_eq!(count, 1);

    // Add another user
    let user2 = create_test_user("count_test2@example.com", Some("Count Test 2".to_owned()));
    db.create_user(&user2).await.unwrap();

    let count = db.get_user_count().await.unwrap();
    assert_eq!(count, 2);
}

#[tokio::test]
async fn test_get_users_by_status() {
    let db = create_test_database().await;

    // Create users with different statuses
    let active_user = create_test_user("active@example.com", Some("Active User".to_owned()));
    let mut pending_user = create_test_user("pending@example.com", Some("Pending User".to_owned()));
    pending_user.user_status = UserStatus::Pending;

    db.create_user(&active_user).await.unwrap();
    db.create_user(&pending_user).await.unwrap();

    // Get active users
    let active_users = db.get_users_by_status("active", None).await.unwrap();
    assert_eq!(active_users.len(), 1);
    assert_eq!(active_users[0].email, "active@example.com");

    // Get pending users
    let pending_users = db.get_users_by_status("pending", None).await.unwrap();
    assert_eq!(pending_users.len(), 1);
    assert_eq!(pending_users[0].email, "pending@example.com");

    // Get non-existent status
    let suspended_users = db.get_users_by_status("suspended", None).await.unwrap();
    assert_eq!(suspended_users.len(), 0);
}

#[tokio::test]
async fn test_update_user_status() {
    let db = create_test_database().await;
    let mut user = create_test_user("status_test@example.com", Some("Status Test".to_owned()));
    user.user_status = UserStatus::Pending;

    // Create admin user for approval
    let admin_user = create_test_admin_user("admin@example.com", Some("Admin".to_owned()));
    db.create_user(&admin_user).await.unwrap();

    db.create_user(&user).await.unwrap();

    // Update status from pending to active with admin user's UUID
    let result = db
        .update_user_status(user.id, UserStatus::Active, Some(admin_user.id))
        .await;

    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.user_status, UserStatus::Active);
    assert_eq!(updated_user.approved_by, Some(admin_user.id));
    assert!(updated_user.approved_at.is_some());
}

#[tokio::test]
async fn test_update_user_status_nonexistent() {
    let db = create_test_database().await;
    let non_existent_id = Uuid::new_v4();
    let admin_id = Uuid::new_v4();

    let result = db
        .update_user_status(non_existent_id, UserStatus::Active, Some(admin_id))
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test]
async fn test_update_user_status_without_approver() {
    let db = create_test_database().await;
    let mut user = create_test_user("no_approver@example.com", Some("No Approver".to_owned()));
    user.user_status = UserStatus::Pending;

    db.create_user(&user).await.unwrap();

    // Service token approval without approver UUID
    let result = db
        .update_user_status(user.id, UserStatus::Active, None)
        .await;

    assert!(result.is_ok());

    let updated_user = result.unwrap();
    assert_eq!(updated_user.user_status, UserStatus::Active);
    // approved_by should be None when no approver UUID is provided
    assert_eq!(updated_user.approved_by, None);
    assert!(updated_user.approved_at.is_some());
}

#[tokio::test]
async fn test_upsert_user_profile() {
    let db = create_test_database().await;
    let user = create_test_user("profile_test@example.com", Some("Profile Test".to_owned()));
    db.create_user(&user).await.unwrap();

    let profile_data = serde_json::json!({
        "age": 30,
        "weight": 70.5,
        "height": 175
    });

    let result = db.upsert_user_profile(user.id, profile_data.clone()).await;
    assert!(result.is_ok());

    // Verify the profile was stored
    let retrieved_profile = db.get_user_profile(user.id).await.unwrap();
    assert!(retrieved_profile.is_some());
    assert_eq!(retrieved_profile.unwrap(), profile_data);
}

#[tokio::test]
async fn test_get_user_profile_nonexistent() {
    let db = create_test_database().await;
    let non_existent_id = Uuid::new_v4();

    let result = db.get_user_profile(non_existent_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_user_fitness_profile() {
    let db = create_test_database().await;
    let user = create_test_user("fitness_test@example.com", Some("Fitness Test".to_owned()));
    db.create_user(&user).await.unwrap();

    // Just test that the method exists and returns None for a user without fitness profile
    let retrieved_profile = db.get_user_fitness_profile(user.id).await.unwrap();
    assert!(retrieved_profile.is_none());
}

#[tokio::test]
async fn test_provider_last_sync() {
    use pierre_mcp_server::database::user_oauth_tokens::OAuthTokenData;

    let db = create_test_database().await;
    let user = create_test_user("sync_test@example.com", Some("Sync Test".to_owned()));
    db.create_user(&user).await.unwrap();

    let provider = "strava";
    let sync_time = Utc::now();

    // First, create an OAuth token record (last_sync lives in user_oauth_tokens)
    let test_tenant_id = TenantId::new();
    let token_data = OAuthTokenData {
        id: &Uuid::new_v4().to_string(),
        user_id: user.id,
        tenant_id: test_tenant_id,
        provider,
        access_token: "test_access_token",
        refresh_token: Some("test_refresh_token"),
        token_type: "bearer",
        expires_at: Some(Utc::now() + chrono::Duration::hours(6)),
        scope: "read_all",
    };
    db.upsert_user_oauth_token(&token_data).await.unwrap();

    // Update last sync (scoped to tenant)
    let update_result = db
        .update_provider_last_sync(user.id, test_tenant_id, provider, sync_time)
        .await;
    assert!(update_result.is_ok());

    // Get last sync (scoped to tenant)
    let retrieved_sync = db
        .get_provider_last_sync(user.id, test_tenant_id, provider)
        .await
        .unwrap();
    assert!(retrieved_sync.is_some());

    // Times should be very close (within a few seconds)
    let time_diff = (retrieved_sync.unwrap() - sync_time).num_seconds().abs();
    assert!(time_diff < 5, "Sync times should be within 5 seconds");
}

#[tokio::test]
async fn test_get_provider_last_sync_nonexistent() {
    let db = create_test_database().await;
    let non_existent_id = Uuid::new_v4();

    let result = db
        .get_provider_last_sync(non_existent_id, TenantId::new(), "strava")
        .await
        .unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_database_migrations() {
    let db = create_test_database().await;

    // Migration should have created the users table
    // Test by inserting a user
    let user = create_test_user(
        "migration_test@example.com",
        Some("Migration Test".to_owned()),
    );
    let result = db.create_user(&user).await;
    assert!(result.is_ok());

    // Verify the user can be retrieved
    let retrieved = db.get_user_by_id(user.id).await.unwrap();
    assert!(retrieved.is_some());
}

#[tokio::test]
async fn test_user_serialization_in_database() {
    let db = create_test_database().await;

    // Create a user with various field types
    let mut user = create_test_user(
        "serialization@example.com",
        Some("Serialization Test".to_owned()),
    );
    user.tier = UserTier::Enterprise;
    user.user_status = UserStatus::Active;
    user.is_admin = true;

    db.create_user(&user).await.unwrap();

    let retrieved = db.get_user_by_id(user.id).await.unwrap().unwrap();

    // Verify all fields are correctly serialized/deserialized
    assert_eq!(retrieved.id, user.id);
    assert_eq!(retrieved.email, user.email);
    assert_eq!(retrieved.display_name, user.display_name);
    assert_eq!(retrieved.tier, user.tier);
    assert_eq!(retrieved.user_status, user.user_status);
    assert_eq!(retrieved.is_admin, user.is_admin);
    assert_eq!(retrieved.is_active, user.is_active);
}

#[tokio::test]
async fn test_user_with_encrypted_tokens() {
    use pierre_mcp_server::database::user_oauth_tokens::OAuthTokenData;

    let db = create_test_database().await;

    let now = Utc::now();
    let user = create_test_user("tokens_test@example.com", Some("Tokens Test".to_owned()));
    db.create_user(&user).await.unwrap();

    // Tokens are stored in user_oauth_tokens table, not in users table
    let test_tenant_id = TenantId::new();
    let token_data = OAuthTokenData {
        id: &Uuid::new_v4().to_string(),
        user_id: user.id,
        tenant_id: test_tenant_id,
        provider: "strava",
        access_token: "encrypted_strava_access",
        refresh_token: Some("encrypted_strava_refresh"),
        token_type: "bearer",
        expires_at: Some(now + chrono::Duration::hours(6)),
        scope: "read_all,activity:read",
    };
    db.upsert_user_oauth_token(&token_data).await.unwrap();

    // User retrieval should not include tokens (they're loaded separately)
    let retrieved = db.get_user_by_id(user.id).await.unwrap().unwrap();
    assert!(retrieved.strava_token.is_none()); // Tokens are loaded separately

    // Verify token via dedicated OAuth token API
    let oauth_token = db
        .get_user_oauth_token(user.id, test_tenant_id, "strava")
        .await
        .unwrap();
    assert!(oauth_token.is_some());
    let token = oauth_token.unwrap();
    assert_eq!(token.scope, Some("read_all,activity:read".to_owned()));
    assert!(token.expires_at.is_some());
}

#[tokio::test]
async fn test_user_status_transitions() {
    let db = create_test_database().await;

    // Create pending user
    let mut user = create_test_user("transition@example.com", Some("Transition Test".to_owned()));
    user.user_status = UserStatus::Pending;

    db.create_user(&user).await.unwrap();

    // Create admin for approvals
    let admin = create_test_admin_user("admin@example.com", Some("Admin".to_owned()));
    db.create_user(&admin).await.unwrap();

    // Transition: Pending -> Active
    let active_user = db
        .update_user_status(user.id, UserStatus::Active, Some(admin.id))
        .await
        .unwrap();
    assert_eq!(active_user.user_status, UserStatus::Active);
    assert_eq!(active_user.approved_by, Some(admin.id));

    // Transition: Active -> Suspended (approved_by is only set when activating)
    let suspended_user = db
        .update_user_status(user.id, UserStatus::Suspended, Some(admin.id))
        .await
        .unwrap();
    assert_eq!(suspended_user.user_status, UserStatus::Suspended);

    // Transition: Suspended -> Active (reactivation)
    let reactivated_user = db
        .update_user_status(user.id, UserStatus::Active, Some(admin.id))
        .await
        .unwrap();
    assert_eq!(reactivated_user.user_status, UserStatus::Active);
}

#[tokio::test]
async fn test_concurrent_user_operations() {
    let db = create_test_database().await;

    // Create multiple users concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            let user = create_test_user(
                &format!("concurrent_{i}@example.com"),
                Some(format!("User {i}")),
            );
            db_clone.create_user(&user).await.map(|_| user.id)
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    let mut user_ids = Vec::new();
    for handle in handles {
        let user_id = handle.await.unwrap().unwrap();
        user_ids.push(user_id);
    }

    // Verify all users were created
    assert_eq!(user_ids.len(), 10);

    // Verify count
    let count = db.get_user_count().await.unwrap();
    assert_eq!(count, 10);

    // Verify all users can be retrieved
    for user_id in user_ids {
        let user = db.get_user_by_id(user_id).await.unwrap();
        assert!(user.is_some());
    }
}

#[tokio::test]
async fn test_user_tier_operations() {
    let db = create_test_database().await;

    // Create users with different tiers
    let tiers = [
        UserTier::Starter,
        UserTier::Professional,
        UserTier::Enterprise,
    ];
    let mut user_ids = Vec::new();

    for (i, tier) in tiers.iter().enumerate() {
        let mut user = create_test_user(
            &format!("tier_{i}@example.com"),
            Some(format!("Tier User {i}")),
        );
        user.tier = tier.clone();

        let user_id = db.create_user(&user).await.unwrap();
        user_ids.push((user_id, tier.clone()));
    }

    // Verify tiers are stored correctly
    for (user_id, expected_tier) in user_ids {
        let user = db.get_user_by_id(user_id).await.unwrap().unwrap();
        assert_eq!(user.tier, expected_tier);
    }
}
