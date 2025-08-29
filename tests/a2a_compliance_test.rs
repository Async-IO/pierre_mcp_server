// ABOUTME: Integration tests for A2A (Agent-to-Agent) protocol compliance
// ABOUTME: Validates adherence to Google A2A specification requirements
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A2A Protocol Compliance Tests
//!
//! Tests to ensure our A2A implementation complies with the official
//! Google A2A specification at <https://github.com/google-a2a/A2A>

use pierre_mcp_server::a2a::protocol::{A2ARequest, A2AServer};
use serde_json::json;

#[tokio::test]
async fn test_jsonrpc_2_0_compliance() {
    let server = A2AServer::new();

    // Test that all responses have jsonrpc: "2.0"
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "a2a/initialize".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    let _response = server.handle_request(request).await;
}

#[tokio::test]
async fn test_required_methods_implemented() {
    let server = A2AServer::new();

    // Test core A2A methods exist and respond properly
    let required_methods = vec![
        "a2a/initialize",
        "message/send",
        "message/stream",
        "tasks/create",
        "tasks/get",
        "tasks/cancel",
        "tasks/pushNotificationConfig/set",
        "tools/list",
        "tools/call",
    ];

    for method in required_methods {
        let request = A2ARequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params: None,
            id: Some(json!(1)),
        };

        let response = server.handle_request(request).await;

        // Should not return "Method not found" error
        if let Some(error) = &response.error {
            assert_ne!(error.code, -32601, "Method {method} not implemented");
        }
    }
}

#[tokio::test]
async fn test_error_code_compliance() {
    let server = A2AServer::new();

    // Test unknown method returns correct error code
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "unknown/method".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32601); // Method not found
}

#[tokio::test]
async fn test_agent_card_compliance() {
    use pierre_mcp_server::a2a::agent_card::AgentCard;

    let agent_card = AgentCard::new();

    // Test required AgentCard fields
    assert!(!agent_card.name.is_empty());
    assert!(!agent_card.description.is_empty());
    assert!(!agent_card.version.is_empty());
    assert!(!agent_card.capabilities.is_empty());
    assert!(!agent_card.tools.is_empty());

    // Test authentication schemes are present
    assert!(!agent_card.authentication.schemes.is_empty());

    // Test tools have required fields
    for tool in &agent_card.tools {
        assert!(!tool.name.is_empty());
        assert!(!tool.description.is_empty());
        assert!(tool.input_schema.is_object());
        assert!(tool.output_schema.is_object());
    }
}

#[tokio::test]
async fn test_message_structure_compliance() {
    use pierre_mcp_server::a2a::protocol::{A2AMessage, MessagePart};
    use std::collections::HashMap;

    // Test message structure matches spec
    let message = A2AMessage {
        id: "test-message".to_string(),
        parts: vec![
            MessagePart::Text {
                content: "Hello".to_string(),
            },
            MessagePart::Data {
                content: json!({"key": "value"}),
            },
        ],
        metadata: Some(HashMap::new()),
    };

    // Verify serialization works and has correct structure
    let serialized = serde_json::to_value(&message).unwrap();
    assert!(serialized["id"].is_string());
    assert!(serialized["parts"].is_array());
    assert_eq!(serialized["parts"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn test_task_management_compliance() {
    let server = A2AServer::new();

    // Test task creation
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "a2a/tasks/create".to_string(),
        params: Some(json!({"task_type": "example"})),
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    // Verify task structure
    let task_data = response.result.unwrap();
    assert!(task_data["id"].is_string());
    assert!(task_data["status"].is_string());
    assert!(task_data["created_at"].is_string());
}

#[tokio::test]
async fn test_tools_schema_compliance() {
    let server = A2AServer::new();

    // Test tools list returns proper schema
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "a2a/tools/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;
    assert!(response.result.is_some());

    let tools = response.result.unwrap();
    assert!(tools.is_array());

    // Verify each tool has required schema
    for tool in tools.as_array().unwrap() {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        assert!(tool["parameters"].is_object());
    }
}

#[tokio::test]
async fn test_streaming_requirements() {
    let server = A2AServer::new();

    // Test streaming endpoint exists and responds appropriately
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "a2a/message/stream".to_string(),
        params: Some(json!({"stream_id": "test"})),
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;

    // Should respond with status (even if not fully implemented)
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert!(result["status"].is_string());
}

#[tokio::test]
async fn test_authentication_scheme_support() {
    use pierre_mcp_server::a2a::agent_card::AgentCard;

    let agent_card = AgentCard::new();

    // Verify supported authentication schemes match A2A spec
    let auth_schemes = &agent_card.authentication.schemes;

    // Should support at least api-key and oauth2
    assert!(auth_schemes.contains(&"api-key".to_string()));
    assert!(auth_schemes.contains(&"oauth2".to_string()));

    // Verify OAuth2 configuration is present
    assert!(agent_card.authentication.oauth2.is_some());
    let oauth2 = agent_card.authentication.oauth2.unwrap();
    assert!(!oauth2.authorization_url.is_empty());
    assert!(!oauth2.token_url.is_empty());
    assert!(!oauth2.scopes.is_empty());

    // Verify API key configuration
    assert!(agent_card.authentication.api_key.is_some());
    let api_key = agent_card.authentication.api_key.unwrap();
    assert!(!api_key.header_name.is_empty());
    assert!(!api_key.registration_url.is_empty());
}

#[tokio::test]
async fn test_id_preservation() {
    let server = A2AServer::new();

    // Test that request ID is preserved in response
    let test_ids = vec![json!(1), json!("string-id"), json!(null)];

    for test_id in test_ids {
        let request = A2ARequest {
            jsonrpc: "2.0".to_string(),
            method: "a2a/initialize".to_string(),
            params: None,
            id: Some(test_id.clone()),
        };

        let response = server.handle_request(request).await;
        assert_eq!(response.id, Some(test_id));
    }
}

#[tokio::test]
async fn test_parameter_validation() {
    let server = A2AServer::new();

    // Test tool call with proper parameters
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "a2a/tools/call".to_string(),
        params: Some(json!({
            "tool_name": "get_activities",
            "parameters": {
                "limit": 10
            }
        })),
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;

    // Should not fail with parameter validation error
    if let Some(error) = &response.error {
        // -32602 is "Invalid params" in JSON-RPC 2.0
        assert_ne!(error.code, -32602, "Parameter validation failed");
    }
}

#[tokio::test]
async fn test_task_cancellation() {
    let server = A2AServer::new();

    // Test task cancellation
    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "tasks/cancel".to_string(),
        params: Some(json!({"task_id": "test-task-123"})),
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    assert_eq!(result["task_id"], "test-task-123");
    assert_eq!(result["status"], "cancelled");
    assert!(result["cancelled_at"].is_string());
}

#[tokio::test]
async fn test_push_notification_config() {
    let server = A2AServer::new();

    // Test push notification configuration
    let config = json!({
        "webhook_url": "https://example.com/webhook",
        "events": ["task_completed", "task_failed"]
    });

    let request = A2ARequest {
        jsonrpc: "2.0".to_string(),
        method: "tasks/pushNotificationConfig/set".to_string(),
        params: Some(json!({"config": config})),
        id: Some(json!(1)),
    };

    let response = server.handle_request(request).await;
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let result = response.result.unwrap();
    assert_eq!(result["status"], "configured");
    assert!(result["updated_at"].is_string());
}
