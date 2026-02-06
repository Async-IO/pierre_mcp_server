// ABOUTME: MCP (Model Context Protocol) route handlers for AI assistant integration
// ABOUTME: Provides MCP protocol endpoints for tool discovery and execution
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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
use tokio::{sync::Mutex, task::yield_now};
use tracing::{debug, error, field::Empty, info, info_span, warn, Instrument};

use crate::{
    constants::mcp_transport::MAX_REQUEST_BODY_BYTES,
    database_plugins::DatabaseProvider,
    mcp::{
        multitenant::{McpRequest, MultiTenantMcpServer},
        resources::ServerResources,
        schema::get_tools,
        tenant_isolation::validate_jwt_token_for_mcp,
    },
    middleware::redact_session_id,
    middleware::RequestId,
};

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
        yield_now().await;

        let tools = get_tools();
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
        // Extract request ID from middleware before consuming request body
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .map_or_else(|| uuid::Uuid::new_v4().to_string(), |r| r.0.clone());

        // Create a span for this MCP request with correlation ID
        let span = info_span!(
            "mcp_request",
            request_id = %request_id,
            method = %method,
            session_id = Empty,
            user_id = Empty,
        );

        Self::handle_mcp_request_inner(state, method, headers, request, request_id)
            .instrument(span)
            .await
    }

    /// Inner handler with tracing span context
    async fn handle_mcp_request_inner(
        state: McpRoutesState,
        method: Method,
        headers: HeaderMap,
        request: Request<Body>,
        request_id: String,
    ) -> Response {
        debug!(request_id = %request_id, "MCP request started");

        // Extract headers
        let mcp_headers = Self::extract_headers(&headers);

        // Parse request body
        let body = match Self::parse_body(request).await {
            Ok(body) => body,
            Err(response) => return response,
        };

        // Determine session ID once and reuse throughout the request
        let session_id = Self::determine_session_id(&mcp_headers);

        // Record session ID in span
        tracing::Span::current().record("session_id", &session_id);

        // Resolve effective auth (from header or session)
        let effective_auth = Self::resolve_effective_auth(&mcp_headers, &state.sessions).await;

        // Validate and store session if needed (pass session_id to avoid regenerating)
        Self::validate_and_store_session(&mcp_headers, &session_id, &state).await;

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
    ///
    /// Enforces a maximum body size to prevent memory exhaustion from oversized payloads.
    async fn parse_body(request: Request<Body>) -> Result<Value, Response> {
        use axum::body::to_bytes;

        let body_bytes = to_bytes(request.into_body(), MAX_REQUEST_BODY_BYTES)
            .await
            .map_err(|e| {
                warn!(error = %e, max_bytes = MAX_REQUEST_BODY_BYTES, "Request body exceeds size limit or read failed");
                (StatusCode::PAYLOAD_TOO_LARGE, "Request body too large").into_response()
            })?;

        if body_bytes.is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_slice(&body_bytes).map_err(|e| {
            warn!(error = %e, "Failed to parse JSON body");
            (StatusCode::BAD_REQUEST, "Invalid JSON").into_response()
        })
    }

    /// Determine session ID with server-side generation
    ///
    /// When an auth header is present, always generates a new server-side session ID
    /// to prevent session fixation attacks where an attacker pre-selects a session ID
    /// and tricks a victim into authenticating with it. Client-provided session IDs are
    /// only used for subsequent unauthenticated requests to resume an existing session.
    fn determine_session_id(headers: &McpRequestHeaders) -> String {
        if headers.auth_header.is_some() {
            // Always generate server-controlled session ID when authenticating
            let new_session_id = format!("session_{}", uuid::Uuid::new_v4());
            info!(
                "Generated server-side MCP session: {}",
                redact_session_id(&new_session_id)
            );
            new_session_id
        } else {
            headers.session_id.clone().unwrap_or_else(|| {
                let new_session_id = format!("session_{}", uuid::Uuid::new_v4());
                info!(
                    "Generated new MCP session: {}",
                    redact_session_id(&new_session_id)
                );
                new_session_id
            })
        }
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
    async fn validate_and_store_session(
        headers: &McpRequestHeaders,
        session_id: &str,
        state: &McpRoutesState,
    ) {
        let Some(ref auth) = headers.auth_header else {
            return;
        };

        // Check if session already exists
        let needs_validation = {
            let sessions_guard = state.sessions.lock().await;
            !sessions_guard.contains(session_id)
        };

        if !needs_validation {
            return;
        }

        // Extract and validate JWT token
        let Some(token) = auth.strip_prefix("Bearer ") else {
            return;
        };

        Self::validate_and_store_jwt(token, &state.sessions, session_id, state).await;
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

        // Record user_id in the tracing span for log correlation
        tracing::Span::current().record("user_id", jwt_result.user_id.to_string());

        info!(
            user_id = %jwt_result.user_id,
            user_email = %user.email,
            session_id = %redact_session_id(session_id),
            "Session stored for authenticated user"
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
        let mut mcp_request: McpRequest = serde_json::from_value(body.clone()).map_err(|e| {
            error!(error = %e, "Failed to parse MCP request");
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
        let response_opt =
            MultiTenantMcpServer::handle_request(mcp_request, &state.resources).await;

        // Convert to HTTP response
        match response_opt {
            Some(mcp_response) => {
                let json_response = serde_json::to_value(&mcp_response).map_err(|e| {
                    error!(error = %e, "Failed to serialize MCP response");
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
