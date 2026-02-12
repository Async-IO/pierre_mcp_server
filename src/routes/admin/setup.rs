// ABOUTME: Admin setup and health check route handlers
// ABOUTME: Handles initial admin setup, status check, and health endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde_json::json;
use tokio::task;
use tracing::{error, info};
use uuid::Uuid;

use crate::{
    admin::{
        jwks::JwksManager,
        models::{AdminPermission, CreateAdminTokenRequest},
    },
    auth::SetupStatusResponse,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppResult,
    models::{User, UserStatus},
};

use super::api_keys::json_response;
use super::types::{AdminResponse, AdminSetupRequest};
use super::AdminApiContext;

/// Check if any admin users already exist
///
/// Returns an error response if an admin already exists, or Ok(None) if setup can proceed
async fn check_no_admin_exists(
    database: &Database,
) -> AppResult<Option<(StatusCode, Json<AdminResponse>)>> {
    match database.get_users_by_status("active", None).await {
        Ok(users) => {
            let admin_exists = users.iter().any(|u| u.is_admin);
            if admin_exists {
                return Ok(Some((
                    StatusCode::CONFLICT,
                    Json(AdminResponse {
                        success: false,
                        message: "Admin user already exists. Use admin token management instead."
                            .into(),
                        data: None,
                    }),
                )));
            }
            Ok(None)
        }
        Err(e) => {
            error!("Failed to check existing admin users: {}", e);
            Ok(Some((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminResponse {
                    success: false,
                    message: format!("Database error: {e}"),
                    data: None,
                }),
            )))
        }
    }
}

/// Create admin user record with hashed password
async fn create_admin_user_record(
    database: &Database,
    request: &AdminSetupRequest,
) -> Result<Uuid, (StatusCode, Json<AdminResponse>)> {
    let user_id = Uuid::new_v4();

    let password_hash = match bcrypt::hash(&request.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminResponse {
                    success: false,
                    message: "Failed to process password".into(),
                    data: None,
                }),
            ));
        }
    };

    let mut admin_user = User::new(
        request.email.clone(),
        password_hash,
        request.display_name.clone(),
    );
    admin_user.id = user_id;
    admin_user.is_admin = true;
    admin_user.user_status = UserStatus::Active;

    match database.create_user(&admin_user).await {
        Ok(_) => {
            info!("Admin user created successfully: {}", request.email);
            Ok(user_id)
        }
        Err(e) => {
            error!("Failed to create admin user: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminResponse {
                    success: false,
                    message: format!("Failed to create admin user: {e}"),
                    data: None,
                }),
            ))
        }
    }
}

/// Generate initial admin token with full permissions
async fn generate_initial_admin_token(
    database: &Database,
    admin_jwt_secret: &str,
    jwks_manager: &Arc<JwksManager>,
) -> Result<String, (StatusCode, Json<AdminResponse>)> {
    let token_request = CreateAdminTokenRequest {
        service_name: "initial_admin_setup".to_owned(),
        service_description: Some("Initial admin setup token".to_owned()),
        permissions: Some(vec![
            AdminPermission::ManageUsers,
            AdminPermission::ManageAdminTokens,
            AdminPermission::ProvisionKeys,
            AdminPermission::ListKeys,
            AdminPermission::UpdateKeyLimits,
            AdminPermission::RevokeKeys,
            AdminPermission::ViewAuditLogs,
        ]),
        is_super_admin: true,
        expires_in_days: Some(365),
    };

    match database
        .create_admin_token(&token_request, admin_jwt_secret, jwks_manager)
        .await
    {
        Ok(generated_token) => Ok(generated_token.jwt_token),
        Err(e) => {
            error!("Failed to generate admin token after creating user: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(AdminResponse {
                    success: false,
                    message: format!("User created but token generation failed: {e}"),
                    data: None,
                }),
            ))
        }
    }
}

/// Handle admin setup
pub(super) async fn handle_admin_setup(
    State(context): State<Arc<AdminApiContext>>,
    Json(request): Json<AdminSetupRequest>,
) -> AppResult<impl IntoResponse> {
    info!("Admin setup request for email: {}", request.email);

    let ctx = context.as_ref();

    if let Some(error_response) = check_no_admin_exists(&ctx.database).await? {
        return Ok(error_response);
    }

    let user_id = match create_admin_user_record(&ctx.database, &request).await {
        Ok(id) => id,
        Err(error_response) => return Ok(error_response),
    };

    let admin_token =
        match generate_initial_admin_token(&ctx.database, &ctx.admin_jwt_secret, &ctx.jwks_manager)
            .await
        {
            Ok(token) => token,
            Err(error_response) => return Ok(error_response),
        };

    info!("Admin setup completed successfully for: {}", request.email);
    Ok((
        StatusCode::CREATED,
        Json(AdminResponse {
            success: true,
            message: format!(
                "Admin user {} created successfully with token",
                request.email
            ),
            data: Some(json!({
                "user_id": user_id.to_string(),
                "admin_token": admin_token,
            })),
        }),
    ))
}

/// Handle setup status check
pub(super) async fn handle_setup_status(
    State(context): State<Arc<AdminApiContext>>,
) -> AppResult<impl IntoResponse> {
    info!("Setup status check requested");

    let ctx = context.as_ref();

    match ctx.auth_manager.check_setup_status(&ctx.database).await {
        Ok(setup_status) => {
            info!(
                "Setup status check successful: needs_setup={}, admin_user_exists={}",
                setup_status.needs_setup, setup_status.admin_user_exists
            );
            Ok(json_response(setup_status, StatusCode::OK))
        }
        Err(e) => {
            error!("Failed to check setup status: {}", e);
            Ok(json_response(
                SetupStatusResponse {
                    needs_setup: true,
                    admin_user_exists: false,
                    message: Some(
                        "Unable to determine setup status. Please ensure admin user is created."
                            .to_owned(),
                    ),
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle health check (GET /admin/health)
pub(super) async fn handle_health() -> Json<serde_json::Value> {
    let health_json = task::spawn_blocking(|| {
        json!({
            "status": "healthy",
            "service": "pierre-mcp-admin-api",
            "timestamp": Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION")
        })
    })
    .await
    .unwrap_or_else(|_| {
        json!({
            "status": "error",
            "service": "pierre-mcp-admin-api"
        })
    });

    Json(health_json)
}
