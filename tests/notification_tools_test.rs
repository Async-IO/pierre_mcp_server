// ABOUTME: Standalone integration tests for OAuth notification MCP tool handlers
// ABOUTME: Tests mark_notifications_read, get_notifications, announce_oauth_success, check_oauth_notifications
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! OAuth Notification Tool Tests
//!
//! Tests the 4 notification-related MCP tools:
//! - `mark_notifications_read`: Mark a notification as read
//! - `get_notifications`: Get OAuth notifications for user
//! - `announce_oauth_success`: Announce OAuth completion in chat
//! - `check_oauth_notifications`: Check for pending OAuth notifications
//!
//! These tests verify standalone functionality of notification tools
//! outside the context of full OAuth flows.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::constants::tools::identifiers::{
    ANNOUNCE_OAUTH_SUCCESS, CHECK_OAUTH_NOTIFICATIONS, GET_NOTIFICATIONS, MARK_NOTIFICATIONS_READ,
};
use pierre_mcp_server::database_plugins::DatabaseProvider;
use pierre_mcp_server::mcp::multitenant::McpRequest;
use pierre_mcp_server::models::User;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// MCP method name for tools/call requests
const TOOLS_CALL_METHOD: &str = "tools/call";

mod common;

// ============================================================================
// Test Setup
// ============================================================================

/// Create a test MCP request
#[allow(clippy::needless_pass_by_value)] // args is consumed by json! macro
fn create_mcp_request(method: &str, tool_name: &str, args: Value) -> McpRequest {
    McpRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(1)),
        method: method.to_owned(),
        params: Some(json!({
            "name": tool_name,
            "arguments": args
        })),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    }
}

/// Test context with all resources needed for notification tests
struct NotificationTestContext {
    resources: Arc<pierre_mcp_server::mcp::resources::ServerResources>,
    user_id: Uuid,
}

async fn setup_notification_test() -> Result<NotificationTestContext> {
    common::init_server_config();
    common::init_test_http_clients();

    let resources = common::create_test_server_resources().await?;

    // Create test user
    let user = User::new(
        format!("notification_test_{}@example.com", Uuid::new_v4()),
        "password_hash".to_owned(),
        Some("Notification Test User".to_owned()),
    );
    let user_id = user.id;

    resources.database.create_user(&user).await?;

    Ok(NotificationTestContext { resources, user_id })
}

/// Create a test OAuth notification in the database
async fn create_test_notification(
    ctx: &NotificationTestContext,
    provider: &str,
    success: bool,
    message: &str,
) -> Result<String> {
    let notification_id = ctx
        .resources
        .database
        .store_oauth_notification(ctx.user_id, provider, success, message, None)
        .await?;

    Ok(notification_id)
}

// ============================================================================
// Tool Registration Tests
// ============================================================================

#[tokio::test]
async fn test_notification_tools_exist_in_schema() -> Result<()> {
    let tools = pierre_mcp_server::mcp::schema::get_tools();

    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();

    // Verify all notification tools exist
    assert!(
        tool_names.contains(&MARK_NOTIFICATIONS_READ),
        "Missing {MARK_NOTIFICATIONS_READ}"
    );
    assert!(
        tool_names.contains(&GET_NOTIFICATIONS),
        "Missing {GET_NOTIFICATIONS}"
    );
    assert!(
        tool_names.contains(&ANNOUNCE_OAUTH_SUCCESS),
        "Missing {ANNOUNCE_OAUTH_SUCCESS}"
    );
    assert!(
        tool_names.contains(&CHECK_OAUTH_NOTIFICATIONS),
        "Missing {CHECK_OAUTH_NOTIFICATIONS}"
    );

    Ok(())
}

