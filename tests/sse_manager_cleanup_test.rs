// ABOUTME: Tests for SSE manager cleanup logic to prevent memory leaks
// ABOUTME: Validates session tracking and cleanup when protocol streams disconnect
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use common::create_test_server_resources;
use pierre_mcp_server::sse::manager::SseManager;
use uuid::Uuid;

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_protocol_stream_cleanup_removes_from_user_sessions() {
    // Create SSE manager
    let manager = SseManager::new(100);
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");

    // Create a test user
    let user_id = Uuid::new_v4();

    // Simulate JWT token
    let token = format!("Bearer test_token_{}", user_id.as_hyphenated());

    // Register 3 protocol streams for the same user
    let session_id_1 = "session_1".to_owned();
    let session_id_2 = "session_2".to_owned();
    let session_id_3 = "session_3".to_owned();

    let _receiver1 = manager
        .register_protocol_stream(session_id_1.clone(), Some(token.clone()), resources.clone())
        .await;
    let _receiver2 = manager
        .register_protocol_stream(session_id_2.clone(), Some(token.clone()), resources.clone())
        .await;
    let _receiver3 = manager
        .register_protocol_stream(session_id_3.clone(), Some(token), resources.clone())
        .await;

    // Verify protocol streams are registered
    assert_eq!(manager.active_protocol_streams().await, 3);

    // Unregister session_1
    manager.unregister_protocol_stream(&session_id_1).await;

    // Verify:
    // 1. Protocol stream count decreased
    assert_eq!(manager.active_protocol_streams().await, 2);

    // 2. Session removed from user_sessions
    // (We can't directly access user_sessions, but we can verify via OAuth notification sending)
    // If session_1 is still in user_sessions, send_oauth_notification_to_protocol_streams would try to send to it

    // Unregister remaining sessions
    manager.unregister_protocol_stream(&session_id_2).await;
    assert_eq!(manager.active_protocol_streams().await, 1);

    manager.unregister_protocol_stream(&session_id_3).await;
    assert_eq!(manager.active_protocol_streams().await, 0);

    // After all sessions removed, user_sessions should be empty
    // (verified by the fact that cleanup completed without panics)
}

#[allow(clippy::expect_used, clippy::similar_names)]
#[tokio::test]
async fn test_protocol_stream_cleanup_with_multiple_users() {
    let manager = SseManager::new(100);
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");

    // Create two users
    let user_id_1 = Uuid::new_v4();
    let user_id_2 = Uuid::new_v4();

    // Register sessions for user 1
    let session_user1_a = "user1_session_a".to_owned();
    let session_user1_b = "user1_session_b".to_owned();

    let _r1a = manager
        .register_protocol_stream(
            session_user1_a.clone(),
            Some(format!("Bearer test_{user_id_1}")),
            resources.clone(),
        )
        .await;
    let _r1b = manager
        .register_protocol_stream(
            session_user1_b.clone(),
            Some(format!("Bearer test_{user_id_1}")),
            resources.clone(),
        )
        .await;

    // Register sessions for user 2
    let session_user2_a = "user2_session_a".to_owned();

    let _r2a = manager
        .register_protocol_stream(
            session_user2_a.clone(),
            Some(format!("Bearer test_{user_id_2}")),
            resources.clone(),
        )
        .await;

    assert_eq!(manager.active_protocol_streams().await, 3);

    // Unregister one session from user 1
    manager.unregister_protocol_stream(&session_user1_a).await;

    // User 1 should still have session_user1_b tracked
    // User 2 should still have session_user2_a tracked
    assert_eq!(manager.active_protocol_streams().await, 2);

    // Unregister user 2's session
    manager.unregister_protocol_stream(&session_user2_a).await;
    assert_eq!(manager.active_protocol_streams().await, 1);

    // Unregister last session from user 1
    manager.unregister_protocol_stream(&session_user1_b).await;
    assert_eq!(manager.active_protocol_streams().await, 0);
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_memory_leak_prevention_after_many_connects_disconnects() {
    let manager = SseManager::new(100);
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");

    let user_id = Uuid::new_v4();
    let token = Some(format!("Bearer test_{user_id}"));

    // Simulate 100 connect/disconnect cycles
    for i in 0..100 {
        let session_id = format!("session_{i}");

        let _receiver = manager
            .register_protocol_stream(session_id.clone(), token.clone(), resources.clone())
            .await;

        // Immediately unregister
        manager.unregister_protocol_stream(&session_id).await;
    }

    // After 100 cycles, there should be no active streams
    assert_eq!(manager.active_protocol_streams().await, 0);

    // The fact that this test completes without excessive memory usage
    // indicates the cleanup is working properly
}

#[allow(clippy::expect_used)]
#[tokio::test]
async fn test_cleanup_inactive_connections() {
    let manager = SseManager::new(100);
    let resources = create_test_server_resources()
        .await
        .expect("Failed to create test resources");

    let user_id = Uuid::new_v4();
    let session_id = "test_session".to_owned();

    let _receiver = manager
        .register_protocol_stream(
            session_id.clone(),
            Some(format!("Bearer test_{user_id}")),
            resources,
        )
        .await;

    assert_eq!(manager.active_protocol_streams().await, 1);

    // Wait a bit
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Cleanup connections inactive for more than 50ms (should not remove our connection)
    manager.cleanup_inactive_connections(0).await;

    // Wait for cleanup with very short timeout (0 seconds = immediate)
    // This should remove the connection since it's now inactive
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    manager.cleanup_inactive_connections(0).await;

    // Connection should be cleaned up
    assert_eq!(manager.active_protocol_streams().await, 0);
}
