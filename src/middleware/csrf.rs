// ABOUTME: CSRF validation middleware for state-changing HTTP requests
// ABOUTME: Validates X-CSRF-Token header against user-scoped tokens to prevent request forgery
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! CSRF validation middleware
//!
//! This middleware validates CSRF tokens for state-changing operations (POST, PUT, DELETE, PATCH).
//! It extracts the token from the X-CSRF-Token header and validates it against the user's session.

use crate::errors::{AppError, AppResult};
use crate::mcp::resources::ServerResources;
use crate::security::cookies::get_cookie_value;
use crate::security::csrf::CsrfTokenManager;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, Method, Request};
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use tracing::{debug, warn};
use uuid::Uuid;

/// CSRF validation middleware
#[derive(Clone)]
pub struct CsrfMiddleware {
    csrf_manager: Arc<CsrfTokenManager>,
}

impl CsrfMiddleware {
    /// Create new CSRF middleware
    #[must_use]
    pub const fn new(csrf_manager: Arc<CsrfTokenManager>) -> Self {
        Self { csrf_manager }
    }

    /// Validate CSRF token for state-changing requests
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - CSRF token header is missing for state-changing request
    /// - CSRF token is invalid or expired
    /// - CSRF token doesn't match the user
    pub async fn validate_csrf(
        &self,
        headers: &HeaderMap,
        method: &Method,
        user_id: Uuid,
    ) -> AppResult<()> {
        // Only validate for state-changing operations
        if !matches!(
            method,
            &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
        ) {
            // GET, HEAD, OPTIONS, etc. don't need CSRF protection
            return Ok(());
        }

        // Extract CSRF token from header
        let csrf_token = headers
            .get("X-CSRF-Token")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                warn!(
                    user_id = %user_id,
                    method = %method,
                    "CSRF token missing for state-changing request"
                );
                AppError::auth_invalid("CSRF token required for this operation")
            })?;

        // Validate token
        self.csrf_manager
            .validate_token(csrf_token, user_id)
            .await
            .map_err(|e| {
                warn!(
                    user_id = %user_id,
                    method = %method,
                    error = %e,
                    "CSRF token validation failed"
                );
                e
            })?;

        debug!(
            user_id = %user_id,
            method = %method,
            "CSRF token validated successfully"
        );

        Ok(())
    }

    /// Check if request requires CSRF validation
    #[must_use]
    pub const fn requires_csrf_validation(method: &Method) -> bool {
        matches!(
            method,
            &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH
        )
    }
}

/// Paths exempt from CSRF validation. These are either pre-authentication
/// (login, register) or session-teardown (logout) endpoints where the client
/// may not yet have -- or has already discarded -- a CSRF token.
const CSRF_EXEMPT_PATHS: &[&str] = &[
    "/oauth/token",
    "/api/auth/logout",
    "/api/auth/register",
    "/api/auth/firebase",
    "/api/auth/refresh",
];

/// Axum middleware layer for CSRF validation on cookie-authenticated requests.
///
/// This middleware enforces CSRF protection for state-changing HTTP methods
/// (POST, PUT, DELETE, PATCH) when the request is authenticated via cookies.
/// Requests using Bearer tokens or API keys (programmatic clients) bypass
/// CSRF validation since they are not susceptible to cross-site request forgery.
///
/// Auth endpoints (login, logout, register) are exempt since they operate
/// before or after a valid session exists.
///
/// # Errors
///
/// Returns 401 if a cookie-authenticated state-changing request lacks a valid
/// X-CSRF-Token header.
pub async fn csrf_protection_layer(
    State(resources): State<Arc<ServerResources>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let headers = request.headers().clone();
    let path = request.uri().path();

    // Only validate state-changing methods
    if !CsrfMiddleware::requires_csrf_validation(&method) {
        return Ok(next.run(request).await);
    }

    // Skip CSRF for auth endpoints that operate before or after a session
    if CSRF_EXEMPT_PATHS.contains(&path) {
        return Ok(next.run(request).await);
    }

    // Only enforce CSRF for cookie-authenticated requests (browser clients).
    // Programmatic clients using Bearer tokens or API keys are not vulnerable
    // to CSRF and should not be required to send CSRF tokens.
    let has_bearer_or_api_key = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .is_some_and(|v| v.starts_with("Bearer ") || v.starts_with("Api-Key "));

    if has_bearer_or_api_key {
        return Ok(next.run(request).await);
    }

    // Check if request has cookie auth; skip CSRF for non-cookie requests
    let Some(auth_token) = get_cookie_value(&headers, "auth_token") else {
        return Ok(next.run(request).await);
    };

    // Extract user_id from the cookie JWT to validate CSRF token ownership.
    // If the cookie JWT is invalid (expired, stale RSA key, etc.), treat the
    // request as unauthenticated — there is no valid session to protect with
    // CSRF. The actual endpoint handler will perform its own authentication.
    let Ok(claims) = resources
        .auth_manager
        .validate_token(&auth_token, &resources.jwks_manager)
    else {
        debug!("Stale or invalid auth cookie in CSRF check, treating as unauthenticated");
        return Ok(next.run(request).await);
    };

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::auth_invalid("Invalid user ID in authentication token"))?;

    // Validate CSRF token
    resources
        .csrf_middleware
        .validate_csrf(&headers, &method, user_id)
        .await?;

    Ok(next.run(request).await)
}
