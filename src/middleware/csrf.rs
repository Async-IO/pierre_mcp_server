// ABOUTME: CSRF validation middleware for state-changing HTTP requests
// ABOUTME: Validates X-CSRF-Token header against user-scoped tokens to prevent request forgery
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! CSRF validation middleware
//!
//! This middleware validates CSRF tokens for state-changing operations (POST, PUT, DELETE, PATCH).
//! It extracts the token from the X-CSRF-Token header and validates it against the user's session.

use crate::errors::{AppError, AppResult};
use crate::security::csrf::CsrfTokenManager;
use axum::http::{HeaderMap, Method};
use std::sync::Arc;
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
                tracing::warn!(
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
                tracing::warn!(
                    user_id = %user_id,
                    method = %method,
                    error = %e,
                    "CSRF token validation failed"
                );
                e
            })?;

        tracing::debug!(
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
