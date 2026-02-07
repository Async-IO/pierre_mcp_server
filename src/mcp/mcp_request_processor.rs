// ABOUTME: MCP request processing and protocol handling for multi-tenant server
// ABOUTME: Validates, routes, and executes MCP protocol requests with proper error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Request/response ownership transfers across async boundaries
// - Resource Arc sharing for concurrent request processing
// - JSON value ownership for MCP protocol serialization

use super::{
    multitenant::{McpError, McpRequest, McpResponse},
    protocol::ProtocolHandler,
    resources::ServerResources,
    schema::{CreateMessageRequest, ToolSchema},
    tenant_isolation::extract_tenant_context_internal,
    tool_handlers::ToolHandlers,
};
use crate::constants::errors::{ERROR_INTERNAL_ERROR, ERROR_METHOD_NOT_FOUND};
use crate::constants::protocol::{mcp_protocol_version, JSONRPC_VERSION};
use crate::constants::tools::PUBLIC_DISCOVERY_TOOLS;
use crate::errors::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncWriteExt, Stdout};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

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
        let start_time = Instant::now();

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

    fn create_error_response(request: &McpRequest, e: &AppError) -> McpResponse {
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
            "tools/list" => Ok(self.handle_tools_list(&request).await),
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
            "protocolVersion": mcp_protocol_version(),
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

    /// Handle tools/list request with tiered visibility based on authentication state
    ///
    /// Tool discovery is filtered by authentication context:
    /// - **Unauthenticated**: Returns only public discovery tools (safe, read-only capabilities)
    /// - **Authenticated + tenant**: Returns tenant-filtered tools via `ToolSelectionService`
    /// - **Authenticated + admin**: Returns all tools including admin tools
    ///
    /// This ensures sensitive tools (connection management, admin operations, future social tools)
    /// are not exposed to unauthenticated MCP clients while still allowing capability discovery.
    async fn handle_tools_list(&self, request: &McpRequest) -> McpResponse {
        debug!("Handling tools/list request");

        let tools = self.resolve_tools_for_request(request).await;

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: request.id.clone(),
            result: Some(serde_json::json!({ "tools": tools })),
            error: None,
        }
    }

    /// Resolve which tools to return based on authentication state in the request
    async fn resolve_tools_for_request(&self, request: &McpRequest) -> Vec<ToolSchema> {
        // Extract auth token (same pattern as tools/call in tool_handlers.rs)
        let auth_token_string = request
            .params
            .as_ref()
            .and_then(|params| params.get("token"))
            .and_then(|token| token.as_str())
            .map(|mcp_token| format!("Bearer {mcp_token}"));

        let auth_token = request
            .auth_token
            .as_deref()
            .or(auth_token_string.as_deref());

        let Some(token) = auth_token else {
            debug!("tools/list: no auth token, returning public discovery tools");
            return self.public_discovery_tools();
        };

        match self
            .resources
            .auth_middleware
            .authenticate_request(Some(token))
            .await
        {
            Ok(auth_result) => {
                info!(
                    "tools/list: authenticated user {} (method: {})",
                    auth_result.user_id,
                    auth_result.auth_method.display_name()
                );
                self.resolve_tools_for_authenticated_user(
                    auth_result.user_id,
                    auth_result.active_tenant_id,
                )
                .await
            }
            Err(e) => {
                debug!(
                    "tools/list: auth failed ({}), returning public discovery tools",
                    e
                );
                self.public_discovery_tools()
            }
        }
    }

    /// Resolve tools for an authenticated user based on their tenant context
    async fn resolve_tools_for_authenticated_user(
        &self,
        user_id: uuid::Uuid,
        active_tenant_id: Option<uuid::Uuid>,
    ) -> Vec<ToolSchema> {
        if let Ok(Some(tenant_ctx)) = extract_tenant_context_internal(
            &self.resources.database,
            Some(user_id),
            active_tenant_id,
            None,
        )
        .await
        {
            if tenant_ctx.is_admin() {
                debug!(
                    "tools/list: admin user in tenant {}, returning all tools",
                    tenant_ctx.tenant_id
                );
                return self.resources.tool_registry.all_schemas();
            }

            debug!(
                "tools/list: user in tenant {}, returning tenant-filtered tools",
                tenant_ctx.tenant_id
            );
            return self.tenant_filtered_tools(tenant_ctx.tenant_id).await;
        }

        // Authenticated but no tenant context: return user-visible tools
        // (non-admin tools from registry, no tenant filtering)
        debug!("tools/list: authenticated user without tenant, returning user-visible tools");
        self.resources.tool_registry.user_visible_schemas()
    }

    /// Return public discovery tools (safe subset for unauthenticated clients)
    fn public_discovery_tools(&self) -> Vec<ToolSchema> {
        self.resources
            .tool_registry
            .list_schemas_by_names(PUBLIC_DISCOVERY_TOOLS)
    }

    /// Get tenant-filtered tool schemas for non-admin users
    ///
    /// Combines two sources to build the tool list:
    /// 1. Enabled tools from `ToolSelectionService` (catalog-based, tenant-aware)
    /// 2. Uncatalogued tools from the registry (feature-flag tools like coaches/mobility)
    ///
    /// Admin-only tools are excluded in both paths to prevent non-admin users
    /// from seeing them even if they appear in the catalog.
    async fn tenant_filtered_tools(&self, tenant_id: uuid::Uuid) -> Vec<ToolSchema> {
        match self
            .resources
            .tool_selection
            .get_effective_tools(tenant_id)
            .await
        {
            Ok(all_effective_tools) => {
                // Separate enabled tool names from full catalog for uncatalogued detection
                let enabled_names: Vec<String> = all_effective_tools
                    .iter()
                    .filter(|t| t.is_enabled)
                    .map(|t| t.tool_name.clone())
                    .collect();
                let all_catalogued_names: Vec<String> = all_effective_tools
                    .into_iter()
                    .map(|t| t.tool_name)
                    .collect();

                // Get catalog-based enabled tools, filtered to non-admin only
                let mut schemas = self
                    .resources
                    .tool_registry
                    .list_schemas_by_name_set(&enabled_names);
                schemas.retain(|s| {
                    self.resources
                        .tool_registry
                        .get(&s.name)
                        .is_none_or(|tool| !tool.capabilities().is_admin_only())
                });

                // Include feature-flag tools not tracked by tool_catalog (coaches, mobility, etc.)
                // Uses all_catalogued_names so disabled-in-catalog tools aren't re-added
                let uncatalogued = self
                    .resources
                    .tool_registry
                    .uncatalogued_user_schemas(&all_catalogued_names);
                schemas.extend(uncatalogued);

                schemas
            }
            Err(e) => {
                warn!(
                    "tools/list: failed to get tenant tools for {}: {}, falling back to user-visible",
                    tenant_id, e
                );
                self.resources.tool_registry.user_visible_schemas()
            }
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
                ProtocolHandler::handle_completion_complete(request.clone())
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
                ProtocolHandler::handle_roots_list(request.clone())
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
                    Some(params) => {
                        match serde_json::from_value::<CreateMessageRequest>(params.clone()) {
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
                        }
                    }
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
            "notifications/initialized" => Self::handle_initialized_notification(),
            "notifications/roots/listChanged" => Self::handle_roots_list_changed_notification(),
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

    /// Handle initialized notification (client confirms initialization complete)
    fn handle_initialized_notification() {
        debug!("Client initialization complete");
    }

    /// Handle roots/listChanged notification (client's workspace roots changed)
    fn handle_roots_list_changed_notification() {
        debug!("Client roots list changed");
    }

    /// Handle unknown notification type
    fn handle_unknown_notification(method: &str) {
        warn!(
            notification_type = %method,
            "Received unhandled notification type - may need implementation"
        );
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
    fn log_completion(request_type: &str, start_time: Instant) {
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
    stdout: &Arc<Mutex<Stdout>>,
) -> AppResult<()> {
    let response_json = serde_json::to_string(response)
        .map_err(|e| AppError::internal(format!("JSON serialization failed: {e}")))?;
    debug!("Sending MCP response (size: {} bytes)", response_json.len());

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
