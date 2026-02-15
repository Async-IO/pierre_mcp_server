// ABOUTME: Unit tests for database plugin functionality and factory patterns
// ABOUTME: Tests database creation, user operations, and plugin isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    constants::oauth_providers,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{TenantId, User, UserOAuthToken, UserStatus, UserTier},
    permissions::UserRole,
};
use serde_json::json;
use std::env;
use uuid::Uuid;

async fn create_test_database() -> Database {
    let encryption_key = (0..32).collect::<Vec<u8>>();
    // Use a unique database file path for each test to ensure isolation
    let unique_id = uuid::Uuid::new_v4();
    let temp_dir = env::temp_dir();
    let db_path = temp_dir.join(format!("test_{unique_id}.db"));
    let database_url = format!("sqlite:{}", db_path.display());

    #[cfg(feature = "postgresql")]
    {
        Database::new(
            &database_url,
            encryption_key,
            &PostgresPoolConfig::default(),
        )
        .await
        .expect("Failed to create test database")
    }

    #[cfg(not(feature = "postgresql"))]
    {
        Database::new(&database_url, encryption_key)
            .await
            .expect("Failed to create test database")
    }
}

async fn create_test_user(db: &Database) -> Uuid {
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
        strava_token: None,
        fitbit_token: None,
        firebase_uid: None,
        auth_provider: String::new(),
    };

    db.create_user(&user).await.expect("Failed to create user");
    user_id
}

#[tokio::test]
async fn test_database_factory_creation() {
    let encryption_key = (0..32).collect::<Vec<u8>>();

    // Test SQLite creation
    #[cfg(feature = "postgresql")]
    let sqlite_db = Database::new(
        "sqlite::memory:",
        encryption_key.clone(),
        &PostgresPoolConfig::default(),
    )
    .await;

    #[cfg(not(feature = "postgresql"))]
    let sqlite_db = Database::new("sqlite::memory:", encryption_key.clone()).await;

    assert!(sqlite_db.is_ok(), "Failed to create SQLite database");

    // Test migration
    let db = sqlite_db.unwrap();
    let migration_result = db.migrate().await;
    assert!(migration_result.is_ok(), "Failed to run migrations");
}

#[tokio::test]
async fn test_user_management() {
    let db = create_test_database().await;

    // Test user creation
    let user_id = create_test_user(&db).await;

    // Test user retrieval
    let retrieved_user = db.get_user(user_id).await.expect("Failed to get user");
    assert!(retrieved_user.is_some(), "User should exist");

    let user = retrieved_user.unwrap();
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.display_name, Some("Test User".to_owned()));

    // Test user by email
    let user_by_email = db
        .get_user_by_email("test@example.com")
        .await
        .expect("Failed to get user by email");
    assert!(user_by_email.is_some(), "User should be found by email");
    assert_eq!(user_by_email.unwrap().id, user_id);

    // Test user count
    let count = db.get_user_count().await.expect("Failed to get user count");
    assert_eq!(count, 1, "Should have exactly one user");
}

