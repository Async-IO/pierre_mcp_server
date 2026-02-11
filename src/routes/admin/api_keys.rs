// ABOUTME: Admin API key management route handlers
// ABOUTME: Handles provisioning, revocation, listing, and token info for API keys
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Extension, Json,
};
use serde_json::{from_slice, json, to_value};
use tokio::task;
use tracing::{info, warn};

use crate::{
    admin::{models::ValidatedAdminToken, AdminPermission as AdminPerm},
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    constants::{
        tiers,
        time_constants::{SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MONTH, SECONDS_PER_WEEK},
    },
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, AppResult},
    models::User,
};

use super::types::{
    AdminResponse, ListApiKeysQuery, ProvisionApiKeyRequest, ProvisionApiKeyResponse,
    RateLimitInfo, RevokeKeyRequest,
};
use super::AdminApiContext;

/// Helper function for JSON responses with status
pub(super) fn json_response<T: serde::Serialize>(
    value: T,
    status: StatusCode,
) -> impl IntoResponse {
    (status, Json(value))
}

/// Convert rate limit period string to window duration in seconds
fn convert_rate_limit_period(period: &str) -> AppResult<u32> {
    match period.to_lowercase().as_str() {
        "hour" => Ok(SECONDS_PER_HOUR),
        "day" => Ok(SECONDS_PER_DAY),
        "week" => Ok(SECONDS_PER_WEEK),
        "month" => Ok(SECONDS_PER_MONTH),
        other => Err(AppError::invalid_input(format!(
            "Invalid rate limit period: {other}. Valid options: hour, day, week, month"
        ))),
    }
}

/// Validate API key tier from string
fn validate_tier(tier_str: &str) -> Result<ApiKeyTier, String> {
    match tier_str {
        tiers::TRIAL => Ok(ApiKeyTier::Trial),
        tiers::STARTER => Ok(ApiKeyTier::Starter),
        tiers::PROFESSIONAL => Ok(ApiKeyTier::Professional),
        tiers::ENTERPRISE => Ok(ApiKeyTier::Enterprise),
        _ => Err(format!(
            "Invalid tier: {tier_str}. Supported: trial, starter, professional, enterprise"
        )),
    }
}

/// Get existing user for API key provisioning (no automatic creation)
async fn get_existing_user(database: &Database, email: &str) -> AppResult<User> {
    database
        .get_user_by_email(email)
        .await
        .map_err(|e| AppError::database(format!("Database error looking up user: {e}")))?
        .ok_or_else(|| {
            AppError::not_found(format!(
                "User with email '{email}' not found. Users must register first before API keys can be provisioned."
            ))
        })
}

/// Create and store API key
pub(super) async fn create_and_store_api_key(
    ctx: &AdminApiContext,
    user: &User,
    request: &ProvisionApiKeyRequest,
    tier: &ApiKeyTier,
    admin_token: &ValidatedAdminToken,
) -> Result<(ApiKey, String), String> {
    let api_key_manager = ApiKeyManager::new();
    let create_request = CreateApiKeyRequest {
        name: request
            .description
            .clone()
            .unwrap_or_else(|| format!("API Key provisioned by {}", admin_token.service_name)),
        description: Some(format!(
            "Provisioned by admin service: {}",
            admin_token.service_name
        )),
        tier: tier.clone(),
        rate_limit_requests: request.rate_limit_requests,
        expires_in_days: request.expires_in_days.map(i64::from),
    };

    let (mut final_api_key, api_key_string) =
        match api_key_manager.create_api_key(user.id, create_request) {
            Ok((key, key_string)) => (key, key_string),
            Err(e) => {
                return Err(format!("Failed to generate API key: {e}"));
            }
        };

    if let Some(requests) = request.rate_limit_requests {
        final_api_key.rate_limit_requests = requests;
        if let Some(ref period) = request.rate_limit_period {
            match convert_rate_limit_period(period) {
                Ok(window_seconds) => {
                    final_api_key.rate_limit_window_seconds = window_seconds;
                }
                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }
    }

    if let Err(e) = ctx.database.create_api_key(&final_api_key).await {
        return Err(format!("Failed to create API key: {e}"));
    }

    Ok((final_api_key, api_key_string))
}

/// Create provision response
fn create_provision_response(
    api_key: &ApiKey,
    api_key_string: String,
    user: &User,
    tier: &ApiKeyTier,
    period_name: &str,
) -> ProvisionApiKeyResponse {
    ProvisionApiKeyResponse {
        success: true,
        api_key_id: api_key.id.clone(),
        api_key: api_key_string,
        user_id: user.id.to_string(),
        tier: format!("{tier:?}").to_lowercase(),
        expires_at: api_key.expires_at.map(|dt| dt.to_rfc3339()),
        rate_limit: Some(RateLimitInfo {
            requests: api_key.rate_limit_requests,
            period: period_name.to_owned(),
        }),
    }
}

/// Parse and validate provision API key request
fn parse_provision_request(
    body: &Bytes,
) -> Result<ProvisionApiKeyRequest, (StatusCode, Json<AdminResponse>)> {
    from_slice::<ProvisionApiKeyRequest>(body).map_err(|e| {
        warn!(error = %e, "Invalid JSON body in provision API key request");
        (
            StatusCode::BAD_REQUEST,
            Json(AdminResponse {
                success: false,
                message: format!("Invalid JSON body: {e}"),
                data: None,
            }),
        )
    })
}

/// Check if admin token has provision permission
fn check_provision_permission(
    admin_token: &ValidatedAdminToken,
) -> Result<(), (StatusCode, Json<AdminResponse>)> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::ProvisionKeys)
    {
        return Err((
            StatusCode::FORBIDDEN,
            Json(AdminResponse {
                success: false,
                message: "Permission denied: ProvisionKeys required".to_owned(),
                data: None,
            }),
        ));
    }
    Ok(())
}

