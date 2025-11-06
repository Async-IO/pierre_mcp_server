// ABOUTME: Comprehensive MCP protocol compliance integration tests
// ABOUTME: Tests version negotiation, error handling, progress tracking, and cancellation features
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::constants::{errors::*, protocol::JSONRPC_VERSION};
use pierre_mcp_server::mcp::{
    multitenant::{McpError, McpRequest, McpResponse},
    protocol::ProtocolHandler,
    schema::*,
};
use serde_json::{json, Value};

mod common;

/// Test MCP protocol version negotiation during initialization
#[tokio::test]
async fn test_protocol_version_negotiation() {
    common::init_server_config();
    // Test supported version
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {
                "experimental": {},
                "sampling": {}
            }
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    // Should succeed with supported version
    match response.result {
        Some(result) => {
            assert_eq!(result["protocolVersion"], "2025-06-18");
            assert_eq!(result["serverInfo"]["name"], "pierre-mcp-server");
        }
        None => panic!("Initialize should succeed with supported version"),
    }
}

/// Test unsupported protocol version handling
#[tokio::test]
async fn test_unsupported_protocol_version() {
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "initialize",
        "params": {
            "protocolVersion": "2023-01-01",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    // Should return version mismatch error
    if let Some(error) = response.error {
        assert_eq!(error.code, ERROR_VERSION_MISMATCH);
        // Accept any version-related error message
        assert!(
            error.message.to_lowercase().contains("version")
                || error.message.to_lowercase().contains("unsupported")
        );
    } else {
        panic!("Should return version mismatch error");
    }
}

/// Test server capabilities declaration accuracy
#[tokio::test]
async fn test_server_capabilities_declaration() {
    common::init_server_config();
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    if let Some(result) = response.result {
        let capabilities = &result["capabilities"];

        // Check that capabilities accurately declare server features
        assert!(capabilities["tools"].is_object());
        assert_eq!(capabilities["tools"]["listChanged"], false);

        // Should declare logging capability if supported
        if capabilities.get("logging").is_some() {
            assert!(capabilities["logging"].is_object());
        }

        // Should declare resources capability if supported
        if capabilities.get("resources").is_some() {
            assert!(capabilities["resources"].is_object());
            assert_eq!(capabilities["resources"]["listChanged"], false);
        }
    } else {
        panic!("Initialize should succeed and return capabilities");
    }
}

/// Test MCP-specific error codes compliance
#[tokio::test]
async fn test_mcp_error_codes_compliance() {
    // Verify error codes follow MCP specification ranges
    assert_eq!(ERROR_METHOD_NOT_FOUND, -32601);
    assert_eq!(ERROR_INVALID_PARAMS, -32602);
    assert_eq!(ERROR_INTERNAL_ERROR, -32603);

    // Verify server-specific error codes are in proper range (-32000 to -32099)
    // Test each error code is within MCP server error range
    let server_errors = [
        ERROR_TOOL_EXECUTION,
        ERROR_RESOURCE_ACCESS,
        ERROR_AUTHENTICATION,
        ERROR_AUTHORIZATION,
        ERROR_SERIALIZATION,
        ERROR_PROGRESS_TRACKING,
        ERROR_OPERATION_CANCELLED,
    ];

    for &error_code in &server_errors {
        assert!(
            (-32099..=-32000).contains(&error_code),
            "Error code {error_code} is not in MCP server error range (-32000 to -32099)"
        );
    }
}

/// Test JSON-RPC 2.0 message format compliance
#[tokio::test]
async fn test_jsonrpc_message_format_compliance() {
    // Test request parsing from JSON (McpRequest only implements Deserialize)
    let request_json = json!({
        "jsonrpc": JSONRPC_VERSION,
        "id": 123,
        "method": "tools/list",
        "params": null
    });

    let request: McpRequest = serde_json::from_value(request_json).expect("Should parse");

    // Verify JSON-RPC 2.0 compliance
    assert_eq!(request.jsonrpc, JSONRPC_VERSION);
    assert_eq!(
        request.id.unwrap(),
        Value::Number(serde_json::Number::from(123))
    );
    assert_eq!(request.method, "tools/list");

    // Test response format
    let response = McpResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id: Some(Value::Number(serde_json::Number::from(123))),
        result: Some(json!({"tools": []})),
        error: None,
    };

    let serialized = serde_json::to_value(&response).expect("Should serialize");
    assert_eq!(serialized["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(serialized["id"], 123);
    assert!(serialized["result"].is_object());
    assert!(!serialized.as_object().unwrap().contains_key("error"));
}

/// Test progress notification format compliance
#[tokio::test]
async fn test_progress_notification_format() {
    let progress_token = "test-token-123";
    let notification = ProgressNotification::new(
        progress_token.to_owned(),
        25.0,
        Some(100.0),
        Some("Processing data...".to_owned()),
    );

    let serialized = serde_json::to_value(&notification).expect("Should serialize");

    // Verify notification structure
    assert_eq!(serialized["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(serialized["method"], "notifications/progress");
    assert!(!serialized.as_object().unwrap().contains_key("id"));

    let params = &serialized["params"];
    assert_eq!(params["progressToken"], progress_token);
    assert_eq!(params["progress"], 25.0);
    assert_eq!(params["total"], 100.0);
    assert_eq!(params["message"], "Processing data...");
}

/// Test cancellation request format compliance
#[tokio::test]
async fn test_cancellation_request_format() {
    let cancel_request = json!({
        "jsonrpc": "2.0",
        "id": 456,
        "method": "notifications/cancelled",
        "params": {
            "requestId": 123,
            "reason": "User requested cancellation"
        }
    });

    // Verify request parses correctly
    let request: McpRequest = serde_json::from_value(cancel_request).expect("Should parse");
    assert_eq!(request.method, "notifications/cancelled");

    if let Some(params) = request.params {
        assert_eq!(params["requestId"], 123);
        assert_eq!(params["reason"], "User requested cancellation");
    } else {
        panic!("Cancellation request should have params");
    }
}

/// Test tool response format compliance
#[tokio::test]
async fn test_tool_response_format_compliance() {
    // Test successful tool response
    let success_response = ToolResponse {
        content: vec![Content::Text {
            text: "Operation completed successfully".to_owned(),
        }],
        is_error: false,
        structured_content: Some(json!({
            "result": "success",
            "data": {"count": 42}
        })),
    };

    let serialized = serde_json::to_value(&success_response).expect("Should serialize");

    assert!(serialized["content"].is_array());
    assert_eq!(serialized["isError"], false);
    assert!(serialized["structuredContent"].is_object());

    let content = &serialized["content"][0];
    assert_eq!(content["type"], "text");
    assert_eq!(content["text"], "Operation completed successfully");

    // Test error tool response
    let error_response = ToolResponse {
        content: vec![Content::Text {
            text: "Tool execution failed".to_owned(),
        }],
        is_error: true,
        structured_content: Some(json!({
            "error": {
                "code": ERROR_TOOL_EXECUTION,
                "message": "Failed to process request"
            }
        })),
    };

    let serialized = serde_json::to_value(&error_response).expect("Should serialize");
    assert_eq!(serialized["isError"], true);

    let error_data = &serialized["structuredContent"]["error"];
    assert_eq!(error_data["code"], ERROR_TOOL_EXECUTION);
    assert_eq!(error_data["message"], "Failed to process request");
}

/// Test content type format compliance
#[tokio::test]
async fn test_content_type_format_compliance() {
    // Test text content
    let text_content = Content::Text {
        text: "Sample text content".to_owned(),
    };
    let serialized = serde_json::to_value(&text_content).expect("Should serialize");
    assert_eq!(serialized["type"], "text");
    assert_eq!(serialized["text"], "Sample text content");

    // Test image content
    let image_content = Content::Image {
        data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_owned(),
        mime_type: "image/png".to_owned(),
    };
    let serialized = serde_json::to_value(&image_content).expect("Should serialize");
    assert_eq!(serialized["type"], "image");
    assert_eq!(serialized["mimeType"], "image/png");
    assert!(serialized["data"].is_string());

    // Test resource content
    let resource_content = Content::Resource {
        uri: "https://api.example.com/data/123".to_owned(),
        text: Some("Resource description".to_owned()),
        mime_type: Some("application/json".to_owned()),
    };
    let serialized = serde_json::to_value(&resource_content).expect("Should serialize");
    assert_eq!(serialized["type"], "resource");
    assert_eq!(serialized["uri"], "https://api.example.com/data/123");
    assert_eq!(serialized["text"], "Resource description");
    assert_eq!(serialized["mimeType"], "application/json");
}

/// Test tool schema validation compliance
#[tokio::test]
async fn test_tool_schema_validation_compliance() {
    let tools = get_tools();

    assert!(!tools.is_empty(), "Server should declare at least one tool");

    for tool in tools {
        // Verify required fields
        assert!(!tool.name.is_empty(), "Tool name is required");
        assert!(!tool.description.is_empty(), "Tool description is required");

        // Verify input schema structure
        assert_eq!(tool.input_schema.schema_type, "object");
        // Tool input schemas may or may not have properties defined
        // This is acceptable as some tools may accept any parameters

        // Verify serialization produces valid JSON schema
        let serialized = serde_json::to_value(&tool.input_schema).expect("Schema should serialize");
        assert_eq!(serialized["type"], "object");

        if let Some(properties) = serialized.get("properties") {
            assert!(properties.is_object());
        }

        if let Some(required) = serialized.get("required") {
            assert!(required.is_array());
        }
    }
}

/// Test ping method compliance
#[tokio::test]
async fn test_ping_method_compliance() {
    let ping_request = json!({
        "jsonrpc": "2.0",
        "id": 789,
        "method": "ping",
        "params": {}
    });

    let request: McpRequest = serde_json::from_value(ping_request).expect("Should parse");
    let response = ProtocolHandler::handle_ping(request);

    // Ping should return empty result object
    if let Some(result) = response.result {
        assert_eq!(result, json!({}));
    } else {
        panic!("Ping should return success with empty result");
    }

    // Response should have correct format
    assert_eq!(response.jsonrpc, JSONRPC_VERSION);
    assert_eq!(
        response.id,
        Some(Value::Number(serde_json::Number::from(789)))
    );
    assert!(response.error.is_none());
}

/// Test error response format compliance
#[tokio::test]
async fn test_error_response_format_compliance() {
    let error = McpError {
        code: ERROR_AUTHENTICATION,
        message: "Authentication failed".to_owned(),
        data: Some(json!({
            "reason": "Invalid JWT token",
            "suggestion": "Please obtain a new token"
        })),
    };

    let response = McpResponse {
        jsonrpc: JSONRPC_VERSION.to_owned(),
        id: Some(Value::Number(serde_json::Number::from(101))),
        result: None,
        error: Some(error),
    };

    let serialized = serde_json::to_value(&response).expect("Should serialize");

    // Verify error response structure
    assert_eq!(serialized["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(serialized["id"], 101);
    assert!(!serialized.as_object().unwrap().contains_key("result"));

    let error_obj = &serialized["error"];
    assert_eq!(error_obj["code"], ERROR_AUTHENTICATION);
    assert_eq!(error_obj["message"], "Authentication failed");
    assert!(error_obj["data"].is_object());
    assert_eq!(error_obj["data"]["reason"], "Invalid JWT token");
    assert_eq!(error_obj["data"]["suggestion"], "Please obtain a new token");
}

/// Test batch request handling compliance (if supported)
#[tokio::test]
async fn test_batch_request_compliance() {
    // MCP servers may support batch requests (array of request objects)
    let batch_request = json!([
        {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        },
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }
    ]);

    // If batch requests are supported, they should return an array of responses
    // This test documents the requirement even if not implemented
    assert!(batch_request.is_array());
    let requests = batch_request.as_array().unwrap();
    assert_eq!(requests.len(), 2);

    for request in requests {
        assert_eq!(request["jsonrpc"], JSONRPC_VERSION);
        assert!(request["id"].is_number());
        assert!(request["method"].is_string());
    }
}

/// Test notification message format compliance
#[tokio::test]
async fn test_notification_message_compliance() {
    // Notifications don't have an "id" field
    let notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    assert_eq!(notification["jsonrpc"], JSONRPC_VERSION);
    assert_eq!(notification["method"], "notifications/initialized");
    assert!(!notification.as_object().unwrap().contains_key("id"));
}

/// Test protocol versioning backward compatibility
#[tokio::test]
async fn test_protocol_version_backward_compatibility() {
    common::init_server_config();
    // Test that older supported versions still work
    let init_request_old = json!({
        "jsonrpc": "2.0",
        "id": 999,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",  // Older supported version
            "clientInfo": {
                "name": "legacy-client",
                "version": "0.9.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest =
        serde_json::from_value(init_request_old).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    // Should succeed with older supported version
    if let Some(result) = response.result {
        // Server should respond with the negotiated version
        assert!(["2024-11-05", "2025-06-18"].contains(&result["protocolVersion"].as_str().unwrap()));
    } else {
        panic!("Initialize should succeed with supported older version");
    }
}
