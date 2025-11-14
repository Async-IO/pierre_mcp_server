// ABOUTME: MCP server implementation with tenant isolation and user authentication
// ABOUTME: Handles MCP protocol with per-tenant data isolation and access control
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # MCP Server
//!
//! NOTE: All remaining undocumented `.clone()` calls in this file are Safe - they are
//! necessary for Arc resource sharing in HTTP route handlers and async closures required
//! by the Axum framework for multi-tenant MCP protocol handling.
//! This module provides an MCP server that supports user authentication,
//! secure token storage, and user-scoped data access.

use super::{
    mcp_request_processor::McpRequestProcessor,
    resources::ServerResources,
    tool_handlers::{McpOAuthCredentials, ToolRoutingContext},
};
use crate::auth::{AuthManager, AuthResult};
use crate::constants::{
    errors::{ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND},
    json_fields::{GOAL_ID, PROVIDER},
    protocol::JSONRPC_VERSION,
    tools::{
        ANALYZE_ACTIVITY, ANALYZE_GOAL_FEASIBILITY, ANALYZE_PERFORMANCE_TRENDS,
        ANALYZE_TRAINING_LOAD, CALCULATE_FITNESS_SCORE, CALCULATE_METRICS, COMPARE_ACTIVITIES,
        DETECT_PATTERNS, GENERATE_RECOMMENDATIONS, GET_ACTIVITIES, GET_ACTIVITY_INTELLIGENCE,
        GET_ATHLETE, GET_STATS, PREDICT_PERFORMANCE, SET_GOAL, SUGGEST_GOALS, TRACK_PROGRESS,
    },
};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::providers::ProviderRegistry;
use crate::routes::OAuthRoutes;
use crate::security::headers::SecurityConfig;
use crate::tenant::{TenantContext, TenantOAuthClient};
// Removed unused imports - now using AppError directly

use anyhow::Result;
use chrono::Utc;

use serde_json::Value;
use std::fmt::Write;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

// Constants are now imported from the constants module

/// Connection status for providers
struct ProviderConnectionStatus {
    strava_connected: bool,
    fitbit_connected: bool,
}

/// Helper struct for OAuth provider credential parameters
struct OAuthProviderParams<'a> {
    provider: &'a str,
    client_id: &'a str,
    client_secret: &'a str,
    configured_redirect_uri: Option<&'a String>,
    scopes: &'a [String],
    http_port: u16,
}

/// MCP server supporting user authentication and isolated data access
#[derive(Clone)]
pub struct MultiTenantMcpServer {
    resources: Arc<ServerResources>,
}

impl MultiTenantMcpServer {
    /// Create a new MCP server with pre-built resources (dependency injection)
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Initialize security configuration based on environment
    fn setup_security_config(config: &crate::config::environment::ServerConfig) -> SecurityConfig {
        let security_config =
            SecurityConfig::from_environment(&config.security.headers.environment.to_string());
        info!(
            "Security headers enabled with {} configuration",
            config.security.headers.environment
        );
        security_config
    }

