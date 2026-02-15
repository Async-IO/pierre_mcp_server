// ABOUTME: Defines ToolExecutionContext which provides tools with access to resources and user context.
// ABOUTME: This replaces scattered parameter passing with a unified context object.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Tool Execution Context
//!
//! Provides a unified context object for tool execution, containing:
//! - User and tenant identity
//! - Access to shared server resources
//! - Request tracing information
//! - Authentication method details
//!
//! This design eliminates the need to pass multiple parameters through
//! tool execution chains and provides consistent access to resources.

use pierre_core::models::TenantId;
use std::fmt;
use std::sync::Arc;

use serde_json::Value;
use uuid::Uuid;

use crate::cache::factory::Cache;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult, ErrorCode};
use crate::intelligence::ActivityIntelligence;
use crate::mcp::resources::ServerResources;
use crate::mcp::tool_selection::ToolSelectionService;
use crate::models::User;
use crate::providers::ProviderRegistry;

/// How the user authenticated for this request.
///
/// Useful for audit logging and determining available permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMethod {
    /// JWT Bearer token (most common)
    JwtBearer,
    /// API key authentication
    ApiKey,
    /// OAuth 2.0 token from MCP OAuth flow
    OAuth2,
    /// MCP client registration token
    McpClient,
}

impl AuthMethod {
    /// Get a string representation for logging
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::JwtBearer => "jwt_bearer",
            Self::ApiKey => "api_key",
            Self::OAuth2 => "oauth2",
            Self::McpClient => "mcp_client",
        }
    }
}

/// Context provided to every tool execution.
///
/// This struct provides tools with everything they need to execute:
/// - User and tenant identity for authorization
/// - Access to shared resources (database, providers, etc.)
/// - Request tracing information
///
/// # Arc Cloning Note
///
/// The `resources` field is `Arc<ServerResources>` which is cloned when
/// creating new contexts. This is necessary because:
/// - `ServerResources` contains expensive resources (DB pools, managers)
/// - Multiple tools may execute concurrently
/// - Arc cloning is cheap (atomic increment)
///
/// See `src/mcp/resources.rs` for the full resource container.
#[derive(Clone)]
pub struct ToolExecutionContext {
    /// Authenticated user ID (always present after auth)
    pub user_id: Uuid,
    /// Tenant ID (present for tenant-scoped operations)
    pub tenant_id: Option<Uuid>,
    /// Request ID for tracing/logging
    pub request_id: Option<Value>,
    /// Access to all server resources (database, providers, etc.)
    pub resources: Arc<ServerResources>,
    /// Authentication method used (for audit logging)
    pub auth_method: AuthMethod,
    /// Whether the user has admin privileges (cached to avoid repeated DB queries)
    is_admin: Option<bool>,
}

impl ToolExecutionContext {
    /// Create a new context with required fields
    ///
    /// # Arguments
    ///
    /// * `user_id` - The authenticated user's ID
    /// * `resources` - Arc-wrapped server resources
    /// * `auth_method` - How the user authenticated
    #[must_use]
    pub const fn new(
        user_id: Uuid,
        resources: Arc<ServerResources>,
        auth_method: AuthMethod,
    ) -> Self {
        Self {
            user_id,
            tenant_id: None,
            request_id: None,
            resources,
            auth_method,
            is_admin: None,
        }
    }

    /// Set tenant ID
    #[must_use]
    pub fn with_tenant(mut self, tenant_id: TenantId) -> Self {
        self.tenant_id = Some(tenant_id.as_uuid());
        self
    }

    /// Set request ID for tracing
    #[must_use]
    pub fn with_request_id(mut self, request_id: Value) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set admin status (cached to avoid repeated DB queries)
    #[must_use]
    pub const fn with_admin_status(mut self, is_admin: bool) -> Self {
        self.is_admin = Some(is_admin);
        self
    }

    /// Get tenant ID or return error for tools requiring tenant context
    ///
    /// # Errors
    ///
    /// Returns `AppError` with `PermissionDenied` if no tenant context is available
    pub fn require_tenant(&self) -> AppResult<Uuid> {
        self.tenant_id.ok_or_else(|| {
            AppError::new(
                ErrorCode::PermissionDenied,
                "Tenant context required for this operation",
            )
        })
    }

    /// Check if the user has admin privileges
    ///
    /// Uses cached value if available, otherwise queries the database.
    ///
    /// # Errors
    ///
    /// Returns `AppError` if database query fails or user not found
    pub async fn is_admin(&self) -> AppResult<bool> {
        // Return cached value if available
        if let Some(is_admin) = self.is_admin {
            return Ok(is_admin);
        }

        // Query database for user to check admin status
        // SECURITY: Global lookup — tool context checks admin status before tenant is known
        let user: User = self
            .resources
            .database
            .get_user_global(self.user_id)
            .await?
            .ok_or_else(|| {
                AppError::new(
                    ErrorCode::ResourceNotFound,
                    format!("User {} not found", self.user_id),
                )
            })?;

        Ok(user.is_admin)
    }

    /// Require admin privileges for the current operation
    ///
    /// # Errors
    ///
    /// Returns `AppError` with `PermissionDenied` if user is not an admin
    pub async fn require_admin(&self) -> AppResult<()> {
        if self.is_admin().await? {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::PermissionDenied,
                "Admin privileges required for this operation",
            ))
        }
    }

    /// Get a reference to the database
    #[must_use]
    pub fn database(&self) -> &Database {
        &self.resources.database
    }

    /// Get a reference to the provider registry
    #[must_use]
    pub fn provider_registry(&self) -> &ProviderRegistry {
        &self.resources.provider_registry
    }

    /// Get a reference to the activity intelligence engine
    #[must_use]
    pub fn activity_intelligence(&self) -> &ActivityIntelligence {
        &self.resources.activity_intelligence
    }

    /// Get a reference to the cache
    #[must_use]
    pub fn cache(&self) -> &Cache {
        &self.resources.cache
    }

    /// Get a reference to the tool selection service
    #[must_use]
    pub fn tool_selection(&self) -> &ToolSelectionService {
        &self.resources.tool_selection
    }

    /// Create a child context with the same resources but different user
    ///
    /// Useful for service-to-service calls or impersonation scenarios.
    #[must_use]
    pub fn with_user(&self, user_id: Uuid) -> Self {
        Self {
            user_id,
            tenant_id: self.tenant_id,
            request_id: self.request_id.clone(),
            resources: self.resources.clone(),
            auth_method: self.auth_method,
            is_admin: None, // Reset admin cache for new user
        }
    }

    /// Get tracing span attributes for this context
    #[must_use]
    pub fn span_attributes(&self) -> Vec<(&'static str, String)> {
        let mut attrs = vec![
            ("user_id", self.user_id.to_string()),
            ("auth_method", self.auth_method.as_str().to_owned()),
        ];

        if let Some(tenant_id) = self.tenant_id {
            attrs.push(("tenant_id", tenant_id.to_string()));
        }

        if let Some(request_id) = &self.request_id {
            attrs.push(("request_id", request_id.to_string()));
        }

        attrs
    }
}

impl fmt::Debug for ToolExecutionContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolExecutionContext")
            .field("user_id", &self.user_id)
            .field("tenant_id", &self.tenant_id)
            .field("request_id", &self.request_id)
            .field("auth_method", &self.auth_method)
            .field("is_admin", &self.is_admin)
            .field("resources", &"<ServerResources>")
            .finish()
    }
}
