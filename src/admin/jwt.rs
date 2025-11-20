// ABOUTME: JWT token generation, validation, and management for admin authentication
// ABOUTME: Creates secure admin JWT tokens with permissions and validates token claims for authorization
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! JWT Token Generation and Validation for Admin Authentication
//!
//! This module provides secure JWT token generation and validation for admin services.
//! Tokens are signed with strong secrets and include proper claims for authorization.

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - String ownership for JWT claims extraction and token validation
// - Permission data cloning for validated token construction

use crate::admin::models::{AdminPermissions, ValidatedAdminToken};
use crate::constants::service_names;
use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// JWT token manager for admin authentication
#[derive(Clone)]
pub struct AdminJwtManager {}

impl Default for AdminJwtManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminJwtManager {
    /// Create new JWT manager for RS256 token operations
    #[must_use]
    pub const fn new() -> Self {
        Self {}
    }

    /// Generate a cryptographically secure JWT secret
    #[must_use]
    pub fn generate_jwt_secret() -> String {
        // Generate 64 character (512-bit) random secret
        rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(64)
            .map(char::from)
            .collect()
    }

    /// Hash a JWT secret for storage
    #[must_use]
    pub fn hash_secret(secret: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Generate JWT token using RS256 (asymmetric signing)
    ///
    /// # Errors
    /// Returns an error if JWT encoding fails
    pub fn generate_token(
        &self,
        token_id: &str,
        service_name: &str,
        permissions: &AdminPermissions,
        is_super_admin: bool,
        expires_at: Option<DateTime<Utc>>,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> AppResult<String> {
        let now = Utc::now();
        let exp = expires_at.unwrap_or_else(|| now + Duration::days(365));

        let claims = AdminTokenClaims {
            // Standard JWT claims
            iss: service_names::PIERRE_MCP_SERVER.into(),
            sub: token_id.to_owned(),
            aud: service_names::ADMIN_API.into(),
            exp: u64::try_from(exp.timestamp().max(0)).unwrap_or(0),
            iat: u64::try_from(now.timestamp().max(0)).unwrap_or(0),
            nbf: u64::try_from(now.timestamp().max(0)).unwrap_or(0),
            jti: token_id.to_owned(),

            // Custom claims
            service_name: service_name.to_owned(),
            permissions: permissions.to_vec(),
            is_super_admin,
            token_type: "admin".into(),
        };

        // Sign with RS256 using JWKS
        jwks_manager
            .sign_admin_token(&claims)
            .map_err(|e| AppError::internal(format!("Failed to generate RS256 admin JWT: {e}")))
    }

    /// Validate and decode JWT token using RS256
    ///
    /// # Errors
    /// Returns an error if token is invalid, expired, or has wrong format
    pub fn validate_token(
        &self,
        token: &str,
        jwks_manager: &crate::admin::jwks::JwksManager,
    ) -> AppResult<ValidatedAdminToken> {
        // Verify RS256 signature and decode claims
        let claims: AdminTokenClaims = jwks_manager.verify_admin_token(token).map_err(|e| {
            AppError::auth_invalid(format!("RS256 admin JWT validation failed: {e}"))
        })?;

        // Verify token type
        if claims.token_type != "admin" {
            return Err(AppError::auth_invalid(format!(
                "Invalid token type: {}",
                claims.token_type
            )));
        }

        // Check expiration
        let now = u64::try_from(Utc::now().timestamp().max(0)).unwrap_or(0);
        if claims.exp < now {
            return Err(AppError::auth_expired());
        }

        // Reconstruct permissions
        let permissions = AdminPermissions::new(claims.permissions.clone()); // Safe: Vec<String> ownership for permissions

        let token_id = claims.sub.clone(); // Safe: String ownership for token validation
        let service_name = claims.service_name.clone(); // Safe: String ownership for token validation
        let is_super_admin = claims.is_super_admin;
        let user_info = serde_json::to_value(&claims)
            .map_err(|e| AppError::internal(format!("JSON serialization failed: {e}")))?;

        Ok(ValidatedAdminToken {
            token_id,
            service_name,
            permissions,
            is_super_admin,
            user_info: Some(user_info),
        })
    }

    /// Generate token prefix for identification
    #[must_use]
    pub fn generate_token_prefix(token: &str) -> String {
        let token_prefix = &token[..8];
        format!("admin_jwt_{token_prefix}")
    }

    /// Hash token for storage (bcrypt-compatible)
    ///
    /// # Errors
    /// Returns an error if bcrypt hashing fails
    pub fn hash_token_for_storage(token: &str) -> AppResult<String> {
        bcrypt::hash(token, bcrypt::DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("Failed to hash token: {e}")))
    }

    /// Verify token hash
    ///
    /// # Errors
    /// Returns an error if bcrypt verification fails
    pub fn verify_token_hash(token: &str, hash: &str) -> AppResult<bool> {
        bcrypt::verify(token, hash)
            .map_err(|e| AppError::internal(format!("Failed to verify token hash: {e}")))
    }
}

/// JWT claims for admin tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdminTokenClaims {
    // Standard JWT claims
    iss: String, // Issuer: "pierre-mcp-server"
    sub: String, // Subject: token ID
    aud: String, // Audience: "admin-api"
    exp: u64,    // Expiration time
    iat: u64,    // Issued at
    nbf: u64,    // Not before
    jti: String, // JWT ID: token ID

    // Custom claims
    service_name: String,
    permissions: Vec<crate::admin::models::AdminPermission>,
    is_super_admin: bool,
    token_type: String, // Always "admin"
}

/// Token generation configuration
#[derive(Debug, Clone)]
pub struct TokenGenerationConfig {
    /// Service name for the token
    pub service_name: String,
    /// Optional human-readable description
    pub service_description: Option<String>,
    /// Permissions granted to this token
    pub permissions: Option<AdminPermissions>,
    /// Token expiration in days (None for no expiration)
    pub expires_in_days: Option<u64>,
    /// Whether this is a super admin token with full privileges
    pub is_super_admin: bool,
}

impl TokenGenerationConfig {
    /// Create config for regular admin token
    #[must_use]
    pub fn regular_admin(service_name: String) -> Self {
        Self {
            service_name,
            service_description: None,
            permissions: Some(AdminPermissions::default_admin()),
            expires_in_days: Some(365), // 1 year
            is_super_admin: false,
        }
    }

    /// Create config for super admin token
    #[must_use]
    pub fn super_admin(service_name: String) -> Self {
        Self {
            service_name,
            service_description: Some("Super Admin Token".into()),
            permissions: Some(AdminPermissions::super_admin()),
            expires_in_days: None, // Never expires
            is_super_admin: true,
        }
    }

    /// Get effective permissions
    #[must_use]
    pub fn get_permissions(&self) -> AdminPermissions {
        self.permissions.as_ref().map_or_else(
            || {
                if self.is_super_admin {
                    AdminPermissions::super_admin()
                } else {
                    AdminPermissions::default_admin()
                }
            },
            std::clone::Clone::clone,
        )
    }

    /// Get expiration date
    #[must_use]
    pub fn get_expiration(&self) -> Option<DateTime<Utc>> {
        self.expires_in_days
            .map(|days| Utc::now() + Duration::days(i64::try_from(days).unwrap_or(365)))
    }
}
