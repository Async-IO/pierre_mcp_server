// ABOUTME: Admin authentication and authorization system for privileged operations
// ABOUTME: Validates admin JWT tokens, enforces permissions, and tracks admin token usage
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Admin Authentication and Authorization
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc resource sharing for admin auth services
// - String ownership for JWT claims and token data
//!
//! This module provides authentication and authorization functionality for admin services.

use crate::admin::{
    jwt::AdminJwtManager,
    models::{AdminPermission, AdminTokenUsage, ValidatedAdminToken},
};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Admin authentication service
#[derive(Clone)]
pub struct AdminAuthService {
    database: Database,
    jwt_manager: AdminJwtManager,
    // TTL cache for validated tokens with automatic expiration
    token_cache:
        Arc<tokio::sync::RwLock<HashMap<String, (ValidatedAdminToken, std::time::Instant)>>>,
}

impl AdminAuthService {
    /// Create new admin auth service
    #[must_use]
    pub fn new(database: Database, jwt_secret: &str) -> Self {
        Self {
            database,
            jwt_manager: AdminJwtManager::with_secret(jwt_secret),
            token_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Authenticate admin token and check permissions
    ///
    /// # Errors
    /// Returns an error if:
    /// - Token is invalid or malformed
    /// - Token is not found in database
    /// - Token is inactive or expired
    /// - Token hash verification fails
    /// - Required permissions are not granted
    /// - Database operations fail
    pub async fn authenticate_and_authorize(
        &self,
        token: &str,
        required_permission: AdminPermission,
        ip_address: Option<&str>,
    ) -> Result<ValidatedAdminToken> {
        // Step 1: Validate JWT structure and extract token ID
        let validated_token = self.jwt_manager.validate_token(token)?;

        // Step 2: Check if token exists and is active in database
        let stored_token = self
            .database
            .get_admin_token_by_id(&validated_token.token_id)
            .await?
            .with_context(|| {
                format!(
                    "Admin token with ID {} not found in database",
                    validated_token.token_id
                )
            })?;

        if !stored_token.is_active {
            return Err(
                anyhow!("Authentication failed: Admin token is inactive").context(format!(
                    "Token ID {} has been deactivated",
                    validated_token.token_id
                )),
            );
        }

        // Step 3: Verify token hash
        if !AdminJwtManager::verify_token_hash(token, &stored_token.token_hash)? {
            return Err(anyhow!("Authentication failed: Invalid token hash")
                .context("Token hash verification failed - token may be tampered with"));
        }

        // Step 4: Check expiration
        if let Some(expires_at) = stored_token.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(anyhow!("Authentication failed: Admin token has expired")
                    .context(format!("Token expired at {expires_at}")));
            }
        }

        // Step 5: Check permissions
        if !stored_token
            .permissions
            .has_permission(&required_permission)
        {
            return Err(
                anyhow!("Authorization failed: Insufficient permissions").context(format!(
                    "Required permission: {:?}, token has: {:?}",
                    required_permission, stored_token.permissions
                )),
            );
        }

        // Step 6: Log usage
        self.log_token_usage(
            &stored_token.id,
            &format!("auth_check_{required_permission:?}"),
            None,
            ip_address,
            true,
            None,
        )
        .await?;

        // Step 7: Update cache
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(
                validated_token.token_id.clone(),
                (validated_token.clone(), std::time::Instant::now()),
            );
        }

        info!(
            "Admin authentication successful: service={}, permission={:?}",
            validated_token.service_name, required_permission
        );

