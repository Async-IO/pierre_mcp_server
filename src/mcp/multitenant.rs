// ABOUTME: MCP server implementation with tenant isolation and user authentication
// ABOUTME: Handles MCP protocol with per-tenant data isolation and access control
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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
use crate::api_keys::ApiKeyUsage;
use crate::auth::{AuthManager, AuthResult};
use crate::config::environment::ServerConfig;
#[cfg(feature = "provider-strava")]
use crate::constants::oauth::STRAVA_DEFAULT_SCOPES;
use crate::constants::{
    errors::{ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND},
    get_server_config,
    protocol::JSONRPC_VERSION,
};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::jsonrpc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::mcp::schema::ProgressNotification;
use crate::protocols::converter::ProtocolConverter;
use crate::protocols::universal::tool_registry::ToolId;
use crate::protocols::universal::types::{CancellationToken, ProgressReporter};
use crate::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use crate::providers::ProviderRegistry;
use crate::security::headers::SecurityConfig;
use crate::tenant::oauth_client::StoreCredentialsRequest;
use crate::tenant::{TenantContext, TenantOAuthClient};
use crate::types::json_schemas;
use chrono::Utc;
use serde_json::Value;
use std::env;
use std::fmt::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tower_http::LatencyUnit;
use tracing::{debug, error, info, warn, Level};
use uuid::Uuid;

