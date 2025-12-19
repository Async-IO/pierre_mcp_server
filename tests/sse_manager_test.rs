// ABOUTME: Unit tests for SSE manager functionality
// ABOUTME: Validates SSE connection management, stream registration, and cleanup
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::sse::manager::{ConnectionMetadata, ConnectionType, SseManager};
use uuid::Uuid;

// =============================================================================
// SseManager Creation Tests
// =============================================================================

#[test]
fn test_sse_manager_new() {
    let _manager = SseManager::new(100);
    // Manager should be created without panicking
    // The buffer_size is stored internally
}

#[test]
fn test_sse_manager_default() {
    let _manager = SseManager::default();
    // Default should use the SSE_BROADCAST_CHANNEL_SIZE constant
}

#[test]
fn test_sse_manager_clone() {
    let manager = SseManager::new(50);
    let cloned = manager.clone();

    // Clone should work and maintain Arc references
    // Both instances should share the same underlying broadcast channel
    // Verify both are functional by dropping them without panic
    drop(cloned);
    drop(manager);
}

// =============================================================================
// ConnectionType Tests
// =============================================================================

#[test]
fn test_connection_type_notification() {
    let user_id = Uuid::new_v4();
    let conn_type = ConnectionType::Notification { user_id };

    if let ConnectionType::Notification { user_id: id } = conn_type {
        assert_eq!(id, user_id);
    } else {
        panic!("Expected Notification type");
    }
}

#[test]
fn test_connection_type_protocol() {
    let session_id = "test-session-123".to_owned();
    let conn_type = ConnectionType::Protocol {
        session_id: session_id.clone(),
    };

    if let ConnectionType::Protocol {
        session_id: session,
    } = conn_type
    {
        assert_eq!(session, session_id);
    } else {
        panic!("Expected Protocol type");
    }
}

#[test]
fn test_connection_type_a2a_task() {
    let task_id = "task-456".to_owned();
    let client_id = "client-789".to_owned();
    let conn_type = ConnectionType::A2ATask {
        task_id: task_id.clone(),
        client_id: client_id.clone(),
    };

    if let ConnectionType::A2ATask {
        task_id: tid,
        client_id: cid,
    } = conn_type
    {
        assert_eq!(tid, task_id);
        assert_eq!(cid, client_id);
    } else {
        panic!("Expected A2ATask type");
    }
}

#[test]
fn test_connection_type_debug() {
    let conn_type = ConnectionType::Notification {
        user_id: Uuid::new_v4(),
    };
    let debug_str = format!("{conn_type:?}");
    assert!(debug_str.contains("Notification"));
}

#[test]
fn test_connection_type_clone() {
    let user_id = Uuid::new_v4();
    let conn_type = ConnectionType::Notification { user_id };
    let cloned = conn_type.clone();

    // Verify original still works after clone
    if let ConnectionType::Notification { user_id: id } = conn_type {
        assert_eq!(id, user_id);
    } else {
        panic!("Original should preserve type");
    }

    // Verify clone has same data
    if let ConnectionType::Notification { user_id: id } = cloned {
        assert_eq!(id, user_id);
    } else {
        panic!("Clone should preserve type");
    }
}

// =============================================================================
// ConnectionMetadata Tests
// =============================================================================

#[test]
fn test_connection_metadata_creation() {
    let metadata = ConnectionMetadata {
        connection_type: ConnectionType::Notification {
            user_id: Uuid::new_v4(),
        },
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };

    assert!(metadata.created_at <= metadata.last_activity);
}

#[test]
fn test_connection_metadata_clone() {
    let metadata = ConnectionMetadata {
        connection_type: ConnectionType::Protocol {
            session_id: "session-1".to_owned(),
        },
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };

    let cloned = metadata.clone();
    assert_eq!(metadata.created_at, cloned.created_at);
}

#[test]
fn test_connection_metadata_debug() {
    let metadata = ConnectionMetadata {
        connection_type: ConnectionType::Protocol {
            session_id: "test".to_owned(),
        },
        created_at: chrono::Utc::now(),
        last_activity: chrono::Utc::now(),
    };

    let debug_str = format!("{metadata:?}");
    assert!(debug_str.contains("connection_type"));
    assert!(debug_str.contains("created_at"));
}

// =============================================================================
// Async SseManager Tests
// =============================================================================

#[tokio::test]
async fn test_register_notification_stream() {
    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    let _receiver = manager.register_notification_stream(user_id).await;

    // Should have 1 active notification stream
    assert_eq!(manager.active_notification_streams().await, 1);
}

