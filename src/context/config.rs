// ABOUTME: Configuration context for dependency injection of config and OAuth services
// ABOUTME: Contains server config, OAuth managers, and tenant services for configuration operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::a2a::client::A2AClientManager;
use crate::a2a::system_user::A2ASystemUserService;
use crate::tenant::TenantOAuthClient;
use std::sync::Arc;

/// Configuration context containing config and OAuth dependencies
///
/// This context provides all configuration-related dependencies needed for
/// OAuth flows, tenant management, and system configuration.
///
/// # Dependencies
/// - `config`: Server configuration settings
/// - `tenant_oauth_client`: Multi-tenant OAuth client management
/// - `a2a_client_manager`: Application-to-application client management
/// - `a2a_system_user_service`: System user service for A2A operations
#[derive(Clone)]
pub struct ConfigContext {
    config: Arc<crate::config::environment::ServerConfig>,
    tenant_oauth_client: Arc<TenantOAuthClient>,
    a2a_client_manager: Arc<A2AClientManager>,
    a2a_system_user_service: Arc<A2ASystemUserService>,
}

impl ConfigContext {
    /// Create new configuration context
    #[must_use]
    pub const fn new(
        config: Arc<crate::config::environment::ServerConfig>,
        tenant_oauth_client: Arc<TenantOAuthClient>,
        a2a_client_manager: Arc<A2AClientManager>,
        a2a_system_user_service: Arc<A2ASystemUserService>,
    ) -> Self {
        Self {
            config,
            tenant_oauth_client,
            a2a_client_manager,
            a2a_system_user_service,
        }
    }

    /// Get server configuration
    #[must_use]
    pub const fn config(&self) -> &Arc<crate::config::environment::ServerConfig> {
        &self.config
    }

    /// Get tenant OAuth client for multi-tenant operations
    #[must_use]
    pub const fn tenant_oauth_client(&self) -> &Arc<TenantOAuthClient> {
        &self.tenant_oauth_client
    }

    /// Get A2A client manager for application-to-application operations
    #[must_use]
    pub const fn a2a_client_manager(&self) -> &Arc<A2AClientManager> {
        &self.a2a_client_manager
    }

    /// Get A2A system user service
    #[must_use]
    pub const fn a2a_system_user_service(&self) -> &Arc<A2ASystemUserService> {
        &self.a2a_system_user_service
    }
}
