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
    database_plugins::DatabaseProvider, errors::AppError, errors::ErrorCode,
    mcp::resources::ServerResources, models::UserStatus,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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
            .with_state(resources)
    }

    /// Authenticate user from authorization header or cookie, requiring admin privileges
    async fn authenticate_admin(
        headers: &axum::http::HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<crate::auth::AuthResult, AppError> {
        // Try Authorization header first, then fall back to auth_token cookie
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

        let auth = resources
            .auth_middleware
            .authenticate_request(Some(&auth_value))
            .await
            .map_err(|e| AppError::auth_invalid(format!("Authentication failed: {e}")))?;

        // Check if user is admin
        let user = resources
            .database
            .get_user(auth.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !user.is_admin {
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
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            "Web admin listing pending users"
        );

        // Fetch users with Pending status
        let users = resources
            .database
            .get_users_by_status("pending")
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch pending users from database");
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

        tracing::info!("Retrieved {count} pending users for web admin");

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
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            "Web admin listing all users"
        );

        // Fetch users by status and combine (no get_all_users method exists)
        let mut all_users = Vec::new();

        for status in ["active", "pending", "suspended"] {
            let users = resources
                .database
                .get_users_by_status(status)
                .await
                .map_err(|e| {
                    tracing::error!(error = %e, status = status, "Failed to fetch users from database");
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

        tracing::info!("Retrieved {total_count} users for web admin");

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
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            "Web admin listing admin tokens"
        );

        // Fetch admin tokens (include_inactive = false for active tokens only)
        let tokens = resources
            .database
            .list_admin_tokens(false)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch admin tokens from database");
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

        tracing::info!("Retrieved {total_count} admin tokens for web admin");

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
        headers: axum::http::HeaderMap,
        Path(user_id): Path<String>,
        Json(request): Json<ApproveUserRequest>,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            admin_user_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin approving user"
        );

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&user_id).map_err(|e| {
            tracing::error!(error = %e, "Invalid user ID format");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

        // Get the user to approve
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch user from database");
                AppError::internal(format!("Failed to fetch user: {e}"))
            })?
            .ok_or_else(|| {
                tracing::warn!("User not found: {}", user_id);
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

        // Use the admin user's ID as the approver (stored as token_id format for consistency)
        let approver_id = auth.user_id.to_string();

        let updated_user = resources
            .database
            .update_user_status(user_uuid, UserStatus::Active, &approver_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to update user status in database");
                AppError::internal(format!("Failed to approve user: {e}"))
            })?;

        let reason = request.reason.as_deref().unwrap_or("No reason provided");
        tracing::info!("User {} approved successfully. Reason: {}", user_id, reason);

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
        headers: axum::http::HeaderMap,
        Path(user_id): Path<String>,
        Json(request): Json<SuspendUserRequest>,
    ) -> Result<Response, AppError> {
        // Authenticate and verify admin status
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            admin_user_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin suspending user"
        );

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&user_id).map_err(|e| {
            tracing::error!(error = %e, "Invalid user ID format");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

        // Get the user to suspend
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch user from database");
                AppError::internal(format!("Failed to fetch user: {e}"))
            })?
            .ok_or_else(|| {
                tracing::warn!("User not found: {}", user_id);
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

        // Use the admin user's ID as the suspender
        let suspender_id = auth.user_id.to_string();

        let updated_user = resources
            .database
            .update_user_status(user_uuid, UserStatus::Suspended, &suspender_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to update user status in database");
                AppError::internal(format!("Failed to suspend user: {e}"))
            })?;

        let reason = request.reason.as_deref().unwrap_or("No reason provided");
        tracing::info!(
            "User {} suspended successfully. Reason: {}",
            user_id,
            reason
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

    /// Handle creating an admin token via web admin (cookie auth)
    async fn handle_create_admin_token(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Json(request): Json<CreateAdminTokenWebRequest>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            service_name = %request.service_name,
            "Web admin creating admin token"
        );

        // Parse permissions if provided
        let permissions = request.permissions.map(|perms| {
            perms
                .iter()
                .filter_map(|p| p.parse::<crate::admin::AdminPermission>().ok())
                .collect::<Vec<_>>()
        });

        // Create the token request
        let token_request = crate::admin::models::CreateAdminTokenRequest {
            service_name: request.service_name,
            service_description: request.service_description,
            permissions,
            expires_in_days: request.expires_in_days,
            is_super_admin: request.is_super_admin.unwrap_or(false),
        };

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
                tracing::error!(error = %e, "Failed to create admin token");
                AppError::internal(format!("Failed to create admin token: {e}"))
            })?;

        tracing::info!(
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
        headers: axum::http::HeaderMap,
        Path(token_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            token_id = %token_id,
            "Web admin getting admin token details"
        );

        let token = resources
            .database
            .get_admin_token_by_id(&token_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch admin token from database");
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
        headers: axum::http::HeaderMap,
        Path(token_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            user_id = %auth.user_id,
            token_id = %token_id,
            "Web admin revoking admin token"
        );

        resources
            .database
            .deactivate_admin_token(&token_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to revoke admin token");
                AppError::internal(format!("Failed to revoke admin token: {e}"))
            })?;

        tracing::info!(
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
    async fn handle_reset_user_password(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(user_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::info!(
            admin_id = %auth.user_id,
            target_user_id = %user_id,
            "Web admin resetting user password"
        );

        let user_uuid = uuid::Uuid::parse_str(&user_id)
            .map_err(|e| AppError::invalid_input(format!("Invalid user ID format: {e}")))?;

        // Generate temporary password
        let temp_password: String = (0..16)
            .map(|_| {
                let chars = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz23456789!@#$%";
                chars[rand::random::<usize>() % chars.len()] as char
            })
            .collect();

        // Hash the password
        let password_hash = bcrypt::hash(&temp_password, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("Failed to hash password: {e}")))?;

        // Update user's password
        resources
            .database
            .update_user_password(user_uuid, &password_hash)
            .await
            .map_err(|e| AppError::internal(format!("Failed to update password: {e}")))?;

        // Get user email for response
        let user = resources
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        Ok((
            StatusCode::OK,
            Json(serde_json::json!({
                "success": true,
                "message": "Password reset successfully",
                "data": {
                    "temporary_password": temp_password,
                    "expires_at": (chrono::Utc::now() + chrono::Duration::hours(24)).to_rfc3339(),
                    "user_email": user.email
                }
            })),
        )
            .into_response())
    }

    /// Handle getting rate limit info for a user via web admin
    async fn handle_get_user_rate_limit(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(user_id): Path<String>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::debug!(
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
        let monthly_reset =
            crate::rate_limiting::UnifiedRateLimitCalculator::calculate_monthly_reset();

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
        headers: axum::http::HeaderMap,
        Path(user_id): Path<String>,
        Query(params): Query<UserActivityQuery>,
    ) -> Result<Response, AppError> {
        let auth = Self::authenticate_admin(&headers, &resources).await?;

        tracing::debug!(
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
}