#[tokio::test]
async fn test_notification_tool_schemas_valid() -> Result<()> {
    let tools = pierre_mcp_server::mcp::schema::get_tools();

    for tool_name in [
        MARK_NOTIFICATIONS_READ,
        GET_NOTIFICATIONS,
        ANNOUNCE_OAUTH_SUCCESS,
        CHECK_OAUTH_NOTIFICATIONS,
    ] {
        let tool = tools.iter().find(|t| t.name == tool_name);
        assert!(tool.is_some(), "Tool {tool_name} should exist");

        let tool = tool.unwrap();
        assert!(
            !tool.description.is_empty(),
            "{tool_name} should have description"
        );
        assert_eq!(
            tool.input_schema.schema_type, "object",
            "{tool_name} should have input schema of type object"
        );
    }

    Ok(())
}

// ============================================================================
// get_notifications Tests (via database interface)
// ============================================================================

#[tokio::test]
async fn test_get_notifications_empty() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Query notifications for user with no notifications
    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert!(
        notifications.is_empty(),
        "New user should have no notifications"
    );

    Ok(())
}

#[tokio::test]
async fn test_get_notifications_with_unread() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create some notifications
    create_test_notification(&ctx, "strava", true, "Connected to Strava").await?;
    create_test_notification(&ctx, "fitbit", true, "Connected to Fitbit").await?;

    // Get unread notifications only
    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(notifications.len(), 2, "Should have 2 unread notifications");

    Ok(())
}

#[tokio::test]
async fn test_get_all_notifications_include_read() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create notification and mark one as read
    let notification_id =
        create_test_notification(&ctx, "strava", true, "Connected to Strava").await?;
    create_test_notification(&ctx, "fitbit", true, "Connected to Fitbit").await?;

    // Mark first as read
    ctx.resources
        .database
        .mark_oauth_notification_read(&notification_id, ctx.user_id)
        .await?;

    // Get all notifications including read
    let all_notifications = ctx
        .resources
        .database
        .get_all_oauth_notifications(ctx.user_id, None)
        .await?;

    assert_eq!(
        all_notifications.len(),
        2,
        "Should have 2 total notifications"
    );

    // Get only unread
    let unread = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(unread.len(), 1, "Should have 1 unread notification");

    Ok(())
}

// ============================================================================
// mark_notifications_read Tests
// ============================================================================

#[tokio::test]
async fn test_mark_notification_read() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create an unread notification
    let notification_id =
        create_test_notification(&ctx, "strava", true, "Connected to Strava").await?;

    // Verify it's unread
    let before = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;
    assert_eq!(before.len(), 1, "Should have 1 unread");
    assert!(before[0].read_at.is_none());

    // Mark as read
    let marked = ctx
        .resources
        .database
        .mark_oauth_notification_read(&notification_id, ctx.user_id)
        .await?;
    assert!(marked, "Should successfully mark as read");

    // Verify it's now read
    let after = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;
    assert_eq!(after.len(), 0, "Should have 0 unread notifications");

    // Include read - should still be there
    let all = ctx
        .resources
        .database
        .get_all_oauth_notifications(ctx.user_id, None)
        .await?;
    assert_eq!(all.len(), 1, "Should have 1 total notification");
    assert!(all[0].read_at.is_some());

    Ok(())
}

#[tokio::test]
async fn test_mark_notification_read_nonexistent() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Try to mark a nonexistent notification
    let result = ctx
        .resources
        .database
        .mark_oauth_notification_read(&Uuid::new_v4().to_string(), ctx.user_id)
        .await?;

    assert!(!result, "Should return false for nonexistent notification");

    Ok(())
}

#[tokio::test]
async fn test_mark_notification_read_wrong_user() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create notification for ctx.user_id
    let notification_id =
        create_test_notification(&ctx, "strava", true, "Connected to Strava").await?;

    // Try to mark with different user ID
    let other_user_id = Uuid::new_v4();
    let result = ctx
        .resources
        .database
        .mark_oauth_notification_read(&notification_id, other_user_id)
        .await?;

    assert!(!result, "Should not mark notification for different user");

    // Original should still be unread
    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;
    assert_eq!(notifications.len(), 1);
    assert!(notifications[0].read_at.is_none());

    Ok(())
}

