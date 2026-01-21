// ABOUTME: Composed server context that provides all focused contexts for gradual migration
// ABOUTME: Replaces ServerResources with focused dependency injection while maintaining compatibility
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use super::{
    AuthContext, ConfigContext, DataContext, ExtensionContext, NotificationContext, SecurityContext,
};
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
///
/// # Contexts
/// - `auth`: Authentication and authorization (auth manager, middleware, JWT, Firebase)
/// - `data`: Data access (database, cache, providers, intelligence)
/// - `config`: Configuration (server config, OAuth, A2A, admin config)
/// - `notification`: Real-time updates (WebSocket, SSE, OAuth notifications)
/// - `security`: Security features (CSRF, PII redaction, rate limiting)
/// - `extension`: Extensions (plugins, sampling peer, progress notifications)
#[derive(Clone)]
pub struct ServerContext {
    auth: AuthContext,
    data: DataContext,
    config: ConfigContext,
    notification: NotificationContext,
    security: SecurityContext,
    extension: ExtensionContext,
}

impl ServerContext {
    /// Create new server context from focused contexts
    #[must_use]
    pub const fn new(
        auth: AuthContext,
        data: DataContext,
        config: ConfigContext,
        notification: NotificationContext,
        security: SecurityContext,
        extension: ExtensionContext,
    ) -> Self {
        Self {
            auth,
            data,
            config,
            notification,
            security,
            extension,
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

    /// Get security context
    #[must_use]
    pub const fn security(&self) -> &SecurityContext {
        &self.security
    }

    /// Get extension context
    #[must_use]
    pub const fn extension(&self) -> &ExtensionContext {
        &self.extension
    }
}

impl From<&ServerResources> for ServerContext {
    /// Create server context from existing `ServerResources` for migration
    ///
    /// This implementation maps all `ServerResources` fields to their appropriate
    /// focused contexts, enabling gradual migration from the service locator pattern.
    fn from(resources: &ServerResources) -> Self {
        let auth = AuthContext::new(
            resources.auth_manager.clone(),
            resources.auth_middleware.clone(),
            resources.admin_jwt_secret.clone(),
            resources.jwks_manager.clone(),
            resources.firebase_auth.clone(),
        );

        let data = DataContext::new(
            resources.database.clone(),
            resources.cache.clone(),
            resources.provider_registry.clone(),
            resources.activity_intelligence.clone(),
        );

        let config = ConfigContext::new(
            resources.config.clone(),
            resources.tenant_oauth_client.clone(),
            resources.a2a_client_manager.clone(),
            resources.a2a_system_user_service.clone(),
            resources.admin_config.clone(),
        );

        let notification = NotificationContext::new(
            #[cfg(feature = "transport-websocket")]
            resources.websocket_manager.clone(),
            #[cfg(feature = "transport-sse")]
            resources.sse_manager.clone(),
            resources.oauth_notification_sender.clone(),
        );

        let security = SecurityContext::new(
            resources.redaction_config.clone(),
            resources.oauth2_rate_limiter.clone(),
            resources.csrf_manager.clone(),
            resources.csrf_middleware.clone(),
        );

        let extension = ExtensionContext::new(
            resources.plugin_executor.clone(),
            resources.sampling_peer.clone(),
            resources.progress_notification_sender.clone(),
            resources.cancellation_registry.clone(),
        );

        Self::new(auth, data, config, notification, security, extension)
    }
}
