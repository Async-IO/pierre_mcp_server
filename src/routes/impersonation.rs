// ABOUTME: Impersonation routes for super admin users to view system as another user
// ABOUTME: Provides secure impersonation with audit logging and session management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Impersonation Routes
//!
//! This module provides endpoints for super admin users to impersonate other users.
//! All impersonation actions are logged for audit purposes.

use crate::{
    database_plugins::DatabaseProvider,
    errors::{AppError, ErrorCode},
    mcp::resources::ServerResources,
    permissions::impersonation::ImpersonationSession,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

/// Response for listing impersonation sessions
#[derive(Serialize)]
struct ImpersonationSessionsResponse {
    sessions: Vec<ImpersonationSessionSummary>,
    total_count: usize,
}

/// Summary of an impersonation session
#[derive(Serialize)]
struct ImpersonationSessionSummary {
    id: String,
    impersonator_id: String,
    impersonator_email: Option<String>,
    target_user_id: String,
    target_user_email: Option<String>,
    reason: Option<String>,
    started_at: String,
    ended_at: Option<String>,
    is_active: bool,
    duration_seconds: i64,
}

/// Response for starting impersonation
#[derive(Serialize)]
struct StartImpersonationResponse {
    success: bool,
    session_id: String,
    token: String,
    target_user: TargetUserInfo,
    message: String,
}

/// Target user information
#[derive(Serialize)]
struct TargetUserInfo {
    id: String,
    email: String,
    display_name: Option<String>,
    role: String,
}

/// Response for ending impersonation
#[derive(Serialize)]
struct EndImpersonationResponse {
    success: bool,
    message: String,
    session_id: String,
    duration_seconds: i64,
}

/// Request body for starting impersonation
#[derive(Deserialize)]
struct StartImpersonationRequestBody {
    target_user_id: String,
    reason: Option<String>,
}

/// Impersonation routes - super admin only
pub struct ImpersonationRoutes;

impl ImpersonationRoutes {
    /// Create all impersonation routes
    pub fn routes(resources: Arc<ServerResources>) -> Router {
        Router::new()
            .route(
                "/api/admin/impersonate",
                post(Self::handle_start_impersonation),
            )
            .route(
                "/api/admin/impersonate/end",
                post(Self::handle_end_impersonation),
            )
            .route(
                "/api/admin/impersonate/sessions",
                get(Self::handle_list_sessions),
            )
            .route(
                "/api/admin/impersonate/sessions/:session_id",
                get(Self::handle_get_session),
            )
            .with_state(resources)
    }

    /// Authenticate user and require super admin role
    async fn authenticate_super_admin(
        headers: &axum::http::HeaderMap,
        resources: &Arc<ServerResources>,
    ) -> Result<(crate::auth::AuthResult, crate::models::User), AppError> {
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

        // Get user and check for super_admin role
        let user = resources
            .database
            .get_user(auth.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        if !user.role.is_super_admin() {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Super admin privileges required for impersonation",
            ));
        }

        Ok((auth, user))
    }

    /// Handle starting an impersonation session
    async fn handle_start_impersonation(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Json(request): Json<StartImpersonationRequestBody>,
    ) -> Result<Response, AppError> {
        // Authenticate and verify super admin status
        let (auth, impersonator) = Self::authenticate_super_admin(&headers, &resources).await?;

        // Parse target user ID
        let target_user_id = Uuid::parse_str(&request.target_user_id)
            .map_err(|e| AppError::invalid_input(format!("Invalid target user ID: {e}")))?;

        // Cannot impersonate yourself
        if target_user_id == auth.user_id {
            return Err(AppError::invalid_input("Cannot impersonate yourself"));
        }

        // Get target user
        let target_user = resources
            .database
            .get_user(target_user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get target user: {e}")))?
            .ok_or_else(|| AppError::not_found("Target user not found"))?;

        // Cannot impersonate another super admin
        if target_user.role.is_super_admin() {
            return Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Cannot impersonate another super admin",
            ));
        }

        // End any existing active impersonation sessions for this impersonator
        resources
            .database
            .end_all_impersonation_sessions(auth.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to end existing sessions: {e}")))?;

        // Create new impersonation session
        let session =
            ImpersonationSession::new(auth.user_id, target_user_id, request.reason.clone());

        // Store session in database
        resources
            .database
            .create_impersonation_session(&session)
            .await
            .map_err(|e| {
                AppError::internal(format!("Failed to create impersonation session: {e}"))
            })?;

        // Generate impersonation token (JWT with impersonation claims)
        let impersonation_token = resources
            .auth_manager
            .generate_impersonation_token(
                &target_user,
                auth.user_id,
                &session.id,
                &resources.jwks_manager,
            )
            .map_err(|e| {
                AppError::internal(format!("Failed to generate impersonation token: {e}"))
            })?;

        info!(
            impersonator_id = %auth.user_id,
            impersonator_email = %impersonator.email,
            target_user_id = %target_user_id,
            target_user_email = %target_user.email,
            session_id = %session.id,
            reason = ?request.reason,
            "Super admin started impersonation session"
        );

        Ok((
            StatusCode::OK,
            Json(StartImpersonationResponse {
                success: true,
                session_id: session.id,
                token: impersonation_token,
                target_user: TargetUserInfo {
                    id: target_user.id.to_string(),
                    email: target_user.email,
                    display_name: target_user.display_name,
                    role: target_user.role.as_str().to_owned(),
                },
                message: "Impersonation session started successfully".to_owned(),
            }),
        )
            .into_response())
    }

    /// Handle ending an impersonation session
    async fn handle_end_impersonation(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate - can be either super admin or impersonated session
        let auth_value =
            if let Some(auth_header) = headers.get("authorization").and_then(|h| h.to_str().ok()) {
                auth_header.to_owned()
            } else if let Some(token) =
                crate::security::cookies::get_cookie_value(&headers, "auth_token")
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

        // Check if this is an impersonation session by looking for active session
        // where the authenticated user_id matches either impersonator or target
        let session = resources
            .database
            .get_active_impersonation_session(auth.user_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get session: {e}")))?;

        let Some(session) = session else {
            return Err(AppError::not_found("No active impersonation session found"));
        };

        // End the session
        resources
            .database
            .end_impersonation_session(&session.id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to end session: {e}")))?;

        let duration = session.duration_seconds();

        info!(
            impersonator_id = %session.impersonator_id,
            target_user_id = %session.target_user_id,
            session_id = %session.id,
            duration_seconds = duration,
            "Impersonation session ended"
        );

        Ok((
            StatusCode::OK,
            Json(EndImpersonationResponse {
                success: true,
                message: "Impersonation session ended successfully".to_owned(),
                session_id: session.id,
                duration_seconds: duration,
            }),
        )
            .into_response())
    }

    /// Handle listing impersonation sessions
    async fn handle_list_sessions(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
    ) -> Result<Response, AppError> {
        // Authenticate and verify super admin status
        let (auth, _user) = Self::authenticate_super_admin(&headers, &resources).await?;

        info!(
            user_id = %auth.user_id,
            "Super admin listing impersonation sessions"
        );

        // Fetch all sessions (not just active)
        let sessions = resources
            .database
            .list_impersonation_sessions(None, None, false, 100)
            .await
            .map_err(|e| AppError::internal(format!("Failed to list sessions: {e}")))?;

        // Build summaries with user emails
        let mut summaries = Vec::with_capacity(sessions.len());
        for session in &sessions {
            let impersonator_email = resources
                .database
                .get_user(session.impersonator_id)
                .await
                .ok()
                .flatten()
                .map(|u| u.email);
            let target_email = resources
                .database
                .get_user(session.target_user_id)
                .await
                .ok()
                .flatten()
                .map(|u| u.email);

            summaries.push(ImpersonationSessionSummary {
                id: session.id.clone(),
                impersonator_id: session.impersonator_id.to_string(),
                impersonator_email,
                target_user_id: session.target_user_id.to_string(),
                target_user_email: target_email,
                reason: session.reason.clone(),
                started_at: session.started_at.to_rfc3339(),
                ended_at: session.ended_at.map(|dt| dt.to_rfc3339()),
                is_active: session.is_active,
                duration_seconds: session.duration_seconds(),
            });
        }

        let total_count = summaries.len();

        Ok((
            StatusCode::OK,
            Json(ImpersonationSessionsResponse {
                sessions: summaries,
                total_count,
            }),
        )
            .into_response())
    }

    /// Handle getting a specific impersonation session
    async fn handle_get_session(
        State(resources): State<Arc<ServerResources>>,
        headers: axum::http::HeaderMap,
        Path(session_id): Path<String>,
    ) -> Result<Response, AppError> {
        // Authenticate and verify super admin status
        let (_auth, _user) = Self::authenticate_super_admin(&headers, &resources).await?;

        // Get the session
        let session = resources
            .database
            .get_impersonation_session(&session_id)
            .await
            .map_err(|e| AppError::internal(format!("Failed to get session: {e}")))?
            .ok_or_else(|| AppError::not_found("Session not found"))?;

        // Get user details
        let impersonator_email = resources
            .database
            .get_user(session.impersonator_id)
            .await
            .ok()
            .flatten()
            .map(|u| u.email);
        let target_email = resources
            .database
            .get_user(session.target_user_id)
            .await
            .ok()
            .flatten()
            .map(|u| u.email);

        let summary = ImpersonationSessionSummary {
            id: session.id.clone(),
            impersonator_id: session.impersonator_id.to_string(),
            impersonator_email,
            target_user_id: session.target_user_id.to_string(),
            target_user_email: target_email,
            reason: session.reason.clone(),
            started_at: session.started_at.to_rfc3339(),
            ended_at: session.ended_at.map(|dt| dt.to_rfc3339()),
            is_active: session.is_active,
            duration_seconds: session.duration_seconds(),
        };

        Ok((StatusCode::OK, Json(summary)).into_response())
    }
}
