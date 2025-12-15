// ABOUTME: Tool execution handlers for MCP server tool calls and provider routing
// ABOUTME: Handles tool call routing, execution, authentication, and provider-specific operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::multitenant::{McpError, McpRequest, McpResponse, MultiTenantMcpServer};
use super::resources::ServerResources;
use super::tenant_isolation::extract_tenant_context_internal;
use crate::auth::AuthResult;
use crate::config::fitness::FitnessConfig;
use crate::constants::{
    errors::{
        ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND, ERROR_TOKEN_EXPIRED,
        ERROR_TOKEN_INVALID, ERROR_TOKEN_MALFORMED, ERROR_UNAUTHORIZED, MSG_TOKEN_EXPIRED,
        MSG_TOKEN_INVALID, MSG_TOKEN_MALFORMED,
    },
    protocol::JSONRPC_VERSION,
    tools::{
        CONNECT_PROVIDER, DELETE_FITNESS_CONFIG, DISCONNECT_PROVIDER, GET_CONNECTION_STATUS,
        GET_FITNESS_CONFIG, LIST_FITNESS_CONFIGS, SET_FITNESS_CONFIG,
    },
};
use crate::database::oauth_notifications::OAuthNotification;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::tenant::TenantContext;
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
pub struct ToolRoutingContext<'a> {
    /// Server resources for dependency injection
    pub resources: &'a Arc<ServerResources>,
    /// Optional tenant context for multi-tenant isolation
    pub tenant_context: &'a Option<TenantContext>,
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
                // Record authentication success in span
                tracing::Span::current()
                    .record("user_id", auth_result.user_id.to_string())
                    .record("tenant_id", auth_result.user_id.to_string()); // Use user_id as tenant_id for now

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
                let tenant_context = extract_tenant_context_internal(
                    &resources.database,
                    Some(auth_result.user_id),
                    None,
                    None, // MCP transport headers not applicable here
                )
                .await
                .inspect_err(|e| {
                    warn!(
                        user_id = %auth_result.user_id,
                        error = %e,
                        "Failed to extract tenant context - tool will execute without tenant isolation"
                    );
                })
                .ok()
                .flatten();

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

    /// Handle tool execution directly using provided `ServerResources`
    #[tracing::instrument(
        skip(request, auth_result, tenant_context, resources),
        fields(
            tool_name = Empty,
            user_id = %auth_result.user_id,
            tenant_id = %auth_result.user_id, // Use user_id as tenant_id for now
            success = Empty,
            duration_ms = Empty,
        )
    )]
    async fn handle_tool_execution_direct(
        request: McpRequest,
        auth_result: AuthResult,
        tenant_context: Option<TenantContext>,
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

    /// Route tool calls to appropriate handlers based on tool type and tenant context
    #[allow(clippy::too_many_lines)] // Long function: Handles comprehensive tool routing for all tool types
    pub async fn route_tool_call(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        match tool_name {
            // Note: connect_to_pierre removed - SDK bridge handles authentication locally
            // The bridge intercepts connect_to_pierre calls and triggers OAuth via RFC 8414 discovery
            CONNECT_PROVIDER => {
                // Handle unified OAuth flow: Pierre + Provider authentication in one session
                let params =
                    serde_json::from_value::<json_schemas::ConnectProviderParams>(args.clone())
                        .unwrap_or_else(|_| json_schemas::ConnectProviderParams {
                            provider: String::new(),
                            strava_client_id: None,
                            strava_client_secret: None,
                            fitbit_client_id: None,
                            fitbit_client_secret: None,
                        });

                let provider_name = params.provider.to_lowercase();

                // Validate provider
                if provider_name.is_empty()
                    || !["strava", "fitbit"].contains(&provider_name.as_str())
                {
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
            GET_CONNECTION_STATUS => {
                if let Some(ref tenant_ctx) = ctx.tenant_context {
                    // Extract optional OAuth credentials from args using typed params
                    let params = serde_json::from_value::<json_schemas::GetConnectionStatusParams>(
                        args.clone(),
                    )
                    .unwrap_or_default();

                    let credentials = McpOAuthCredentials {
                        strava_client_id: params.strava_client_id.as_deref(),
                        strava_client_secret: params.strava_client_secret.as_deref(),
                        fitbit_client_id: params.fitbit_client_id.as_deref(),
                        fitbit_client_secret: params.fitbit_client_secret.as_deref(),
                    };

                    return MultiTenantMcpServer::handle_tenant_connection_status(
                        tenant_ctx,
                        &ctx.resources.tenant_oauth_client,
                        &ctx.resources.database,
                        request_id,
                        credentials,
                        ctx.resources.config.http_port,
                        &ctx.resources.config,
                    )
                    .await;
                }
                // No legacy fallback - require tenant context
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: "No tenant context found. User must be assigned to a tenant."
                            .to_owned(),
                        data: None,
                    }),
                    id: Some(request_id),
                }
            }
            DISCONNECT_PROVIDER => {
                let params = match serde_json::from_value::<json_schemas::DisconnectProviderParams>(
                    args.clone(),
                ) {
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
                MultiTenantMcpServer::route_disconnect_tool(
                    &params.provider,
                    user_id,
                    request_id,
                    ctx,
                )
                .await
            }
            // Fitness configuration tools
            GET_FITNESS_CONFIG
            | SET_FITNESS_CONFIG
            | LIST_FITNESS_CONFIGS
            | DELETE_FITNESS_CONFIG => {
                Self::handle_fitness_config_tool(
                    tool_name,
                    args.clone(),
                    request_id,
                    &user_id,
                    ctx.resources.clone(),
                )
                .await
            }
            _ => {
                MultiTenantMcpServer::route_provider_tool(tool_name, args, request_id, user_id, ctx)
                    .await
            }
        }
    }

    /// Handle fitness configuration tool calls
    #[allow(clippy::too_many_lines)] // Long function: Handles complete fitness configuration tool operations
    async fn handle_fitness_config_tool(
        tool_name: &str,
        args: serde_json::Value,
        request_id: serde_json::Value,
        user_id: &Uuid,
        resources: Arc<ServerResources>,
    ) -> McpResponse {
        // Get user's tenant_id for tenant isolation
        let tenant_id = match resources.database.get_user(*user_id).await {
            Ok(Some(user)) => match user.tenant_id {
                Some(tid) => tid,
                None => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: "User has no tenant assigned".to_owned(),
                            data: None,
                        }),
                        id: Some(request_id),
                    };
                }
            },
            Ok(None) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: "User not found".to_owned(),
                        data: None,
                    }),
                    id: Some(request_id),
                };
            }
            Err(e) => {
                error!("Database error getting user: {}", e);
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Database error".to_owned(),
                        data: None,
                    }),
                    id: Some(request_id),
                };
            }
        };

        match tool_name {
            GET_FITNESS_CONFIG => {
                Self::handle_get_fitness_config(
                    args,
                    request_id,
                    &tenant_id,
                    &user_id.to_string(),
                    &resources.database,
                )
                .await
            }
            SET_FITNESS_CONFIG => {
                Self::handle_set_fitness_config(
                    args,
                    request_id,
                    &tenant_id,
                    &user_id.to_string(),
                    &resources.database,
                )
                .await
            }
            LIST_FITNESS_CONFIGS => {
                Self::handle_list_fitness_configs(
                    request_id,
                    &tenant_id,
                    &user_id.to_string(),
                    &resources.database,
                )
                .await
            }
            DELETE_FITNESS_CONFIG => {
                Self::handle_delete_fitness_config(
                    args,
                    request_id,
                    &tenant_id,
                    &user_id.to_string(),
                    &resources.database,
                )
                .await
            }
            _ => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: "Unknown fitness config tool".to_owned(),
                    data: None,
                }),
                id: Some(request_id),
            },
        }
    }

    async fn handle_get_fitness_config(
        args: serde_json::Value,
        request_id: serde_json::Value,
        tenant_id: &str,
        user_id: &str,
        database: &Database,
    ) -> McpResponse {
        let params = serde_json::from_value::<json_schemas::GetFitnessConfigParams>(args)
            .unwrap_or_default();
        let config_name = &params.configuration_name;

        match database
            .get_user_fitness_config(tenant_id, user_id, config_name)
            .await
        {
            Ok(Some(config)) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: Some(json!({
                    "configuration_name": config_name,
                    "configuration": config
                })),
                error: None,
                id: Some(request_id),
            },
            Ok(None) => {
                // Try tenant-level config
                match database
                    .get_tenant_fitness_config(tenant_id, config_name)
                    .await
                {
                    Ok(Some(config)) => McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: Some(json!({
                            "configuration_name": config_name,
                            "configuration": config,
                            "source": "tenant"
                        })),
                        error: None,
                        id: Some(request_id),
                    },
                    Ok(None) => McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Configuration '{config_name}' not found"),
                            data: None,
                        }),
                        id: Some(request_id),
                    },
                    Err(e) => {
                        error!("Error getting tenant fitness config: {}", e);
                        McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_owned(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: "Database error".to_owned(),
                                data: None,
                            }),
                            id: Some(request_id),
                        }
                    }
                }
            }
            Err(e) => {
                error!("Error getting user fitness config: {}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Database error".to_owned(),
                        data: None,
                    }),
                    id: Some(request_id),
                }
            }
        }
    }

    async fn handle_set_fitness_config(
        args: serde_json::Value,
        request_id: serde_json::Value,
        tenant_id: &str,
        user_id: &str,
        database: &Database,
    ) -> McpResponse {
        let params = match serde_json::from_value::<json_schemas::SetFitnessConfigParams>(args) {
            Ok(p) => p,
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid parameters: {e}"),
                        data: None,
                    }),
                    id: Some(request_id),
                };
            }
        };

        let config_name = &params.configuration_name;

        let configuration = match serde_json::from_value::<FitnessConfig>(params.configuration) {
            Ok(fc) => fc,
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid configuration format: {e}"),
                        data: None,
                    }),
                    id: Some(request_id),
                };
            }
        };

        match database
            .save_user_fitness_config(tenant_id, user_id, config_name, &configuration)
            .await
        {
            Ok(config_id) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: Some(json!({
                    "configuration_id": config_id,
                    "configuration_name": config_name,
                    "message": "Fitness configuration saved successfully"
                })),
                error: None,
                id: Some(request_id),
            },
            Err(e) => {
                error!("Error saving fitness config: {}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Failed to save configuration".to_owned(),
                        data: None,
                    }),
                    id: Some(request_id),
                }
            }
        }
    }

    async fn handle_list_fitness_configs(
        request_id: serde_json::Value,
        tenant_id: &str,
        user_id: &str,
        database: &Database,
    ) -> McpResponse {
        let user_configs = database
            .list_user_fitness_configurations(tenant_id, user_id)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    tenant_id = %tenant_id,
                    user_id = %user_id,
                    error = %e,
                    "Failed to fetch user fitness configurations, using empty list"
                );
                Vec::new()
            });
        let tenant_configs = database
            .list_tenant_fitness_configurations(tenant_id)
            .await
            .unwrap_or_else(|e| {
                warn!(
                    tenant_id = %tenant_id,
                    error = %e,
                    "Failed to fetch tenant fitness configurations, using empty list"
                );
                Vec::new()
            });

        let mut all_configs = user_configs;
        all_configs.extend(tenant_configs);
        all_configs.sort();
        all_configs.dedup();

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: Some(json!({
                "configurations": all_configs,
                "total_count": all_configs.len()
            })),
            error: None,
            id: Some(request_id),
        }
    }

    async fn handle_delete_fitness_config(
        args: serde_json::Value,
        request_id: serde_json::Value,
        tenant_id: &str,
        user_id: &str,
        database: &Database,
    ) -> McpResponse {
        let params = match serde_json::from_value::<json_schemas::DeleteFitnessConfigParams>(args) {
            Ok(p) => p,
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid parameters: {e}"),
                        data: None,
                    }),
                    id: Some(request_id),
                };
            }
        };

        let config_name = &params.configuration_name;

        match database
            .delete_fitness_config(tenant_id, Some(user_id), config_name)
            .await
        {
            Ok(true) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: Some(json!({
                    "configuration_name": config_name,
                    "message": "Fitness configuration deleted successfully"
                })),
                error: None,
                id: Some(request_id),
            },
            Ok(false) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INVALID_PARAMS,
                    message: format!("Configuration '{config_name}' not found"),
                    data: None,
                }),
                id: Some(request_id),
            },
            Err(e) => {
                error!("Error deleting fitness config: {}", e);
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Failed to delete configuration".to_owned(),
                        data: None,
                    }),
                    id: Some(request_id),
                }
            }
        }
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
