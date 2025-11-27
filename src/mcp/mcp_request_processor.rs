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
use crate::errors::{AppError, AppResult};
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
            self.handle_notification(&request).await;
            Self::log_completion("notification", start_time);
            return None;
        }

        // Process request and generate response
        let response = self.process_or_error(request).await;

        Self::log_completion("request", start_time);
        Some(response)
    }

    async fn process_or_error(&self, request: McpRequest) -> McpResponse {
        match self.process_request(request.clone()).await {
            Ok(response) => response,
            Err(e) => Self::create_error_response(&request, &e),
        }
    }

    fn create_error_response(request: &McpRequest, e: &crate::errors::AppError) -> McpResponse {
        error!(
            "Failed to process MCP request: {} | Request: method={}, jsonrpc={}, id={:?}",
            e, request.method, request.jsonrpc, request.id
        );
        error!("Request params: {:?}", request.params);
        error!("Full error details: {:#}", e);

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: None,
            error: Some(McpError {
                code: ERROR_INTERNAL_ERROR,
                message: format!("Internal server error: {e}"),
                data: None,
            }),
        }
    }

    /// Process an MCP request and generate response
    async fn process_request(&self, request: McpRequest) -> AppResult<McpResponse> {
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
            method if method.starts_with("sampling/") => self.handle_sampling(&request).await,
            method if method.starts_with("completion/") => Ok(Self::handle_completion(&request)),
            method if method.starts_with("roots/") => Ok(Self::handle_roots(&request)),
            _ => Ok(Self::handle_unknown_method(&request)),
        }
    }

    /// Validate MCP request format and required fields
    fn validate_request(request: &McpRequest) -> AppResult<()> {
        if request.jsonrpc != JSONRPC_VERSION {
            return Err(AppError::invalid_input(format!(
                "Invalid JSON-RPC version: got '{}', expected '{}'",
                request.jsonrpc, JSONRPC_VERSION
            )));
        }

        if request.method.is_empty() {
            return Err(AppError::invalid_input("Missing method"));
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
                },
                "sampling": {}
            },
            "serverInfo": {
                "name": "pierre-mcp-server",
                "version": env!("CARGO_PKG_VERSION")
            }
        });

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: Some(server_info),
            error: None,
        }
    }

    /// Handle MCP ping request
    fn handle_ping(request: &McpRequest) -> McpResponse {
        debug!("Handling ping request");

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
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
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: None,
            error: Some(McpError {
                code: -32602, // Invalid params
                message: "Invalid authentication parameters".to_owned(),
                data: None,
            }),
        }
    }

    /// Handle tools/list request
    ///
    /// Per MCP specification, tools/list does NOT require authentication.
    /// All tools are returned regardless of authentication status.
    /// Individual tool calls will check authentication and trigger OAuth if needed.
    fn handle_tools_list(request: &McpRequest) -> McpResponse {
        debug!("Handling tools/list request");

        // Get all available tools from schema
        // MCP spec: tools/list must work without authentication
        // Authentication is checked at tools/call time, not discovery time
        let tools = crate::mcp::schema::get_tools();

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: Some(serde_json::json!({ "tools": tools })),
            error: None,
        }
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, request: &McpRequest) -> AppResult<McpResponse> {
        debug!("Handling tools/call request");

        request
            .params
            .as_ref()
            .ok_or_else(|| AppError::invalid_input("Missing parameters for tools/call"))?;

        // Execute tool using static method - delegate to ToolHandlers
        // Clone the entire request and reset metadata
        let mut handler_request = request.clone();
        handler_request.metadata = HashMap::new();
        let response =
            ToolHandlers::handle_tools_call_with_resources(handler_request, &self.resources).await;
        Ok(response)
    }

    /// Handle resources requests
    fn handle_resources(request: &McpRequest) -> McpResponse {
        debug!("Handling resources request: {}", request.method);

        // Return empty resources list for now
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
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
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: Some(serde_json::json!({ "prompts": [] })),
            error: None,
        }
    }

    /// Handle completion requests
    fn handle_completion(request: &McpRequest) -> McpResponse {
        debug!("Handling completion request: {}", request.method);

        match request.method.as_str() {
            "completion/complete" => {
                // Delegate to protocol handler
                crate::mcp::protocol::ProtocolHandler::handle_completion_complete(request.clone())
            }
            _ => Self::handle_unknown_method(request),
        }
    }

    /// Handle roots requests
    fn handle_roots(request: &McpRequest) -> McpResponse {
        debug!("Handling roots request: {}", request.method);

        match request.method.as_str() {
            "roots/list" => {
                // Delegate to protocol handler
                crate::mcp::protocol::ProtocolHandler::handle_roots_list(request.clone())
            }
            _ => Self::handle_unknown_method(request),
        }
    }

    /// Handle sampling requests (server-initiated LLM calls)
    async fn handle_sampling(&self, request: &McpRequest) -> Result<McpResponse, AppError> {
        debug!("Handling sampling request: {}", request.method);

        match request.method.as_str() {
            "sampling/createMessage" => {
                // Check if sampling peer is available (only for stdio transport)
                let Some(sampling_peer) = &self.resources.sampling_peer else {
                    return Ok(McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        id: request.id.clone(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_METHOD_NOT_FOUND,
                            message: "Sampling not available (stdio transport only)".to_owned(),
                            data: None,
                        }),
                    });
                };

                // Parse request parameters
                let create_message_request = match &request.params {
                    Some(params) => match serde_json::from_value::<
                        crate::mcp::schema::CreateMessageRequest,
                    >(params.clone())
                    {
                        Ok(req) => req,
                        Err(e) => {
                            return Ok(McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_owned(),
                                id: request.id.clone(),
                                result: None,
                                error: Some(McpError {
                                    code: -32602, // Invalid params
                                    message: format!("Invalid sampling parameters: {e}"),
                                    data: None,
                                }),
                            });
                        }
                    },
                    None => {
                        return Ok(McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_owned(),
                            id: request.id.clone(),
                            result: None,
                            error: Some(McpError {
                                code: -32602,
                                message: "Missing sampling parameters".to_owned(),
                                data: None,
                            }),
                        });
                    }
                };

                // Send sampling request to client and await response
                match sampling_peer.create_message(create_message_request).await {
                    Ok(result) => match serde_json::to_value(&result) {
                        Ok(result_value) => Ok(McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_owned(),
                            id: request.id.clone(),
                            result: Some(result_value),
                            error: None,
                        }),
                        Err(e) => Ok(McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_owned(),
                            id: request.id.clone(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to serialize sampling result: {e}"),
                                data: None,
                            }),
                        }),
                    },
                    Err(e) => {
                        warn!("Sampling request failed: {e}");
                        Ok(McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_owned(),
                            id: request.id.clone(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Sampling failed: {e}"),
                                data: None,
                            }),
                        })
                    }
                }
            }
            _ => Ok(Self::handle_unknown_method(request)),
        }
    }

    /// Handle unknown method
    fn handle_unknown_method(request: &McpRequest) -> McpResponse {
        warn!("Unknown MCP method: {}", request.method);

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
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
    async fn handle_notification(&self, request: &McpRequest) {
        debug!("Handling notification: {}", request.method);

        match request.method.as_str() {
            "notifications/cancelled" => self.handle_cancelled_notification(request).await,
            "notifications/progress" => Self::handle_progress_notification(),
            _ => Self::handle_unknown_notification(&request.method),
        }
    }

    /// Handle cancelled notification
    async fn handle_cancelled_notification(&self, request: &McpRequest) {
        debug!("Request cancelled notification received");

        // Extract progress token from params
        if let Some(params) = &request.params {
            if let Some(progress_token) = params.get("progressToken").and_then(|v| v.as_str()) {
                self.resources
                    .cancel_by_progress_token(progress_token)
                    .await;
            } else {
                warn!("Cancelled notification missing progressToken parameter");
            }
        } else {
            warn!("Cancelled notification missing params");
        }
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
) -> AppResult<()> {
    use tokio::io::AsyncWriteExt;

    let response_json = serde_json::to_string(response)
        .map_err(|e| AppError::internal(format!("JSON serialization failed: {e}")))?;
    debug!("Sending MCP response: {}", response_json);

    {
        let mut stdout_lock = stdout.lock().await;
        stdout_lock
            .write_all(response_json.as_bytes())
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        stdout_lock
            .write_all(b"\n")
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        stdout_lock
            .flush()
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        drop(stdout_lock);
    }

    Ok(())
}
