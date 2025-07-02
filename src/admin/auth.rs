// ABOUTME: Admin authentication and authorization system for privileged operations
// ABOUTME: Validates admin JWT tokens, enforces permissions, and tracks admin token usage
//! Admin Authentication and Authorization
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
use tracing::{info, warn};

/// Admin authentication service
#[derive(Clone)]
pub struct AdminAuthService {
    database: Database,
    jwt_manager: AdminJwtManager,
    // Cache for validated tokens (in production, use Redis)
    token_cache: Arc<tokio::sync::RwLock<HashMap<String, ValidatedAdminToken>>>,
}

impl AdminAuthService {
    /// Create new admin auth service
    pub fn new(database: Database, jwt_secret: &str) -> Self {
        Self {
            database,
            jwt_manager: AdminJwtManager::with_secret(jwt_secret),
            token_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Authenticate admin token and check permissions
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
                    .context(format!("Token expired at {}", expires_at)));
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
            &format!("auth_check_{:?}", required_permission),
            None,
            ip_address,
            true,
            None,
        )
        .await?;

        // Step 7: Update cache
        {
            let mut cache = self.token_cache.write().await;
            cache.insert(validated_token.token_id.clone(), validated_token.clone());
        }

        info!(
            "Admin authentication successful: service={}, permission={:?}",
            validated_token.service_name, required_permission
        );

        Ok(validated_token)
    }

    /// Fast authentication check using cache
    pub async fn quick_auth_check(
        &self,
        token: &str,
        required_permission: AdminPermission,
    ) -> Result<ValidatedAdminToken> {
        // Try cache first
        let token_id = self.jwt_manager.extract_token_id(token)?;

        {
            let cache = self.token_cache.read().await;
            if let Some(cached_token) = cache.get(&token_id) {
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
            target_resource: target_resource.map(|s| s.to_string()),
            ip_address: ip_address.map(|s| s.to_string()),
            user_agent: None, // Can be added later
            request_size_bytes: None,
            success,
            error_message: error_message.map(|s| s.to_string()),
            response_time_ms: None,
        };

        self.database.record_admin_token_usage(&usage).await?;
        Ok(())
    }

    /// Invalidate token cache (call when token is revoked)
    pub async fn invalidate_cache(&self, token_id: &str) {
        let mut cache = self.token_cache.write().await;
        cache.remove(token_id);
        info!("Invalidated admin token cache for: {}", token_id);
    }

    /// Clear all cached tokens
    pub async fn clear_cache(&self) {
        let mut cache = self.token_cache.write().await;
        cache.clear();
        info!("Cleared admin token cache");
    }

    /// Get JWT manager for token operations
    pub fn jwt_manager(&self) -> &AdminJwtManager {
        &self.jwt_manager
    }
}

/// Admin authentication middleware for HTTP requests
pub mod middleware {
    use super::*;
    use warp::{Filter, Rejection};

    /// Create admin authentication filter
    pub fn admin_auth(
        auth_service: AdminAuthService,
        required_permission: AdminPermission,
    ) -> impl Filter<Extract = (ValidatedAdminToken,), Error = Rejection> + Clone {
        warp::header::<String>("authorization").and_then(move |auth_header: String| {
            let auth_service = auth_service.clone();
            let required_permission = required_permission.clone();

            async move {
                // Extract Bearer token
                let token = extract_bearer_token(&auth_header)
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

    /// Extract Bearer token from Authorization header
    fn extract_bearer_token(auth_header: &str) -> Result<String> {
        if !auth_header.starts_with("Bearer ") {
            return Err(anyhow!("Invalid authorization header format"));
        }

        let token = auth_header.strip_prefix("Bearer ").unwrap().trim();
        if token.is_empty() {
            return Err(anyhow!("Empty bearer token"));
        }

        Ok(token.to_string())
    }

    /// Admin authentication errors
    #[derive(Debug)]
    pub enum AdminAuthError {
        InvalidAuthHeader,
        AuthenticationFailed(String),
    }

    impl warp::reject::Reject for AdminAuthError {}

    /// Convert admin auth errors to HTTP responses
    pub async fn handle_admin_auth_rejection(
        err: Rejection,
    ) -> Result<impl warp::Reply, std::convert::Infallible> {
        if let Some(AdminAuthError::InvalidAuthHeader) = err.find() {
            Ok(warp::reply::with_status(
                "Invalid Authorization header".to_string(),
                warp::http::StatusCode::BAD_REQUEST,
            ))
        } else if let Some(AdminAuthError::AuthenticationFailed(msg)) = err.find() {
            Ok(warp::reply::with_status(
                format!("Authentication failed: {}", msg),
                warp::http::StatusCode::UNAUTHORIZED,
            ))
        } else {
            Ok(warp::reply::with_status(
                "Internal server error".to_string(),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::generate_encryption_key;

    #[tokio::test]
    async fn test_admin_authentication_flow() {
        // Create test database
        let encryption_key = generate_encryption_key().to_vec();
        let database = Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap();

        // Create auth service
        let jwt_secret = "test_jwt_secret_for_admin_auth";
        let auth_service = AdminAuthService::new(database.clone(), jwt_secret);

        // Manually create a token with a known secret and store it in database
        let jwt_manager = crate::admin::jwt::AdminJwtManager::with_secret(jwt_secret);
        let test_token = jwt_manager
            .generate_token(
                "test_token_123",
                "test_service",
                &crate::admin::models::AdminPermissions::default_admin(),
                false,
                Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            )
            .unwrap();

        // Generate token hash and prefix for storage
        let token_prefix = crate::admin::jwt::AdminJwtManager::generate_token_prefix(&test_token);
        let token_hash =
            crate::admin::jwt::AdminJwtManager::hash_token_for_storage(&test_token).unwrap();
        let jwt_secret_hash = crate::admin::jwt::AdminJwtManager::hash_secret(jwt_secret);

        // Store token in database manually
        let permissions_json = crate::admin::models::AdminPermissions::default_admin()
            .to_json()
            .unwrap();
        match &database {
            crate::database_plugins::factory::Database::SQLite(sqlite_db) => {
                sqlx::query(
                    r#"
                    INSERT INTO admin_tokens (
                        id, service_name, token_hash, token_prefix,
                        jwt_secret_hash, permissions, is_super_admin, is_active,
                        created_at, usage_count
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind("test_token_123")
                .bind("test_service")
                .bind(&token_hash)
                .bind(&token_prefix)
                .bind(&jwt_secret_hash)
                .bind(&permissions_json)
                .bind(false)
                .bind(true)
                .bind(chrono::Utc::now())
                .bind(0)
                .execute(sqlite_db.inner().pool())
                .await
                .unwrap();
            }
            #[cfg(feature = "postgresql")]
            crate::database_plugins::factory::Database::PostgreSQL(_) => {
                panic!("PostgreSQL not supported in this test");
            }
        }

        // Test authentication
        let result = auth_service
            .authenticate_and_authorize(
                &test_token,
                AdminPermission::ProvisionKeys,
                Some("127.0.0.1"),
            )
            .await;

        if result.is_err() {
            println!("Auth test error: {}", result.as_ref().unwrap_err());
        }
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated.service_name, "test_service");
        assert!(validated
            .permissions
            .has_permission(&AdminPermission::ProvisionKeys));
    }
}
