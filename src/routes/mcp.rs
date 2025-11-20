// ABOUTME: MCP (Model Context Protocol) route handlers for AI assistant integration
// ABOUTME: Provides MCP protocol endpoints for tool discovery and execution
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! MCP protocol routes for AI assistant integration

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    Json, Router,
};
use lru::LruCache;
use serde_json::Value;
use std::{num::NonZeroUsize, sync::Arc};
use tokio::sync::Mutex;
use tracing::{debug, info};

use crate::database_plugins::DatabaseProvider;
use crate::mcp::resources::ServerResources;
use crate::mcp::tenant_isolation::validate_jwt_token_for_mcp;

/// Session data for MCP requests
#[derive(Clone)]
struct SessionData {
    jwt_token: String,
    user_id: uuid::Uuid,
}

/// MCP request headers
struct McpRequestHeaders {
    auth_header: Option<String>,
    _origin: Option<String>,
    _accept: Option<String>,
    session_id: Option<String>,
}

/// MCP routes state
#[derive(Clone)]
pub struct McpRoutesState {
    resources: Arc<ServerResources>,
    sessions: Arc<Mutex<LruCache<String, SessionData>>>,
}

/// MCP routes implementation
pub struct McpRoutes;

impl McpRoutes {
    /// Create all MCP routes with server resources
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        use axum::routing::{get, post};

        // Create session cache with capacity for 1000 sessions
        // If NonZeroUsize::new somehow fails, fall back to minimum cache size of 1
        let cache_size = NonZeroUsize::new(1000).unwrap_or(NonZeroUsize::MIN);
        let sessions = Arc::new(Mutex::new(LruCache::new(cache_size)));

        let state = McpRoutesState {
            resources,
            sessions,
        };

