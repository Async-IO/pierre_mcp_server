// ABOUTME: Web-facing admin routes for authenticated admin users via browser
// ABOUTME: Uses cookie-based auth (same as /api/keys) for users with is_admin=true
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Web Admin Routes
//!
//! This module provides admin endpoints accessible via browser cookie authentication.
//! Unlike `/admin/*` routes which require admin service tokens, these routes
//! accept standard user authentication for users with `is_admin: true`.

use crate::{
    admin::{models::CreateAdminTokenRequest, AdminPermission},
    auth::AuthResult,
    database::CreateUserMcpTokenRequest,
    database_plugins::DatabaseProvider,
    errors::{AppError, ErrorCode},
    mcp::resources::ServerResources,
    models::UserStatus,
    rate_limiting::UnifiedRateLimitCalculator,
    security::cookies::get_cookie_value,
};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Response for pending users list
#[derive(Serialize)]
struct PendingUsersResponse {
    count: usize,
    users: Vec<UserSummary>,
}

/// Response for all users list
#[derive(Serialize)]
struct AllUsersResponse {
    users: Vec<UserSummaryFull>,
    total_count: usize,
}

/// Response for admin tokens list
#[derive(Serialize)]
struct AdminTokensResponse {
    admin_tokens: Vec<AdminTokenSummary>,
    total_count: usize,
}

/// Admin token summary for listing
#[derive(Serialize)]
struct AdminTokenSummary {
    id: String,
    service_name: String,
    service_description: Option<String>,
    is_active: bool,
    is_super_admin: bool,
    created_at: String,
    expires_at: Option<String>,
    last_used_at: Option<String>,
    token_prefix: Option<String>,
}

/// Full user summary for listing all users
#[derive(Serialize)]
struct UserSummaryFull {
    id: String,
    email: String,
    display_name: Option<String>,
    tier: String,
    user_status: String,
    is_admin: bool,
    created_at: String,
    last_active: String,
    approved_at: Option<String>,
    approved_by: Option<String>,
}

/// User summary for listing
#[derive(Serialize)]
struct UserSummary {
    id: String,
    email: String,
    display_name: Option<String>,
    tier: String,
    created_at: String,
    last_active: String,
}

/// Request to approve a user
#[derive(Deserialize)]
struct ApproveUserRequest {
    reason: Option<String>,
}

/// Request to suspend a user
#[derive(Deserialize)]
struct SuspendUserRequest {
    reason: Option<String>,
}

/// Request to set a tool override
#[derive(Deserialize)]
struct SetToolOverrideRequest {
    tool_name: String,
    is_enabled: bool,
    reason: Option<String>,
}

/// Request to create an admin token via web admin
#[derive(Deserialize)]
struct CreateAdminTokenWebRequest {
    service_name: String,
    service_description: Option<String>,
    permissions: Option<Vec<String>>,
    is_super_admin: Option<bool>,
    expires_in_days: Option<u64>,
}

/// Response for created admin token
#[derive(Serialize)]
struct CreateAdminTokenWebResponse {
    success: bool,
    token_id: String,
    service_name: String,
    jwt_token: String,
    token_prefix: String,
    is_super_admin: bool,
    expires_at: Option<String>,
}

/// Response for user status change operations
#[derive(Serialize)]
struct UserStatusChangeResponse {
    success: bool,
    message: String,
    user: UserStatusChangeUser,
}

/// User data in status change response
#[derive(Serialize)]
struct UserStatusChangeUser {
    id: String,
    email: String,
    user_status: String,
}

/// Query parameters for user activity endpoint
#[derive(Debug, Deserialize)]
pub struct UserActivityQuery {
    /// Number of days to look back (default: 30)
    pub days: Option<u32>,
}

/// Auto-create a default MCP token for a newly activated user.
/// This is a non-fatal operation - failure is logged but does not propagate.
async fn create_default_mcp_token_for_user(database: &impl DatabaseProvider, user_id: uuid::Uuid) {
    let token_request = CreateUserMcpTokenRequest {
        name: "Default Token".to_owned(),
        expires_in_days: None, // Never expires
    };

    match database
        .create_user_mcp_token(user_id, &token_request)
        .await
    {
        Ok(token_result) => {
            info!(
                user_id = %user_id,
                token_id = %token_result.token.id,
                "Auto-created default MCP token for user"
            );
        }
        Err(e) => {
            // Log error but don't fail - user can create token manually
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to auto-create MCP token for user (non-fatal)"
            );
        }
    }
}

