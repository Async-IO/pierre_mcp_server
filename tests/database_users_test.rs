use pierre_mcp_server::database::Database;
use pierre_mcp_server::models::{User, UserTier};
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
        tenant_id: Some("test-tenant".to_string()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
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
        tenant_id: Some("test-tenant".to_string()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now() - chrono::Duration::hours(1),
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