#[tokio::test]
async fn test_register_multiple_notification_streams() {
    let manager = SseManager::new(10);

    let user1 = Uuid::new_v4();
    let user2 = Uuid::new_v4();
    let user3 = Uuid::new_v4();

    let _r1 = manager.register_notification_stream(user1).await;
    let _r2 = manager.register_notification_stream(user2).await;
    let _r3 = manager.register_notification_stream(user3).await;

    assert_eq!(manager.active_notification_streams().await, 3);
}

#[tokio::test]
async fn test_unregister_notification_stream() {
    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    let _receiver = manager.register_notification_stream(user_id).await;
    assert_eq!(manager.active_notification_streams().await, 1);

    manager.unregister_notification_stream(user_id).await;
    assert_eq!(manager.active_notification_streams().await, 0);
}

#[tokio::test]
async fn test_unregister_nonexistent_notification_stream() {
    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    // Should not panic when unregistering non-existent stream
    manager.unregister_notification_stream(user_id).await;
    assert_eq!(manager.active_notification_streams().await, 0);
}

#[tokio::test]
async fn test_register_a2a_task_stream() {
    let manager = SseManager::new(10);
    let task_id = "task-123".to_owned();
    let client_id = "client-456".to_owned();

    let _receiver = manager
        .register_a2a_task_stream(task_id.clone(), client_id)
        .await;

    assert_eq!(manager.active_a2a_task_streams().await, 1);
}

#[tokio::test]
async fn test_unregister_a2a_task_stream() {
    let manager = SseManager::new(10);
    let task_id = "task-789".to_owned();
    let client_id = "client-012".to_owned();

    let _receiver = manager
        .register_a2a_task_stream(task_id.clone(), client_id)
        .await;
    assert_eq!(manager.active_a2a_task_streams().await, 1);

    manager.unregister_a2a_task_stream(&task_id).await;
    assert_eq!(manager.active_a2a_task_streams().await, 0);
}

#[tokio::test]
async fn test_get_connection_metadata() {
    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    let _receiver = manager.register_notification_stream(user_id).await;

    let metadata = manager.get_connection_metadata().await;
    assert_eq!(metadata.len(), 1);

    let key = format!("notification_{user_id}");
    assert!(metadata.contains_key(&key));
}

#[tokio::test]
async fn test_get_connection_metadata_multiple_types() {
    let manager = SseManager::new(10);

    let user_id = Uuid::new_v4();
    let _r1 = manager.register_notification_stream(user_id).await;

    let task_id = "task-multi".to_owned();
    let _r2 = manager
        .register_a2a_task_stream(task_id.clone(), "client".to_owned())
        .await;

    let metadata = manager.get_connection_metadata().await;
    assert_eq!(metadata.len(), 2);

    // Check notification connection exists
    let notification_key = format!("notification_{user_id}");
    assert!(metadata.contains_key(&notification_key));

    // Check A2A task connection exists
    let task_key = format!("a2a_task_{task_id}");
    assert!(metadata.contains_key(&task_key));
}

#[tokio::test]
async fn test_active_streams_count() {
    let manager = SseManager::new(10);

    assert_eq!(manager.active_notification_streams().await, 0);
    assert_eq!(manager.active_protocol_streams().await, 0);
    assert_eq!(manager.active_a2a_task_streams().await, 0);

    // Add notification stream
    let _r1 = manager.register_notification_stream(Uuid::new_v4()).await;
    assert_eq!(manager.active_notification_streams().await, 1);

    // Add A2A task stream
    let _r2 = manager
        .register_a2a_task_stream("task".to_owned(), "client".to_owned())
        .await;
    assert_eq!(manager.active_a2a_task_streams().await, 1);

    // Protocol streams stay at 0 (requires server resources)
    assert_eq!(manager.active_protocol_streams().await, 0);
}

