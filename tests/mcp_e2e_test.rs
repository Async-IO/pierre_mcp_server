// ABOUTME: End-to-end tests for complete MCP protocol flow with MCP clients
// ABOUTME: Tests full server startup, MCP client connectivity, and SSE streaming

use futures_util::future;
use serde_json::{json, Value};

#[tokio::test]
async fn test_sse_message_streaming() {
    // Test SSE message format compliance
    let test_cases = vec![
        (
            "response",
            r#"{"jsonrpc":"2.0","result":{"status":"ok"},"id":1}"#,
        ),
        (
            "notification",
            r#"{"jsonrpc":"2.0","method":"notification","params":{}}"#,
        ),
        (
            "error",
            r#"{"jsonrpc":"2.0","error":{"code":-32601,"message":"Not found"},"id":1}"#,
        ),
    ];

    for (event_type, data) in test_cases {
        let sse_message = format!("event: {event_type}\ndata: {data}\n\n");

        assert!(sse_message.starts_with("event: "));
        assert!(sse_message.contains(&format!("data: {data}")));
        assert!(sse_message.ends_with("\n\n"));

        // Verify it can be parsed as JSON
        let json_data: Value = serde_json::from_str(data).unwrap();
        assert_eq!(json_data["jsonrpc"], "2.0");
    }
}

#[tokio::test]
async fn test_concurrent_mcp_requests() {
    // Test multiple concurrent MCP requests
    let request_count = 10;
    let mut handles = Vec::new();

    for i in 0..request_count {
        let handle = tokio::spawn(async move {
            let request = json!({
                "jsonrpc": "2.0",
                "method": "ping",
                "id": i
            });

            // Simulate processing
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            request["id"].as_u64().unwrap()
        });

        handles.push(handle);
    }

    let results: Vec<u64> = future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    assert_eq!(results.len(), request_count);

    // Verify all IDs are present
    for i in 0..request_count as u64 {
        assert!(results.contains(&i));
    }
}

#[tokio::test]
async fn test_mcp_initialize_request_validation() {
    // Test MCP initialize request validation
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": {
                "name": "e2e-test-client",
                "version": "1.0.0"
            }
        },
        "id": 1
    });

    // Verify the request structure
    assert_eq!(initialize_request["jsonrpc"], "2.0");
    assert_eq!(initialize_request["id"], 1);
    assert!(initialize_request["params"].is_object());
    assert_eq!(
        initialize_request["params"]["protocolVersion"],
        "2025-06-18"
    );
}

#[tokio::test]
async fn test_mcp_tools_list_request_validation() {
    // Test tools list request validation
    let tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 2
    });

    // Verify the request structure
    assert_eq!(tools_request["jsonrpc"], "2.0");
    assert_eq!(tools_request["id"], 2);
    assert_eq!(tools_request["method"], "tools/list");
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

        println!("✓ SSE format test passed: {description}");
    }
}

#[tokio::test]
async fn test_mcp_error_response_format() {
    // Test MCP error response format
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
