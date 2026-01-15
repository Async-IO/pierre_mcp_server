// ABOUTME: Admin API routes for per-tenant MCP tool selection management
// ABOUTME: Enables admins to view, configure, and override tool availability per tenant
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tool selection admin routes for managing MCP tool availability per tenant.
//!
//! This module provides REST endpoints for:
//! - Viewing the tool catalog
//! - Managing per-tenant tool overrides
//! - Checking globally disabled tools

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::task::yield_now;
use tracing::info;
use uuid::Uuid;

use crate::{
    admin::models::{AdminPermission, ValidatedAdminToken},
    errors::{AppError, AppResult},
    mcp::ToolSelectionService,
};

/// Context for tool selection routes
#[derive(Clone)]
pub struct ToolSelectionContext {
    /// Tool selection service for business logic
    pub tool_selection: Arc<ToolSelectionService>,
}

/// Response wrapper for tool selection endpoints
#[derive(Debug, Serialize)]
pub struct ToolSelectionResponse<T> {
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Response data (if successful)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T: Serialize> ToolSelectionResponse<T> {
    fn success(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Response for globally disabled tools
#[derive(Debug, Serialize)]
pub struct GlobalDisabledToolsResponse {
    /// List of tool names disabled via `PIERRE_DISABLED_TOOLS`
    pub disabled_tools: Vec<String>,
    /// Number of disabled tools
    pub count: usize,
}

/// Tool selection admin routes
pub struct ToolSelectionRoutes;

impl ToolSelectionRoutes {
    /// Create all tool selection routes
    pub fn routes(context: ToolSelectionContext) -> Router {
        let context = Arc::new(context);

        Router::new()
            // Catalog routes (read-only)
            .route("/admin/tools/catalog", get(Self::handle_get_catalog))
            .route(
                "/admin/tools/catalog/:tool_name",
                get(Self::handle_get_catalog_entry),
            )
            // Tenant configuration routes
            .route(
                "/admin/tools/tenant/:tenant_id",
                get(Self::handle_get_tenant_tools),
            )
            .route(
                "/admin/tools/tenant/:tenant_id/override",
                post(Self::handle_set_override),
            )
            .route(
                "/admin/tools/tenant/:tenant_id/override/:tool_name",
                delete(Self::handle_remove_override),
            )
            .route(
                "/admin/tools/tenant/:tenant_id/summary",
                get(Self::handle_get_summary),
            )
            // Global status
            .route(
                "/admin/tools/global-disabled",
                get(Self::handle_get_global_disabled),
            )
            .with_state(context)
    }

    /// GET /admin/tools/catalog - List all tools in catalog
    async fn handle_get_catalog(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ViewConfiguration)?;

        let catalog = context.tool_selection.get_catalog().await?;

        Ok((
            StatusCode::OK,
            Json(ToolSelectionResponse::success(
                format!("Retrieved {} tools from catalog", catalog.len()),
                catalog,
            )),
        ))
    }

    /// GET `/admin/tools/catalog/:tool_name` - Get single tool details
    async fn handle_get_catalog_entry(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(tool_name): Path<String>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ViewConfiguration)?;

        let catalog = context.tool_selection.get_catalog().await?;
        let entry = catalog
            .into_iter()
            .find(|e| e.tool_name == tool_name)
            .ok_or_else(|| AppError::not_found(format!("Tool '{tool_name}'")))?;

        Ok((
            StatusCode::OK,
            Json(ToolSelectionResponse::success(
                format!("Retrieved tool '{tool_name}'"),
                entry,
            )),
        ))
    }

    /// GET `/admin/tools/tenant/:tenant_id` - Get effective tools for tenant
    async fn handle_get_tenant_tools(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(tenant_id): Path<Uuid>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ViewConfiguration)?;

        let tools = context
            .tool_selection
            .get_effective_tools(tenant_id)
            .await?;

        Ok((
            StatusCode::OK,
            Json(ToolSelectionResponse::success(
                format!(
                    "Retrieved {} effective tools for tenant {tenant_id}",
                    tools.len()
                ),
                tools,
            )),
        ))
    }

