// ABOUTME: Multi-tenant architecture support for enterprise SaaS deployment
// ABOUTME: Provides tenant management, OAuth credential isolation, and per-tenant rate limiting
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Multi-Tenant Architecture
//!
//! This module implements true multi-tenancy for Pierre MCP Server, enabling:
//! - Per-tenant OAuth credential management
//! - Tenant-isolated rate limiting
//! - Enterprise-ready `SaaS` deployment
//! - Secure tenant data isolation

/// Tenant-aware OAuth client implementation
pub mod oauth_client;
/// OAuth credential management for tenants
pub mod oauth_manager;
/// Tenant database schema and models
pub mod schema;

pub use oauth_client::{StoreCredentialsRequest, TenantOAuthClient};
pub use oauth_manager::{CredentialConfig, TenantOAuthCredentials, TenantOAuthManager};
pub use schema::{Tenant, TenantRole, TenantUser};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tenant context for all operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
    /// Tenant ID
    pub tenant_id: Uuid,
    /// Tenant name for display
    pub tenant_name: String,
    /// User ID within tenant context
    pub user_id: Uuid,
    /// User's role within the tenant
    pub user_role: TenantRole,
}

impl TenantContext {
    /// Create new tenant context
    #[must_use]
    pub const fn new(
        tenant_id: Uuid,
        tenant_name: String,
        user_id: Uuid,
        user_role: TenantRole,
    ) -> Self {
        Self {
            tenant_id,
            tenant_name,
            user_id,
            user_role,
        }
    }

    /// Check if user has admin privileges in this tenant
    #[must_use]
    pub const fn is_admin(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }

    /// Check if user can configure OAuth apps
    #[must_use]
    pub const fn can_configure_oauth(&self) -> bool {
        matches!(self.user_role, TenantRole::Admin | TenantRole::Owner)
    }
}
