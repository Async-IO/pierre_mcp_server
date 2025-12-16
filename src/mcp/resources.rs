// ABOUTME: Centralized resource container for dependency injection in MCP server
// ABOUTME: Manages expensive shared resources like database, auth, and OAuth managers
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

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
use crate::admin::FirebaseAuth;
use crate::auth::AuthManager;
use crate::cache::factory::Cache;
use crate::config::admin::AdminConfigService;
use crate::config::environment::ServerConfig;
use crate::database_plugins::factory::Database;
use crate::database_plugins::DatabaseProvider;
use crate::errors::{AppError, AppResult};
use crate::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators,
};
use crate::mcp::sampling_peer::SamplingPeer;
use crate::mcp::schema::{OAuthCompletedNotification, ProgressNotification};
use crate::middleware::redaction::RedactionConfig;
use crate::middleware::{CsrfMiddleware, McpAuthMiddleware};
use crate::oauth2_server::rate_limiting::OAuth2RateLimiter;
use crate::plugins::executor::PluginToolExecutor;
use crate::protocols::universal::types::CancellationToken;
use crate::providers::ProviderRegistry;
use crate::security::csrf::CsrfTokenManager;
use crate::sse::SseManager;
use crate::tenant::{oauth_manager::TenantOAuthManager, TenantOAuthClient};
use crate::websocket::WebSocketManager;
use chrono::Utc;
use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::task;
use tracing::{error, info, warn};

/// Centralized resource container for dependency injection
///
/// This struct holds all shared server resources to eliminate the anti-pattern
/// of recreating expensive objects like `AuthManager` and excessive Arc cloning.
#[derive(Clone)]
pub struct ServerResources {
    /// Database connection pool for persistent storage operations
    pub database: Arc<Database>,
    /// Authentication manager for user identity verification
    pub auth_manager: Arc<AuthManager>,
    /// JSON Web Key Set manager for RS256 JWT signing and verification
    pub jwks_manager: Arc<JwksManager>,
    /// Authentication middleware for MCP request validation
    pub auth_middleware: Arc<McpAuthMiddleware>,
    /// WebSocket connection manager for real-time updates
    pub websocket_manager: Arc<WebSocketManager>,
    /// Server-Sent Events manager for streaming notifications and MCP protocol
    pub sse_manager: Arc<SseManager>,
    /// OAuth client for multi-tenant authentication flows
    pub tenant_oauth_client: Arc<TenantOAuthClient>,
    /// Registry of fitness data providers (Strava, Fitbit, Garmin, WHOOP, Terra)
    pub provider_registry: Arc<ProviderRegistry>,
    /// Secret key for admin JWT token generation
    pub admin_jwt_secret: Arc<str>,
    /// Server configuration loaded from environment
    pub config: Arc<ServerConfig>,
    /// AI-powered fitness activity analysis engine
    pub activity_intelligence: Arc<ActivityIntelligence>,
    /// A2A protocol client manager for agent-to-agent communication
    pub a2a_client_manager: Arc<A2AClientManager>,
    /// Service for managing A2A system user accounts
    pub a2a_system_user_service: Arc<A2ASystemUserService>,
    /// Broadcast channel for OAuth completion notifications
    pub oauth_notification_sender: Option<broadcast::Sender<OAuthCompletedNotification>>,
    /// Cache layer for performance optimization
    pub cache: Arc<Cache>,
    /// Optional plugin executor for custom tool implementations
    pub plugin_executor: Option<Arc<PluginToolExecutor>>,
    /// Configuration for PII redaction in logs and responses
    pub redaction_config: Arc<RedactionConfig>,
    /// Rate limiter for `OAuth2` endpoints
    pub oauth2_rate_limiter: Arc<OAuth2RateLimiter>,
    /// CSRF token manager for request forgery protection
    pub csrf_manager: Arc<CsrfTokenManager>,
    /// CSRF validation middleware
    pub csrf_middleware: Arc<CsrfMiddleware>,
    /// Optional sampling peer for server-initiated LLM requests (stdio transport only)
    pub sampling_peer: Option<Arc<SamplingPeer>>,
    /// Optional progress notification sender (stdio transport only)
    pub progress_notification_sender: Option<mpsc::UnboundedSender<ProgressNotification>>,
    /// Cancellation token registry for progress token -> cancellation token mapping
    pub cancellation_registry: Arc<RwLock<HashMap<String, CancellationToken>>>,
    /// Firebase Authentication handler for social login (Google, Apple, etc.)
    pub firebase_auth: Option<Arc<FirebaseAuth>>,
    /// Admin configuration service for runtime parameter management
    pub admin_config: Option<Arc<AdminConfigService>>,
}

