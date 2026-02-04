// ABOUTME: Tower middleware for extracting tenant context from JWT claims
// ABOUTME: Injects TenantContext into request extensions for route handlers
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tenant Context Middleware
//!
//! This middleware extracts tenant context from JWT tokens and injects it
//! into Axum request extensions. Route handlers can then access the tenant
//! context without re-validating the JWT token.
//!
//! # Design
//!
//! The middleware extracts tenant information from:
//! 1. The `active_tenant_id` claim in the JWT token (primary)
//! 2. The `x-tenant-id` HTTP header (fallback for explicit tenant selection)
//! 3. The user's default tenant from the database (if no explicit tenant)
//!
//! # Usage
//!
//! ```rust,no_run
//! use axum::{Router, routing::get, Extension};
//! use pierre_mcp_server::middleware::tenant::{tenant_context_middleware, ExtractedTenantContext};
//! use pierre_mcp_server::tenant::TenantContext;
//!
//! async fn handler(
//!     Extension(tenant_ctx): Extension<ExtractedTenantContext>,
//! ) -> String {
//!     match tenant_ctx.0 {
//!         Some(ctx) => format!("Tenant: {}", ctx.tenant_name),
//!         None => "No tenant context".to_owned(),
//!     }
//! }
//! ```

use crate::auth::Claims;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::AppError;
use crate::mcp::resources::ServerResources;
use crate::security::cookies::get_cookie_value;
use crate::tenant::{TenantContext, TenantRole};
use crate::utils::uuid::parse_uuid;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

/// Extracted tenant context wrapper for request extensions
///
/// This wrapper is inserted into request extensions by the middleware.
/// It contains `Option<TenantContext>` because:
/// - Some routes are public and don't require authentication
/// - Some routes have optional authentication
/// - Authentication or tenant extraction may fail gracefully
#[derive(Debug, Clone)]
pub struct ExtractedTenantContext(pub Option<TenantContext>);

impl ExtractedTenantContext {
    /// Get the tenant context if available
    #[must_use]
    pub const fn get(&self) -> Option<&TenantContext> {
        self.0.as_ref()
    }

    /// Check if tenant context is present
    #[must_use]
    pub const fn is_present(&self) -> bool {
        self.0.is_some()
    }

    /// Get the tenant ID if available
    #[must_use]
    pub fn tenant_id(&self) -> Option<Uuid> {
        self.0.as_ref().map(|ctx| ctx.tenant_id)
    }

    /// Get the user ID if available
    #[must_use]
    pub fn user_id(&self) -> Option<Uuid> {
        self.0.as_ref().map(|ctx| ctx.user_id)
    }
}

/// Tenant context middleware that extracts tenant information from JWT claims
///
/// This middleware:
/// 1. Extracts JWT token from Authorization header or `auth_token` cookie
/// 2. Validates the token and extracts claims
/// 3. Resolves tenant context from JWT claims or user's default tenant
/// 4. Injects `ExtractedTenantContext` into request extensions
///
/// The middleware does NOT reject requests without valid authentication.
/// Instead, it injects `ExtractedTenantContext(None)` for unauthenticated
/// requests. Route handlers can then decide whether to require authentication.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{Router, routing::get, middleware};
/// use pierre_mcp_server::middleware::tenant::tenant_context_middleware;
/// use pierre_mcp_server::mcp::resources::ServerResources;
/// use std::sync::Arc;
///
/// # async fn handler() -> &'static str { "" }
/// # fn example(resources: Arc<ServerResources>) {
/// let app: Router<Arc<ServerResources>> = Router::new()
///     .route("/", get(handler))
///     .layer(middleware::from_fn_with_state(resources.clone(), tenant_context_middleware));
/// # }
/// ```
pub async fn tenant_context_middleware(
    State(resources): State<Arc<ServerResources>>,
    mut req: Request,
    next: Next,
) -> Response {
    let headers = req.headers();

    // Try to extract JWT token from cookie first (web clients)
    let token = get_cookie_value(headers, "auth_token").or_else(|| {
        // Fall back to Authorization header (API clients)
        headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|auth| auth.strip_prefix("Bearer "))
            .map(ToOwned::to_owned)
    });

    let tenant_context = if let Some(token) = token {
        extract_tenant_from_token(&token, &resources).await
    } else {
        debug!("No authentication token found, proceeding without tenant context");
        None
    };

    // Record tenant context in tracing span
    if let Some(ref ctx) = tenant_context {
        tracing::Span::current()
            .record("tenant_id", ctx.tenant_id.to_string())
            .record("tenant_user_id", ctx.user_id.to_string());
    }

    // Insert tenant context into request extensions
    req.extensions_mut()
        .insert(ExtractedTenantContext(tenant_context));

    next.run(req).await
}

/// Extract tenant context from a validated JWT token
///
/// This function:
/// 1. Validates the JWT token
/// 2. Extracts the user ID from claims
/// 3. Resolves tenant ID from `active_tenant_id` claim or user's default
/// 4. Fetches tenant details and user role from database
async fn extract_tenant_from_token(
    token: &str,
    resources: &Arc<ServerResources>,
) -> Option<TenantContext> {
    // Validate token and extract claims
    let claims = match resources
        .auth_manager
        .validate_token(token, &resources.jwks_manager)
    {
        Ok(claims) => claims,
        Err(e) => {
            debug!("JWT validation failed in tenant middleware: {}", e);
            return None;
        }
    };

    // Parse user ID from claims
    let user_id = match parse_uuid(&claims.sub) {
        Ok(id) => id,
        Err(e) => {
            warn!(sub = %claims.sub, error = %e, "Invalid user ID in JWT claims");
            return None;
        }
    };

    // Resolve tenant ID and build context
    let database = &resources.database;
    let tenant_id = resolve_tenant_id_from_claims(&claims, user_id, database).await?;
    build_tenant_context(tenant_id, user_id, database).await
}

