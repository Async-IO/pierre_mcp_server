// ABOUTME: Centralized resource container for dependency injection in MCP server
// ABOUTME: Manages expensive shared resources like database, auth, and OAuth managers
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Server Resources Module
//!
// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Arc sharing of expensive resources (database, auth managers) across threads
// - Resource ownership transfers for dependency injection
//!
//! Centralized resource container for dependency injection.
//! Eliminates anti-patterns of recreating expensive objects and excessive Arc cloning.

use crate::a2a::client::A2AClientManager;
use crate::a2a::system_user::A2ASystemUserService;
use crate::admin::jwks::JwksManager;
use crate::auth::AuthManager;
use crate::cache::factory::Cache;
use crate::database_plugins::factory::Database;
use crate::intelligence::ActivityIntelligence;
use crate::mcp::schema::OAuthCompletedNotification;
use crate::middleware::redaction::RedactionConfig;
use crate::middleware::McpAuthMiddleware;
use crate::plugins::executor::PluginToolExecutor;
use crate::providers::ProviderRegistry;
use crate::tenant::{oauth_manager::TenantOAuthManager, TenantOAuthClient};
use crate::websocket::WebSocketManager;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Centralized resource container for dependency injection
///
/// This struct holds all shared server resources to eliminate the anti-pattern
/// of recreating expensive objects like `AuthManager` and excessive Arc cloning.
#[derive(Clone)]
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    pub jwks_manager: Arc<JwksManager>,
    pub auth_middleware: Arc<McpAuthMiddleware>,
    pub websocket_manager: Arc<WebSocketManager>,
    pub tenant_oauth_client: Arc<TenantOAuthClient>,
    pub provider_registry: Arc<ProviderRegistry>,
    pub admin_jwt_secret: Arc<str>,
    pub config: Arc<crate::config::environment::ServerConfig>,
    pub activity_intelligence: Arc<ActivityIntelligence>,
    pub a2a_client_manager: Arc<A2AClientManager>,
    pub a2a_system_user_service: Arc<A2ASystemUserService>,
    pub oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
    pub sse_manager: Arc<crate::sse::SseManager>,
    pub cache: Arc<Cache>,
    pub plugin_executor: Option<Arc<PluginToolExecutor>>,
    pub redaction_config: Arc<RedactionConfig>,
}

impl ServerResources {
    /// Create new server resources with proper Arc sharing
    ///
    /// # Parameters
    /// - `rsa_key_size_bits`: Size of RSA keys for JWT signing (2048 for tests, 4096 for production)
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
        admin_jwt_secret: &str,
        config: Arc<crate::config::environment::ServerConfig>,
        cache: Cache,
        rsa_key_size_bits: usize,
    ) -> Self {
        let database_arc = Arc::new(database);
        let auth_manager_arc = Arc::new(auth_manager);

        // Create tenant OAuth client and provider registry once
        let tenant_oauth_client = Arc::new(TenantOAuthClient::new(TenantOAuthManager::new()));
        let provider_registry = Arc::new(ProviderRegistry::new());

        // Create activity intelligence once for shared use
        let activity_intelligence =
            std::sync::Arc::new(crate::intelligence::ActivityIntelligence::new(
                "MCP Intelligence".into(),
                vec![],
                crate::intelligence::PerformanceMetrics {
                    relative_effort: Some(7.5),
                    zone_distribution: None,
                    personal_records: vec![],
                    efficiency_score: Some(85.0),
                    trend_indicators: crate::intelligence::TrendIndicators {
                        pace_trend: crate::intelligence::TrendDirection::Improving,
                        effort_trend: crate::intelligence::TrendDirection::Stable,
                        distance_trend: crate::intelligence::TrendDirection::Improving,
                        consistency_score: 8.2,
                    },
                },
                crate::intelligence::ContextualFactors {
                    weather: None,
                    location: None,
                    time_of_day: crate::intelligence::TimeOfDay::Morning,
                    days_since_last_activity: Some(1),
                    weekly_load: None,
                },
            ));

        // Create A2A system user service once for shared use
        let a2a_system_user_service = Arc::new(A2ASystemUserService::new(database_arc.clone()));

        // Create A2A client manager once for shared use
        let a2a_client_manager = Arc::new(A2AClientManager::new(
            database_arc.clone(),
            a2a_system_user_service.clone(),
        ));

        // Create unified SSE manager for both notifications and MCP protocol
        let sse_manager = Arc::new(crate::sse::SseManager::new(config.sse.max_buffer_size));

        // Spawn background task to cleanup inactive SSE connections
        // Uses configurable intervals and timeouts from config
        {
            let manager_for_cleanup = sse_manager.clone();
            let cleanup_interval_secs = config.sse.cleanup_interval_secs;
            let connection_timeout_secs = config.sse.connection_timeout_secs;

            tokio::spawn(async move {
                let mut interval =
                    tokio::time::interval(std::time::Duration::from_secs(cleanup_interval_secs));
                loop {
                    interval.tick().await;
                    tracing::debug!(
                        "Running SSE connection cleanup task (timeout={}s)",
                        connection_timeout_secs
                    );
                    manager_for_cleanup
                        .cleanup_inactive_connections(connection_timeout_secs)
                        .await;
                }
            });
        }

        // Wrap cache in Arc for shared access across handlers
        let cache_arc = Arc::new(cache);

        // Initialize PII redaction config from environment
        let redaction_config = Arc::new(RedactionConfig::from_env());
        tracing::info!(
            "Redaction middleware initialized: enabled={}",
            redaction_config.enabled
        );

        // Create JWKS manager for RS256 JWT signing
        let mut jwks_manager = JwksManager::new();
        // Generate initial RSA key pair for RS256 signing with configurable key size
        if let Err(e) =
            jwks_manager.generate_rsa_key_pair_with_size("initial_key", rsa_key_size_bits)
        {
            tracing::warn!(
                "Failed to generate initial JWKS key pair: {}. RS256 tokens will not be available.",
                e
            );
        }
        let jwks_manager_arc = Arc::new(jwks_manager);

        // Create websocket manager after jwks_manager is initialized
        let websocket_manager = Arc::new(WebSocketManager::new(
            database_arc.clone(),
            &auth_manager_arc,
            &jwks_manager_arc,
        ));

        // Create auth middleware after jwks_manager is initialized
        let auth_middleware = Arc::new(McpAuthMiddleware::new(
            (*auth_manager_arc).clone(),
            database_arc.clone(),
            jwks_manager_arc.clone(),
        ));

        Self {
            database: database_arc,
            auth_manager: auth_manager_arc,
            jwks_manager: jwks_manager_arc,
            auth_middleware,
            websocket_manager,
            tenant_oauth_client,
            provider_registry,
            admin_jwt_secret: admin_jwt_secret.into(),
            config,
            activity_intelligence,
            a2a_client_manager,
            a2a_system_user_service,
            oauth_notification_sender: None,
            sse_manager,
            cache: cache_arc,
            plugin_executor: None,
            redaction_config,
        }
    }

    /// Set the OAuth notification sender for push notifications
    pub fn set_oauth_notification_sender(
        &mut self,
        sender: broadcast::Sender<OAuthCompletedNotification>,
    ) {
        self.oauth_notification_sender = Some(sender);
    }

    /// Set the plugin executor after `ServerResources` is wrapped in Arc
    pub fn set_plugin_executor(&mut self, executor: Arc<PluginToolExecutor>) {
        self.plugin_executor = Some(executor);
    }

    /// Create a new builder for `ServerResources`
    #[must_use]
    pub const fn builder() -> ServerResourcesBuilder {
        ServerResourcesBuilder::new()
    }
}