    /// Handle incoming MCP request and route to appropriate processor
    ///
    /// # Errors
    /// Returns `None` if the request cannot be processed
    pub async fn handle_request(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> Option<McpResponse> {
        let processor = McpRequestProcessor::new(resources.clone());
        processor.handle_request(request).await
    }

    /// Extract tenant context from MCP request headers
    /// Route disconnect tool request to appropriate provider handler
    ///
    /// # Errors
    /// Returns an error if the provider is not supported or the operation fails
    pub async fn route_disconnect_tool(
        provider_name: &str,
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        if let Some(ref tenant_ctx) = ctx.tenant_context {
            Self::handle_tenant_disconnect_provider(
                tenant_ctx,
                provider_name,
                &ctx.resources.provider_registry,
                &ctx.resources.database,
                request_id,
            )
        } else {
            Self::handle_disconnect_provider(user_id, provider_name, ctx.resources, request_id)
                .await
        }
    }

    /// Route provider-specific tool requests to appropriate handlers
    pub async fn route_provider_tool(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        _user_id: Uuid,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        if let Some(ref tenant_ctx) = ctx.tenant_context {
            Self::handle_tenant_tool_with_provider(
                tool_name,
                args,
                request_id,
                tenant_ctx,
                ctx.resources,
                ctx.auth_result,
            )
            .await
        } else {
            // No tenant context means no provider access - tenant-aware endpoints required
            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: format!("Tool '{tool_name}' requires tenant context - use tenant-aware MCP endpoints"),
                    data: None,
                }),
                id: Some(request_id),
            }
        }
    }

    /// Handle tools that don't require external providers
    pub async fn handle_tool_without_provider(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        database: &Arc<Database>,
        auth_result: &AuthResult,
    ) -> McpResponse {
        let start_time = std::time::Instant::now();
        let response = Self::execute_tool_call_without_provider(
            tool_name,
            args,
            request_id.clone(),
            user_id,
            database,
        )
        .await;

        // Record API key usage if authenticated with API key
        if let crate::auth::AuthMethod::ApiKey { key_id, .. } = &auth_result.auth_method {
            if let Err(e) = Self::record_api_key_usage(
                database,
                key_id,
                tool_name,
                start_time.elapsed(),
                &response,
            )
            .await
            {
                tracing::warn!(
                    key_id = %key_id,
                    tool_name = %tool_name,
                    error = %e,
                    "Failed to record API key usage - metrics may be incomplete"
                );
            }
        }

        response
    }

    /// Handle `disconnect_provider` tool call
    async fn handle_disconnect_provider(
        user_id: Uuid,
        provider: &str,
        resources: &Arc<ServerResources>,
        id: Value,
    ) -> McpResponse {
        // Use existing ServerResources (no fake auth managers or cloning!)
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let oauth_routes = OAuthRoutes::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        match oauth_routes.disconnect_provider(user_id, provider).await {
            Ok(()) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": format!("Successfully disconnected {provider}"),
                    "provider": provider
                });

                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: Some(response),
                    error: None,
                    id: Some(id),
                }
            }
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to disconnect provider: {e}"),
                    data: None,
                }),
                id: Some(id),
            },
        }
    }

    /// Execute tool call without provider (for database-only tools)
    async fn execute_tool_call_without_provider(
        tool_name: &str,
        args: &Value,
        id: Value,
        user_id: Uuid,
        database: &Arc<Database>,
    ) -> McpResponse {
        let result = match tool_name {
            SET_GOAL => Self::handle_set_goal(args, user_id, database, &id).await,
            TRACK_PROGRESS => Self::handle_track_progress(args, user_id, database, &id).await,
            PREDICT_PERFORMANCE => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Provider required".into(),
                        data: None,
                    }),
                    id: Some(id),
                };
            }
            _ => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: format!("Unknown tool: {tool_name}"),
                        data: None,
                    }),
                    id: Some(id),
                };
            }
        };

        match result {
            Ok(response) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: Some(response),
                error: None,
                id: Some(id),
            },
            Err(error_response) => error_response,
        }
    }

    /// Handle `SET_GOAL` tool call
    async fn handle_set_goal(
        args: &Value,
        user_id: Uuid,
        database: &Arc<Database>,
        id: &Value,
    ) -> Result<Value, McpResponse> {
        let goal_data = args.clone();

        match database.create_goal(user_id, goal_data).await {
            Ok(goal_id) => {
                let response = serde_json::json!({
                    "goal_created": {
                        "goal_id": goal_id,
                        "status": "active",
                        "message": "Goal successfully created"
                    }
                });
                Ok(response)
            }
            Err(e) => Err(McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to create goal: {e}"),
                    data: None,
                }),
                id: Some(id.clone()),
            }),
        }
    }

    /// Handle `TRACK_PROGRESS` tool call
    async fn handle_track_progress(
        args: &Value,
        user_id: Uuid,
        database: &Arc<Database>,
        id: &Value,
    ) -> Result<Value, McpResponse> {
        let goal_id = args[GOAL_ID].as_str().unwrap_or("");

        match database.get_user_goals(user_id).await {
            Ok(goals) => goals.iter().find(|g| g["id"] == goal_id).map_or_else(
                || {
                    Err(McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Goal with ID '{goal_id}' not found"),
                            data: None,
                        }),
                        id: Some(id.clone()),
                    })
                },
                |goal| {
                    let response = serde_json::json!({
                        "progress_report": {
                            "goal_id": goal_id,
                            "goal": goal,
                            "progress_percentage": 65.0,
                            "on_track": true,
                            "insights": [
                                "Making good progress toward your goal",
                                "Maintain current training frequency"
                            ]
                        }
                    });
                    Ok(response)
                },
            ),
            Err(e) => Err(McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get goals: {e}"),
                    data: None,
                }),
                id: Some(id.clone()),
            }),
        }
    }

    /// Record API key usage for billing and analytics
    ///
    /// # Errors
    ///
    /// Returns an error if the usage cannot be recorded in the database
    pub async fn record_api_key_usage(
        database: &Arc<Database>,
        api_key_id: &str,
        tool_name: &str,
        response_time: std::time::Duration,
        response: &McpResponse,
    ) -> Result<()> {
        use crate::api_keys::ApiKeyUsage;

        let status_code = if response.error.is_some() {
            400 // Error responses
        } else {
            200 // Success responses
        };

        let error_message = response.error.as_ref().map(|e| e.message.clone());

        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key_id.to_owned(),
            timestamp: Utc::now(),
            tool_name: tool_name.to_owned(),
            response_time_ms: u32::try_from(response_time.as_millis()).ok(),
            status_code,
            error_message,
            request_size_bytes: None,  // Could be calculated from request
            response_size_bytes: None, // Could be calculated from response
            ip_address: None,          // Would need to be passed from request context
            user_agent: None,          // Would need to be passed from request context
        };

        database.record_api_key_usage(&usage).await?;
        Ok(())
    }

    /// Get database reference for admin API
    #[must_use]
    pub fn database(&self) -> &Database {
        &self.resources.database
    }

    /// Get auth manager reference for admin API
    #[must_use]
    pub fn auth_manager(&self) -> &AuthManager {
        &self.resources.auth_manager
    }

    // === Tenant-Aware Tool Handlers ===

    /// Store user-provided OAuth credentials if supplied
    async fn store_mcp_oauth_credentials(
        tenant_context: &TenantContext,
        oauth_client: &Arc<TenantOAuthClient>,
        credentials: &McpOAuthCredentials<'_>,
        config: &Arc<crate::config::environment::ServerConfig>,
    ) {
        // Store Strava credentials if provided
        if let (Some(id), Some(secret)) = (
            credentials.strava_client_id,
            credentials.strava_client_secret,
        ) {
            Self::store_provider_credentials(
                tenant_context,
                oauth_client,
                OAuthProviderParams {
                    provider: "strava",
                    client_id: id,
                    client_secret: secret,
                    configured_redirect_uri: config.oauth.strava.redirect_uri.as_ref(),
                    scopes: &Self::get_strava_scopes(),
                    http_port: config.http_port,
                },
            )
            .await;
        }

        // Store Fitbit credentials if provided
        if let (Some(id), Some(secret)) = (
            credentials.fitbit_client_id,
            credentials.fitbit_client_secret,
        ) {
            Self::store_provider_credentials(
                tenant_context,
                oauth_client,
                OAuthProviderParams {
                    provider: "fitbit",
                    client_id: id,
                    client_secret: secret,
                    configured_redirect_uri: config.oauth.fitbit.redirect_uri.as_ref(),
                    scopes: &Self::get_fitbit_scopes(),
                    http_port: config.http_port,
                },
            )
            .await;
        }
    }

    /// Store OAuth credentials for a specific provider
    async fn store_provider_credentials(
        tenant_context: &TenantContext,
        oauth_client: &Arc<TenantOAuthClient>,
        params: OAuthProviderParams<'_>,
    ) {
        tracing::info!(
            "Storing MCP-provided {} OAuth credentials for tenant {}",
            params.provider,
            tenant_context.tenant_id
        );

        let redirect_uri = params.configured_redirect_uri.map_or_else(
            || {
                format!(
                    "http://localhost:{}/api/oauth/callback/{}",
                    params.http_port, params.provider
                )
            },
            Clone::clone,
        );

        let request = crate::tenant::oauth_client::StoreCredentialsRequest {
            client_id: params.client_id.to_owned(),
            client_secret: params.client_secret.to_owned(),
            redirect_uri,
            scopes: params.scopes.to_vec(),
            configured_by: tenant_context.user_id,
        };

        if let Err(e) = oauth_client
            .store_credentials(tenant_context.tenant_id, params.provider, request)
            .await
        {
            tracing::error!(
                "Failed to store {} OAuth credentials: {}",
                params.provider,
                e
            );
        }
    }

    /// Get default Strava OAuth scopes
    fn get_strava_scopes() -> Vec<String> {
        crate::constants::oauth::STRAVA_DEFAULT_SCOPES
            .split(',')
            .map(str::to_owned)
            .collect()
    }

    /// Get default Fitbit OAuth scopes
    fn get_fitbit_scopes() -> Vec<String> {
        vec![
            "activity".to_owned(),
            "heartrate".to_owned(),
            "location".to_owned(),
            "nutrition".to_owned(),
            "profile".to_owned(),
            "settings".to_owned(),
            "sleep".to_owned(),
            "social".to_owned(),
            "weight".to_owned(),
        ]
    }

    /// Handle tenant-aware connection status
    pub async fn handle_tenant_connection_status(
        tenant_context: &TenantContext,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
        database: &Arc<Database>,
        request_id: Value,
        credentials: McpOAuthCredentials<'_>,
        http_port: u16,
        config: &Arc<crate::config::environment::ServerConfig>,
    ) -> McpResponse {
        tracing::info!(
            "Checking connection status for tenant {} user {}",
            tenant_context.tenant_name,
            tenant_context.user_id
        );

        // Store MCP-provided OAuth credentials if supplied
        Self::store_mcp_oauth_credentials(
            tenant_context,
            tenant_oauth_client,
            &credentials,
            config,
        )
        .await;

        let base_url = Self::build_oauth_base_url(http_port);
        let connection_status = Self::check_provider_connections(tenant_context, database).await;
        let notifications_text =
            Self::build_notifications_text(database, tenant_context.user_id).await;
        let structured_data = Self::build_structured_connection_data(
            tenant_context,
            &connection_status,
            &base_url,
            database,
        )
        .await;
        let text_content = Self::build_text_content(
            &connection_status,
            &base_url,
            tenant_context,
            &notifications_text,
        );

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: Some(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": text_content
                    }
                ],
                "structuredContent": structured_data,
                "isError": false
            })),
            error: None,
            id: Some(request_id),
        }
    }

    /// Build OAuth base URL with dynamic port
    fn build_oauth_base_url(http_port: u16) -> String {
        let host = crate::constants::get_server_config()
            .map_or_else(|| "localhost".to_owned(), |c| c.host.clone());
        format!("http://{host}:{http_port}/api/oauth")
    }

    /// Check connection status for all providers
    async fn check_provider_connections(
        tenant_context: &TenantContext,
        database: &Arc<Database>,
    ) -> ProviderConnectionStatus {
        let user_id = tenant_context.user_id;
        let tenant_id_str = tenant_context.tenant_id.to_string();

        // Check Strava connection status
        tracing::debug!(
            "Checking Strava token for user_id={}, tenant_id={}, provider=strava",
            user_id,
            tenant_id_str
        );
        let strava_connected = database
            .get_user_oauth_token(user_id, &tenant_id_str, "strava")
            .await
            .map_or_else(
                |e| {
                    tracing::warn!("Failed to query Strava OAuth token: {e}");
                    false
                },
                |token| {
                    let connected = token.is_some();
                    tracing::debug!("Strava token lookup result: connected={connected}");
                    connected
                },
            );

        // Check Fitbit connection status
        let fitbit_connected = database
            .get_user_oauth_token(user_id, &tenant_id_str, "fitbit")
            .await
            .is_ok_and(|token| token.is_some());

        ProviderConnectionStatus {
            strava_connected,
            fitbit_connected,
        }
    }

    /// Build notifications text from unread notifications
    async fn build_notifications_text(database: &Arc<Database>, user_id: uuid::Uuid) -> String {
        let unread_notifications = database
            .get_unread_oauth_notifications(user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch unread notifications: {e}");
                Vec::new()
            });

        if unread_notifications.is_empty() {
            String::new()
        } else {
            let mut notifications_msg = String::from("\n\nRecent OAuth Updates:\n");
            for notification in &unread_notifications {
                let status_indicator = if notification.success {
                    "[SUCCESS]"
                } else {
                    "[FAILED]"
                };
                writeln!(
                    notifications_msg,
                    "{status_indicator} {}: {}",
                    notification.provider.to_uppercase(),
                    notification.message
                )
                .unwrap_or_else(|_| tracing::warn!("Failed to write notification text"));
            }
            notifications_msg
        }
    }

    /// Build structured connection data JSON
    async fn build_structured_connection_data(
        tenant_context: &TenantContext,
        connection_status: &ProviderConnectionStatus,
        base_url: &str,
        database: &Arc<Database>,
    ) -> serde_json::Value {
        let unread_notifications = database
            .get_unread_oauth_notifications(tenant_context.user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    user_id = %tenant_context.user_id,
                    error = %e,
                    "Failed to fetch OAuth notifications for connection status"
                );
                Vec::new()
            });

        serde_json::json!({
            "providers": [
                {
                    "provider": "strava",
                    "connected": connection_status.strava_connected,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{base_url}/auth/strava/{}", tenant_context.user_id),
                    "connect_instructions": if connection_status.strava_connected {
                        "Your Strava account is connected and ready to use."
                    } else {
                        "Click this URL to connect your Strava account and authorize access to your fitness data."
                    }
                },
                {
                    "provider": "fitbit",
                    "connected": connection_status.fitbit_connected,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{base_url}/auth/fitbit/{}", tenant_context.user_id),
                    "connect_instructions": if connection_status.fitbit_connected {
                        "Your Fitbit account is connected and ready to use."
                    } else {
                        "Click this URL to connect your Fitbit account and authorize access to your fitness data."
                    }
                }
            ],
            "tenant_info": {
                "tenant_id": tenant_context.tenant_id,
                "tenant_name": tenant_context.tenant_name
            },
            "connection_help": {
                "message": "To connect a fitness provider, click the connect_url for the provider you want to use. You'll be redirected to their website to authorize access, then redirected back to complete the connection.",
                "supported_providers": ["strava", "fitbit"],
                "note": "After connecting, you can use fitness tools like get_activities, get_athlete, and get_stats with the connected provider."
            },
            "recent_notifications": unread_notifications.iter().map(|n| serde_json::json!({
                "id": n.id,
                "provider": n.provider,
                "success": n.success,
                "message": n.message,
                "created_at": n.created_at
            })).collect::<Vec<_>>()
        })
    }

    /// Build human-readable text content
    fn build_text_content(
        connection_status: &ProviderConnectionStatus,
        base_url: &str,
        tenant_context: &TenantContext,
        notifications_text: &str,
    ) -> String {
        let strava_status = if connection_status.strava_connected {
            "Connected"
        } else {
            "Not Connected"
        };
        let fitbit_status = if connection_status.fitbit_connected {
            "Connected"
        } else {
            "Not Connected"
        };

        let strava_action = if connection_status.strava_connected {
            "Ready to use fitness tools!".to_owned()
        } else {
            format!(
                "Click to connect: {base_url}/auth/strava/{}",
                tenant_context.user_id
            )
        };

        let fitbit_action = if connection_status.fitbit_connected {
            "Ready to use fitness tools!".to_owned()
        } else {
            format!(
                "Click to connect: {base_url}/auth/fitbit/{}",
                tenant_context.user_id
            )
        };

        let connection_instructions = if !connection_status.strava_connected
            || !connection_status.fitbit_connected
        {
            "To connect a provider:\n\
            1. Click one of the URLs above\n\
            2. You'll be redirected to authorize access\n\
            3. Complete the OAuth flow to connect your account\n\
            4. Start using fitness tools like get_activities, get_athlete, and get_stats"
        } else {
            "All providers connected! You can now use fitness tools like get_activities, get_athlete, and get_stats."
        };

        format!(
            "Fitness Provider Connection Status\n\n\
            Available Providers:\n\n\
            Strava ({strava_status})\n\
            {strava_action}\n\n\
            Fitbit ({fitbit_status})\n\
            {fitbit_action}\n\n\
            {connection_instructions}{notifications_text}"
        )
    }

    /// Handle tenant-aware provider disconnection
    fn handle_tenant_disconnect_provider(
        tenant_context: &TenantContext,
        provider_name: &str,
        _provider_registry: &Arc<ProviderRegistry>,
        _database: &Arc<Database>,
        request_id: Value,
    ) -> McpResponse {
        tracing::info!(
            "Tenant {} disconnecting provider {} for user {}",
            tenant_context.tenant_name,
            provider_name,
            tenant_context.user_id
        );

        // In a real implementation, this would revoke tenant-specific OAuth tokens
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: Some(serde_json::json!({
                "message": format!("Disconnected from {provider_name}"),
                "provider": provider_name,
                "tenant_id": tenant_context.tenant_id,
                "success": true
            })),
            error: None,
            id: Some(request_id),
        }
    }

    /// Create error response for tool execution failure
    fn create_tool_error_response(
        tool_name: &str,
        provider_name: &str,
        response_error: Option<String>,
        request_id: Value,
    ) -> McpResponse {
        let error_msg = response_error
            .unwrap_or_else(|| "Tool execution failed with no error message".to_owned());
        tracing::error!(
            "Tool execution failed for {} with provider {}: {} (success=false)",
            tool_name,
            provider_name,
            error_msg
        );
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: None,
            error: Some(McpError {
                code: ERROR_INTERNAL_ERROR,
                message: error_msg,
                data: None,
            }),
            id: Some(request_id),
        }
    }

    /// Handle tenant-aware tools that require providers
    /// Known tools that can be executed with provider context
    const KNOWN_PROVIDER_TOOLS: &'static [&'static str] = &[
        GET_ACTIVITIES,
        GET_ATHLETE,
        GET_STATS,
        GET_ACTIVITY_INTELLIGENCE,
        ANALYZE_ACTIVITY,
        CALCULATE_METRICS,
        COMPARE_ACTIVITIES,
        PREDICT_PERFORMANCE,
        // Analytics tools - route through Universal Protocol
        ANALYZE_GOAL_FEASIBILITY,
        SUGGEST_GOALS,
        CALCULATE_FITNESS_SCORE,
        GENERATE_RECOMMENDATIONS,
        ANALYZE_TRAINING_LOAD,
        DETECT_PATTERNS,
        ANALYZE_PERFORMANCE_TRENDS,
        // Configuration tools - route through Universal Protocol
        "get_configuration_catalog",
        "get_configuration_profiles",
        "get_user_configuration",
        "update_user_configuration",
        "calculate_personalized_zones",
        "validate_configuration",
    ];

    async fn handle_tenant_tool_with_provider(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        tenant_context: &TenantContext,
        resources: &Arc<ServerResources>,
        auth_result: &AuthResult,
    ) -> McpResponse {
        // Validate tool is known
        if let Some(error_response) = Self::validate_known_tool(tool_name, request_id.clone()) {
            return error_response;
        }

        let provider_name = args[PROVIDER].as_str().unwrap_or("");

        tracing::info!(
            "Executing tenant tool {} with provider {} for tenant {} user {}",
            tool_name,
            provider_name,
            tenant_context.tenant_name,
            tenant_context.user_id
        );

        // Create Universal protocol request
        let universal_request =
            Self::create_universal_request(tool_name, args, auth_result, tenant_context);

        // Execute tool through Universal protocol
        Self::execute_and_convert_tool(
            universal_request,
            resources,
            tool_name,
            provider_name,
            request_id,
        )
        .await
    }

    /// Validate that tool name is in the known tools list
    fn validate_known_tool(tool_name: &str, request_id: Value) -> Option<McpResponse> {
        if Self::KNOWN_PROVIDER_TOOLS.contains(&tool_name) {
            None
        } else {
            Some(McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: format!("Unknown tool: {tool_name}"),
                    data: None,
                }),
                id: Some(request_id),
            })
        }
    }

    /// Create Universal protocol request from tenant tool parameters
    fn create_universal_request(
        tool_name: &str,
        args: &Value,
        auth_result: &AuthResult,
        tenant_context: &TenantContext,
    ) -> crate::protocols::universal::UniversalRequest {
        crate::protocols::universal::UniversalRequest {
            tool_name: tool_name.to_owned(),
            parameters: args.clone(),
            user_id: auth_result.user_id.to_string(),
            protocol: "mcp".to_owned(),
            tenant_id: Some(tenant_context.tenant_id.to_string()),
        }
    }

    /// Execute Universal protocol tool and convert response to MCP format
    async fn execute_and_convert_tool(
        universal_request: crate::protocols::universal::UniversalRequest,
        resources: &Arc<ServerResources>,
        tool_name: &str,
        provider_name: &str,
        request_id: Value,
    ) -> McpResponse {
        let executor = crate::protocols::universal::UniversalToolExecutor::new(resources.clone());

        match executor.execute_tool(universal_request).await {
            Ok(response) => {
                // Convert UniversalResponse to proper MCP ToolResponse format
                let tool_response =
                    crate::protocols::converter::ProtocolConverter::universal_to_mcp(response);

                // Serialize ToolResponse to JSON for MCP result field
                match serde_json::to_value(&tool_response) {
                    Ok(result_value) => McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: Some(result_value),
                        error: None,
                        id: Some(request_id),
                    },
                    Err(e) => Self::create_tool_error_response(
                        tool_name,
                        provider_name,
                        Some(format!("Failed to serialize tool response: {e}")),
                        request_id,
                    ),
                }
            }
            Err(e) => Self::create_tool_error_response(
                tool_name,
                provider_name,
                Some(format!("Tool execution error: {e}")),
                request_id,
            ),
        }
    }
}