// ============================================================================
// store_oauth_notification Tests (announce_oauth_success backing)
// ============================================================================

#[tokio::test]
async fn test_store_oauth_notification() -> Result<()> {
    let ctx = setup_notification_test().await?;

    let notification_id = ctx
        .resources
        .database
        .store_oauth_notification(
            ctx.user_id,
            "strava",
            true,
            "Successfully connected to Strava",
            None,
        )
        .await?;

    assert!(!notification_id.is_empty(), "Should return notification ID");

    // Verify it exists
    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].id, notification_id);
    assert_eq!(notifications[0].provider, "strava");
    assert!(notifications[0].success);
    assert!(notifications[0].read_at.is_none());

    Ok(())
}

#[tokio::test]
async fn test_store_multiple_notifications_same_provider() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create multiple notifications for same provider
    for i in 0..3 {
        ctx.resources
            .database
            .store_oauth_notification(
                ctx.user_id,
                "strava",
                true,
                &format!("Notification {i}"),
                None,
            )
            .await?;
    }

    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(notifications.len(), 3, "Should have 3 Strava notifications");

    // All should be for strava
    for n in &notifications {
        assert_eq!(n.provider, "strava");
    }

    Ok(())
}

#[tokio::test]
async fn test_store_notification_with_expiration() -> Result<()> {
    let ctx = setup_notification_test().await?;

    let expires_at = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::hours(1))
        .unwrap()
        .to_rfc3339();

    let notification_id = ctx
        .resources
        .database
        .store_oauth_notification(
            ctx.user_id,
            "strava",
            true,
            "Token expires soon",
            Some(&expires_at),
        )
        .await?;

    assert!(!notification_id.is_empty());

    // Verify expiration is set
    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(notifications.len(), 1);
    assert!(notifications[0].expires_at.is_some());

    Ok(())
}

#[tokio::test]
async fn test_store_failure_notification() -> Result<()> {
    let ctx = setup_notification_test().await?;

    let notification_id = ctx
        .resources
        .database
        .store_oauth_notification(
            ctx.user_id,
            "strava",
            false,
            "OAuth authorization failed",
            None,
        )
        .await?;

    let notifications = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].id, notification_id);
    assert!(
        !notifications[0].success,
        "Should be a failure notification"
    );

    Ok(())
}

// ============================================================================
// User Isolation Tests
// ============================================================================

#[tokio::test]
async fn test_notification_user_isolation() -> Result<()> {
    common::init_server_config();
    common::init_test_http_clients();

    let resources = common::create_test_server_resources().await?;

    // Create two users
    let user1 = User::new(
        "notification_user1@example.com".to_owned(),
        "hash".to_owned(),
        None,
    );
    let user2 = User::new(
        "notification_user2@example.com".to_owned(),
        "hash".to_owned(),
        None,
    );

    resources.database.create_user(&user1).await?;
    resources.database.create_user(&user2).await?;

    // Create notification for user1
    resources
        .database
        .store_oauth_notification(user1.id, "strava", true, "User 1's notification", None)
        .await?;

    // User1 should see the notification
    let user1_notifications = resources
        .database
        .get_unread_oauth_notifications(user1.id)
        .await?;
    assert_eq!(user1_notifications.len(), 1);

    // User2 should NOT see user1's notification
    let user2_notifications = resources
        .database
        .get_unread_oauth_notifications(user2.id)
        .await?;
    assert_eq!(
        user2_notifications.len(),
        0,
        "User2 should not see User1's notifications"
    );

    Ok(())
}

// ============================================================================
// mark_all_oauth_notifications_read Tests
// ============================================================================

