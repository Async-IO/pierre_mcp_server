// ABOUTME: Tool execution handlers for MCP server tool calls and provider routing
// ABOUTME: Handles tool call routing, execution, authentication, and provider-specific operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::multitenant::{McpError, McpRequest, McpResponse, MultiTenantMcpServer};
use super::resources::ServerResources;
use super::tenant_isolation::extract_tenant_context_internal;
use crate::auth::AuthMethod as AuthResultMethod;
use crate::auth::AuthResult;
use crate::constants::{
    errors::{
        ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND, ERROR_TOKEN_EXPIRED,
        ERROR_TOKEN_INVALID, ERROR_TOKEN_MALFORMED, ERROR_UNAUTHORIZED, MSG_TOKEN_EXPIRED,
        MSG_TOKEN_INVALID, MSG_TOKEN_MALFORMED,
    },
    protocol::JSONRPC_VERSION,
    tools::{CONNECT_PROVIDER, DISCONNECT_PROVIDER, GET_CONNECTION_STATUS},
};
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, ErrorCode};
use crate::models::{OAuthNotification, TenantId};
use crate::tenant::TenantContext;
use crate::tools::context::{AuthMethod, ToolExecutionContext};
use crate::tools::result::ToolResult;
use crate::types::json_schemas;
use serde_json::{json, Value};
use std::fmt::Write;
use std::sync::Arc;
use std::time::Instant;
use tracing::field::Empty;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    Value::Number(serde_json::Number::from(0))
}

/// OAuth credentials provided in MCP requests
pub struct McpOAuthCredentials<'a> {
    /// Strava OAuth client ID
    pub strava_client_id: Option<&'a str>,
    /// Strava OAuth client secret
    pub strava_client_secret: Option<&'a str>,
    /// Fitbit OAuth client ID
    pub fitbit_client_id: Option<&'a str>,
    /// Fitbit OAuth client secret
    pub fitbit_client_secret: Option<&'a str>,
}

/// Context for routing tool calls with necessary resources and auth information
///
/// Tenant context is required for all tool executions to ensure proper
/// tenant isolation and tool enablement policy enforcement.
pub struct ToolRoutingContext<'a> {
    /// Server resources for dependency injection
    pub resources: &'a Arc<ServerResources>,
    /// Tenant context for multi-tenant isolation (required)
    pub tenant_context: &'a TenantContext,
    /// Authentication result with user and rate limit info
    pub auth_result: &'a AuthResult,
}

/// Tool execution handlers for MCP protocol
pub struct ToolHandlers;

