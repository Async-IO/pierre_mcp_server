// ABOUTME: Composed server context that provides all focused contexts for gradual migration
// ABOUTME: Replaces ServerResources with focused dependency injection while maintaining compatibility

use super::{AuthContext, ConfigContext, DataContext, NotificationContext};
use crate::mcp::resources::ServerResources;

/// Composed server context containing all focused contexts
///
/// This context provides a migration path from `ServerResources` to focused contexts.
/// It composes all focused contexts and provides convenience methods for accessing
/// specific dependency groups.
///
/// # Migration Strategy
/// 1. Replace `ServerResources` construction with `ServerContext::from(resources)`
/// 2. Update handlers to use focused contexts: `ctx.auth()`, `ctx.data()`, etc.
/// 3. Gradually remove `ServerResources` dependency as handlers are migrated
#[derive(Clone)]
pub struct ServerContext {
    auth: AuthContext,
    data: DataContext,
    config: ConfigContext,
    notification: NotificationContext,
}

impl ServerContext {
    /// Create new server context from focused contexts
    #[must_use]
    pub const fn new(
        auth: AuthContext,
        data: DataContext,
        config: ConfigContext,
        notification: NotificationContext,
    ) -> Self {
        Self {
            auth,
            data,
            config,
            notification,
        }
    }

    /// Get authentication context
    #[must_use]
    pub const fn auth(&self) -> &AuthContext {
        &self.auth
    }

    /// Get data context
    #[must_use]
    pub const fn data(&self) -> &DataContext {
        &self.data
    }

    /// Get configuration context
    #[must_use]
    pub const fn config(&self) -> &ConfigContext {
        &self.config
    }

    /// Get notification context
    #[must_use]
    pub const fn notification(&self) -> &NotificationContext {
        &self.notification
    }
}

impl From<&ServerResources> for ServerContext {
    /// Create server context from existing `ServerResources` for migration
    fn from(resources: &ServerResources) -> Self {
        let auth = AuthContext::new(
            resources.auth_manager.clone(),
            resources.auth_middleware.clone(),
            resources.admin_jwt_secret.clone(),
        );

        let data = DataContext::new(
            resources.database.clone(),
            resources.provider_registry.clone(),
            resources.activity_intelligence.clone(),
        );

        let config = ConfigContext::new(
            resources.config.clone(),
            resources.tenant_oauth_client.clone(),
            resources.a2a_client_manager.clone(),
            resources.a2a_system_user_service.clone(),
        );

        let notification = NotificationContext::new(
            resources.websocket_manager.clone(),
            resources.sse_manager.clone(),
            resources.oauth_notification_sender.clone(),
        );

        Self::new(auth, data, config, notification)
    }
}
