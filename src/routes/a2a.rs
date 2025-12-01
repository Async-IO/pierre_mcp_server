// ABOUTME: A2A (Agent-to-Agent) protocol route handlers for inter-agent communication
// ABOUTME: Provides endpoints for agent registration, messaging, and protocol management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! A2A protocol routes for agent-to-agent communication
//!
//! This module provides endpoints for A2A client management and protocol operations.
//! All client management routes require valid JWT authentication.

use crate::a2a::agent_card::AgentCard;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::mcp::resources::ServerResources;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Response for A2A client list
#[derive(Debug, Serialize)]
pub struct A2AClientResponse {
    /// Unique client identifier
    pub id: String,
    /// Human-readable client name
    pub name: String,
    /// Description of the client application
    pub description: String,
    /// Public key for identification
    pub public_key: String,
    /// List of capabilities this client can access
    pub capabilities: Vec<String>,
    /// List of permissions granted to this client
    pub permissions: Vec<String>,
    /// Whether this client is active
    pub is_active: bool,
    /// When this client was created
    pub created_at: String,
    /// When this client was last updated
    pub updated_at: String,
    /// Rate limit for requests per window
    pub rate_limit_requests: u32,
    /// Rate limit window in seconds
    pub rate_limit_window_seconds: u32,
}

/// Request to create a new A2A client
#[derive(Debug, Deserialize)]
struct CreateA2AClientRequest {
    /// Name of the client application
    name: String,
    /// Description of the client's purpose
    description: String,
    /// List of agent capabilities this client provides
    capabilities: Vec<String>,
    /// `OAuth2` redirect URIs for authorization flows (optional)
    #[serde(default)]
    redirect_uris: Vec<String>,
    /// Contact email for the client administrator
    contact_email: String,
}

/// Response for created A2A client with credentials
#[derive(Debug, Serialize)]
struct CreateA2AClientResponse {
    /// Unique client identifier
    client_id: String,
    /// Client secret for authentication
    client_secret: String,
    /// API key for direct API access
    api_key: String,
    /// Ed25519 public key for signature verification
    public_key: String,
    /// Ed25519 private key for signing
    private_key: String,
    /// Key type identifier
    key_type: String,
}

/// A2A routes implementation
pub struct A2ARoutes;