impl ToolHandlers {
    /// Handle tools/call request with authentication from resources
    #[tracing::instrument(
        skip(request, resources),
        fields(
            method = %request.method,
            request_id = ?request.id,
            tool_name = Empty,
            user_id = Empty,
            tenant_id = Empty,
            success = Empty,
            duration_ms = Empty,
        )
    )]
    pub async fn handle_tools_call_with_resources(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        // Extract auth token from either HTTP Authorization header or MCP params
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

        debug!(
            "MCP tool call authentication attempt for method: {} (token source: {})",
            request.method,
            if request.auth_token.is_some() {
                "HTTP header"
            } else {
                "MCP params"
            }
        );

        match resources
            .auth_middleware
            .authenticate_request(auth_token)
            .await
        {
            Ok(auth_result) => {
                // Record authentication success in span (tenant_id recorded after extraction)
                tracing::Span::current().record("user_id", auth_result.user_id.to_string());

                info!(
                    "MCP tool call authentication successful for user: {} (method: {})",
                    auth_result.user_id,
                    auth_result.auth_method.display_name()
                );

                // Update user's last active timestamp
                if let Err(e) = resources
                    .database
                    .update_last_active(auth_result.user_id)
                    .await
                {
                    warn!(
                        user_id = %auth_result.user_id,
                        error = %e,
                        "Failed to update user last active timestamp (activity tracking impacted)"
                    );
                }

                // Extract tenant context from request and auth result
                // Tenant context is REQUIRED for tool execution to ensure tenant isolation
                // Priority: JWT active_tenant_id > user's default tenant
                let tenant_context = match extract_tenant_context_internal(
                    &resources.database,
                    Some(auth_result.user_id),
                    auth_result.active_tenant_id.map(TenantId::from), // Pass active_tenant_id from JWT claims
                    None, // MCP transport headers not applicable here
                )
                .await
                {
                    Ok(Some(ctx)) => ctx,
                    Ok(None) => {
                        // User has no tenant membership - cannot execute tools
                        warn!(
                            user_id = %auth_result.user_id,
                            "User has no tenant membership - rejecting tool execution"
                        );
                        return McpResponse::error_with_data(
                            request.id,
                            ERROR_UNAUTHORIZED,
                            "User must be assigned to a tenant to execute tools".to_owned(),
                            serde_json::json!({
                                "error_type": "tenant_required",
                                "user_id": auth_result.user_id.to_string()
                            }),
                        );
                    }
                    Err(e) => {
                        // Tenant extraction failed - cannot proceed safely
                        error!(
                            user_id = %auth_result.user_id,
                            error = %e,
                            "Tenant context extraction failed - rejecting tool execution"
                        );
                        return McpResponse::error_with_data(
                            request.id,
                            ERROR_INTERNAL_ERROR,
                            "Failed to extract tenant context".to_owned(),
                            serde_json::json!({
                                "error_type": "tenant_extraction_failed",
                                "detailed_error": e.to_string()
                            }),
                        );
                    }
                };

                // Record tenant context in span now that we have it
                tracing::Span::current().record("tenant_id", tenant_context.tenant_id.to_string());

                // Use the provided ServerResources directly
                Self::handle_tool_execution_direct(request, auth_result, tenant_context, resources)
                    .await
            }
            Err(e) => {
                tracing::Span::current().record("success", false);
                Self::handle_authentication_error(request, &e)
            }
        }
    }

    /// Check if a tool is enabled for a tenant, returning an error response if disabled
    ///
    /// Tenant context is now required - tool execution without tenant isolation is not allowed.
    async fn check_tool_enabled(
        resources: &Arc<ServerResources>,
        tenant_context: &TenantContext,
        tool_name: &str,
        request_id: Option<Value>,
    ) -> Option<McpResponse> {
        match resources
            .tool_selection
            .is_tool_enabled(tenant_context.tenant_id, tool_name)
            .await
        {
            Ok(true) => {
                debug!(
                    "Tool {} is enabled for tenant {}",
                    tool_name, tenant_context.tenant_id
                );
                None
            }
            Ok(false) => {
                warn!(
                    "Tool {} not enabled for tenant {} - rejecting",
                    tool_name, tenant_context.tenant_id
                );
                Some(McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    id: request_id,
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: format!(
                            "Tool '{tool_name}' is not available for your tenant. \
                             Contact your administrator to enable it."
                        ),
                        data: None,
                    }),
                })
            }
            Err(e) => {
                debug!(
                    "Tool {} not in catalog ({}), allowing execution",
                    tool_name, e
                );
                None
            }
        }
    }

    /// Handle tool execution directly using provided `ServerResources`
    ///
    /// Tenant context is now required for all tool executions to ensure proper
    /// tenant isolation and tool enablement policy enforcement.
    #[tracing::instrument(
        skip(request, auth_result, tenant_context, resources),
        fields(
            tool_name = Empty,
            user_id = %auth_result.user_id,
            tenant_id = %tenant_context.tenant_id,
            success = Empty,
            duration_ms = Empty,
        )
    )]
    async fn handle_tool_execution_direct(
        request: McpRequest,
        auth_result: AuthResult,
        tenant_context: TenantContext,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let Some(params) = request.params else {
            error!("Missing request parameters in tools/call");
            return McpResponse {
                jsonrpc: "2.0".to_owned(),
                id: request.id,
                result: None,
                error: Some(McpError {
                    code: ERROR_INVALID_PARAMS,
                    message: "Invalid params: Missing request parameters".to_owned(),
                    data: None,
                }),
            };
        };

        // Parse tool call parameters with type safety
        let tool_params = match serde_json::from_value::<json_schemas::ToolCallParams>(params) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to parse tool call parameters: {}", e);
                return McpResponse {
                    jsonrpc: "2.0".to_owned(),
                    id: request.id,
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid tool call parameters: {e}"),
                        data: None,
                    }),
                };
            }
        };

        let tool_name = &tool_params.name;
        let args = &tool_params.arguments;
        let user_id = auth_result.user_id;

        // Record tool name in span
        tracing::Span::current().record("tool_name", tool_name.as_str());

        // Check if tool is enabled for this tenant
        if let Some(error_response) =
            Self::check_tool_enabled(resources, &tenant_context, tool_name, request.id.clone())
                .await
        {
            return error_response;
        }

        let start_time = Instant::now();

        info!(
            "Executing tool call: {} for user: {} using {} authentication",
            tool_name,
            user_id,
            auth_result.auth_method.display_name()
        );

        // Use the provided ServerResources directly - no fake resource creation!
        let routing_context = ToolRoutingContext {
            resources,
            tenant_context: &tenant_context,
            auth_result: &auth_result,
        };

        let result = Self::route_tool_call(
            tool_name,
            args,
            request.id.unwrap_or_else(default_request_id),
            user_id,
            &routing_context,
        )
        .await;

        // Automatically append unread OAuth notifications to successful responses
        debug!(
            "About to check for OAuth notifications for user {} after {} tool call",
            user_id, tool_name
        );
        let result = Self::append_oauth_notifications_to_response(
            result,
            user_id,
            tool_name,
            &routing_context.resources.database,
        )
        .await;

        // Record completion metrics in span
        let duration = start_time.elapsed();
        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);
        let success = result.error.is_none();

        tracing::Span::current()
            .record("duration_ms", duration_ms)
            .record("success", success);

        if success {
            info!(
                "Tool call completed successfully: {} for user: {} in {}ms",
                tool_name, user_id, duration_ms
            );
        } else {
            warn!(
                "Tool call failed: {} for user: {} in {}ms - {:?}",
                tool_name, user_id, duration_ms, result.error
            );
        }

        result
    }

    /// Handle authentication error
    fn handle_authentication_error(request: McpRequest, e: &AppError) -> McpResponse {
        warn!("MCP tool call authentication failed: {}", e);

        // Determine specific error code based on error message
        let error_message = e.to_string();
        let (error_code, error_msg) = if error_message.contains("JWT token expired") {
            (ERROR_TOKEN_EXPIRED, MSG_TOKEN_EXPIRED)
        } else if error_message.contains("JWT token signature is invalid") {
            (ERROR_TOKEN_INVALID, MSG_TOKEN_INVALID)
        } else if error_message.contains("JWT token is malformed") {
            (ERROR_TOKEN_MALFORMED, MSG_TOKEN_MALFORMED)
        } else {
            (ERROR_UNAUTHORIZED, "Authentication required")
        };

        McpResponse::error_with_data(
            request.id,
            error_code,
            error_msg.to_owned(),
            serde_json::json!({
                "detailed_error": error_message,
                "authentication_failed": true
            }),
        )
    }

    /// Build a `ToolExecutionContext` from MCP routing context
    ///
    /// Tenant context is always available since tool execution requires it.
    fn build_tool_context(
        user_id: Uuid,
        request_id: Option<Value>,
        ctx: &ToolRoutingContext<'_>,
    ) -> ToolExecutionContext {
        // Map AuthResult auth_method to ToolExecutionContext AuthMethod
        let auth_method = match &ctx.auth_result.auth_method {
            AuthResultMethod::JwtToken { .. } => AuthMethod::JwtBearer,
            AuthResultMethod::ApiKey { .. } => AuthMethod::ApiKey,
        };

        let mut tool_ctx = ToolExecutionContext::new(user_id, ctx.resources.clone(), auth_method)
            .with_tenant(ctx.tenant_context.tenant_id);

        // Add request ID if available
        if let Some(req_id) = request_id {
            tool_ctx = tool_ctx.with_request_id(req_id);
        }

        tool_ctx
    }

    /// Convert a `ToolResult` to an `McpResponse`
    ///
    /// Returns both `content` and `structuredContent` per MCP Specification 2025-06-18:
    ///
    /// > "For backwards compatibility, a tool that returns structured content
    /// > SHOULD also return the serialized JSON in a `TextContent` block."
    ///
    /// - `content`: Text array containing serialized JSON for older MCP clients
    ///   (pre-June 2025) that only understand this format
    /// - `structuredContent`: Typed JSON for modern clients that can leverage
    ///   schema validation and programmatic access
    ///
    /// Both fields are returned to ensure maximum interoperability across the
    /// MCP ecosystem. See `sdk/MCP_COMPLIANCE.md` for compliance details.
    fn tool_result_to_mcp_response(result: &ToolResult, request_id: Value) -> McpResponse {
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: Some(request_id),
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": result.content.to_string()
                }],
                "structuredContent": result.content,
                "isError": result.is_error
            })),
            error: None,
        }
    }

    /// Convert an `AppError` to an `McpResponse`
    fn error_to_mcp_response(error: &AppError, request_id: Value) -> McpResponse {
        let error_code = match error.code {
            ErrorCode::ResourceNotFound => ERROR_METHOD_NOT_FOUND,
            ErrorCode::InvalidInput => ERROR_INVALID_PARAMS,
            ErrorCode::PermissionDenied => ERROR_UNAUTHORIZED,
            _ => ERROR_INTERNAL_ERROR,
        };

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: Some(request_id),
            result: None,
            error: Some(McpError {
                code: error_code,
                message: error.to_string(),
                data: None,
            }),
        }
    }

    /// Route tool calls to appropriate handlers based on tool type and tenant context
    ///
    /// Uses the `ToolRegistry` for tool execution. OAuth connection tools are handled
    /// specially due to their complex flow requirements.
    pub async fn route_tool_call(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        // Handle OAuth connection tools specially - they have complex flow requirements
        // that don't fit the standard McpTool pattern
        match tool_name {
            CONNECT_PROVIDER => {
                return Self::handle_connect_provider(args, request_id);
            }
            GET_CONNECTION_STATUS => {
                return Self::handle_get_connection_status(args, request_id, ctx).await;
            }
            DISCONNECT_PROVIDER => {
                return Self::handle_disconnect_provider(args, request_id, ctx).await;
            }
            _ => {}
        }

        // Try the registry first for all other tools
        if ctx.resources.tool_registry.contains(tool_name) {
            let tool_ctx = Self::build_tool_context(user_id, Some(request_id.clone()), ctx);

            match ctx
                .resources
                .tool_registry
                .execute(tool_name, args.clone(), &tool_ctx)
                .await
            {
                Ok(result) => Self::tool_result_to_mcp_response(&result, request_id),
                Err(e) => Self::error_to_mcp_response(&e, request_id),
            }
        } else {
            // Fall back to provider tool routing for tools not in the registry
            MultiTenantMcpServer::route_provider_tool(tool_name, args, request_id, ctx).await
        }
    }

    /// Handle `connect_provider` OAuth tool
    fn handle_connect_provider(args: &Value, request_id: Value) -> McpResponse {
        let params = serde_json::from_value::<json_schemas::ConnectProviderParams>(args.clone())
            .unwrap_or_else(|_| json_schemas::ConnectProviderParams {
                provider: String::new(),
                strava_client_id: None,
                strava_client_secret: None,
                fitbit_client_id: None,
                fitbit_client_secret: None,
            });

        let provider_name = params.provider.to_lowercase();

        // Validate provider
        if provider_name.is_empty() || !["strava", "fitbit"].contains(&provider_name.as_str()) {
            return McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                id: Some(request_id),
                result: Some(json!({
                    "content": [{
                        "type": "text",
                        "text": format!("Invalid provider '{provider_name}'. Supported providers are: strava, fitbit")
                    }],
                    "isError": true
                })),
                error: None,
            };
        }

        // Return unified auth flow response
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            id: Some(request_id),
            result: Some(json!({
                "content": [{
                    "type": "text",
                    "text": format!(
                        "Starting unified authentication for {}. This will:\n\n1. First authenticate you with Pierre Fitness Server\n2. Then connect you to {} for your fitness data\n\nOpening browser for secure authentication...",
                        provider_name.to_uppercase(),
                        provider_name.to_uppercase()
                    )
                }],
                "isError": false,
                "requiresAuth": true,
                "authUrl": "oauth2/authorize",
                "unifiedFlow": true,
                "provider": provider_name,
                "message": format!("Please complete unified authentication with Pierre and {} in your browser.", provider_name.to_uppercase())
            })),
            error: None,
        }
    }

    /// Handle `get_connection_status` OAuth tool
    ///
    /// Tenant context is always available since tool execution requires it.
    async fn handle_get_connection_status(
        args: &Value,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        let params =
            serde_json::from_value::<json_schemas::GetConnectionStatusParams>(args.clone())
                .unwrap_or_default();

        let credentials = McpOAuthCredentials {
            strava_client_id: params.strava_client_id.as_deref(),
            strava_client_secret: params.strava_client_secret.as_deref(),
            fitbit_client_id: params.fitbit_client_id.as_deref(),
            fitbit_client_secret: params.fitbit_client_secret.as_deref(),
        };

        MultiTenantMcpServer::handle_tenant_connection_status(
            ctx.tenant_context,
            &ctx.resources.tenant_oauth_client,
            &ctx.resources.database,
            request_id,
            credentials,
            ctx.resources.config.http_port,
            &ctx.resources.config,
        )
        .await
    }

    /// Handle `disconnect_provider` OAuth tool
    async fn handle_disconnect_provider(
        args: &Value,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        let params =
            match serde_json::from_value::<json_schemas::DisconnectProviderParams>(args.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Invalid disconnect_provider parameters: {e}"),
                            data: None,
                        }),
                        id: Some(request_id),
                    };
                }
            };

        MultiTenantMcpServer::route_disconnect_tool(&params.provider, request_id, ctx).await
    }

    /// Build notification text from a list of OAuth notifications
    fn build_notification_text(notifications: &[OAuthNotification]) -> String {
        let mut notification_text = String::from("\n\nOAuth Connection Updates:\n");
        for notification in notifications {
            let status_indicator = if notification.success {
                "[SUCCESS]"
            } else {
                "[FAILED]"
            };
            writeln!(
                &mut notification_text,
                "{} {}: {}",
                status_indicator,
                notification.provider.to_uppercase(),
                notification.message
            )
            .unwrap_or_else(|_| warn!("Failed to write notification text"));
        }
        notification_text
    }

    /// Append notification text to an MCP response result
    fn append_notification_to_result(result: &mut Value, notification_text: &str) {
        if let Some(content) = result.get_mut("content") {
            if let Some(text_value) = content.as_array_mut() {
                text_value.push(json!({
                    "type": "text",
                    "text": notification_text
                }));
                return;
            }
            if let Some(text_str) = content.as_str() {
                *content = json!(format!("{text_str}{notification_text}"));
                return;
            }
        }

        if let Some(message) = result.get_mut("message") {
            if let Some(msg_str) = message.as_str() {
                *message = json!(format!("{msg_str}{notification_text}"));
                return;
            }
        }

        if let Some(obj) = result.as_object_mut() {
            obj.insert("oauth_notifications".to_owned(), json!(notification_text));
        }
    }

    /// Mark a list of notifications as read in the database
    async fn mark_notifications_read(
        database: &Database,
        notifications: &[OAuthNotification],
        user_id: Uuid,
    ) {
        for notification in notifications {
            if let Err(e) = database
                .mark_oauth_notification_read(&notification.id, user_id)
                .await
            {
                warn!(
                    "Failed to mark notification {} as read after delivery: {}",
                    notification.id, e
                );
            }
        }
    }

    /// Check if a tool name should skip notification checking
    /// Note: With OAuth notification tools removed, this always returns false.
    /// Kept for potential future tools that might need to skip notification checks.
    const fn should_skip_notification_check(_tool_name: &str) -> bool {
        false
    }

    /// Check if notifications should be fetched for this response
    fn should_fetch_notifications(response: &McpResponse, tool_name: &str, user_id: Uuid) -> bool {
        if response.error.is_some() {
            debug!(
                "NOTIFICATION_CHECK: Skipping due to error response for user {}",
                user_id
            );
            return false;
        }

        if Self::should_skip_notification_check(tool_name) {
            debug!(
                "NOTIFICATION_CHECK: Skipping for notification-related tool {} for user {}",
                tool_name, user_id
            );
            return false;
        }

        true
    }

    /// Fetch unread notifications if any exist
    async fn fetch_unread_notifications(
        database: &Database,
        user_id: Uuid,
        tool_name: &str,
    ) -> Option<Vec<OAuthNotification>> {
        match database.get_unread_oauth_notifications(user_id).await {
            Ok(notifications) if !notifications.is_empty() => {
                debug!(
                    "Found {} unread OAuth notifications for user {} during {} tool call",
                    notifications.len(),
                    user_id,
                    tool_name
                );
                Some(notifications)
            }
            Ok(_) => {
                debug!(
                    "NOTIFICATION_CHECK: No unread notifications found for user {} during {} tool call",
                    user_id, tool_name
                );
                None
            }
            Err(e) => {
                warn!(
                    "Failed to check OAuth notifications for user {} during {} tool call: {}",
                    user_id, tool_name, e
                );
                None
            }
        }
    }

    /// Automatically append unread OAuth notifications to successful tool responses
    async fn append_oauth_notifications_to_response(
        mut response: McpResponse,
        user_id: Uuid,
        tool_name: &str,
        database: &Database,
    ) -> McpResponse {
        debug!(
            "NOTIFICATION_CHECK: Starting notification check for user {} with tool {}",
            user_id, tool_name
        );

        if !Self::should_fetch_notifications(&response, tool_name, user_id) {
            return response;
        }

        let Some(unread_notifications) =
            Self::fetch_unread_notifications(database, user_id, tool_name).await
        else {
            return response;
        };

        let notification_text = Self::build_notification_text(&unread_notifications);

        if let Some(ref mut result) = response.result {
            Self::append_notification_to_result(result, &notification_text);
        }

        info!(
            "Automatically delivered {} OAuth notifications to user {} via {} tool response",
            unread_notifications.len(),
            user_id,
            tool_name
        );

        Self::mark_notifications_read(database, &unread_notifications, user_id).await;

        response
    }
}
