// ABOUTME: JWT token generation, validation, and management for admin authentication
// ABOUTME: Creates secure admin JWT tokens with permissions and validates token claims for authorization
//! JWT Token Generation and Validation for Admin Authentication
//!
//! This module provides secure JWT token generation and validation for admin services.
//! Tokens are signed with strong secrets and include proper claims for authorization.

use crate::admin::models::{AdminPermissions, ValidatedAdminToken};
use crate::database_plugins::DatabaseProvider;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// JWT token manager for admin authentication
#[derive(Clone)]
pub struct AdminJwtManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    algorithm: Algorithm,
}

impl Default for AdminJwtManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AdminJwtManager {
    /// Create new JWT manager with generated secret
    #[must_use]
    pub fn new() -> Self {
        let secret = Self::generate_jwt_secret();
        Self::with_secret(&secret)
    }

    /// Create JWT manager with provided secret
    #[must_use]
    pub fn with_secret(secret: &str) -> Self {
        let encoding_key = EncodingKey::from_secret(secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());

        Self {
            encoding_key,
            decoding_key,
            algorithm: Algorithm::HS256, // HMAC with SHA-256
        }
    }

    /// Create JWT manager with database secret
    ///
    /// # Errors
    /// Returns an error if database secret retrieval fails
    pub async fn from_database(
        database: &crate::database_plugins::factory::Database,
    ) -> Result<Self> {
        let Ok(jwt_secret) = database.get_system_secret("admin_jwt_secret").await else {
            return Err(anyhow::anyhow!(
                "Admin JWT secret not found. Run admin-setup create-admin-user first."
            ));
        };
        Ok(Self::with_secret(&jwt_secret))
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

    /// Generate JWT token for admin service
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
    ) -> Result<String> {
        let now = Utc::now();
        let exp = expires_at.unwrap_or_else(|| now + Duration::days(365));

        let claims = AdminTokenClaims {
            // Standard JWT claims
            iss: "pierre-mcp-server".into(),
            sub: token_id.to_string(),
            aud: "admin-api".into(),
            exp: u64::try_from(exp.timestamp().max(0)).unwrap_or(0),
            iat: u64::try_from(now.timestamp().max(0)).unwrap_or(0),
            nbf: u64::try_from(now.timestamp().max(0)).unwrap_or(0),
            jti: token_id.to_string(),

            // Custom claims
            service_name: service_name.to_string(),
            permissions: permissions.to_vec(),
            is_super_admin,
            token_type: "admin".into(),
        };

        let header = Header::new(self.algorithm);
        encode(&header, &claims, &self.encoding_key)
            .map_err(|e| anyhow!("Failed to generate JWT: {}", e))
    }

    /// Validate and decode JWT token
    ///
    /// # Errors
    /// Returns an error if token is invalid, expired, or has wrong format
    pub fn validate_token(&self, token: &str) -> Result<ValidatedAdminToken> {
        let mut validation = Validation::new(self.algorithm);
        validation.set_audience(&["admin-api"]);
        validation.set_issuer(&["pierre-mcp-server"]);

        let token_data = decode::<AdminTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow!("Invalid JWT token: {}", e))?;

        let claims = token_data.claims;

        // Verify token type
        if claims.token_type != "admin" {
            return Err(anyhow!("Invalid token type: {}", claims.token_type));
        }

        // Check expiration
        let now = u64::try_from(Utc::now().timestamp().max(0)).unwrap_or(0);
        if claims.exp < now {
            return Err(anyhow!("Token has expired"));
        }

        // Reconstruct permissions
        let permissions = AdminPermissions::new(claims.permissions.clone());

        let token_id = claims.sub.clone();
        let service_name = claims.service_name.clone();
        let is_super_admin = claims.is_super_admin;
        let user_info = serde_json::to_value(&claims)?;

        Ok(ValidatedAdminToken {
            token_id,
            service_name,
            permissions,
            is_super_admin,
            user_info: Some(user_info),
        })
    }

    /// Extract token ID without full validation (for prefix matching)
    ///
    /// # Errors
    /// Returns an error if token cannot be decoded
    pub fn extract_token_id(&self, token: &str) -> Result<String> {
        // Decode without verification for prefix extraction
        let mut validation = Validation::new(self.algorithm);
        validation.insecure_disable_signature_validation();
        validation.validate_exp = false;
        validation.validate_nbf = false;

        let token_data = decode::<AdminTokenClaims>(token, &self.decoding_key, &validation)
            .map_err(|e| anyhow!("Failed to extract token ID: {}", e))?;

        Ok(token_data.claims.sub)
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
    pub fn hash_token_for_storage(token: &str) -> Result<String> {
        bcrypt::hash(token, bcrypt::DEFAULT_COST)
            .map_err(|e| anyhow!("Failed to hash token: {}", e))
    }

    /// Verify token hash
    ///
    /// # Errors
    /// Returns an error if bcrypt verification fails
    pub fn verify_token_hash(token: &str, hash: &str) -> Result<bool> {
        bcrypt::verify(token, hash).map_err(|e| anyhow!("Failed to verify token hash: {}", e))
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
    pub service_name: String,
    pub service_description: Option<String>,
    pub permissions: Option<AdminPermissions>,
    pub expires_in_days: Option<u64>,
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
