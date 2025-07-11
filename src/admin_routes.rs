// ABOUTME: HTTP route handlers for admin API endpoints and administrative operations
// ABOUTME: Provides REST endpoints for API key provisioning, user management, and admin functions
//! Admin API Routes
//!
//! This module provides REST API endpoints for admin services to manage API keys
//! and perform administrative operations on the Pierre MCP Server.

use crate::{
    admin::{auth::AdminAuthService, models::AdminPermission},
    api_keys::{ApiKey, ApiKeyTier},
    auth::AuthManager,
    constants::time_constants::{
        SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MONTH, SECONDS_PER_WEEK,
    },
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
};
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;
use warp::{
    http::StatusCode,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

/// Admin API context shared across all endpoints
#[derive(Clone)]
pub struct AdminApiContext {
    pub database: Database,
    pub auth_service: AdminAuthService,
    pub auth_manager: AuthManager,
}

impl AdminApiContext {
    pub fn new(database: Database, jwt_secret: &str, auth_manager: AuthManager) -> Self {
        let auth_service = AdminAuthService::new(database.clone(), jwt_secret);
        Self {
            database,
            auth_service,
            auth_manager,
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

/// Create admin routes filter
pub fn admin_routes(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = std::convert::Infallible> + Clone {
    let provision_route = provision_api_key_route(context.clone());
    let revoke_route = revoke_api_key_route(context.clone());
    let list_keys_route = list_api_keys_route(context.clone());
    let token_info_route = token_info_route(context.clone());
    let setup_status_route = setup_status_route(context.clone());

    // Admin token management routes
    let admin_tokens_list_route = admin_tokens_list_route(context.clone());
    let admin_tokens_create_route = admin_tokens_create_route(context.clone());
    let admin_tokens_details_route = admin_tokens_details_route(context.clone());
    let admin_tokens_revoke_route = admin_tokens_revoke_route(context.clone());
    let admin_tokens_rotate_route = admin_tokens_rotate_route(context);

    let health_route = admin_health_route();

    let admin_routes = provision_route
        .or(revoke_route)
        .or(list_keys_route)
        .or(token_info_route)
        .or(setup_status_route)
        .or(admin_tokens_list_route)
        .or(admin_tokens_create_route)
        .or(admin_tokens_details_route)
        .or(admin_tokens_revoke_route)
        .or(admin_tokens_rotate_route)
        .or(health_route);

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
    let setup_status_route = setup_status_route(context.clone());

    // Admin token management routes
    let admin_tokens_list_route = admin_tokens_list_route(context.clone());
    let admin_tokens_create_route = admin_tokens_create_route(context.clone());
    let admin_tokens_details_route = admin_tokens_details_route(context.clone());
    let admin_tokens_revoke_route = admin_tokens_revoke_route(context.clone());
    let admin_tokens_rotate_route = admin_tokens_rotate_route(context);

    let health_route = admin_health_route();

    let admin_routes = provision_route
        .or(revoke_route)
        .or(list_keys_route)
        .or(token_info_route)
        .or(setup_status_route)
        .or(admin_tokens_list_route)
        .or(admin_tokens_create_route)
        .or(admin_tokens_details_route)
        .or(admin_tokens_revoke_route)
        .or(admin_tokens_rotate_route)
        .or(health_route);

    warp::path("admin")
        .and(admin_routes)
        .recover(handle_admin_rejection)
}

/// Create admin routes filter without recovery (maintains Rejection error type)
/// This is used for embedding in other servers that handle rejections globally
pub fn admin_routes_with_rejection(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let provision_route = provision_api_key_route(context.clone());
    let revoke_route = revoke_api_key_route(context.clone());
    let list_keys_route = list_api_keys_route(context.clone());
    let token_info_route = token_info_route(context.clone());
    let setup_status_route = setup_status_route(context.clone());

    // Admin token management routes
    let admin_tokens_list_route = admin_tokens_list_route(context.clone());
    let admin_tokens_create_route = admin_tokens_create_route(context.clone());
    let admin_tokens_details_route = admin_tokens_details_route(context.clone());
    let admin_tokens_revoke_route = admin_tokens_revoke_route(context.clone());
    let admin_tokens_rotate_route = admin_tokens_rotate_route(context);

    let health_route = admin_health_route();

    let admin_routes = provision_route
        .or(revoke_route)
        .or(list_keys_route)
        .or(token_info_route)
        .or(setup_status_route)
        .or(admin_tokens_list_route)
        .or(admin_tokens_create_route)
        .or(admin_tokens_details_route)
        .or(admin_tokens_revoke_route)
        .or(admin_tokens_rotate_route)
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

/// Admin health check endpoint
fn admin_health_route() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("health").and(warp::get()).map(|| {
        json(&serde_json::json!({
            "status": "healthy",
            "service": "pierre-mcp-admin-api",
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
                    let token = extract_bearer_token(&auth_header)
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

/// Extract Bearer token from Authorization header
///
/// # Errors
///
/// Returns an error if:
/// - Authorization header format is invalid
/// - Bearer token is empty or missing
pub fn extract_bearer_token(auth_header: &str) -> Result<String> {
    if !auth_header.starts_with("Bearer ") {
        return Err(anyhow!("Invalid authorization header format"));
    }

    let token = auth_header
        .strip_prefix("Bearer ")
        .context("Failed to extract bearer token")?
        .trim();
    if token.is_empty() {
        return Err(anyhow!("Empty bearer token"));
    }

    Ok(token.to_string())
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
        "trial" => Ok(ApiKeyTier::Trial),
        "starter" => Ok(ApiKeyTier::Starter),
        "professional" => Ok(ApiKeyTier::Professional),
        "enterprise" => Ok(ApiKeyTier::Enterprise),
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

/// Get or create user for API key provisioning
async fn get_or_create_user(database: &Database, email: &str) -> Result<User, warp::Rejection> {
    match database.get_user_by_email(email).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => {
            // Create new user for API key
            let new_user = User {
                id: Uuid::new_v4(),
                email: email.to_string(),
                display_name: Some(format!("API User ({email})")),
                password_hash: "api-key-only".into(), // API-only user
                tier: crate::models::UserTier::Starter, // Default tier for API users
                strava_token: None,
                fitbit_token: None,
                is_active: true,
                created_at: chrono::Utc::now(),
                last_active: chrono::Utc::now(),
            };

            let user_id = database.create_user(&new_user).await.map_err(|e| {
                warp::reject::custom(AdminApiError::DatabaseError(format!(
                    "Failed to create user: {e}"
                )))
            })?;

            Ok(User {
                id: user_id,
                ..new_user
            })
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

    // Get or create user
    let Ok(user) = get_or_create_user(&context.database, &request.user_email).await else {
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
        "✅ API key provisioned successfully: {} for user: {}",
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
            info!("✅ API key revoked successfully: {}", request.api_key_id);

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

/// Admin API error types
#[derive(Debug)]
pub enum AdminApiError {
    InvalidAuthHeader,
    AuthenticationFailed(String),
    DatabaseError(String),
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

/// Setup status endpoint - check if admin user exists (no authentication required)
fn setup_status_route(
    context: AdminApiContext,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    warp::path!("setup-status")
        .and(warp::get())
        .and(warp::any().map(move || context.clone()))
        .and_then(handle_setup_status)
}

/// Handle setup status check
async fn handle_setup_status(context: AdminApiContext) -> Result<impl Reply, Rejection> {
    info!("🔍 Checking setup status - admin user existence");

    match context
        .auth_manager
        .check_setup_status(&context.database)
        .await
    {
        Ok(status) => {
            info!(
                "Setup status check complete - needs_setup: {}, admin_exists: {}",
                status.needs_setup, status.admin_user_exists
            );
            Ok(with_status(json(&status), StatusCode::OK))
        }
        Err(e) => {
            warn!("Failed to check setup status: {}", e);
            let error_status = crate::routes::SetupStatusResponse {
                needs_setup: true,
                admin_user_exists: false,
                message: Some("Error checking setup status. Please contact administrator.".into()),
            };
            Ok(with_status(
                json(&error_status),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
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

    match context.database.create_admin_token(&token_request).await {
        Ok(generated_token) => {
            info!(
                "✅ Admin token created successfully: {}",
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
    info!("🔍 Getting admin token details: {}", token_id);

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
            info!("✅ Admin token revoked successfully: {}", token_id);
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
    info!("🔄 Rotating admin token: {}", token_id);

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
        .create_admin_token(&new_token_request)
        .await
    {
        Ok(new_token) => {
            // Revoke old token
            if let Err(e) = context.database.deactivate_admin_token(&token_id).await {
                warn!("Failed to revoke old token during rotation: {}", e);
                // Continue anyway since new token was created
            }

            info!(
                "✅ Admin token rotated successfully: {} -> {}",
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