    /// POST `/admin/tools/tenant/:tenant_id/override` - Set tool override
    async fn handle_set_override(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(tenant_id): Path<Uuid>,
        Json(request): Json<SetOverrideRequest>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ManageConfiguration)?;

        info!(
            "Setting tool override: tenant={}, tool={}, enabled={}, by={}",
            tenant_id, request.tool_name, request.is_enabled, admin_token.service_name
        );

        // Parse admin token ID as UUID for audit trail
        let admin_user_id = Uuid::parse_str(&admin_token.token_id)
            .map_err(|e| AppError::invalid_input(format!("Invalid admin token ID: {e}")))?;

        let override_entry = context
            .tool_selection
            .set_tool_override(
                tenant_id,
                &request.tool_name,
                request.is_enabled,
                admin_user_id,
                request.reason.clone(),
            )
            .await?;

        let action = if request.is_enabled {
            "enabled"
        } else {
            "disabled"
        };
        Ok((
            StatusCode::OK,
            Json(ToolSelectionResponse::success(
                format!(
                    "Tool '{}' {} for tenant {tenant_id}",
                    request.tool_name, action
                ),
                override_entry,
            )),
        ))
    }

    /// DELETE `/admin/tools/tenant/:tenant_id/override/:tool_name` - Remove override
    async fn handle_remove_override(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path((tenant_id, tool_name)): Path<(Uuid, String)>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ManageConfiguration)?;

        info!(
            "Removing tool override: tenant={}, tool={}, by={}",
            tenant_id, tool_name, admin_token.service_name
        );

        let deleted = context
            .tool_selection
            .remove_tool_override(tenant_id, &tool_name)
            .await?;

        if deleted {
            Ok((
                StatusCode::OK,
                Json(ToolSelectionResponse::<()>::success(
                    format!("Override removed for tool '{tool_name}' on tenant {tenant_id}"),
                    (),
                )),
            ))
        } else {
            Err(AppError::not_found(format!(
                "No override found for tool '{tool_name}' on tenant {tenant_id}"
            )))
        }
    }

    /// GET `/admin/tools/tenant/:tenant_id/summary` - Get availability summary
    async fn handle_get_summary(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(tenant_id): Path<Uuid>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ViewConfiguration)?;

        let summary = context
            .tool_selection
            .get_availability_summary(tenant_id)
            .await?;

        Ok((
            StatusCode::OK,
            Json(ToolSelectionResponse::success(
                format!(
                    "Tenant {tenant_id}: {}/{} tools enabled",
                    summary.enabled_tools, summary.total_tools
                ),
                summary,
            )),
        ))
    }

    /// GET `/admin/tools/global-disabled` - List `PIERRE_DISABLED_TOOLS` values
    async fn handle_get_global_disabled(
        State(context): State<Arc<ToolSelectionContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> AppResult<impl IntoResponse> {
        admin_token.require_permission(&AdminPermission::ViewConfiguration)?;

        // Yield to satisfy async requirement (Axum handlers must be async)
        yield_now().await;

        let disabled_tools = context.tool_selection.get_globally_disabled_tools();
        let count = disabled_tools.len();

        Ok((
            StatusCode::OK,
            Json(ToolSelectionResponse::success(
                if count == 0 {
                    "No tools are globally disabled".to_owned()
                } else {
                    format!("{count} tool(s) globally disabled via PIERRE_DISABLED_TOOLS")
                },
                GlobalDisabledToolsResponse {
                    disabled_tools,
                    count,
                },
            )),
        ))
    }
}

/// Request body for setting a tool override
#[derive(Debug, Deserialize)]
pub struct SetOverrideRequest {
    /// Name of the tool to override
    pub tool_name: String,
    /// Whether the tool should be enabled
    pub is_enabled: bool,
    /// Optional reason for the override
    pub reason: Option<String>,
}
