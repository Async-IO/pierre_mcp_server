// ABOUTME: Admin user management route handlers
// ABOUTME: Handles user listing, approval, suspension, deletion, password reset, rate limits, and activity
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::Serialize;
use serde_json::{json, to_value};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    admin::{models::ValidatedAdminToken, AdminPermission as AdminPerm},
    constants::tiers,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, AppResult},
    models::{Tenant, TenantId, UserStatus},
    rate_limiting::UnifiedRateLimitCalculator,
};

use super::api_keys::json_response;
use super::types::{
    AdminResponse, ApproveUserRequest, DeleteUserRequest, ListUsersQuery, SuspendUserRequest,
    TenantCreatedInfo, UserActivityQuery,
};
use super::AdminApiContext;

/// User list response
#[derive(Debug, Clone, Serialize)]
pub(super) struct UserListResponse {
    /// List of users (sanitized - no passwords)
    users: Vec<UserSummary>,
    /// Total number of users
    total: usize,
}

/// Sanitized user summary for listing
#[derive(Debug, Clone, Serialize)]
pub(super) struct UserSummary {
    /// User ID
    id: String,
    /// User email
    email: String,
    /// Display name
    display_name: Option<String>,
    /// User tier
    tier: String,
    /// When user was created
    created_at: String,
    /// Last active time
    last_active: String,
}

/// Get user status string
pub(super) const fn user_status_str(status: UserStatus) -> &'static str {
    match status {
        UserStatus::Pending => "pending",
        UserStatus::Active => "active",
        UserStatus::Suspended => "suspended",
    }
}

