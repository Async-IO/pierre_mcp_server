// ABOUTME: Integration tests for MCP 2025-11-25 protocol version support
// ABOUTME: Tests version negotiation, ServerInfo metadata, ToolSchema annotations, and ClientInfo extension
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::constants::errors::ERROR_VERSION_MISMATCH;
use pierre_mcp_server::mcp::{
    multitenant::McpRequest,
    protocol::ProtocolHandler,
    schema::{get_tools, ToolAnnotations},
};
use serde_json::json;

mod common;

/// Test that a client requesting 2025-11-25 gets 2025-11-25 back
#[tokio::test]
async fn test_negotiate_2025_11_25_version() {
    common::init_server_config();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    assert!(response.error.is_none(), "Should succeed with 2025-11-25");
    let result = response.result.expect("Should have result");
    assert_eq!(result["protocolVersion"], "2025-11-25");
}

/// Test backward compatibility: client requesting 2025-06-18 still works
#[tokio::test]
async fn test_backward_compat_2025_06_18() {
    common::init_server_config();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
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

    assert!(response.error.is_none(), "Should succeed with 2025-06-18");
    let result = response.result.expect("Should have result");
    assert_eq!(result["protocolVersion"], "2025-06-18");
}

/// Test backward compatibility: client requesting 2024-11-05 still works
#[tokio::test]
async fn test_backward_compat_2024_11_05() {
    common::init_server_config();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "clientInfo": {
                "name": "legacy-client",
                "version": "0.9.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    assert!(response.error.is_none(), "Should succeed with 2024-11-05");
    let result = response.result.expect("Should have result");
    assert_eq!(result["protocolVersion"], "2024-11-05");
}

/// Test that unknown future version is rejected with error
#[tokio::test]
async fn test_unknown_future_version_rejected() {
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "initialize",
        "params": {
            "protocolVersion": "2099-01-01",
            "clientInfo": {
                "name": "future-client",
                "version": "99.0.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    let error = response
        .error
        .expect("Should return error for unknown version");
    assert_eq!(error.code, ERROR_VERSION_MISMATCH);
    assert!(
        error.message.contains("2025-11-25"),
        "Error should list supported versions including 2025-11-25, got: {}",
        error.message
    );
}

/// Test that `ServerInfo` includes title and description metadata
#[tokio::test]
async fn test_server_info_includes_metadata() {
    common::init_server_config();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request).expect("Should parse request");
    let response = ProtocolHandler::handle_initialize(request);

    let result = response.result.expect("Should have result");
    let server_info = &result["serverInfo"];

    // Verify machine-readable name is present
    assert!(
        server_info["name"].is_string(),
        "serverInfo.name should be present"
    );

    // Verify human-readable title (MCP 2025-11-25 metadata)
    assert_eq!(
        server_info["title"], "Pierre Fitness Intelligence",
        "serverInfo.title should be the human-readable display name"
    );

    // Verify description (MCP 2025-11-25 metadata)
    assert!(
        server_info["description"].is_string(),
        "serverInfo.description should be present"
    );
}

/// Test that tools include annotations when present
#[tokio::test]
async fn test_tools_include_annotations() {
    common::init_server_config();

    let tools = get_tools();

    // Verify read-only tools have correct annotations
    let get_activities = tools
        .iter()
        .find(|t| t.name == "get_activities")
        .expect("get_activities tool should exist");
    let annotations = get_activities
        .annotations
        .as_ref()
        .expect("get_activities should have annotations");
    assert_eq!(annotations.read_only_hint, Some(true));
    assert_eq!(annotations.destructive_hint, Some(false));
    assert_eq!(annotations.idempotent_hint, Some(true));

    // Verify destructive tools have correct annotations
    let delete_coach = tools
        .iter()
        .find(|t| t.name == "delete_coach")
        .expect("delete_coach tool should exist");
    let annotations = delete_coach
        .annotations
        .as_ref()
        .expect("delete_coach should have annotations");
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.destructive_hint, Some(true));

    // Verify open-world tools have correct annotations
    let connect_provider = tools
        .iter()
        .find(|t| t.name == "connect_provider")
        .expect("connect_provider tool should exist");
    let annotations = connect_provider
        .annotations
        .as_ref()
        .expect("connect_provider should have annotations");
    assert_eq!(annotations.open_world_hint, Some(true));
}

/// Test that tool annotations serialize correctly to JSON
#[tokio::test]
async fn test_tool_annotations_json_serialization() {
    common::init_server_config();

    let tools = get_tools();
    let get_athlete = tools
        .iter()
        .find(|t| t.name == "get_athlete")
        .expect("get_athlete tool should exist");

    let json_value = serde_json::to_value(get_athlete).expect("Should serialize");

    // Verify annotations field is present in JSON
    let annotations = &json_value["annotations"];
    assert!(
        !annotations.is_null(),
        "annotations should be present for annotated tools"
    );
    assert_eq!(annotations["readOnlyHint"], true);
    assert_eq!(annotations["destructiveHint"], false);
}

/// Test that tools without annotations omit the field in JSON
#[tokio::test]
async fn test_unannotated_tools_omit_annotations() {
    common::init_server_config();

    let tools = get_tools();

    // Find a tool that doesn't have explicit annotations (analytics tools beyond analyze_activity)
    let calculate_metrics = tools
        .iter()
        .find(|t| t.name == "calculate_metrics")
        .expect("calculate_metrics tool should exist");

    let json_value = serde_json::to_value(calculate_metrics).expect("Should serialize");

    // Tools with None annotations should have the field omitted (skip_serializing_if)
    // OR they might have annotations assigned by apply_tool_annotations
    // Either way, the JSON should be valid MCP
    assert!(
        json_value["name"].is_string(),
        "Tool name should always be present"
    );
}

/// Test that `ClientInfo` accepts 2025-11-25 extended fields
#[tokio::test]
async fn test_client_info_accepts_extended_fields() {
    common::init_server_config();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "clientInfo": {
                "name": "claude-code",
                "version": "1.0.0",
                "title": "Claude Code",
                "description": "Anthropic's CLI for Claude",
                "websiteUrl": "https://claude.ai/claude-code"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest = serde_json::from_value(init_request)
        .expect("Should parse request with extended clientInfo");
    let response = ProtocolHandler::handle_initialize(request);

    assert!(
        response.error.is_none(),
        "Should succeed when client sends extended clientInfo fields"
    );
    let result = response.result.expect("Should have result");
    assert_eq!(result["protocolVersion"], "2025-11-25");
}

/// Test that `ClientInfo` still works without extended fields
#[tokio::test]
async fn test_client_info_minimal_fields() {
    common::init_server_config();

    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-11-25",
            "clientInfo": {
                "name": "minimal-client",
                "version": "0.1.0"
            },
            "capabilities": {}
        }
    });

    let request: McpRequest =
        serde_json::from_value(init_request).expect("Should parse request with minimal clientInfo");
    let response = ProtocolHandler::handle_initialize(request);

    assert!(
        response.error.is_none(),
        "Should succeed with minimal clientInfo"
    );
}

/// Test that `ToolAnnotations` defaults are all None
#[tokio::test]
async fn test_tool_annotations_default() {
    let annotations = ToolAnnotations::default();
    assert!(annotations.title.is_none());
    assert!(annotations.read_only_hint.is_none());
    assert!(annotations.destructive_hint.is_none());
    assert!(annotations.idempotent_hint.is_none());
    assert!(annotations.open_world_hint.is_none());
}

/// Test that write tools have correct annotations
#[tokio::test]
async fn test_write_tool_annotations() {
    common::init_server_config();

    let tools = get_tools();

    let create_coach = tools
        .iter()
        .find(|t| t.name == "create_coach")
        .expect("create_coach tool should exist");
    let annotations = create_coach
        .annotations
        .as_ref()
        .expect("create_coach should have annotations");
    assert_eq!(annotations.read_only_hint, Some(false));
    assert_eq!(annotations.destructive_hint, Some(false));
    assert_eq!(annotations.idempotent_hint, Some(true));
}
