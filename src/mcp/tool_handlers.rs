// ABOUTME: Tool execution handlers for MCP server tool calls and provider routing
// ABOUTME: Handles tool call routing, execution, authentication, and provider-specific operations

use super::multitenant::{McpError, McpRequest, McpResponse, MultiTenantMcpServer};
use super::resources::ServerResources;
use crate::auth::AuthResult;
use crate::constants::{
    errors::{
        ERROR_AUTHENTICATION, ERROR_INVALID_PARAMS, ERROR_TOOL_EXECUTION, ERROR_UNAUTHORIZED,
        MSG_AUTHENTICATION, MSG_TOOL_EXECUTION,
    },
    json_fields::PROVIDER,
    protocol::JSONRPC_VERSION,
    tools::{
        ANNOUNCE_OAUTH_SUCCESS, CHECK_OAUTH_NOTIFICATIONS, DISCONNECT_PROVIDER,
        GET_CONNECTION_STATUS, GET_NOTIFICATIONS, MARK_NOTIFICATIONS_READ, SET_GOAL,
        TRACK_PROGRESS,
    },
};
use crate::database_plugins::DatabaseProvider;
use crate::tenant::TenantContext;
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    Value::Number(serde_json::Number::from(0))
}

/// OAuth credentials provided in MCP requests
pub struct McpOAuthCredentials<'a> {
    pub strava_client_id: Option<&'a str>,
    pub strava_client_secret: Option<&'a str>,
    pub fitbit_client_id: Option<&'a str>,
    pub fitbit_client_secret: Option<&'a str>,
}

/// Context for routing tool calls with necessary resources and auth information
pub struct ToolRoutingContext<'a> {
    pub resources: &'a Arc<ServerResources>,
    pub tenant_context: &'a Option<TenantContext>,
    pub auth_result: &'a AuthResult,
}

/// Tool execution handlers for MCP protocol
pub struct ToolHandlers;

impl ToolHandlers {
    /// Handle tools/call request with authentication from resources
    pub async fn handle_tools_call_with_resources(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let auth_token = request.auth_token.as_deref();

        debug!(
            "MCP tool call authentication attempt for method: {}",
            request.method
        );

        match resources
            .auth_middleware
            .authenticate_request(auth_token)
            .await
        {
            Ok(auth_result) => {
                info!(
                    "MCP tool call authentication successful for user: {} (method: {})",
                    auth_result.user_id,
                    auth_result.auth_method.display_name()
                );

                // Update user's last active timestamp
                let _ = resources
                    .database
                    .update_last_active(auth_result.user_id)
                    .await;

                // Extract tenant context from request and auth result
                let tenant_context = MultiTenantMcpServer::extract_tenant_context_internal(
                    &request,
                    &auth_result,
                    &resources.database,
                )
                .await
                .unwrap_or(None);

                // Use the provided ServerResources directly
                Self::handle_tool_execution_direct(request, auth_result, tenant_context, resources)
                    .await
            }
            Err(e) => Self::handle_authentication_error(request, &e),
        }
    }

    /// Handle tool execution directly using provided `ServerResources`
    async fn handle_tool_execution_direct(
        request: McpRequest,
        auth_result: AuthResult,
        tenant_context: Option<TenantContext>,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let Some(params) = request.params else {
            error!("Missing request parameters in tools/call");
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.unwrap_or_else(default_request_id),
                result: None,
                error: Some(McpError {
                    code: ERROR_INVALID_PARAMS,
                    message: "Invalid params: Missing request parameters".to_string(),
                    data: None,
                }),
            };
        };
        let tool_name = params["name"].as_str().unwrap_or("");
        let args = &params["arguments"];
        let user_id = auth_result.user_id;

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

