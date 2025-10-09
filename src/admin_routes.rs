// ABOUTME: HTTP route handlers for admin API endpoints and administrative operations
// ABOUTME: Provides REST endpoints for API key provisioning, user management, and admin functions
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Admin API Routes
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource sharing in HTTP route handlers (context.clone())
// - String ownership transfers for API keys, user data, and response models
// - Database result ownership transfers across async boundaries
//!
//! This module provides REST API endpoints for admin services to manage API keys
//! and perform administrative operations on the Pierre MCP Server.

use crate::{
    admin::{auth::AdminAuthService, models::AdminPermission},
    api_keys::{ApiKey, ApiKeyTier},
    auth::AuthManager,
    constants::{
        service_names, tiers,
        time_constants::{SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MONTH, SECONDS_PER_WEEK},
    },
    database_plugins::{factory::Database, DatabaseProvider},
    models::{User, UserStatus},
    utils::auth::extract_bearer_token_owned,
    utils::errors::{operation_error, validation_error},
};
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;
use warp::{
    http::StatusCode,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

/// Admin API context shared across all endpoints
#[derive(Clone)]
pub struct AdminApiContext {
    pub database: Arc<Database>,
    pub auth_service: AdminAuthService,
    pub auth_manager: Arc<AuthManager>,
    pub admin_jwt_secret: String,
}

impl AdminApiContext {
    pub fn new(database: Arc<Database>, jwt_secret: &str, auth_manager: Arc<AuthManager>) -> Self {
        tracing::info!(
            "Creating AdminApiContext with JWT secret (first 10 chars): {}...",
            jwt_secret.chars().take(10).collect::<String>()
        );
        let auth_service = AdminAuthService::new((*database).clone(), jwt_secret); // Safe: Arc clone for auth service
        Self {
            database,
            auth_service,
            auth_manager,
            admin_jwt_secret: jwt_secret.to_string(),
        }
    }
}

/// API Key provisioning request
#[derive(Debug, Deserialize)]
pub struct ProvisionApiKeyRequest {
    pub user_email: String,
    pub tier: String,
    pub description: Option<String>,
    pub expires_in_days: Option<u32>,
    pub rate_limit_requests: Option<u32>,
    pub rate_limit_period: Option<String>,
}

/// API Key provisioning response
#[derive(Debug, Serialize)]
pub struct ProvisionApiKeyResponse {
    pub success: bool,
    pub api_key_id: String,
    pub api_key: String,
    pub user_id: String,
    pub tier: String,
    pub expires_at: Option<String>,
    pub rate_limit: Option<RateLimitInfo>,
}

/// Rate limit information
#[derive(Debug, Serialize)]
pub struct RateLimitInfo {
    pub requests: u32,
    pub period: String,
}

/// API Key management request
#[derive(Debug, Deserialize)]
pub struct RevokeApiKeyRequest {
    pub api_key_id: String,
    pub reason: Option<String>,
}

/// Admin setup request
#[derive(Debug, Deserialize)]
pub struct AdminSetupRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

/// Admin setup response
#[derive(Debug, Serialize)]
pub struct AdminSetupResponse {
    pub user_id: String,
    pub admin_token: String,
    pub message: String,
}

/// Generic admin response
#[derive(Debug, Serialize)]
pub struct AdminResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Admin token info response
#[derive(Debug, Serialize)]
pub struct AdminTokenInfoResponse {
    pub token_id: String,
    pub service_name: String,
    pub permissions: Vec<String>,
    pub is_super_admin: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
    pub usage_count: u64,
}

/// User management request
#[derive(Debug, Deserialize)]
pub struct ApproveUserRequest {
    pub reason: Option<String>,
    /// Auto-create default tenant for single-user workflows
    pub create_default_tenant: Option<bool>,
    /// Custom tenant name (if `create_default_tenant` is true)
    pub tenant_name: Option<String>,
    /// Custom tenant slug (if `create_default_tenant` is true)
    pub tenant_slug: Option<String>,
}

/// User management response
#[derive(Debug, Serialize)]
pub struct UserManagementResponse {
    pub success: bool,
    pub message: String,
    pub user: Option<UserInfo>,
    pub tenant_created: Option<TenantCreatedInfo>,
}

/// Information about created tenant
#[derive(Debug, Serialize)]
pub struct TenantCreatedInfo {
    pub tenant_id: String,
    pub name: String,
    pub slug: String,
    pub plan: String,
}

/// User information for admin responses
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub display_name: Option<String>,
    pub user_status: String,
    pub tier: String,
    pub created_at: String,
    pub last_active: String,
    pub approved_by: Option<String>,
    pub approved_at: Option<String>,
}

/// Pending users list response
#[derive(Debug, Serialize)]
pub struct PendingUsersResponse {
    pub success: bool,
    pub users: Vec<UserInfo>,
    pub count: usize,
}