        Ok(validated_token)
    }

    /// Fast authentication check using cache
    ///
    /// # Errors
    /// Returns an error if:
    /// - Token extraction fails
    /// - Full authentication fails when cache misses
    pub async fn quick_auth_check(
        &self,
        token: &str,
        required_permission: AdminPermission,
    ) -> Result<ValidatedAdminToken> {
        // Try cache first
        let token_id = self.jwt_manager.extract_token_id(token)?;

        {
            let cache = self.token_cache.read().await;
            if let Some((cached_token, _timestamp)) = cache.get(&token_id) {
                if cached_token
                    .permissions
                    .has_permission(&required_permission)
                {
                    return Ok(cached_token.clone());
                }
            }
        }

        // Cache miss - do full authentication
        self.authenticate_and_authorize(token, required_permission, None)
            .await
    }

    /// Log admin token usage for audit trail
    ///
    /// # Errors
    /// Returns an error if database recording fails
    pub async fn log_token_usage(
        &self,
        admin_token_id: &str,
        action: &str,
        target_resource: Option<&str>,
        ip_address: Option<&str>,
        success: bool,
        error_message: Option<&str>,
    ) -> Result<()> {
        let usage = AdminTokenUsage {
            id: None,
            admin_token_id: admin_token_id.to_string(),
            timestamp: chrono::Utc::now(),
            action: action
                .parse()
                .unwrap_or(crate::admin::models::AdminAction::ProvisionKey),
            target_resource: target_resource.map(std::string::ToString::to_string),
            ip_address: ip_address.map(std::string::ToString::to_string),
            user_agent: None, // Optional user agent information
            request_size_bytes: None,
            success,
            error_message: error_message.map(std::string::ToString::to_string),
            response_time_ms: None,
        };

        self.database.record_admin_token_usage(&usage).await?;
        Ok(())
    }

    /// Invalidate token cache (call when token is revoked)
    pub async fn invalidate_cache(&self, token_id: &str) {
        {
            let mut cache = self.token_cache.write().await;
            cache.remove(token_id);
        }
        info!("Invalidated admin token cache for: {token_id}");
    }

    /// Clear all cached tokens
    pub async fn clear_cache(&self) {
        {
            let mut cache = self.token_cache.write().await;
            cache.clear();
        }
        info!("Cleared admin token cache");
    }

    /// Get JWT manager for token operations
    #[must_use]
    pub const fn jwt_manager(&self) -> &AdminJwtManager {
        &self.jwt_manager
    }
}

/// Admin authentication middleware for HTTP requests
pub mod middleware {
    use super::{AdminAuthService, AdminPermission, ValidatedAdminToken};
    use crate::utils::auth::extract_bearer_token_owned;
    use tracing::warn;
    use warp::{Filter, Rejection};

    /// Create admin authentication filter
    #[must_use]
    pub fn admin_auth(
        auth_service: AdminAuthService,
        required_permission: AdminPermission,
    ) -> impl Filter<Extract = (ValidatedAdminToken,), Error = Rejection> + Clone {
        warp::header::<String>("authorization").and_then(move |auth_header: String| {
            let auth_service = auth_service.clone(); // Safe: Arc clone for async closure
            let required_permission = required_permission.clone(); // Safe: AdminPermission clone for async closure

            async move {
                // Extract Bearer token
                let token = extract_bearer_token_owned(&auth_header)
                    .map_err(|_| warp::reject::custom(AdminAuthError::InvalidAuthHeader))?;

                // Authenticate and authorize
                auth_service
                    .authenticate_and_authorize(&token, required_permission, None)
                    .await
                    .map_err(|e| {
                        warn!("Admin authentication failed: {}", e);
                        warp::reject::custom(AdminAuthError::AuthenticationFailed(e.to_string()))
                    })
            }
        })
    }

    /// Admin authentication errors
    #[derive(Debug, thiserror::Error)]
    pub enum AdminAuthError {
        #[error("Invalid authentication header")]
        InvalidAuthHeader,
        #[error("Authentication failed: {0}")]
        AuthenticationFailed(String),
    }

    impl warp::reject::Reject for AdminAuthError {}

    /// Convert admin auth errors to HTTP responses
    ///
    /// # Errors
    /// This function is infallible and always returns `Ok`
    pub fn handle_admin_auth_rejection(
        err: &Rejection,
    ) -> Result<impl warp::Reply, std::convert::Infallible> {
        if matches!(err.find(), Some(AdminAuthError::InvalidAuthHeader)) {
            Ok(warp::reply::with_status(
                "Invalid Authorization header".into(),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        } else if let Some(AdminAuthError::AuthenticationFailed(msg)) = err.find() {
            Ok(warp::reply::with_status(
                format!("Authentication failed: {msg}"),
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        } else {
            Ok(warp::reply::with_status(
                "Internal server error".into(),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}
