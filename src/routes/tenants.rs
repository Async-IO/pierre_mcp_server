// ABOUTME: Tenant management route handlers for multi-tenant operations
// ABOUTME: Provides REST endpoints for creating, listing, updating, and deleting tenants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tenant management routes
//!
//! This module handles tenant CRUD operations for multi-tenant functionality.
//! All handlers require valid JWT authentication.

use crate::{auth::AuthResult, errors::AppError, mcp::resources::ServerResources, tenant_routes};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

/// Tenant management routes
pub struct TenantRoutes;

impl TenantRoutes {
    /// Create all tenant management routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/tenants", post(Self::handle_create_tenant))
            .route("/tenants", get(Self::handle_list_tenants))
            .with_state(resources)
    }

    /// Extract and authenticate user from authorization header
    async fn authenticate(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| AppError::auth_invalid("Missing authorization header"))?;

        resources
            .auth_middleware
            .authenticate_request(Some(auth_header))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))
    }

    /// Handle tenant creation
    async fn handle_create_tenant(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<tenant_routes::CreateTenantRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let response =
            tenant_routes::create_tenant(request, auth, resources.database.clone()).await?;

        Ok((StatusCode::CREATED, Json(response)).into_response())
    }

    /// Handle listing tenants
    async fn handle_list_tenants(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        let response = tenant_routes::list_tenants(auth, resources.database.clone()).await?;

        Ok((StatusCode::OK, Json(response)).into_response())
    }
}
