// ABOUTME: Web-facing admin routes for authenticated admin users via browser
// ABOUTME: Uses cookie-based auth (same as /api/keys) for users with is_admin=true
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Web Admin Routes
//!
//! This module provides admin endpoints accessible via browser cookie authentication.
//! Unlike `/admin/*` routes which require admin service tokens, these routes
//! accept standard user authentication for users with `is_admin: true`.

use crate::{
    database_plugins::DatabaseProvider, errors::AppError, errors::ErrorCode,
    mcp::resources::ServerResources,
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::Serialize;
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

/// Web admin routes - accessible via browser for admin users
pub struct WebAdminRoutes;

impl WebAdminRoutes {
    /// Create all web admin routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route("/api/admin/pending-users", get(Self::handle_pending_users))
            .route("/api/admin/users", get(Self::handle_all_users))
            .route("/api/admin/tokens", get(Self::handle_admin_tokens))
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
}
