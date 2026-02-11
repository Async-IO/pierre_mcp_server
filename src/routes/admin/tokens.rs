// ABOUTME: Admin token management route handlers
// ABOUTME: Handles CRUD operations for admin service tokens (create, list, get, revoke, rotate)
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::{json, to_value, Value};
use tracing::{error, info};

use crate::{
    admin::{
        models::{
            AdminPermission, AdminTokenSummary, CreateAdminTokenRequest, ValidatedAdminToken,
        },
        AdminPermission as AdminPerm,
    },
    database_plugins::DatabaseProvider,
    errors::{AppError, AppResult},
};

use super::api_keys::json_response;
use super::types::AdminResponse;
use super::AdminApiContext;

/// Handle admin token creation
pub(super) async fn handle_create_admin_token(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Json(request): Json<serde_json::Value>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageAdminTokens)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageAdminTokens required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Creating admin token by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    let service_name = request
        .get("service_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::invalid_input("service_name is required"))?
        .to_owned();

    let service_description = request
        .get("service_description")
        .and_then(|v| v.as_str())
        .map(String::from);

    let is_super_admin = request
        .get("is_super_admin")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if is_super_admin && !admin_token.is_super_admin {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Only super-admin tokens can create super-admin tokens".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    let expires_in_days = request.get("expires_in_days").and_then(Value::as_u64);

    let permissions =
        if let Some(perms_array) = request.get("permissions").and_then(|v| v.as_array()) {
            let mut parsed_permissions = Vec::new();
            for p in perms_array {
                if let Some(perm_str) = p.as_str() {
                    match perm_str.parse::<AdminPermission>() {
                        Ok(perm) => parsed_permissions.push(perm),
                        Err(_) => {
                            return Ok(json_response(
                                AdminResponse {
                                    success: false,
                                    message: format!("Invalid permission: {perm_str}"),
                                    data: None,
                                },
                                StatusCode::BAD_REQUEST,
                            ));
                        }
                    }
                }
            }
            Some(parsed_permissions)
        } else {
            None
        };

    let token_request = CreateAdminTokenRequest {
        service_name,
        service_description,
        permissions,
        expires_in_days,
        is_super_admin,
    };

    let generated_token = ctx
        .database
        .create_admin_token(&token_request, &ctx.admin_jwt_secret, &ctx.jwks_manager)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to generate admin token");
            AppError::internal(format!("Failed to generate admin token: {e}"))
        })?;

    info!("Admin token created: {}", generated_token.token_id);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Admin token created successfully".to_owned(),
            data: to_value(json!({
                "token_id": generated_token.token_id,
                "service_name": generated_token.service_name,
                "jwt_token": generated_token.jwt_token,
                "token_prefix": generated_token.token_prefix,
                "is_super_admin": generated_token.is_super_admin,
                "expires_at": generated_token.expires_at.map(|t| t.to_rfc3339()),
            }))
            .ok(),
        },
        StatusCode::CREATED,
    ))
}

/// Handle listing admin tokens
pub(super) async fn handle_list_admin_tokens(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageAdminTokens)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageAdminTokens required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Listing admin tokens by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    let tokens = ctx.database.list_admin_tokens(false).await.map_err(|e| {
        error!(error = %e, "Failed to list admin tokens");
        AppError::internal(format!("Failed to list admin tokens: {e}"))
    })?;

    info!("Retrieved {} admin tokens", tokens.len());

    let redacted_tokens: Vec<AdminTokenSummary> =
        tokens.into_iter().map(AdminTokenSummary::from).collect();

    Ok(json_response(
        AdminResponse {
            success: true,
            message: format!("Retrieved {} admin tokens", redacted_tokens.len()),
            data: to_value(json!({
                "count": redacted_tokens.len(),
                "tokens": redacted_tokens
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle getting admin token details
pub(super) async fn handle_get_admin_token(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(token_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageAdminTokens)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageAdminTokens required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Getting admin token {} by service: {}",
        token_id, admin_token.service_name
    );

    let ctx = context.as_ref();

    let token = match ctx.database.get_admin_token_by_id(&token_id).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Admin token not found".to_owned(),
                    data: None,
                },
                StatusCode::NOT_FOUND,
            ));
        }
        Err(e) => {
            error!(error = %e, "Failed to get admin token");
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: format!("Failed to get admin token: {e}"),
                    data: None,
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let redacted_token = AdminTokenSummary::from(token);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Admin token retrieved successfully".to_owned(),
            data: to_value(redacted_token).ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle revoking admin token
pub(super) async fn handle_revoke_admin_token(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(token_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageAdminTokens)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageAdminTokens required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Revoking admin token {} by service: {}",
        token_id, admin_token.service_name
    );

    let ctx = context.as_ref();

    ctx.database
        .deactivate_admin_token(&token_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to revoke admin token");
            AppError::internal(format!("Failed to revoke admin token: {e}"))
        })?;

    info!("Admin token {} revoked successfully", token_id);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Admin token revoked successfully".to_owned(),
            data: to_value(json!({
                "token_id": token_id
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle rotating admin token
pub(super) async fn handle_rotate_admin_token(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Path(token_id): Path<String>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ManageAdminTokens)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ManageAdminTokens required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Rotating admin token {} by service: {}",
        token_id, admin_token.service_name
    );

    let ctx = context.as_ref();

    let existing_token = ctx
        .database
        .get_admin_token_by_id(&token_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get admin token");
            AppError::internal(format!("Failed to get admin token: {e}"))
        })?
        .ok_or_else(|| AppError::not_found("Admin token not found"))?;

    ctx.database
        .deactivate_admin_token(&token_id)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to deactivate old token");
            AppError::internal(format!("Failed to deactivate old token: {e}"))
        })?;

    let token_request = CreateAdminTokenRequest {
        service_name: existing_token.service_name.clone(),
        service_description: existing_token.service_description.clone(),
        permissions: None,
        is_super_admin: existing_token.is_super_admin,
        expires_in_days: Some(365_u64),
    };

    let new_token = ctx
        .database
        .create_admin_token(&token_request, &ctx.admin_jwt_secret, &ctx.jwks_manager)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to generate new admin token");
            AppError::internal(format!("Failed to generate new admin token: {e}"))
        })?;

    info!(
        "Admin token {} rotated successfully, new token: {}",
        token_id, new_token.token_id
    );

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Admin token rotated successfully".to_owned(),
            data: to_value(json!({
                "old_token_id": token_id,
                "new_token": {
                    "token_id": new_token.token_id,
                    "service_name": new_token.service_name,
                    "jwt_token": new_token.jwt_token,
                    "token_prefix": new_token.token_prefix,
                    "expires_at": new_token.expires_at.map(|t| t.to_rfc3339()),
                }
            }))
            .ok(),
        },
        StatusCode::OK,
    ))
}
