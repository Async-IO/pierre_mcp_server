// ABOUTME: Integration tests for A2A (Agent-to-Agent) protocol compliance
// ABOUTME: Validates adherence to Google A2A specification requirements
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! A2A Protocol Compliance Tests
//!
//! Tests to ensure our A2A implementation complies with the official
//! Google A2A specification at <https://github.com/google-a2a/A2A>

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::a2a::protocol::{A2ARequest, A2AServer};
use serde_json::json;
use std::collections::HashMap;

/// JSON-RPC auth error code used when authentication is required
const AUTH_ERROR_CODE: i32 = -32001;

#[tokio::test]
async fn test_jsonrpc_2_0_compliance() {
    let server = A2AServer::new();

    // Test that all responses have jsonrpc: "2.0" (initialize does not require auth)
    let request = A2ARequest {
        jsonrpc: "2.0".to_owned(),
        method: "a2a/initialize".to_owned(),
        params: None,
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = server.handle_request(request).await;
    assert_eq!(response.jsonrpc, "2.0");
}

#[tokio::test]
async fn test_unauthenticated_methods_require_auth() {
    let server = A2AServer::new();

    // Methods that use require_auth_then must reject unauthenticated requests
    let protected_methods = vec!["tasks/create", "tasks/get", "a2a/tasks/list", "tools/call"];

    for method in protected_methods {
        let request = A2ARequest {
            jsonrpc: "2.0".to_owned(),
            method: method.to_owned(),
            params: None,
            id: Some(json!(1)),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        };

        let response = server.handle_request(request).await;

        assert!(
            response.error.is_some(),
            "Method {method} should require authentication"
        );
        assert_eq!(
            response.error.as_ref().unwrap().code,
            AUTH_ERROR_CODE,
            "Method {method} should return auth error code {AUTH_ERROR_CODE}"
        );
    }
}

#[tokio::test]
async fn test_initialize_does_not_require_auth() {
    let server = A2AServer::new();

    // a2a/initialize is the bootstrapping endpoint and must work without auth
    let request = A2ARequest {
        jsonrpc: "2.0".to_owned(),
        method: "a2a/initialize".to_owned(),
        params: None,
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = server.handle_request(request).await;
    assert!(
        response.error.is_none(),
        "a2a/initialize should not require auth"
    );
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_error_code_compliance() {
    let server = A2AServer::new();

    // Test unknown method returns correct error code (-32601 Method not found)
    let request = A2ARequest {
        jsonrpc: "2.0".to_owned(),
        method: "unknown/method".to_owned(),
        params: None,
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = server.handle_request(request).await;
    assert!(response.error.is_some());
    // Unknown methods are routed to handle_unknown_method which returns -32601
    assert_eq!(response.error.unwrap().code, -32601);
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
        id: "test-message".to_owned(),
        parts: vec![
            MessagePart::Text {
                content: "Hello".to_owned(),
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

    // Test that unauthenticated task creation is rejected (security requirement)
    let request = A2ARequest {
        jsonrpc: "2.0".to_owned(),
        method: "a2a/tasks/create".to_owned(),
        params: Some(json!({"task_type": "example"})),
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = server.handle_request(request).await;
    // tasks/create requires authentication - unauthenticated requests must be rejected
    assert!(
        response.error.is_some(),
        "Unauthenticated tasks/create must return an error"
    );
    assert!(response.result.is_none());
}

#[tokio::test]
async fn test_tools_schema_compliance() {
    let server = A2AServer::new();

    // Test tools list returns proper schema
    let request = A2ARequest {
        jsonrpc: "2.0".to_owned(),
        method: "a2a/tools/list".to_owned(),
        params: None,
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = server.handle_request(request).await;
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    // Response format is {"tools": [...]} with tools wrapped in an object
    let tools = &result["tools"];
    assert!(tools.is_array(), "Expected tools array in response");

    // Verify each tool has required schema
    for tool in tools.as_array().unwrap() {
        assert!(tool["name"].is_string());
        assert!(tool["description"].is_string());
        // Schema may use either "inputSchema" (MCP standard) or "parameters" (legacy)
        assert!(
            tool["inputSchema"].is_object() || tool["parameters"].is_object(),
            "Tool must have inputSchema or parameters"
        );
    }
}

#[tokio::test]
async fn test_streaming_requirements() {
    let server = A2AServer::new();

    // Test streaming endpoint exists and responds appropriately
    let request = A2ARequest {
        jsonrpc: "2.0".to_owned(),
        method: "a2a/message/stream".to_owned(),
        params: Some(json!({"stream_id": "test"})),
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
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
    assert!(auth_schemes.contains(&"api-key".to_owned()));
    assert!(auth_schemes.contains(&"oauth2".to_owned()));

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

    // Test that request ID is preserved in response (using initialize which doesn't need auth)
    let test_ids = vec![json!(1), json!("string-id"), json!(null)];

    for test_id in test_ids {
        let request = A2ARequest {
            jsonrpc: "2.0".to_owned(),
            method: "a2a/initialize".to_owned(),
            params: None,
            id: Some(test_id.clone()),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        };

        let response = server.handle_request(request).await;
        assert_eq!(response.id, Some(test_id));
    }
}

#[tokio::test]
async fn test_id_preservation_on_auth_errors() {
    let server = A2AServer::new();

    // Verify request ID is preserved even when auth fails
    let test_ids = vec![json!(42), json!("req-abc"), json!(null)];

    for test_id in test_ids {
        let request = A2ARequest {
            jsonrpc: "2.0".to_owned(),
            method: "tools/list".to_owned(),
            params: None,
            id: Some(test_id.clone()),
            auth_token: None,
            headers: None,
            metadata: HashMap::new(),
        };

        let response = server.handle_request(request).await;
        assert_eq!(
            response.id,
            Some(test_id),
            "Request ID must be preserved in auth error responses"
        );
    }
}

#[tokio::test]
async fn test_agent_card_with_custom_base_url() {
    use pierre_mcp_server::a2a::agent_card::AgentCard;

    let base_url = "https://api.pierre.ai";
    let agent_card = AgentCard::with_base_url(base_url);

    // Verify transport endpoints use the custom base URL
    assert!(!agent_card.transports.is_empty());
    let transport = &agent_card.transports[0];
    assert!(
        transport.endpoint.starts_with(base_url),
        "Transport endpoint should use custom base URL"
    );

    // Verify OAuth URLs use the custom base URL
    let oauth2 = agent_card.authentication.oauth2.as_ref().unwrap();
    assert!(
        oauth2.authorization_url.starts_with(base_url),
        "OAuth URLs should use custom base URL"
    );
}