        Self::route_tool_call(
            tool_name,
            args,
            request.id.unwrap_or_else(default_request_id),
            user_id,
            &routing_context,
        )
        .await
    }

    /// Handle authentication error
    fn handle_authentication_error(request: McpRequest, e: &anyhow::Error) -> McpResponse {
        warn!("MCP tool call authentication failed: {}", e);

        // Determine specific error code based on error message
        let error_message = e.to_string();
        let (error_code, error_msg) = if error_message.contains("JWT token expired") {
            (
                crate::constants::errors::ERROR_TOKEN_EXPIRED,
                crate::constants::errors::MSG_TOKEN_EXPIRED,
            )
        } else if error_message.contains("JWT token signature is invalid") {
            (
                crate::constants::errors::ERROR_TOKEN_INVALID,
                crate::constants::errors::MSG_TOKEN_INVALID,
            )
        } else if error_message.contains("JWT token is malformed") {
            (
                crate::constants::errors::ERROR_TOKEN_MALFORMED,
                crate::constants::errors::MSG_TOKEN_MALFORMED,
            )
        } else {
            (ERROR_UNAUTHORIZED, "Authentication required")
        };

        McpResponse::error_with_data(
            request.id.unwrap_or_else(default_request_id),
            error_code,
            error_msg.to_string(),
            serde_json::json!({
                "detailed_error": error_message,
                "authentication_failed": true
            }),
        )
    }

    /// Route tool calls to appropriate handlers based on tool type and tenant context
    pub async fn route_tool_call(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        match tool_name {
            // Note: CONNECT_STRAVA and CONNECT_FITBIT tools removed - use tenant-level OAuth configuration
            GET_CONNECTION_STATUS => {
                if let Some(ref tenant_ctx) = ctx.tenant_context {
                    // Extract optional OAuth credentials from args
                    let credentials = McpOAuthCredentials {
                        strava_client_id: args.get("strava_client_id").and_then(|v| v.as_str()),
                        strava_client_secret: args
                            .get("strava_client_secret")
                            .and_then(|v| v.as_str()),
                        fitbit_client_id: args.get("fitbit_client_id").and_then(|v| v.as_str()),
                        fitbit_client_secret: args
                            .get("fitbit_client_secret")
                            .and_then(|v| v.as_str()),
                    };

                    return MultiTenantMcpServer::handle_tenant_connection_status(
                        tenant_ctx,
                        &ctx.resources.tenant_oauth_client,
                        request_id,
                        credentials,
                    )
                    .await;
                }
                // No legacy fallback - require tenant context
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: "No tenant context found. User must be assigned to a tenant."
                            .to_string(),
                        data: None,
                    }),
                    id: request_id,
                }
            }
            DISCONNECT_PROVIDER => {
                let provider_name = args[PROVIDER].as_str().unwrap_or("");
                MultiTenantMcpServer::route_disconnect_tool(provider_name, user_id, request_id, ctx)
            }
            MARK_NOTIFICATIONS_READ => {
                let notification_id = args.get("notification_id").and_then(|v| v.as_str());
                Self::handle_mark_notifications_read(notification_id, user_id, request_id, ctx)
                    .await
            }
            GET_NOTIFICATIONS => {
                let include_read = args
                    .get("include_read")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let provider = args.get("provider").and_then(|v| v.as_str());
                Self::handle_get_notifications(include_read, provider, user_id, request_id, ctx)
                    .await
            }
            ANNOUNCE_OAUTH_SUCCESS => {
                let provider = args
                    .get(PROVIDER)
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let message = args
                    .get("message")
                    .and_then(|v| v.as_str())
                    .unwrap_or("OAuth completed successfully");
                let notification_id = args
                    .get("notification_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                Self::handle_announce_oauth_success(
                    provider,
                    message,
                    notification_id,
                    user_id,
                    request_id,
                    ctx,
                )
                .await
            }
            CHECK_OAUTH_NOTIFICATIONS => {
                Self::handle_check_oauth_notifications(user_id, request_id, ctx).await
            }
            // Goal tools - use existing MCP implementations
            SET_GOAL | TRACK_PROGRESS => {
                MultiTenantMcpServer::handle_tool_without_provider(
                    tool_name,
                    args,
                    request_id,
                    user_id,
                    &ctx.resources.database,
                    ctx.auth_result,
                )
                .await
            }
            _ => {
                MultiTenantMcpServer::route_provider_tool(tool_name, args, request_id, user_id, ctx)
                    .await
            }
        }
    }

    /// Handle mark notifications read tool
    async fn handle_mark_notifications_read(
        notification_id: Option<&str>,
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        match notification_id {
            Some(id) => {
                // Mark specific notification as read
                match ctx
                    .resources
                    .database
                    .mark_oauth_notification_read(id, user_id)
                    .await
                {
                    Ok(marked) => {
                        if marked {
                            McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: Some(serde_json::json!({
                                    "success": true,
                                    "message": "Notification marked as read",
                                    "notification_id": id
                                })),
                                error: None,
                                id: request_id,
                            }
                        } else {
                            McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: "Notification not found or already read".to_string(),
                                    data: None,
                                }),
                                id: request_id,
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to mark notification as read: {}", e);
                        McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_TOOL_EXECUTION,
                                message: format!("{MSG_TOOL_EXECUTION}: Failed to mark notification as read - {e}"),
                                data: None,
                            }),
                            id: request_id,
                        }
                    }
                }
            }
            None => {
                // Mark all notifications as read
                match ctx
                    .resources
                    .database
                    .mark_all_oauth_notifications_read(user_id)
                    .await
                {
                    Ok(count) => McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: Some(serde_json::json!({
                            "success": true,
                            "message": format!("Marked {} notifications as read", count),
                            "marked_count": count
                        })),
                        error: None,
                        id: request_id,
                    },
                    Err(e) => {
                        error!("Failed to mark all notifications as read: {}", e);
                        McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_TOOL_EXECUTION,
                                message: format!("{MSG_TOOL_EXECUTION}: Failed to mark notifications as read - {e}"),
                                data: None,
                            }),
                            id: request_id,
                        }
                    }
                }
            }
        }
    }

    /// Handle get notifications tool
    async fn handle_get_notifications(
        include_read: bool,
        provider: Option<&str>,
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        match if include_read {
            ctx.resources
                .database
                .get_all_oauth_notifications(user_id, None)
        } else {
            ctx.resources
                .database
                .get_unread_oauth_notifications(user_id)
        }
        .await
        {
            Ok(mut notifications) => {
                tracing::debug!(
                    "Retrieved {} notifications from database for user {}",
                    notifications.len(),
                    user_id
                );

                // Filter by provider if specified
                if let Some(provider_filter) = provider {
                    let initial_count = notifications.len();
                    notifications.retain(|n| n.provider.as_str() == provider_filter);
                    tracing::debug!(
                        "After provider filter '{}': {} notifications (was {})",
                        provider_filter,
                        notifications.len(),
                        initial_count
                    );
                }

                tracing::debug!(
                    "Final notification count to return: {}",
                    notifications.len()
                );
                for (i, notif) in notifications.iter().enumerate() {
                    tracing::debug!(
                        "Notification {}: id={}, provider={}, message={}",
                        i,
                        notif.id,
                        notif.provider,
                        notif.message
                    );
                }

                let response_data = serde_json::json!({
                    "success": true,
                    "notifications": notifications,
                    "count": notifications.len()
                });
                tracing::debug!(
                    "MCP Response JSON: {}",
                    serde_json::to_string_pretty(&response_data).unwrap_or_default()
                );

                let mcp_response = McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: Some(response_data),
                    error: None,
                    id: request_id,
                };
                tracing::debug!(
                    "Full MCP Response: {}",
                    serde_json::to_string_pretty(&mcp_response).unwrap_or_default()
                );

                mcp_response
            }
            Err(e) => {
                error!("Failed to retrieve notifications: {}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_TOOL_EXECUTION,
                        message: format!(
                            "{MSG_TOOL_EXECUTION}: Failed to retrieve notifications - {e}"
                        ),
                        data: None,
                    }),
                    id: request_id,
                }
            }
        }
    }

    /// Handle announce OAuth success tool - displays OAuth completion message in chat
    async fn handle_announce_oauth_success(
        provider: &str,
        message: &str,
        notification_id: &str,
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        info!(
            "Announcing OAuth success for user {}: provider={}, message={}, notification_id={}",
            user_id, provider, message, notification_id
        );

        // Create a visible success message in the chat
        let success_message = format!(
            "🎉 **OAuth Connection Successful!**\n\n✅ Successfully connected to **{}**\n📝 {}",
            provider.to_uppercase(),
            message
        );

        // Also mark this notification as read to avoid re-processing
        if let Err(e) = ctx
            .resources
            .database
            .mark_oauth_notification_read(notification_id, user_id)
            .await
        {
            debug!(
                "Failed to mark notification {} as read after announcing: {}",
                notification_id, e
            );
        }

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(serde_json::json!({
                "success": true,
                "announcement": success_message,
                "provider": provider,
                "original_message": message,
                "notification_id": notification_id,
                "visible_to_user": true
            })),
            error: None,
            id: request_id,
        }
    }

    /// Handle check OAuth notifications tool - looks for new unread notifications and announces them
    async fn handle_check_oauth_notifications(
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        info!("Checking for OAuth notifications for user {}", user_id);

        match ctx
            .resources
            .database
            .get_unread_oauth_notifications(user_id)
            .await
        {
            Ok(notifications) => {
                if notifications.is_empty() {
                    // No new notifications
                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: Some(serde_json::json!({
                            "success": true,
                            "message": "No new OAuth notifications",
                            "notifications_count": 0
                        })),
                        error: None,
                        id: request_id,
                    }
                } else {
                    // Announce all new notifications
                    let mut announcements = Vec::new();

                    for notification in &notifications {
                        let announcement = format!(
                            "🎉 **OAuth Connection Successful!**\n\n✅ Successfully connected to **{}**\n📝 {}",
                            notification.provider.to_uppercase(),
                            notification.message
                        );
                        announcements.push(announcement);

                        // Mark this notification as read
                        if let Err(e) = ctx
                            .resources
                            .database
                            .mark_oauth_notification_read(&notification.id, user_id)
                            .await
                        {
                            debug!(
                                "Failed to mark notification {} as read: {}",
                                notification.id, e
                            );
                        }
                    }

                    let combined_announcement = announcements.join("\n\n---\n\n");

                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: Some(serde_json::json!({
                            "success": true,
                            "announcement": combined_announcement,
                            "notifications_count": notifications.len(),
                            "visible_to_user": true
                        })),
                        error: None,
                        id: request_id,
                    }
                }
            }
            Err(e) => {
                error!("Failed to check OAuth notifications: {}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_AUTHENTICATION,
                        message: format!(
                            "{MSG_AUTHENTICATION}: Failed to check OAuth notifications - {e}"
                        ),
                        data: None,
                    }),
                    id: request_id,
                }
            }
        }
    }
}