impl A2ARoutes {
    /// Create all A2A routes
    ///
    /// Routes match frontend API expectations:
    /// - /a2a/status - Basic A2A protocol status
    /// - /a2a/clients - List A2A clients
    /// - /a2a/clients/:id - Get/delete A2A client
    /// - /a2a/dashboard/overview - Dashboard overview for A2A clients
    /// - /a2a/dashboard/analytics - Usage analytics for A2A clients
    /// - /.well-known/agent-card.json - Agent card discovery
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            // Public routes (no auth required)
            .route("/a2a/status", get(Self::handle_status))
            .route(
                "/.well-known/agent-card.json",
                get(Self::handle_agent_card_discovery),
            )
            // Client management routes (auth required)
            .route(
                "/a2a/clients",
                get(Self::handle_list_clients).post(Self::handle_create_client),
            )
            .route("/a2a/clients/:client_id", get(Self::handle_get_client))
            .route(
                "/a2a/clients/:client_id",
                delete(Self::handle_delete_client),
            )
            .route(
                "/a2a/clients/:client_id/usage",
                get(Self::handle_client_usage),
            )
            .route(
                "/a2a/clients/:client_id/rate-limit",
                get(Self::handle_client_rate_limit),
            )
            // Dashboard routes
            .route(
                "/a2a/dashboard/overview",
                get(Self::handle_dashboard_overview),
            )
            .route(
                "/a2a/dashboard/analytics",
                get(Self::handle_dashboard_analytics),
            )
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &axum::http::HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<crate::auth::AuthResult, AppError> {
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) =
                crate::security::cookies::get_cookie_value(headers, "auth_token")
            {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Handle A2A status (public endpoint)
    async fn handle_status() -> Json<serde_json::Value> {
        // Yield to scheduler for cooperative multitasking
        tokio::task::yield_now().await;
        Json(serde_json::json!({
            "status": "active"
        }))
    }

    /// Handle agent card discovery endpoint (public endpoint)
    async fn handle_agent_card_discovery() -> Json<AgentCard> {
        // Yield to scheduler for cooperative multitasking
        tokio::task::yield_now().await;
        Json(AgentCard::new())
    }

    /// List all A2A clients for authenticated user
    async fn handle_list_clients(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;

        let clients = resources.database.list_a2a_clients(&user_id).await?;

        let response: Vec<A2AClientResponse> = clients
            .into_iter()
            .map(|c| A2AClientResponse {
                id: c.id,
                name: c.name,
                description: c.description,
                public_key: c.public_key,
                capabilities: c.capabilities,
                permissions: c.permissions,
                is_active: c.is_active,
                created_at: c.created_at.to_rfc3339(),
                updated_at: c.updated_at.to_rfc3339(),
                rate_limit_requests: c.rate_limit_requests,
                rate_limit_window_seconds: c.rate_limit_window_seconds,
            })
            .collect();

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Create a new A2A client
    async fn handle_create_client(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Json(request): Json<CreateA2AClientRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            client_name = %request.name,
            "Creating new A2A client"
        );

        // Convert to the A2A client registration request format
        let registration_request = crate::a2a::client::ClientRegistrationRequest {
            name: request.name,
            description: request.description,
            capabilities: request.capabilities,
            redirect_uris: request.redirect_uris,
            contact_email: request.contact_email,
        };

        // Register the client using the A2A client manager with the authenticated user's ID
        let credentials = resources
            .a2a_client_manager
            .register_client(registration_request, auth.user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to register A2A client");
                AppError::internal(format!("Failed to register A2A client: {e}"))
            })?;

        let response = CreateA2AClientResponse {
            client_id: credentials.client_id,
            client_secret: credentials.client_secret,
            api_key: credentials.api_key,
            public_key: credentials.public_key,
            private_key: credentials.private_key,
            key_type: credentials.key_type,
        };

        tracing::info!(
            client_id = %response.client_id,
            "A2A client created successfully"
        );

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Get a specific A2A client
    async fn handle_get_client(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(client_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;

        let client = resources
            .database
            .get_a2a_client(&client_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("A2A client {client_id}")))?;

        // Check ownership
        if client.user_id != user_id {
            return Err(AppError::not_found(format!("A2A client {client_id}")));
        }

        let response = A2AClientResponse {
            id: client.id,
            name: client.name,
            description: client.description,
            public_key: client.public_key,
            capabilities: client.capabilities,
            permissions: client.permissions,
            is_active: client.is_active,
            created_at: client.created_at.to_rfc3339(),
            updated_at: client.updated_at.to_rfc3339(),
            rate_limit_requests: client.rate_limit_requests,
            rate_limit_window_seconds: client.rate_limit_window_seconds,
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Delete (deactivate) an A2A client
    async fn handle_delete_client(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(client_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;

        // Verify ownership
        let client = resources
            .database
            .get_a2a_client(&client_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("A2A client {client_id}")))?;

        if client.user_id != user_id {
            return Err(AppError::not_found(format!("A2A client {client_id}")));
        }

        // Deactivate client
        resources.database.deactivate_a2a_client(&client_id).await?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "message": "Client deactivated successfully",
                "client_id": client_id
            })),
        )
            .into_response())
    }

    /// Get A2A client usage statistics
    async fn handle_client_usage(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(client_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;

        // Verify ownership
        let client = resources
            .database
            .get_a2a_client(&client_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("A2A client {client_id}")))?;

        if client.user_id != user_id {
            return Err(AppError::not_found(format!("A2A client {client_id}")));
        }

        // Get current usage count
        let current_usage = resources
            .database
            .get_a2a_client_current_usage(&client_id)
            .await
            .unwrap_or(0);

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "client_id": client_id,
                "total_requests": current_usage,
                "requests_today": 0,
                "daily_usage": []
            })),
        )
            .into_response())
    }

    /// Get A2A client rate limit status
    async fn handle_client_rate_limit(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(client_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;

        // Verify ownership and get client
        let client = resources
            .database
            .get_a2a_client(&client_id)
            .await?
            .ok_or_else(|| AppError::not_found(format!("A2A client {client_id}")))?;

        if client.user_id != user_id {
            return Err(AppError::not_found(format!("A2A client {client_id}")));
        }

        let current_usage = resources
            .database
            .get_a2a_client_current_usage(&client_id)
            .await
            .unwrap_or(0);

        let limit = client.rate_limit_requests;
        let remaining = limit.saturating_sub(current_usage);

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "client_id": client_id,
                "rate_limit_requests": limit,
                "rate_limit_window_seconds": client.rate_limit_window_seconds,
                "current_usage": current_usage,
                "remaining": remaining,
                "reset_at": chrono::Utc::now().to_rfc3339()
            })),
        )
            .into_response())
    }

    /// Handle A2A dashboard overview
    async fn handle_dashboard_overview(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;
        let user_id = auth.user_id;

        let clients = resources.database.list_a2a_clients(&user_id).await?;
        let active_count = clients.iter().filter(|c| c.is_active).count();

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "total_clients": clients.len(),
                "active_clients": active_count,
                "total_requests": 0,
                "requests_today": 0,
                "status": "active"
            })),
        )
            .into_response())
    }

    /// Handle A2A dashboard analytics
    async fn handle_dashboard_analytics(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        Self::authenticate(&headers, &resources).await?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "daily_requests": [],
                "top_clients": [],
                "request_types": {},
                "period_days": 30
            })),
        )
            .into_response())
    }
}