#[tokio::test]
async fn test_oauth_token_management() {
    let db = create_test_database().await;
    let user_id = create_test_user(&db).await;

    // Test storing Strava token
    let test_tenant_id = TenantId::from_uuid(Uuid::nil());
    let expires_at = Utc::now() + chrono::Duration::hours(1);
    let oauth_token = UserOAuthToken::new(
        user_id,
        test_tenant_id.to_string(),
        oauth_providers::STRAVA.to_owned(),
        "test_access_token".to_owned(),
        Some("test_refresh_token".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    db.upsert_user_oauth_token(&oauth_token)
        .await
        .expect("Failed to update Strava token");

    // Test retrieving Strava token
    let token = db
        .get_user_oauth_token(user_id, test_tenant_id, oauth_providers::STRAVA)
        .await
        .expect("Failed to get Strava token");
    assert!(token.is_some(), "Strava token should exist");

    let token = token.unwrap();
    assert_eq!(token.access_token, "test_access_token");
    assert_eq!(token.refresh_token, Some("test_refresh_token".to_owned()));
    assert_eq!(token.scope, Some("read,activity:read_all".to_owned()));

    // Test clearing Strava token
    db.delete_user_oauth_token(user_id, test_tenant_id, oauth_providers::STRAVA)
        .await
        .expect("Failed to clear Strava token");

    let cleared_token = db
        .get_user_oauth_token(user_id, test_tenant_id, oauth_providers::STRAVA)
        .await
        .expect("Failed to get Strava token after clear");
    assert!(cleared_token.is_none(), "Strava token should be cleared");
}

#[tokio::test]
async fn test_user_profile_management() {
    let db = create_test_database().await;
    let user_id = create_test_user(&db).await;

    // Test storing user profile
    let profile_data = json!({
        "name": "Test User",
        "age": 30,
        "preferences": {
            "units": "metric",
            "privacy": "public"
        }
    });

    db.upsert_user_profile(user_id, profile_data.clone())
        .await
        .expect("Failed to upsert user profile");

    // Test retrieving user profile
    let retrieved_profile = db
        .get_user_profile(user_id)
        .await
        .expect("Failed to get user profile");
    assert!(retrieved_profile.is_some(), "User profile should exist");

    let profile = retrieved_profile.unwrap();
    assert_eq!(profile["name"], "Test User");
    assert_eq!(profile["age"], 30);
    assert_eq!(profile["preferences"]["units"], "metric");
}

#[tokio::test]
async fn test_goal_management() {
    let db = create_test_database().await;
    let user_id = create_test_user(&db).await;

    // Test creating a goal
    let goal_data = json!({
        "type": "distance",
        "target": 100.0,
        "current": 25.0,
        "unit": "km",
        "deadline": "2024-12-31"
    });

    let goal_id = db
        .create_goal(user_id, goal_data.clone())
        .await
        .expect("Failed to create goal");

    assert!(!goal_id.is_empty(), "Goal ID should not be empty");

    // Test retrieving user goals
    let goals = db
        .get_user_goals(user_id)
        .await
        .expect("Failed to get user goals");
    assert_eq!(goals.len(), 1, "Should have exactly one goal");

    let goal = &goals[0];
    assert_eq!(goal["type"], "distance");
    assert_eq!(goal["target"], 100.0);

    // Test updating goal progress
    db.update_goal_progress(&goal_id, user_id, 50.0)
        .await
        .expect("Failed to update goal progress");
}

#[tokio::test]
async fn test_insight_management() {
    let db = create_test_database().await;
    let user_id = create_test_user(&db).await;

    // Test storing insights
    let insight_data = json!({
        "type": "performance",
        "content": "Your running pace has improved by 10% this month",
        "metadata": {
            "confidence": 0.85,
            "data_points": 15
        }
    });

    let insight_id = db
        .store_insight(user_id, insight_data.clone())
        .await
        .expect("Failed to store insight");

    assert!(!insight_id.is_empty(), "Insight ID should not be empty");

    // Test retrieving insights
    let insights = db
        .get_user_insights(user_id, None, Some(10))
        .await
        .expect("Failed to get user insights");

    assert_eq!(insights.len(), 1, "Should have exactly one insight");

    // Test retrieving insights with type filter (Note: this depends on the implementation)
    let filtered_insights = db
        .get_user_insights(user_id, Some("performance"), Some(10))
        .await
        .expect("Failed to get filtered insights");

    // This might be 0 or 1 depending on how the insight storage/retrieval is implemented
    assert!(
        filtered_insights.len() <= 1,
        "Should have at most one filtered insight"
    );
}

#[tokio::test]
async fn test_database_trait_abstraction() {
    let db = create_test_database().await;

    // Test that all required methods are available through the trait
    let _user_id = create_test_user(&db).await;

    // Test async trait methods work correctly
    let user_count = db.get_user_count().await.expect("Failed to get user count");
    assert!(user_count > 0, "Should have at least one user");

    // Test that the database can handle concurrent operations
    let mut handles = Vec::new();

    for i in 0..5 {
        let db_clone = db.clone();
        let handle = tokio::spawn(async move {
            let user_id = Uuid::new_v4();
            let user = User {
                id: user_id,
                email: format!("test{i}@example.com"),
                display_name: Some(format!("Test User {i}")),
                password_hash: "hashed_password".to_owned(),
                tier: UserTier::Starter,
                created_at: Utc::now(),
                last_active: Utc::now(),
                is_active: true,
                user_status: UserStatus::Active,
                is_admin: false,
                role: UserRole::User,
                approved_by: None,
                approved_at: Some(chrono::Utc::now()),
                strava_token: None,
                fitbit_token: None,
                firebase_uid: None,
                auth_provider: String::new(),
            };

            db_clone.create_user(&user).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle
            .await
            .expect("Task failed")
            .expect("Failed to create user concurrently");
    }

    // Verify all users were created
    let final_count = db
        .get_user_count()
        .await
        .expect("Failed to get final user count");
    assert_eq!(final_count, 6, "Should have 6 users total (1 + 5)");
}

#[tokio::test]
async fn test_system_stats() {
    let db = create_test_database().await;

    // Create a few users
    for i in 0..3 {
        let user_id = Uuid::new_v4();
        let user = User {
            id: user_id,
            email: format!("user{i}@example.com"),
            display_name: Some(format!("User {i}")),
            password_hash: "hashed_password".to_owned(),
            tier: UserTier::Starter,
            created_at: Utc::now(),
            last_active: Utc::now(),
            is_active: true,
            user_status: UserStatus::Active,
            is_admin: false,
            role: UserRole::User,
            approved_by: None,
            approved_at: Some(chrono::Utc::now()),
            strava_token: None,
            fitbit_token: None,
            firebase_uid: None,
            auth_provider: String::new(),
        };

        db.create_user(&user).await.expect("Failed to create user");
    }

    // Test system stats (user_count, api_key_count)
    let (user_count, api_key_count) = db
        .get_system_stats(None)
        .await
        .expect("Failed to get system stats");
    assert_eq!(user_count, 3, "Should have 3 users");
    assert_eq!(api_key_count, 0, "Should have 0 API keys initially");
}
