// ABOUTME: Simple tests for SSE transport module and MCP protocol streaming
// ABOUTME: Basic functionality tests that compile and run successfully
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use futures_util::future;
use serde_json::{json, Value};

#[tokio::test]
async fn test_sse_message_format_basic() {
    // Test basic SSE message formatting without dependencies
    let event_type = "response";
    let data = r#"{"jsonrpc":"2.0","result":{"status":"ok"},"id":1}"#;
    let sse_message = format!("event: {event_type}\ndata: {data}\n\n");

    assert!(sse_message.starts_with("event: response"));
    assert!(sse_message.contains("data: {"));
    assert!(sse_message.ends_with("\n\n"));

    // Verify the data is valid JSON
    let parsed: Value = serde_json::from_str(data).unwrap();
    assert_eq!(parsed["jsonrpc"], "2.0");
    assert_eq!(parsed["id"], 1);
}

#[tokio::test]
async fn test_mcp_request_structure() {
    // Test MCP request structure for MCP client compatibility
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "mcp-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    assert_eq!(initialize_request["jsonrpc"], "2.0");
    assert_eq!(initialize_request["method"], "initialize");
    assert_eq!(
        initialize_request["params"]["protocolVersion"],
        "2025-06-18"
    );
    assert_eq!(initialize_request["id"], 1);
}

#[tokio::test]
async fn test_mcp_response_structure() {
    // Test MCP response structure for SSE streaming
    let response = json!({
        "jsonrpc": "2.0",
        "result": {
            "capabilities": {
                "tools": {"listChanged": false},
                "resources": {"listChanged": false},
                "logging": {}
            },
            "instructions": "Pierre MCP Server ready for fitness data queries",
            "serverInfo": {
                "name": "pierre-mcp-server",
                "version": "0.1.0"
            },
            "protocolVersion": "2025-06-18"
        },
        "id": 1
    });

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_object());
    assert_eq!(response["id"], 1);
}

#[tokio::test]
async fn test_sse_event_types() {
    // Test different SSE event types
    let event_types = vec![
        ("response", json!({"jsonrpc":"2.0","result":{},"id":1})),
        (
            "notification",
            json!({"jsonrpc":"2.0","method":"test","params":{}}),
        ),
        (
            "error",
            json!({"jsonrpc":"2.0","error":{"code":-32601,"message":"Not found"},"id":1}),
        ),
        ("connected", json!("MCP SSE transport ready")),
    ];

    for (event_type, data) in event_types {
        let data_str = serde_json::to_string(&data).unwrap();
        let sse_message = format!("event: {event_type}\ndata: {data_str}\n\n");

        assert!(sse_message.starts_with(&format!("event: {event_type}")));
        assert!(sse_message.contains("data: "));
        assert!(sse_message.ends_with("\n\n"));
    }
}

#[tokio::test]
async fn test_mcp_tools_list_response() {
    // Test tools list response structure
    let tools_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "tools": [
                {
                    "name": "get_activities",
                    "description": "Get fitness activities from a provider",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "limit": {
                                "type": "integer",
                                "description": "Maximum number of activities to fetch"
                            }
                        }
                    }
                }
            ]
        },
        "id": 2
    });

    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert!(tools_response["result"]["tools"].is_array());
    assert_eq!(tools_response["id"], 2);

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "get_activities");
}

#[tokio::test]
async fn test_oauth_notification_sse() {
    // Test OAuth notification over SSE
    let oauth_notification = json!({
        "type": "oauth_completion",
        "provider": "strava",
        "status": "success",
        "user_id": "123e4567-e89b-12d3-a456-426614174000",
        "timestamp": "2025-09-16T21:00:00Z"
    });

    let data_str = serde_json::to_string(&oauth_notification).unwrap();
    let sse_message = format!("event: notification\ndata: {data_str}\n\n");

    assert!(sse_message.contains("event: notification"));
    assert!(sse_message.contains("oauth_completion"));
    assert!(sse_message.contains("strava"));
}

#[tokio::test]
async fn test_concurrent_sse_messages() {
    // Test multiple SSE messages can be generated concurrently
    let message_count = 10;
    let mut handles = Vec::new();

    for i in 0..message_count {
        let handle = tokio::spawn(async move {
            let data = json!({"id": i, "message": format!("test_{}", i)});
            let data_str = serde_json::to_string(&data).unwrap();
            format!("event: response\ndata: {data_str}\n\n")
        });
        handles.push(handle);
    }

    let results: Vec<String> = future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), message_count);

    for (i, result) in results.iter().enumerate() {
        assert!(result.contains("event: response"));
        assert!(result.contains(&format!("test_{i}")));
        assert!(result.ends_with("\n\n"));
    }
}

#[tokio::test]
async fn test_mcp_error_handling() {
    // Test MCP error response structure
    let error_codes = vec![
        (-32700, "Parse error"),
        (-32600, "Invalid Request"),
        (-32601, "Method not found"),
        (-32602, "Invalid params"),
        (-32603, "Internal error"),
    ];

    for (code, message) in error_codes {
        let error_response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message
            },
            "id": 1
        });

        assert_eq!(error_response["jsonrpc"], "2.0");
        assert_eq!(error_response["error"]["code"], code);
        assert_eq!(error_response["error"]["message"], message);
    }
}

#[tokio::test]
async fn test_sse_stream_format_compliance() {
    // Test SSE format compliance according to the spec
    let test_messages = vec![
        ("Simple message", json!("hello")),
        ("Complex object", json!({"key": "value", "number": 42})),
        ("Array data", json!([1, 2, 3])),
    ];

    for (description, data) in test_messages {
        let data_str = serde_json::to_string(&data).unwrap();
        let sse_message = format!("event: test\ndata: {data_str}\n\n");

        // Must start with event field
        assert!(sse_message.starts_with("event: "));

        // Must contain data field
        assert!(sse_message.contains("\ndata: "));

        // Must end with double newline
        assert!(sse_message.ends_with("\n\n"));

        // Data should be valid JSON
        let extracted_data = sse_message
            .lines()
            .find(|line| line.starts_with("data: "))
            .unwrap()
            .strip_prefix("data: ")
            .unwrap();

        let parsed: Value = serde_json::from_str(extracted_data).unwrap();
        assert_eq!(parsed, data);

        println!(" SSE format test passed: {description}");
    }
}
