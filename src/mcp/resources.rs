// ABOUTME: Centralized resource container for dependency injection in MCP server
// ABOUTME: Manages expensive shared resources like database, auth, and OAuth managers
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Server Resources Module
//!
//! Centralized resource container for dependency injection.
//! Eliminates anti-patterns of recreating expensive objects and excessive Arc cloning.

use crate::a2a::client::A2AClientManager;
use crate::a2a::system_user::A2ASystemUserService;
use crate::auth::{AuthManager, McpAuthMiddleware};
use crate::database_plugins::factory::Database;
use crate::intelligence::ActivityIntelligence;
use crate::mcp::progress::ProgressTracker;
use crate::oauth::manager::OAuthManager;
use crate::providers::ProviderRegistry;
use crate::tenant::{oauth_manager::TenantOAuthManager, TenantOAuthClient};
use crate::websocket::WebSocketManager;
use std::sync::Arc;

/// Centralized resource container for dependency injection
///
/// This struct holds all shared server resources to eliminate the anti-pattern
/// of recreating expensive objects like `AuthManager` and excessive Arc cloning.
#[derive(Clone)]
pub struct ServerResources {
    pub database: Arc<Database>,
    pub auth_manager: Arc<AuthManager>,
    pub auth_middleware: Arc<McpAuthMiddleware>,
    pub websocket_manager: Arc<WebSocketManager>,
    pub tenant_oauth_client: Arc<TenantOAuthClient>,
    pub provider_registry: Arc<ProviderRegistry>,
    pub admin_jwt_secret: Arc<str>,
    pub config: Arc<crate::config::environment::ServerConfig>,
    pub activity_intelligence: Arc<ActivityIntelligence>,
    pub oauth_manager: Arc<tokio::sync::RwLock<OAuthManager>>,
    pub a2a_client_manager: Arc<A2AClientManager>,
    pub a2a_system_user_service: Arc<A2ASystemUserService>,
    pub progress_tracker: Arc<ProgressTracker>,
}

impl ServerResources {
    /// Create OAuth manager with pre-registered providers to avoid lock contention
    fn create_initialized_oauth_manager(
        database: Arc<Database>,
        config: &Arc<crate::config::environment::ServerConfig>,
    ) -> OAuthManager {
        let mut oauth_manager = OAuthManager::new(database);

        // Pre-register providers at startup to avoid write lock contention on each request
        if let Ok(strava_provider) =
            crate::oauth::providers::StravaOAuthProvider::from_config(&config.oauth.strava)
        {
            oauth_manager.register_provider(Box::new(strava_provider));
        }

        if let Ok(fitbit_provider) =
            crate::oauth::providers::FitbitOAuthProvider::from_config(&config.oauth.fitbit)
        {
            oauth_manager.register_provider(Box::new(fitbit_provider));
        }

        oauth_manager
    }

    /// Create new server resources with proper Arc sharing
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
        admin_jwt_secret: &str,
        config: Arc<crate::config::environment::ServerConfig>,
    ) -> Self {
        let database_arc = Arc::new(database);
        let auth_manager_arc = Arc::new(auth_manager);

        // Create auth middleware with shared references (no cloning)
        let auth_middleware = Arc::new(McpAuthMiddleware::new(
            (*auth_manager_arc).clone(),
            database_arc.clone(),
        ));

        // Create websocket manager with shared references (no cloning)
        let websocket_manager = Arc::new(WebSocketManager::new(
            database_arc.clone(),
            &auth_manager_arc,
        ));

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

        // Create OAuth manager once for shared use with RwLock for concurrent access
        let oauth_manager = Arc::new(tokio::sync::RwLock::new(
            Self::create_initialized_oauth_manager(database_arc.clone(), &config),
        ));

        // Create A2A system user service once for shared use
        let a2a_system_user_service = Arc::new(A2ASystemUserService::new(database_arc.clone()));

        // Create A2A client manager once for shared use
        let a2a_client_manager = Arc::new(A2AClientManager::new(
            database_arc.clone(),
            a2a_system_user_service.clone(),
        ));

        // Create progress tracker for MCP operations
        let progress_tracker = Arc::new(ProgressTracker::new());

        Self {
            database: database_arc,
            auth_manager: auth_manager_arc,
            auth_middleware,
            websocket_manager,
            tenant_oauth_client,
            provider_registry,
            admin_jwt_secret: admin_jwt_secret.into(),
            config,
            activity_intelligence,
            oauth_manager,
            a2a_client_manager,
            a2a_system_user_service,
            progress_tracker,
        }
    }
}
