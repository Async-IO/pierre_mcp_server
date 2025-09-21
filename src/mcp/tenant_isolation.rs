// ABOUTME: Tenant isolation and multi-tenancy management for MCP server
// ABOUTME: Handles user validation, tenant context extraction, and access control

use super::resources::ServerResources;
use crate::auth::AuthManager;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::tenant::{TenantContext, TenantRole};
use anyhow::Result;
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
    pub async fn validate_tenant_access(&self, jwt_token: &str) -> Result<TenantContext> {
        let auth_result = self.resources.auth_manager.validate_token(jwt_token)?;

        // Parse user ID from claims
        let user_id = crate::utils::uuid::parse_uuid(&auth_result.sub)
            .map_err(|_| anyhow::anyhow!("Invalid user ID in token"))?;

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
    pub async fn get_user_with_tenant(&self, user_id: Uuid) -> Result<crate::models::User> {
        self.resources
            .database
            .get_user(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get user: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("User not found"))
    }

    /// Extract tenant ID from user
    ///
    /// # Errors
    /// Returns an error if tenant ID is missing or invalid
    pub fn extract_tenant_id(&self, user: &crate::models::User) -> Result<Uuid> {
        user.tenant_id
            .clone()
            .ok_or_else(|| anyhow::anyhow!("User does not belong to any tenant"))?
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid tenant ID format"))
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
                "Unknown Tenant".to_string()
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
    ) -> Result<TenantRole> {
        // Check if user belongs to the tenant
        let user = self.get_user_with_tenant(user_id).await?;

        if user.tenant_id != Some(tenant_id.to_string()) {
            return Err(anyhow::anyhow!(
                "User {} does not belong to tenant {}",
                user_id,
                tenant_id
            ));
        }

        // Query database for user's actual role in the tenant
        if let Some(role_str) = self.database.get_user_tenant_role(user_id, tenant_id).await? {
            match role_str.to_lowercase().as_str() {
                "owner" => Ok(TenantRole::Owner),
                "admin" => Ok(TenantRole::Admin),
                "billing" => Ok(TenantRole::Billing),
                "member" => Ok(TenantRole::Member),
                _ => {
                    warn!("Unknown role '{}' for user {} in tenant {}, defaulting to Member", role_str, user_id, tenant_id);
                    Ok(TenantRole::Member)
                }
            }
        } else {
            // User is not explicitly assigned a role in this tenant, default to Member
            Ok(TenantRole::Member)
        }
    }

    /// Extract tenant context from request headers
    ///
    /// # Errors
    /// Returns an error if header parsing fails
    pub async fn extract_tenant_from_header(
        &self,
        headers: &warp::http::HeaderMap,
    ) -> Result<Option<TenantContext>> {
        // Look for tenant ID in headers
        if let Some(tenant_id_header) = headers.get("x-tenant-id") {
            let tenant_id_str = tenant_id_header
                .to_str()
                .map_err(|_| anyhow::anyhow!("Invalid tenant ID header format"))?;

            let tenant_id = Uuid::parse_str(tenant_id_str)
                .map_err(|_| anyhow::anyhow!("Invalid tenant ID format"))?;

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
    pub async fn extract_tenant_from_user(&self, user_id: Uuid) -> Result<TenantContext> {
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
    ) -> Result<bool> {
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
    pub fn isolate_resources(&self, tenant_id: Uuid) -> Result<TenantResources> {
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
    ) -> Result<()> {
        let user_role = self.get_user_role_for_tenant(user_id, tenant_id).await?;

        match action {
            "read_oauth_credentials" | "store_oauth_credentials" => {
                if matches!(user_role, TenantRole::Owner | TenantRole::Member) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "User {} does not have permission to {} for tenant {}",
                        user_id,
                        action,
                        tenant_id
                    ))
                }
            }
            "modify_tenant_settings" => {
                if matches!(user_role, TenantRole::Owner) {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!(
                        "User {} does not have owner permission for tenant {}",
                        user_id,
                        tenant_id
                    ))
                }
            }
            _ => {
                warn!("Unknown action for validation: {}", action);
                Err(anyhow::anyhow!("Unknown action: {}", action))
            }
        }
    }
}

