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
use crate::database_plugins::factory::Database;
use crate::errors::{AppError, AppResult};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

/// Admin authentication service
#[derive(Clone)]
pub struct AdminAuthService {
    database: Database,
    jwt_manager: AdminJwtManager,
    jwks_manager: Arc<crate::admin::jwks::JwksManager>,
    // TTL cache for validated tokens with automatic expiration
    token_cache:
        Arc<tokio::sync::RwLock<HashMap<String, (ValidatedAdminToken, std::time::Instant)>>>,
    // Cache TTL in seconds (default: 300 seconds = 5 minutes)
    cache_ttl: std::time::Duration,
}

impl AdminAuthService {
    /// Default cache TTL (5 minutes)
    const DEFAULT_CACHE_TTL_SECS: u64 = 300;

    /// Create new admin auth service with RS256 (REQUIRED)
    #[must_use]
    pub fn new(database: Database, jwks_manager: Arc<crate::admin::jwks::JwksManager>) -> Self {
        let cache_ttl_secs = std::env::var("ADMIN_TOKEN_CACHE_TTL_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(Self::DEFAULT_CACHE_TTL_SECS);

        Self {
            database,
            jwt_manager: AdminJwtManager::new(),
            jwks_manager,
            token_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            cache_ttl: std::time::Duration::from_secs(cache_ttl_secs),
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
    ) -> AppResult<ValidatedAdminToken> {
        self.authenticate(token, ip_address)
            .await
            .and_then(|validated_token| {
                // Check permissions
                let stored_token = validated_token.clone();
                if stored_token
                    .permissions
                    .has_permission(&required_permission)
                {
                    Ok(validated_token)
                } else {
                    Err(AppError::new(
                        crate::errors::ErrorCode::PermissionDenied,
                        format!(
                            "Required permission: {:?}, token has: {:?}",
                            required_permission, stored_token.permissions
                        ),
                    ))
                }
            })
    }

    /// Authenticate admin token without checking permissions
    ///
    /// Validates the JWT token and checks if it exists in the database,
    /// but does NOT enforce any permission requirements. Handlers should
    /// check permissions themselves using `has_permission()` on the validated token.
    ///
    /// # Errors
    /// Returns error if:
    /// - JWT validation fails
    /// - Token not found in database
    /// - Token is inactive or expired
    /// - Token hash verification fails
    pub async fn authenticate(
        &self,
        token: &str,
        ip_address: Option<&str>,
    ) -> AppResult<ValidatedAdminToken> {
        // Step 1: Validate JWT structure and extract token ID using RS256
        let validated_token = self.jwt_manager.validate_token(token, &self.jwks_manager)?;

        // Step 2: Check if token exists and is active in database
        let stored_token = self
            .database
            .get_admin_token_by_id(&validated_token.token_id)
            .await?
            .ok_or_else(|| {
                AppError::auth_invalid(format!(
                    "Admin token with ID {} not found in database",
                    validated_token.token_id
                ))
            })?;

        if !stored_token.is_active {
            return Err(AppError::auth_invalid("Admin token is inactive"));
        }

        // Step 3: Verify token hash
        if !AdminJwtManager::verify_token_hash(token, &stored_token.token_hash)? {
            return Err(AppError::auth_invalid(
                "Invalid token hash - token may be tampered with",
            ));
        }

        // Step 4: Check expiration
        if let Some(expires_at) = stored_token.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(AppError::auth_expired());
            }
        }

        // Step 5: Log usage (no permission check)
        self.log_token_usage(&stored_token.id, "auth_check", None, ip_address, true, None)
            .await?;

        // Step 6: Update cache
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(
                validated_token.token_id.clone(),
                (validated_token.clone(), std::time::Instant::now()),
            );
        }

        info!(
            "Admin authentication successful: service={}",
            validated_token.service_name
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
    ) -> AppResult<ValidatedAdminToken> {
        // Validate token to extract token_id for cache lookup
        let validated_token = self.jwt_manager.validate_token(token, &self.jwks_manager)?;

        // Try cache first with TTL check
        {
            let mut cache = self.token_cache.write().await;
            if let Some((cached_token, timestamp)) = cache.get(&validated_token.token_id) {
                // Check if cache entry is still valid (within TTL)
                if timestamp.elapsed() < self.cache_ttl {
                    if cached_token
                        .permissions
                        .has_permission(&required_permission)
                    {
                        let result = cached_token.clone();
                        drop(cache);
                        return Ok(result);
                    }
                } else {
                    // Expired - remove it
                    cache.remove(&validated_token.token_id);
                    drop(cache);
                    tracing::debug!(
                        "Removed expired admin token from cache: {}",
                        validated_token.token_id
                    );
                }
            }
        }

        // Cache miss or expired - do full authentication
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
    ) -> AppResult<()> {
        let usage = AdminTokenUsage {
            id: None,
            admin_token_id: admin_token_id.to_owned(),
            timestamp: chrono::Utc::now(),
            action: action
                .parse()
                .unwrap_or(crate::admin::models::AdminAction::ProvisionKey),
            target_resource: target_resource.map(str::to_owned),
            ip_address: ip_address.map(str::to_owned),
            user_agent: None, // Optional user agent information
            request_size_bytes: None,
            success,
            error_message: error_message.map(str::to_owned),
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

/// Admin authentication middleware for Axum
pub mod middleware {
    use super::AdminAuthService;
    use crate::utils::auth::extract_bearer_token_owned;
    use axum::{
        body::Body,
        extract::State,
        http::{Request, StatusCode},
        middleware::Next,
        response::{IntoResponse, Response},
        Json,
    };
    use serde_json::json;
    use tracing::warn;

    /// Axum middleware for admin authentication
    ///
    /// Extracts Bearer token from Authorization header, validates it,
    /// and adds `ValidatedAdminToken` as a request extension.
    ///
    /// # Errors
    /// Returns error if authorization header is missing, malformed, or token is invalid
    pub async fn admin_auth_middleware(
        State(auth_service): State<AdminAuthService>,
        mut request: Request<Body>,
        next: Next,
    ) -> Result<Response, Response> {
        // Extract authorization header
        let auth_header = request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| {
                warn!("Missing Authorization header in admin request");
                (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "success": false,
                        "message": "Missing Authorization header"
                    })),
                )
                    .into_response()
            })?;

        // Extract Bearer token
        let token = extract_bearer_token_owned(auth_header).map_err(|e| {
            warn!(error = %e, "Failed to extract bearer token from admin auth header");
            (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "success": false,
                    "message": "Invalid Authorization header format"
                })),
            )
                .into_response()
        })?;

        // Authenticate token without checking permissions
        // Each handler will check its own required permissions
        let validated_token = auth_service.authenticate(&token, None).await.map_err(|e| {
            warn!(error = %e, "Admin authentication failed");
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "success": false,
                    "message": format!("Authentication failed: {}", e)
                })),
            )
                .into_response()
        })?;

        // Insert validated token as extension
        request.extensions_mut().insert(validated_token);

        // Continue to next middleware/handler
        Ok(next.run(request).await)
    }
}
