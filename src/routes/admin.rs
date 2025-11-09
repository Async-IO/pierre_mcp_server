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
    admin::{auth::AdminAuthService, models::AdminPermission},
    api_keys::{ApiKeyManager, CreateApiKeyRequestSimple},
    auth::AuthManager,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, ErrorCode},
    models::{User, UserStatus, UserTier},
    utils::auth::extract_bearer_token_owned,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use warp::{
    http::StatusCode,
    reply::{json, with_status},
    Filter, Rejection, Reply,
};

/// API key provisioning request
#[derive(Debug, Deserialize)]
pub struct ApiKeyRequest {
    /// Name of the service requesting the API key
    pub service_name: String,
    /// Number of days until the key expires
    pub expires_days: Option<i64>,
    /// Optional list of permission scopes for the key
    pub scopes: Option<Vec<String>>,
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

/// API key list response
#[derive(Debug, Serialize)]
struct ApiKeyListResponse {
    /// List of API keys (sanitized - no secrets)
    api_keys: Vec<ApiKeySummary>,
    /// Total number of keys
    total: usize,
}

/// Sanitized API key summary for listing
#[derive(Debug, Serialize)]
struct ApiKeySummary {
    /// API key ID
    id: String,
    /// User ID who owns the key
    user_id: String,
    /// Key prefix for identification
    prefix: String,
    /// Key tier
    tier: String,
    /// Whether key is active
    is_active: bool,
    /// When key was created
    created_at: String,
    /// When key expires (if set)
    expires_at: Option<String>,
}

/// User list response
#[derive(Debug, Serialize)]
struct UserListResponse {
    /// List of users (sanitized - no passwords)
    users: Vec<UserSummary>,
    /// Total number of users
    total: usize,
}

/// Sanitized user summary for listing
#[derive(Debug, Serialize)]
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
    /// Default monthly request limit for admin-provisioned API keys
    pub admin_api_key_monthly_limit: u32,
}

impl AdminApiContext {
    /// Creates a new admin API context
    pub fn new(
        database: &Arc<Database>,
        jwt_secret: &str,
        auth_manager: &Arc<AuthManager>,
        jwks_manager: &Arc<crate::admin::jwks::JwksManager>,
        admin_api_key_monthly_limit: u32,
    ) -> Self {
        tracing::info!(
            "Creating AdminApiContext with JWT secret (first 10 chars): {}...",
            &jwt_secret[..10.min(jwt_secret.len())]
        );

        Self {
            database: database.clone(),
            auth_service: AdminAuthService::new((**database).clone(), jwks_manager.clone()),
            auth_manager: auth_manager.clone(),
            admin_jwt_secret: jwt_secret.to_owned(),
            admin_api_key_monthly_limit,
        }
    }
}

/// Admin routes implementation
pub struct AdminRoutes;

impl AdminRoutes {
    /// Create all admin routes
    #[must_use]
    pub fn routes(
        context: AdminApiContext,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let api_keys = Self::api_key_routes(context.clone());
        let users = Self::user_routes(context.clone());
        let setup = Self::setup_routes(context);

        api_keys.or(users).or(setup).boxed()
    }

