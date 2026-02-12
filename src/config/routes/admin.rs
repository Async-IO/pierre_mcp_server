// ABOUTME: HTTP REST endpoints for admin configuration management
// ABOUTME: Provides endpoints for viewing, updating, and auditing runtime configuration
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Admin Configuration Routes
//!
//! HTTP endpoints for managing runtime configuration parameters through
//! an admin API. Supports viewing the full catalog, updating values,
//! resetting to defaults, and viewing audit history.

use crate::config::admin::{
    AdminConfigService, ConfigAuditFilter, ConfigAuditResponse, ResetConfigRequest,
    UpdateConfigRequest, ValidateConfigRequest,
};
use crate::errors::{AppError, AppResult};
use crate::mcp::resources::ServerResources;
use crate::middleware::require_admin;
use crate::security::cookies::get_cookie_value;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::info;

/// Shared state for admin config routes
#[derive(Clone)]
pub struct AdminConfigState {
    /// Admin configuration service
    pub service: Arc<AdminConfigService>,
    /// Server resources for authentication
    pub resources: Arc<ServerResources>,
}

impl AdminConfigState {
    /// Create new admin config state
    #[must_use]
    pub const fn new(service: Arc<AdminConfigService>, resources: Arc<ServerResources>) -> Self {
        Self { service, resources }
    }

    /// Authenticate user from authorization header or cookie, requiring admin privileges
    async fn authenticate_admin(&self, headers: &HeaderMap) -> Result<AdminAuthInfo, AppError> {
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) = get_cookie_value(headers, "auth_token") {
                format!("Bearer {token}")
            } else {
                return Err(AppError::auth_invalid(
                    "Missing authorization header or cookie",
                ));
            };

        let auth = self
            .resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))?;

        // Verify admin privileges using centralized guard
        let user = require_admin(auth.user_id, &self.resources.database).await?;

        Ok(AdminAuthInfo {
            user_id: auth.user_id.to_string(),
            email: user.email,
        })
    }
}

/// Authenticated admin info for audit logging
struct AdminAuthInfo {
    user_id: String,
    email: String,
}

// ============================================================================
// Request/Response Types
// ============================================================================

/// Query parameters for catalog endpoint
#[derive(Debug, Deserialize)]
pub struct CatalogQuery {
    /// Optional tenant ID for tenant-specific overrides
    pub tenant_id: Option<String>,
}