/// Resolve the tenant ID from JWT claims or fall back to user's default tenant
///
/// Priority order:
/// 1. `active_tenant_id` from JWT claims (verified against membership)
/// 2. User's default tenant from `tenant_users` table
async fn resolve_tenant_id_from_claims(
    claims: &Claims,
    user_id: Uuid,
    database: &Arc<Database>,
) -> Option<Uuid> {
    if let Some(tenant_id_str) = claims.active_tenant_id.as_deref() {
        resolve_explicit_tenant_id(tenant_id_str, user_id, database).await
    } else {
        // No tenant ID in claims, get user's default tenant
        get_user_default_tenant(user_id, database).await
    }
}

/// Resolve an explicitly specified tenant ID from JWT claims
///
/// Verifies the user belongs to the tenant before accepting it.
async fn resolve_explicit_tenant_id(
    tenant_id_str: &str,
    user_id: Uuid,
    database: &Arc<Database>,
) -> Option<Uuid> {
    let Some(tid) = parse_tenant_id(tenant_id_str) else {
        return get_user_default_tenant(user_id, database).await;
    };

    verify_tenant_membership(user_id, tid, database).await
}

/// Parse tenant ID string into UUID, logging errors
fn parse_tenant_id(tenant_id_str: &str) -> Option<Uuid> {
    tenant_id_str.parse::<Uuid>().map_or_else(
        |e| {
            warn!(
                tenant_id = %tenant_id_str,
                error = %e,
                "Invalid tenant ID format in JWT claims"
            );
            None
        },
        Some,
    )
}

/// Verify user belongs to tenant, falling back to default if not
async fn verify_tenant_membership(
    user_id: Uuid,
    tenant_id: Uuid,
    database: &Arc<Database>,
) -> Option<Uuid> {
    match database.get_user_tenant_role(user_id, tenant_id).await {
        Ok(Some(_)) => Some(tenant_id),
        Ok(None) => {
            warn!(
                user_id = %user_id,
                tenant_id = %tenant_id,
                "User does not belong to tenant specified in JWT claims"
            );
            get_user_default_tenant(user_id, database).await
        }
        Err(e) => {
            warn!(error = %e, "Failed to verify tenant membership");
            get_user_default_tenant(user_id, database).await
        }
    }
}

/// Build the full tenant context from a resolved tenant ID
async fn build_tenant_context(
    tenant_id: Uuid,
    user_id: Uuid,
    database: &Arc<Database>,
) -> Option<TenantContext> {
    // Fetch tenant details
    let tenant_name = fetch_tenant_name(tenant_id, database).await;

    // Fetch user's role in this tenant
    let user_role = fetch_user_role(user_id, tenant_id, database).await;

    Some(TenantContext::new(
        tenant_id,
        tenant_name,
        user_id,
        user_role,
    ))
}

/// Fetch tenant name from database, with fallback to default
async fn fetch_tenant_name(tenant_id: Uuid, database: &Arc<Database>) -> String {
    match database.get_tenant_by_id(tenant_id).await {
        Ok(tenant) => tenant.name,
        Err(e) => {
            warn!(
                tenant_id = %tenant_id,
                error = %e,
                "Failed to fetch tenant details, using default name"
            );
            "Unknown Tenant".to_owned()
        }
    }
}

/// Fetch user's role in a tenant, with fallback to Member
async fn fetch_user_role(user_id: Uuid, tenant_id: Uuid, database: &Arc<Database>) -> TenantRole {
    match database.get_user_tenant_role(user_id, tenant_id).await {
        Ok(Some(role_str)) => TenantRole::from_db_string(&role_str),
        Ok(None) => {
            warn!(
                user_id = %user_id,
                tenant_id = %tenant_id,
                "User has no role in tenant, defaulting to Member"
            );
            TenantRole::Member
        }
        Err(e) => {
            warn!(
                error = %e,
                "Failed to fetch user tenant role, defaulting to Member"
            );
            TenantRole::Member
        }
    }
}

/// Get user's default tenant from the database
async fn get_user_default_tenant(user_id: Uuid, database: &Arc<Database>) -> Option<Uuid> {
    match database.list_tenants_for_user(user_id).await {
        Ok(tenants) => {
            if tenants.is_empty() {
                debug!(user_id = %user_id, "User does not belong to any tenant");
                None
            } else {
                Some(tenants[0].id)
            }
        }
        Err(e) => {
            warn!(user_id = %user_id, error = %e, "Failed to list user tenants");
            None
        }
    }
}

/// Require tenant context extractor
///
/// Use this in route handlers that REQUIRE a valid tenant context.
/// Returns an error response if tenant context is not available.
///
/// # Errors
///
/// Returns `AppError::auth_invalid` if the tenant context is not present,
/// indicating that authentication is required but was not provided.
///
/// # Example
///
/// ```rust,no_run
/// use axum::{Extension, response::IntoResponse};
/// use pierre_mcp_server::middleware::tenant::{ExtractedTenantContext, require_tenant_context};
/// use pierre_mcp_server::errors::AppError;
///
/// async fn protected_handler(
///     Extension(tenant_ctx): Extension<ExtractedTenantContext>,
/// ) -> Result<impl IntoResponse, AppError> {
///     let ctx = require_tenant_context(&tenant_ctx)?;
///     Ok(format!("Welcome to {}", ctx.tenant_name))
/// }
/// ```
pub fn require_tenant_context(
    extracted: &ExtractedTenantContext,
) -> Result<&TenantContext, AppError> {
    extracted.get().ok_or_else(|| {
        AppError::auth_invalid("Authentication required - no valid tenant context found")
    })
}