    /// API key management routes
    fn api_key_routes(
        context: AdminApiContext,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let provision = warp::path("admin")
            .and(warp::path("provision"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("authorization"))
            .and(with_context(context.clone()))
            .and_then(Self::handle_provision_api_key);

        let revoke = warp::path("admin")
            .and(warp::path("revoke"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("authorization"))
            .and(with_context(context.clone()))
            .and_then(Self::handle_revoke_api_key);

        let list = warp::path("admin")
            .and(warp::path("list"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::query::query())
            .and(warp::header::optional::<String>("authorization"))
            .and(with_context(context))
            .and_then(Self::handle_list_api_keys);

        provision.or(revoke).or(list).boxed()
    }

    /// User management routes
    fn user_routes(
        context: AdminApiContext,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        warp::path("admin")
            .and(warp::path("users"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(with_context(context))
            .and_then(Self::handle_list_users)
    }

    /// Setup routes
    fn setup_routes(
        context: AdminApiContext,
    ) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let setup = warp::path("admin")
            .and(warp::path("setup"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and(with_context(context.clone()))
            .and_then(Self::handle_admin_setup);

        let status = warp::path("admin")
            .and(warp::path("setup").and(warp::path("status")))
            .and(warp::path::end())
            .and(warp::get())
            .and(with_context(context))
            .and_then(Self::handle_setup_status);

        setup.or(status).boxed()
    }

    /// Handle API key provisioning
    async fn handle_provision_api_key(
        request: serde_json::Value,
        auth_header: Option<String>,
        context: AdminApiContext,
    ) -> Result<impl Reply, Rejection> {
        tokio::task::yield_now().await;

        // Extract and validate admin token
        let auth_header = auth_header.ok_or_else(|| {
            warp::reject::custom(AppError::auth_invalid("Authorization header required"))
        })?;

        let token = extract_bearer_token_owned(&auth_header).map_err(|e| {
            tracing::warn!(error = %e, "Failed to extract bearer token");
            warp::reject::custom(AppError::auth_invalid("Invalid authorization header"))
        })?;

        // Authenticate admin with ProvisionKeys permission
        context
            .auth_service
            .authenticate_and_authorize(&token, AdminPermission::ProvisionKeys, None)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Admin authentication failed");
                warp::reject::custom(AppError::auth_invalid("Admin authentication failed"))
            })?;

        // Parse request
        let req: ApiKeyRequest = serde_json::from_value(request).map_err(|e| {
            tracing::warn!(error = %e, "Invalid API key request");
            warp::reject::custom(AppError::invalid_input(format!(
                "Invalid request format: {e}"
            )))
        })?;

        tracing::info!("Provisioning API key for service: {}", req.service_name);

        // Use a dummy user ID for admin-created keys (or we could look up by email)
        // For now, create a system user ID for admin-provisioned keys
        let system_user_id = Uuid::nil(); // Could be a dedicated system user

        // Generate new API key using ApiKeyManager
        let api_key_manager = ApiKeyManager::default();
        let request = CreateApiKeyRequestSimple {
            name: req.service_name.clone(),
            description: None,
            rate_limit_requests: context.admin_api_key_monthly_limit,
            expires_in_days: req.expires_days,
        };

        let (api_key, raw_key) = api_key_manager
            .create_api_key_simple(system_user_id, request)
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to generate API key");
                warp::reject::custom(AppError::internal(format!(
                    "Failed to generate API key: {e}"
                )))
            })?;

        // Store in database
        context
            .database
            .create_api_key(&api_key)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create API key in database");
                warp::reject::custom(AppError::internal(format!("Failed to create API key: {e}")))
            })?;

        tracing::info!(
            "API key provisioned successfully for service: {}",
            req.service_name
        );

        Ok(with_status(
            json(&serde_json::json!({
                "success": true,
                "api_key_id": api_key.id,
                "api_key_prefix": api_key.key_prefix,
                "api_key": raw_key, // Return the actual key (shown only once!)
                "message": "API key provisioned successfully"
            })),
            StatusCode::CREATED,
        ))
    }

