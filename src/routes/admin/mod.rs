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

mod api_keys;
mod settings;
mod setup;
mod store;
mod tokens;
mod types;
mod users;

pub use types::{
    AdminResponse, AdminSetupRequest, AdminSetupResponse, ApproveUserRequest, AutoApprovalResponse,
    CoachReviewQuery, DeleteUserRequest, ListApiKeysQuery, ListPendingCoachesQuery, ListUsersQuery,
    ProvisionApiKeyRequest, ProvisionApiKeyResponse, RateLimitInfo, RejectCoachRequest,
    RevokeKeyRequest, SuspendUserRequest, TenantCreatedInfo, UpdateAutoApprovalRequest,
    UserActivityQuery,
};

use std::sync::Arc;

use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use tracing::info;

use crate::{
    admin::{auth::AdminAuthService, jwks::JwksManager, middleware::admin_auth_middleware},
    auth::AuthManager,
    database_plugins::factory::Database,
    mcp::ToolSelectionService,
    routes::tool_selection::{ToolSelectionContext, ToolSelectionRoutes},
};

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

/// Admin routes implementation (Axum)
///
/// Provides administrative endpoints for user management, API keys, JWKS, and server administration.
pub struct AdminRoutes;

impl AdminRoutes {
    /// Create all admin routes (Axum)
    pub fn routes(context: AdminApiContext) -> Router {
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
            .route("/admin/provision", post(api_keys::handle_provision_api_key))
            .route("/admin/revoke", post(api_keys::handle_revoke_api_key))
            .route("/admin/list", get(api_keys::handle_list_api_keys))
            .route("/admin/token-info", get(api_keys::handle_token_info))
            .with_state(context)
    }

    /// User management routes (Axum)
    fn user_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/users", get(users::handle_list_users))
            .route("/admin/pending-users", get(users::handle_pending_users))
            .route(
                "/admin/approve-user/:user_id",
                post(users::handle_approve_user),
            )
            .route(
                "/admin/suspend-user/:user_id",
                post(users::handle_suspend_user),
            )
            .route(
                "/admin/users/:user_id/reset-password",
                post(users::handle_reset_user_password),
            )
            .route(
                "/admin/users/:user_id/rate-limit",
                get(users::handle_get_user_rate_limit),
            )
            .route(
                "/admin/users/:user_id/activity",
                get(users::handle_get_user_activity),
            )
            .route("/admin/users/:user_id", delete(users::handle_delete_user))
            .with_state(context)
    }

    /// System settings routes (Axum)
    fn settings_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route(
                "/admin/settings/auto-approval",
                get(settings::handle_get_auto_approval),
            )
            .route(
                "/admin/settings/auto-approval",
                put(settings::handle_set_auto_approval),
            )
            .route(
                "/admin/settings/social-insights",
                get(settings::handle_get_social_insights_config),
            )
            .route(
                "/admin/settings/social-insights",
                put(settings::handle_set_social_insights_config),
            )
            .route(
                "/admin/settings/social-insights",
                delete(settings::handle_reset_social_insights_config),
            )
            .with_state(context)
    }

    /// Setup routes (Axum)
    fn setup_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/setup", post(setup::handle_admin_setup))
            .route("/admin/setup/status", get(setup::handle_setup_status))
            .route("/admin/health", get(setup::handle_health))
            .with_state(context)
    }

    /// Admin token management routes (Axum)
    fn admin_token_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route("/admin/tokens", post(tokens::handle_create_admin_token))
            .route("/admin/tokens", get(tokens::handle_list_admin_tokens))
            .route(
                "/admin/tokens/:token_id",
                get(tokens::handle_get_admin_token),
            )
            .route(
                "/admin/tokens/:token_id/revoke",
                post(tokens::handle_revoke_admin_token),
            )
            .route(
                "/admin/tokens/:token_id/rotate",
                post(tokens::handle_rotate_admin_token),
            )
            .with_state(context)
    }

    /// Store review queue routes for admin coach approval (Axum)
    fn store_review_routes(context: Arc<AdminApiContext>) -> Router {
        Router::new()
            .route(
                "/admin/store/pending",
                get(store::handle_list_pending_coaches),
            )
            .route(
                "/admin/store/coaches/:coach_id/approve",
                post(store::handle_approve_coach),
            )
            .route(
                "/admin/store/coaches/:coach_id/reject",
                post(store::handle_reject_coach),
            )
            .with_state(context)
    }
}
