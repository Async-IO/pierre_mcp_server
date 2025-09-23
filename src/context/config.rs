// ABOUTME: Configuration context for dependency injection of config and OAuth services
// ABOUTME: Contains server config, OAuth managers, and tenant services for configuration operations

use crate::a2a::client::A2AClientManager;
use crate::a2a::system_user::A2ASystemUserService;
use crate::oauth::manager::OAuthManager;
use crate::tenant::TenantOAuthClient;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Configuration context containing config and OAuth dependencies
///
/// This context provides all configuration-related dependencies needed for
/// OAuth flows, tenant management, and system configuration.
///
/// # Dependencies
/// - `config`: Server configuration settings
/// - `oauth_manager`: OAuth provider management and flows
/// - `tenant_oauth_client`: Multi-tenant OAuth client management
/// - `a2a_client_manager`: Application-to-application client management
/// - `a2a_system_user_service`: System user service for A2A operations
#[derive(Clone)]
pub struct ConfigContext {
    config: Arc<crate::config::environment::ServerConfig>,
    oauth_manager: Arc<RwLock<OAuthManager>>,
    tenant_oauth_client: Arc<TenantOAuthClient>,
    a2a_client_manager: Arc<A2AClientManager>,
    a2a_system_user_service: Arc<A2ASystemUserService>,
}

impl ConfigContext {
    /// Create new configuration context
    #[must_use]
    pub const fn new(
        config: Arc<crate::config::environment::ServerConfig>,
        oauth_manager: Arc<RwLock<OAuthManager>>,
        tenant_oauth_client: Arc<TenantOAuthClient>,
        a2a_client_manager: Arc<A2AClientManager>,
        a2a_system_user_service: Arc<A2ASystemUserService>,
    ) -> Self {
        Self {
            config,
            oauth_manager,
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

    /// Get OAuth manager for provider operations
    #[must_use]
    pub const fn oauth_manager(&self) -> &Arc<RwLock<OAuthManager>> {
        &self.oauth_manager
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
