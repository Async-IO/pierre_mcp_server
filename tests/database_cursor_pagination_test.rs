// ABOUTME: Test suite for database cursor-based pagination
// ABOUTME: Validates keyset pagination correctness, cursor encoding, and consistency
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::Result;
use pierre_mcp_server::{
    database_plugins::{factory::Database, DatabaseProvider},
    models::{User, UserStatus, UserTier},
    pagination::PaginationParams,
};
use uuid::Uuid;

/// Test cursor pagination for users by status
#[tokio::test]
async fn test_get_users_by_status_cursor() -> Result<()> {
    // Initialize in-memory database
    let database_url = "sqlite::memory:";
    let database =
        Database::new(database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    // Create test users with different statuses
    for i in 0..5 {
        let user = User {
            id: Uuid::new_v4(),
            email: format!("user{i}@test.com"),
            display_name: Some(format!("User {i}")),
            password_hash: "hashed_password".to_string(),
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

        database.create_user(&user).await?;

        // Small delay to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Test first page (limit 2)
    let params = PaginationParams::forward(None, 2);
    let page1 = database
        .get_users_by_status_cursor("pending", &params)
        .await?;

    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);
    assert!(page1.next_cursor.is_some());

    // Test second page using cursor from first page
    let params2 = PaginationParams::forward(page1.next_cursor.clone(), 2);
    let page2 = database
        .get_users_by_status_cursor("pending", &params2)
        .await?;

    assert_eq!(page2.items.len(), 2);
    assert!(page2.has_more);
    assert!(page2.next_cursor.is_some());

    // Test third page (should have remaining item)
    let params3 = PaginationParams::forward(page2.next_cursor.clone(), 2);
    let page3 = database
        .get_users_by_status_cursor("pending", &params3)
        .await?;

    assert_eq!(page3.items.len(), 1);
    assert!(!page3.has_more);
    assert!(page3.next_cursor.is_none());

    // Verify no duplicate users across pages
    let mut all_user_ids = Vec::new();
    all_user_ids.extend(page1.items.iter().map(|u| u.id));
    all_user_ids.extend(page2.items.iter().map(|u| u.id));
    all_user_ids.extend(page3.items.iter().map(|u| u.id));

    // Check for duplicates
    let unique_count = all_user_ids.len();
    all_user_ids.sort();
    all_user_ids.dedup();
    assert_eq!(
        unique_count,
        all_user_ids.len(),
        "Found duplicate users across pages"
    );

    Ok(())
}

/// Test empty results with cursor pagination
#[tokio::test]
async fn test_cursor_pagination_empty_results() -> Result<()> {
    let database_url = "sqlite::memory:";
    let database =
        Database::new(database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    let params = PaginationParams::forward(None, 10);
    let page = database
        .get_users_by_status_cursor("active", &params)
        .await?;

    assert_eq!(page.items.len(), 0);
    assert!(!page.has_more);
    assert!(page.next_cursor.is_none());

    Ok(())
}

/// Test cursor pagination consistency when new items are added
#[tokio::test]
async fn test_cursor_pagination_consistency() -> Result<()> {
    let database_url = "sqlite::memory:";
    let database =
        Database::new(database_url, b"test_encryption_key_32_bytes_long".to_vec()).await?;

    // Create initial users
    for i in 0..3 {
        let user = User {
            id: Uuid::new_v4(),
            email: format!("initial{i}@test.com"),
            display_name: Some(format!("Initial User {i}")),
            password_hash: "hashed".to_string(),
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
        database.create_user(&user).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    // Get first page
    let params = PaginationParams::forward(None, 2);
    let page1 = database
        .get_users_by_status_cursor("pending", &params)
        .await?;

    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);

    // Add new user AFTER getting first page
    let new_user = User {
        id: Uuid::new_v4(),
        email: "newer@test.com".to_string(),
        display_name: Some("Newer User".to_string()),
        password_hash: "hashed".to_string(),
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
    database.create_user(&new_user).await?;

    // Get second page - should NOT include the newly added user
    // (cursor-based pagination ensures consistency)
    let params2 = PaginationParams::forward(page1.next_cursor.clone(), 2);
    let page2 = database
        .get_users_by_status_cursor("pending", &params2)
        .await?;

    // Should get remaining item from original 3 (not the newly added one)
    assert_eq!(page2.items.len(), 1);

    // Verify the new user is NOT in page2
    assert!(!page2.items.iter().any(|u| u.id == new_user.id));

    Ok(())
}