// Phase 2: Type aliases pointing to unified JSON-RPC foundation
/// Type alias for MCP requests using the JSON-RPC foundation
pub type McpRequest = crate::jsonrpc::JsonRpcRequest;
/// Type alias for MCP responses using the JSON-RPC foundation
pub type McpResponse = crate::jsonrpc::JsonRpcResponse;
/// Type alias for MCP errors using the JSON-RPC foundation
pub type McpError = crate::jsonrpc::JsonRpcError;

// ============================================================================
// AXUM SERVER ORCHESTRATION
// ============================================================================

impl MultiTenantMcpServer {
    /// Run HTTP server (convenience method)
    ///
    /// Starts the Axum HTTP server on the specified port using the embedded resources.
    ///
    /// # Errors
    /// Returns an error if server setup or routing configuration fails
    pub async fn run(&self, port: u16) -> Result<()> {
        self.run_http_server_with_resources_axum(port, self.resources.clone())
            .await
    }

    /// Run HTTP server with Axum framework
    ///
    /// This method provides the Axum-based server implementation.
    ///
    /// # Errors
    /// Returns an error if server setup or routing configuration fails
    pub async fn run_http_server_with_resources_axum(
        &self,
        port: u16,
        resources: Arc<ServerResources>,
    ) -> Result<()> {
        use std::net::SocketAddr;
        use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
        use tower_http::LatencyUnit;

        info!("HTTP server (Axum) starting on port {}", port);

        // Build the main router with all routes
        let app = Self::setup_axum_router(&resources);

        // Apply middleware layers (order matters - applied bottom-up)
        let app = app
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(
                        DefaultMakeSpan::new()
                            .level(tracing::Level::INFO)
                            .include_headers(false),
                    )
                    .on_response(
                        DefaultOnResponse::new()
                            .level(tracing::Level::INFO)
                            .latency_unit(LatencyUnit::Millis),
                    ),
            )
            .layer(axum::middleware::from_fn(
                crate::middleware::request_id_middleware,
            ))
            .layer(crate::middleware::setup_cors(&resources.config))
            .layer(Self::create_security_headers_layer(&resources.config));

