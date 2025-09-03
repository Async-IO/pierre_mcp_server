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
use crate::database_plugins::DatabaseProvider;
use crate::mcp::resources::ServerResources;
use crate::mcp::schema::{get_tools, InitializeResponse};
use crate::models::AuthRequest;
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;

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
    pub fn handle_resources_list(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // Extract user_id from auth context if available
        let user_id = request.auth_token.as_ref().and_then(|auth_token| {
            match resources.auth_manager.validate_token(auth_token) {
                Ok(claims) => {
                    if let Ok(id) = Uuid::parse_str(&claims.sub) {
                        Some(id)
                    } else {
                        error!("Invalid user ID in token: {}", claims.sub);
                        None
                    }
                }
                Err(_) => None,
            }
        });

        let mut resource_list = Vec::new();

        // Add OAuth notifications resource if user is authenticated
        if user_id.is_some() {
            resource_list.push(serde_json::json!({
                "uri": "oauth://notifications",
                "name": "OAuth Notifications",
                "description": "Real-time notifications for OAuth connection status and completion events",
                "mimeType": "application/json"
            }));
        }

        McpResponse::success(
            request_id,
            serde_json::json!({ "resources": resource_list }),
        )
    }

    /// Handle resources read request
    pub async fn handle_resources_read(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // Extract user_id from auth context
        let user_id = if let Some(auth_token) = &request.auth_token {
            match resources.auth_manager.validate_token(auth_token) {
                Ok(claims) => {
                    if let Ok(id) = Uuid::parse_str(&claims.sub) {
                        id
                    } else {
                        error!("Invalid user ID in token: {}", claims.sub);
                        return McpResponse::error(
                            request_id,
                            ERROR_INVALID_PARAMS,
                            "Invalid user ID in token".to_string(),
                        );
                    }
                }
                Err(e) => {
                    error!("Authentication failed: {}", e);
                    return McpResponse::error(
                        request_id,
                        ERROR_INVALID_PARAMS,
                        "Authentication required".to_string(),
                    );
                }
            }
        } else {
            return McpResponse::error(
                request_id,
                ERROR_INVALID_PARAMS,
                "Authentication token required".to_string(),
            );
        };

        // Extract resource URI from params
        let uri = if let Some(params) = &request.params {
            if let Some(uri_value) = params.get("uri") {
                uri_value.as_str().unwrap_or_default()
            } else {
                return McpResponse::error(
                    request_id,
                    ERROR_INVALID_PARAMS,
                    "Missing uri parameter".to_string(),
                );
            }
        } else {
            return McpResponse::error(
                request_id,
                ERROR_INVALID_PARAMS,
                "Missing parameters".to_string(),
            );
        };

        match uri {
            "oauth://notifications" => {
                // Get unread notifications
                match resources
                    .database
                    .get_unread_oauth_notifications(user_id)
                    .await
                {
                    Ok(notifications) => {
                        let response_data = serde_json::json!({
                            "contents": [{
                                "uri": "oauth://notifications",
                                "mimeType": "application/json",
                                "text": serde_json::to_string_pretty(&notifications).unwrap_or_else(|_| "[]".to_string())
                            }]
                        });
                        McpResponse::success(request_id, response_data)
                    }
                    Err(e) => {
                        error!("Failed to fetch OAuth notifications: {}", e);
                        McpResponse::error(
                            request_id,
                            -32603,
                            "Failed to fetch notifications".to_string(),
                        )
                    }
                }
            }
            _ => McpResponse::error(
                request_id,
                ERROR_METHOD_NOT_FOUND,
                format!("Unknown resource URI: {uri}"),
            ),
        }
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
