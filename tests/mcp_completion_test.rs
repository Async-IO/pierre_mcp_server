#![allow(clippy::unwrap_used)]
#![allow(missing_docs)]
// ABOUTME: Tests for MCP completion (auto-complete) feature
// ABOUTME: Validates completion suggestions for tool arguments and resources
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use pierre_mcp_server::mcp::protocol::{McpRequest, ProtocolHandler};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_completion_activity_type() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            "ref": {
                "type": "ref/prompt",
                "name": "analyze_activity"
            },
            "argument": {
                "name": "activity_type",
                "value": "r"
            }
        })),
        id: Some(json!(1)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(response.error.is_none(), "Should not have error");
    assert!(response.result.is_some(), "Should have result");

    let result = response.result.unwrap();
    let completion = &result["completion"];

    assert!(completion["values"].is_array(), "Should have values array");
    let values = completion["values"].as_array().unwrap();

    // Should match "run" and "ride" for prefix "r"
    assert_eq!(values.len(), 2, "Should have 2 completions for 'r'");
    assert!(values.contains(&json!("run")));
    assert!(values.contains(&json!("ride")));
}

#[test]
fn test_completion_provider() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            "ref": {
                "type": "ref/prompt",
                "name": "get_activities"
            },
            "argument": {
                "name": "provider",
                "value": "st"
            }
        })),
        id: Some(json!(2)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let values = result["completion"]["values"].as_array().unwrap();

    // Should match "strava" for prefix "st"
    assert_eq!(values.len(), 1);
    assert_eq!(values[0], "strava");
}

#[test]
fn test_completion_goal_type() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            "ref": {
                "type": "ref/prompt",
                "name": "set_goal"
            },
            "argument": {
                "name": "goal_type",
                "value": ""
            }
        })),
        id: Some(json!(3)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let values = result["completion"]["values"].as_array().unwrap();

    // Empty prefix should return all goal types
    assert_eq!(values.len(), 5);
    assert!(values.contains(&json!("distance")));
    assert!(values.contains(&json!("time")));
    assert!(values.contains(&json!("frequency")));
    assert!(values.contains(&json!("performance")));
    assert!(values.contains(&json!("custom")));
}

#[test]
fn test_completion_resource_uri() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            "ref": {
                "type": "ref/resource",
                "name": "notifications"
            },
            "argument": {
                "name": "uri",
                "value": "oauth"
            }
        })),
        id: Some(json!(4)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let values = result["completion"]["values"].as_array().unwrap();

    // Should match "oauth://notifications"
    assert_eq!(values.len(), 1);
    assert_eq!(values[0], "oauth://notifications");
}

#[test]
fn test_completion_no_matches() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            "ref": {
                "type": "ref/prompt",
                "name": "analyze_activity"
            },
            "argument": {
                "name": "activity_type",
                "value": "xyz"
            }
        })),
        id: Some(json!(5)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let values = result["completion"]["values"].as_array().unwrap();

    // No matches for "xyz"
    assert_eq!(values.len(), 0);
    assert_eq!(result["completion"]["total"], 0);
}

#[test]
fn test_completion_unknown_argument() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            "ref": {
                "type": "ref/prompt",
                "name": "some_tool"
            },
            "argument": {
                "name": "unknown_arg",
                "value": "test"
            }
        })),
        id: Some(json!(6)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(response.error.is_none());
    let result = response.result.unwrap();
    let values = result["completion"]["values"].as_array().unwrap();

    // Unknown arguments return empty list
    assert_eq!(values.len(), 0);
}

#[test]
fn test_completion_invalid_params() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: Some(json!({
            // Missing "ref" field
            "argument": {
                "name": "activity_type",
                "value": "r"
            }
        })),
        id: Some(json!(7)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(
        response.error.is_some(),
        "Should have error for invalid params"
    );
    let error = response.error.unwrap();
    assert_eq!(error.code, -32602); // Invalid params error code
}

#[test]
fn test_completion_missing_params() {
    let request = McpRequest {
        jsonrpc: "2.0".to_owned(),
        method: "completion/complete".to_owned(),
        params: None,
        id: Some(json!(8)),
        auth_token: None,
        headers: None,
        metadata: HashMap::new(),
    };

    let response = ProtocolHandler::handle_completion_complete(request);

    assert!(
        response.error.is_some(),
        "Should have error for missing params"
    );
}