#[tokio::test]
async fn test_send_notification_nonexistent_user() {
    use pierre_mcp_server::database::oauth_notifications::OAuthNotification;

    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    let notification = OAuthNotification {
        id: user_id.to_string(),
        user_id: user_id.to_string(),
        provider: "strava".to_owned(),
        success: true,
        message: "Token refreshed successfully".to_owned(),
        expires_at: None,
        created_at: chrono::Utc::now(),
        read_at: None,
    };

    let result = manager.send_notification(user_id, &notification).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_a2a_task_update_nonexistent_task() {
    let manager = SseManager::new(10);

    let result = manager
        .send_a2a_task_update("nonexistent-task", "update data".to_owned())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_mcp_request_nonexistent_session() {
    use pierre_mcp_server::jsonrpc::JsonRpcRequest;
    use std::collections::HashMap;

    let manager = SseManager::new(10);

    // Create a minimal JSON-RPC request
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(serde_json::json!(1)),
        method: "ping".to_owned(),
        params: None,
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let result = manager
        .send_mcp_request("nonexistent-session", request)
        .await;
    assert!(result.is_err());
}

// =============================================================================
// Cleanup Tests
// =============================================================================

#[tokio::test]
async fn test_cleanup_inactive_connections_no_connections() {
    let manager = SseManager::new(10);

    // Should not panic with no connections
    manager.cleanup_inactive_connections(3600).await;
    assert_eq!(manager.active_notification_streams().await, 0);
}

#[tokio::test]
async fn test_cleanup_with_active_connections() {
    let manager = SseManager::new(10);

    let _r1 = manager.register_notification_stream(Uuid::new_v4()).await;
    let _r2 = manager
        .register_a2a_task_stream("task".to_owned(), "client".to_owned())
        .await;

    // With a large timeout, no connections should be cleaned up
    manager.cleanup_inactive_connections(3600).await;

    // Connections should still exist (they were just created)
    assert_eq!(manager.active_notification_streams().await, 1);
    assert_eq!(manager.active_a2a_task_streams().await, 1);
}

// =============================================================================
// Concurrent Access Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_stream_registration() {
    let manager = SseManager::new(100);
    let manager_clone = manager.clone();

    // Spawn multiple tasks to register streams concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let mgr = manager.clone();
            tokio::spawn(async move {
                let user_id = Uuid::new_v4();
                let _receiver = mgr.register_notification_stream(user_id).await;
                (i, user_id)
            })
        })
        .collect();

    // Wait for all to complete
    for handle in handles {
        let _ = handle.await.unwrap();
    }

    // Should have 10 notification streams
    assert_eq!(manager_clone.active_notification_streams().await, 10);
}

#[tokio::test]
async fn test_concurrent_a2a_task_registration() {
    let manager = SseManager::new(100);

    let handles: Vec<_> = (0..5)
        .map(|i| {
            let mgr = manager.clone();
            tokio::spawn(async move {
                let task_id = format!("task-{i}");
                let client_id = format!("client-{i}");
                let _receiver = mgr.register_a2a_task_stream(task_id, client_id).await;
            })
        })
        .collect();

    for handle in handles {
        handle.await.unwrap();
    }

    assert_eq!(manager.active_a2a_task_streams().await, 5);
}

// =============================================================================
// Edge Cases
// =============================================================================

#[tokio::test]
async fn test_register_same_user_twice() {
    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    let _r1 = manager.register_notification_stream(user_id).await;
    let _r2 = manager.register_notification_stream(user_id).await;

    // Second registration should replace the first
    // So we should have 1 stream, not 2
    assert_eq!(manager.active_notification_streams().await, 1);
}

#[tokio::test]
async fn test_register_same_task_twice() {
    let manager = SseManager::new(10);
    let task_id = "same-task".to_owned();

    let _r1 = manager
        .register_a2a_task_stream(task_id.clone(), "client1".to_owned())
        .await;
    let _r2 = manager
        .register_a2a_task_stream(task_id.clone(), "client2".to_owned())
        .await;

    // Second registration should replace the first
    assert_eq!(manager.active_a2a_task_streams().await, 1);
}

#[tokio::test]
async fn test_small_buffer_size() {
    let manager = SseManager::new(1); // Very small buffer

    let _r1 = manager.register_notification_stream(Uuid::new_v4()).await;
    assert_eq!(manager.active_notification_streams().await, 1);
}

#[tokio::test]
async fn test_large_buffer_size() {
    let manager = SseManager::new(10000); // Large buffer

    let _r1 = manager.register_notification_stream(Uuid::new_v4()).await;
    assert_eq!(manager.active_notification_streams().await, 1);
}

#[tokio::test]
async fn test_metadata_timestamps() {
    let manager = SseManager::new(10);
    let user_id = Uuid::new_v4();

    let _receiver = manager.register_notification_stream(user_id).await;

    let metadata = manager.get_connection_metadata().await;
    let key = format!("notification_{user_id}");
    let conn_meta = metadata.get(&key).unwrap();

    // created_at should be <= last_activity
    assert!(conn_meta.created_at <= conn_meta.last_activity);

    // Both timestamps should be recent (within last minute)
    let now = chrono::Utc::now();
    let one_minute_ago = now - chrono::Duration::minutes(1);
    assert!(conn_meta.created_at > one_minute_ago);
}
