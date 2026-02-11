// ABOUTME: Admin API route handlers for administrative operations and API key management
// ABOUTME: Provides REST endpoints for admin services with proper authentication and authorization
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Admin routes for administrative operations
//!
//! This module handles admin-specific operations like API key provisioning,
//! user management, and administrative functions. All handlers are thin
//! wrappers that delegate business logic to service layers.

mod types;

pub use types::{
    AdminResponse, AdminSetupRequest, AdminSetupResponse, ApproveUserRequest, AutoApprovalResponse,
    CoachReviewQuery, DeleteUserRequest, ListApiKeysQuery, ListPendingCoachesQuery, ListUsersQuery,
    ProvisionApiKeyRequest, ProvisionApiKeyResponse, RateLimitInfo, RejectCoachRequest,
    RevokeKeyRequest, SuspendUserRequest, TenantCreatedInfo, UpdateAutoApprovalRequest,
    UserActivityQuery,
};

use std::sync::Arc;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::Serialize;
use serde_json::{from_slice, json, to_value, Value};
use tokio::task;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    admin::{
        auth::AdminAuthService,
        jwks::JwksManager,
        middleware::admin_auth_middleware,
        models::{
            AdminPermission, AdminTokenSummary, CreateAdminTokenRequest, ValidatedAdminToken,
        },
        AdminPermission as AdminPerm,
    },
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::{AuthManager, SetupStatusResponse},
    config::social::SocialInsightsConfig,
    constants::{
        tiers,
        time_constants::{SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MONTH, SECONDS_PER_WEEK},
    },
    database::CoachesManager,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, AppResult},
    mcp::ToolSelectionService,
    models::{Tenant, User, UserStatus},
    rate_limiting::UnifiedRateLimitCalculator,
    routes::tool_selection::{ToolSelectionContext, ToolSelectionRoutes},
};

// Helper function for JSON responses with status
fn json_response<T: Serialize>(value: T, status: StatusCode) -> impl IntoResponse {
    (status, Json(value))
}

/// User list response
#[derive(Debug, Clone, Serialize)]
struct UserListResponse {
    /// List of users (sanitized - no passwords)
    users: Vec<UserSummary>,
    /// Total number of users
    total: usize,
}

