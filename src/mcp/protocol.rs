// ABOUTME: MCP protocol message handlers for core protocol operations
// ABOUTME: Handles initialize, ping, tools/list, and authentication protocol messages
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # MCP Protocol Handlers
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - JSON value ownership for MCP protocol message serialization
// - Resource Arc sharing for concurrent protocol message processing
//!
//! Core MCP protocol message handling for initialization, tools listing,
//! and authentication operations.

use crate::auth::AuthManager;
use crate::constants::{
    errors::{
        ERROR_AUTHENTICATION, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND, ERROR_SERIALIZATION,
        ERROR_VERSION_MISMATCH, MSG_AUTHENTICATION, MSG_SERIALIZATION, MSG_VERSION_MISMATCH,
    },
    protocol::SERVER_VERSION,
};
use crate::database_plugins::DatabaseProvider;
use crate::mcp::resources::ServerResources;
use crate::mcp::schema::{get_tools, InitializeRequest, InitializeResponse};
use crate::models::AuthRequest;
use crate::types::json_schemas;
use serde_json::Value;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// MCP protocol handlers
pub struct ProtocolHandler;

// Re-export types from multitenant module to avoid duplication
pub use super::multitenant::{McpError, McpRequest, McpResponse};

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    serde_json::Value::Number(serde_json::Number::from(0))
}

/// Extract and validate user ID from auth token
fn extract_user_id_from_token(
    auth_token: Option<&String>,
    resources: &Arc<ServerResources>,
    request_id: &Value,
) -> Result<Uuid, Box<McpResponse>> {
    let Some(token) = auth_token else {
        return Err(Box::new(McpResponse::error(
            Some(request_id.clone()),
            ERROR_INVALID_PARAMS,
            "Authentication token required".to_owned(),
        )));
    };

    let claims = resources
        .auth_manager
        .validate_token(token, &resources.jwks_manager)
        .map_err(|e| {
            error!("Authentication failed: {}", e);
            Box::new(McpResponse::error(
                Some(request_id.clone()),
                ERROR_INVALID_PARAMS,
                "Authentication required".to_owned(),
            ))
        })?;

    Uuid::parse_str(&claims.sub).map_err(|_| {
        error!("Invalid user ID in token: {}", claims.sub);
        Box::new(McpResponse::error(
            Some(request_id.clone()),
            ERROR_INVALID_PARAMS,
            "Invalid user ID in token".to_owned(),
        ))
    })
}

/// Parse resource read parameters from request
fn parse_resource_params(
    params: Option<&Value>,
    request_id: &Value,
) -> Result<json_schemas::ResourceReadParams, Box<McpResponse>> {
    let Some(params) = params else {
        return Err(Box::new(McpResponse::error(
            Some(request_id.clone()),
            ERROR_INVALID_PARAMS,
            "Missing parameters".to_owned(),
        )));
    };

    serde_json::from_value::<json_schemas::ResourceReadParams>(params.clone()).map_err(|e| {
        Box::new(McpResponse::error(
            Some(request_id.clone()),
            ERROR_INVALID_PARAMS,
            format!("Invalid resource read parameters: {e}"),
        ))
    })
}

