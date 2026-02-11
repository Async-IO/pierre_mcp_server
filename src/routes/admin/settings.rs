// ABOUTME: Admin system settings route handlers
// ABOUTME: Handles auto-approval and social insights configuration endpoints
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde_json::to_value;
use tracing::{error, info};

use crate::{
    admin::{models::ValidatedAdminToken, AdminPermission as AdminPerm},
    config::social::SocialInsightsConfig,
    errors::{AppError, AppResult},
};

use super::api_keys::json_response;
use super::types::{AdminResponse, AutoApprovalResponse, UpdateAutoApprovalRequest};
use super::AdminApiContext;

/// Handle getting auto-approval setting
pub(super) async fn handle_get_auto_approval(
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
        "Getting auto-approval setting by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    let enabled = ctx
        .database
        .is_auto_approval_enabled()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get auto-approval setting");
            AppError::internal(format!("Failed to get auto-approval setting: {e}"))
        })?
        .unwrap_or(false);

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Auto-approval setting retrieved".to_owned(),
            data: to_value(AutoApprovalResponse {
                enabled,
                description: "When enabled, new user registrations are automatically approved without admin intervention".to_owned(),
            })
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle setting auto-approval
pub(super) async fn handle_set_auto_approval(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Json(request): Json<UpdateAutoApprovalRequest>,
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
        "Setting auto-approval to {} by service: {}",
        request.enabled, admin_token.service_name
    );

    let ctx = context.as_ref();

    ctx.database
        .set_auto_approval_enabled(request.enabled)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to set auto-approval setting");
            AppError::internal(format!("Failed to set auto-approval setting: {e}"))
        })?;

    info!(
        "Auto-approval setting updated to {} by {}",
        request.enabled, admin_token.service_name
    );

    Ok(json_response(
        AdminResponse {
            success: true,
            message: format!(
                "Auto-approval has been {}",
                if request.enabled { "enabled" } else { "disabled" }
            ),
            data: to_value(AutoApprovalResponse {
                enabled: request.enabled,
                description: "When enabled, new user registrations are automatically approved without admin intervention".to_owned(),
            })
            .ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle getting social insights configuration
pub(super) async fn handle_get_social_insights_config(
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
        "Getting social insights config by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    let config = ctx
        .database
        .get_social_insights_config()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to get social insights config");
            AppError::internal(format!("Failed to get social insights config: {e}"))
        })?
        .unwrap_or_default();

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Social insights configuration retrieved".to_owned(),
            data: to_value(&config).ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle setting social insights configuration
pub(super) async fn handle_set_social_insights_config(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Json(config): Json<SocialInsightsConfig>,
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
        "Setting social insights config by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    ctx.database
        .set_social_insights_config(&config)
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to set social insights config");
            AppError::internal(format!("Failed to set social insights config: {e}"))
        })?;

    info!(
        "Social insights config updated by {}",
        admin_token.service_name
    );

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Social insights configuration updated".to_owned(),
            data: to_value(&config).ok(),
        },
        StatusCode::OK,
    ))
}

/// Handle resetting social insights configuration to defaults
pub(super) async fn handle_reset_social_insights_config(
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
        "Resetting social insights config to defaults by service: {}",
        admin_token.service_name
    );

    let ctx = context.as_ref();

    ctx.database
        .delete_social_insights_config()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to reset social insights config");
            AppError::internal(format!("Failed to reset social insights config: {e}"))
        })?;

    let default_config = SocialInsightsConfig::default();

    info!(
        "Social insights config reset to defaults by {}",
        admin_token.service_name
    );

    Ok(json_response(
        AdminResponse {
            success: true,
            message: "Social insights configuration reset to defaults".to_owned(),
            data: to_value(&default_config).ok(),
        },
        StatusCode::OK,
    ))
}
