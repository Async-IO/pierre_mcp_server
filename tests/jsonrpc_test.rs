// ABOUTME: Tests for unified JSON-RPC 2.0 foundation module
// ABOUTME: Validates request, response, error structures and serialization
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::jsonrpc::{JsonRpcRequest, JsonRpcResponse, JSONRPC_VERSION};
use serde_json::Value;

#[test]
fn test_request_creation() {
    let req = JsonRpcRequest::new("test_method", None);
    assert_eq!(req.jsonrpc, JSONRPC_VERSION);
    assert_eq!(req.method, "test_method");
    assert!(req.params.is_none());
    assert!(req.id.is_some());
}

#[test]
fn test_notification_creation() {
    let req = JsonRpcRequest::notification("notify", None);
    assert_eq!(req.jsonrpc, JSONRPC_VERSION);
    assert!(req.id.is_none());
}

#[test]
fn test_metadata() {
    let req = JsonRpcRequest::new("test", None)
        .with_metadata("auth_token", "secret")
        .with_metadata("client_id", "test_client");

    assert_eq!(req.get_metadata("auth_token"), Some(&"secret".to_owned()));
    assert_eq!(
        req.get_metadata("client_id"),
        Some(&"test_client".to_owned())
    );
}

#[test]
fn test_success_response() {
    let resp = JsonRpcResponse::success(Some(Value::from(1)), Value::from("ok"));
    assert!(resp.is_success());
    assert!(!resp.is_error());
    assert_eq!(resp.jsonrpc, JSONRPC_VERSION);
}

#[test]
fn test_error_response() {
    let resp = JsonRpcResponse::error(Some(Value::from(1)), -32600, "Invalid Request");
    assert!(resp.is_error());
    assert!(!resp.is_success());
    assert!(resp.error.is_some());
}

#[test]
fn test_serialization() {
    let req = JsonRpcRequest::new("test", Some(Value::from("param")));
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"test\""));
}

#[test]
fn test_metadata_extension_pattern() {
    // Test the extension pattern for protocol-specific fields
    let req = JsonRpcRequest::new("a2a/initialize", None)
        .with_metadata("auth_token", "bearer_token_123")
        .with_metadata("client_version", "1.0.0");

    assert_eq!(
        req.get_metadata("auth_token"),
        Some(&"bearer_token_123".to_owned())
    );
    assert_eq!(
        req.get_metadata("client_version"),
        Some(&"1.0.0".to_owned())
    );
    assert_eq!(req.get_metadata("nonexistent"), None);
}

#[test]
fn test_error_with_data() {
    let error_data = serde_json::json!({
        "code": "INVALID_PARAMS",
        "details": "Missing required field"
    });

    let resp = JsonRpcResponse::error_with_data(
        Some(Value::from(1)),
        -32602,
        "Invalid params",
        error_data.clone(),
    );

    assert!(resp.is_error());
    assert!(resp.error.is_some());

    let error = resp.error.unwrap();
    assert_eq!(error.code, -32602);
    assert_eq!(error.message, "Invalid params");
    assert_eq!(error.data, Some(error_data));
}

#[test]
fn test_invalid_request_error_code() {
    // Per JSON-RPC 2.0 spec, invalid request should use error code -32600
    let resp = JsonRpcResponse::error(Some(Value::from(1)), -32600, "Invalid Request");

    assert!(resp.is_error());
    assert!(resp.error.is_some());

    let error = resp.error.unwrap();
    assert_eq!(error.code, -32600);
    assert_eq!(error.message, "Invalid Request");
}

#[test]
fn test_parse_error_code() {
    // Per JSON-RPC 2.0 spec, parse error should use error code -32700
    let resp = JsonRpcResponse::error(Some(Value::Null), -32700, "Parse error");

    assert!(resp.is_error());
    let error = resp.error.unwrap();
    assert_eq!(error.code, -32700);
}