/// Validate tier string and return appropriate response on error
fn validate_tier_or_respond(
    tier_str: &str,
) -> Result<ApiKeyTier, (StatusCode, Json<AdminResponse>)> {
    validate_tier(tier_str).map_err(|msg| {
        (
            StatusCode::BAD_REQUEST,
            Json(AdminResponse {
                success: false,
                message: msg,
                data: None,
            }),
        )
    })
}

/// Get user and return appropriate response on error
async fn get_user_or_respond(
    database: &Database,
    email: &str,
) -> Result<User, (StatusCode, Json<AdminResponse>)> {
    get_existing_user(database, email).await.map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(AdminResponse {
                success: false,
                message: e.to_string(),
                data: None,
            }),
        )
    })
}

/// Record API key provisioning action in audit log
async fn record_provisioning_audit(
    database: &Database,
    admin_token: &ValidatedAdminToken,
    api_key: &ApiKey,
    user_email: &str,
    tier: &ApiKeyTier,
    period_name: &str,
) {
    if let Err(e) = database
        .record_admin_provisioned_key(
            &admin_token.token_id,
            &api_key.id,
            user_email,
            &format!("{tier:?}").to_lowercase(),
            api_key.rate_limit_requests,
            period_name,
        )
        .await
    {
        warn!("Failed to record admin provisioned key: {}", e);
    }
}

/// Handle API key provisioning
pub(super) async fn handle_provision_api_key(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    body: Bytes,
) -> AppResult<impl IntoResponse> {
    let request = match parse_provision_request(&body) {
        Ok(req) => req,
        Err(response) => return Ok(response),
    };

    if let Err(response) = check_provision_permission(&admin_token) {
        return Ok(response);
    }

    info!(
        "Provisioning API key for user: {} by service: {}",
        request.user_email, admin_token.service_name
    );

    let ctx = context.as_ref();

    let tier = match validate_tier_or_respond(&request.tier) {
        Ok(t) => t,
        Err(response) => return Ok(response),
    };

    let user = match get_user_or_respond(&ctx.database, &request.user_email).await {
        Ok(u) => u,
        Err(response) => return Ok(response),
    };

    let (final_api_key, api_key_string) =
        match create_and_store_api_key(ctx, &user, &request, &tier, &admin_token).await {
            Ok((key, key_string)) => (key, key_string),
            Err(error_msg) => {
                let status_code = if error_msg.contains("Invalid rate limit period")
                    || error_msg.contains("Invalid tier")
                {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };

                return Ok((
                    status_code,
                    Json(AdminResponse {
                        success: false,
                        message: error_msg,
                        data: None,
                    }),
                ));
            }
        };

    let period_name = request.rate_limit_period.as_deref().unwrap_or("month");
    record_provisioning_audit(
        &ctx.database,
        &admin_token,
        &final_api_key,
        &user.email,
        &tier,
        period_name,
    )
    .await;

    info!(
        "API key provisioned successfully: {} for user: {}",
        final_api_key.id, user.email
    );

    let provision_response =
        create_provision_response(&final_api_key, api_key_string, &user, &tier, period_name);

    Ok((
        StatusCode::CREATED,
        Json(AdminResponse {
            success: true,
            message: format!("API key provisioned successfully for {}", user.email),
            data: to_value(&provision_response).ok(),
        }),
    ))
}

