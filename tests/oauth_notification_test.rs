// ABOUTME: Tests for OAuth notification functionality and database operations
// ABOUTME: Verifies notification creation, storage, retrieval, and cleanup workflows

use anyhow::Result;
use chrono::Utc;
use pierre_mcp_server::{
    database::oauth_notifications::OAuthNotification, database_plugins::DatabaseProvider,
};
use std::sync::Arc;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_oauth_notification_storage_and_retrieval() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    // Store OAuth notification
    database
        .store_oauth_notification(
            user_id,
            "strava",
            true,
            "Successfully connected to Strava!",
            None,
        )
        .await?;

    // Retrieve unread notifications
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].provider, "strava");
    assert_eq!(
        notifications[0].message,
        "Successfully connected to Strava!"
    );
    assert!(notifications[0].read_at.is_none());

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_mark_as_read() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    // Store OAuth notification
    database
        .store_oauth_notification(
            user_id,
            "strava",
            true,
            "OAuth completed successfully",
            None,
        )
        .await?;

    // Get unread notifications
    let unread_notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(unread_notifications.len(), 1);
    let notification_id = &unread_notifications[0].id;

    // Mark as read
    database
        .mark_oauth_notification_read(notification_id, user_id)
        .await?;

    // Verify no unread notifications remain
    let remaining_unread = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(remaining_unread.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_with_expiry() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    let expires_at = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();

    // Store OAuth notification with expiry
    database
        .store_oauth_notification(
            user_id,
            "strava",
            true,
            "Temporary OAuth notification",
            Some(&expires_at),
        )
        .await?;

    // Retrieve and verify expiry is set
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 1);
    assert!(notifications[0].expires_at.is_some());

    Ok(())
}

#[tokio::test]
async fn test_multiple_provider_notifications() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    // Store notifications from different providers
    database
        .store_oauth_notification(user_id, "strava", true, "Connected to Strava", None)
        .await?;

    database
        .store_oauth_notification(user_id, "fitbit", true, "Connected to Fitbit", None)
        .await?;

    // Verify both notifications are stored
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 2);

    let providers: Vec<&str> = notifications.iter().map(|n| n.provider.as_str()).collect();
    assert!(providers.contains(&"strava"));
    assert!(providers.contains(&"fitbit"));

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_user_isolation() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id_1, _user_1) = common::create_test_user(&database).await?;
    let (user_id_2, _user_2) =
        common::create_test_user_with_email(&database, "user2@example.com").await?;

    // Store notification for user 1
    database
        .store_oauth_notification(user_id_1, "strava", true, "User 1 notification", None)
        .await?;

    // Store notification for user 2
    database
        .store_oauth_notification(user_id_2, "strava", true, "User 2 notification", None)
        .await?;

    // Verify user isolation
    let user_1_notifications = database.get_unread_oauth_notifications(user_id_1).await?;
    let user_2_notifications = database.get_unread_oauth_notifications(user_id_2).await?;

    assert_eq!(user_1_notifications.len(), 1);
    assert_eq!(user_2_notifications.len(), 1);
    assert_eq!(user_1_notifications[0].message, "User 1 notification");
    assert_eq!(user_2_notifications[0].message, "User 2 notification");

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_cleanup() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    // Store multiple notifications
    for i in 0..5 {
        database
            .store_oauth_notification(user_id, "strava", true, &format!("Notification {i}"), None)
            .await?;
    }

    // Verify all notifications are stored
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 5);

    // Mark some as read
    for notification in notifications.iter().take(3) {
        database
            .mark_oauth_notification_read(&notification.id, user_id)
            .await?;
    }

    // Verify only unread notifications remain
    let remaining = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(remaining.len(), 2);

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_error_handling() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    // Store OAuth notification with error status
    database
        .store_oauth_notification(
            user_id,
            "strava",
            false,
            "OAuth failed - invalid credentials",
            None,
        )
        .await?;

    // Retrieve and verify error notification
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 1);
    assert!(notifications[0].message.contains("failed"));

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_concurrent_access() -> Result<()> {
    let database = Arc::new(common::create_test_database().await?);
    let (user_id, _user) = common::create_test_user(&database).await?;

    // Create multiple concurrent notification operations
    let mut handles = Vec::new();

    for i in 0..10 {
        let db = database.clone();
        let handle = tokio::spawn(async move {
            db.store_oauth_notification(
                user_id,
                "strava",
                true,
                &format!("Concurrent notification {i}"),
                None,
            )
            .await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    // Verify all notifications were stored
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 10);

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_with_special_characters() -> Result<()> {
    let database = common::create_test_database().await?;
    let (user_id, _user) = common::create_test_user(&database).await?;

    let special_message = "OAuth completed ✅ with émojis and spëcial châractërs! 🎉";

    // Store notification with special characters
    database
        .store_oauth_notification(user_id, "strava", true, special_message, None)
        .await?;

    // Retrieve and verify special characters are preserved
    let notifications = database.get_unread_oauth_notifications(user_id).await?;
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].message, special_message);

    Ok(())
}

#[tokio::test]
async fn test_oauth_notification_struct_creation() -> Result<()> {
    let user_id = Uuid::new_v4();
    let created_at = Utc::now();

    // Test creating OAuthNotification struct
    let notification = OAuthNotification {
        id: "test-notification-id".to_string(),
        user_id: user_id.to_string(),
        provider: "strava".to_string(),
        success: true,
        message: "OAuth completed successfully".to_string(),
        expires_at: None,
        created_at,
        read_at: None,
    };

    // Verify struct fields
    assert_eq!(notification.id, "test-notification-id");
    assert_eq!(notification.user_id, user_id.to_string());
    assert_eq!(notification.provider, "strava");
    assert!(notification.success);
    assert_eq!(notification.message, "OAuth completed successfully");
    assert!(notification.expires_at.is_none());
    assert!(notification.read_at.is_none());
    assert_eq!(notification.created_at, created_at);

    Ok(())
}
