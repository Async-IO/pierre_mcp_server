// ABOUTME: Admin API route handlers for administrative operations and API key management
// ABOUTME: Provides REST endpoints for admin services with proper authentication and authorization
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Admin routes for administrative operations
//!
//! This module handles admin-specific operations like API key provisioning,
//! user management, and administrative functions. All handlers are thin
//! wrappers that delegate business logic to service layers.

use crate::{
    admin::{auth::AdminAuthService, models::ValidatedAdminToken},
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::AuthManager,
    constants::{
        tiers,
        time_constants::{SECONDS_PER_DAY, SECONDS_PER_HOUR, SECONDS_PER_MONTH, SECONDS_PER_WEEK},
    },
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppError,
    models::{User, UserStatus},
};
use anyhow::Result;
use axum::{
    extract::{Query, State},
    Extension, Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

// Helper function for JSON responses with status
fn json_response<T: serde::Serialize>(
    value: T,
    status: axum::http::StatusCode,
) -> impl axum::response::IntoResponse {
    (status, Json(value))
}

/// API key provisioning request
#[derive(Debug, Deserialize)]
pub struct ProvisionApiKeyRequest {
    /// Email of the user to provision the key for
    pub user_email: String,
    /// Tier level for the API key (starter/professional/enterprise)
    pub tier: String,
    /// Optional description of the API key's purpose
    pub description: Option<String>,
    /// Number of days until the key expires
    pub expires_in_days: Option<u32>,
    /// Maximum requests allowed
    pub rate_limit_requests: Option<u32>,
    /// Rate limit period (e.g., "hour", "day", "month")
    pub rate_limit_period: Option<String>,
}

/// API key revocation request
#[derive(Debug, Deserialize)]
pub struct RevokeKeyRequest {
    /// ID of the API key to revoke
    pub api_key_id: String,
    /// Optional reason for revoking the key
    pub reason: Option<String>,
}

/// Admin setup request
#[derive(Debug, Deserialize)]
pub struct AdminSetupRequest {
    /// Admin email address
    pub email: String,
    /// Admin password
    pub password: String,
    /// Optional display name for the admin
    pub display_name: Option<String>,
}

/// Query parameters for listing API keys
#[derive(Debug, Deserialize)]
pub struct ListApiKeysQuery {
    /// Filter by user email
    pub user_email: Option<String>,
    /// Show only active keys
    pub active_only: Option<bool>,
    /// Maximum number of results (use String to allow invalid values that will be ignored)
    pub limit: Option<String>,
    /// Offset for pagination (use String to allow invalid values that will be ignored)
    pub offset: Option<String>,
}

/// Query parameters for listing users
#[derive(Debug, Deserialize)]
pub struct ListUsersQuery {
    /// Filter by status
    pub status: Option<String>,
    /// Maximum number of results
    pub limit: Option<i32>,
    /// Offset for pagination
    pub offset: Option<i32>,
}

/// API Key provisioning response
#[derive(Debug, Clone, Serialize)]
pub struct ProvisionApiKeyResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Unique identifier for the API key
    pub api_key_id: String,
    /// The actual API key (shown only once)
    pub api_key: String,
    /// ID of the user who owns this key
    pub user_id: String,
    /// Tier level of the key
    pub tier: String,
    /// When the key expires (ISO 8601 format)
    pub expires_at: Option<String>,
    /// Rate limit configuration
    pub rate_limit: Option<RateLimitInfo>,
}

/// Rate limit information
#[derive(Debug, Clone, Serialize)]
pub struct RateLimitInfo {
    /// Maximum number of requests allowed
    pub requests: u32,
    /// Time period for the rate limit
    pub period: String,
}

/// Generic admin response
#[derive(Debug, Clone, Serialize)]
pub struct AdminResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Response message
    pub message: String,
    /// Optional additional data
    pub data: Option<serde_json::Value>,
}