/// Assigns a user to the admin's tenant for multi-tenant isolation.
/// This ensures the user sees the same prompts, configuration, etc. as other users in the tenant.
async fn assign_user_to_admin_tenant(
    resources: &Arc<ServerResources>,
    admin_user_id: uuid::Uuid,
    target_user_id: uuid::Uuid,
) -> Result<(), AppError> {
    let admin_user = resources
        .database
        .get_user(admin_user_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch admin user for tenant assignment");
            AppError::internal(format!("Failed to fetch admin user: {e}"))
        })?
        .ok_or_else(|| {
            error!("Admin user not found during approval");
            AppError::internal("Admin user not found")
        })?;

    // Get admin's tenant from tenant_users junction table
    let admin_tenants = resources
        .database
        .list_tenants_for_user(admin_user.id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get admin's tenants");
            AppError::internal(format!("Failed to get admin tenants: {e}"))
        })?;

    if let Some(admin_tenant) = admin_tenants.first() {
        // Update user's tenant_id in users table (kept in sync with tenant_users junction)
        resources
            .database
            .update_user_tenant_id(target_user_id, &admin_tenant.id.to_string())
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to assign user to admin's tenant");
                AppError::internal(format!("Failed to assign tenant: {e}"))
            })?;
        info!(
            user_id = %target_user_id,
            tenant_id = %admin_tenant.id,
            "Assigned approved user to admin's tenant"
        );
    }
    Ok(())
}