/// Create admin routes filter
pub fn admin_routes(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = std::convert::Infallible> + Clone {
    // Safe: All context.clone() calls below are Arc clones for HTTP route sharing in warp framework
    let provision_route = provision_api_key_route(context.clone());
    let revoke_route = revoke_api_key_route(context.clone());
    let list_keys_route = list_api_keys_route(context.clone());
    let token_info_route = token_info_route(context.clone());
    let setup_route = admin_setup_route(context.clone());
    let setup_status_route = setup_status_route(context.clone());

    // Admin token management routes
    let admin_tokens_list_route = admin_tokens_list_route(context.clone());
    let admin_tokens_create_route = admin_tokens_create_route(context.clone());
    let admin_tokens_details_route = admin_tokens_details_route(context.clone());
    let admin_tokens_revoke_route = admin_tokens_revoke_route(context.clone());
    let admin_tokens_rotate_route = admin_tokens_rotate_route(context.clone());

    // JWT secret management routes
    let rotate_jwt_secret_route = rotate_jwt_secret_route(context.clone());

    // User management routes
    let pending_users_route = pending_users_route(context.clone());
    let approve_user_route = approve_user_route(context.clone());
    let suspend_user_route = suspend_user_route(context);

    let health_route = admin_health_route();

    let admin_routes = provision_route
        .or(revoke_route)
        .or(list_keys_route)
        .or(token_info_route)
        .or(setup_route)
        .or(setup_status_route)
        .or(admin_tokens_list_route)
        .or(admin_tokens_create_route)
        .or(admin_tokens_details_route)
        .or(admin_tokens_revoke_route)
        .or(admin_tokens_rotate_route)
        .or(rotate_jwt_secret_route)
        .or(pending_users_route)
        .or(approve_user_route)
        .or(suspend_user_route)
        .or(health_route)
        .boxed();

    warp::path("admin")
        .and(admin_routes)
        .recover(handle_admin_rejection)
}

/// Create admin routes filter with proper scoped recovery
/// This handles admin-specific rejections and lets other errors pass through
pub fn admin_routes_with_scoped_recovery(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = std::convert::Infallible> + Clone {
    let provision_route = provision_api_key_route(context.clone());
    let revoke_route = revoke_api_key_route(context.clone());
    let list_keys_route = list_api_keys_route(context.clone());
    let token_info_route = token_info_route(context.clone());
    let setup_route = admin_setup_route(context.clone());
    let setup_status_route = setup_status_route(context.clone());

    // Admin token management routes
    let admin_tokens_list_route = admin_tokens_list_route(context.clone());
    let admin_tokens_create_route = admin_tokens_create_route(context.clone());
    let admin_tokens_details_route = admin_tokens_details_route(context.clone());
    let admin_tokens_revoke_route = admin_tokens_revoke_route(context.clone());
    let admin_tokens_rotate_route = admin_tokens_rotate_route(context.clone());

    // JWT secret management routes
    let rotate_jwt_secret_route = rotate_jwt_secret_route(context.clone());

    // User management routes
    let pending_users_route = pending_users_route(context.clone());
    let approve_user_route = approve_user_route(context.clone());
    let suspend_user_route = suspend_user_route(context);

    let health_route = admin_health_route();

    let admin_routes = provision_route
        .or(revoke_route)
        .or(list_keys_route)
        .or(token_info_route)
        .or(setup_route)
        .or(setup_status_route)
        .or(admin_tokens_list_route)
        .or(admin_tokens_create_route)
        .or(admin_tokens_details_route)
        .or(admin_tokens_revoke_route)
        .or(admin_tokens_rotate_route)
        .or(rotate_jwt_secret_route)
        .or(pending_users_route)
        .or(approve_user_route)
        .or(suspend_user_route)
        .or(health_route);

    warp::path("admin")
        .and(admin_routes)
        .recover(handle_admin_rejection)
}

/// Create admin routes filter without recovery (maintains Rejection error type)
/// This is used for embedding in other servers that handle rejections globally
#[must_use]
pub fn admin_routes_with_rejection(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let provision_route = provision_api_key_route(context.clone());
    let revoke_route = revoke_api_key_route(context.clone());
    let list_keys_route = list_api_keys_route(context.clone());
    let token_info_route = token_info_route(context.clone());
    let setup_route = admin_setup_route(context.clone());
    let setup_status_route = setup_status_route(context.clone());

    // Admin token management routes
    let admin_tokens_list_route = admin_tokens_list_route(context.clone());
    let admin_tokens_create_route = admin_tokens_create_route(context.clone());
    let admin_tokens_details_route = admin_tokens_details_route(context.clone());
    let admin_tokens_revoke_route = admin_tokens_revoke_route(context.clone());
    let admin_tokens_rotate_route = admin_tokens_rotate_route(context.clone());

    // JWT secret management routes
    let rotate_jwt_secret_route = rotate_jwt_secret_route(context.clone());

    // User management routes
    let pending_users_route = pending_users_route(context.clone());
    let approve_user_route = approve_user_route(context.clone());
    let suspend_user_route = suspend_user_route(context);

    let health_route = admin_health_route();

    let admin_routes = provision_route
        .or(revoke_route)
        .or(list_keys_route)
        .or(token_info_route)
        .or(setup_route)
        .or(setup_status_route)
        .or(admin_tokens_list_route)
        .or(admin_tokens_create_route)
        .or(admin_tokens_details_route)
        .or(admin_tokens_revoke_route)
        .or(admin_tokens_rotate_route)
        .or(rotate_jwt_secret_route)
        .or(pending_users_route)
        .or(approve_user_route)
        .or(suspend_user_route)
        .or(health_route);

    warp::path("admin").and(admin_routes)
}

/// Provision API key endpoint
fn provision_api_key_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("provision-api-key")
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ProvisionKeys,
        ))
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_provision_api_key)
}

/// Revoke API key endpoint
fn revoke_api_key_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("revoke-api-key")
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::RevokeKeys,
        ))
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_revoke_api_key)
}

/// List API keys endpoint
fn list_api_keys_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("list-api-keys")
        .and(warp::get())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ListKeys,
        ))
        .and(warp::query::<HashMap<String, String>>())
        .and(with_context(context))
        .and_then(handle_list_api_keys)
}

/// Admin token info endpoint
fn token_info_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("token-info")
        .and(warp::get())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ManageAdminTokens,
        ))
        .and(with_context(context))
        .and_then(handle_token_info)
}

/// Admin setup endpoint - creates first admin user and returns token
fn admin_setup_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("setup")
        .and(warp::post())
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_admin_setup)
}

/// Admin setup status check endpoint (no auth required)
fn setup_status_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("setup-status")
        .and(warp::get())
        .and(with_context(context))
        .and_then(handle_setup_status)
}

/// Admin health check endpoint
fn admin_health_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("health").and(warp::get()).map(|| {
        json(&serde_json::json!({
            "status": "healthy",
            "service": service_names::PIERRE_MCP_ADMIN_API,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "version": env!("CARGO_PKG_VERSION")
        }))
    })
}

