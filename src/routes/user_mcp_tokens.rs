// ABOUTME: User MCP token management route handlers for user self-service token operations
// ABOUTME: Provides REST endpoints for creating, listing, and revoking MCP tokens for AI clients
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! User MCP token management routes
//!
//! This module handles MCP token creation, listing, and revocation
//! for authenticated users. All handlers require valid JWT authentication.

use crate::{
    auth::AuthResult, database::CreateUserMcpTokenRequest, database_plugins::DatabaseProvider,
    errors::AppError, mcp::resources::ServerResources, security::cookies::get_cookie_value,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// User MCP token routes
pub struct UserMcpTokenRoutes;

/// Response for token creation (includes the secret token value)
#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    /// The token ID
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// First 12 characters for identification
    pub token_prefix: String,
    /// The actual token value (only shown once!)
    pub token_value: String,
    /// Expiration timestamp (if set)
    pub expires_at: Option<String>,
    /// Creation timestamp
    pub created_at: String,
}

/// Response for listing tokens
#[derive(Debug, Serialize)]
pub struct TokenListResponse {
    /// List of user's MCP tokens
    pub tokens: Vec<TokenInfo>,
}

/// Token info for listing (no secret value)
#[derive(Debug, Serialize)]
pub struct TokenInfo {
    /// Unique token ID
    pub id: String,
    /// Human-readable token name
    pub name: String,
    /// First 12 characters for identification
    pub token_prefix: String,
    /// Expiration timestamp (if set)
    pub expires_at: Option<String>,
    /// Last time the token was used
    pub last_used_at: Option<String>,
    /// Number of times the token has been used
    pub usage_count: u32,
    /// Whether the token has been revoked
    pub is_revoked: bool,
    /// Creation timestamp
    pub created_at: String,
}

/// Request to create a new token
#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    /// Human-readable name for the token
    pub name: String,
    /// Days until expiration (None = never expires)
    pub expires_in_days: Option<u32>,
}

impl UserMcpTokenRoutes {
    /// Create all user MCP token routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/api/user/mcp-tokens", post(Self::handle_create_token))
            .route("/api/user/mcp-tokens", get(Self::handle_list_tokens))
            .route(
                "/api/user/mcp-tokens/:token_id",
                delete(Self::handle_revoke_token),
            )
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header or cookie
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        // Try Authorization header first, then fall back to auth_token cookie
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                // Fall back to auth_token cookie, format as Bearer token
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

    /// Handle token creation
    async fn handle_create_token(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<CreateTokenRequest>,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // Create token
        let db_request = CreateUserMcpTokenRequest {
            name: request.name,
            expires_in_days: request.expires_in_days,
        };

        let result = resources
            .database
            .create_user_mcp_token(auth.user_id, &db_request)
            .await?;

        let response = CreateTokenResponse {
            id: result.token.id,
            name: result.token.name,
            token_prefix: result.token.token_prefix,
            token_value: result.token_value,
            expires_at: result.token.expires_at.map(|dt| dt.to_rfc3339()),
            created_at: result.token.created_at.to_rfc3339(),
        };

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle listing user's tokens
    async fn handle_list_tokens(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // List tokens
        let tokens = resources
            .database
            .list_user_mcp_tokens(auth.user_id)
            .await?;

        let response = TokenListResponse {
            tokens: tokens
                .into_iter()
                .map(|t| TokenInfo {
                    id: t.id,
                    name: t.name,
                    token_prefix: t.token_prefix,
                    expires_at: t.expires_at.map(|dt| dt.to_rfc3339()),
                    last_used_at: t.last_used_at.map(|dt| dt.to_rfc3339()),
                    usage_count: t.usage_count,
                    is_revoked: t.is_revoked,
                    created_at: t.created_at.to_rfc3339(),
                })
                .collect(),
        };

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle token revocation
    async fn handle_revoke_token(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(token_id): Path<String>,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // Revoke token
        resources
            .database
            .revoke_user_mcp_token(&token_id, auth.user_id)
            .await?;

        Ok((StatusCode::OK, Json(serde_json::json!({"success": true}))).into_response())
    }
}