        Router::new()
            .route("/mcp/tools", get(Self::handle_tools))
            .route("/mcp", post(Self::handle_mcp_request))
            .with_state(state)
    }

    /// Handle MCP tools discovery
    ///
    /// Returns all available MCP tools for client discovery.
    /// This endpoint allows MCP clients to enumerate available tools
    /// before making tool call requests.
    async fn handle_tools() -> Json<Value> {
        // Yield to scheduler for cooperative multitasking
        tokio::task::yield_now().await;

        let tools = crate::mcp::schema::get_tools();
        Json(serde_json::json!({
            "tools": tools
        }))
    }

    /// Handle MCP JSON-RPC requests
    ///
    /// # Errors
    /// Returns error response for invalid requests or internal errors
    async fn handle_mcp_request(
        State(state): State<McpRoutesState>,
        method: Method,
        headers: HeaderMap,
        request: Request<Body>,
    ) -> Response {
        debug!("=== MCP HTTP Request START ===");
        debug!("Method: {}", method);

        // Extract headers
        let mcp_headers = Self::extract_headers(&headers);

        // Parse request body
        let body = match Self::parse_body(request).await {
            Ok(body) => body,
            Err(response) => return response,
        };

        // Determine session ID
        let session_id = Self::determine_session_id(&mcp_headers);

        // Resolve effective auth (from header or session)
        let effective_auth = Self::resolve_effective_auth(&mcp_headers, &state.sessions).await;

        // Validate and store session if needed
        Self::validate_and_store_session(&mcp_headers, &state).await;

        // Handle the MCP request
        match Self::handle_mcp_http_request(method, effective_auth, body, &state).await {
            Ok(mut response) => {
                // Add session ID header to response
                if let Ok(header_value) = session_id.parse() {
                    response
                        .headers_mut()
                        .insert("Mcp-Session-Id", header_value);
                }
                response
            }
            Err(response) => response,
        }
    }

    /// Extract MCP headers from request
    fn extract_headers(headers: &HeaderMap) -> McpRequestHeaders {
        McpRequestHeaders {
            auth_header: headers
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            _origin: headers
                .get("origin")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            _accept: headers
                .get("accept")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
            session_id: headers
                .get("mcp-session-id")
                .and_then(|h| h.to_str().ok())
                .map(String::from),
        }
    }

    /// Parse request body as JSON
    async fn parse_body(request: Request<Body>) -> Result<Value, Response> {
        use axum::body::to_bytes;

        let body_bytes = to_bytes(request.into_body(), usize::MAX)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to read request body");
                (StatusCode::BAD_REQUEST, "Failed to read request body").into_response()
            })?;

        if body_bytes.is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_slice(&body_bytes).map_err(|e| {
            tracing::warn!(error = %e, "Failed to parse JSON body");
            (StatusCode::BAD_REQUEST, "Invalid JSON").into_response()
        })
    }

    /// Determine session ID (use provided or generate new)
    fn determine_session_id(headers: &McpRequestHeaders) -> String {
        headers.session_id.clone().unwrap_or_else(|| {
            let new_session_id = format!("session_{}", uuid::Uuid::new_v4());
            info!("Generated new MCP session: {}", new_session_id);
            new_session_id
        })
    }

    /// Resolve effective auth header from current request or stored session
    async fn resolve_effective_auth(
        headers: &McpRequestHeaders,
        sessions: &Arc<Mutex<LruCache<String, SessionData>>>,
    ) -> Option<String> {
        if headers.auth_header.is_some() {
            debug!("Using auth header from current request");
            headers.auth_header.clone()
        } else if let Some(sid) = headers.session_id.as_ref() {
            let mut sessions_guard = sessions.lock().await;
            sessions_guard.get(sid).map(|session_data| {
                info!(
                    "Using stored session auth for user {}",
                    session_data.user_id
                );
                format!("Bearer {}", session_data.jwt_token)
            })
        } else {
            None
        }
    }

    /// Validate JWT and store session if auth header provided
    async fn validate_and_store_session(headers: &McpRequestHeaders, state: &McpRoutesState) {
        let Some(ref auth) = headers.auth_header else {
            return;
        };

        let session_id = Self::determine_session_id(headers);

        // Check if session already exists
        let needs_validation = {
            let sessions_guard = state.sessions.lock().await;
            !sessions_guard.contains(&session_id)
        };

        if !needs_validation {
            return;
        }

        // Extract and validate JWT token
        let Some(token) = auth.strip_prefix("Bearer ") else {
            return;
        };

        Self::validate_and_store_jwt(token, &state.sessions, &session_id, state).await;
    }

    /// Validate JWT token and store session data
    async fn validate_and_store_jwt(
        token: &str,
        sessions: &Arc<Mutex<LruCache<String, SessionData>>>,
        session_id: &str,
        state: &McpRoutesState,
    ) {
        // Validate the JWT
        let Ok(jwt_result) = validate_jwt_token_for_mcp(
            token,
            &state.resources.auth_manager,
            &state.resources.jwks_manager,
            &state.resources.database,
        )
        .await
        else {
            return;
        };

        // Get user details
        let Ok(Some(user)) = state.resources.database.get_user(jwt_result.user_id).await else {
            return;
        };

        // Store session
        let mut sessions_guard = sessions.lock().await;
        sessions_guard.put(
            session_id.to_owned(),
            SessionData {
                jwt_token: token.to_owned(),
                user_id: jwt_result.user_id,
            },
        );
        drop(sessions_guard);
        info!(
            "Stored session {} for user {} ({})",
            session_id, jwt_result.user_id, user.email
        );
    }

    /// Handle MCP HTTP request with conditional auth
    async fn handle_mcp_http_request(
        _method: Method,
        auth_header: Option<String>,
        body: Value,
        state: &McpRoutesState,
    ) -> Result<Response, Response> {
        // Parse JSON-RPC request
        let mut mcp_request: crate::mcp::multitenant::McpRequest =
            serde_json::from_value(body.clone()).map_err(|e| {
                tracing::error!(error = %e, "Failed to parse MCP request");
                (StatusCode::BAD_REQUEST, "Invalid MCP request format").into_response()
            })?;

        // Inject HTTP Authorization header into JSON-RPC auth_token field
        // This allows HTTP Bearer tokens to work with JSON-RPC authentication
        // Keep the full "Bearer <token>" format as expected by the auth middleware
        if mcp_request.auth_token.is_none() {
            if let Some(auth) = auth_header {
                mcp_request.auth_token = Some(auth);
                debug!("Injected Authorization header from HTTP into MCP request");
            }
        }

        // Process MCP request
        let response_opt = crate::mcp::multitenant::MultiTenantMcpServer::handle_request(
            mcp_request,
            &state.resources,
        )
        .await;

        // Convert to HTTP response
        match response_opt {
            Some(mcp_response) => {
                let json_response = serde_json::to_value(&mcp_response).map_err(|e| {
                    tracing::error!(error = %e, "Failed to serialize MCP response");
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to serialize response",
                    )
                        .into_response()
                })?;

                Ok((StatusCode::OK, Json(json_response)).into_response())
            }
            None => {
                // No response for notifications
                Ok(StatusCode::ACCEPTED.into_response())
            }
        }
    }
}