/// Handle API key revocation
pub(super) async fn handle_revoke_api_key(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Json(request): Json<RevokeKeyRequest>,
) -> AppResult<impl IntoResponse> {
    if !admin_token
        .permissions
        .has_permission(&AdminPerm::RevokeKeys)
    {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: RevokeKeys required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!(
        "Revoking API key: {} by service: {}",
        request.api_key_id, admin_token.service_name
    );

    let ctx = context.as_ref();

    let api_key = match ctx
        .database
        .get_api_key_by_id(&request.api_key_id, None)
        .await
    {
        Ok(Some(key)) => key,
        Ok(None) => {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: format!("API key {} not found", request.api_key_id),
                    data: None,
                },
                StatusCode::NOT_FOUND,
            ));
        }
        Err(e) => {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: format!("Failed to lookup API key: {e}"),
                    data: None,
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    match ctx
        .database
        .deactivate_api_key(&request.api_key_id, api_key.user_id)
        .await
    {
        Ok(()) => {
            info!("API key revoked successfully: {}", request.api_key_id);

            Ok(json_response(
                AdminResponse {
                    success: true,
                    message: format!("API key {} revoked successfully", request.api_key_id),
                    data: Some(json!({
                        "api_key_id": request.api_key_id,
                        "revoked_by": admin_token.service_name,
                        "reason": request.reason.unwrap_or_else(|| "Admin revocation".into())
                    })),
                },
                StatusCode::OK,
            ))
        }
        Err(e) => {
            warn!("Failed to revoke API key {}: {}", request.api_key_id, e);

            Ok(json_response(
                AdminResponse {
                    success: false,
                    message: format!("Failed to revoke API key: {e}"),
                    data: None,
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle API key listing
pub(super) async fn handle_list_api_keys(
    State(context): State<Arc<AdminApiContext>>,
    Extension(admin_token): Extension<ValidatedAdminToken>,
    Query(params): Query<ListApiKeysQuery>,
) -> AppResult<impl IntoResponse> {
    if !admin_token.permissions.has_permission(&AdminPerm::ListKeys) {
        return Ok(json_response(
            AdminResponse {
                success: false,
                message: "Permission denied: ListKeys required".to_owned(),
                data: None,
            },
            StatusCode::FORBIDDEN,
        ));
    }

    info!("Listing API keys by service: {}", admin_token.service_name);

    let ctx = context.as_ref();

    let user_email = params.user_email.as_deref();
    let active_only = params.active_only.unwrap_or(true);
    let limit = params
        .limit
        .as_ref()
        .and_then(|s| s.parse::<i32>().ok())
        .map(|l| l.clamp(1, 100));
    let offset = params
        .offset
        .as_ref()
        .and_then(|s| s.parse::<i32>().ok())
        .map(|o| o.max(0));

    match ctx
        .database
        .get_api_keys_filtered(user_email, active_only, limit, offset)
        .await
    {
        Ok(api_keys) => {
            let api_key_responses: Vec<serde_json::Value> = api_keys
                .into_iter()
                .map(|key| {
                    json!({
                        "id": key.id,
                        "user_id": key.user_id.clone(),
                        "name": key.name,
                        "description": key.description,
                        "tier": format!("{:?}", key.tier).to_lowercase(),
                        "rate_limit": {
                            "requests": key.rate_limit_requests,
                            "window": key.rate_limit_window_seconds
                        },
                        "is_active": key.is_active,
                        "created_at": key.created_at.to_rfc3339(),
                        "last_used_at": key.last_used_at.map(|dt| dt.to_rfc3339()),
                        "expires_at": key.expires_at.map(|dt| dt.to_rfc3339()),
                        "usage_count": 0
                    })
                })
                .collect();

            Ok(json_response(
                AdminResponse {
                    success: true,
                    message: format!("Found {} API keys", api_key_responses.len()),
                    data: Some(json!({
                        "filters": {
                            "user_email": user_email,
                            "active_only": active_only,
                            "limit": limit,
                            "offset": offset
                        },
                        "keys": api_key_responses,
                        "count": api_key_responses.len()
                    })),
                },
                StatusCode::OK,
            ))
        }
        Err(e) => {
            warn!("Failed to list API keys: {}", e);
            Ok(json_response(
                AdminResponse {
                    success: false,
                    message: format!("Failed to list API keys: {e}"),
                    data: None,
                },
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle token info (GET /admin/token-info)
/// Returns information about the authenticated admin token
pub(super) async fn handle_token_info(
    Extension(admin_token): Extension<ValidatedAdminToken>,
) -> Json<serde_json::Value> {
    let token_id = admin_token.token_id;
    let service_name = admin_token.service_name.clone();
    let permissions = admin_token.permissions.clone();
    let is_super_admin = admin_token.is_super_admin;

    let token_info_json = task::spawn_blocking(move || {
        let permission_strings: Vec<String> = permissions
            .to_vec()
            .iter()
            .map(ToString::to_string)
            .collect();

        json!({
            "token_id": token_id,
            "service_name": service_name,
            "permissions": permission_strings,
            "is_super_admin": is_super_admin
        })
    })
    .await
    .unwrap_or_else(|_| {
        json!({
            "error": "Failed to serialize token info"
        })
    });

    Json(token_info_json)
}
