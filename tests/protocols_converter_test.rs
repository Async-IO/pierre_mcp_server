// ABOUTME: Integration tests for protocol converter functionality
// ABOUTME: Tests conversion between A2A, MCP, and universal protocol formats
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::a2a::protocol::A2ARequest;
use pierre_mcp_server::mcp::schema::ToolCall;
use pierre_mcp_server::protocols::converter::{ProtocolConverter, ProtocolType};
use pierre_mcp_server::protocols::universal::UniversalResponse;
use serde_json::Value;

#[test]
fn test_a2a_to_universal_conversion() {
    let a2a_request = A2ARequest {
        jsonrpc: "2.0".into(),
        method: "a2a/tools/call".into(),
        params: Some(serde_json::json!({
            "tool": "get_activities",
            "arguments": {
                "limit": 10
            }
        })),
        id: Some(Value::Number(1.into())),
        auth_token: None,
        headers: None,
        metadata: std::collections::HashMap::new(),
    };

    let universal = ProtocolConverter::a2a_to_universal(&a2a_request, "test_user").unwrap();

    assert_eq!(universal.tool_name, "get_activities");
    assert_eq!(universal.user_id, "test_user");
    assert_eq!(universal.protocol, "a2a");
    assert_eq!(
        universal.parameters.get("limit").unwrap().as_u64().unwrap(),
        10
    );
}

#[test]
fn test_universal_to_a2a_conversion_success() {
    let universal_response = UniversalResponse {
        success: true,
        result: Some(serde_json::json!({"activities": []})),
        error: None,
        metadata: None,
    };

    let a2a_response =
        ProtocolConverter::universal_to_a2a(universal_response, Some(Value::Number(1.into())));

    assert_eq!(a2a_response.jsonrpc, "2.0");
    assert!(a2a_response.result.is_some());
    assert!(a2a_response.error.is_none());
}

#[test]
fn test_universal_to_a2a_conversion_error() {
    let universal_response = UniversalResponse {
        success: false,
        result: None,
        error: Some("Tool not found".into()),
        metadata: None,
    };

    let a2a_response =
        ProtocolConverter::universal_to_a2a(universal_response, Some(Value::Number(1.into())));

    assert_eq!(a2a_response.jsonrpc, "2.0");
    assert!(a2a_response.result.is_none());
    assert!(a2a_response.error.is_some());
    assert_eq!(a2a_response.error.unwrap().message, "Tool not found");
}

#[test]
fn test_mcp_to_universal_conversion() {
    let mcp_call = ToolCall {
        name: "get_activities".into(),
        arguments: Some(serde_json::json!({"limit": 5})),
    };

    let universal = ProtocolConverter::mcp_to_universal(mcp_call, "test_user", None);

    assert_eq!(universal.tool_name, "get_activities");
    assert_eq!(universal.user_id, "test_user");
    assert_eq!(universal.protocol, "mcp");
    assert_eq!(
        universal.parameters.get("limit").unwrap().as_u64().unwrap(),
        5
    );
}

#[test]
fn test_universal_to_mcp_conversion_success() {
    let universal_response = UniversalResponse {
        success: true,
        result: Some(serde_json::json!({"data": "test"})),
        error: None,
        metadata: None,
    };

    let mcp_response = ProtocolConverter::universal_to_mcp(universal_response);

    assert!(!mcp_response.is_error);
    assert_eq!(mcp_response.content.len(), 1);
    match &mcp_response.content[0] {
        pierre_mcp_server::mcp::schema::Content::Text { text } => {
            assert!(text.contains("\"data\""));
            assert!(text.contains("\"test\""));
        }
        pierre_mcp_server::mcp::schema::Content::Image { .. } => {
            panic!("Expected text content, got image");
        }
        pierre_mcp_server::mcp::schema::Content::Resource { .. } => {
            panic!("Expected text content, got resource");
        }
        pierre_mcp_server::mcp::schema::Content::Progress { .. } => {
            panic!("Expected text content, got progress");
        }
    }
}

#[test]
fn test_universal_to_mcp_conversion_error() {
    let universal_response = UniversalResponse {
        success: false,
        result: None,
        error: Some("Invalid parameters".into()),
        metadata: None,
    };

    let mcp_response = ProtocolConverter::universal_to_mcp(universal_response);

    assert!(mcp_response.is_error);
    assert_eq!(mcp_response.content.len(), 1);
    match &mcp_response.content[0] {
        pierre_mcp_server::mcp::schema::Content::Text { text } => {
            assert!(text.contains("Invalid parameters"));
        }
        pierre_mcp_server::mcp::schema::Content::Image { .. } => {
            panic!("Expected text content, got image");
        }
        pierre_mcp_server::mcp::schema::Content::Resource { .. } => {
            panic!("Expected text content, got resource");
        }
        pierre_mcp_server::mcp::schema::Content::Progress { .. } => {
            panic!("Expected text content, got progress");
        }
    }
}

#[test]
fn test_detect_protocol_a2a() {
    let a2a_request = r#"{"jsonrpc": "2.0", "method": "a2a/tools/call", "id": 1}"#;
    let protocol = ProtocolConverter::detect_protocol(a2a_request).unwrap();
    assert_eq!(protocol, ProtocolType::A2A);
}

#[test]
fn test_detect_protocol_mcp() {
    let mcp_request = r#"{"method": "tools/call", "params": {}}"#;
    let protocol = ProtocolConverter::detect_protocol(mcp_request).unwrap();
    assert_eq!(protocol, ProtocolType::MCP);
}

#[test]
fn test_detect_protocol_unknown() {
    let unknown_request = r#"{"some": "data"}"#;
    let result = ProtocolConverter::detect_protocol(unknown_request);
    assert!(result.is_err());
}