impl ServerResources {
    /// Create new server resources with proper Arc sharing
    ///
    /// # Parameters
    /// - `rsa_key_size_bits`: Size of RSA keys for JWT signing (2048 for tests, 4096 for production)
    /// - `jwks_manager`: Optional pre-existing JWKS manager (for test performance - reuses RSA keys)
    // Function exceeds line limit because it assembles 20+ interdependent resources
    // Splitting would reduce clarity without improving maintainability
    #[allow(clippy::too_many_lines)]
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
        admin_jwt_secret: &str,
        config: Arc<ServerConfig>,
        cache: Cache,
        rsa_key_size_bits: usize,
        jwks_manager: Option<Arc<JwksManager>>,
    ) -> Self {
        let database_arc = Arc::new(database);
        let auth_manager_arc = Arc::new(auth_manager);

        // Create tenant OAuth client and provider registry once
        let tenant_oauth_client = Arc::new(TenantOAuthClient::new(TenantOAuthManager::new(
            Arc::new(config.oauth.clone()),
        )));
        let provider_registry = Arc::new(ProviderRegistry::new());

        // Create activity intelligence once for shared use
        let activity_intelligence = Self::create_default_intelligence();

        // Create A2A system user service once for shared use
        let a2a_system_user_service = Arc::new(A2ASystemUserService::new(database_arc.clone()));

        // Create A2A client manager once for shared use
        let a2a_client_manager = Arc::new(A2AClientManager::new(
            database_arc.clone(),
            a2a_system_user_service.clone(),
        ));

        // Wrap cache in Arc for shared access across handlers
        let cache_arc = Arc::new(cache);

        // Initialize PII redaction config from environment
        let redaction_config = Arc::new(RedactionConfig::from_env());
        info!(
            "Redaction middleware initialized: enabled={}",
            redaction_config.enabled
        );

        // Use provided JWKS manager or load/create new one for RS256 JWT signing
        let jwks_manager_arc = jwks_manager.unwrap_or_else(|| {
            // Try to load persisted keys from database, blocking on async call
            let loaded_jwks = task::block_in_place(|| {
                Handle::current().block_on(async {
                    Self::load_or_create_jwks_manager(&database_arc, rsa_key_size_bits).await
                })
            });

            match loaded_jwks {
                Ok(jwks) => Arc::new(jwks),
                Err(e) => {
                    error!(
                        "Failed to initialize JWKS manager: {}. Creating new keys without persistence.",
                        e
                    );
                    let mut new_jwks = JwksManager::new();
                    if let Err(e) = new_jwks.generate_rsa_key_pair_with_size("initial_key", rsa_key_size_bits) {
                        warn!(
                            "Failed to generate initial JWKS key pair: {}. RS256 tokens will not be available.",
                            e
                        );
                    }
                    Arc::new(new_jwks)
                }
            }
        });

        // Create websocket manager after jwks_manager is initialized
        let websocket_manager = Arc::new(WebSocketManager::new(
            database_arc.clone(),
            &auth_manager_arc,
            &jwks_manager_arc,
            config.rate_limiting.clone(),
        ));

        // Create SSE manager with configured buffer size
        let sse_manager = Arc::new(SseManager::new(config.sse.max_buffer_size));

        // Create auth middleware after jwks_manager is initialized
        let auth_middleware = Arc::new(McpAuthMiddleware::new(
            (*auth_manager_arc).clone(),
            database_arc.clone(),
            jwks_manager_arc.clone(),
            config.rate_limiting.clone(),
        ));

        // Create OAuth2 rate limiter once for shared use
        let oauth2_rate_limiter = Arc::new(OAuth2RateLimiter::from_rate_limit_config(
            config.rate_limiting.clone(),
        ));

        // Create CSRF token manager for request forgery protection
        let csrf_manager = Arc::new(CsrfTokenManager::new());

        // Create CSRF validation middleware
        let csrf_middleware = Arc::new(CsrfMiddleware::new(csrf_manager.clone()));

        // Create Firebase auth handler if configured
        let firebase_auth = if config.firebase.is_configured() {
            Some(Arc::new(FirebaseAuth::new(config.firebase.clone())))
        } else {
            None
        };

        // Create admin config service if SQLite is available
        // This provides runtime-configurable parameters via admin API
        // Note: block_in_place only works on multi-threaded runtime, so we use catch_unwind
        // to gracefully handle single-threaded test environments
        let admin_config = database_arc.sqlite_pool().and_then(|pool| {
            let pool_clone = pool.clone();
            let result = catch_unwind(AssertUnwindSafe(|| {
                task::block_in_place(|| {
                    Handle::current()
                        .block_on(async { AdminConfigService::new(pool_clone).await })
                })
            }));
            match result {
                Ok(Ok(service)) => {
                    info!("Admin configuration service initialized successfully");
                    Some(Arc::new(service))
                }
                Ok(Err(e)) => {
                    warn!(
                        "Failed to initialize admin config service: {}. Runtime config will not be available.",
                        e
                    );
                    None
                }
                Err(_) => {
                    // Panic occurred - likely single-threaded runtime where block_in_place is not supported
                    warn!(
                        "Admin config service not initialized - requires multi-threaded runtime. \
                        Runtime config will use defaults."
                    );
                    None
                }
            }
        });

        Self {
            database: database_arc,
            auth_manager: auth_manager_arc,
            jwks_manager: jwks_manager_arc,
            auth_middleware,
            websocket_manager,
            sse_manager,
            tenant_oauth_client,
            provider_registry,
            admin_jwt_secret: admin_jwt_secret.into(),
            config,
            activity_intelligence,
            a2a_client_manager,
            a2a_system_user_service,
            oauth_notification_sender: None,
            cache: cache_arc,
            plugin_executor: None,
            redaction_config,
            oauth2_rate_limiter,
            csrf_manager,
            csrf_middleware,
            sampling_peer: None,
            progress_notification_sender: None,
            cancellation_registry: Arc::new(RwLock::new(HashMap::new())),
            firebase_auth,
            admin_config,
        }
    }

    /// Create default activity intelligence for MCP server
    fn create_default_intelligence() -> Arc<ActivityIntelligence> {
        Arc::new(ActivityIntelligence::new(
            "MCP Intelligence".into(),
            vec![],
            PerformanceMetrics {
                relative_effort: Some(7.5),
                zone_distribution: None,
                personal_records: vec![],
                efficiency_score: Some(85.0),
                trend_indicators: TrendIndicators {
                    pace_trend: TrendDirection::Improving,
                    effort_trend: TrendDirection::Stable,
                    distance_trend: TrendDirection::Improving,
                    consistency_score: 8.2,
                },
            },
            ContextualFactors {
                weather: None,
                location: None,
                time_of_day: TimeOfDay::Morning,
                days_since_last_activity: Some(1),
                weekly_load: None,
            },
        ))
    }

    /// Generate a unique key ID based on current timestamp
    fn generate_key_id() -> String {
        format!("key_{}", Utc::now().format("%Y%m%d_%H%M%S"))
    }

    /// Generate and persist a new RSA keypair
    async fn generate_and_persist_keypair(
        database: &Arc<Database>,
        jwks_manager: &mut JwksManager,
        rsa_key_size_bits: usize,
    ) -> AppResult<()> {
        let kid = Self::generate_key_id();
        jwks_manager.generate_rsa_key_pair_with_size(&kid, rsa_key_size_bits)?;

        let key = jwks_manager
            .get_active_key()
            .map_err(|e| AppError::internal(format!("Failed to get active key: {e}")))?;

        let private_pem = key.export_private_key_pem()?;
        let public_pem = key.export_public_key_pem()?;

        database
            .save_rsa_keypair(
                &kid,
                &private_pem,
                &public_pem,
                key.created_at,
                true,
                i32::try_from(rsa_key_size_bits).map_err(|e| {
                    AppError::internal(format!("RSA key size exceeds i32 maximum: {e}"))
                })?,
            )
            .await?;

        info!("Generated and persisted new RSA keypair: {}", kid);
        Ok(())
    }

    /// Load persisted RSA keys from database or create new ones
    ///
    /// # Errors
    /// Returns error if database operations fail
    async fn load_or_create_jwks_manager(
        database: &Arc<Database>,
        rsa_key_size_bits: usize,
    ) -> AppResult<JwksManager> {
        let mut jwks_manager = JwksManager::new();

        match database.load_rsa_keypairs().await {
            Ok(keypairs) if !keypairs.is_empty() => {
                Self::load_existing_keys(&mut jwks_manager, keypairs)?;
            }
            Ok(_) => {
                Self::generate_new_keys(database, &mut jwks_manager, rsa_key_size_bits).await?;
            }
            Err(e) => {
                Self::fallback_generate_keys(&mut jwks_manager, rsa_key_size_bits, &e)?;
            }
        }

        Ok(jwks_manager)
    }

    fn load_existing_keys(
        jwks_manager: &mut JwksManager,
        keypairs: Vec<(String, String, String, chrono::DateTime<Utc>, bool)>,
    ) -> AppResult<()> {
        info!(
            "Loading {} persisted RSA keypairs from database",
            keypairs.len()
        );
        jwks_manager.load_keys_from_database(keypairs)?;
        info!("Successfully loaded RSA keys from database");
        Ok(())
    }

    async fn generate_new_keys(
        database: &Arc<Database>,
        jwks_manager: &mut JwksManager,
        rsa_key_size_bits: usize,
    ) -> AppResult<()> {
        info!("No persisted RSA keys found, generating new keypair");
        Self::generate_and_persist_keypair(database, jwks_manager, rsa_key_size_bits).await
    }

    fn fallback_generate_keys(
        jwks_manager: &mut JwksManager,
        rsa_key_size_bits: usize,
        error: &AppError,
    ) -> AppResult<()> {
        warn!(
            "Failed to load RSA keys from database: {}. Generating new keys without persistence.",
            error
        );
        let kid = Self::generate_key_id();
        jwks_manager.generate_rsa_key_pair_with_size(&kid, rsa_key_size_bits)?;
        Ok(())
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

    /// Set the sampling peer for server-initiated LLM requests (stdio transport only)
    pub fn set_sampling_peer(&mut self, peer: Arc<SamplingPeer>) {
        self.sampling_peer = Some(peer);
    }

    /// Set the progress notification sender (stdio transport only)
    pub fn set_progress_notification_sender(
        &mut self,
        sender: mpsc::UnboundedSender<ProgressNotification>,
    ) {
        self.progress_notification_sender = Some(sender);
    }

    /// Register a cancellation token for a progress token
    pub async fn register_cancellation_token(
        &self,
        progress_token: String,
        cancellation_token: CancellationToken,
    ) {
        let mut registry = self.cancellation_registry.write().await;
        registry.insert(progress_token, cancellation_token);
    }

    /// Cancel an operation by progress token (called from MCP notifications/cancelled)
    pub async fn cancel_by_progress_token(&self, progress_token: &str) {
        let registry = self.cancellation_registry.read().await;
        if let Some(token) = registry.get(progress_token) {
            info!(
                "Cancelling operation with progress token: {}",
                progress_token
            );
            token.cancel().await;
        } else {
            warn!(
                "Received cancellation for unknown progress token: {}",
                progress_token
            );
        }
    }

    /// Cleanup a cancellation token after operation completes
    pub async fn cleanup_cancellation_token(&self, progress_token: &str) {
        let mut registry = self.cancellation_registry.write().await;
        registry.remove(progress_token);
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
    config: Option<Arc<ServerConfig>>,
    cache: Option<Cache>,
    rsa_key_size_bits: usize,
    jwks_manager: Option<Arc<JwksManager>>,
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
            jwks_manager: None,
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
    pub fn with_config(mut self, config: Arc<ServerConfig>) -> Self {
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

    /// Set a pre-existing JWKS manager (for test performance - reuses RSA keys)
    #[must_use]
    pub fn with_jwks_manager(mut self, jwks_manager: Arc<JwksManager>) -> Self {
        self.jwks_manager = Some(jwks_manager);
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
            self.jwks_manager,
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