use crate::constants::service_names::PIERRE_MCP_SERVER;
use crate::middleware::{request_id_middleware, setup_cors};
#[cfg(feature = "oauth")]
use crate::oauth2_server::OAuth2RateLimiter;
#[cfg(feature = "client-admin-api")]
use crate::routes::admin::AdminApiContext;
#[cfg(feature = "oauth")]
use crate::routes::oauth2::OAuth2Context;
use axum::middleware;
use tokio::net::TcpListener;
use tower::layer::util::Identity;

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

    /// Get shared reference to server resources
    #[must_use]
    pub fn resources(&self) -> Arc<ServerResources> {
        self.resources.clone()
    }

    /// Initialize security configuration based on environment
    fn setup_security_config(config: &ServerConfig) -> SecurityConfig {
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
    #[tracing::instrument(
        skip(request, resources),
        fields(
            method = %request.method,
            request_id = ?request.id,
        )
    )]
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
    #[tracing::instrument(
        skip(ctx, request_id),
        fields(
            provider = %provider_name,
            user_id = %ctx.tenant_context.user_id,
            tenant_id = %ctx.tenant_context.tenant_id,
        )
    )]
    pub async fn route_disconnect_tool(
        provider_name: &str,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        // Tenant context is always available since tool execution requires it
        Self::handle_tenant_disconnect_provider(
            ctx.tenant_context,
            provider_name,
            &ctx.resources.provider_registry,
            &ctx.resources.database,
            request_id,
        )
    }

    /// Route provider-specific tool requests to appropriate handlers
    ///
    /// Tenant context is always available since tool execution requires it.
    #[tracing::instrument(
        skip(args, request_id, ctx),
        fields(
            tool_name = %tool_name,
            user_id = %ctx.tenant_context.user_id,
            tenant_id = %ctx.tenant_context.tenant_id,
        )
    )]
    pub async fn route_provider_tool(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        // Tenant context is always available since tool execution requires it
        Self::handle_tenant_tool_with_provider(
            tool_name,
            args,
            request_id,
            ctx.tenant_context,
            ctx.resources,
            ctx.auth_result,
        )
        .await
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
        response_time: Duration,
        response: &McpResponse,
    ) -> AppResult<()> {
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

        database
            .record_api_key_usage(&usage)
            .await
            .map_err(|e| AppError::database(format!("Failed to record API key usage: {e}")))?;
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
        config: &Arc<ServerConfig>,
    ) {
        // Store Strava credentials if provided
        #[cfg(feature = "provider-strava")]
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
        info!(
            "Storing MCP-provided {} OAuth credentials for tenant {}",
            params.provider, tenant_context.tenant_id
        );

        let redirect_uri = params.configured_redirect_uri.map_or_else(
            || {
                // Use BASE_URL if set for tunnel/external access
                let base_url = env::var("BASE_URL")
                    .unwrap_or_else(|_| format!("http://localhost:{}", params.http_port));
                format!("{base_url}/api/oauth/callback/{}", params.provider)
            },
            String::clone,
        );

        let request = StoreCredentialsRequest {
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
            error!(
                "Failed to store {} OAuth credentials: {}",
                params.provider, e
            );
        }
    }

    /// Get default Strava OAuth scopes
    #[cfg(feature = "provider-strava")]
    fn get_strava_scopes() -> Vec<String> {
        STRAVA_DEFAULT_SCOPES
            .split(',')
            .map(<str as ToOwned>::to_owned)
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
    #[tracing::instrument(
        skip(tenant_oauth_client, database, request_id, credentials, config),
        fields(
            tenant_id = %tenant_context.tenant_id,
            tenant_name = %tenant_context.tenant_name,
            user_id = %tenant_context.user_id,
        )
    )]
    pub async fn handle_tenant_connection_status(
        tenant_context: &TenantContext,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
        database: &Arc<Database>,
        request_id: Value,
        credentials: McpOAuthCredentials<'_>,
        http_port: u16,
        config: &Arc<ServerConfig>,
    ) -> McpResponse {
        info!(
            "Checking connection status for tenant {} user {}",
            tenant_context.tenant_name, tenant_context.user_id
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

    /// Build OAuth base URL from server config (respects `BASE_URL` scheme for TLS/proxy)
    fn build_oauth_base_url(http_port: u16) -> String {
        let base = get_server_config().map_or_else(
            || format!("http://localhost:{http_port}"),
            |c| c.base_url.clone(),
        );
        format!("{base}/api/oauth")
    }

    /// Check connection status for all providers
    async fn check_provider_connections(
        tenant_context: &TenantContext,
        database: &Arc<Database>,
    ) -> ProviderConnectionStatus {
        let user_id = tenant_context.user_id;
        let tenant_id_str = tenant_context.tenant_id.to_string();

        // Check Strava connection status
        debug!(
            "Checking Strava token for user_id={}, tenant_id={}, provider=strava",
            user_id, tenant_id_str
        );
        let strava_connected = database
            .get_user_oauth_token(user_id, tenant_context.tenant_id, "strava")
            .await
            .map_or_else(
                |e| {
                    warn!("Failed to query Strava OAuth token: {e}");
                    false
                },
                |token| {
                    let connected = token.is_some();
                    debug!("Strava token lookup result: connected={connected}");
                    connected
                },
            );

        // Check Fitbit connection status
        let fitbit_connected = database
            .get_user_oauth_token(user_id, tenant_context.tenant_id, "fitbit")
            .await
            .is_ok_and(|token| token.is_some());

        ProviderConnectionStatus {
            strava_connected,
            fitbit_connected,
        }
    }

    /// Build notifications text from unread notifications
    async fn build_notifications_text(database: &Arc<Database>, user_id: Uuid) -> String {
        let unread_notifications = database
            .get_unread_oauth_notifications(user_id)
            .await
            .unwrap_or_else(|e| {
                warn!("Failed to fetch unread notifications: {e}");
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
                .unwrap_or_else(|_| warn!("Failed to write notification text"));
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
    ) -> Value {
        let unread_notifications = database
            .get_unread_oauth_notifications(tenant_context.user_id)
            .await
            .unwrap_or_else(|e| {
                warn!(
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
            "connection_help": serde_json::to_value(json_schemas::ConnectionHelp {
                message: "To connect a fitness provider, click the connect_url for the provider you want to use. You'll be redirected to their website to authorize access, then redirected back to complete the connection.".to_owned(),
                supported_providers: vec!["strava".to_owned(), "fitbit".to_owned()],
                note: "After connecting, you can use fitness tools like get_activities, get_athlete, and get_stats with the connected provider.".to_owned(),
            }).unwrap_or_else(|_| serde_json::json!({})),
            "recent_notifications": unread_notifications.iter().map(|n| {
                json_schemas::NotificationItem {
                    id: n.id.clone(),
                    provider: n.provider.clone(),
                    success: n.success,
                    message: n.message.clone(),
                    created_at: n.created_at,
                }
            }).collect::<Vec<_>>()
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
        info!(
            "Tenant {} disconnecting provider {} for user {}",
            tenant_context.tenant_name, provider_name, tenant_context.user_id
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
        error!(
            "Tool execution failed for {} with provider {}: {} (success=false)",
            tool_name, provider_name, error_msg
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

    // Tool routing now uses ToolId::from_name() to validate tools
    // All tools registered in ToolId enum are automatically routed through Universal Protocol

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

        let params = match serde_json::from_value::<json_schemas::ProviderParams>(args.clone()) {
            Ok(p) => p,
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid provider parameters: {e}"),
                        data: None,
                    }),
                    id: Some(request_id),
                };
            }
        };
        let provider_name = params.provider.as_deref().unwrap_or("");

        info!(
            "Executing tenant tool {} with provider {} for tenant {} user {}",
            tool_name, provider_name, tenant_context.tenant_name, tenant_context.user_id
        );

        // Create Universal protocol request
        let universal_request = Self::create_universal_request(
            tool_name,
            args,
            auth_result,
            tenant_context,
            resources,
            &request_id,
        );

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

    /// Validate that tool name is registered in the Universal protocol `ToolId` registry
    /// All tools registered in `ToolId` enum are automatically routed through Universal Protocol
    fn validate_known_tool(tool_name: &str, request_id: Value) -> Option<McpResponse> {
        if ToolId::from_name(tool_name).is_some() {
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
        resources: &Arc<ServerResources>,
        request_id: &Value,
    ) -> UniversalRequest {
        // Create progress reporter if notification sender is available
        let progress_reporter = resources
            .progress_notification_sender
            .as_ref()
            .map(|sender| {
                let progress_token = format!("mcp-{request_id}");
                let mut reporter = ProgressReporter::new(progress_token.clone());

                // Set callback to send progress notifications
                let sender_clone = sender.clone();
                reporter.set_callback(move |progress, total, message| {
                    let notification =
                        ProgressNotification::new(progress_token.clone(), progress, total, message);
                    let _ = sender_clone.send(notification);
                });

                reporter
            });

        // Create cancellation token for this operation
        let cancellation_token = Some(CancellationToken::new());

        UniversalRequest {
            tool_name: tool_name.to_owned(),
            parameters: args.clone(),
            user_id: auth_result.user_id.to_string(),
            protocol: "mcp".to_owned(),
            tenant_id: Some(tenant_context.tenant_id.to_string()),
            progress_token: progress_reporter.as_ref().map(|r| r.progress_token.clone()),
            cancellation_token,
            progress_reporter,
        }
    }

    /// Execute Universal protocol tool and convert response to MCP format
    async fn execute_and_convert_tool(
        universal_request: UniversalRequest,
        resources: &Arc<ServerResources>,
        tool_name: &str,
        provider_name: &str,
        request_id: Value,
    ) -> McpResponse {
        // Register cancellation token if present
        if let (Some(progress_token), Some(cancellation_token)) = (
            &universal_request.progress_token,
            &universal_request.cancellation_token,
        ) {
            resources
                .register_cancellation_token(progress_token.clone(), cancellation_token.clone())
                .await;
        }

        let executor = UniversalToolExecutor::new(resources.clone());

        let result = executor.execute_tool(universal_request.clone()).await;

        // Cleanup cancellation token after execution
        if let Some(progress_token) = &universal_request.progress_token {
            resources.cleanup_cancellation_token(progress_token).await;
        }

        match result {
            Ok(response) => {
                // Convert UniversalResponse to proper MCP ToolResponse format
                let tool_response = ProtocolConverter::universal_to_mcp(response);

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
pub type McpRequest = JsonRpcRequest;
/// Type alias for MCP responses using the JSON-RPC foundation
pub type McpResponse = JsonRpcResponse;
/// Type alias for MCP errors using the JSON-RPC foundation
pub type McpError = JsonRpcError;

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
    pub async fn run(&self, port: u16) -> AppResult<()> {
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
    ) -> AppResult<()> {
        info!("HTTP server (Axum) starting on port {}", port);

        // Build the main router with all routes
        let app = Self::setup_axum_router(&resources);

        // Apply middleware layers (order matters - applied bottom-up)
        let app = app
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(
                        DefaultMakeSpan::new()
                            .level(Level::INFO)
                            .include_headers(false),
                    )
                    .on_response(
                        DefaultOnResponse::new()
                            .level(Level::INFO)
                            .latency_unit(LatencyUnit::Millis),
                    ),
            )
            .layer(middleware::from_fn(request_id_middleware))
            .layer(setup_cors(&resources.config))
            .layer(Self::create_security_headers_layer(&resources.config));

        // Create server address using host from config (defaults to localhost, can be 0.0.0.0 for network access)
        let host = &resources.config.host;
        let addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .unwrap_or_else(|_| SocketAddr::from(([127, 0, 0, 1], port)));
        info!("HTTP server (Axum) listening on http://{}", addr);

        // Start the Axum server with ConnectInfo for IP extraction (rate limiting)
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .map_err(|e| AppError::internal(format!("Transport error: {e}")))?;

        Ok(())
    }

    /// Setup complete Axum router with all route modules
    ///
    /// Routes are conditionally compiled based on feature flags to support
    /// modular server configurations. See Cargo.toml for feature definitions.
    ///
    /// Note: This function is intentionally long due to the conditional route
    /// registration pattern. Splitting it would fragment related route setup
    /// logic and make the code harder to follow. Each section is clearly
    /// documented and the structure follows the feature flag hierarchy.
    #[allow(clippy::too_many_lines)]
    fn setup_axum_router(resources: &Arc<ServerResources>) -> axum::Router {
        use axum::{middleware::from_fn_with_state, Router};

        use crate::middleware::csrf_protection_layer;

        // ═══════════════════════════════════════════════════════════════
        // CONDITIONAL IMPORTS - Based on feature flags
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "protocol-a2a")]
        use crate::routes::a2a::A2ARoutes;
        #[cfg(feature = "client-admin-api")]
        use crate::routes::admin::AdminRoutes;
        #[cfg(feature = "client-api-keys")]
        use crate::routes::api_keys::ApiKeyRoutes;
        #[cfg(feature = "protocol-rest")]
        use crate::routes::auth::AuthRoutes;
        #[cfg(feature = "client-chat")]
        use crate::routes::chat::ChatRoutes;
        #[cfg(feature = "client-coaches")]
        use crate::routes::coaches::CoachesRoutes;
        #[cfg(feature = "client-settings")]
        use crate::routes::configuration::ConfigurationRoutes;
        #[cfg(feature = "client-dashboard")]
        use crate::routes::dashboard::DashboardRoutes;
        #[cfg(feature = "client-settings")]
        use crate::routes::fitness::FitnessConfigurationRoutes;
        #[cfg(feature = "client-impersonation")]
        use crate::routes::impersonation::ImpersonationRoutes;
        #[cfg(feature = "client-llm-settings")]
        use crate::routes::llm_settings::LlmSettingsRoutes;
        #[cfg(feature = "protocol-mcp")]
        use crate::routes::mcp::McpRoutes;
        #[cfg(feature = "oauth")]
        use crate::routes::oauth2::OAuth2Routes;
        #[cfg(feature = "openapi")]
        use crate::routes::openapi::OpenApiRoutes;
        #[cfg(feature = "client-tenants")]
        use crate::routes::tenants::TenantRoutes;
        #[cfg(feature = "client-mcp-tokens")]
        use crate::routes::user_mcp_tokens::UserMcpTokenRoutes;
        #[cfg(feature = "client-oauth-apps")]
        use crate::routes::user_oauth_apps::UserOAuthAppRoutes;
        #[cfg(feature = "client-admin-ui")]
        use crate::routes::web_admin::WebAdminRoutes;
        #[cfg(feature = "transport-websocket")]
        use crate::routes::websocket::WebSocketRoutes;
        #[cfg(feature = "transport-sse")]
        use crate::sse::SseRoutes;

        #[cfg(feature = "client-admin-api")]
        use crate::config::routes::{admin_config_router, AdminConfigState};

        // ═══════════════════════════════════════════════════════════════
        // HEALTH ROUTES - Always enabled
        // ═══════════════════════════════════════════════════════════════

        let health_routes = Self::create_axum_health_routes();
        let app = Router::new().merge(health_routes);

        // ═══════════════════════════════════════════════════════════════
        // CLIENT-ADMIN-API ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "client-admin-api")]
        let app = {
            let admin_api_key_limit = resources
                .config
                .rate_limiting
                .admin_provisioned_api_key_monthly_limit;
            let admin_token_cache_ttl = resources.config.auth.admin_token_cache_ttl_secs;
            let admin_context = AdminApiContext::new(
                resources.database.clone(),
                &resources.admin_jwt_secret,
                resources.auth_manager.clone(),
                resources.jwks_manager.clone(),
                admin_api_key_limit,
                admin_token_cache_ttl,
                resources.tool_selection.clone(),
            );
            let admin_routes = AdminRoutes::routes(admin_context);

            let admin_config_routes = resources.admin_config.as_ref().map_or_else(
                || {
                    tracing::warn!(
                        "Admin config service not available - admin config API disabled"
                    );
                    Router::new()
                },
                |admin_config| {
                    let admin_config_state = Arc::new(AdminConfigState::new(
                        Arc::clone(admin_config),
                        Arc::clone(resources),
                    ));
                    admin_config_router(admin_config_state)
                },
            );

            app.merge(admin_routes)
                .nest("/api/admin/config", admin_config_routes)
        };

        // ═══════════════════════════════════════════════════════════════
        // PROTOCOL ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "protocol-rest")]
        let app = app.merge(AuthRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "oauth")]
        let app = {
            let oauth2_context = OAuth2Context {
                database: resources.database.clone(),
                auth_manager: resources.auth_manager.clone(),
                jwks_manager: resources.jwks_manager.clone(),
                config: resources.config.clone(),
                rate_limiter: Arc::new(OAuth2RateLimiter::from_rate_limit_config(
                    resources.config.rate_limiting.clone(),
                )),
            };
            app.merge(OAuth2Routes::routes(oauth2_context))
        };

        #[cfg(feature = "protocol-mcp")]
        let app = app.merge(McpRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "protocol-a2a")]
        let app = app.merge(A2ARoutes::routes(Arc::clone(resources)));

        // ═══════════════════════════════════════════════════════════════
        // TRANSPORT ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "transport-sse")]
        let app = app.merge(SseRoutes::routes(
            Arc::clone(&resources.sse_manager),
            Arc::clone(resources),
        ));

        #[cfg(feature = "transport-websocket")]
        let app = app.merge(WebSocketRoutes::routes(Arc::clone(
            &resources.websocket_manager,
        )));

        // ═══════════════════════════════════════════════════════════════
        // CLIENT-WEB ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "client-dashboard")]
        let app = app.merge(DashboardRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-settings")]
        let app = app
            .merge(ConfigurationRoutes::routes(Arc::clone(resources)))
            .merge(FitnessConfigurationRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-chat")]
        let app = app.merge(ChatRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-coaches")]
        let app = app
            .merge(CoachesRoutes::routes(Arc::clone(resources)))
            .nest(
                "/api/admin",
                CoachesRoutes::admin_routes(Arc::clone(resources)),
            );

        #[cfg(feature = "client-store")]
        let app = {
            use crate::routes::StoreRoutes;
            app.merge(StoreRoutes::router(resources))
        };

        #[cfg(feature = "client-oauth-apps")]
        let app = app.merge(UserOAuthAppRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-social")]
        let app = {
            use crate::routes::SocialRoutes;
            app.merge(SocialRoutes::routes(Arc::clone(resources)))
        };

        // ═══════════════════════════════════════════════════════════════
        // CLIENT-ADMIN ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "client-admin-ui")]
        let app = app.merge(WebAdminRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-api-keys")]
        let app = app.merge(ApiKeyRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-tenants")]
        let app = app.merge(TenantRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-impersonation")]
        let app = app.merge(ImpersonationRoutes::routes(Arc::clone(resources)));

        #[cfg(feature = "client-llm-settings")]
        let app = app.merge(LlmSettingsRoutes::routes(Arc::clone(resources)));

        // ═══════════════════════════════════════════════════════════════
        // OTHER CLIENT ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "client-mcp-tokens")]
        let app = app.merge(UserMcpTokenRoutes::routes(Arc::clone(resources)));

        // ═══════════════════════════════════════════════════════════════
        // OPENAPI DOCUMENTATION ROUTES
        // ═══════════════════════════════════════════════════════════════

        #[cfg(feature = "openapi")]
        let app = app.merge(OpenApiRoutes::routes());

        // ═══════════════════════════════════════════════════════════════
        // CSRF PROTECTION LAYER
        // ═══════════════════════════════════════════════════════════════
        // Applied globally but only activates for cookie-authenticated
        // state-changing requests (POST/PUT/DELETE/PATCH). Bearer token
        // and API key requests pass through without CSRF validation.
        app.layer(from_fn_with_state(
            Arc::clone(resources),
            csrf_protection_layer,
        ))
    }

    /// Create health check routes for Axum
    fn create_axum_health_routes() -> axum::Router {
        use axum::{routing::get, Json, Router};

        async fn health_handler() -> Json<serde_json::Value> {
            Json(serde_json::json!({
                "status": "ok",
                "service": PIERRE_MCP_SERVER
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
    fn create_security_headers_layer(config: &Arc<ServerConfig>) -> Identity {
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
        Identity::new()
    }
}
