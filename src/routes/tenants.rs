// ABOUTME: Tenant management route handlers for multi-tenant operations
// ABOUTME: Provides REST endpoints for creating, listing, switching tenants, and tenant configuration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tenant management routes
//!
//! This module handles tenant CRUD operations for multi-tenant functionality.
//! All handlers require valid JWT authentication.
//!
//! Users can belong to multiple tenants (like Slack workspaces or GitHub organizations).
//! The active tenant for a session is determined by the `active_tenant_id` claim in the JWT.
//! Use the POST /tenants/switch endpoint to change the active tenant and receive a new JWT.

use crate::{
    auth::AuthResult, database_plugins::DatabaseProvider, errors::AppError,
    mcp::resources::ServerResources, tenant_routes, utils::uuid::parse_uuid,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, warn};

/// Request body for switching active tenant
#[derive(Debug, Deserialize)]
pub struct SwitchTenantRequest {
    /// UUID of the tenant to switch to
    pub tenant_id: String,
}

/// Response after successful tenant switch
#[derive(Debug, Serialize)]
pub struct SwitchTenantResponse {
    /// New JWT token with the `active_tenant_id` claim set
    pub token: String,
    /// Tenant ID that is now active
    pub active_tenant_id: String,
    /// Tenant name for display
    pub tenant_name: String,
    /// User's role in this tenant
    pub role: String,
    /// Token expiration time in seconds
    pub expires_in: u64,
}

/// Response listing all tenants a user belongs to
#[derive(Debug, Serialize)]
pub struct UserTenantsResponse {
    /// List of tenants the user belongs to
    pub tenants: Vec<UserTenantInfo>,
    /// Currently active tenant ID (if any)
    pub active_tenant_id: Option<String>,
}

/// Information about a tenant membership
#[derive(Debug, Serialize)]
pub struct UserTenantInfo {
    /// Tenant UUID
    pub tenant_id: String,
    /// Tenant display name
    pub name: String,
    /// Tenant slug
    pub slug: String,
    /// User's role in this tenant
    pub role: String,
    /// Whether this is the currently active tenant
    pub is_active: bool,
}

/// Tenant management routes
pub struct TenantRoutes;

impl TenantRoutes {
    /// Create all tenant management routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/tenants", post(Self::handle_create_tenant))
            .route("/tenants", get(Self::handle_list_tenants))
            .route("/tenants/switch", post(Self::handle_switch_tenant))
            .route("/tenants/my", get(Self::handle_list_my_tenants))
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

    /// Handle switching active tenant
    ///
    /// Validates that the user belongs to the target tenant, then returns a new JWT
    /// with the `active_tenant_id` claim set to the specified tenant.
    async fn handle_switch_tenant(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<SwitchTenantRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            target_tenant = %request.tenant_id,
            "Processing tenant switch request"
        );

        // Parse the target tenant ID
        let tenant_id = parse_uuid(&request.tenant_id).map_err(|e| {
            warn!(tenant_id = %request.tenant_id, error = %e, "Invalid tenant ID format");
            AppError::invalid_input(format!("Invalid tenant ID format: {e}"))
        })?;

        // Verify user belongs to this tenant via tenant_users table
        let role_str = resources
            .database
            .get_user_tenant_role(auth.user_id, tenant_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to check tenant membership: {e}")))?
            .ok_or_else(|| {
                warn!(
                    user_id = %auth.user_id,
                    tenant_id = %tenant_id,
                    "User attempted to switch to tenant they don't belong to"
                );
                AppError::auth_invalid(format!("User does not belong to tenant {tenant_id}"))
            })?;

        // Get tenant details
        let tenant = resources
            .database
            .get_tenant_by_id(tenant_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get tenant: {e}")))?;

        // Get user to generate new token
        let user = resources
            .database
            .get_user(auth.user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))?;

        // Generate new JWT with active_tenant_id set
        let token = resources
            .auth_manager
            .generate_token_with_tenant(&user, &resources.jwks_manager, Some(tenant_id.to_string()))
            .map_err(|e| AppError::internal(format!("Failed to generate token: {e}")))?;

        info!(
            user_id = %auth.user_id,
            tenant_id = %tenant_id,
            tenant_name = %tenant.name,
            "Successfully switched tenant context"
        );

        Ok((
            StatusCode::OK,
            Json(SwitchTenantResponse {
                token,
                active_tenant_id: tenant_id.to_string(),
                tenant_name: tenant.name,
                role: role_str,
                expires_in: 86400, // 24 hours (matches default token expiry)
            }),
        )
            .into_response())
    }

    /// Handle listing all tenants the user belongs to
    ///
    /// Returns a list of all tenants the user is a member of, along with their role
    /// in each tenant and which one is currently active.
    async fn handle_list_my_tenants(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate(&headers, &resources).await?;

        // Get the current active tenant from JWT claims (if any)
        let active_tenant_id = Self::extract_active_tenant_from_header(&headers, &resources);

        // Get all tenants the user belongs to
        let tenants = resources
            .database
            .list_tenants_for_user(auth.user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to list user tenants: {e}")))?;

        let mut tenant_infos = Vec::with_capacity(tenants.len());

        for tenant in tenants {
            // Get user's role in this tenant
            let role = resources
                .database
                .get_user_tenant_role(auth.user_id, tenant.id)
                .await
                .map_err(|e| AppError::database(format!("Failed to get tenant role: {e}")))?
                .unwrap_or_else(|| "member".to_owned());

            let is_active = active_tenant_id
                .as_ref()
                .is_some_and(|active| *active == tenant.id);

            tenant_infos.push(UserTenantInfo {
                tenant_id: tenant.id.to_string(),
                name: tenant.name,
                slug: tenant.slug,
                role,
                is_active,
            });
        }

        Ok((
            StatusCode::OK,
            Json(UserTenantsResponse {
                tenants: tenant_infos,
                active_tenant_id: active_tenant_id.map(|id| id.to_string()),
            }),
        )
            .into_response())
    }

    /// Extract active tenant ID from Authorization header JWT claims
    fn extract_active_tenant_from_header(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Option<uuid::Uuid> {
        let auth_header = headers.get("authorization")?.to_str().ok()?;
        let token = auth_header.strip_prefix("Bearer ")?;

        let claims = resources
            .auth_manager
            .validate_token(token, &resources.jwks_manager)
            .ok()?;

        claims
            .effective_tenant_id()
            .and_then(|tid| tid.parse().ok())
    }
}