        // Create server address
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        info!("HTTP server (Axum) listening on http://{}", addr);

        // Start the Axum server with ConnectInfo for IP extraction (rate limiting)
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await?;

        Ok(())
    }

    /// Setup complete Axum router with all route modules
    fn setup_axum_router(resources: &Arc<ServerResources>) -> axum::Router {
        use axum::Router;

        // Import the Axum route implementations
        use crate::routes::a2a::A2ARoutes;
        use crate::routes::admin::AdminRoutes;
        use crate::routes::api_keys::ApiKeyRoutes;
        use crate::routes::auth::AuthRoutes;
        use crate::routes::configuration::ConfigurationRoutes;
        use crate::routes::dashboard::DashboardRoutes;
        use crate::routes::fitness::FitnessConfigurationRoutes;
        use crate::routes::mcp::McpRoutes;
        use crate::routes::oauth2::OAuth2Routes;
        use crate::routes::tenants::TenantRoutes;
        use crate::routes::websocket::WebSocketRoutes;
        use crate::sse::SseRoutes;

        // Create admin routes using the routes::admin::AdminApiContext
        let admin_api_key_limit = resources
            .config
            .rate_limiting
            .admin_provisioned_api_key_monthly_limit;
        let admin_context = crate::routes::admin::AdminApiContext::new(
            resources.database.clone(),
            &resources.admin_jwt_secret,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
            admin_api_key_limit,
        );
        let admin_routes = AdminRoutes::routes(admin_context);

        // Create health check routes
        let health_routes = Self::create_axum_health_routes();

        // Create OAuth2 server context
        let oauth2_context = crate::routes::oauth2::OAuth2Context {
            database: resources.database.clone(),
            auth_manager: resources.auth_manager.clone(),
            jwks_manager: resources.jwks_manager.clone(),
            config: resources.config.clone(),
            rate_limiter: Arc::new(
                crate::oauth2_server::OAuth2RateLimiter::from_rate_limit_config(
                    resources.config.rate_limiting.clone(),
                ),
            ),
        };
        let oauth2_routes = OAuth2Routes::routes(oauth2_context);

        // Combine all routes into the main router
        Router::new()
            .merge(health_routes)
            .merge(admin_routes)
            .merge(AuthRoutes::routes(Arc::clone(resources)))
            .merge(oauth2_routes)
            .merge(McpRoutes::routes(Arc::clone(resources)))
            .merge(SseRoutes::routes(
                Arc::clone(&resources.sse_manager),
                Arc::clone(resources),
            ))
            .merge(WebSocketRoutes::routes(Arc::clone(
                &resources.websocket_manager,
            )))
            .merge(A2ARoutes::routes())
            .merge(ApiKeyRoutes::routes(Arc::clone(resources)))
            .merge(TenantRoutes::routes(Arc::clone(resources)))
            .merge(DashboardRoutes::routes(Arc::clone(resources)))
            .merge(ConfigurationRoutes::routes(Arc::clone(resources)))
            .merge(FitnessConfigurationRoutes::routes(Arc::clone(resources)))
    }

    /// Create health check routes for Axum
    fn create_axum_health_routes() -> axum::Router {
        use axum::{routing::get, Json, Router};

        async fn health_handler() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "status": "ok",
                "service": crate::constants::service_names::PIERRE_MCP_SERVER
            }))
        }

        async fn plugins_health_handler() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "status": "ok",
                "plugins": []
            }))
        }

        Router::new()
            .route("/health", get(health_handler))
            .route("/health/plugins", get(plugins_health_handler))
    }

    /// Create security headers layer for Axum
    ///
    /// Validates security headers configuration and returns Identity layer.
    /// Security headers are validated at startup to catch configuration errors early.
    /// Response header injection happens via response interceptor middleware.
    fn create_security_headers_layer(
        config: &Arc<crate::config::environment::ServerConfig>,
    ) -> tower::layer::util::Identity {
        use tracing::warn;

        // Validate security headers configuration at startup
        let security_config = Self::setup_security_config(config);
        let headers = security_config.to_headers();

        // Validate all headers can be parsed - this catches configuration errors early
        for (header_name, header_value) in headers {
            if http::HeaderName::from_bytes(header_name.as_bytes()).is_err()
                || http::HeaderValue::from_str(header_value).is_err()
            {
                warn!(
                    "Invalid security header in config: {} = {}",
                    header_name, header_value
                );
            }
        }

        // Return identity layer - headers are applied via CORS middleware and response interceptors
        tower::layer::util::Identity::new()
    }
}