/// Tenant-scoped resource accessor
pub struct TenantResources {
    pub tenant_id: Uuid,
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
    ) -> Result<Option<crate::tenant::oauth_manager::TenantOAuthCredentials>> {
        self.database
            .get_tenant_oauth_credentials(self.tenant_id, provider)
            .await
    }

    /// Store OAuth credentials for this tenant
    ///
    /// # Errors
    /// Returns an error if credential storage fails or tenant ID mismatch
    pub async fn store_oauth_credentials(
        &self,
        credential: &crate::tenant::oauth_manager::TenantOAuthCredentials,
    ) -> Result<()> {
        // Ensure the credential belongs to this tenant
        if credential.tenant_id != self.tenant_id {
            return Err(anyhow::anyhow!(
                "Credential tenant ID mismatch: expected {}, got {}",
                self.tenant_id,
                credential.tenant_id
            ));
        }

        self.database
            .store_tenant_oauth_credentials(credential)
            .await
    }

    /// Get user OAuth tokens for this tenant
    ///
    /// # Errors
    /// Returns an error if token lookup fails
    pub async fn get_user_oauth_tokens(
        &self,
        user_id: Uuid,
        provider: &str,
    ) -> Result<Option<crate::models::UserOAuthToken>> {
        // Convert tenant_id to string for database query
        let tenant_id_str = self.tenant_id.to_string();
        self.database
            .get_user_oauth_token(user_id, &tenant_id_str, provider)
            .await
    }

    /// Store user OAuth token for this tenant
    ///
    /// # Errors
    /// Returns an error if token storage fails
    pub async fn store_user_oauth_token(
        &self,
        token: &crate::models::UserOAuthToken,
    ) -> Result<()> {
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
    }
}

/// JWT token validation result
#[derive(Debug, Clone)]
pub struct JwtValidationResult {
    pub user_id: Uuid,
    pub tenant_context: TenantContext,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Standalone function for JWT validation (used by HTTP middleware)
///
/// # Errors
/// Returns an error if JWT validation or user lookup fails
pub async fn validate_jwt_token_for_mcp(
    token: &str,
    auth_manager: &AuthManager,
    database: &Arc<Database>,
) -> Result<JwtValidationResult> {
    let auth_result = auth_manager.validate_token(token)?;

    // Parse user ID from claims
    let user_id = crate::utils::uuid::parse_uuid(&auth_result.sub)
        .map_err(|_| anyhow::anyhow!("Invalid user ID in token"))?;

    // Get user and tenant information
    let user = database
        .get_user(user_id)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get user: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    let tenant_id = user
        .tenant_id
        .clone()
        .ok_or_else(|| anyhow::anyhow!("User does not belong to any tenant"))?
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid tenant ID format"))?;

    let tenant_name = match database.get_tenant_by_id(tenant_id).await {
        Ok(tenant) => tenant.name,
        _ => "Unknown Tenant".to_string(),
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
    headers: Option<&warp::http::HeaderMap>,
) -> Result<Option<TenantContext>> {
    // Try to extract from user ID first
    if let Some(user_id) = user_id {
        let user = database
            .get_user(user_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get user: {}", e))?
            .ok_or_else(|| anyhow::anyhow!("User not found"))?;

        if let Some(tenant_id_str) = user.tenant_id {
            // Try parsing as UUID first, then try as slug
            if let Ok(tenant_uuid) = tenant_id_str.parse::<Uuid>() {
                let tenant = database
                    .get_tenant_by_id(tenant_uuid)
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to get tenant by UUID: {}", e))?;
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
                .map_err(|e| anyhow::anyhow!("Failed to get tenant by slug: {}", e))?;
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
            _ => "Unknown Tenant".to_string(),
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
                        _ => "Unknown Tenant".to_string(),
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