/// Admin setup response
#[derive(Debug, Clone, Serialize)]
pub struct AdminSetupResponse {
    /// ID of the created admin user
    pub user_id: String,
    /// JWT token for admin authentication
    pub admin_token: String,
    /// Success message
    pub message: String,
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
    pub jwks_manager: Arc<crate::admin::jwks::JwksManager>,
    /// Default monthly request limit for admin-provisioned API keys
    pub admin_api_key_monthly_limit: u32,
}

impl AdminApiContext {
    /// Creates a new admin API context
    pub fn new(
        database: Arc<Database>,
        jwt_secret: &str,
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<crate::admin::jwks::JwksManager>,
        admin_api_key_monthly_limit: u32,
    ) -> Self {
        tracing::info!(
            "Creating AdminApiContext with JWT secret (first 10 chars): {}...",
            jwt_secret.chars().take(10).collect::<String>()
        );
        let auth_service = AdminAuthService::new((*database).clone(), jwks_manager.clone());
        Self {
            database,
            auth_service,
            auth_manager,
            admin_jwt_secret: jwt_secret.to_owned(),
            jwks_manager,
            admin_api_key_monthly_limit,
        }
    }
}

/// Helper functions for admin operations
/// Convert rate limit period string to window duration in seconds
fn convert_rate_limit_period(period: &str) -> Result<u32> {
    match period.to_lowercase().as_str() {
        "hour" => Ok(SECONDS_PER_HOUR),   // 1 hour
        "day" => Ok(SECONDS_PER_DAY),     // 24 hours
        "week" => Ok(SECONDS_PER_WEEK),   // 7 days
        "month" => Ok(SECONDS_PER_MONTH), // 30 days
        _ => Err(AppError::invalid_input(
            "Invalid rate limit period. Supported: hour, day, week, month",
        )
        .into()),
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
async fn get_existing_user(database: &Database, email: &str) -> Result<User, AppError> {
    match database.get_user_by_email(email).await {
        Ok(Some(user)) => Ok(user),
        Ok(None) => {
            tracing::warn!("API key provisioning failed: User {} does not exist", email);
            Err(AppError::invalid_input(format!(
                "User {email} must register and be approved before API key provisioning"
            )))
        }
        Err(e) => Err(AppError::internal(format!("Failed to lookup user: {e}"))),
    }
}

/// Create and store API key
async fn create_and_store_api_key(
    context: &AdminApiContext,
    user: &User,
    request: &ProvisionApiKeyRequest,
    tier: &ApiKeyTier,
    admin_token: &crate::admin::models::ValidatedAdminToken,
) -> Result<(ApiKey, String), String> {
    // Generate API key using ApiKeyManager
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
) -> Result<ProvisionApiKeyRequest, (axum::http::StatusCode, Json<AdminResponse>)> {
    match serde_json::from_slice(body) {
        Ok(req) => Ok(req),
        Err(e) => {
            tracing::warn!(error = %e, "Invalid JSON body in provision API key request");
            Err((
                axum::http::StatusCode::BAD_REQUEST,
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
    admin_token: &crate::admin::models::ValidatedAdminToken,
) -> Result<(), (axum::http::StatusCode, Json<AdminResponse>)> {
    if admin_token
        .permissions
        .has_permission(&crate::admin::AdminPermission::ProvisionKeys)
    {
        Ok(())
    } else {
        Err((
            axum::http::StatusCode::FORBIDDEN,
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
) -> Result<ApiKeyTier, (axum::http::StatusCode, Json<AdminResponse>)> {
    validate_tier(tier_str).map_err(|error_msg| {
        (
            axum::http::StatusCode::BAD_REQUEST,
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
) -> Result<User, (axum::http::StatusCode, Json<AdminResponse>)> {
    get_existing_user(database, email).await.map_err(|_e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
    admin_token: &crate::admin::models::ValidatedAdminToken,
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
        tracing::warn!("Failed to record admin provisioned key: {}", e);
    }
}

/// Check if any admin users already exist
///
/// Returns an error response if an admin already exists, or Ok(None) if setup can proceed
async fn check_no_admin_exists(
    database: &Database,
) -> Result<Option<(axum::http::StatusCode, Json<AdminResponse>)>> {
    match database.get_users_by_status("active").await {
        Ok(users) => {
            let admin_exists = users.iter().any(|u| u.is_admin);
            if admin_exists {
                return Ok(Some((
                    axum::http::StatusCode::CONFLICT,
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
            tracing::error!("Failed to check existing admin users: {}", e);
            Ok(Some((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
) -> Result<Uuid, (axum::http::StatusCode, Json<AdminResponse>)> {
    let user_id = Uuid::new_v4();

    // Hash password
    let password_hash = match bcrypt::hash(&request.password, bcrypt::DEFAULT_COST) {
        Ok(hash) => hash,
        Err(e) => {
            tracing::error!("Failed to hash password: {}", e);
            return Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
            tracing::info!("Admin user created successfully: {}", request.email);
            Ok(user_id)
        }
        Err(e) => {
            tracing::error!("Failed to create admin user: {}", e);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
    jwks_manager: &Arc<crate::admin::jwks::JwksManager>,
) -> Result<String, (axum::http::StatusCode, Json<AdminResponse>)> {
    let token_request = crate::admin::models::CreateAdminTokenRequest {
        service_name: "initial_admin_setup".to_owned(),
        service_description: Some("Initial admin setup token".to_owned()),
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

    match database
        .create_admin_token(&token_request, admin_jwt_secret, jwks_manager)
        .await
    {
        Ok(generated_token) => Ok(generated_token.jwt_token),
        Err(e) => {
            tracing::error!("Failed to generate admin token after creating user: {}", e);
            Err((
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
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
    pub fn routes(context: AdminApiContext) -> axum::Router {
        use axum::{middleware, Router};

        let context = Arc::new(context);

        // Create admin auth service for middleware
        let auth_service = crate::admin::AdminAuthService::new(
            context.database.as_ref().clone(),
            context.jwks_manager.clone(),
        );

        // Protected routes require admin authentication
        let api_key_routes =
            Self::api_key_routes(context.clone()).layer(middleware::from_fn_with_state(
                auth_service.clone(),
                crate::admin::middleware::admin_auth_middleware,
            ));

        let user_routes = Self::user_routes(context.clone()).layer(middleware::from_fn_with_state(
            auth_service.clone(),
            crate::admin::middleware::admin_auth_middleware,
        ));

        let admin_token_routes =
            Self::admin_token_routes(context.clone()).layer(middleware::from_fn_with_state(
                auth_service,
                crate::admin::middleware::admin_auth_middleware,
            ));

        // Setup routes are public (no auth required for initial setup)
        let setup_routes = Self::setup_routes(context);

        Router::new()
            .merge(api_key_routes)
            .merge(user_routes)
            .merge(admin_token_routes)
            .merge(setup_routes)
    }

    /// API key management routes (Axum)
    fn api_key_routes(context: Arc<AdminApiContext>) -> axum::Router {
        use axum::{routing::get, routing::post, Router};

        Router::new()
            .route("/admin/provision", post(Self::handle_provision_api_key))
            .route("/admin/revoke", post(Self::handle_revoke_api_key))
            .route("/admin/list", get(Self::handle_list_api_keys))
            .route("/admin/token-info", get(Self::handle_token_info))
            .with_state(context)
    }

    /// User management routes (Axum)
    fn user_routes(context: Arc<AdminApiContext>) -> axum::Router {
        use axum::{routing::get, routing::post, Router};

        Router::new()
            .route("/admin/users", get(Self::handle_list_users))
            .route("/admin/pending-users", get(Self::handle_pending_users))
            .route(
                "/admin/approve-user/:user_id",
                post(Self::handle_approve_user),
            )
            .with_state(context)
    }

    /// Setup routes (Axum)
    fn setup_routes(context: Arc<AdminApiContext>) -> axum::Router {
        use axum::{
            routing::{get, post},
            Router,
        };

        Router::new()
            .route("/admin/setup", post(Self::handle_admin_setup))
            .route("/admin/setup/status", get(Self::handle_setup_status))
            .route("/admin/health", get(Self::handle_health))
            .with_state(context)
    }

    /// Admin token management routes (Axum)
    fn admin_token_routes(context: Arc<AdminApiContext>) -> axum::Router {
        use axum::{routing::get, routing::post, Router};

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

    /// Handle API key provisioning (Axum)
    async fn handle_provision_api_key(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        body: axum::body::Bytes,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Parse and validate request
        let request = match parse_provision_request(&body) {
            Ok(req) => req,
            Err(response) => return Ok(response),
        };

        // Check required permission
        if let Err(response) = check_provision_permission(&admin_token) {
            return Ok(response);
        }

        tracing::info!(
            "Provisioning API key for user: {} by service: {}",
            request.user_email,
            admin_token.service_name
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
                        axum::http::StatusCode::BAD_REQUEST
                    } else {
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
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

        tracing::info!(
            "API key provisioned successfully: {} for user: {}",
            final_api_key.id,
            user.email
        );

        let provision_response =
            create_provision_response(&final_api_key, api_key_string, &user, &tier, period_name);

        // Wrap in AdminResponse for consistency
        Ok((
            axum::http::StatusCode::CREATED,
            Json(AdminResponse {
                success: true,
                message: format!("API key provisioned successfully for {}", user.email),
                data: serde_json::to_value(&provision_response).ok(),
            }),
        ))
    }

    /// Handle API key revocation (Axum)
    async fn handle_revoke_api_key(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Json(request): Json<RevokeKeyRequest>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::RevokeKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: RevokeKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Revoking API key: {} by service: {}",
            request.api_key_id,
            admin_token.service_name
        );

        let ctx = context.as_ref();

        // Get the API key to find the user_id
        let api_key = match ctx.database.get_api_key_by_id(&request.api_key_id).await {
            Ok(Some(key)) => key,
            Ok(None) => {
                return Ok(json_response(
                    AdminResponse {
                        success: false,
                        message: format!("API key {} not found", request.api_key_id),
                        data: None,
                    },
                    axum::http::StatusCode::NOT_FOUND,
                ));
            }
            Err(e) => {
                return Ok(json_response(
                    AdminResponse {
                        success: false,
                        message: format!("Failed to lookup API key: {e}"),
                        data: None,
                    },
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        };

        match ctx
            .database
            .deactivate_api_key(&request.api_key_id, api_key.user_id)
            .await
        {
            Ok(()) => {
                tracing::info!("API key revoked successfully: {}", request.api_key_id);

                Ok(json_response(
                    AdminResponse {
                        success: true,
                        message: format!("API key {} revoked successfully", request.api_key_id),
                        data: Some(serde_json::json!({
                            "api_key_id": request.api_key_id,
                            "revoked_by": admin_token.service_name,
                            "reason": request.reason.unwrap_or_else(|| "Admin revocation".into())
                        })),
                    },
                    axum::http::StatusCode::OK,
                ))
            }
            Err(e) => {
                tracing::warn!("Failed to revoke API key {}: {}", request.api_key_id, e);

                Ok(json_response(
                    AdminResponse {
                        success: false,
                        message: format!("Failed to revoke API key: {e}"),
                        data: None,
                    },
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }

    /// Handle API key listing (Axum)
    async fn handle_list_api_keys(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Query(params): Query<ListApiKeysQuery>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ListKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ListKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!("Listing API keys by service: {}", admin_token.service_name);

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
                        serde_json::json!({
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
                    },
                    axum::http::StatusCode::OK,
                ))
            }
            Err(e) => {
                tracing::warn!("Failed to list API keys: {}", e);
                Ok(json_response(
                    AdminResponse {
                        success: false,
                        message: format!("Failed to list API keys: {e}"),
                        data: None,
                    },
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }

    /// Handle user listing (Axum)
    async fn handle_list_users(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Query(params): Query<ListUsersQuery>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ManageUsers)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ManageUsers required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!("Listing users by service: {}", admin_token.service_name);

        let ctx = context.as_ref();

        // Determine status filter - default to "active"
        let status = params.status.as_deref().unwrap_or("active");

        // Fetch users from database by status
        let users = ctx
            .database
            .get_users_by_status(status)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch users from database");
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

        tracing::info!("Retrieved {} users", total);

        Ok(json_response(
            AdminResponse {
                success: true,
                message: format!("Retrieved {total} users"),
                data: serde_json::to_value(UserListResponse {
                    users: user_summaries,
                    total,
                })
                .ok(),
            },
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle pending users listing (Axum)
    async fn handle_pending_users(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ManageUsers)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ManageUsers required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Listing pending users by service: {}",
            admin_token.service_name
        );

        let ctx = context.as_ref();

        // Fetch users with Pending status
        let users = ctx
            .database
            .get_users_by_status("pending")
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch pending users from database");
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

        tracing::info!("Retrieved {} pending users", count);

        Ok(json_response(
            AdminResponse {
                success: true,
                message: format!("Retrieved {count} pending users"),
                data: serde_json::to_value(serde_json::json!({
                    "count": count,
                    "users": user_summaries
                }))
                .ok(),
            },
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle user approval (Axum)
    async fn handle_approve_user(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        axum::extract::Path(user_id): axum::extract::Path<String>,
        Json(request): Json<serde_json::Value>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ManageUsers)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ManageUsers required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Approving user {} by service: {}",
            user_id,
            admin_token.service_name
        );

        let ctx = context.as_ref();

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&user_id).map_err(|e| {
            tracing::error!(error = %e, "Invalid user ID format");
            AppError::invalid_input(format!("Invalid user ID format: {e}"))
        })?;

        // Fetch user from database
        let user = ctx
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

        // Check if user is already approved
        if user.user_status == UserStatus::Active {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "User is already approved".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::BAD_REQUEST,
            ));
        }

        // Update user status to Active (this also sets approved_by and approved_at)
        let updated_user = ctx
            .database
            .update_user_status(user_uuid, UserStatus::Active, &admin_token.service_name)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to update user status in database");
                AppError::internal(format!("Failed to approve user: {e}"))
            })?;

        let reason = request
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("No reason provided");

        tracing::info!("User {} approved successfully. Reason: {}", user_id, reason);

        Ok(json_response(
            AdminResponse {
                success: true,
                message: "User approved successfully".to_owned(),
                data: serde_json::to_value(serde_json::json!({
                    "user": {
                        "id": updated_user.id.to_string(),
                        "email": updated_user.email,
                        "user_status": match updated_user.user_status {
                            UserStatus::Pending => "pending",
                            UserStatus::Active => "active",
                            UserStatus::Suspended => "suspended",
                        },
                        "approved_by": updated_user.approved_by,
                        "approved_at": updated_user.approved_at.map(|t| t.to_rfc3339()),
                    },
                    "reason": reason
                }))
                .ok(),
            },
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle admin token creation (Axum)
    async fn handle_create_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        Json(request): Json<serde_json::Value>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ProvisionKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ProvisionKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
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
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        let expires_in_days = request
            .get("expires_in_days")
            .and_then(serde_json::Value::as_u64);

        // Parse permissions if provided
        let permissions =
            if let Some(perms_array) = request.get("permissions").and_then(|v| v.as_array()) {
                let mut parsed_permissions = Vec::new();
                for p in perms_array {
                    if let Some(perm_str) = p.as_str() {
                        match perm_str.parse::<crate::admin::models::AdminPermission>() {
                            Ok(perm) => parsed_permissions.push(perm),
                            Err(_) => {
                                return Ok(json_response(
                                    AdminResponse {
                                        success: false,
                                        message: format!("Invalid permission: {perm_str}"),
                                        data: None,
                                    },
                                    axum::http::StatusCode::BAD_REQUEST,
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
        let token_request = crate::admin::models::CreateAdminTokenRequest {
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
                tracing::error!(error = %e, "Failed to generate admin token");
                AppError::internal(format!("Failed to generate admin token: {e}"))
            })?;

        tracing::info!("Admin token created: {}", generated_token.token_id);

        Ok(json_response(
            AdminResponse {
                success: true,
                message: "Admin token created successfully".to_owned(),
                data: serde_json::to_value(serde_json::json!({
                    "token_id": generated_token.token_id,
                    "service_name": generated_token.service_name,
                    "jwt_token": generated_token.jwt_token,
                    "token_prefix": generated_token.token_prefix,
                    "is_super_admin": generated_token.is_super_admin,
                    "expires_at": generated_token.expires_at.map(|t| t.to_rfc3339()),
                }))
                .ok(),
            },
            axum::http::StatusCode::CREATED,
        ))
    }

    /// Handle listing admin tokens (Axum)
    async fn handle_list_admin_tokens(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ProvisionKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ProvisionKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Listing admin tokens by service: {}",
            admin_token.service_name
        );

        let ctx = context.as_ref();

        let tokens = ctx.database.list_admin_tokens(false).await.map_err(|e| {
            tracing::error!(error = %e, "Failed to list admin tokens");
            AppError::internal(format!("Failed to list admin tokens: {e}"))
        })?;

        tracing::info!("Retrieved {} admin tokens", tokens.len());

        Ok(json_response(
            AdminResponse {
                success: true,
                message: format!("Retrieved {} admin tokens", tokens.len()),
                data: serde_json::to_value(serde_json::json!({
                    "count": tokens.len(),
                    "tokens": tokens
                }))
                .ok(),
            },
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle getting admin token details (Axum)
    async fn handle_get_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        axum::extract::Path(token_id): axum::extract::Path<String>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ProvisionKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ProvisionKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Getting admin token {} by service: {}",
            token_id,
            admin_token.service_name
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
                    axum::http::StatusCode::NOT_FOUND,
                ));
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to get admin token");
                return Ok(json_response(
                    AdminResponse {
                        success: false,
                        message: format!("Failed to get admin token: {e}"),
                        data: None,
                    },
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        };

        Ok(json_response(
            AdminResponse {
                success: true,
                message: "Admin token retrieved successfully".to_owned(),
                data: serde_json::to_value(token).ok(),
            },
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle revoking admin token (Axum)
    async fn handle_revoke_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        axum::extract::Path(token_id): axum::extract::Path<String>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ProvisionKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ProvisionKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Revoking admin token {} by service: {}",
            token_id,
            admin_token.service_name
        );

        let ctx = context.as_ref();

        // Deactivate the token
        ctx.database
            .deactivate_admin_token(&token_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to revoke admin token");
                AppError::internal(format!("Failed to revoke admin token: {e}"))
            })?;

        tracing::info!("Admin token {} revoked successfully", token_id);

        Ok(json_response(
            AdminResponse {
                success: true,
                message: "Admin token revoked successfully".to_owned(),
                data: serde_json::to_value(serde_json::json!({
                    "token_id": token_id
                }))
                .ok(),
            },
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle rotating admin token (Axum)
    async fn handle_rotate_admin_token(
        State(context): State<Arc<AdminApiContext>>,
        Extension(admin_token): Extension<ValidatedAdminToken>,
        axum::extract::Path(token_id): axum::extract::Path<String>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        // Check required permission
        if !admin_token
            .permissions
            .has_permission(&crate::admin::AdminPermission::ProvisionKeys)
        {
            return Ok(json_response(
                AdminResponse {
                    success: false,
                    message: "Permission denied: ProvisionKeys required".to_owned(),
                    data: None,
                },
                axum::http::StatusCode::FORBIDDEN,
            ));
        }

        tracing::info!(
            "Rotating admin token {} by service: {}",
            token_id,
            admin_token.service_name
        );

        let ctx = context.as_ref();

        // Get existing token to copy its properties
        let existing_token = ctx
            .database
            .get_admin_token_by_id(&token_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to get admin token");
                AppError::internal(format!("Failed to get admin token: {e}"))
            })?
            .ok_or_else(|| AppError::not_found("Admin token not found"))?;

        // Deactivate old token
        ctx.database
            .deactivate_admin_token(&token_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to deactivate old token");
                AppError::internal(format!("Failed to deactivate old token: {e}"))
            })?;

        // Generate new token with same properties
        let token_request = crate::admin::models::CreateAdminTokenRequest {
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
                tracing::error!(error = %e, "Failed to generate new admin token");
                AppError::internal(format!("Failed to generate new admin token: {e}"))
            })?;

        tracing::info!(
            "Admin token {} rotated successfully, new token: {}",
            token_id,
            new_token.token_id
        );

        Ok(json_response(
            AdminResponse {
                success: true,
                message: "Admin token rotated successfully".to_owned(),
                data: serde_json::to_value(serde_json::json!({
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
            axum::http::StatusCode::OK,
        ))
    }

    /// Handle admin setup (Axum)
    async fn handle_admin_setup(
        State(context): State<Arc<AdminApiContext>>,
        Json(request): Json<AdminSetupRequest>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        tracing::info!("Admin setup request for email: {}", request.email);

        let ctx = context.as_ref();

        // Check if any admin users already exist
        if let Some(error_response) = check_no_admin_exists(&ctx.database).await? {
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
        tracing::info!("Admin setup completed successfully for: {}", request.email);
        Ok((
            axum::http::StatusCode::CREATED,
            Json(AdminResponse {
                success: true,
                message: format!(
                    "Admin user {} created successfully with token",
                    request.email
                ),
                data: Some(serde_json::json!({
                    "user_id": user_id.to_string(),
                    "admin_token": admin_token,
                })),
            }),
        ))
    }

    /// Handle setup status check
    async fn handle_setup_status(
        State(context): State<Arc<AdminApiContext>>,
    ) -> Result<impl axum::response::IntoResponse, AppError> {
        tracing::info!("Setup status check requested");

        let ctx = context.as_ref();

        match ctx.auth_manager.check_setup_status(&ctx.database).await {
            Ok(setup_status) => {
                tracing::info!(
                    "Setup status check successful: needs_setup={}, admin_user_exists={}",
                    setup_status.needs_setup,
                    setup_status.admin_user_exists
                );
                Ok(json_response(setup_status, axum::http::StatusCode::OK))
            }
            Err(e) => {
                use crate::routes::auth::SetupStatusResponse;

                tracing::error!("Failed to check setup status: {}", e);
                Ok(json_response(
                    SetupStatusResponse {
                        needs_setup: true,
                        admin_user_exists: false,
                        message: Some("Unable to determine setup status. Please ensure admin user is created.".to_owned()),
                    },
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }

    /// Handle health check (GET /admin/health)
    async fn handle_health() -> axum::Json<serde_json::Value> {
        // Use spawn_blocking for JSON serialization (CPU-bound operation)
        let health_json = tokio::task::spawn_blocking(|| {
            serde_json::json!({
                "status": "healthy",
                "service": "pierre-mcp-admin-api",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "version": env!("CARGO_PKG_VERSION")
            })
        })
        .await
        .unwrap_or_else(|_| {
            serde_json::json!({
                "status": "error",
                "service": "pierre-mcp-admin-api"
            })
        });

        axum::Json(health_json)
    }

    /// Handle token info (GET /admin/token-info)
    /// Returns information about the authenticated admin token
    async fn handle_token_info(
        Extension(admin_token): Extension<ValidatedAdminToken>,
    ) -> axum::Json<serde_json::Value> {
        // Clone values before spawn_blocking
        let token_id = admin_token.token_id;
        let service_name = admin_token.service_name.clone();
        let permissions = admin_token.permissions.clone();
        let is_super_admin = admin_token.is_super_admin;

        // Use spawn_blocking for JSON serialization (CPU-bound operation)
        let token_info_json = tokio::task::spawn_blocking(move || {
            // Convert permissions to JSON array
            let permission_strings: Vec<String> = permissions
                .to_vec()
                .iter()
                .map(ToString::to_string)
                .collect();

            serde_json::json!({
                "token_id": token_id,
                "service_name": service_name,
                "permissions": permission_strings,
                "is_super_admin": is_super_admin
            })
        })
        .await
        .unwrap_or_else(|_| {
            serde_json::json!({
                "error": "Failed to serialize token info"
            })
        });

        axum::Json(token_info_json)
    }
}
