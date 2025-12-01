// ABOUTME: Tenant isolation and multi-tenancy management for MCP server
// ABOUTME: Handles user validation, tenant context extraction, and access control
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::resources::ServerResources;
use crate::auth::AuthManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::tenant::{TenantContext, TenantRole};
use http::HeaderMap;
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
    /// # Errors
    /// Returns an error if JWT validation fails or tenant information cannot be retrieved
    pub async fn validate_tenant_access(&self, jwt_token: &str) -> AppResult<TenantContext> {
        let auth_result = self
            .resources
            .auth_manager
            .validate_token(jwt_token, &self.resources.jwks_manager)
            .map_err(|e| AppError::auth_invalid(format!("Failed to validate token: {e}")))?;

        // Parse user ID from claims
        let user_id = crate::utils::uuid::parse_uuid(&auth_result.sub)
            .map_err(|e| {
                tracing::warn!(sub = %auth_result.sub, error = %e, "Invalid user ID in JWT token claims");
                AppError::auth_invalid("Invalid user ID in token")
            })?;

        let user = self.get_user_with_tenant(user_id).await?;
        let tenant_id = self.extract_tenant_id(&user)?;
        let tenant_name = self.get_tenant_name(tenant_id).await;
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        Ok(TenantContext {
            tenant_id,
            tenant_name,
            user_id,
            user_role,
        })
    }

    /// Get user with tenant information
    ///
    /// # Errors
    /// Returns an error if user lookup fails
    pub async fn get_user_with_tenant(&self, user_id: Uuid) -> AppResult<crate::models::User> {
        self.resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))
    }

    /// Extract tenant ID from user
    ///
    /// # Errors
    /// Returns an error if tenant ID is missing or invalid
    pub fn extract_tenant_id(&self, user: &crate::models::User) -> AppResult<Uuid> {
        user.tenant_id
            .clone() // Safe: Option<String> ownership for UUID parsing
            .ok_or_else(|| AppError::auth_invalid("User does not belong to any tenant"))?
            .parse()
            .map_err(|e| {
                tracing::warn!(user_id = %user.id, tenant_id = ?user.tenant_id, error = %e, "Invalid tenant ID format for user");
                AppError::invalid_input("Invalid tenant ID format")
            })
    }

    /// Get tenant name by ID
    pub async fn get_tenant_name(&self, tenant_id: Uuid) -> String {
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
    /// # Errors
    /// Returns an error if role lookup fails
    pub async fn get_user_role_for_tenant(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> AppResult<TenantRole> {
        // Check if user belongs to the tenant
        let user = self.get_user_with_tenant(user_id).await?;

        if user.tenant_id != Some(tenant_id.to_string()) {
            return Err(AppError::auth_invalid(format!(
                "User {user_id} does not belong to tenant {tenant_id}"
            )));
        }

        // Query database for user's actual role in the tenant
        (self
            .resources
            .database
            .get_user_tenant_role(user_id, tenant_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user tenant role: {e}")))?)
        .map_or_else(
            || Ok(TenantRole::Member),
            |role_str| match role_str.to_lowercase().as_str() {
                "owner" => Ok(TenantRole::Owner),
                "admin" => Ok(TenantRole::Admin),
                "billing" => Ok(TenantRole::Billing),
                "member" => Ok(TenantRole::Member),
                _ => {
                    warn!(
                        "Unknown role '{}' for user {} in tenant {}, defaulting to Member",
                        role_str, user_id, tenant_id
                    );
                    Ok(TenantRole::Member)
                }
            },
        )
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
                tracing::warn!(error = %e, "Invalid x-tenant-id header format (non-UTF8)");
                AppError::invalid_input("Invalid tenant ID header format")
            })?;

            let tenant_id = Uuid::parse_str(tenant_id_str)
                .map_err(|e| {
                    tracing::warn!(tenant_id = %tenant_id_str, error = %e, "Invalid tenant ID format in x-tenant-id header");
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

    /// Extract tenant context from user
    ///
    /// # Errors
    /// Returns an error if user lookup or tenant extraction fails
    pub async fn extract_tenant_from_user(&self, user_id: Uuid) -> AppResult<TenantContext> {
        let user = self.get_user_with_tenant(user_id).await?;
        let tenant_id = self.extract_tenant_id(&user)?;
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
        tenant_id: Uuid,
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
    pub fn isolate_resources(&self, tenant_id: Uuid) -> AppResult<TenantResources> {
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
        tenant_id: Uuid,
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
    pub tenant_id: Uuid,
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
    ) -> AppResult<Option<crate::tenant::oauth_manager::TenantOAuthCredentials>> {
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
        credential: &crate::tenant::oauth_manager::TenantOAuthCredentials,
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
    ) -> AppResult<Option<crate::models::UserOAuthToken>> {
        // Convert tenant_id to string for database query
        let tenant_id_str = self.tenant_id.to_string();
        self.database
            .get_user_oauth_token(user_id, &tenant_id_str, provider)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user OAuth token: {e}")))
    }

    /// Store user OAuth token for this tenant
    ///
    /// # Errors
    /// Returns an error if token storage fails
    pub async fn store_user_oauth_token(
        &self,
        token: &crate::models::UserOAuthToken,
    ) -> AppResult<()> {
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
/// # Errors
/// Returns an error if JWT validation or user lookup fails
pub async fn validate_jwt_token_for_mcp(
    token: &str,
    auth_manager: &AuthManager,
    jwks_manager: &crate::admin::jwks::JwksManager,
    database: &Arc<Database>,
) -> AppResult<JwtValidationResult> {
    let auth_result = auth_manager
        .validate_token(token, jwks_manager)
        .map_err(|e| AppError::auth_invalid(format!("Failed to validate token: {e}")))?;

    // Parse user ID from claims
    let user_id = crate::utils::uuid::parse_uuid(&auth_result.sub)
        .map_err(|e| {
            tracing::warn!(sub = %auth_result.sub, error = %e, "Invalid user ID in JWT token claims (MCP validation)");
            AppError::auth_invalid("Invalid user ID in token")
        })?;

    // Get user and tenant information
    let user = database
        .get_user(user_id)
        .await
        .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
        .ok_or_else(|| AppError::not_found("User"))?;

    let tenant_id = user
        .tenant_id
        .clone() // Safe: Option<String> ownership for UUID parsing
        .ok_or_else(|| AppError::auth_invalid("User does not belong to any tenant"))?
        .parse()
        .map_err(|e| {
            tracing::warn!(user_id = %user_id, tenant_id = ?user.tenant_id, error = %e, "Invalid tenant ID format for user (MCP validation)");
            AppError::invalid_input("Invalid tenant ID format")
        })?;

    let tenant_name = match database.get_tenant_by_id(tenant_id).await {
        Ok(tenant) => tenant.name,
        _ => "Unknown Tenant".to_owned(),
    };

    let tenant_context = TenantContext {
        tenant_id,
        tenant_name,
        user_id,
        user_role: TenantRole::Member, // Default role
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
/// # Errors
/// Returns an error if tenant extraction fails
pub async fn extract_tenant_context_internal(
    database: &Arc<Database>,
    user_id: Option<Uuid>,
    tenant_id: Option<Uuid>,
    headers: Option<&HeaderMap>,
) -> AppResult<Option<TenantContext>> {
    // Try to extract from user ID first
    if let Some(user_id) = user_id {
        let user = database
            .get_user(user_id)
            .await
            .map_err(|e| AppError::database(format!("Failed to get user: {e}")))?
            .ok_or_else(|| AppError::not_found("User"))?;

        if let Some(tenant_id_str) = user.tenant_id {
            // Try parsing as UUID first, then try as slug
            if let Ok(tenant_uuid) = tenant_id_str.parse::<Uuid>() {
                let tenant = database.get_tenant_by_id(tenant_uuid).await.map_err(|e| {
                    AppError::database(format!("Failed to get tenant by UUID: {e}"))
                })?;
                return Ok(Some(TenantContext {
                    tenant_id: tenant_uuid,
                    tenant_name: tenant.name,
                    user_id,
                    user_role: TenantRole::Member, // Default role
                }));
            }
            // Try as slug
            let tenant = database
                .get_tenant_by_slug(&tenant_id_str)
                .await
                .map_err(|e| AppError::database(format!("Failed to get tenant by slug: {e}")))?;
            return Ok(Some(TenantContext {
                tenant_id: tenant.id,
                tenant_name: tenant.name,
                user_id,
                user_role: TenantRole::Member, // Default role
            }));
        }
    }

    // Try to extract from explicit tenant ID
    if let Some(tenant_id) = tenant_id {
        let tenant_name = match database.get_tenant_by_id(tenant_id).await {
            Ok(tenant) => tenant.name,
            _ => "Unknown Tenant".to_owned(),
        };

        return Ok(Some(TenantContext {
            tenant_id,
            user_id: user_id.unwrap_or_else(Uuid::nil),
            tenant_name,
            user_role: TenantRole::Member,
        }));
    }

    // Try to extract from headers
    if let Some(headers) = headers {
        if let Some(tenant_id_header) = headers.get("x-tenant-id") {
            if let Ok(tenant_id_str) = tenant_id_header.to_str() {
                if let Ok(tenant_id) = Uuid::parse_str(tenant_id_str) {
                    let tenant_name = match database.get_tenant_by_id(tenant_id).await {
                        Ok(tenant) => tenant.name,
                        _ => "Unknown Tenant".to_owned(),
                    };

                    return Ok(Some(TenantContext {
                        tenant_id,
                        user_id: user_id.unwrap_or_else(Uuid::nil),
                        tenant_name,
                        user_role: TenantRole::Member,
                    }));
                }
            }
        }
    }

    Ok(None)
}
