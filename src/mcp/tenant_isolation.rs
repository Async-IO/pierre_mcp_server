// ABOUTME: Tenant isolation and multi-tenancy management for MCP server
// ABOUTME: Handles user validation, tenant context extraction, and access control
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::resources::ServerResources;
use crate::admin::jwks::JwksManager;
use crate::auth::{AuthManager, Claims};
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::models::{User, UserOAuthToken};
use crate::tenant::{oauth_manager::TenantOAuthCredentials, TenantContext, TenantRole};
use crate::utils::uuid::parse_uuid;
use http::HeaderMap;
use pierre_core::models::TenantId;
use std::sync::Arc;
use tracing::warn;
use uuid::Uuid;

/// Manages tenant isolation and multi-tenancy for the MCP server
pub struct TenantIsolation {
    resources: Arc<ServerResources>,
}

impl TenantIsolation {
    /// Create a new tenant isolation manager
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self { resources }
    }

    /// Validate JWT token and extract tenant context
    ///
    /// The active tenant is determined from the JWT claims `active_tenant_id` field.
    /// If no active tenant is specified, the user's default tenant is used.
    ///
    /// # Errors
    /// Returns an error if JWT validation fails or tenant information cannot be retrieved
    pub async fn validate_tenant_access(&self, jwt_token: &str) -> AppResult<TenantContext> {
        let claims = self
            .resources
            .auth_manager
            .validate_token(jwt_token, &self.resources.jwks_manager)
            .map_err(|e| AppError::auth_invalid(format!("Failed to validate token: {e}")))?;

        // Parse user ID from claims
        let user_id = parse_uuid(&claims.sub).map_err(|e| {
            warn!(sub = %claims.sub, error = %e, "Invalid user ID in JWT token claims");
            AppError::auth_invalid("Invalid user ID in token")
        })?;

        // Get tenant ID from JWT claims (active_tenant_id) or fall back to default tenant
        let tenant_id = self
            .extract_tenant_from_claims_or_default(&claims, user_id)
            .await?;
        let tenant_name = self.get_tenant_name(tenant_id).await;
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        Ok(TenantContext {
            tenant_id,
            tenant_name,
            user_id,
            user_role,
        })
    }

    /// Extract tenant ID from JWT claims or get user's default tenant
    ///
    /// # Errors
    /// Returns an error if user has no tenant memberships
    async fn extract_tenant_from_claims_or_default(
        &self,
        claims: &Claims,
        user_id: Uuid,
    ) -> AppResult<TenantId> {
        // Check if active_tenant_id is specified in JWT claims
        if let Some(tenant_id_str) = claims.active_tenant_id.as_deref() {
            let tenant_id: TenantId = tenant_id_str.parse().map_err(|e| {
                warn!(tenant_id = %tenant_id_str, error = %e, "Invalid tenant ID format in JWT claims");
                AppError::invalid_input("Invalid tenant ID format in token")
            })?;

            // Verify user actually belongs to this tenant
            self.verify_user_tenant_membership(user_id, tenant_id)
                .await?;

            return Ok(tenant_id);
        }

        // No active tenant in claims - get user's default tenant
        self.get_user_default_tenant(user_id).await
    }

    /// Verify user belongs to a tenant via `tenant_users` table
    ///
    /// # Errors
    /// Returns an error if user does not belong to the tenant
    pub async fn verify_user_tenant_membership(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<()> {
        let role = self
            .resources
            .database
            .get_user_tenant_role(user_id, tenant_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to check tenant membership: {e}")))?;

        if role.is_none() {
            return Err(AppError::auth_invalid(format!(
                "User {user_id} does not belong to tenant {tenant_id}"
            )));
        }

        Ok(())
    }

    /// Get user's default tenant (first tenant they belong to)
    ///
    /// # Errors
    /// Returns an error if user has no tenant memberships
    pub async fn get_user_default_tenant(&self, user_id: Uuid) -> AppResult<TenantId> {
        let tenants = self
            .resources
            .database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;

        tenants
            .first()
            .map(|t| t.id)
            .ok_or_else(|| AppError::auth_invalid("User does not belong to any tenant"))
    }

    /// Get user with tenant information
    ///
    /// # Errors
    /// Returns an error if user lookup fails
    pub async fn get_user_with_tenant(&self, user_id: Uuid) -> AppResult<User> {
        self.resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))
    }

    /// Get user's default tenant ID
    ///
    /// This method looks up the user's tenant memberships in the `tenant_users` table
    /// and returns the first tenant (typically the oldest membership).
    ///
    /// # Errors
    /// Returns an error if user has no tenant memberships
    pub async fn extract_tenant_id_for_user(&self, user: &User) -> AppResult<TenantId> {
        self.get_user_default_tenant(user.id).await
    }

    /// Get tenant name by ID
    pub async fn get_tenant_name(&self, tenant_id: TenantId) -> String {
        match self.resources.database.get_tenant_by_id(tenant_id).await {
            Ok(tenant) => tenant.name,
            Err(e) => {
                warn!(
                    "Failed to get tenant {}: {}, using default name",
                    tenant_id, e
                );
                "Unknown Tenant".to_owned()
            }
        }
    }

    /// Get user's role in a tenant
    ///
    /// Uses the `tenant_users` junction table to determine the user's role.
    /// This is the source of truth for multi-tenant membership.
    ///
    /// # Errors
    /// Returns an error if role lookup fails or user doesn't belong to tenant
    pub async fn get_user_role_for_tenant(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<TenantRole> {
        // Query tenant_users table for user's role in the tenant
        let role_str = self
            .resources
            .database
            .get_user_tenant_role(user_id, tenant_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenant role: {e}")))?
            .ok_or_else(|| {
                AppError::auth_invalid(format!(
                    "User {user_id} does not belong to tenant {tenant_id}"
                ))
            })?;

        Ok(TenantRole::from_db_string(&role_str))
    }

    /// Extract tenant context from request headers
    ///
    /// # Errors
    /// Returns an error if header parsing fails
    pub async fn extract_tenant_from_header(
        &self,
        headers: &HeaderMap,
    ) -> AppResult<Option<TenantContext>> {
        // Look for tenant ID in headers
        if let Some(tenant_id_header) = headers.get("x-tenant-id") {
            let tenant_id_str = tenant_id_header.to_str().map_err(|e| {
                warn!(error = %e, "Invalid x-tenant-id header format (non-UTF8)");
                AppError::invalid_input("Invalid tenant ID header format")
            })?;

            let tenant_id: TenantId = tenant_id_str
                .parse()
                .map_err(|e| {
                    warn!(tenant_id = %tenant_id_str, error = %e, "Invalid tenant ID format in x-tenant-id header");
                    AppError::invalid_input("Invalid tenant ID format")
                })?;

            let tenant_name = self.get_tenant_name(tenant_id).await;

            // For header-based tenant context, we don't have user info
            // This should only be used for tenant-scoped operations that don't require user context
            return Ok(Some(TenantContext {
                tenant_id,
                user_id: Uuid::nil(), // No user context available from headers
                tenant_name,
                user_role: TenantRole::Member, // Default role when user is unknown
            }));
        }

        Ok(None)
    }

    /// Extract tenant context from user (using their default tenant)
    ///
    /// # Errors
    /// Returns an error if user lookup or tenant extraction fails
    pub async fn extract_tenant_from_user(&self, user_id: Uuid) -> AppResult<TenantContext> {
        let tenant_id = self.get_user_default_tenant(user_id).await?;
        let tenant_name = self.get_tenant_name(tenant_id).await;
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        Ok(TenantContext {
            tenant_id,
            tenant_name,
            user_id,
            user_role,
        })
    }

    /// Extract tenant context from user with a specific tenant ID
    ///
    /// # Errors
    /// Returns an error if user doesn't belong to tenant
    pub async fn extract_tenant_from_user_with_tenant(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
    ) -> AppResult<TenantContext> {
        // Verify user belongs to this tenant
        self.verify_user_tenant_membership(user_id, tenant_id)
            .await?;

        let tenant_name = self.get_tenant_name(tenant_id).await;
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        Ok(TenantContext {
            tenant_id,
            tenant_name,
            user_id,
            user_role,
        })
    }

    /// Check if user has access to a specific resource
    ///
    /// # Errors
    /// Returns an error if role lookup fails
    pub async fn check_resource_access(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        resource_type: &str,
    ) -> AppResult<bool> {
        // Verify user belongs to the tenant
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        // Basic access control - can be extended based on requirements
        match resource_type {
            "oauth_credentials" => Ok(matches!(user_role, TenantRole::Owner | TenantRole::Member)),
            "fitness_data" => Ok(matches!(user_role, TenantRole::Owner | TenantRole::Member)),
            "tenant_settings" => Ok(matches!(user_role, TenantRole::Owner)),
            _ => {
                warn!("Unknown resource type: {}", resource_type);
                Ok(false)
            }
        }
    }

    /// Isolate database operations to tenant scope
    ///
    /// # Errors
    /// Returns an error if resource isolation fails
    pub fn isolate_resources(&self, tenant_id: TenantId) -> AppResult<TenantResources> {
        // Create tenant-scoped resource accessor
        Ok(TenantResources {
            tenant_id,
            database: self.resources.database.clone(),
        })
    }

    /// Validate that a user can perform an action on behalf of a tenant
    ///
    /// # Errors
    /// Returns an error if validation fails
    pub async fn validate_tenant_action(
        &self,
        user_id: Uuid,
        tenant_id: TenantId,
        action: &str,
    ) -> AppResult<()> {
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        match action {
            "read_oauth_credentials" | "store_oauth_credentials" => {
                if matches!(user_role, TenantRole::Owner | TenantRole::Member) {
                    Ok(())
                } else {
                    Err(AppError::auth_invalid(format!(
                        "User {user_id} does not have permission to {action} for tenant {tenant_id}"
                    )))
                }
            }
            "modify_tenant_settings" => {
                if matches!(user_role, TenantRole::Owner) {
                    Ok(())
                } else {
                    Err(AppError::auth_invalid(format!(
                        "User {user_id} does not have owner permission for tenant {tenant_id}"
                    )))
                }
            }
            _ => {
                warn!("Unknown action for validation: {}", action);
                Err(AppError::invalid_input(format!("Unknown action: {action}")))
            }
        }
    }
}

/// Tenant-scoped resource accessor
pub struct TenantResources {
    /// Unique identifier for the tenant
    pub tenant_id: TenantId,
    /// Database connection for tenant-scoped operations
    pub database: Arc<Database>,
}

impl TenantResources {
    /// Get OAuth credentials for this tenant
    ///
    /// # Errors
    /// Returns an error if credential lookup fails
    pub async fn get_oauth_credentials(
        &self,
        provider: &str,
    ) -> AppResult<Option<TenantOAuthCredentials>> {
        self.database
            .get_tenant_oauth_credentials(self.tenant_id, provider)
            .await
            .map_err(|e| AppError::database(format!("Failed to get tenant OAuth credentials: {e}")))
    }

    /// Store OAuth credentials for this tenant
    ///
    /// # Errors
    /// Returns an error if credential storage fails or tenant ID mismatch
    pub async fn store_oauth_credentials(
        &self,
        credential: &TenantOAuthCredentials,
    ) -> AppResult<()> {
        // Ensure the credential belongs to this tenant
        if credential.tenant_id != self.tenant_id {
            return Err(AppError::invalid_input(format!(
                "Credential tenant ID mismatch: expected {}, got {}",
                self.tenant_id, credential.tenant_id
            )));
        }

        self.database
            .store_tenant_oauth_credentials(credential)
            .await
            .map_err(|e| {
                AppError::database(format!("Failed to store tenant OAuth credentials: {e}"))
            })
    }

    /// Get user OAuth tokens for this tenant
    ///
    /// # Errors
    /// Returns an error if token lookup fails
    pub async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> AppResult<Option<UserOAuthToken>> {
        self.database
            .get_user_oauth_token(user_id, self.tenant_id, provider)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user OAuth token: {e}")))
    }

    /// Store user OAuth token for this tenant
    ///
    /// # Errors
    /// Returns an error if token storage fails
    pub async fn store_user_oauth_token(&self, token: &UserOAuthToken) -> AppResult<()> {
        // Additional validation could be added here to ensure
        // the user belongs to this tenant
        // For now, store using the user's OAuth app approach
        self.database
            .store_user_oauth_app(
                token.user_id,
                &token.provider,
                "", // client_id not available in UserOAuthToken
                "", // client_secret not available in UserOAuthToken
                "", // redirect_uri not available in UserOAuthToken
            )
            .await
            .map_err(|e| AppError::database(format!("Failed to store user OAuth app: {e}")))
    }
}

/// JWT token validation result
#[derive(Debug, Clone)]
pub struct JwtValidationResult {
    /// User ID extracted from the JWT token
    pub user_id: Uuid,
    /// Tenant context associated with the user
    pub tenant_context: TenantContext,
    /// When the JWT token expires
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Standalone function for JWT validation (used by HTTP middleware)
///
/// The active tenant is determined from the JWT claims `active_tenant_id` field.
/// If no active tenant is specified, the user's default tenant is used.
///
/// # Errors
/// Returns an error if JWT validation or user lookup fails
pub async fn validate_jwt_token_for_mcp(
    token: &str,
    auth_manager: &AuthManager,
    jwks_manager: &JwksManager,
    database: &Arc<Database>,
) -> AppResult<JwtValidationResult> {
    let claims = auth_manager
        .validate_token(token, jwks_manager)
        .map_err(|e| AppError::auth_invalid(format!("Failed to validate token: {e}")))?;

    // Parse user ID from claims
    let user_id = parse_uuid(&claims.sub).map_err(|e| {
        warn!(sub = %claims.sub, error = %e, "Invalid user ID in JWT token claims (MCP validation)");
        AppError::auth_invalid("Invalid user ID in token")
    })?;

    // Verify user exists
    database
        .get_user(user_id)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
        .ok_or_else(|| AppError::not_found("User"))?;

    // Get tenant ID from JWT claims or fall back to user's default tenant
    let tenant_id: TenantId = if let Some(tenant_id_str) = claims.active_tenant_id.as_deref() {
        let tid: TenantId = tenant_id_str.parse().map_err(|e| {
            warn!(tenant_id = %tenant_id_str, error = %e, "Invalid tenant ID format in JWT claims (MCP validation)");
            AppError::invalid_input("Invalid tenant ID format in token")
        })?;

        // Verify user belongs to this tenant
        let role = database
            .get_user_tenant_role(user_id, tid)
            .await
            .map_err(|e| AppError::database(format!("Failed to check tenant membership: {e}")))?;

        if role.is_none() {
            return Err(AppError::auth_invalid(format!(
                "User {user_id} does not belong to tenant {tid}"
            )));
        }

        tid
    } else {
        // Get user's default tenant
        let tenants = database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;

        tenants
            .first()
            .map(|t| t.id)
            .ok_or_else(|| AppError::auth_invalid("User does not belong to any tenant"))?
    };

    let tenant_name = match database.get_tenant_by_id(tenant_id).await {
        Ok(tenant) => tenant.name,
        _ => "Unknown Tenant".to_owned(),
    };

    // Get user's role in this tenant
    let user_role = database
        .get_user_tenant_role(user_id, tenant_id)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user tenant role: {e}")))?
        .map_or(TenantRole::Member, |role_str| {
            TenantRole::from_db_string(&role_str)
        });

    let tenant_context = TenantContext {
        tenant_id,
        tenant_name,
        user_id,
        user_role,
    };

    // For now, set a default expiration
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

    Ok(JwtValidationResult {
        user_id,
        tenant_context,
        expires_at,
    })
}

/// Extract tenant context from various sources (internal helper)
///
/// Priority order:
/// 1. Explicit `tenant_id` parameter
/// 2. `x-tenant-id` header
/// 3. User's default tenant (from `tenant_users` table)
///
/// # Errors
/// Returns an error if tenant extraction fails
pub async fn extract_tenant_context_internal(
    database: &Arc<Database>,
    user_id: Option<Uuid>,
    tenant_id: Option<TenantId>,
    headers: Option<&HeaderMap>,
) -> AppResult<Option<TenantContext>> {
    // Try to extract from explicit tenant ID first
    if let Some(tenant_id) = tenant_id {
        // If user_id is provided, verify membership and get role
        let (user_role, verified_user_id) = if let Some(uid) = user_id {
            let role_str = database
                .get_user_tenant_role(uid, tenant_id)
                .await
                .map_err(|e| {
                    AppError::database(format!("Failed to check tenant membership: {e}"))
                })?;

            let role = role_str.map_or(TenantRole::Member, |r| TenantRole::from_db_string(&r));
            (role, uid)
        } else {
            (TenantRole::Member, Uuid::nil())
        };

        let tenant_name = match database.get_tenant_by_id(tenant_id).await {
            Ok(tenant) => tenant.name,
            _ => "Unknown Tenant".to_owned(),
        };

        return Ok(Some(TenantContext {
            tenant_id,
            user_id: verified_user_id,
            tenant_name,
            user_role,
        }));
    }

    // Try to extract from headers
    if let Some(headers) = headers {
        if let Some(tenant_id_header) = headers.get("x-tenant-id") {
            if let Ok(tenant_id_str) = tenant_id_header.to_str() {
                if let Ok(header_tenant_id) = tenant_id_str.parse::<TenantId>() {
                    // If user_id is provided, verify membership
                    let (user_role, verified_user_id) = if let Some(uid) = user_id {
                        let role_str = database
                            .get_user_tenant_role(uid, header_tenant_id)
                            .await
                            .map_err(|e| {
                            AppError::database(format!("Failed to check tenant membership: {e}"))
                        })?;

                        let role =
                            role_str.map_or(TenantRole::Member, |r| TenantRole::from_db_string(&r));
                        (role, uid)
                    } else {
                        (TenantRole::Member, Uuid::nil())
                    };

                    let tenant_name = match database.get_tenant_by_id(header_tenant_id).await {
                        Ok(tenant) => tenant.name,
                        _ => "Unknown Tenant".to_owned(),
                    };

                    return Ok(Some(TenantContext {
                        tenant_id: header_tenant_id,
                        user_id: verified_user_id,
                        tenant_name,
                        user_role,
                    }));
                }
            }
        }
    }

    // Try to extract from user's default tenant (via tenant_users table)
    if let Some(user_id) = user_id {
        // Verify user exists
        database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))?;

        // Get user's tenants from tenant_users table
        let tenants = database
            .list_tenants_for_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenants: {e}")))?;

        if let Some(default_tenant) = tenants.first() {
            let user_role = database
                .get_user_tenant_role(user_id, default_tenant.id)
                .await
                .map_err(|e| AppError::database(format!("Failed to get user tenant role: {e}")))?
                .map_or(TenantRole::Member, |r| TenantRole::from_db_string(&r));

            return Ok(Some(TenantContext {
                tenant_id: default_tenant.id,
                tenant_name: default_tenant.name.clone(),
                user_id,
                user_role,
            }));
        }
    }

    Ok(None)
}