/// Handle user listing
pub(super) async fn handle_list_users(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Query(params): Query<ListUsersQuery>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!("Listing users by service: {}", admin_token.service_name);

    let ctx = context.as_ref();

    let status = params.status.as_deref().unwrap_or("active");

    let users = ctx
        .database
        .get_users_by_status(status, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch users from database");
            AppError::internal(format!("Failed to fetch users: {e}"))
        })?;

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

    let total = user_summaries.len();

    info!("Retrieved {} users", total);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: format!("Retrieved {total} users"),
            data: to_value(UserListResponse {
                users: user_summaries,
                total,
            })
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle pending users listing
pub(super) async fn handle_pending_users(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Listing pending users by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    let users = ctx
        .database
        .get_users_by_status("pending", None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to fetch pending users from database");
            AppError::internal(format!("Failed to fetch pending users: {e}"))
        })?;

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

    info!("Retrieved {} pending users", count);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: format!("Retrieved {count} pending users"),
            data: to_value(json!({
                "count": count,
                "users": user_summaries
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle tenant creation and linking for user approval
async fn create_and_link_tenant(
    database: &Database,
    user_uuid: Uuid,
    user_email: &str,
    request: &ApproveUserRequest,
    display_name: Option<&str>,
) -> AppResult<Option<TenantCreatedInfo>> {
    if !request.create_default_tenant.unwrap_or(false) {
        return Ok(None);
    }

    let tenant_name = request
        .tenant_name
        .clone()
        .unwrap_or_else(|| format!("{}'s Organization", display_name.unwrap_or(user_email)));
    let tenant_slug = request
        .tenant_slug
        .clone()
        .unwrap_or_else(|| format!("user-{}", user_uuid.as_simple()));

    let tenant = create_default_tenant_for_user(database, user_uuid, &tenant_name, &tenant_slug)
        .await
        .map_err(|e| {
            error!(
                "Failed to create default tenant for user {}: {}",
                user_email, e
            );
            AppError::internal(format!("Failed to create tenant: {e}"))
        })?;

    info!(
        "Created default tenant '{}' for user {}",
        tenant.name, user_email
    );

    database
        .update_user_tenant_id(user_uuid, tenant.id)
        .await
        .map_err(|e| {
            error!(
                "Failed to link user {} to tenant {}: {}",
                user_email, tenant.id, e
            );
            AppError::internal(format!("Failed to link user to created tenant: {e}"))
        })?;

    Ok(Some(TenantCreatedInfo {
        tenant_id: tenant.id.to_string(),
        name: tenant.name,
        slug: tenant.slug,
        plan: tenant.plan,
    }))
}

/// Create default tenant for a user
///
/// # Errors
/// Returns error if tenant slug is invalid, already exists, or database operation fails
async fn create_default_tenant_for_user(
    database: &Database,
    owner_user_id: Uuid,
    tenant_name: &str,
    tenant_slug: &str,
) -> AppResult<Tenant> {
    const RESERVED_SLUGS: &[&str] = &[
        "admin",
        "api",
        "www",
        "app",
        "dashboard",
        "auth",
        "oauth",
        "login",
        "logout",
        "signup",
        "system",
        "root",
        "public",
        "static",
        "assets",
    ];

    let tenant_id = TenantId::new();
    let slug = tenant_slug.trim().to_lowercase();

    if slug.is_empty() {
        return Err(AppError::invalid_input("Tenant slug cannot be empty"));
    }

    if slug.len() > 63 {
        return Err(AppError::invalid_input(
            "Tenant slug must be 63 characters or less",
        ));
    }

    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(AppError::invalid_input(
            "Tenant slug can only contain letters, numbers, and hyphens",
        ));
    }

    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(AppError::invalid_input(
            "Tenant slug cannot start or end with a hyphen",
        ));
    }

    if RESERVED_SLUGS.contains(&slug.as_str()) {
        return Err(AppError::invalid_input(format!(
            "Tenant slug '{slug}' is reserved and cannot be used",
        )));
    }

    if database.get_tenant_by_slug(&slug).await.is_ok() {
        return Err(AppError::invalid_input(format!(
            "Tenant slug '{slug}' is already in use",
        )));
    }

    let tenant_data = Tenant {
        id: tenant_id,
        name: tenant_name.to_owned(),
        slug,
        domain: None,
        plan: tiers::STARTER.to_owned(),
        owner_user_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    database
        .create_tenant(&tenant_data)
        .await
        .map_err(|e| AppError::database(format!("Failed to create tenant: {e}")))?;

    Ok(tenant_data)
}

/// Handle user approval workflow
#[allow(clippy::too_many_lines)]
pub(super) async fn handle_approve_user(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(user_id): Path<String>,
    Json(request): Json<ApproveUserRequest>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Approving user {} by service: {}",
        user_id, admin_token.service_name
    );

    let ctx = context.as_ref();
    let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
        error!(error = %e, "Invalid user ID format");
        AppError::invalid_input(format!("Invalid user ID format: {e}"))
    })?;

    let user = ctx
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
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "User is already approved".to_owned(),
                data: None,
            },
            StatusCode::BAD_REQUEST,
        ));
    }

    let updated_user = ctx
        .database
        .update_user_status(user_uuid, UserStatus::Active, None)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to update user status in database");
            AppError::internal(format!("Failed to approve user: {e}"))
        })?;

    let tenant_created = create_and_link_tenant(
        &ctx.database,
        user_uuid,
        &updated_user.email,
        &request,
        updated_user.display_name.as_deref(),
    )
    .await?;

    let reason = request.reason.as_deref().unwrap_or("No reason provided");
    info!("User {} approved successfully. Reason: {}", user_id, reason);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "User approved successfully".to_owned(),
            data: to_value(json!({
                "user": {
                    "id": updated_user.id.to_string(),
                    "email": updated_user.email,
                    "user_status": user_status_str(updated_user.user_status),
                    "approved_by": updated_user.approved_by,
                    "approved_at": updated_user.approved_at.map(|t| t.to_rfc3339()),
                },
                "tenant_created": tenant_created,
                "reason": reason
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle user suspension workflow
pub(super) async fn handle_suspend_user(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(user_id): Path<String>,
    Json(request): Json<SuspendUserRequest>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Suspending user {} by service: {}",
        user_id, admin_token.service_name
    );

    let ctx = context.as_ref();
    let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
        error!(error = %e, "Invalid user ID format");
        AppError::invalid_input(format!("Invalid user ID format: {e}"))
    })?;

    let user = ctx
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
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "User is already suspended".to_owned(),
                data: None,
            },
            StatusCode::BAD_REQUEST,
        ));
    }

    let updated_user = ctx
        .database
        .update_user_status(user_uuid, UserStatus::Suspended, None)
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

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "User suspended successfully".to_owned(),
            data: to_value(json!({
                "user": {
                    "id": updated_user.id.to_string(),
                    "email": updated_user.email,
                    "user_status": user_status_str(updated_user.user_status),
                },
                "reason": reason
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle user deletion workflow
///
/// Permanently deletes a user and all associated data (cascades via foreign keys).
/// This action cannot be undone.
pub(super) async fn handle_delete_user(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(user_id): Path<String>,
    Json(request): Json<DeleteUserRequest>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Deleting user {} by service: {}",
        user_id, admin_token.service_name
    );

    let ctx = context.as_ref();
    let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
        error!(error = %e, "Invalid user ID format");
        AppError::invalid_input(format!("Invalid user ID format: {e}"))
    })?;

    let user = ctx
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

    let user_email = user.email.clone();

    ctx.database.delete_user(user_uuid).await.map_err(|e| {
        error!(error = %e, "Failed to delete user from database");
        AppError::internal(format!("Failed to delete user: {e}"))
    })?;

    let reason = request.reason.as_deref().unwrap_or("No reason provided");
    info!(
        "User {} ({}) deleted successfully. Reason: {}",
        user_id, user_email, reason
    );

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "User deleted successfully".to_owned(),
            data: to_value(json!({
                "deleted_user": {
                    "id": user_id,
                    "email": user_email,
                },
                "reason": reason
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle password reset for a user (admin only)
///
/// Issues a one-time reset token instead of a temporary password. The admin
/// delivers the token to the user, who then calls `POST /api/auth/complete-reset`
/// with the token and their chosen new password. The token expires after 1 hour
/// and can only be used once.
pub(super) async fn handle_reset_user_password(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(user_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    use sha2::{Digest, Sha256};

    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Issuing password reset token for user {} by service: {}",
        user_id, admin_token.service_name
    );

    let ctx = context.as_ref();
    let user_uuid = Uuid::parse_str(&user_id).map_err(|e| {
        error!(error = %e, "Invalid user ID format");
        AppError::invalid_input(format!("Invalid user ID format: {e}"))
    })?;

    let user = ctx
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

    let raw_token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect();

    let token_hash = format!("{:x}", Sha256::digest(raw_token.as_bytes()));

    ctx.database
        .store_password_reset_token(user_uuid, &token_hash, &admin_token.service_name)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to store password reset token");
            AppError::internal(format!("Failed to create reset token: {e}"))
        })?;

    info!(
        "Password reset token issued for user {} by service {}",
        user.email, admin_token.service_name
    );

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Password reset token issued".to_owned(),
            data: to_value(json!({
                "user_id": user_uuid.to_string(),
                "email": user.email,
                "reset_token": raw_token,
                "expires_in_seconds": 3600,
                "reset_by": admin_token.service_name,
                "note": "Deliver this token to the user. They must call POST /api/auth/complete-reset with the token and their new password within 1 hour."
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle getting rate limit info for a user
pub(super) async fn handle_get_user_rate_limit(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(user_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    let ctx = context.as_ref();
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|e| AppError::invalid_input(format!("Invalid user ID format: {e}")))?;

    let user = ctx
        .database
        .get_user(user_uuid)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| AppError::not_found("User not found"))?;

    let monthly_used = ctx
        .database
        .get_jwt_current_usage(user_uuid)
        .await
        .unwrap_or(0);

    let now = Utc::now();
    let today_start = now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map_or(now, |t| DateTime::<Utc>::from_naive_utc_and_offset(t, Utc));
    let daily_used = ctx
        .database
        .get_top_tools_analysis(user_uuid, today_start, now)
        .await
        .map(|tools| {
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            tools.iter().map(|t| t.request_count as u32).sum::<u32>()
        })
        .unwrap_or(0);

    let monthly_limit = user.tier.monthly_limit();
    let daily_limit = monthly_limit.map(|m| m / 30);

    let monthly_remaining = monthly_limit.map(|l| l.saturating_sub(monthly_used));
    let daily_remaining = daily_limit.map(|l| l.saturating_sub(daily_used));

    let daily_reset = (now + Duration::days(1))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .map_or(now, |t| DateTime::<Utc>::from_naive_utc_and_offset(t, Utc));
    let monthly_reset = UnifiedRateLimitCalculator::calculate_monthly_reset();

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Rate limit information retrieved".to_owned(),
            data: to_value(json!({
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
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle getting user activity logs
pub(super) async fn handle_get_user_activity(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(user_id): Path<String>,
    Query(params): Query<UserActivityQuery>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageUsers)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageUsers required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    let ctx = context.as_ref();
    let user_uuid = Uuid::parse_str(&user_id)
        .map_err(|e| AppError::invalid_input(format!("Invalid user ID format: {e}")))?;

    ctx.database
        .get_user(user_uuid)
        .await
        .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
        .ok_or_else(|| AppError::not_found("User not found"))?;

    let days = i64::from(params.days.unwrap_or(30).clamp(1, 365));
    let now = Utc::now();
    let start_time = now - Duration::days(days);

    let top_tools_raw = ctx
        .database
        .get_top_tools_analysis(user_uuid, start_time, now)
        .await
        .unwrap_or_default();

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
            json!({
                "tool_name": t.tool_name,
                "call_count": t.request_count,
                "percentage": percentage,
            })
        })
        .collect();

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "User activity retrieved".to_owned(),
            data: to_value(json!({
                "user_id": user_uuid.to_string(),
                "period_days": days,
                "total_requests": total_requests,
                "top_tools": top_tools,
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}