    /// Handle API key revocation
    async fn handle_revoke_api_key(
        request: serde_json::Value,
        auth_header: Option<String>,
        context: AdminApiContext,
    ) -> Result<impl Reply, Rejection> {
        tokio::task::yield_now().await;

        // Extract and validate admin token
        let auth_header = auth_header.ok_or_else(|| {
            warp::reject::custom(AppError::auth_invalid("Authorization header required"))
        })?;

        let token = extract_bearer_token_owned(&auth_header).map_err(|e| {
            tracing::warn!(error = %e, "Failed to extract bearer token");
            warp::reject::custom(AppError::auth_invalid("Invalid authorization header"))
        })?;

        // Authenticate admin with RevokeKeys permission
        context
            .auth_service
            .authenticate_and_authorize(&token, AdminPermission::RevokeKeys, None)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Admin authentication failed");
                warp::reject::custom(AppError::auth_invalid("Admin authentication failed"))
            })?;

        // Parse request
        let req: RevokeKeyRequest = serde_json::from_value(request).map_err(|e| {
            tracing::warn!(error = %e, "Invalid revoke key request");
            warp::reject::custom(AppError::invalid_input(format!(
                "Invalid request format: {e}"
            )))
        })?;

        tracing::info!("Revoking API key: {}", req.api_key_id);

        // Get API key to verify it exists and get user_id
        let api_key = context
            .database
            .get_api_key_by_id(&req.api_key_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch API key from database");
                warp::reject::custom(AppError::internal(format!("Failed to fetch API key: {e}")))
            })?
            .ok_or_else(|| {
                warp::reject::custom(AppError::not_found(format!(
                    "API key not found: {}",
                    req.api_key_id
                )))
            })?;

        // Deactivate the key
        context
            .database
            .deactivate_api_key(&req.api_key_id, api_key.user_id)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to deactivate API key");
                warp::reject::custom(AppError::internal(format!("Failed to revoke API key: {e}")))
            })?;

        tracing::info!("API key revoked successfully: {}", req.api_key_id);

        Ok(with_status(
            json(&serde_json::json!({
                "success": true,
                "message": "API key revoked successfully"
            })),
            StatusCode::OK,
        ))
    }

    /// Handle API key listing
    async fn handle_list_api_keys(
        params: HashMap<String, String>,
        auth_header: Option<String>,
        context: AdminApiContext,
    ) -> Result<impl Reply, Rejection> {
        tokio::task::yield_now().await;

        // Extract and validate admin token
        let auth_header = auth_header.ok_or_else(|| {
            warp::reject::custom(AppError::auth_invalid("Authorization header required"))
        })?;

        let token = extract_bearer_token_owned(&auth_header).map_err(|e| {
            tracing::warn!(error = %e, "Failed to extract bearer token");
            warp::reject::custom(AppError::auth_invalid("Invalid authorization header"))
        })?;

        // Authenticate admin with ListKeys permission
        context
            .auth_service
            .authenticate_and_authorize(&token, AdminPermission::ListKeys, None)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Admin authentication failed");
                warp::reject::custom(AppError::auth_invalid("Admin authentication failed"))
            })?;

        tracing::info!("Listing API keys with params: {:?}", params);

        // Extract optional filtering parameters
        let user_email = params.get("email").map(String::as_str);
        let active_only = params
            .get("active_only")
            .and_then(|s| s.parse().ok())
            .unwrap_or(true);
        let limit: Option<i32> = params
            .get("limit")
            .and_then(|s| s.parse().ok())
            .or(Some(100)); // Default limit
        let offset: Option<i32> = params.get("offset").and_then(|s| s.parse().ok());

        // Fetch API keys from database with filtering
        let api_keys = context
            .database
            .get_api_keys_filtered(user_email, active_only, limit, offset)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch API keys from database");
                warp::reject::custom(AppError::internal(format!("Failed to fetch API keys: {e}")))
            })?;

        // Convert to sanitized summaries
        let api_key_summaries: Vec<ApiKeySummary> = api_keys
            .iter()
            .map(|key| ApiKeySummary {
                id: key.id.clone(),
                user_id: key.user_id.to_string(),
                prefix: key.key_prefix.clone(),
                tier: key.tier.to_string(),
                is_active: key.is_active,
                created_at: key.created_at.to_rfc3339(),
                expires_at: key.expires_at.map(|dt| dt.to_rfc3339()),
            })
            .collect();

        let total = api_key_summaries.len();

        tracing::info!("Retrieved {} API keys", total);

        Ok(with_status(
            json(&ApiKeyListResponse {
                api_keys: api_key_summaries,
                total,
            }),
            StatusCode::OK,
        ))
    }

    /// Handle user listing
    async fn handle_list_users(
        auth_header: Option<String>,
        context: AdminApiContext,
    ) -> Result<impl Reply, Rejection> {
        tokio::task::yield_now().await;

        // Extract and validate admin token
        let auth_header = auth_header.ok_or_else(|| {
            warp::reject::custom(AppError::auth_invalid("Authorization header required"))
        })?;

        let token = extract_bearer_token_owned(&auth_header).map_err(|e| {
            tracing::warn!(error = %e, "Failed to extract bearer token");
            warp::reject::custom(AppError::auth_invalid("Invalid authorization header"))
        })?;

        // Authenticate admin with ManageUsers permission
        context
            .auth_service
            .authenticate_and_authorize(&token, AdminPermission::ManageUsers, None)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "Admin authentication failed");
                warp::reject::custom(AppError::auth_invalid("Admin authentication failed"))
            })?;

        tracing::info!("Listing users");

        // Fetch all active users from database (status = "active")
        let users = context
            .database
            .get_users_by_status("active")
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to fetch users from database");
                warp::reject::custom(AppError::internal(format!("Failed to fetch users: {e}")))
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

        Ok(with_status(
            json(&UserListResponse {
                users: user_summaries,
                total,
            }),
            StatusCode::OK,
        ))
    }

    /// Handle admin setup
    async fn handle_admin_setup(
        request: serde_json::Value,
        context: AdminApiContext,
    ) -> Result<impl Reply, Rejection> {
        tokio::task::yield_now().await;

        // Parse request
        let req: AdminSetupRequest = serde_json::from_value(request).map_err(|e| {
            tracing::warn!(error = %e, "Invalid admin setup request");
            warp::reject::custom(AppError::invalid_input(format!(
                "Invalid request format: {e}"
            )))
        })?;

        tracing::info!("Setting up admin user: {}", req.email);

        // Check if any users already exist
        let user_count = context.database.get_user_count().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to count users");
            warp::reject::custom(AppError::internal(format!(
                "Failed to check user count: {e}"
            )))
        })?;

        if user_count > 0 {
            tracing::warn!("Admin setup rejected: users already exist");
            return Err(warp::reject::custom(AppError::new(
                ErrorCode::ResourceAlreadyExists,
                "Admin setup not allowed: users already exist",
            )));
        }

        // Hash password
        let password_hash = bcrypt::hash(&req.password, bcrypt::DEFAULT_COST).map_err(|e| {
            tracing::error!(error = %e, "Failed to hash password");
            warp::reject::custom(AppError::internal("Failed to hash password"))
        })?;

        // Create admin user
        let admin_user = User {
            id: Uuid::new_v4(),
            email: req.email.clone(),
            display_name: req.display_name.clone(),
            password_hash,
            tier: UserTier::Enterprise, // Admin gets highest tier
            tenant_id: None,
            strava_token: None,
            fitbit_token: None,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            is_active: true,
            user_status: UserStatus::Active, // Admin is pre-approved
            is_admin: true,
            approved_by: None, // Self-approved (initial admin)
            approved_at: Some(chrono::Utc::now()),
        };

        // Store in database
        let user_id = context
            .database
            .create_user(&admin_user)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "Failed to create admin user");
                warp::reject::custom(AppError::internal(format!("Failed to create user: {e}")))
            })?;

        tracing::info!("Admin user created successfully: {}", user_id);

        Ok(with_status(
            json(&serde_json::json!({
                "success": true,
                "user_id": user_id,
                "email": req.email,
                "message": "Admin user created successfully"
            })),
            StatusCode::CREATED,
        ))
    }

    /// Handle setup status check
    async fn handle_setup_status(context: AdminApiContext) -> Result<impl Reply, Rejection> {
        tokio::task::yield_now().await;

        tracing::debug!("Checking admin setup status");

        // Check if any users exist
        let user_count = context.database.get_user_count().await.map_err(|e| {
            tracing::error!(error = %e, "Failed to count users");
            warp::reject::custom(AppError::internal(format!(
                "Failed to check user count: {e}"
            )))
        })?;

        let needs_setup = user_count == 0;
        let admin_user_exists = user_count > 0;

        tracing::info!(
            "Setup status: needs_setup={}, admin_user_exists={}",
            needs_setup,
            admin_user_exists
        );

        Ok(with_status(
            json(&serde_json::json!({
                "needs_setup": needs_setup,
                "admin_user_exists": admin_user_exists,
                "user_count": user_count
            })),
            StatusCode::OK,
        ))
    }
}

/// Helper to inject admin context into route handlers
fn with_context(
    context: AdminApiContext,
) -> impl Filter<Extract = (AdminApiContext,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || context.clone())
}