/// Query parameters for audit log endpoint
#[derive(Debug, Deserialize)]
pub struct AuditLogQuery {
    /// Filter by category
    pub category: Option<String>,
    /// Filter by parameter key
    pub config_key: Option<String>,
    /// Filter by admin user ID
    pub admin_user_id: Option<String>,
    /// Filter by tenant ID
    pub tenant_id: Option<String>,
    /// Maximum results to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Query parameters for update/reset endpoints
#[derive(Debug, Deserialize)]
pub struct TenantQuery {
    /// Optional tenant ID for tenant-specific changes
    pub tenant_id: Option<String>,
}

/// Generic success response
#[derive(Debug, Serialize)]
pub struct AdminConfigApiResponse<T> {
    /// Whether the operation succeeded
    pub success: bool,
    /// Response data
    pub data: T,
}

// ============================================================================
// Route Handlers
// ============================================================================

/// Get the full configuration catalog with all parameters and metadata
///
/// `GET /api/admin/config/catalog`
///
/// Returns all configuration categories, parameters, current values,
/// defaults, validation rules, and metadata.
///
/// # Errors
///
/// Returns an error if the catalog cannot be retrieved from the database.
pub async fn get_catalog(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Query(query): Query<CatalogQuery>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    info!(
        user_id = %auth.user_id,
        tenant_id = ?query.tenant_id,
        "Admin fetching configuration catalog"
    );

    let catalog = state
        .service
        .get_catalog(query.tenant_id.as_deref())
        .await?;

    Ok(Json(AdminConfigApiResponse {
        success: true,
        data: catalog,
    }))
}

/// Get current configuration values
///
/// `GET /api/admin/config`
///
/// Returns the current effective configuration values (defaults + overrides)
///
/// # Errors
///
/// Returns an error if the configuration cannot be retrieved from the database.
pub async fn get_config(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Query(query): Query<TenantQuery>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    info!(
        user_id = %auth.user_id,
        tenant_id = ?query.tenant_id,
        "Admin fetching current configuration"
    );

    let catalog = state
        .service
        .get_catalog(query.tenant_id.as_deref())
        .await?;

    Ok(Json(AdminConfigApiResponse {
        success: true,
        data: catalog,
    }))
}

/// Get configuration for a specific category
///
/// `GET /api/admin/config/category/:category_name`
///
/// Returns parameters for a specific category
///
/// # Errors
///
/// Returns an error if the category is not found or database access fails.
pub async fn get_category_config(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Path(category_name): Path<String>,
    Query(query): Query<TenantQuery>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    info!(
        user_id = %auth.user_id,
        category = %category_name,
        tenant_id = ?query.tenant_id,
        "Admin fetching category configuration"
    );

    let mut catalog = state
        .service
        .get_catalog(query.tenant_id.as_deref())
        .await?;

    // Filter to requested category
    catalog.categories.retain(|c| c.name == category_name);

    if catalog.categories.is_empty() {
        return Err(AppError::not_found(format!(
            "Category '{category_name}' not found"
        )));
    }

    // Update counts for filtered catalog
    catalog.total_parameters = catalog.categories.iter().map(|c| c.parameters.len()).sum();
    catalog.runtime_configurable_count = catalog
        .categories
        .iter()
        .flat_map(|c| &c.parameters)
        .filter(|p| p.is_runtime_configurable)
        .count();
    catalog.static_count = catalog.total_parameters - catalog.runtime_configurable_count;

    Ok(Json(AdminConfigApiResponse {
        success: true,
        data: catalog,
    }))
}

/// Validate configuration values before applying
///
/// `POST /api/admin/config/validate`
///
/// Validates proposed configuration changes without applying them
///
/// # Errors
///
/// Returns an error if validation cannot be performed.
pub async fn validate_config(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Json(request): Json<ValidateConfigRequest>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    info!(
        user_id = %auth.user_id,
        parameter_count = request.parameters.len(),
        "Admin validating configuration changes"
    );

    let validation = state.service.validate(&request).await;

    Ok(Json(AdminConfigApiResponse {
        success: validation.is_valid,
        data: validation,
    }))
}

/// Update configuration values
///
/// `PUT /api/admin/config`
///
/// Updates one or more configuration parameters
///
/// # Errors
///
/// Returns an error if the update fails due to validation or database errors.
pub async fn update_config(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Query(query): Query<TenantQuery>,
    Json(request): Json<UpdateConfigRequest>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    let user_id = auth.user_id;
    let user_email = &auth.email;
    info!(
        user_id = %user_id,
        tenant_id = ?query.tenant_id,
        parameter_count = request.parameters.len(),
        "Admin updating configuration"
    );

    let response = state
        .service
        .update_config(
            &request,
            &user_id,
            user_email,
            query.tenant_id.as_deref(),
            None, // IP address - would come from request headers in production
            None, // User agent - would come from request headers in production
        )
        .await?;

    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    Ok((
        status,
        Json(AdminConfigApiResponse {
            success: response.success,
            data: response,
        }),
    ))
}

/// Update configuration for a specific category
///
/// `PUT /api/admin/config/category/:category_name`
///
/// Updates parameters within a specific category
///
/// # Errors
///
/// Returns an error if the category is not found or update fails.
pub async fn update_category_config(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Path(category_name): Path<String>,
    Query(query): Query<TenantQuery>,
    Json(request): Json<UpdateConfigRequest>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    let user_id = auth.user_id;
    let user_email = &auth.email;
    info!(
        user_id = %user_id,
        category = %category_name,
        tenant_id = ?query.tenant_id,
        parameter_count = request.parameters.len(),
        "Admin updating category configuration"
    );

    // Filter parameters to only those in the requested category
    let catalog = state
        .service
        .get_catalog(query.tenant_id.as_deref())
        .await?;
    let category_keys: HashSet<String> = catalog
        .categories
        .iter()
        .find(|c| c.name == category_name)
        .map(|c| c.parameters.iter().map(|p| p.key.clone()).collect())
        .unwrap_or_default();

    if category_keys.is_empty() {
        return Err(AppError::not_found(format!(
            "Category '{category_name}' not found"
        )));
    }

    // Filter request to only include parameters from this category
    let filtered_params: HashMap<String, serde_json::Value> = request
        .parameters
        .into_iter()
        .filter(|(k, _)| category_keys.contains(k))
        .collect();

    let filtered_request = UpdateConfigRequest {
        parameters: filtered_params,
        reason: request.reason,
    };

    let response = state
        .service
        .update_config(
            &filtered_request,
            &user_id,
            user_email,
            query.tenant_id.as_deref(),
            None,
            None,
        )
        .await?;

    let status = if response.success {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    };

    Ok((
        status,
        Json(AdminConfigApiResponse {
            success: response.success,
            data: response,
        }),
    ))
}

/// Reset configuration to defaults
///
/// `POST /api/admin/config/reset`
///
/// Resets configuration parameters to their default values
///
/// # Errors
///
/// Returns an error if the reset operation fails.
pub async fn reset_config(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Query(query): Query<TenantQuery>,
    Json(request): Json<ResetConfigRequest>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    let user_id = auth.user_id;
    let user_email = &auth.email;
    info!(
        user_id = %user_id,
        tenant_id = ?query.tenant_id,
        category = ?request.category,
        "Admin resetting configuration"
    );

    let response = state
        .service
        .reset_config(
            &request,
            &user_id,
            user_email,
            query.tenant_id.as_deref(),
            None,
            None,
        )
        .await?;

    Ok(Json(AdminConfigApiResponse {
        success: response.success,
        data: response,
    }))
}

/// Get configuration audit log
///
/// `GET /api/admin/config/history`
///
/// Returns the audit log of configuration changes
///
/// # Errors
///
/// Returns an error if the audit log cannot be retrieved.
pub async fn get_audit_log(
    State(state): State<Arc<AdminConfigState>>,
    headers: HeaderMap,
    Query(query): Query<AuditLogQuery>,
) -> AppResult<impl IntoResponse> {
    let auth = state.authenticate_admin(&headers).await?;
    info!(
        user_id = %auth.user_id,
        "Admin fetching configuration audit log"
    );

    let filter = ConfigAuditFilter {
        category: query.category,
        config_key: query.config_key,
        admin_user_id: query.admin_user_id,
        tenant_id: query.tenant_id,
        from_timestamp: None,
        to_timestamp: None,
    };

    let limit = query.limit.unwrap_or(50).min(500);
    let offset = query.offset.unwrap_or(0);

    let (entries, total_count) = state.service.get_audit_log(&filter, limit, offset).await?;

    Ok(Json(AdminConfigApiResponse {
        success: true,
        data: ConfigAuditResponse {
            entries,
            total_count,
            offset,
            limit,
        },
    }))
}

// ============================================================================
// Router Builder
// ============================================================================

/// Build the admin configuration router
///
/// This creates the router for `/api/admin/config/*` endpoints
pub fn admin_config_router(state: Arc<AdminConfigState>) -> axum::Router {
    use axum::routing::{get, post, put};

    axum::Router::new()
        .route("/catalog", get(get_catalog))
        .route("/", get(get_config))
        .route("/", put(update_config))
        .route("/category/:category_name", get(get_category_config))
        .route("/category/:category_name", put(update_category_config))
        .route("/validate", post(validate_config))
        .route("/reset", post(reset_config))
        .route("/audit", get(get_audit_log))
        .with_state(state)
}
