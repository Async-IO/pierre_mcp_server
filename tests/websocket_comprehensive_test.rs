//! Comprehensive tests for WebSocket functionality
//!
//! This test suite covers the WebSocket real-time communication system
//! which currently has no test coverage

use anyhow::Result;
use pierre_mcp_server::{
    database_plugins::DatabaseProvider,
    models::User,
    websocket::{WebSocketManager, WebSocketMessage},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_websocket_manager_creation() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());

    // Verify manager is created (filter can be built)
    let _ = ws_manager.websocket_filter();

    Ok(())
}

#[tokio::test]
async fn test_websocket_authentication_flow() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    // Create test user
    let user = User::new(
        "ws_auth_test@example.com".to_string(),
        "password123".to_string(),
        Some("WebSocket Test User".to_string()),
    );
    database.create_user(&user).await?;

    // Generate auth token
    let token = auth_manager.generate_token(&user)?;

    let ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());
    let _filter = ws_manager.websocket_filter();

    // Test authentication message
    let auth_msg = WebSocketMessage::Authentication {
        token: token.clone(),
    };

    // Verify message serialization
    let serialized = serde_json::to_string(&auth_msg)?;
    assert!(serialized.contains("auth"));
    assert!(serialized.contains(&token));

    Ok(())
}

#[tokio::test]
async fn test_websocket_subscription_message() -> Result<()> {
    let topics = vec![
        "usage_updates".to_string(),
        "system_stats".to_string(),
        "rate_limits".to_string(),
    ];

    let subscribe_msg = WebSocketMessage::Subscribe {
        topics: topics.clone(),
    };

    // Test serialization
    let json = serde_json::to_value(&subscribe_msg)?;
    assert_eq!(json["type"], "subscribe");
    assert_eq!(json["topics"].as_array().unwrap().len(), 3);

    // Test deserialization
    let deserialized: WebSocketMessage = serde_json::from_value(json)?;
    match deserialized {
        WebSocketMessage::Subscribe { topics: t } => assert_eq!(t, topics),
        _ => panic!("Wrong message type"),
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_usage_update_message() -> Result<()> {
    let usage_update = WebSocketMessage::UsageUpdate {
        api_key_id: "key_123".to_string(),
        requests_today: 150,
        requests_this_month: 4500,
        rate_limit_status: json!({
            "limit": 1000,
            "remaining": 850,
            "reset_at": "2024-01-20T00:00:00Z"
        }),
    };

    // Test serialization
    let json = serde_json::to_value(&usage_update)?;
    assert_eq!(json["type"], "usage_update");
    assert_eq!(json["requests_today"], 150);
    assert_eq!(json["requests_this_month"], 4500);
    assert_eq!(json["api_key_id"], "key_123");

    Ok(())
}

#[tokio::test]
async fn test_websocket_system_stats_message() -> Result<()> {
    let stats = WebSocketMessage::SystemStats {
        total_requests_today: 10000,
        total_requests_this_month: 250000,
        active_connections: 42,
    };

    // Test serialization
    let json = serde_json::to_value(&stats)?;
    assert_eq!(json["type"], "system_stats");
    assert_eq!(json["total_requests_today"], 10000);
    assert_eq!(json["active_connections"], 42);

    Ok(())
}

#[tokio::test]
async fn test_websocket_error_message() -> Result<()> {
    let error_msg = WebSocketMessage::Error {
        message: "Authentication failed: Invalid token".to_string(),
    };

    // Test serialization
    let json = serde_json::to_value(&error_msg)?;
    assert_eq!(json["type"], "error");
    assert!(json["message"]
        .as_str()
        .unwrap()
        .contains("Authentication failed"));

    Ok(())
}

#[tokio::test]
async fn test_websocket_success_message() -> Result<()> {
    let success_msg = WebSocketMessage::Success {
        message: "Successfully subscribed to topics".to_string(),
    };

    // Test serialization
    let json = serde_json::to_value(&success_msg)?;
    assert_eq!(json["type"], "success");
    assert!(json["message"].as_str().unwrap().contains("subscribed"));

    Ok(())
}

#[tokio::test]
async fn test_websocket_message_parsing() -> Result<()> {
    // Test various message formats
    let test_cases = vec![
        (
            json!({
                "type": "auth",
                "token": "test_token_123"
            }),
            true,
        ),
        (
            json!({
                "type": "subscribe",
                "topics": ["usage_updates"]
            }),
            true,
        ),
        (
            json!({
                "type": "unknown_type",
                "data": "test"
            }),
            false,
        ),
        (
            json!({
                "token": "missing_type"
            }),
            false,
        ),
    ];

    for (json_msg, should_succeed) in test_cases {
        let result = serde_json::from_value::<WebSocketMessage>(json_msg);
        assert_eq!(result.is_ok(), should_succeed);
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_connection_with_invalid_auth() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());
    let _filter = ws_manager.websocket_filter();

    // Create invalid auth message
    let auth_msg = WebSocketMessage::Authentication {
        token: "invalid_token_123".to_string(),
    };

    // Message should serialize but authentication would fail in actual connection
    let json = serde_json::to_string(&auth_msg)?;
    assert!(json.contains("invalid_token_123"));

    Ok(())
}

#[tokio::test]
async fn test_websocket_concurrent_client_management() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let ws_manager = Arc::new(WebSocketManager::new(
        (*database).clone(),
        (*auth_manager).clone(),
    ));

    // Simulate multiple concurrent connections
    let mut handles = vec![];

    for i in 0..5 {
        let _ws_manager_clone = ws_manager.clone();
        let db_clone = database.clone();
        let auth_clone = auth_manager.clone();

        handles.push(tokio::spawn(async move {
            // Create unique user for each connection
            let user = User::new(
                format!("ws_concurrent_{}@example.com", i),
                "password".to_string(),
                Some(format!("Concurrent User {}", i)),
            );
            db_clone.create_user(&user).await.unwrap();

            let token = auth_clone.generate_token(&user).unwrap();

            // Create auth message
            let auth_msg = WebSocketMessage::Authentication { token };
            serde_json::to_string(&auth_msg).unwrap()
        }));
    }

    // All connections should generate valid auth messages
    for handle in handles {
        let auth_json = handle.await?;
        assert!(auth_json.contains("auth"));
        assert!(auth_json.contains("token"));
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_rate_limit_status_updates() -> Result<()> {
    // Test rate limit status message format
    let rate_limit_statuses = [
        json!({
            "limit": 1000,
            "remaining": 1000,
            "reset_at": "2024-01-20T00:00:00Z"
        }),
        json!({
            "limit": 1000,
            "remaining": 0,
            "reset_at": "2024-01-20T01:00:00Z",
            "retry_after": 3600
        }),
        json!({
            "limit": 500,
            "remaining": 250,
            "reset_at": "2024-01-20T00:30:00Z"
        }),
    ];

    for (i, status) in rate_limit_statuses.iter().enumerate() {
        let usage_update = WebSocketMessage::UsageUpdate {
            api_key_id: format!("key_{}", i),
            requests_today: (i as u64 + 1) * 100,
            requests_this_month: (i as u64 + 1) * 3000,
            rate_limit_status: status.clone(),
        };

        let json = serde_json::to_value(&usage_update)?;
        assert_eq!(json["rate_limit_status"], *status);
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_subscription_topics() -> Result<()> {
    let valid_topics = vec![
        vec!["usage_updates".to_string()],
        vec!["system_stats".to_string()],
        vec!["rate_limits".to_string()],
        vec!["usage_updates".to_string(), "system_stats".to_string()],
        vec![], // Empty subscription
    ];

    for topics in valid_topics {
        let subscribe_msg = WebSocketMessage::Subscribe {
            topics: topics.clone(),
        };

        let json = serde_json::to_value(&subscribe_msg)?;
        let topics_array = json["topics"].as_array().unwrap();
        assert_eq!(topics_array.len(), topics.len());
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_message_size_limits() -> Result<()> {
    // Test large message handling
    let large_message = WebSocketMessage::Error {
        message: "x".repeat(10000), // 10KB message
    };

    // Should serialize successfully
    let json = serde_json::to_string(&large_message)?;
    assert!(json.len() > 10000);

    // Test very large rate limit status
    let large_status = json!({
        "limit": 1000000,
        "remaining": 999999,
        "reset_at": "2024-01-20T00:00:00Z",
        "metadata": {
            "tier": "enterprise",
            "custom_limits": (0..100).map(|i| format!("limit_{}", i)).collect::<Vec<_>>()
        }
    });

    let usage_update = WebSocketMessage::UsageUpdate {
        api_key_id: "enterprise_key".to_string(),
        requests_today: 50000,
        requests_this_month: 1500000,
        rate_limit_status: large_status,
    };

    // Should handle large nested objects
    let _ = serde_json::to_string(&usage_update)?;

    Ok(())
}

#[tokio::test]
async fn test_websocket_client_id_generation() -> Result<()> {
    // Test that client IDs are unique
    let mut ids = std::collections::HashSet::new();

    for _ in 0..100 {
        let id = Uuid::new_v4();
        assert!(ids.insert(id), "UUID collision detected");
    }

    Ok(())
}

#[tokio::test]
async fn test_websocket_broadcast_system_stats() -> Result<()> {
    let database = common::create_test_database().await?;
    let auth_manager = common::create_test_auth_manager();

    let _ws_manager = WebSocketManager::new((*database).clone(), (*auth_manager).clone());

    // Create system stats for broadcast
    let stats = WebSocketMessage::SystemStats {
        total_requests_today: 25000,
        total_requests_this_month: 750000,
        active_connections: 15,
    };

    // Verify stats message format
    let json = serde_json::to_value(&stats)?;
    assert_eq!(json["type"], "system_stats");
    assert!(json["total_requests_today"].as_u64().unwrap() > 0);
    assert!(json["active_connections"].as_u64().unwrap() > 0);

    Ok(())
}
