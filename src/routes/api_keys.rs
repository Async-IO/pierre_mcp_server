// ABOUTME: API key management route handlers for user self-service key operations
// ABOUTME: Provides REST endpoints for creating, listing, and managing API keys with authentication
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! API key management routes
//!
//! This module handles API key creation, listing, deactivation, and usage tracking
//! for authenticated users. All handlers require valid JWT authentication.

use crate::{
    api_key_routes::ApiKeyRoutes as ApiKeyService, api_keys::CreateApiKeyRequestSimple,
    auth::AuthResult, errors::AppError, mcp::resources::ServerResources,
    security::cookies::get_cookie_value,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

/// Query parameters for API key usage statistics
#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    /// Start date for usage statistics (ISO 8601 format)
    pub start_date: DateTime<Utc>,
    /// End date for usage statistics (ISO 8601 format)
    pub end_date: DateTime<Utc>,
}

/// API key management routes
pub struct ApiKeyRoutes;

impl ApiKeyRoutes {
    /// Create all API key management routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/api/keys", post(Self::handle_create_api_key))
            .route("/api/keys", get(Self::handle_list_api_keys))
            .route("/api/keys/:key_id", delete(Self::handle_deactivate_api_key))
            .route("/api/keys/:key_id/usage", get(Self::handle_get_usage))
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

    /// Handle API key creation
    async fn handle_create_api_key(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<CreateApiKeyRequestSimple>,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // Create API key using service layer
        let service = ApiKeyService::new(resources);
        let response = service
            .create_api_key_simple(&auth, request)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create API key: {e}")))?;

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle listing user's API keys
    async fn handle_list_api_keys(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // List API keys using service layer
        let service = ApiKeyService::new(resources);
        let response = service
            .list_api_keys(&auth)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list API keys: {e}")))?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle API key deactivation
    async fn handle_deactivate_api_key(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(key_id): Path<String>,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // Deactivate API key using service layer
        let service = ApiKeyService::new(resources);
        let response = service
            .deactivate_api_key(&auth, &key_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to deactivate API key: {e}")))?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }

    /// Handle getting API key usage statistics
    async fn handle_get_usage(
        State(resources): State<Arc<ServerResources>>,
        Path(key_id): Path<String>,
        Query(query): Query<UsageQuery>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate user from JWT token
        let auth = Self::authenticate(&headers, &resources).await?;

        // Use the service layer which enforces ownership verification
        let service = ApiKeyService::new(resources);
        let response = service
            .get_api_key_usage(&auth, &key_id, query.start_date, query.end_date)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get API key usage: {e}")))?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