/// Admin authentication filter
fn admin_auth_filter(
    context: AdminApiContext,
    required_permission: AdminPermission,
) -> impl Filter<Extract = (crate::admin::models::ValidatedAdminToken,), Error = Rejection> + Clone
{
    warp::header::<String>("authorization")
        .and(warp::header::optional::<String>("x-forwarded-for"))
        .and(warp::header::optional::<String>("x-real-ip"))
        .and(warp::addr::remote())
        .and_then(
            move |auth_header: String,
                  x_forwarded_for: Option<String>,
                  x_real_ip: Option<String>,
                  remote_addr: Option<std::net::SocketAddr>| {
                let context = context.clone();
                let required_permission = required_permission.clone();

                async move {
                    // Extract Bearer token
                    let token = extract_bearer_token_owned(&auth_header)
                        .map_err(|_| warp::reject::custom(AdminApiError::InvalidAuthHeader))?;

                    // Extract client IP from headers or remote address
                    let ip_address = extract_client_ip(x_forwarded_for, x_real_ip, remote_addr);

                    // Authenticate and authorize
                    context
                        .auth_service
                        .authenticate_and_authorize(
                            &token,
                            required_permission,
                            ip_address.as_deref(),
                        )
                        .await
                        .map_err(|e| {
                            warn!("Admin authentication failed: {}", e);
                            warp::reject::custom(AdminApiError::AuthenticationFailed(e.to_string()))
                        })
                }
            },
        )
}

/// Extract client IP from headers or remote address
fn extract_client_ip(
    x_forwarded_for: Option<String>,
    x_real_ip: Option<String>,
    remote_addr: Option<std::net::SocketAddr>,
) -> Option<String> {
    // Priority: X-Forwarded-For > X-Real-IP > Remote Address
    x_forwarded_for.map_or_else(
        || {
            x_real_ip.map_or_else(
                || remote_addr.map(|addr| addr.ip().to_string()),
                |real_ip| Some(real_ip.trim().to_string()),
            )
        },
        |xff| {
            // X-Forwarded-For can contain multiple IPs, take the first one
            xff.split(',').next().map(|ip| ip.trim().to_string())
        },
    )
}

/// Convert rate limit period string to window duration in seconds
fn convert_rate_limit_period(period: &str) -> Result<u32> {
    match period.to_lowercase().as_str() {
        "hour" => Ok(SECONDS_PER_HOUR),   // 1 hour
        "day" => Ok(SECONDS_PER_DAY),     // 24 hours
        "week" => Ok(SECONDS_PER_WEEK),   // 7 days
        "month" => Ok(SECONDS_PER_MONTH), // 30 days
        _ => Err(anyhow!(
            "Invalid rate limit period. Supported: hour, day, week, month"
        )),
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

/// Create and store API key
async fn create_and_store_api_key(
    context: &AdminApiContext,
    user: &User,
    request: &ProvisionApiKeyRequest,
    tier: &ApiKeyTier,
    admin_token: &crate::admin::models::ValidatedAdminToken,
) -> Result<(crate::api_keys::ApiKey, String), String> {
    // Generate API key using ApiKeyManager
    let api_key_manager = crate::api_keys::ApiKeyManager::new();
    let create_request = crate::api_keys::CreateApiKeyRequest {
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

    // Apply custom rate limits if provided
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

    // Store API key
    if let Err(e) = context.database.create_api_key(&final_api_key).await {
        return Err(format!("Failed to create API key: {e}"));
    }

    Ok((final_api_key, api_key_string))
}

/// Helper to inject context into filters
fn with_context(
    context: AdminApiContext,
) -> impl Filter<Extract = (AdminApiContext,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || context.clone())
}

/// Get existing user for API key provisioning (no automatic creation)
async fn get_existing_user(database: &Database, email: &str) -> Result<User, warp::Rejection> {
    match database.get_user_by_email(email).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => {
            tracing::warn!("API key provisioning failed: User {} does not exist", email);
            Err(warp::reject::custom(AdminApiError::InvalidRequest(
                format!("User {email} must register and be approved before API key provisioning"),
            )))
        }
        Err(e) => Err(warp::reject::custom(AdminApiError::DatabaseError(format!(
            "Failed to lookup user: {e}"
        )))),
    }
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
            period: period_name.to_string(),
        }),
    }
}