impl ProtocolHandler {
    /// Supported MCP protocol versions (in preference order)
    const SUPPORTED_VERSIONS: &'static [&'static str] = &["2025-06-18", "2024-11-05"];

    /// Handle initialize request with proper version negotiation
    #[must_use]
    pub fn handle_initialize(request: McpRequest) -> McpResponse {
        Self::handle_initialize_internal(request, None)
    }

    /// Handle initialize request with resources (for dynamic port configuration)
    #[must_use]
    pub fn handle_initialize_with_resources(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        Self::handle_initialize_internal(request, Some(resources))
    }

    /// Try to store OAuth credentials from an initialize request
    async fn try_store_oauth_credentials(request: &McpRequest, resources: &Arc<ServerResources>) {
        // Extract params
        let Some(params) = &request.params else {
            return;
        };

        // Parse initialize request
        let Ok(init_request) = serde_json::from_value::<InitializeRequest>(params.clone()) else {
            return;
        };

        // Check for OAuth credentials
        let Some(oauth_creds) = init_request.oauth_credentials else {
            return;
        };

        // Authenticate and store credentials
        let Ok(user_id) = Self::authenticate_request(request, resources) else {
            warn!("OAuth credentials provided but authentication failed - credentials not stored");
            return;
        };

        if let Err(e) = Self::store_oauth_credentials(oauth_creds, &user_id, resources).await {
            warn!(
                "Failed to store OAuth credentials during initialization: {}",
                e
            );
        } else {
            info!("Successfully stored OAuth credentials for user {}", user_id);
        }
    }

    /// Handle initialize request with `ServerResources` for OAuth credential storage
    pub async fn handle_initialize_with_oauth(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        // Handle basic initialization first (doesn't require authentication)
        let response = Self::handle_initialize_internal(request.clone(), Some(resources));

        // If initialization successful and OAuth credentials provided, try to store them
        if response.error.is_none() {
            Self::try_store_oauth_credentials(&request, resources).await;
        }

        response
    }

    /// Internal initialize handler
    fn handle_initialize_internal(
        request: McpRequest,
        resources: Option<&Arc<ServerResources>>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // Parse initialize request parameters
        let Some(init_request) = request
            .params
            .as_ref()
            .and_then(|params| serde_json::from_value::<InitializeRequest>(params.clone()).ok())
        else {
            return McpResponse::error(
                Some(request_id),
                ERROR_INVALID_PARAMS,
                "Invalid initialize request parameters".to_owned(),
            );
        };

        // Validate client protocol version
        let client_version = &init_request.protocol_version;
        let negotiated_version = if Self::SUPPORTED_VERSIONS.contains(&client_version.as_str()) {
            // Use client version if supported
            client_version.clone()
        } else {
            // Return error for unsupported versions
            let supported_versions = Self::SUPPORTED_VERSIONS.join(", ");
            return McpResponse::error(
                Some(request_id),
                ERROR_VERSION_MISMATCH,
                format!("{MSG_VERSION_MISMATCH}. Client version: {client_version}, Supported versions: {supported_versions}")
            );
        };

        info!(
            "MCP version negotiated: {} (client: {}, server supports: {:?})",
            negotiated_version,
            client_version,
            Self::SUPPORTED_VERSIONS
        );

        // Create successful initialize response with negotiated version
        let init_response = if let Some(resources) = resources {
            // Use dynamic HTTP port from server configuration
            InitializeResponse::new_with_ports(
                negotiated_version,
                crate::constants::protocol::server_name_multitenant(),
                SERVER_VERSION.to_owned(),
                resources.config.http_port,
            )
        } else {
            // Fallback to default (hardcoded port)
            InitializeResponse::new(
                negotiated_version,
                crate::constants::protocol::server_name_multitenant(),
                SERVER_VERSION.to_owned(),
            )
        };

        match serde_json::to_value(&init_response) {
            Ok(result) => McpResponse::success(Some(request_id), result),
            Err(e) => {
                error!("Failed to serialize initialize response: {}", e);
                McpResponse::error(
                    Some(request_id),
                    ERROR_SERIALIZATION,
                    format!("{MSG_SERIALIZATION}: {e}"),
                )
            }
        }
    }

    /// Handle ping request
    pub fn handle_ping(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(Some(request_id), serde_json::json!({}))
    }

    /// Handle tools list request
    pub fn handle_tools_list(request: McpRequest) -> McpResponse {
        let tools = get_tools();

        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(Some(request_id), serde_json::json!({ "tools": tools }))
    }

    /// Handle prompts list request
    pub fn handle_prompts_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(Some(request_id), serde_json::json!({ "prompts": [] }))
    }

    /// Handle resources list request
    pub fn handle_resources_list(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // Extract user_id from auth context if available
        let user_id = request.auth_token.as_ref().and_then(|auth_token| {
            match resources
                .auth_manager
                .validate_token(auth_token, &resources.jwks_manager)
            {
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
            Some(request_id),
            serde_json::json!({ "resources": resource_list }),
        )
    }

    /// Handle resources read request
    pub async fn handle_resources_read(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // Extract and validate user ID from auth token
        let user_id =
            match extract_user_id_from_token(request.auth_token.as_ref(), resources, &request_id) {
                Ok(id) => id,
                Err(response) => return *response,
            };

        // Parse resource parameters
        let resource_params = match parse_resource_params(request.params.as_ref(), &request_id) {
            Ok(p) => p,
            Err(response) => return *response,
        };

        let uri = &resource_params.uri;

        match uri.as_str() {
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
                                "text": serde_json::to_string_pretty(&notifications).unwrap_or_else(|_| "[]".to_owned())
                            }]
                        });
                        McpResponse::success(Some(request_id), response_data)
                    }
                    Err(e) => {
                        error!("Failed to fetch OAuth notifications: {}", e);
                        McpResponse::error(
                            Some(request_id),
                            ERROR_AUTHENTICATION,
                            format!("{MSG_AUTHENTICATION}: Failed to fetch notifications - {e}"),
                        )
                    }
                }
            }
            _ => McpResponse::error(
                Some(request_id),
                ERROR_METHOD_NOT_FOUND,
                format!("Unknown resource URI: {uri}"),
            ),
        }
    }

    /// Handle unknown method request
    pub fn handle_unknown_method(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::error(
            Some(request_id),
            ERROR_METHOD_NOT_FOUND,
            format!("Unknown method: {}", request.method),
        )
    }

    /// Handle authenticate request
    pub fn handle_authenticate(
        request: McpRequest,
        auth_manager: &Arc<AuthManager>,
        jwks_manager: &Arc<crate::admin::jwks::JwksManager>,
    ) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        let auth_request: AuthRequest =
            match request.params.and_then(|p| serde_json::from_value(p).ok()) {
                Some(req) => req,
                None => {
                    return McpResponse::error(
                        Some(request_id),
                        ERROR_INVALID_PARAMS,
                        "Invalid authentication parameters".to_owned(),
                    );
                }
            };

        let auth_response = auth_manager.authenticate(&auth_request, jwks_manager);
        if auth_response.authenticated {
            info!("MCP authentication successful");
            McpResponse::success(
                Some(request_id),
                serde_json::json!({ "authenticated": true }),
            )
        } else {
            let error_msg = auth_response
                .error
                .as_deref()
                .unwrap_or("Authentication failed");
            info!("MCP authentication failed: {}", error_msg);
            McpResponse::error(
                Some(request_id),
                ERROR_INVALID_PARAMS,
                format!("Authentication failed: {error_msg}"),
            )
        }
    }

    /// Authenticate the MCP request and extract user information
    fn authenticate_request(
        request: &McpRequest,
        resources: &Arc<ServerResources>,
    ) -> Result<uuid::Uuid, Box<McpResponse>> {
        let request_id = request.id.clone().unwrap_or_else(default_request_id);

        // Extract auth token from request
        let auth_token = request.auth_token.as_deref().ok_or_else(|| {
            Box::new(McpResponse::error(
                Some(request_id.clone()),
                ERROR_AUTHENTICATION,
                "Authentication token required for OAuth credential storage".to_owned(),
            ))
        })?;

        // Validate token and extract user_id
        match resources
            .auth_manager
            .validate_token(auth_token, &resources.jwks_manager)
        {
            Ok(claims) => uuid::Uuid::parse_str(&claims.sub).map_or_else(
                |_| {
                    Err(Box::new(McpResponse::error(
                        Some(request_id.clone()),
                        ERROR_AUTHENTICATION,
                        "Invalid user ID in authentication token".to_owned(),
                    )))
                },
                Ok,
            ),
            Err(_) => Err(Box::new(McpResponse::error(
                Some(request_id),
                ERROR_AUTHENTICATION,
                "Invalid authentication token".to_owned(),
            ))),
        }
    }

    /// Store OAuth credentials provided during initialization
    async fn store_oauth_credentials(
        oauth_creds: std::collections::HashMap<String, crate::mcp::schema::OAuthAppCredentials>,
        user_id: &uuid::Uuid,
        resources: &Arc<ServerResources>,
    ) -> Result<(), String> {
        for (provider, creds) in oauth_creds {
            info!("Storing OAuth credentials for provider {provider} for user {user_id}");

            // Store encrypted OAuth app credentials in database
            // Use default redirect URI for MCP clients
            let redirect_uri = format!("urn:ietf:wg:oauth:2.0:oob:{provider}:mcp");
            resources
                .database
                .store_user_oauth_app(
                    *user_id,
                    &provider,
                    &creds.client_id,
                    &creds.client_secret,
                    &redirect_uri,
                )
                .await
                .map_err(|e| format!("Failed to store {provider} OAuth credentials: {e}"))?;
        }

        Ok(())
    }

    // === SAMPLING HANDLERS (FUTURE FEATURE) ===
    //
    // NOTE: Sampling support requires bidirectional transport infrastructure.
    // MCP Sampling allows servers to request the CLIENT to run LLM inference.
    // This requires server→client request capability, which Pierre's transport
    // layer will support in a future release once the following are complete:
    //
    // Requirements for sampling support:
    // 1. Bidirectional transport (SSE already supports server→client notifications)
    // 2. Request correlation (track server-initiated requests)
    // 3. Timeout handling for client LLM calls
    // 4. Client capability negotiation (verify client supports sampling)
    //
    // Intelligence features enabled by sampling:
    // - Natural language activity summaries generated by client LLM
    // - Personalized coaching advice with Claude reasoning
    // - Complex training pattern interpretation
    //
    // Sampling types are defined in schema.rs (CreateMessageRequest/Result) but not
    // advertised in server capabilities until the bidirectional transport is ready.

    // === COMPLETION HANDLERS ===

    /// Handle completion/complete request for auto-complete suggestions
    pub fn handle_completion_complete(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // Parse the completion request
        let complete_request: Result<crate::mcp::schema::CompleteRequest, _> = request
            .params
            .ok_or("Missing parameters")
            .and_then(|p| serde_json::from_value(p).map_err(|_| "Invalid parameters"));

        match complete_request {
            Ok(req) => {
                // Generate completions based on the reference type
                let completion = Self::generate_completions(&req);

                let result = crate::mcp::schema::CompleteResult { completion };

                match serde_json::to_value(&result) {
                    Ok(result_value) => McpResponse::success(Some(request_id), result_value),
                    Err(e) => {
                        error!("Failed to serialize completion result: {}", e);
                        McpResponse::error(
                            Some(request_id),
                            ERROR_SERIALIZATION,
                            format!("{MSG_SERIALIZATION}: {e}"),
                        )
                    }
                }
            }
            Err(e) => McpResponse::error(
                Some(request_id),
                ERROR_INVALID_PARAMS,
                format!("Invalid completion request: {e}"),
            ),
        }
    }

    /// Generate completion suggestions based on the request
    fn generate_completions(
        req: &crate::mcp::schema::CompleteRequest,
    ) -> crate::mcp::schema::Completion {
        use crate::mcp::schema::Completion;

        // Match on the reference type
        match req.ref_.type_.as_str() {
            "ref/prompt" => {
                // Complete prompt arguments
                if req.argument.name == "activity_type" {
                    let activity_types = ["run", "ride", "swim", "strength", "walk", "hike"];
                    let matching: Vec<String> = activity_types
                        .iter()
                        .filter(|t| t.starts_with(&req.argument.value))
                        .map(|s| (*s).to_owned())
                        .collect();

                    return Completion {
                        values: matching.clone(),
                        total: Some(matching.len()),
                        has_more: Some(false),
                    };
                }

                if req.argument.name == "provider" {
                    let providers = ["strava", "fitbit", "garmin"];
                    let matching: Vec<String> = providers
                        .iter()
                        .filter(|p| p.starts_with(&req.argument.value))
                        .map(|s| (*s).to_owned())
                        .collect();

                    return Completion {
                        values: matching.clone(),
                        total: Some(matching.len()),
                        has_more: Some(false),
                    };
                }

                if req.argument.name == "goal_type" {
                    let goal_types = ["distance", "time", "frequency", "performance", "custom"];
                    let matching: Vec<String> = goal_types
                        .iter()
                        .filter(|g| g.starts_with(&req.argument.value))
                        .map(|s| (*s).to_owned())
                        .collect();

                    return Completion {
                        values: matching.clone(),
                        total: Some(matching.len()),
                        has_more: Some(false),
                    };
                }
            }
            "ref/resource" => {
                // Complete resource URIs
                let resource_uris = ["oauth://notifications"];
                let matching: Vec<String> = resource_uris
                    .iter()
                    .filter(|uri| uri.starts_with(&req.argument.value))
                    .map(|s| (*s).to_owned())
                    .collect();

                return Completion {
                    values: matching.clone(),
                    total: Some(matching.len()),
                    has_more: Some(false),
                };
            }
            _ => {}
        }

        // Default: no completions
        Completion {
            values: vec![],
            total: Some(0),
            has_more: Some(false),
        }
    }

    // === ROOTS HANDLERS ===

    /// Handle roots/list request
    pub fn handle_roots_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);

        // For now, return an empty list of roots
        // In a full implementation, this would return filesystem roots
        // that the MCP client has access to
        let roots: Vec<crate::mcp::schema::Root> = vec![];

        let result = serde_json::json!({
            "roots": roots
        });

        McpResponse::success(Some(request_id), result)
    }
}