/// Builder pattern for `ServerResources` to avoid manual resource assembly anti-patterns
pub struct ServerResourcesBuilder {
    database: Option<Database>,
    auth_manager: Option<AuthManager>,
    admin_jwt_secret: Option<String>,
    config: Option<Arc<crate::config::environment::ServerConfig>>,
    cache: Option<Cache>,
    rsa_key_size_bits: usize,
}

impl ServerResourcesBuilder {
    /// Create a new builder with production defaults (4096-bit RSA keys)
    #[must_use]
    pub const fn new() -> Self {
        Self {
            database: None,
            auth_manager: None,
            admin_jwt_secret: None,
            config: None,
            cache: None,
            rsa_key_size_bits: 4096, // Production default
        }
    }

    /// Set the database
    #[must_use]
    pub fn with_database(mut self, database: Database) -> Self {
        self.database = Some(database);
        self
    }

    /// Set the auth manager
    #[must_use]
    pub const fn with_auth_manager(mut self, auth_manager: AuthManager) -> Self {
        self.auth_manager = Some(auth_manager);
        self
    }

    /// Set the admin JWT secret
    #[must_use]
    pub fn with_admin_jwt_secret(mut self, admin_jwt_secret: impl Into<String>) -> Self {
        self.admin_jwt_secret = Some(admin_jwt_secret.into());
        self
    }

    /// Set the server configuration
    #[must_use]
    pub fn with_config(mut self, config: Arc<crate::config::environment::ServerConfig>) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the cache
    #[must_use]
    pub fn with_cache(mut self, cache: Cache) -> Self {
        self.cache = Some(cache);
        self
    }

    /// Set the RSA key size for JWT signing (2048 for tests, 4096 for production)
    #[must_use]
    pub const fn with_rsa_key_size_bits(mut self, rsa_key_size_bits: usize) -> Self {
        self.rsa_key_size_bits = rsa_key_size_bits;
        self
    }

    /// Build the `ServerResources`
    ///
    /// # Errors
    ///
    /// Returns an error if any required fields are missing
    pub fn build(self) -> Result<ServerResources, &'static str> {
        let database = self.database.ok_or("Database is required")?;
        let auth_manager = self.auth_manager.ok_or("AuthManager is required")?;
        let admin_jwt_secret = self
            .admin_jwt_secret
            .ok_or("Admin JWT secret is required")?;
        let config = self.config.ok_or("Server config is required")?;
        let cache = self.cache.ok_or("Cache is required")?;

        let resources = ServerResources::new(
            database,
            auth_manager,
            &admin_jwt_secret,
            config,
            cache,
            self.rsa_key_size_bits,
        );
        Ok(resources)
    }

    /// Build the `ServerResources` wrapped in an `Arc`
    ///
    /// # Errors
    ///
    /// Returns an error if any required fields are missing
    pub fn build_arc(self) -> Result<Arc<ServerResources>, &'static str> {
        Ok(Arc::new(self.build()?))
    }
}

impl Default for ServerResourcesBuilder {
    fn default() -> Self {
        Self::new()
    }
}