/// Handle API key provisioning
async fn handle_provision_api_key(
    admin_token: crate::admin::models::ValidatedAdminToken,
    request: ProvisionApiKeyRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Key Provisioning API key for user: {} by service: {}",
        request.user_email, admin_token.service_name
    );

    // Validate tier
    let tier = match validate_tier(&request.tier) {
        Ok(t) => t,
        Err(error_msg) => {
            return Ok(with_status(
                json(&AdminResponse {
                    success: false,
                    message: error_msg,
                    data: None,
                }),
                StatusCode::BAD_REQUEST,
            ));
        }
    };

    // Get existing user (no automatic creation)
    let Ok(user) = get_existing_user(&context.database, &request.user_email).await else {
        return Ok(with_status(
            json(&AdminResponse {
                success: false,
                message: format!("Failed to lookup user: {}", request.user_email),
                data: None,
            }),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    };

    // Create and store API key
    let (final_api_key, api_key_string) =
        match create_and_store_api_key(&context, &user, &request, &tier, &admin_token).await {
            Ok((key, key_string)) => (key, key_string),
            Err(error_msg) => {
                // Check if this is a validation error or server error
                let status_code = if error_msg.contains("Invalid rate limit period")
                    || error_msg.contains("Invalid tier")
                {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };

                return Ok(with_status(
                    json(&AdminResponse {
                        success: false,
                        message: error_msg,
                        data: None,
                    }),
                    status_code,
                ));
            }
        };

    // Record the provisioning action for audit
    let period_name = request.rate_limit_period.as_deref().unwrap_or("month");
    if let Err(e) = context
        .database
        .record_admin_provisioned_key(
            &admin_token.token_id,
            &final_api_key.id,
            &user.email,
            &format!("{tier:?}").to_lowercase(),
            final_api_key.rate_limit_requests,
            period_name,
        )
        .await
    {
        warn!("Failed to record admin provisioned key: {}", e);
    }

    info!(
        "API key provisioned successfully: {} for user: {}",
        final_api_key.id, user.email
    );

    let response =
        create_provision_response(&final_api_key, api_key_string, &user, &tier, period_name);
    Ok(with_status(json(&response), StatusCode::CREATED))
}

/// Handle API key revocation
async fn handle_revoke_api_key(
    admin_token: crate::admin::models::ValidatedAdminToken,
    request: RevokeApiKeyRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Revoking API key: {} by service: {}",
        request.api_key_id, admin_token.service_name
    );

    // Get the API key to find the user_id
    let api_key = match context
        .database
        .get_api_key_by_id(&request.api_key_id)
        .await
    {
        Ok(Some(key)) => key,
        Ok(None) => {
            let response = AdminResponse {
                success: false,
                message: format!("API key {} not found", request.api_key_id),
                data: None,
            };
            return Ok(with_status(json(&response), StatusCode::NOT_FOUND));
        }
        Err(e) => {
            let response = AdminResponse {
                success: false,
                message: format!("Failed to lookup API key: {e}"),
                data: None,
            };
            return Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    match context
        .database
        .deactivate_api_key(&request.api_key_id, api_key.user_id)
        .await
    {
        Ok(()) => {
            info!("API key revoked successfully: {}", request.api_key_id);

            let response = AdminResponse {
                success: true,
                message: format!("API key {} revoked successfully", request.api_key_id),
                data: Some(serde_json::json!({
                    "api_key_id": request.api_key_id,
                    "revoked_by": admin_token.service_name,
                    "reason": request.reason.unwrap_or_else(|| "Admin revocation".into())
                })),
            };

            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to revoke API key {}: {}", request.api_key_id, e);

            let response = AdminResponse {
                success: false,
                message: format!("Failed to revoke API key: {e}"),
                data: None,
            };

            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle API key listing
async fn handle_list_api_keys(
    admin_token: crate::admin::models::ValidatedAdminToken,
    query: HashMap<String, String>,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "List Listing API keys by service: {}",
        admin_token.service_name
    );

    // Parse query parameters
    let user_email = query.get("user_email").map(std::string::String::as_str);
    let active_only = query
        .get("active_only")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(true);
    let limit = query
        .get("limit")
        .and_then(|v| v.parse::<i32>().ok())
        .map(|l| l.clamp(1, 100)); // Limit between 1-100
    let offset = query
        .get("offset")
        .and_then(|v| v.parse::<i32>().ok())
        .map(|o| o.max(0)); // Ensure non-negative

    // Get API keys from database
    match context
        .database
        .get_api_keys_filtered(user_email, active_only, limit, offset)
        .await
    {
        Ok(api_keys) => {
            let api_key_responses: Vec<serde_json::Value> = api_keys
                .into_iter()
                .map(|key| {
                    serde_json::json!({
                        "id": key.id,
                        "user_id": key.user_id.to_string(),
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

            let response = AdminResponse {
                success: true,
                message: format!("Found {} API keys", api_key_responses.len()),
                data: Some(serde_json::json!({
                    "filters": {
                        "user_email": user_email,
                        "active_only": active_only,
                        "limit": limit,
                        "offset": offset
                    },
                    "keys": api_key_responses,
                    "count": api_key_responses.len()
                })),
            };

            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to list API keys: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to list API keys: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle admin token info
async fn handle_token_info(
    admin_token: crate::admin::models::ValidatedAdminToken,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Getting token info for service: {}",
        admin_token.service_name
    );

    // Get full token details from database
    match context
        .database
        .get_admin_token_by_id(&admin_token.token_id)
        .await
    {
        Ok(Some(token_details)) => {
            let response = AdminTokenInfoResponse {
                token_id: token_details.id,
                service_name: token_details.service_name,
                permissions: token_details
                    .permissions
                    .to_vec()
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect(),
                is_super_admin: token_details.is_super_admin,
                created_at: token_details.created_at.to_rfc3339(),
                last_used_at: token_details.last_used_at.map(|dt| dt.to_rfc3339()),
                usage_count: token_details.usage_count,
            };

            Ok(with_status(json(&response), StatusCode::OK))
        }
        Ok(None) => {
            let response = AdminResponse {
                success: false,
                message: "Token not found in database".into(),
                data: None,
            };

            Ok(with_status(json(&response), StatusCode::NOT_FOUND))
        }
        Err(e) => {
            let response = AdminResponse {
                success: false,
                message: format!("Failed to retrieve token info: {e}"),
                data: None,
            };

            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle admin setup - create first admin user and return token
// Long function: Creates complete initial admin setup with validation and token generation
#[allow(clippy::too_many_lines)]
async fn handle_admin_setup(
    request: AdminSetupRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!("Admin setup request for email: {}", request.email);

    // First check if any admin users already exist
    match context.database.get_users_by_status("active").await {
        Ok(users) => {
            let admin_exists = users.iter().any(|u| u.is_admin);
            if admin_exists {
                let response = AdminResponse {
                    success: false,
                    message: "Admin user already exists. Use admin token management instead."
                        .into(),
                    data: None,
                };
                return Ok(with_status(json(&response), StatusCode::CONFLICT));
            }
        }
        Err(e) => {
            error!("Failed to check existing admin users: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Database error: {e}"),
                data: None,
            };
            return Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    }

    // Create admin user
    let user_id = Uuid::new_v4();
    let password_hash = match bcrypt::hash(&request.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            error!("Failed to hash password: {}", e);
            let response = AdminResponse {
                success: false,
                message: "Failed to process password".into(),
                data: None,
            };
            return Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    // Create admin user struct
    let mut admin_user = crate::models::User::new(
        request.email.clone(),
        password_hash,
        request.display_name.clone(),
    );
    admin_user.id = user_id;
    admin_user.is_admin = true;
    admin_user.user_status = crate::models::UserStatus::Active;

    // Create user in database
    match context.database.create_user(&admin_user).await {
        Ok(_) => {
            info!("Admin user created successfully: {}", request.email);

            // Generate admin token immediately
            let token_request = crate::admin::models::CreateAdminTokenRequest {
                service_name: "initial_admin_setup".to_string(),
                service_description: Some("Initial admin setup token".to_string()),
                permissions: Some(vec![
                    crate::admin::models::AdminPermission::ManageUsers,
                    crate::admin::models::AdminPermission::ManageAdminTokens,
                    crate::admin::models::AdminPermission::ProvisionKeys,
                    crate::admin::models::AdminPermission::ListKeys,
                    crate::admin::models::AdminPermission::UpdateKeyLimits,
                    crate::admin::models::AdminPermission::RevokeKeys,
                    crate::admin::models::AdminPermission::ViewAuditLogs,
                ]),
                is_super_admin: true,
                expires_in_days: Some(365),
            };

            match context
                .database
                .create_admin_token(&token_request, &context.admin_jwt_secret)
                .await
            {
                Ok(generated_token) => {
                    let response = AdminSetupResponse {
                        user_id: user_id.to_string(),
                        admin_token: generated_token.jwt_token,
                        message: format!(
                            "Admin user {} created successfully with token",
                            request.email
                        ),
                    };
                    info!("Admin setup completed successfully for: {}", request.email);
                    Ok(with_status(json(&response), StatusCode::CREATED))
                }
                Err(e) => {
                    error!("Failed to generate admin token after creating user: {}", e);
                    let response = AdminResponse {
                        success: false,
                        message: format!("User created but token generation failed: {e}"),
                        data: None,
                    };
                    Ok(with_status(
                        json(&response),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    ))
                }
            }
        }
        Err(e) => {
            error!("Failed to create admin user: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to create admin user: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle setup status check - returns whether admin setup is needed
async fn handle_setup_status(context: AdminApiContext) -> Result<impl Reply, Rejection> {
    info!("Setup status check requested");

    match context
        .auth_manager
        .check_setup_status(&context.database)
        .await
    {
        Ok(setup_status) => {
            info!(
                "Setup status check successful: needs_setup={}, admin_user_exists={}",
                setup_status.needs_setup, setup_status.admin_user_exists
            );
            Ok(with_status(json(&setup_status), StatusCode::OK))
        }
        Err(e) => {
            error!("Failed to check setup status: {}", e);
            let error_response = serde_json::json!({
                "needs_setup": true,
                "admin_user_exists": false,
                "message": "Unable to determine setup status. Please ensure admin user is created."
            });
            Ok(with_status(
                json(&error_response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Admin API error types
#[derive(Debug, thiserror::Error)]
pub enum AdminApiError {
    #[error("Invalid authentication header")]
    InvalidAuthHeader,
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

impl warp::reject::Reject for AdminApiError {}

/// Handle admin API rejections - only handle admin-specific errors
async fn handle_admin_rejection(err: Rejection) -> Result<impl Reply, std::convert::Infallible> {
    // Check if this is an admin-specific error first
    if matches!(err.find(), Some(AdminApiError::InvalidAuthHeader)) {
        let response = AdminResponse {
            success: false,
            message: "Invalid Authorization header".to_string(),
            data: None,
        };
        return Ok(with_status(json(&response), StatusCode::BAD_REQUEST));
    }

    if let Some(AdminApiError::AuthenticationFailed(msg)) = err.find() {
        let response = AdminResponse {
            success: false,
            message: msg.clone(),
            data: None,
        };
        return Ok(with_status(json(&response), StatusCode::UNAUTHORIZED));
    }

    if let Some(AdminApiError::DatabaseError(msg)) = err.find() {
        let response = AdminResponse {
            success: false,
            message: msg.clone(),
            data: None,
        };
        return Ok(with_status(
            json(&response),
            StatusCode::INTERNAL_SERVER_ERROR,
        ));
    }

    if let Some(AdminApiError::InvalidRequest(msg)) = err.find() {
        let response = AdminResponse {
            success: false,
            message: msg.clone(),
            data: None,
        };
        return Ok(with_status(json(&response), StatusCode::BAD_REQUEST));
    }

    // For other errors within admin routes (body parsing, missing headers, etc.)
    let (status, message) = if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        (StatusCode::BAD_REQUEST, "Invalid JSON body")
    } else if err.find::<warp::reject::MissingHeader>().is_some() {
        (StatusCode::BAD_REQUEST, "Missing required header")
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        (StatusCode::METHOD_NOT_ALLOWED, "Method not allowed")
    } else if err.is_not_found() {
        // This should only happen for admin routes under /admin/*
        (StatusCode::NOT_FOUND, "Admin endpoint not found")
    } else {
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
    };

    let response = AdminResponse {
        success: false,
        message: message.to_string(),
        data: None,
    };

    Ok(with_status(json(&response), status))
}

/// Admin token management routes
/// List admin tokens endpoint
fn admin_tokens_list_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("tokens")
        .and(warp::get())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ProvisionKeys, // Admin with provision permission can view tokens
        ))
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(with_context(context))
        .and_then(handle_admin_tokens_list)
}

/// Create admin token endpoint
fn admin_tokens_create_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("tokens")
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ProvisionKeys, // Admin with provision permission can create tokens
        ))
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_admin_tokens_create)
}

/// Get admin token details endpoint
fn admin_tokens_details_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("tokens" / String)
        .and(warp::get())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ProvisionKeys,
        ))
        .and(with_context(context))
        .and_then(handle_admin_tokens_details)
}

/// Revoke admin token endpoint
fn admin_tokens_revoke_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("tokens" / String / "revoke")
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::RevokeKeys,
        ))
        .and(with_context(context))
        .and_then(handle_admin_tokens_revoke)
}

/// Rotate admin token endpoint
fn admin_tokens_rotate_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("tokens" / String / "rotate")
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ProvisionKeys,
        ))
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_admin_tokens_rotate)
}

/// JWT secret rotation endpoint
fn rotate_jwt_secret_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("jwt-secret" / "rotate")
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ManageAdminTokens, // Only super admins can rotate JWT secrets
        ))
        .and(with_context(context))
        .and_then(handle_rotate_jwt_secret)
}

#[derive(Debug, serde::Deserialize)]
struct CreateAdminTokenRequest {
    service_name: String,
    service_description: Option<String>,
    is_super_admin: Option<bool>,
    expires_in_days: Option<u64>,
    permissions: Option<Vec<String>>, // Custom permissions as strings
}

#[derive(Debug, serde::Deserialize)]
struct RotateAdminTokenRequest {
    expires_in_days: Option<u64>,
}

/// Handle list admin tokens
async fn handle_admin_tokens_list(
    _admin_token: crate::admin::models::ValidatedAdminToken,
    query_params: std::collections::HashMap<String, String>,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!("List Listing admin tokens");

    let include_inactive = query_params
        .get("include_inactive")
        .and_then(|v| v.parse::<bool>().ok())
        .unwrap_or(false);

    match context.database.list_admin_tokens(include_inactive).await {
        Ok(tokens) => {
            let response = AdminResponse {
                success: true,
                message: format!("Found {} admin tokens", tokens.len()),
                data: Some(serde_json::json!({
                    "tokens": tokens,
                    "count": tokens.len(),
                    "include_inactive": include_inactive
                })),
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to list admin tokens: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to list admin tokens: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle create admin token
async fn handle_admin_tokens_create(
    _admin_token: crate::admin::models::ValidatedAdminToken,
    request: CreateAdminTokenRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Key Creating admin token for service: {}",
        request.service_name
    );

    // Create token request
    let mut token_request = if request.is_super_admin.unwrap_or(false) {
        crate::admin::models::CreateAdminTokenRequest::super_admin(request.service_name.clone())
    } else {
        crate::admin::models::CreateAdminTokenRequest::new(request.service_name.clone())
    };

    if let Some(desc) = request.service_description {
        token_request.service_description = Some(desc);
    }

    if let Some(expires) = request.expires_in_days {
        if expires == 0 {
            token_request.expires_in_days = None; // Never expires
        } else {
            token_request.expires_in_days = Some(expires);
        }
    }

    // Handle custom permissions from request.permissions
    if let Some(permission_strings) = request.permissions {
        // Parse permission strings into AdminPermission enum values
        let mut parsed_permissions = Vec::new();
        for perm_str in permission_strings {
            if let Ok(permission) = perm_str.parse::<AdminPermission>() {
                parsed_permissions.push(permission);
            } else {
                warn!("Invalid permission string: {}", perm_str);
                let response = AdminResponse {
                    success: false,
                    message: format!("Invalid permission: {perm_str}"),
                    data: None,
                };
                return Ok(with_status(json(&response), StatusCode::BAD_REQUEST));
            }
        }

        if !parsed_permissions.is_empty() {
            token_request.permissions = Some(parsed_permissions);
        }
    }

    match context
        .database
        .create_admin_token(&token_request, &context.admin_jwt_secret)
        .await
    {
        Ok(generated_token) => {
            info!(
                "Admin token created successfully: {}",
                generated_token.token_id
            );
            let response = AdminResponse {
                success: true,
                message: "Admin token created successfully".into(),
                data: Some(serde_json::json!(generated_token)),
            };
            Ok(with_status(json(&response), StatusCode::CREATED))
        }
        Err(e) => {
            warn!("Failed to create admin token: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to create admin token: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle get admin token details
async fn handle_admin_tokens_details(
    token_id: String,
    _admin_token: crate::admin::models::ValidatedAdminToken,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!("Getting admin token details: {}", token_id);

    match context.database.get_admin_token_by_id(&token_id).await {
        Ok(Some(token)) => {
            let response = AdminResponse {
                success: true,
                message: "Admin token details retrieved".into(),
                data: Some(serde_json::json!(token)),
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Ok(None) => {
            let response = AdminResponse {
                success: false,
                message: "Admin token not found".into(),
                data: None,
            };
            Ok(with_status(json(&response), StatusCode::NOT_FOUND))
        }
        Err(e) => {
            warn!("Failed to get admin token details: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to get admin token details: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle revoke admin token
async fn handle_admin_tokens_revoke(
    token_id: String,
    _admin_token: crate::admin::models::ValidatedAdminToken,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!("Revoking admin token: {}", token_id);

    match context.database.deactivate_admin_token(&token_id).await {
        Ok(()) => {
            info!("Admin token revoked successfully: {}", token_id);
            let response = AdminResponse {
                success: true,
                message: "Admin token revoked successfully".into(),
                data: Some(serde_json::json!({
                    "token_id": token_id,
                    "status": "revoked"
                })),
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to revoke admin token: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to revoke admin token: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle rotate admin token
async fn handle_admin_tokens_rotate(
    token_id: String,
    _admin_token: crate::admin::models::ValidatedAdminToken,
    request: RotateAdminTokenRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!("Rotating admin token: {}", token_id);

    // Get existing token first
    let old_token = match context.database.get_admin_token_by_id(&token_id).await {
        Ok(Some(token)) => token,
        Ok(None) => {
            let response = AdminResponse {
                success: false,
                message: "Admin token not found".into(),
                data: None,
            };
            return Ok(with_status(json(&response), StatusCode::NOT_FOUND));
        }
        Err(e) => {
            warn!("Failed to get admin token for rotation: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to get admin token: {e}"),
                data: None,
            };
            return Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    // Create new token with same properties
    let mut new_token_request = crate::admin::models::CreateAdminTokenRequest {
        service_name: old_token.service_name.clone(),
        service_description: old_token.service_description.clone(),
        permissions: Some(old_token.permissions.to_vec()),
        expires_in_days: request.expires_in_days.or(Some(365)),
        is_super_admin: old_token.is_super_admin,
    };

    if old_token.is_super_admin {
        new_token_request.expires_in_days = None; // Super admin tokens never expire
    }

    // Create new token and revoke old one
    match context
        .database
        .create_admin_token(&new_token_request, &context.admin_jwt_secret)
        .await
    {
        Ok(new_token) => {
            // Revoke old token
            if let Err(e) = context.database.deactivate_admin_token(&token_id).await {
                warn!("Failed to revoke old token during rotation: {}", e);
                // Continue anyway since new token was created
            }

            info!(
                "Admin token rotated successfully: {} -> {}",
                token_id, new_token.token_id
            );
            let response = AdminResponse {
                success: true,
                message: "Admin token rotated successfully".into(),
                data: Some(serde_json::json!({
                    "old_token_id": token_id,
                    "new_token": new_token
                })),
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to create new token during rotation: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to rotate admin token: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle JWT secret rotation
async fn handle_rotate_jwt_secret(
    admin_token: crate::admin::models::ValidatedAdminToken,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Rotating JWT secret requested by admin: {}",
        admin_token.service_name
    );

    // Only super admins can rotate JWT secrets
    if !admin_token.is_super_admin {
        let response = AdminResponse {
            success: false,
            message: "Only super admin tokens can rotate JWT secrets".into(),
            data: None,
        };
        return Ok(with_status(json(&response), StatusCode::FORBIDDEN));
    }

    // Generate new JWT secret
    let new_jwt_secret = crate::admin::jwt::AdminJwtManager::generate_jwt_secret();

    // Update the secret in database
    match context
        .database
        .update_system_secret("admin_jwt_secret", &new_jwt_secret)
        .await
    {
        Ok(()) => {
            info!(
                "JWT secret rotated successfully by admin: {}",
                admin_token.service_name
            );

            // Record admin action for audit trail
            let usage = crate::admin::models::AdminTokenUsage {
                id: None,
                admin_token_id: admin_token.token_id,
                timestamp: chrono::Utc::now(),
                action: crate::admin::models::AdminAction::ViewAuditLogs, // Closest available action
                target_resource: Some("jwt_secret".to_string()),
                ip_address: None,
                user_agent: None,
                request_size_bytes: None,
                success: true,
                error_message: None,
                response_time_ms: None,
            };

            // Record usage (ignore errors for audit)
            if let Err(e) = context.database.record_admin_token_usage(&usage).await {
                warn!("Failed to record admin token usage for JWT rotation: {}", e);
            }

            let response = AdminResponse {
                success: true,
                message: "JWT secret rotated successfully. All existing admin tokens are now invalid and must be regenerated.".into(),
                data: Some(serde_json::json!({
                    "rotated_at": chrono::Utc::now(),
                    "rotated_by": admin_token.service_name,
                    "warning": "All existing admin tokens are now invalid and must be regenerated"
                })),
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to rotate JWT secret: {}", e);

            // Record failed action for audit trail
            let usage = crate::admin::models::AdminTokenUsage {
                id: None,
                admin_token_id: admin_token.token_id,
                timestamp: chrono::Utc::now(),
                action: crate::admin::models::AdminAction::ViewAuditLogs,
                target_resource: Some("jwt_secret".to_string()),
                ip_address: None,
                user_agent: None,
                request_size_bytes: None,
                success: false,
                error_message: Some(e.to_string()),
                response_time_ms: None,
            };

            // Record usage (ignore errors for audit)
            if let Err(e) = context.database.record_admin_token_usage(&usage).await {
                warn!(
                    "Failed to record admin token usage for failed JWT rotation: {}",
                    e
                );
            }

            let response = AdminResponse {
                success: false,
                message: format!("Failed to rotate JWT secret: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Pending users route
fn pending_users_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("pending-users")
        .and(warp::get())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ManageUsers,
        ))
        .and(with_context(context))
        .and_then(handle_pending_users)
}

/// Approve user route  
fn approve_user_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("approve-user" / String)
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ManageUsers,
        ))
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_approve_user)
}

/// Suspend user route
fn suspend_user_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("suspend-user" / String)
        .and(warp::post())
        .and(admin_auth_filter(
            context.clone(),
            AdminPermission::ManageUsers,
        ))
        .and(warp::body::json())
        .and(with_context(context))
        .and_then(handle_suspend_user)
}

/// Handle pending users list
async fn handle_pending_users(
    _admin_token: crate::admin::models::ValidatedAdminToken,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!("Admin requesting pending users list");

    match get_users_by_status(&context.database, UserStatus::Pending).await {
        Ok(users) => {
            let user_infos: Vec<UserInfo> = users.into_iter().map(user_to_info).collect();

            let count = user_infos.len();
            let response = PendingUsersResponse {
                success: true,
                users: user_infos,
                count,
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to get pending users: {}", e);
            let response = AdminResponse {
                success: false,
                message: format!("Failed to get pending users: {e}"),
                data: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle user approval
async fn handle_approve_user(
    user_id: String,
    admin_token: crate::admin::models::ValidatedAdminToken,
    request: ApproveUserRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Admin approving user: {} by service: {}",
        user_id, admin_token.service_name
    );

    let Ok(user_uuid) = Uuid::parse_str(&user_id) else {
        let response = UserManagementResponse {
            success: false,
            message: "Invalid user ID format".into(),
            user: None,
            tenant_created: None,
        };
        return Ok(with_status(json(&response), StatusCode::BAD_REQUEST));
    };

    // Use JWT token ID as the approval authority (consistent with suspend_user_status)
    // This eliminates the fragile dependency on finding an admin user in the database
    match approve_user_with_optional_tenant(
        &context.database,
        user_uuid,
        &admin_token.token_id,
        &request,
    )
    .await
    {
        Ok((user, tenant_info)) => {
            info!("User approved successfully: {}", user.email);
            let mut success_message = format!(
                "User {} approved successfully{}",
                user.email,
                request
                    .reason
                    .map(|r| format!(" (Reason: {r})"))
                    .unwrap_or_default()
            );

            let tenant_created = if let Some(tenant) = tenant_info {
                use std::fmt::Write;
                let _ = write!(
                    &mut success_message,
                    " and default tenant '{}' created",
                    tenant.name
                );
                Some(TenantCreatedInfo {
                    tenant_id: tenant.id.to_string(),
                    name: tenant.name,
                    slug: tenant.slug,
                    plan: tenant.plan,
                })
            } else {
                None
            };

            let response = UserManagementResponse {
                success: true,
                message: success_message,
                user: Some(user_to_info(user)),
                tenant_created,
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            error!("Failed to approve user {}: {}", user_id, e);
            error!(
                "Error context - admin token: {}, user UUID: {}",
                admin_token.token_id, user_uuid
            );
            error!("Full error chain: {:#}", e);

            let response = UserManagementResponse {
                success: false,
                message: format!("Failed to approve user: {e}"),
                user: None,
                tenant_created: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Handle user suspension
async fn handle_suspend_user(
    user_id: String,
    admin_token: crate::admin::models::ValidatedAdminToken,
    request: ApproveUserRequest,
    context: AdminApiContext,
) -> Result<impl Reply, Rejection> {
    info!(
        "Admin suspending user: {} by service: {}",
        user_id, admin_token.service_name
    );

    let Ok(user_uuid) = Uuid::parse_str(&user_id) else {
        let response = UserManagementResponse {
            success: false,
            message: "Invalid user ID format".into(),
            user: None,
            tenant_created: None,
        };
        return Ok(with_status(json(&response), StatusCode::BAD_REQUEST));
    };

    match suspend_user_status(&context.database, user_uuid, &admin_token.token_id).await {
        Ok(user) => {
            info!("User suspended successfully: {}", user.email);
            let response = UserManagementResponse {
                success: true,
                message: format!(
                    "User {} suspended successfully{}",
                    user.email,
                    request
                        .reason
                        .map(|r| format!(" (Reason: {r})"))
                        .unwrap_or_default()
                ),
                user: Some(user_to_info(user)),
                tenant_created: None,
            };
            Ok(with_status(json(&response), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to suspend user {}: {}", user_id, e);
            let response = UserManagementResponse {
                success: false,
                message: format!("Failed to suspend user: {e}"),
                user: None,
                tenant_created: None,
            };
            Ok(with_status(
                json(&response),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

/// Convert User to `UserInfo`
fn user_to_info(user: User) -> UserInfo {
    UserInfo {
        id: user.id.to_string(),
        email: user.email,
        display_name: user.display_name,
        user_status: match user.user_status {
            UserStatus::Pending => "pending".into(),
            UserStatus::Active => "active".into(),
            UserStatus::Suspended => "suspended".into(),
        },
        tier: format!("{:?}", user.tier).to_lowercase(),
        created_at: user.created_at.to_rfc3339(),
        last_active: user.last_active.to_rfc3339(),
        approved_by: user.approved_by.map(|id| id.to_string()),
        approved_at: user.approved_at.map(|dt| dt.to_rfc3339()),
    }
}

/// Get users by status
async fn get_users_by_status(database: &Database, status: UserStatus) -> Result<Vec<User>> {
    let status_str = match status {
        UserStatus::Pending => "pending",
        UserStatus::Active => "active",
        UserStatus::Suspended => "suspended",
    };

    let users = database.get_users_by_status(status_str).await?;
    Ok(users)
}

/// Approve user and optionally create default tenant in a single transaction
async fn approve_user_with_optional_tenant(
    database: &Database,
    user_id: Uuid,
    admin_token_id: &str,
    request: &ApproveUserRequest,
) -> Result<(User, Option<crate::models::Tenant>)> {
    info!(
        "Attempting to approve user: {} with admin token: {}",
        user_id, admin_token_id
    );

    // First approve the user
    let user = match database
        .update_user_status(user_id, UserStatus::Active, admin_token_id)
        .await
    {
        Ok(user) => {
            info!("Successfully approved user: {} ({})", user.email, user_id);
            user
        }
        Err(e) => {
            error!("Failed to approve user {}: {}", user_id, e);
            error!("Error details: {:?}", e);
            return Err(e);
        }
    };

    // Optionally create default tenant
    let tenant_info = if request.create_default_tenant.unwrap_or(false) {
        let tenant_name = request.tenant_name.clone().unwrap_or_else(|| {
            format!(
                "{}'s Organization",
                user.display_name.as_ref().unwrap_or(&user.email)
            )
        });
        let tenant_slug = request
            .tenant_slug
            .clone()
            .unwrap_or_else(|| format!("user-{}", user.id.simple()));

        match create_default_tenant_for_user(database, user_id, &tenant_name, &tenant_slug).await {
            Ok(tenant) => {
                info!(
                    "Created default tenant '{}' for user {}",
                    tenant.name, user.email
                );

                // Update user's tenant_id to link them to the new tenant
                if let Err(e) = database
                    .update_user_tenant_id(user_id, &tenant.id.to_string())
                    .await
                {
                    error!(
                        "Failed to link user {} to tenant {}: {}",
                        user.email, tenant.id, e
                    );
                    // This is critical - if we can't link the user to tenant, return error
                    return Err(operation_error(
                        "Link user to tenant",
                        &format!("Failed to link user to created tenant: {e}"),
                    ));
                }

                info!(
                    "Successfully linked user {} to tenant {} ({})",
                    user.email, tenant.slug, tenant.id
                );
                Some(tenant)
            }
            Err(e) => {
                warn!(
                    "Failed to create default tenant for user {}: {}. User approval succeeded.",
                    user.email, e
                );
                // Don't fail the entire approval if tenant creation fails
                None
            }
        }
    } else {
        None
    };

    Ok((user, tenant_info))
}

/// Create default tenant for a user
async fn create_default_tenant_for_user(
    database: &Database,
    owner_user_id: Uuid,
    tenant_name: &str,
    tenant_slug: &str,
) -> Result<crate::models::Tenant> {
    let tenant_id = Uuid::new_v4();
    let slug = tenant_slug.trim().to_lowercase();

    // Check if slug already exists
    if database.get_tenant_by_slug(&slug).await.is_ok() {
        return Err(validation_error(&format!(
            "Tenant slug '{slug}' already exists"
        )));
    }

    let tenant_data = crate::models::Tenant {
        id: tenant_id,
        name: tenant_name.to_string(),
        slug,
        domain: None,
        plan: tiers::STARTER.to_string(), // Default plan for auto-created tenants
        owner_user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&tenant_data).await?;
    Ok(tenant_data)
}

/// Suspend user and update status
async fn suspend_user_status(
    database: &Database,
    user_id: Uuid,
    admin_token_id: &str,
) -> Result<User> {
    let user = database
        .update_user_status(user_id, UserStatus::Suspended, admin_token_id)
        .await?;
    Ok(user)
}

// ARCHITECTURE IMPROVEMENT: The previous get_system_admin_user_id() function has been removed
// because it created an unnecessary dependency between admin-setup user creation and server operations.
// Instead, we now use the validated JWT token ID directly as the approval authority.
// This is more robust because:
// 1. JWT tokens are already validated and authorized
// 2. No database dependency or lookup required
// 3. Consistent with suspend_user_status implementation
// 4. Eliminates coupling between different code paths
