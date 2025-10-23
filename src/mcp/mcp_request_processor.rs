// ABOUTME: MCP request processing and protocol handling for multi-tenant server
// ABOUTME: Validates, routes, and executes MCP protocol requests with proper error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Request/response ownership transfers across async boundaries
// - Resource Arc sharing for concurrent request processing
// - JSON value ownership for MCP protocol serialization

use super::{
    multitenant::{McpError, McpRequest, McpResponse},
    resources::ServerResources,
    tool_handlers::ToolHandlers,
};
use crate::constants::errors::{ERROR_INTERNAL_ERROR, ERROR_METHOD_NOT_FOUND};
use crate::constants::protocol::JSONRPC_VERSION;
use crate::errors::AppError;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Processes MCP protocol requests with validation, routing, and execution
pub struct McpRequestProcessor {
    resources: Arc<ServerResources>,
}

impl McpRequestProcessor {
    /// Create a new MCP request processor
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Handle an MCP request and return a response
    pub async fn handle_request(&self, request: McpRequest) -> Option<McpResponse> {
        let start_time = std::time::Instant::now();

        // Log request with optional truncation
        Self::log_request(&request);

        // Handle notifications (no response needed)
        if request.method.starts_with("notifications/") {
            Self::handle_notification(&request);
            Self::log_completion("notification", start_time);
            return None;
        }

        // Process request and generate response
        let response = match self.process_request(request.clone()).await {
            Ok(response) => response,
            Err(e) => {
                error!(
                    "Failed to process MCP request: {} | Request: method={}, jsonrpc={}, id={:?}",
                    e, request.method, request.jsonrpc, request.id
                );
                error!("Request params: {:?}", request.params);
                error!("Full error details: {:#}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    id: request.id.clone(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: format!("Internal server error: {e}"),
                        data: None,
                    }),
                }
            }
        };

        Self::log_completion("request", start_time);
        Some(response)
    }

    /// Process an MCP request and generate response
    async fn process_request(&self, request: McpRequest) -> Result<McpResponse> {
        // Validate request format
        Self::validate_request(&request)?;

        // Route to appropriate handler based on method
        match request.method.as_str() {
            "initialize" => Ok(Self::handle_initialize(&request)),
            "ping" => Ok(Self::handle_ping(&request)),
            "tools/list" => Ok(Self::handle_tools_list(&request)),
            "tools/call" => self.handle_tools_call(&request).await,
            "authenticate" => Ok(Self::handle_authenticate(&request)),
            method if method.starts_with("resources/") => Ok(Self::handle_resources(&request)),
            method if method.starts_with("prompts/") => Ok(Self::handle_prompts(&request)),
            _ => Ok(Self::handle_unknown_method(&request)),
        }
    }

    /// Validate MCP request format and required fields
    fn validate_request(request: &McpRequest) -> Result<()> {
        if request.jsonrpc != JSONRPC_VERSION {
            return Err(AppError::invalid_input(format!(
                "Invalid JSON-RPC version: got '{}', expected '{}'",
                request.jsonrpc, JSONRPC_VERSION
            ))
            .into());
        }

        if request.method.is_empty() {
            return Err(AppError::invalid_input("Missing method").into());
        }

        Ok(())
    }

    /// Handle MCP initialize request
    fn handle_initialize(request: &McpRequest) -> McpResponse {
        debug!("Handling initialize request");

        let server_info = serde_json::json!({
            "protocolVersion": crate::constants::protocol::mcp_protocol_version(),
            "capabilities": {
                "tools": {
                    "listChanged": true
                },
                "resources": {
                    "subscribe": true,
                    "listChanged": true
                },
                "prompts": {
                    "listChanged": true
                }
            },
            "serverInfo": {
                "name": "pierre-mcp-server",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: Some(server_info),
            error: None,
        }
    }

    /// Handle MCP ping request
    fn handle_ping(request: &McpRequest) -> McpResponse {
        debug!("Handling ping request");

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: Some(serde_json::json!({})),
            error: None,
        }
    }

    /// Handle MCP authenticate request
    fn handle_authenticate(request: &McpRequest) -> McpResponse {
        debug!("Handling authenticate request");

        // Always return authentication parameter error for authenticate method
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: None,
            error: Some(McpError {
                code: -32602, // Invalid params
                message: "Invalid authentication parameters".to_string(),
                data: None,
            }),
        }
    }

    /// Handle tools/list request
    fn handle_tools_list(request: &McpRequest) -> McpResponse {
        debug!("Handling tools/list request");

        // Get all available tools from schema
        let tools = crate::mcp::schema::get_tools();

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: Some(serde_json::json!({ "tools": tools })),
            error: None,
        }
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, request: &McpRequest) -> Result<McpResponse> {
        debug!("Handling tools/call request");

        request
            .params
            .as_ref()
            .ok_or_else(|| AppError::invalid_input("Missing parameters for tools/call"))?;

        // Execute tool using static method - delegate to ToolHandlers
        let handler_request = McpRequest {
            jsonrpc: request.jsonrpc.clone(),
            method: request.method.clone(),
            params: request.params.clone(),
            id: request.id.clone(),
            auth_token: request.auth_token.clone(),
            headers: request.headers.clone(),
            metadata: HashMap::new(),
        };
        let response =
            ToolHandlers::handle_tools_call_with_resources(handler_request, &self.resources).await;
        Ok(response)
    }

    /// Handle resources requests
    fn handle_resources(request: &McpRequest) -> McpResponse {
        debug!("Handling resources request: {}", request.method);

        // Return empty resources list for now
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: Some(serde_json::json!({ "resources": [] })),
            error: None,
        }
    }

    /// Handle prompts requests
    fn handle_prompts(request: &McpRequest) -> McpResponse {
        debug!("Handling prompts request: {}", request.method);

        // Return empty prompts list for now
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: Some(serde_json::json!({ "prompts": [] })),
            error: None,
        }
    }

    /// Handle unknown method
    fn handle_unknown_method(request: &McpRequest) -> McpResponse {
        warn!("Unknown MCP method: {}", request.method);

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: request.id.clone(),
            result: None,
            error: Some(McpError {
                code: ERROR_METHOD_NOT_FOUND,
                message: format!("Unknown method: {}", request.method),
                data: None,
            }),
        }
    }

    /// Handle notification (no response required)
    fn handle_notification(request: &McpRequest) {
        debug!("Handling notification: {}", request.method);

        match request.method.as_str() {
            "notifications/cancelled" => Self::handle_cancelled_notification(),
            "notifications/progress" => Self::handle_progress_notification(),
            _ => Self::handle_unknown_notification(&request.method),
        }
    }

    /// Handle cancelled notification
    fn handle_cancelled_notification() {
        debug!("Request cancelled notification received");
    }

    /// Handle progress notification
    fn handle_progress_notification() {
        debug!("Progress notification received");
    }

    /// Handle unknown notification type
    fn handle_unknown_notification(method: &str) {
        debug!("Unknown notification type: {}", method);
    }

    /// Log incoming request with optional truncation
    fn log_request(request: &McpRequest) {
        let should_truncate = true; // MCP request logging is always truncated for security

        if should_truncate {
            let request_summary = format!("{}(id={:?})", request.method, request.id);
            debug!(
                mcp_method = %request.method,
                mcp_id = ?request.id,
                mcp_params_preview = ?request.params.as_ref().map(|p| {
                    let s = p.to_string();
                    if s.len() > 100 {
                        format!("{}...[truncated]", &s[..100])
                    } else {
                        s
                    }
                }),
                auth_present = request.auth_token.is_some(),
                "Received MCP request: {}",
                request_summary
            );
        } else {
            debug!(
                mcp_request = ?request,
                "Received MCP request (full)"
            );
        }
    }

    /// Log request completion with timing
    fn log_completion(request_type: &str, start_time: std::time::Instant) {
        let duration = start_time.elapsed();
        debug!(
            duration_ms = u64::try_from(duration.as_millis()).unwrap_or(0),
            "Completed MCP {} processing", request_type
        );
    }
}

/// Write MCP response to stdout for stdio transport
///
/// # Errors
/// Returns an error if JSON serialization fails or I/O operations fail
pub async fn write_response_to_stdout(
    response: &McpResponse,
    stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
) -> Result<()> {
    use tokio::io::AsyncWriteExt;

    let response_json = serde_json::to_string(response)?;
    debug!("Sending MCP response: {}", response_json);

    {
        let mut stdout_lock = stdout.lock().await;
        stdout_lock.write_all(response_json.as_bytes()).await?;
        stdout_lock.write_all(b"\n").await?;
        stdout_lock.flush().await?;
        drop(stdout_lock);
    }

    Ok(())
}