/// Get the admin user's tenant scope for listing queries.
///
/// Super-admins see all tenants (returns None). Regular admins are scoped
/// to their first tenant membership (returns `Some(tenant_id)`).
async fn get_admin_tenant_scope(
    resources: &Arc<ServerResources>,
    admin_user_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    let user = resources
        .database
        .get_user(admin_user_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch admin user: {e}")))?
        .ok_or_else(|| AppError::not_found("Admin user not found"))?;

    // Super-admins see all tenants
    if user.role.is_super_admin() {
        return Ok(None);
    }

    // Regular admins are scoped to their tenant
    let admin_tenants = resources
        .database
        .list_tenants_for_user(admin_user_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to get admin tenants: {e}")))?;

    Ok(admin_tenants.first().map(|t| t.id))
}

/// Verify an admin user belongs to the target tenant.
///
/// Super-admin users can access any tenant. Regular admins are restricted
/// to tenants they belong to via the `tenant_users` junction table.
async fn verify_admin_tenant_access(
    resources: &Arc<ServerResources>,
    admin_user_id: Uuid,
    target_tenant_id: Uuid,
) -> Result<(), AppError> {
    let user = resources
        .database
        .get_user(admin_user_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch admin user: {e}")))?
        .ok_or_else(|| AppError::not_found("Admin user not found"))?;

    // Super-admins can access any tenant
    if user.role.is_super_admin() {
        return Ok(());
    }

    // Regular admins must belong to the target tenant
    let admin_tenants = resources
        .database
        .list_tenants_for_user(admin_user_id)
        .await
        .map_err(|e| AppError::internal(format!("Failed to get admin tenants: {e}")))?;

    let belongs_to_tenant = admin_tenants.iter().any(|t| t.id == target_tenant_id);

    if belongs_to_tenant {
        Ok(())
    } else {
        Err(AppError::new(
            ErrorCode::PermissionDenied,
            "Admin does not belong to the target tenant",
        ))
    }
}

/// Web admin routes - accessible via browser for admin users
pub struct WebAdminRoutes;

impl WebAdminRoutes {
    /// Create all web admin routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/api/admin/pending-users", get(Self::handle_pending_users))
            .route("/api/admin/users", get(Self::handle_all_users))
            .route(
                "/api/admin/tokens",
                get(Self::handle_admin_tokens).post(Self::handle_create_admin_token),
            )
            .route(
                "/api/admin/tokens/:token_id",
                get(Self::handle_get_admin_token),
            )
            .route(
                "/api/admin/tokens/:token_id/revoke",
                post(Self::handle_revoke_admin_token),
            )
            .route(
                "/api/admin/approve-user/:user_id",
                post(Self::handle_approve_user),
            )
            .route(
                "/api/admin/suspend-user/:user_id",
                post(Self::handle_suspend_user),
            )
            .route(
                "/api/admin/users/:user_id/reset-password",
                post(Self::handle_reset_user_password),
            )
            .route(
                "/api/admin/users/:user_id/rate-limit",
                get(Self::handle_get_user_rate_limit),
            )
            .route(
                "/api/admin/users/:user_id/activity",
                get(Self::handle_get_user_activity),
            )
            .route(
                "/api/admin/settings/auto-approval",
                get(Self::handle_get_auto_approval).put(Self::handle_set_auto_approval),
            )
            // Tool selection routes (web admin versions with cookie auth)
            .route(
                "/api/admin/tools/catalog",
                get(Self::handle_get_tool_catalog),
            )
            .route(
                "/api/admin/tools/catalog/:tool_name",
                get(Self::handle_get_tool_catalog_entry),
            )
            .route(
                "/api/admin/tools/global-disabled",
                get(Self::handle_get_global_disabled_tools),
            )
            .route(
                "/api/admin/tools/tenant/:tenant_id",
                get(Self::handle_get_tenant_tools),
            )
            .route(
                "/api/admin/tools/tenant/:tenant_id/override",
                post(Self::handle_set_tool_override),
            )
            .route(
                "/api/admin/tools/tenant/:tenant_id/override/:tool_name",
                delete(Self::handle_remove_tool_override),
            )
            .route(
                "/api/admin/tools/tenant/:tenant_id/summary",
                get(Self::handle_get_tool_summary),
            )
            .with_state(resources)
    }

    /// Authenticate user from authorization header or cookie, requiring admin privileges
    async fn authenticate_admin(
        headers: &HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<AuthResult, AppError> {
        // Try Authorization header first, then fall back to auth_token cookie
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

        let auth = resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))?;

        // Check if user has admin role or higher (admin or super_admin)
        let user = resources
            .database
            .get_user(auth.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !user.role.is_admin_or_higher() {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Admin privileges required",
            ));
        }

        Ok(auth)
    }

    /// Handle pending users listing for web admin users
    async fn handle_pending_users(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            "Web admin listing pending users"
        );

        // Scope listing to admin's tenant (super-admins see all tenants)
        let admin_tenant_id = get_admin_tenant_scope(&resources, auth.user_id).await?;

        // Fetch users with Pending status
        let users = resources
            .database
            .get_users_by_status("pending", admin_tenant_id)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch pending users from database");
                AppError::internal(format!("Failed to fetch pending users: {e}"))
            })?;

        // Convert to summaries
        let user_summaries: Vec<UserSummary> = users
            .iter()
            .map(|user| UserSummary {
                id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name.clone(),
                tier: user.tier.to_string(),
                created_at: user.created_at.to_rfc3339(),
                last_active: user.last_active.to_rfc3339(),
            })
            .collect();

        let count = user_summaries.len();

        info!("Retrieved {count} pending users for web admin");

        Ok((
            StatusCode::OK,
            Json(PendingUsersResponse {
                count,
                users: user_summaries,
            }),
        )
            .into_response())
    }

    /// Handle listing all users for web admin users
    async fn handle_all_users(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            "Web admin listing all users"
        );

        // Scope listing to admin's tenant (super-admins see all tenants)
        let admin_tenant_id = get_admin_tenant_scope(&resources, auth.user_id).await?;

        // Fetch users by status and combine (no get_all_users method exists)
        let mut all_users = Vec::new();

        for status in ["active", "pending", "suspended"] {
            let users = resources
                .database
                .get_users_by_status(status, admin_tenant_id)
                .await
                .map_err(|e| {
                    error!(error = %e, status = status, "Failed to fetch users from database");
                    AppError::internal(format!("Failed to fetch {status} users: {e}"))
                })?;
            all_users.extend(users);
        }

        let users = all_users;

        // Convert to full summaries
        let user_summaries: Vec<UserSummaryFull> = users
            .iter()
            .map(|user| UserSummaryFull {
                id: user.id.to_string(),
                email: user.email.clone(),
                display_name: user.display_name.clone(),
                tier: user.tier.to_string(),
                user_status: user.user_status.to_string(),
                is_admin: user.is_admin,
                created_at: user.created_at.to_rfc3339(),
                last_active: user.last_active.to_rfc3339(),
                approved_at: user.approved_at.map(|d| d.to_rfc3339()),
                approved_by: user.approved_by.map(|id| id.to_string()),
            })
            .collect();

        let total_count = user_summaries.len();

        info!("Retrieved {total_count} users for web admin");

        Ok((
            StatusCode::OK,
            Json(AllUsersResponse {
                users: user_summaries,
                total_count,
            }),
        )
            .into_response())
    }

    /// Handle listing admin tokens for web admin users
    async fn handle_admin_tokens(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            "Web admin listing admin tokens"
        );

        // Fetch admin tokens (include_inactive = false for active tokens only)
        let tokens = resources
            .database
            .list_admin_tokens(false)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch admin tokens from database");
                AppError::internal(format!("Failed to fetch admin tokens: {e}"))
            })?;

        // Convert to summaries
        let token_summaries: Vec<AdminTokenSummary> = tokens
            .iter()
            .map(|token| AdminTokenSummary {
                id: token.id.clone(),
                service_name: token.service_name.clone(),
                service_description: token.service_description.clone(),
                is_active: token.is_active,
                is_super_admin: token.is_super_admin,
                created_at: token.created_at.to_rfc3339(),
                expires_at: token.expires_at.map(|d| d.to_rfc3339()),
                last_used_at: token.last_used_at.map(|d| d.to_rfc3339()),
                token_prefix: Some(token.token_prefix.clone()),
            })
            .collect();

        let total_count = token_summaries.len();

        info!("Retrieved {total_count} admin tokens for web admin");

        Ok((
            StatusCode::OK,
            Json(AdminTokensResponse {
                admin_tokens: token_summaries,
                total_count,
            }),
        )
            .into_response())
    }

    /// Handle approving a user via web admin (cookie auth)
    async fn handle_approve_user(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(user_id): Path<String>,
        Json(request): Json<ApproveUserRequest>,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            admin_user_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin approving user"
        );

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&user_id).map_err(|e| {
            error!(error = %e, "Invalid user ID format");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

        // Get the user to approve
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch user from database");
                AppError::internal(format!("Failed to fetch user: {e}"))
            })?
            .ok_or_else(|| {
                warn!("User not found: {}", user_id);
                AppError::not_found("User not found")
            })?;

        if user.user_status == UserStatus::Active {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "message": "User is already approved"
                })),
            )
                .into_response());
        }

        // Use the admin user's UUID as the approver for proper audit trail
        let updated_user = resources
            .database
            .update_user_status(user_uuid, UserStatus::Active, Some(auth.user_id))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update user status in database");
                AppError::internal(format!("Failed to approve user: {e}"))
            })?;

        // Assign approved user to admin's tenant for multi-tenant isolation
        assign_user_to_admin_tenant(&resources, auth.user_id, user_uuid).await?;

        // Auto-create a default MCP token for the newly approved user
        create_default_mcp_token_for_user(resources.database.as_ref(), user_uuid).await;

        let reason = request.reason.as_deref().unwrap_or("No reason provided");
        info!("User {} approved successfully. Reason: {}", user_id, reason);

        Ok((
            StatusCode::OK,
            Json(UserStatusChangeResponse {
                success: true,
                message: "User approved successfully".to_owned(),
                user: UserStatusChangeUser {
                    id: updated_user.id.to_string(),
                    email: updated_user.email,
                    user_status: updated_user.user_status.to_string(),
                },
            }),
        )
            .into_response())
    }

    /// Handle suspending a user via web admin (cookie auth)
    async fn handle_suspend_user(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(user_id): Path<String>,
        Json(request): Json<SuspendUserRequest>,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            admin_user_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin suspending user"
        );

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&user_id).map_err(|e| {
            error!(error = %e, "Invalid user ID format");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

        // Get the user to suspend
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch user from database");
                AppError::internal(format!("Failed to fetch user: {e}"))
            })?
            .ok_or_else(|| {
                warn!("User not found: {}", user_id);
                AppError::not_found("User not found")
            })?;

        if user.user_status == UserStatus::Suspended {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "success": false,
                    "message": "User is already suspended"
                })),
            )
                .into_response());
        }

        // Use the admin user's UUID for audit trail (Note: approved_by is used for both approve/suspend)
        let updated_user = resources
            .database
            .update_user_status(user_uuid, UserStatus::Suspended, Some(auth.user_id))
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update user status in database");
                AppError::internal(format!("Failed to suspend user: {e}"))
            })?;

        let reason = request.reason.as_deref().unwrap_or("No reason provided");
        info!(
            "User {} suspended successfully. Reason: {}",
            user_id, reason
        );

        Ok((
            StatusCode::OK,
            Json(UserStatusChangeResponse {
                success: true,
                message: "User suspended successfully".to_owned(),
                user: UserStatusChangeUser {
                    id: updated_user.id.to_string(),
                    email: updated_user.email,
                    user_status: updated_user.user_status.to_string(),
                },
            }),
        )
            .into_response())
    }

    /// Verify the authenticated user has super-admin privileges
    async fn require_super_admin(
        user_id: Uuid,
        resources: &Arc<ServerResources>,
    ) -> Result<(), AppError> {
        let user = resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !user.role.is_super_admin() {
            warn!(
                user_id = %user_id,
                "Non-super-admin attempted privileged operation"
            );
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Super-admin privileges required to create super-admin tokens",
            ));
        }
        Ok(())
    }

    /// Build a `CreateAdminTokenRequest` from the web request payload
    fn build_admin_token_request(request: CreateAdminTokenWebRequest) -> CreateAdminTokenRequest {
        let permissions = request.permissions.map(|perms| {
            perms
                .iter()
                .filter_map(|p| p.parse::<AdminPermission>().ok())
                .collect::<Vec<_>>()
        });

        CreateAdminTokenRequest {
            service_name: request.service_name,
            service_description: request.service_description,
            permissions,
            expires_in_days: request.expires_in_days,
            is_super_admin: request.is_super_admin.unwrap_or(false),
        }
    }

    /// Handle creating an admin token via web admin (cookie auth)
    async fn handle_create_admin_token(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Json(request): Json<CreateAdminTokenWebRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        // Only super-admins can create super-admin tokens
        if request.is_super_admin.unwrap_or(false) {
            Self::require_super_admin(auth.user_id, &resources).await?;
        }

        info!(
            user_id = %auth.user_id,
            service_name = %request.service_name,
            "Web admin creating admin token"
        );

        let token_request = Self::build_admin_token_request(request);

        // Generate token using database method
        let generated_token = resources
            .database
            .create_admin_token(
                &token_request,
                &resources.admin_jwt_secret,
                &resources.jwks_manager,
            )
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to create admin token");
                AppError::internal(format!("Failed to create admin token: {e}"))
            })?;

        info!(
            token_id = %generated_token.token_id,
            "Admin token created successfully via web admin"
        );

        Ok((
            StatusCode::CREATED,
            Json(CreateAdminTokenWebResponse {
                success: true,
                token_id: generated_token.token_id,
                service_name: generated_token.service_name,
                jwt_token: generated_token.jwt_token,
                token_prefix: generated_token.token_prefix,
                is_super_admin: generated_token.is_super_admin,
                expires_at: generated_token.expires_at.map(|t| t.to_rfc3339()),
            }),
        )
            .into_response())
    }

    /// Handle getting a specific admin token via web admin (cookie auth)
    async fn handle_get_admin_token(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(token_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            token_id = %token_id,
            "Web admin getting admin token details"
        );

        let token = resources
            .database
            .get_admin_token_by_id(&token_id)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch admin token from database");
                AppError::internal(format!("Failed to fetch admin token: {e}"))
            })?
            .ok_or_else(|| AppError::not_found(format!("Admin token {token_id}")))?;

        Ok((
            StatusCode::OK,
            Json(AdminTokenSummary {
                id: token.id,
                service_name: token.service_name,
                service_description: token.service_description,
                is_active: token.is_active,
                is_super_admin: token.is_super_admin,
                created_at: token.created_at.to_rfc3339(),
                expires_at: token.expires_at.map(|d| d.to_rfc3339()),
                last_used_at: token.last_used_at.map(|d| d.to_rfc3339()),
                token_prefix: Some(token.token_prefix),
            }),
        )
            .into_response())
    }

    /// Handle revoking an admin token via web admin (cookie auth)
    async fn handle_revoke_admin_token(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(token_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            token_id = %token_id,
            "Web admin revoking admin token"
        );

        resources
            .database
            .deactivate_admin_token(&token_id)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to revoke admin token");
                AppError::internal(format!("Failed to revoke admin token: {e}"))
            })?;

        info!(
            "Admin token {} revoked successfully via web admin",
            token_id
        );

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Admin token revoked successfully",
                "token_id": token_id
            })),
        )
            .into_response())
    }

    /// Handle password reset via web admin
    ///
    /// Issues a one-time reset token instead of returning a temporary password.
    /// The admin delivers the token to the user, who calls `POST /api/auth/complete-reset`
    /// with the token and their chosen new password. Token expires after 1 hour.
    async fn handle_reset_user_password(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(user_id): Path<String>,
    ) -> Result<Response, AppError> {
        use rand::distributions::Alphanumeric;
        use rand::Rng;
        use sha2::{Digest, Sha256};

        let auth = Self::authenticate_admin(&headers, &resources).await?;

        info!(
            admin_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin issuing password reset token"
        );

        let user_uuid = Uuid::parse_str(&user_id)
            .map_err(|e| AppError::invalid_input(format!("Invalid user ID format: {e}")))?;

        // Verify user exists and get email for response
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // Generate a cryptographically random reset token (48 chars alphanumeric)
        let raw_token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(48)
            .map(char::from)
            .collect();

        // Store only the SHA-256 hash of the token in the database
        let token_hash = format!("{:x}", Sha256::digest(raw_token.as_bytes()));

        let admin_id_str = auth.user_id.to_string();
        resources
            .database
            .store_password_reset_token(user_uuid, &token_hash, &admin_id_str)
            .await
            .map_err(|e| AppError::internal(format!("Failed to create reset token: {e}")))?;

        info!(
            admin_id = %auth.user_id,
            target_user_id = %user_id,
            "Password reset token issued via web admin"
        );

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Password reset token issued",
                "data": {
                    "reset_token": raw_token,
                    "expires_in_seconds": 3600,
                    "user_email": user.email,
                    "note": "Deliver this token to the user. They must call POST /api/auth/complete-reset with the token and their new password within 1 hour."
                }
            })),
        )
            .into_response())
    }

    /// Handle getting rate limit info for a user via web admin
    async fn handle_get_user_rate_limit(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(user_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        debug!(
            admin_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin fetching user rate limit"
        );

        let user_uuid = uuid::Uuid::parse_str(&user_id)
            .map_err(|e| AppError::invalid_input(format!("Invalid user ID format: {e}")))?;

        // Get user
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // Get current monthly usage
        let monthly_used = resources
            .database
            .get_jwt_current_usage(user_uuid)
            .await
            .unwrap_or(0);

        // Get daily usage from activity logs (today's requests)
        let now = chrono::Utc::now();
        let today_start = now.date_naive().and_hms_opt(0, 0, 0).map_or(now, |t| {
            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(t, chrono::Utc)
        });
        let daily_used = resources
            .database
            .get_top_tools_analysis(user_uuid, today_start, now)
            .await
            .map(|tools| {
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                tools.iter().map(|t| t.request_count as u32).sum::<u32>()
            })
            .unwrap_or(0);

        // Calculate limits based on tier
        let monthly_limit = user.tier.monthly_limit();
        let daily_limit = monthly_limit.map(|m| m / 30);

        // Calculate remaining
        let monthly_remaining = monthly_limit.map(|l| l.saturating_sub(monthly_used));
        let daily_remaining = daily_limit.map(|l| l.saturating_sub(daily_used));

        // Calculate reset times
        let daily_reset = (now + chrono::Duration::days(1))
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map_or(now, |t| {
                chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(t, chrono::Utc)
            });
        let monthly_reset = UnifiedRateLimitCalculator::calculate_monthly_reset();

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Rate limit information retrieved",
                "data": {
                    "user_id": user_uuid.to_string(),
                    "tier": user.tier.to_string(),
                    "rate_limits": {
                        "daily": {
                            "limit": daily_limit,
                            "used": daily_used,
                            "remaining": daily_remaining,
                        },
                        "monthly": {
                            "limit": monthly_limit,
                            "used": monthly_used,
                            "remaining": monthly_remaining,
                        },
                    },
                    "reset_times": {
                        "daily_reset": daily_reset.to_rfc3339(),
                        "monthly_reset": monthly_reset.to_rfc3339(),
                    },
                }
            })),
        )
            .into_response())
    }

    /// Handle getting user activity via web admin
    async fn handle_get_user_activity(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(user_id): Path<String>,
        Query(params): Query<UserActivityQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        debug!(
            admin_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin fetching user activity"
        );

        let user_uuid = uuid::Uuid::parse_str(&user_id)
            .map_err(|e| AppError::invalid_input(format!("Invalid user ID format: {e}")))?;

        // Verify user exists
        resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // Get time range for activity using days parameter (default 30)
        let days = i64::from(params.days.unwrap_or(30).clamp(1, 365));
        let now = chrono::Utc::now();
        let start_time = now - chrono::Duration::days(days);

        // Get top tools usage
        let top_tools_raw = resources
            .database
            .get_top_tools_analysis(user_uuid, start_time, now)
            .await
            .unwrap_or_default();

        // Calculate total requests and percentages
        let total_requests: u64 = top_tools_raw.iter().map(|t| t.request_count).sum();
        let top_tools: Vec<serde_json::Value> = top_tools_raw
            .into_iter()
            .map(|t| {
                let percentage = if total_requests > 0 {
                    #[allow(clippy::cast_precision_loss)]
                    let pct = (t.request_count as f64 / total_requests as f64) * 100.0;
                    pct
                } else {
                    0.0
                };
                serde_json::json!({
                    "tool_name": t.tool_name,
                    "call_count": t.request_count,
                    "percentage": percentage,
                })
            })
            .collect();

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "User activity retrieved",
                "data": {
                    "user_id": user_uuid.to_string(),
                    "period_days": days,
                    "total_requests": total_requests,
                    "top_tools": top_tools,
                }
            })),
        )
            .into_response())
    }

    /// Handle getting auto-approval setting
    async fn handle_get_auto_approval(
        headers: HeaderMap,
        State(resources): State<Arc<ServerResources>>,
    ) -> Result<impl IntoResponse, AppError> {
        Self::authenticate_admin(&headers, &resources).await?;

        // Get effective auto-approval setting
        // Precedence: env var (if set) > database > default
        let enabled = if resources.config.app_behavior.auto_approve_users_from_env {
            resources.config.app_behavior.auto_approve_users
        } else {
            match resources.database.is_auto_approval_enabled().await {
                Ok(Some(db_setting)) => db_setting,
                Ok(None) => resources.config.app_behavior.auto_approve_users,
                Err(e) => {
                    error!(error = %e, "Failed to get auto-approval setting");
                    return Err(AppError::internal(format!(
                        "Failed to get auto-approval setting: {e}"
                    )));
                }
            }
        };

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Auto-approval setting retrieved",
                "data": {
                    "enabled": enabled,
                    "description": "When enabled, new user registrations are automatically approved without admin intervention"
                }
            })),
        )
            .into_response())
    }

    /// Handle setting auto-approval
    async fn handle_set_auto_approval(
        headers: HeaderMap,
        State(resources): State<Arc<ServerResources>>,
        Json(request): Json<serde_json::Value>,
    ) -> Result<impl IntoResponse, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        let enabled = request
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| AppError::invalid_input("Missing or invalid 'enabled' field"))?;

        info!(
            user_id = %auth.user_id,
            enabled = enabled,
            "Setting auto-approval"
        );

        resources
            .database
            .set_auto_approval_enabled(enabled)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to set auto-approval setting");
                AppError::internal(format!("Failed to set auto-approval setting: {e}"))
            })?;

        info!(
            user_id = %auth.user_id,
            enabled = enabled,
            "Auto-approval setting updated"
        );

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("Auto-approval has been {}", if enabled { "enabled" } else { "disabled" }),
                "data": {
                    "enabled": enabled,
                    "description": "When enabled, new user registrations are automatically approved without admin intervention"
                }
            })),
        )
            .into_response())
    }

    // =========================================================================
    // Tool Selection Routes (web admin versions with cookie auth)
    // =========================================================================

    /// GET `/api/admin/tools/catalog` - List all tools in catalog
    async fn handle_get_tool_catalog(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        Self::authenticate_admin(&headers, &resources).await?;

        let catalog = resources.tool_selection.get_catalog().await?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("Retrieved {} tools from catalog", catalog.len()),
                "data": catalog
            })),
        )
            .into_response())
    }

    /// GET `/api/admin/tools/catalog/:tool_name` - Get single tool details
    async fn handle_get_tool_catalog_entry(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(tool_name): Path<String>,
    ) -> Result<Response, AppError> {
        Self::authenticate_admin(&headers, &resources).await?;

        let catalog = resources.tool_selection.get_catalog().await?;
        let entry = catalog
            .into_iter()
            .find(|e| e.tool_name == tool_name)
            .ok_or_else(|| AppError::not_found(format!("Tool '{tool_name}'")))?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("Retrieved tool '{tool_name}'"),
                "data": entry
            })),
        )
            .into_response())
    }

    /// GET `/api/admin/tools/global-disabled` - List globally disabled tools
    async fn handle_get_global_disabled_tools(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
    ) -> Result<Response, AppError> {
        Self::authenticate_admin(&headers, &resources).await?;

        let disabled_tools = resources.tool_selection.get_globally_disabled_tools();
        let count = disabled_tools.len();

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": if count == 0 {
                    "No tools are globally disabled".to_owned()
                } else {
                    format!("{count} tool(s) globally disabled via PIERRE_DISABLED_TOOLS")
                },
                "data": {
                    "disabled_tools": disabled_tools,
                    "count": count
                }
            })),
        )
            .into_response())
    }

    /// GET `/api/admin/tools/tenant/:tenant_id` - Get effective tools for tenant
    async fn handle_get_tenant_tools(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(tenant_id): Path<uuid::Uuid>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;
        verify_admin_tenant_access(&resources, auth.user_id, tenant_id).await?;

        let tools = resources
            .tool_selection
            .get_effective_tools(tenant_id)
            .await?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!("Retrieved {} effective tools for tenant {tenant_id}", tools.len()),
                "data": tools
            })),
        )
            .into_response())
    }

    /// POST `/api/admin/tools/tenant/:tenant_id/override` - Set tool override
    async fn handle_set_tool_override(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(tenant_id): Path<uuid::Uuid>,
        Json(request): Json<SetToolOverrideRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;
        verify_admin_tenant_access(&resources, auth.user_id, tenant_id).await?;

        info!(
            "Setting tool override: tenant={}, tool={}, enabled={}, by={}",
            tenant_id, request.tool_name, request.is_enabled, auth.user_id
        );

        let override_entry = resources
            .tool_selection
            .set_tool_override(
                tenant_id,
                &request.tool_name,
                request.is_enabled,
                auth.user_id,
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
            Json(serde_json::json!({
                "success": true,
                "message": format!("Tool '{}' {} for tenant {tenant_id}", request.tool_name, action),
                "data": override_entry
            })),
        )
            .into_response())
    }

    /// DELETE `/api/admin/tools/tenant/:tenant_id/override/:tool_name` - Remove override
    async fn handle_remove_tool_override(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path((tenant_id, tool_name)): Path<(uuid::Uuid, String)>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;
        verify_admin_tenant_access(&resources, auth.user_id, tenant_id).await?;

        info!(
            "Removing tool override: tenant={}, tool={}, by={}",
            tenant_id, tool_name, auth.user_id
        );

        let deleted = resources
            .tool_selection
            .remove_tool_override(tenant_id, &tool_name)
            .await?;

        if deleted {
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({
                    "success": true,
                    "message": format!("Override removed for tool '{tool_name}' on tenant {tenant_id}")
                })),
            )
                .into_response())
        } else {
            Err(AppError::not_found(format!(
                "No override found for tool '{tool_name}' on tenant {tenant_id}"
            )))
        }
    }

    /// GET `/api/admin/tools/tenant/:tenant_id/summary` - Get availability summary
    async fn handle_get_tool_summary(
        State(resources): State<Arc<ServerResources>>,
        headers: HeaderMap,
        Path(tenant_id): Path<uuid::Uuid>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;
        verify_admin_tenant_access(&resources, auth.user_id, tenant_id).await?;

        let summary = resources
            .tool_selection
            .get_availability_summary(tenant_id)
            .await?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": format!(
                    "Tenant {tenant_id}: {}/{} tools enabled",
                    summary.enabled_tools, summary.total_tools
                ),
                "data": summary
            })),
        )
            .into_response())
    }
}
