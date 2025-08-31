// ABOUTME: MCP protocol message handlers for core protocol operations
// ABOUTME: Handles initialize, ping, tools/list, and authentication protocol messages
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # MCP Protocol Handlers
//!
//! Core MCP protocol message handling for initialization, tools listing,
//! and authentication operations.

use crate::auth::AuthManager;
use crate::constants::{
    errors::{ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND},
    protocol::SERVER_VERSION,
};
use crate::mcp::schema::{get_tools, InitializeResponse};
use crate::models::AuthRequest;
use serde_json::Value;
use std::sync::Arc;
use tracing::info;

/// MCP protocol handlers
pub struct ProtocolHandler;

// Re-export types from multitenant module to avoid duplication
pub use super::multitenant::{McpError, McpRequest, McpResponse};

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    serde_json::Value::Number(serde_json::Number::from(0))
}

impl ProtocolHandler {
    /// Handle initialize request
    pub fn handle_initialize(request: McpRequest) -> McpResponse {
        let init_response = InitializeResponse::new(
            crate::constants::protocol::mcp_protocol_version(),
            crate::constants::protocol::server_name_multitenant(),
            SERVER_VERSION.to_string(),
        );

        let request_id = request.id.unwrap_or_else(default_request_id);
        match serde_json::to_value(&init_response) {
            Ok(result) => McpResponse::success(request_id, result),
            Err(_) => McpResponse::error(request_id, -32603, "Internal error".to_string()),
        }
    }

    /// Handle ping request
    pub fn handle_ping(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(request_id, serde_json::json!({}))
    }

    /// Handle tools list request
    pub fn handle_tools_list(request: McpRequest) -> McpResponse {
        let tools = get_tools();

        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(request_id, serde_json::json!({ "tools": tools }))
    }

    /// Handle prompts list request
    pub fn handle_prompts_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(request_id, serde_json::json!({ "prompts": [] }))
    }

    /// Handle resources list request
    pub fn handle_resources_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(request_id, serde_json::json!({ "resources": [] }))
    }

    /// Handle unknown method request
    pub fn handle_unknown_method(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::error(
            request_id,
            ERROR_METHOD_NOT_FOUND,
            format!("Unknown method: {}", request.method),
        )
    }

    /// Handle authenticate request
    pub fn handle_authenticate(
        request: McpRequest,
        auth_manager: &Arc<AuthManager>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        let auth_request: AuthRequest =
            match request.params.and_then(|p| serde_json::from_value(p).ok()) {
                Some(req) => req,
                None => {
                    return McpResponse::error(
                        request_id,
                        ERROR_INVALID_PARAMS,
                        "Invalid authentication parameters".to_string(),
                    );
                }
            };

        let auth_response = auth_manager.authenticate(&auth_request);
        if auth_response.authenticated {
            info!("MCP authentication successful");
            McpResponse::success(request_id, serde_json::json!({ "authenticated": true }))
        } else {
            let error_msg = auth_response
                .error
                .as_deref()
                .unwrap_or("Authentication failed");
            info!("MCP authentication failed: {}", error_msg);
            McpResponse::error(
                request_id,
                ERROR_INVALID_PARAMS,
                format!("Authentication failed: {error_msg}"),
            )
        }
    }
}