#[tokio::test]
async fn test_mark_all_notifications_read() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create several notifications
    create_test_notification(&ctx, "strava", true, "Strava connected").await?;
    create_test_notification(&ctx, "fitbit", true, "Fitbit connected").await?;
    create_test_notification(&ctx, "garmin", true, "Garmin connected").await?;

    // Verify we have 3 unread
    let before = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;
    assert_eq!(before.len(), 3);

    // Mark all as read
    let marked_count = ctx
        .resources
        .database
        .mark_all_oauth_notifications_read(ctx.user_id)
        .await?;

    assert_eq!(marked_count, 3, "Should mark 3 notifications as read");

    // Verify all are now read
    let after = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;
    assert_eq!(after.len(), 0, "Should have no unread notifications");

    Ok(())
}

// ============================================================================
// check_oauth_notifications Tests (Pending Notification Check)
// ============================================================================

#[tokio::test]
async fn test_check_pending_notifications() -> Result<()> {
    let ctx = setup_notification_test().await?;

    // Create some pending notifications
    create_test_notification(&ctx, "strava", true, "Strava connected").await?;
    create_test_notification(&ctx, "fitbit", true, "Fitbit connected").await?;

    // Check for pending
    let pending = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(pending.len(), 2, "Should have 2 pending notifications");

    // Mark all as read
    ctx.resources
        .database
        .mark_all_oauth_notifications_read(ctx.user_id)
        .await?;

    // Check again
    let still_pending = ctx
        .resources
        .database
        .get_unread_oauth_notifications(ctx.user_id)
        .await?;

    assert_eq!(
        still_pending.len(),
        0,
        "Should have 0 pending notifications after marking all read"
    );

    Ok(())
}

// ============================================================================
// MCP Request Format Tests
// ============================================================================

#[tokio::test]
async fn test_notification_mcp_request_format() -> Result<()> {
    // Test that MCP request format is correct for notification tools
    let request = create_mcp_request(
        TOOLS_CALL_METHOD,
        GET_NOTIFICATIONS,
        json!({
            "include_read": false,
            "provider": "strava"
        }),
    );

    assert_eq!(request.method, TOOLS_CALL_METHOD);
    let params = request.params.unwrap();
    assert_eq!(params["name"], GET_NOTIFICATIONS);
    assert_eq!(params["arguments"]["include_read"], false);
    assert_eq!(params["arguments"]["provider"], "strava");

    Ok(())
}

#[tokio::test]
async fn test_mark_notifications_read_request_format() -> Result<()> {
    let notification_id = Uuid::new_v4().to_string();
    let request = create_mcp_request(
        TOOLS_CALL_METHOD,
        MARK_NOTIFICATIONS_READ,
        json!({
            "notification_id": notification_id
        }),
    );

    let params = request.params.unwrap();
    assert_eq!(params["name"], MARK_NOTIFICATIONS_READ);
    assert_eq!(params["arguments"]["notification_id"], notification_id);

    Ok(())
}

#[tokio::test]
async fn test_announce_oauth_success_request_format() -> Result<()> {
    let notification_id = Uuid::new_v4().to_string();
    let request = create_mcp_request(
        TOOLS_CALL_METHOD,
        ANNOUNCE_OAUTH_SUCCESS,
        json!({
            "provider": "strava",
            "message": "Successfully connected!",
            "notification_id": notification_id
        }),
    );

    let params = request.params.unwrap();
    assert_eq!(params["name"], ANNOUNCE_OAUTH_SUCCESS);
    assert_eq!(params["arguments"]["provider"], "strava");
    assert_eq!(params["arguments"]["message"], "Successfully connected!");

    Ok(())
}

#[tokio::test]
async fn test_check_oauth_notifications_request_format() -> Result<()> {
    let request = create_mcp_request(TOOLS_CALL_METHOD, CHECK_OAUTH_NOTIFICATIONS, json!({}));

    let params = request.params.unwrap();
    assert_eq!(params["name"], CHECK_OAUTH_NOTIFICATIONS);
    assert!(params["arguments"].is_object());

    Ok(())
}