/// Sanitized user summary for listing
#[derive(Debug, Clone, Serialize)]
struct UserSummary {
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

/// Admin API context shared across all endpoints
#[derive(Clone)]
pub struct AdminApiContext {
    /// Database connection for persistence operations
    pub database: Arc<Database>,
    /// Admin authentication service
    pub auth_service: AdminAuthService,
    /// Authentication manager for token operations
    pub auth_manager: Arc<AuthManager>,
    /// JWT secret for admin token validation
    pub admin_jwt_secret: String,
    /// JWKS manager for key rotation and validation
    pub jwks_manager: Arc<JwksManager>,
    /// Default monthly request limit for admin-provisioned API keys
    pub admin_api_key_monthly_limit: u32,
    /// Tool selection service for managing per-tenant MCP tool availability
    pub tool_selection: Arc<ToolSelectionService>,
}

impl AdminApiContext {
    /// Creates a new admin API context
    pub fn new(
        database: Arc<Database>,
        jwt_secret: &str,
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<JwksManager>,
        admin_api_key_monthly_limit: u32,
        admin_token_cache_ttl_secs: u64,
        tool_selection: Arc<ToolSelectionService>,
    ) -> Self {
        info!("AdminApiContext initialized with JWT signing key");
        let auth_service = AdminAuthService::new(
            (*database).clone(),
            jwks_manager.clone(),
            admin_token_cache_ttl_secs,
        );
        Self {
            database,
            auth_service,
            auth_manager,
            admin_jwt_secret: jwt_secret.to_owned(),
            jwks_manager,
            admin_api_key_monthly_limit,
            tool_selection,
        }
    }
}

/// Helper functions for admin operations
/// Convert rate limit period string to window duration in seconds
fn convert_rate_limit_period(period: &str) -> AppResult<u32> {
    match period.to_lowercase().as_str() {
        "hour" => Ok(SECONDS_PER_HOUR),   // 1 hour
        "day" => Ok(SECONDS_PER_DAY),     // 24 hours
        "week" => Ok(SECONDS_PER_WEEK),   // 7 days
        "month" => Ok(SECONDS_PER_MONTH), // 30 days
        _ => Err(AppError::invalid_input(
            "Invalid rate limit period. Supported: hour, day, week, month",
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

/// Get existing user for API key provisioning (no automatic creation)
async fn get_existing_user(database: &Database, email: &str) -> AppResult<User> {
    match database.get_user_by_email(email).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => {
            warn!("API key provisioning failed: user does not exist");
            Err(AppError::invalid_input(
                "User must register and be approved before API key provisioning",
            ))
        }
        Err(e) => Err(AppError::internal(format!("Failed to lookup user: {e}"))),
    }
}

/// Create and store API key
#[tracing::instrument(skip(context, user, request, admin_token), fields(route = "provision_api_key", user_id = %user.id))]
async fn create_and_store_api_key(
    context: &AdminApiContext,
    user: &User,
    request: &ProvisionApiKeyRequest,
    tier: &ApiKeyTier,
    admin_token: &ValidatedAdminToken,
) -> Result<(ApiKey, String), String> {
    // Generate API key using ApiKeyManager
    let api_key_manager = ApiKeyManager::new();
    let create_request = CreateApiKeyRequest {
        name: request
            .description
            .clone() // Safe: Option<String> ownership for struct field
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
    body: &[u8],
) -> Result<ProvisionApiKeyRequest, (StatusCode, Json<AdminResponse>)> {
    match from_slice(body) {
        Ok(req) => Ok(req),
        Err(e) => {
            warn!(error = %e, "Invalid JSON body in provision API key request");
            Err((
                StatusCode::BAD_REQUEST,
                Json(AdminResponse {
                    success: false,
                    message: format!("Invalid JSON body: {e}"),
                    data: None,
                }),
            ))
        }
    }
}

/// Check if admin token has provision permission
fn check_provision_permission(
    admin_token: &ValidatedAdminToken,
) -> Result<(), (StatusCode, Json<AdminResponse>)> {
    if admin_token
        .permissions
        .has_permission(&AdminPerm::ProvisionKeys)
    {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(AdminResponse {
                success: false,
                message: "Permission denied: ProvisionKeys required".to_owned(),
                data: None,
            }),
        ))
    }
}

/// Validate tier string and return appropriate response on error
fn validate_tier_or_respond(
    tier_str: &str,
) -> Result<ApiKeyTier, (StatusCode, Json<AdminResponse>)> {
    validate_tier(tier_str).map_err(|error_msg| {
        (
            StatusCode::BAD_REQUEST,
            Json(AdminResponse {
                success: false,
                message: error_msg,
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
    get_existing_user(database, email).await.map_err(|_e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AdminResponse {
                success: false,
                message: format!("Failed to lookup user: {email}"),
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

    // Hash password
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

    // Create admin user struct
    let mut admin_user = User::new(
        request.email.clone(),
        password_hash,
        request.display_name.clone(),
    );
    admin_user.id = user_id;
    admin_user.is_admin = true;
    admin_user.user_status = UserStatus::Active;

    // Persist to database
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

/// Admin routes implementation (Axum)
///
/// Provides administrative endpoints for user management, API keys, JWKS, and server administration.
pub struct AdminRoutes;

impl AdminRoutes {
    /// Create all admin routes (Axum)
    pub fn routes(context: AdminApiContext) -> Router {
        // Reuse auth service from context (already configured with proper TTL)
        let auth_service = context.auth_service.clone();
        let tool_selection_context = ToolSelectionContext {
            tool_selection: context.tool_selection.clone(),
        };
        let context = Arc::new(context);

        // Protected routes require admin authentication
        let api_key_routes = Self::api_key_routes(context.clone()).layer(
            middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware),
        );

        let user_routes = Self::user_routes(context.clone()).layer(middleware::from_fn_with_state(
            auth_service.clone(),
            admin_auth_middleware,
        ));

        let settings_routes = Self::settings_routes(context.clone()).layer(
            middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware),
        );

        let admin_token_routes = Self::admin_token_routes(context.clone()).layer(
            middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware),
        );

        // Tool selection routes for per-tenant MCP tool configuration
        let tool_selection_routes = ToolSelectionRoutes::routes(tool_selection_context).layer(
            middleware::from_fn_with_state(auth_service.clone(), admin_auth_middleware),
        );

        // Store review routes for admin coach review queue
        let store_review_routes = Self::store_review_routes(context.clone()).layer(
            middleware::from_fn_with_state(auth_service, admin_auth_middleware),
        );

        // Setup routes are public (no auth required for initial setup)
        let setup_routes = Self::setup_routes(context);

        Router::new()
            .merge(api_key_routes)
            .merge(user_routes)
            .merge(settings_routes)
            .merge(admin_token_routes)
            .merge(tool_selection_routes)
            .merge(store_review_routes)
            .merge(setup_routes)
    }

    /// API key management routes (Axum)
    fn api_key_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/provision", post(Self::handle_provision_api_key))
            .route("/admin/revoke", post(Self::handle_revoke_api_key))
            .route("/admin/list", get(Self::handle_list_api_keys))
            .route("/admin/token-info", get(Self::handle_token_info))
            .with_state(context)
    }

    /// User management routes (Axum)
    fn user_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/users", get(Self::handle_list_users))
            .route("/admin/pending-users", get(Self::handle_pending_users))
            .route(
                "/admin/approve-user/:user_id",
                post(Self::handle_approve_user),
            )
            .route(
                "/admin/suspend-user/:user_id",
                post(Self::handle_suspend_user),
            )
            .route(
                "/admin/users/:user_id/reset-password",
                post(Self::handle_reset_user_password),
            )
            .route(
                "/admin/users/:user_id/rate-limit",
                get(Self::handle_get_user_rate_limit),
            )
            .route(
                "/admin/users/:user_id/activity",
                get(Self::handle_get_user_activity),
            )
            .route("/admin/users/:user_id", delete(Self::handle_delete_user))
            .with_state(context)
    }

    /// System settings routes (Axum)
    fn settings_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route(
                "/admin/settings/auto-approval",
                get(Self::handle_get_auto_approval),
            )
            .route(
                "/admin/settings/auto-approval",
                put(Self::handle_set_auto_approval),
            )
            .route(
                "/admin/settings/social-insights",
                get(Self::handle_get_social_insights_config),
            )
            .route(
                "/admin/settings/social-insights",
                put(Self::handle_set_social_insights_config),
            )
            .route(
                "/admin/settings/social-insights",
                delete(Self::handle_reset_social_insights_config),
            )
            .with_state(context)
    }

    /// Setup routes (Axum)
    fn setup_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/setup", post(Self::handle_admin_setup))
            .route("/admin/setup/status", get(Self::handle_setup_status))
            .route("/admin/health", get(Self::handle_health))
            .with_state(context)
    }

    /// Admin token management routes (Axum)
    fn admin_token_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/tokens", post(Self::handle_create_admin_token))
            .route("/admin/tokens", get(Self::handle_list_admin_tokens))
            .route("/admin/tokens/:token_id", get(Self::handle_get_admin_token))
            .route(
                "/admin/tokens/:token_id/revoke",
                post(Self::handle_revoke_admin_token),
            )
            .route(
                "/admin/tokens/:token_id/rotate",
                post(Self::handle_rotate_admin_token),
            )
            .with_state(context)
    }

    /// Store review queue routes for admin coach approval (Axum)
    fn store_review_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route(
                "/admin/store/pending",
                get(Self::handle_list_pending_coaches),
            )
            .route(
                "/admin/store/coaches/:coach_id/approve",
                post(Self::handle_approve_coach),
            )
            .route(
                "/admin/store/coaches/:coach_id/reject",
                post(Self::handle_reject_coach),
            )
            .with_state(context)
    }

    /// Handle API key provisioning (Axum)
    async fn handle_provision_api_key(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        body: Bytes,
    ) -> AppResult<impl IntoResponse> {
        // Parse and validate request
        let request = match parse_provision_request(&body) {
            Ok(req) => req,
            Err(response) => return Ok(response),
        };

        // Check required permission
        if let Err(response) = check_provision_permission(&admin_token) {
            return Ok(response);
        }

        info!(
            "Provisioning API key for user: {} by service: {}",
            request.user_email, admin_token.service_name
        );

        let ctx = context.as_ref();

        // Validate tier
        let tier = match validate_tier_or_respond(&request.tier) {
            Ok(t) => t,
            Err(response) => return Ok(response),
        };

        // Get existing user (no automatic creation)
        let user = match get_user_or_respond(&ctx.database, &request.user_email).await {
            Ok(u) => u,
            Err(response) => return Ok(response),
        };

        // Create and store API key
        let (final_api_key, api_key_string) =
            match create_and_store_api_key(ctx, &user, &request, &tier, &admin_token).await {
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

        // Record the provisioning action for audit
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

        // Wrap in AdminResponse for consistency
        Ok((
            StatusCode::CREATED,
            Json(AdminResponse {
                success: true,
                message: format!("API key provisioned successfully for {}", user.email),
                data: to_value(&provision_response).ok(),
            }),
        ))
    }

    /// Handle API key revocation (Axum)
    async fn handle_revoke_api_key(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Json(request): Json<RevokeKeyRequest>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission
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

        // Get the API key to find the user_id
        // Admin cross-user access: pass None for user_id
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

    /// Handle API key listing (Axum)
    async fn handle_list_api_keys(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Query(params): Query<ListApiKeysQuery>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission
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

        // Parse query parameters
        let user_email = params.user_email.as_deref();
        let active_only = params.active_only.unwrap_or(true);
        let limit = params
            .limit
            .as_ref()
            .and_then(|s| s.parse::<i32>().ok())
            .map(|l| l.clamp(1, 100)); // Limit between 1-100
        let offset = params
            .offset
            .as_ref()
            .and_then(|s| s.parse::<i32>().ok())
            .map(|o| o.max(0)); // Ensure non-negative

        // Get API keys from database
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

    /// Handle user listing (Axum)
    async fn handle_list_users(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Query(params): Query<ListUsersQuery>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission
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

        // Determine status filter - default to "active"
        let status = params.status.as_deref().unwrap_or("active");

        // Fetch users from database by status
        let users = ctx
            .database
            .get_users_by_status(status, None)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch users from database");
                AppError::internal(format!("Failed to fetch users: {e}"))
            })?;

        // Convert to sanitized summaries (no password hashes!)
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

    /// Handle pending users listing (Axum)
    async fn handle_pending_users(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission
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

        // Fetch users with Pending status
        let users = ctx
            .database
            .get_users_by_status("pending", None)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to fetch pending users from database");
                AppError::internal(format!("Failed to fetch pending users: {e}"))
            })?;

        // Convert to sanitized summaries
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

    /// Get user status string
    const fn user_status_str(status: UserStatus) -> &'static str {
        match status {
            UserStatus::Pending => "pending",
            UserStatus::Active => "active",
            UserStatus::Suspended => "suspended",
        }
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

        let tenant =
            Self::create_default_tenant_for_user(database, user_uuid, &tenant_name, &tenant_slug)
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

        let tenant_id_str = tenant.id.to_string();
        database
            .update_user_tenant_id(user_uuid, &tenant_id_str)
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

    /// Handle user approval workflow
    #[allow(clippy::too_many_lines)]
    async fn handle_approve_user(
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

        // Service tokens don't have an associated user UUID, so approved_by is None
        // The audit trail is maintained via admin_token.token_id in logs
        let updated_user = ctx
            .database
            .update_user_status(user_uuid, UserStatus::Active, None)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to update user status in database");
                AppError::internal(format!("Failed to approve user: {e}"))
            })?;

        let tenant_created = Self::create_and_link_tenant(
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
                        "user_status": Self::user_status_str(updated_user.user_status),
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
    async fn handle_suspend_user(
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

        // Service tokens don't have an associated user UUID, so approved_by is None
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
                        "user_status": Self::user_status_str(updated_user.user_status),
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
    async fn handle_delete_user(
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

        // Fetch user to confirm existence and get email for logging
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

        // Delete user (cascades to related tables via foreign keys)
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
    async fn handle_reset_user_password(
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

        // Verify user exists
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

        // Generate a cryptographically random reset token (32 bytes, base64url-encoded)
        let raw_token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(48)
            .map(char::from)
            .collect();

        // Store only the SHA-256 hash of the token in the database
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
    async fn handle_get_user_rate_limit(
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

        // Get user
        let user = ctx
            .database
            .get_user(user_uuid)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // Get current monthly usage
        let monthly_used = ctx
            .database
            .get_jwt_current_usage(user_uuid)
            .await
            .unwrap_or(0);

        // Get daily usage from activity logs (today's requests)
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
                // Safe: daily usage won't exceed u32::MAX
                tools.iter().map(|t| t.request_count as u32).sum::<u32>()
            })
            .unwrap_or(0);

        // Calculate limits based on tier
        let monthly_limit = user.tier.monthly_limit();
        let daily_limit = monthly_limit.map(|m| m / 30); // Approximate daily limit

        // Calculate remaining
        let monthly_remaining = monthly_limit.map(|l| l.saturating_sub(monthly_used));
        let daily_remaining = daily_limit.map(|l| l.saturating_sub(daily_used));

        // Calculate reset times
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
    async fn handle_get_user_activity(
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

        // Verify user exists
        ctx.database
            .get_user(user_uuid)
            .await
            .map_err(|e| AppError::internal(format!("Failed to fetch user: {e}")))?
            .ok_or_else(|| AppError::not_found("User not found"))?;

        // Get time range for activity using days parameter (default 30)
        let days = i64::from(params.days.unwrap_or(30).clamp(1, 365));
        let now = Utc::now();
        let start_time = now - Duration::days(days);

        // Get top tools usage
        let top_tools_raw = ctx
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

    /// Handle getting auto-approval setting
    async fn handle_get_auto_approval(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission
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

        // Get database setting (None = no explicit database override, use config default)
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
    async fn handle_set_auto_approval(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Json(request): Json<UpdateAutoApprovalRequest>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission
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
    async fn handle_get_social_insights_config(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> AppResult<impl IntoResponse> {
        // ManageUsers permission allows reading/writing system settings
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

        // Get database setting or use defaults
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
    async fn handle_set_social_insights_config(
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
    async fn handle_reset_social_insights_config(
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

    /// Handle admin token creation (Axum)
    async fn handle_create_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Json(request): Json<serde_json::Value>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission - token management requires ManageAdminTokens
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

        // Parse request fields
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

        // Prevent privilege escalation: only super-admin tokens can mint super-admin tokens
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

        // Parse permissions if provided
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

        // Create token request
        let token_request = CreateAdminTokenRequest {
            service_name,
            service_description,
            permissions,
            expires_in_days,
            is_super_admin,
        };

        // Generate token using database method
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

    /// Handle listing admin tokens (Axum)
    async fn handle_list_admin_tokens(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission - token management requires ManageAdminTokens
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

        // Redact sensitive hash fields before returning
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

    /// Handle getting admin token details (Axum)
    async fn handle_get_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(token_id): Path<String>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission - token management requires ManageAdminTokens
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

        // Redact sensitive hash fields before returning
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

    /// Handle revoking admin token (Axum)
    async fn handle_revoke_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(token_id): Path<String>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission - token management requires ManageAdminTokens
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

        // Deactivate the token
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

    /// Handle rotating admin token (Axum)
    async fn handle_rotate_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(token_id): Path<String>,
    ) -> AppResult<impl IntoResponse> {
        // Check required permission - token management requires ManageAdminTokens
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

        // Get existing token to copy its properties
        let existing_token = ctx
            .database
            .get_admin_token_by_id(&token_id)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to get admin token");
                AppError::internal(format!("Failed to get admin token: {e}"))
            })?
            .ok_or_else(|| AppError::not_found("Admin token not found"))?;

        // Deactivate old token
        ctx.database
            .deactivate_admin_token(&token_id)
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to deactivate old token");
                AppError::internal(format!("Failed to deactivate old token: {e}"))
            })?;

        // Generate new token with same properties
        let token_request = CreateAdminTokenRequest {
            service_name: existing_token.service_name.clone(),
            service_description: existing_token.service_description.clone(),
            permissions: None, // Will use existing token's permissions
            is_super_admin: existing_token.is_super_admin,
            expires_in_days: Some(365_u64), // Default 1 year expiry
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

    /// Handle admin setup (Axum)
    async fn handle_admin_setup(
        State(context): State<Arc<AdminApiContext>>,
        Json(request): Json<AdminSetupRequest>,
    ) -> AppResult<impl IntoResponse> {
        info!("Admin setup request for email: {}", request.email);

        let ctx = context.as_ref();

        // Check if any admin users already exist
        if let Some(error_response) = check_no_admin_exists(&ctx.database)
            .await
            .map_err(|e| AppError::database(format!("Failed to check for existing admin: {e}")))?
        {
            return Ok(error_response);
        }

        // Create admin user
        let user_id = match create_admin_user_record(&ctx.database, &request).await {
            Ok(id) => id,
            Err(error_response) => return Ok(error_response),
        };

        // Generate admin token
        let admin_token = match generate_initial_admin_token(
            &ctx.database,
            &ctx.admin_jwt_secret,
            &ctx.jwks_manager,
        )
        .await
        {
            Ok(token) => token,
            Err(error_response) => return Ok(error_response),
        };

        // Return success response
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
    async fn handle_setup_status(
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
                use SetupStatusResponse;

                error!("Failed to check setup status: {}", e);
                Ok(json_response(
                    SetupStatusResponse {
                        needs_setup: true,
                        admin_user_exists: false,
                        message: Some("Unable to determine setup status. Please ensure admin user is created.".to_owned()),
                    },
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }

    /// Handle health check (GET /admin/health)
    async fn handle_health() -> Json<serde_json::Value> {
        // Use spawn_blocking for JSON serialization (CPU-bound operation)
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

    /// Handle token info (GET /admin/token-info)
    /// Returns information about the authenticated admin token
    async fn handle_token_info(
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> Json<serde_json::Value> {
        // Clone values before spawn_blocking
        let token_id = admin_token.token_id;
        let service_name = admin_token.service_name.clone();
        let permissions = admin_token.permissions.clone();
        let is_super_admin = admin_token.is_super_admin;

        // Use spawn_blocking for JSON serialization (CPU-bound operation)
        let token_info_json = task::spawn_blocking(move || {
            // Convert permissions to JSON array
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
        // Reserved slugs that cannot be used for tenants
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

        let tenant_id = Uuid::new_v4();
        let slug = tenant_slug.trim().to_lowercase();

        // Validate slug format
        if slug.is_empty() {
            return Err(AppError::invalid_input("Tenant slug cannot be empty"));
        }

        if slug.len() > 63 {
            return Err(AppError::invalid_input(
                "Tenant slug must be 63 characters or less",
            ));
        }

        // Check for valid characters (alphanumeric and hyphens only)
        if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Err(AppError::invalid_input(
                "Tenant slug can only contain letters, numbers, and hyphens",
            ));
        }

        // Check for leading/trailing hyphens
        if slug.starts_with('-') || slug.ends_with('-') {
            return Err(AppError::invalid_input(
                "Tenant slug cannot start or end with a hyphen",
            ));
        }

        // Check against reserved slugs
        if RESERVED_SLUGS.contains(&slug.as_str()) {
            return Err(AppError::invalid_input(format!(
                "Tenant slug '{slug}' is reserved and cannot be used",
            )));
        }

        // Check if slug already exists
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
            plan: tiers::STARTER.to_owned(), // Default plan for auto-created tenants
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

    // ==========================================
    // Store Review Handlers
    // ==========================================

    /// List coaches pending admin review
    async fn handle_list_pending_coaches(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Query(query): Query<ListPendingCoachesQuery>,
    ) -> AppResult<impl IntoResponse> {
        // Check permission - super admin or ManageUsers permission required
        if !admin_token.is_super_admin
            && !admin_token
                .permissions
                .has_permission(&AdminPermission::ManageUsers)
        {
            return Ok(json_response(
                json!({"error": "Insufficient permissions to view review queue"}),
                StatusCode::FORBIDDEN,
            ));
        }

        let pool = context.database.sqlite_pool().ok_or_else(|| {
            AppError::internal("SQLite database required for coach store operations")
        })?;
        let coaches_manager = CoachesManager::new(pool.clone());

        let coaches = coaches_manager
            .get_pending_review_coaches(&query.tenant_id, query.limit, query.offset)
            .await?;

        info!(
            "Admin {} listed {} pending coaches for review in tenant {}",
            admin_token.service_name,
            coaches.len(),
            query.tenant_id
        );

        Ok(json_response(
            json!({
                "coaches": coaches,
                "count": coaches.len()
            }),
            StatusCode::OK,
        ))
    }

    /// Approve a coach for publishing to the Store
    async fn handle_approve_coach(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(coach_id): Path<String>,
        Query(query): Query<CoachReviewQuery>,
    ) -> AppResult<impl IntoResponse> {
        // Check permission - super admin or ManageUsers permission required
        if !admin_token.is_super_admin
            && !admin_token
                .permissions
                .has_permission(&AdminPermission::ManageUsers)
        {
            return Ok(json_response(
                json!({"error": "Insufficient permissions to approve coaches"}),
                StatusCode::FORBIDDEN,
            ));
        }

        let pool = context.database.sqlite_pool().ok_or_else(|| {
            AppError::internal("SQLite database required for coach store operations")
        })?;
        let coaches_manager = CoachesManager::new(pool.clone());

        // Service-based admin tokens don't have user IDs, so pass None
        // The review_decision_by field will be NULL for admin-initiated approvals
        let coach = coaches_manager
            .approve_coach(&coach_id, &query.tenant_id, None::<Uuid>)
            .await?;

        info!(
            "Admin {} approved coach {} for Store in tenant {}",
            admin_token.service_name, coach_id, query.tenant_id
        );

        Ok(json_response(
            json!({
                "message": "Coach approved and published to Store",
                "coach": coach
            }),
            StatusCode::OK,
        ))
    }

    /// Reject a coach with a reason
    async fn handle_reject_coach(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Path(coach_id): Path<String>,
        Query(query): Query<CoachReviewQuery>,
        Json(request): Json<RejectCoachRequest>,
    ) -> AppResult<impl IntoResponse> {
        // Check permission - super admin or ManageUsers permission required
        if !admin_token.is_super_admin
            && !admin_token
                .permissions
                .has_permission(&AdminPermission::ManageUsers)
        {
            return Ok(json_response(
                json!({"error": "Insufficient permissions to reject coaches"}),
                StatusCode::FORBIDDEN,
            ));
        }

        if request.reason.trim().is_empty() {
            return Ok(json_response(
                json!({"error": "Rejection reason is required"}),
                StatusCode::BAD_REQUEST,
            ));
        }

        let pool = context.database.sqlite_pool().ok_or_else(|| {
            AppError::internal("SQLite database required for coach store operations")
        })?;
        let coaches_manager = CoachesManager::new(pool.clone());

        // Service-based admin tokens don't have user IDs, so pass None
        // The review_decision_by field will be NULL for admin-initiated rejections
        let coach = coaches_manager
            .reject_coach(&coach_id, &query.tenant_id, None::<Uuid>, &request.reason)
            .await?;

        info!(
            "Admin {} rejected coach {} in tenant {} with reason: {}",
            admin_token.service_name, coach_id, query.tenant_id, request.reason
        );

        Ok(json_response(
            json!({
                "message": "Coach rejected",
                "coach": coach
            }),
            StatusCode::OK,
        ))
    }
}
