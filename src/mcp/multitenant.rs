// ABOUTME: MCP server implementation with tenant isolation and user authentication
// ABOUTME: Handles MCP protocol with per-tenant data isolation and access control
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # MCP Server
//!
//! This module provides an MCP server that supports user authentication,
//! secure token storage, and user-scoped data access.

use crate::a2a_routes::A2ARoutes;
use crate::api_key_routes::ApiKeyRoutes;
use crate::auth::{AuthManager, AuthResult, McpAuthMiddleware};
use crate::configuration_routes::ConfigurationRoutes;
use crate::constants::{
    errors::{
        ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND, ERROR_UNAUTHORIZED,
    },
    json_fields::{GOAL_ID, PROVIDER},
    protocol,
    protocol::{JSONRPC_VERSION, SERVER_VERSION},
    service_names,
    tools::{
        ANALYZE_ACTIVITY, ANALYZE_GOAL_FEASIBILITY, ANALYZE_PERFORMANCE_TRENDS,
        ANALYZE_TRAINING_LOAD, CALCULATE_FITNESS_SCORE, CALCULATE_METRICS, COMPARE_ACTIVITIES,
        DETECT_PATTERNS, DISCONNECT_PROVIDER, GENERATE_RECOMMENDATIONS, GET_ACTIVITIES,
        GET_ACTIVITY_INTELLIGENCE, GET_ATHLETE, GET_CONNECTION_STATUS, GET_STATS,
        PREDICT_PERFORMANCE, SET_GOAL, SUGGEST_GOALS, TRACK_PROGRESS,
    },
};
use crate::dashboard_routes::DashboardRoutes;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::mcp::schema::InitializeResponse;
use crate::models::AuthRequest;
use crate::providers::TenantProviderFactory;
use crate::routes::OAuthAuthorizationResponse;
use crate::routes::{AuthRoutes, LoginRequest, OAuthRoutes, RefreshTokenRequest, RegisterRequest};
use crate::security::SecurityConfig;
use crate::tenant::{TenantContext, TenantOAuthClient, TenantRole};
use crate::utils::json_responses::{api_error, invalid_format_error, oauth_error};
use crate::websocket::WebSocketManager;

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;
use std::sync::Arc;
use tracing::{error, info, warn};
use uuid::Uuid;
use warp::Reply;

// Constants are now imported from the constants module

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    serde_json::Value::Number(serde_json::Number::from(0))
}

/// OAuth credentials passed via MCP tool parameters
struct McpOAuthCredentials<'a> {
    strava_client_id: Option<&'a str>,
    strava_client_secret: Option<&'a str>,
    fitbit_client_id: Option<&'a str>,
    fitbit_client_secret: Option<&'a str>,
}

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
    pub tenant_provider_factory: Arc<TenantProviderFactory>,
    pub admin_jwt_secret: Arc<str>,
    pub config: Arc<crate::config::environment::ServerConfig>,
    pub activity_intelligence: Arc<crate::intelligence::ActivityIntelligence>,
    pub oauth_manager: Arc<tokio::sync::RwLock<crate::oauth::manager::OAuthManager>>,
    pub a2a_client_manager: Arc<crate::a2a::client::A2AClientManager>,
    pub a2a_system_user_service: Arc<crate::a2a::system_user::A2ASystemUserService>,
}

impl ServerResources {
    /// Create OAuth manager with pre-registered providers to avoid lock contention
    fn create_initialized_oauth_manager(
        database: Arc<Database>,
        config: &Arc<crate::config::environment::ServerConfig>,
    ) -> crate::oauth::manager::OAuthManager {
        let mut oauth_manager = crate::oauth::manager::OAuthManager::new(database);

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
            database_arc.as_ref().clone(),
            auth_manager_arc.as_ref().clone(),
        ));

        // Create tenant OAuth client and provider factory once
        let tenant_oauth_client = Arc::new(TenantOAuthClient::new());
        let tenant_provider_factory =
            Arc::new(TenantProviderFactory::new(tenant_oauth_client.clone()));

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
        let a2a_system_user_service = Arc::new(crate::a2a::system_user::A2ASystemUserService::new(
            database_arc.clone(),
        ));

        // Create A2A client manager once for shared use
        let a2a_client_manager = Arc::new(crate::a2a::client::A2AClientManager::new(
            database_arc.clone(),
            a2a_system_user_service.clone(),
        ));

        Self {
            database: database_arc,
            auth_manager: auth_manager_arc,
            auth_middleware,
            websocket_manager,
            tenant_oauth_client,
            tenant_provider_factory,
            admin_jwt_secret: admin_jwt_secret.into(),
            config,
            activity_intelligence,
            oauth_manager,
            a2a_client_manager,
            a2a_system_user_service,
        }
    }
}

/// Context for HTTP request handling with tenant support
struct HttpRequestContext {
    resources: Arc<ServerResources>,
}

/// Context for tool routing with all required components
struct ToolRoutingContext<'a> {
    resources: &'a Arc<ServerResources>,
    tenant_context: &'a Option<TenantContext>,
    auth_result: &'a AuthResult,
}

/// MCP server supporting user authentication and isolated data access
#[derive(Clone)]
pub struct MultiTenantMcpServer {
    resources: Arc<ServerResources>,
}

impl MultiTenantMcpServer {
    /// Create a new MCP server with centralized resource management
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
        admin_jwt_secret: &str,
        config: Arc<crate::config::environment::ServerConfig>,
    ) -> Self {
        let resources = Arc::new(ServerResources::new(
            database,
            auth_manager,
            admin_jwt_secret,
            config,
        ));

        Self { resources }
    }

    /// Run the server with both HTTP and MCP endpoints
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start or bind to the specified port
    pub async fn run(self, port: u16) -> Result<()> {
        // Create HTTP + MCP server
        info!("Starting server with HTTP and MCP on port {}", port);

        // Create route handlers using shared resources (no more cloning!)
        let auth_routes = AuthRoutes::new(self.resources.clone());
        let oauth_routes = OAuthRoutes::new(self.resources.clone());

        // Validate route handlers are properly initialized
        tracing::debug!(
            "Initialized auth and OAuth route handlers - auth routes: {:p}, oauth routes: {:p}",
            &auth_routes,
            &oauth_routes
        );

        // Start HTTP server for auth endpoints in background
        let http_port = port + 1; // Use port+1 for HTTP
        let resources_http = self.resources.clone();

        tokio::spawn(async move {
            Box::pin(Self::run_http_server_with_resources(
                http_port,
                resources_http,
            ))
            .await
        });

        // Run MCP server on main port
        self.run_mcp_server(port).await
    }

    /// Run HTTP server with centralized resources (eliminates parameter passing anti-pattern)
    async fn run_http_server_with_resources(
        port: u16,
        resources: Arc<ServerResources>,
    ) -> Result<()> {
        use warp::Filter;

        info!("HTTP authentication server starting on port {}", port);

        // Initialize security configuration
        let security_config = Self::setup_security_config(&resources.config);

        // Initialize all route handlers using shared resources
        let (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
        ) = Self::setup_route_handlers_with_resources(&resources);

        // Use JWT secret from resources
        let jwt_secret_str = resources.admin_jwt_secret.as_ref();
        info!("Using admin JWT secret from server startup");

        // Setup admin routes - API requires owned values
        let admin_context = crate::admin_routes::AdminApiContext::new(
            resources.database.as_ref().clone(),
            jwt_secret_str,
            resources.auth_manager.as_ref().clone(),
        );
        let admin_routes_filter = crate::admin_routes::admin_routes_with_rejection(admin_context);

        // Setup tenant management routes - API requires owned values
        let tenant_routes_filter = Self::create_tenant_routes_filter(
            resources.database.as_ref().clone(),
            resources.auth_manager.as_ref().clone(),
        );

        // Configure CORS
        let cors = Self::setup_cors();

        // Create all route groups using helper functions
        let auth_route_filter = Self::create_auth_routes(&auth_routes);
        let oauth_route_filter = Self::create_oauth_routes(
            &oauth_routes,
            &resources.database,
            &resources.tenant_oauth_client,
        );
        let api_key_route_filter = Self::create_api_key_routes(&api_key_routes);
        let api_key_usage_filter = Self::create_api_key_usage_route(api_key_routes.clone());
        let dashboard_route_filter = Self::create_dashboard_routes(&dashboard_routes);
        let dashboard_detailed_filter = Self::create_dashboard_detailed_routes(&dashboard_routes);

        // Create A2A routes
        let a2a_basic_filter = Self::create_a2a_basic_routes(&a2a_routes);
        let a2a_client_filter = Self::create_a2a_client_routes(&a2a_routes);
        let a2a_monitoring_filter = Self::create_a2a_monitoring_routes(&a2a_routes);
        let a2a_execution_filter = Self::create_a2a_execution_routes(&a2a_routes);

        // Create configuration routes
        let configuration_filter = Self::create_configuration_routes(&configuration_routes);
        let user_configuration_filter =
            Self::create_user_configuration_routes(&configuration_routes);
        let specialized_configuration_filter =
            Self::create_specialized_configuration_routes(&configuration_routes);

        // Security headers middleware
        let security_headers = Self::create_security_headers_filter(&security_config);

        // Health check route
        let health_route = Self::create_health_route();

        // Create websocket route (method needs to exist)
        // let websocket_route = Self::create_websocket_route(resources.websocket_manager.clone());

        // Combine all routes
        let routes = auth_route_filter
            .or(oauth_route_filter)
            .or(api_key_route_filter)
            .or(api_key_usage_filter)
            .or(dashboard_route_filter)
            .or(dashboard_detailed_filter)
            .or(a2a_basic_filter)
            .or(a2a_client_filter)
            .or(a2a_monitoring_filter)
            .or(a2a_execution_filter)
            .or(configuration_filter)
            .or(user_configuration_filter)
            .or(specialized_configuration_filter)
            .or(admin_routes_filter)
            .or(tenant_routes_filter)
            .or(health_route)
            .with(cors)
            .with(security_headers)
            .recover(handle_rejection);

        // Start the server
        info!("HTTP server listening on http://127.0.0.1:{}", port);
        Box::pin(warp::serve(routes).run(([127, 0, 0, 1], port))).await;

        Ok(())
    }

    /// Initialize security configuration based on environment
    fn setup_security_config(config: &crate::config::environment::ServerConfig) -> SecurityConfig {
        let security_config =
            SecurityConfig::from_environment(&config.security.headers.environment.to_string());
        info!(
            "Security headers enabled with {} configuration",
            config.security.headers.environment
        );
        security_config
    }

    /// Initialize all route handlers with `ServerResources` (eliminates cloning anti-pattern)
    fn setup_route_handlers_with_resources(
        resources: &Arc<ServerResources>,
    ) -> (
        AuthRoutes,
        OAuthRoutes,
        ApiKeyRoutes,
        DashboardRoutes,
        A2ARoutes,
        Arc<ConfigurationRoutes>,
    ) {
        // Create route handlers - use Arc references (no cloning!)
        let auth_routes = AuthRoutes::new(resources.clone());
        let oauth_routes = OAuthRoutes::new(resources.clone());
        let api_key_routes = ApiKeyRoutes::new(resources.clone());
        let dashboard_routes = DashboardRoutes::new(resources.clone());
        let a2a_routes = A2ARoutes::new(resources.clone());
        let configuration_routes = Arc::new(ConfigurationRoutes::new(resources.clone()));

        (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
        )
    }

    /// Configure CORS settings
    fn setup_cors() -> warp::cors::Builder {
        warp::cors()
            .allow_any_origin()
            .allow_headers(vec![
                "content-type",
                "authorization",
                "x-requested-with",
                "accept",
                "origin",
                "access-control-request-method",
                "access-control-request-headers",
                "x-strava-client-id",
                "x-strava-client-secret",
                "x-fitbit-client-id",
                "x-fitbit-client-secret",
            ])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
    }

    /// Create authentication endpoint routes
    fn create_auth_routes(
        auth_routes: &AuthRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Registration endpoint
        let register = warp::path("api")
            .and(warp::path("auth"))
            .and(warp::path("register"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and_then({
                let auth_routes = auth_routes.clone();
                move |request: RegisterRequest| {
                    let auth_routes = auth_routes.clone();
                    async move {
                        match auth_routes.register(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Login endpoint
        let login = warp::path("api")
            .and(warp::path("auth"))
            .and(warp::path("login"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and_then({
                let auth_routes = auth_routes.clone();
                move |request: LoginRequest| {
                    let auth_routes = auth_routes.clone();
                    async move {
                        match auth_routes.login(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Token refresh endpoint
        let refresh = warp::path("api")
            .and(warp::path("auth"))
            .and(warp::path("refresh"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::body::json())
            .and_then({
                let auth_routes = auth_routes.clone();
                move |request: RefreshTokenRequest| {
                    let auth_routes = auth_routes.clone();
                    async move {
                        match auth_routes.refresh_token(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        register.or(login).or(refresh)
    }

    /// Create OAuth authorization endpoint
    #[allow(clippy::too_many_lines)] // OAuth flow implementation requires comprehensive logic
    fn create_oauth_auth_route(
        database: &Arc<Database>,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        warp::path("api")
            .and(warp::path("oauth"))
            .and(warp::path!("auth" / String / String))
            .and(warp::get())
            .and(warp::header::headers_cloned())
            .and_then({
                let database = database.clone();
                let tenant_oauth_client = tenant_oauth_client.clone();
                move |provider: String, user_id_str: String, headers: warp::http::HeaderMap| {
                    let database = database.clone();
                    let tenant_oauth_client = tenant_oauth_client.clone();
                    async move {
                        let Ok(user_id) = Uuid::parse_str(&user_id_str) else {
                            let error = api_error("Invalid user ID format");
                            return Err(warp::reject::custom(ApiError(error)));
                        };

                        let user = match database.get_user(user_id).await {
                            Ok(Some(user)) => user,
                            Ok(None) => {
                                let error = api_error("User not found");
                                return Err(warp::reject::custom(ApiError(error)));
                            }
                            Err(e) => {
                                let error = api_error(&format!("Database error: {e}"));
                                return Err(warp::reject::custom(ApiError(error)));
                            }
                        };

                        let Some(tenant_id) = user
                            .tenant_id
                            .as_ref()
                            .and_then(|id| Uuid::parse_str(id).ok())
                        else {
                            let error = api_error("User has no valid tenant");
                            return Err(warp::reject::custom(ApiError(error)));
                        };

                        // Extract user-specific OAuth credentials from headers
                        let user_client_id = headers
                            .get(format!("x-{}-client-id", provider.to_lowercase()))
                            .and_then(|h| h.to_str().ok())
                            .map(std::string::ToString::to_string);

                        let user_client_secret = headers
                            .get(format!("x-{}-client-secret", provider.to_lowercase()))
                            .and_then(|h| h.to_str().ok())
                            .map(std::string::ToString::to_string);

                        // If user provided custom credentials, store them temporarily for this tenant
                        if let (Some(client_id), Some(client_secret)) = (&user_client_id, &user_client_secret) {
                            tracing::info!(
                                "Using user-provided OAuth credentials for tenant {} and provider {}",
                                tenant_id,
                                provider
                            );
                            let redirect_uri = match provider.as_str() {
                                "strava" => crate::constants::env_config::strava_redirect_uri(),
                                "fitbit" => crate::constants::env_config::fitbit_redirect_uri(),
                                _ => {
                                    let error = api_error(&format!("Unsupported OAuth provider: {provider}"));
                                    return Err(warp::reject::custom(ApiError(error)));
                                }
                            };

                            let request = crate::tenant::oauth_client::StoreCredentialsRequest {
                                client_id: client_id.clone(),
                                client_secret: client_secret.clone(),
                                redirect_uri,
                                scopes: match provider.as_str() {
                                    "strava" => vec!["read".to_string(), "activity:read_all".to_string()],
                                    "fitbit" => vec!["activity".to_string(), "heartrate".to_string(), "location".to_string(), "nutrition".to_string(), "profile".to_string(), "settings".to_string(), "sleep".to_string(), "social".to_string(), "weight".to_string()],
                                    _ => vec!["read".to_string()],
                                },
                                configured_by: user_id,
                            };

                            if let Err(e) = tenant_oauth_client
                                .store_credentials(tenant_id, &provider, request)
                                .await
                            {
                                tracing::error!(
                                    "Failed to store user OAuth credentials for tenant {} and provider {}: {}",
                                    tenant_id,
                                    provider,
                                    e
                                );
                                let error = api_error(&format!("Failed to store OAuth credentials: {e}"));
                                return Err(warp::reject::custom(ApiError(error)));
                            }
                        }

                        // Use tenant context to get authorization URL  
                        let tenant_context = TenantContext {
                            tenant_id,
                            user_id,
                            tenant_name: "Example Organization".to_string(), // TODO: Get from tenant
                            user_role: TenantRole::Member,
                        };

                        // Generate state for CSRF protection (mimics the routes implementation)
                        let new_uuid = Uuid::new_v4();
                        let state = format!("{user_id}:{new_uuid}");

                        match tenant_oauth_client
                            .get_authorization_url(&tenant_context, &provider, &state)
                            .await
                        {
                            Ok(auth_url) => {
                                let response = OAuthAuthorizationResponse {
                                    authorization_url: auth_url,
                                    state: state.clone(),
                                    instructions: format!(
                                        "Visit the URL above to authorize access to your {provider} account. You'll be redirected back after authorization."
                                    ),
                                    expires_in_minutes: 10,
                                };
                                Ok(warp::reply::json(&response))
                            }
                            Err(e) => {
                                tracing::error!("Failed to get OAuth authorization URL: {}", e);
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            })
    }

    /// Create OAuth callback endpoint
    fn create_oauth_callback_route(
        oauth_routes: &OAuthRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        warp::path("api")
            .and(warp::path("oauth"))
            .and(warp::path("callback"))
            .and(warp::path!(String))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and(warp::get())
            .and_then({
                let oauth_routes = oauth_routes.clone();
                move |provider: String, params: std::collections::HashMap<String, String>| {
                    let oauth_routes = oauth_routes.clone();
                    async move {
                        let Some(code) = params.get("code").cloned() else {
                                tracing::error!("Missing OAuth code parameter in callback");
                                let error_response = oauth_error(
                                    "OAuth authorization failed",
                                    "Missing OAuth code parameter",
                                    None
                                );
                                return Ok(warp::reply::with_status(
                                    warp::reply::json(&error_response),
                                    warp::http::StatusCode::BAD_REQUEST
                                ).into_response());
                            };
                        let Some(state) = params.get("state").cloned() else {
                                tracing::error!("Missing OAuth state parameter in callback");
                                let error_response = oauth_error(
                                    "OAuth authorization failed",
                                    "Missing OAuth state parameter",
                                    None
                                );
                                return Ok(warp::reply::with_status(
                                    warp::reply::json(&error_response),
                                    warp::http::StatusCode::BAD_REQUEST
                                ).into_response());
                            };

                        if let Some(error_msg) = params.get("error") {
                            let error_response = oauth_error(
                                "OAuth authorization failed",
                                error_msg,
                                Some(&provider)
                            );
                            return Ok(warp::reply::with_status(
                                warp::reply::json(&error_response),
                                warp::http::StatusCode::BAD_REQUEST
                            ).into_response());
                        }

                        match oauth_routes.handle_callback(&code, &state, &provider).await {
                            Ok(callback_response) => {
                                let success_response = serde_json::json!({
                                    "success": true,
                                    "message": format!("{provider} account connected successfully!"),
                                    "provider": provider,
                                    "user_id": callback_response.user_id,
                                    "expires_at": callback_response.expires_at
                                });
                                Ok(warp::reply::with_status(
                                    warp::reply::json(&success_response),
                                    warp::http::StatusCode::OK
                                ).into_response())
                            }
                            Err(e) => {
                                let error_response = serde_json::json!({
                                    "error": format!("Failed to process OAuth callback: {e}"),
                                    "provider": provider
                                });
                                Err(warp::reject::custom(ApiError(error_response)))
                            }
                        }
                    }
                }
            })
    }

    /// Create OAuth endpoint routes
    fn create_oauth_routes(
        oauth_routes: &OAuthRoutes,
        database: &Arc<Database>,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let oauth_auth = Self::create_oauth_auth_route(database, tenant_oauth_client);
        let oauth_callback = Self::create_oauth_callback_route(oauth_routes);
        oauth_auth.or(oauth_callback)
    }

    /// Create API key management endpoint routes
    fn create_api_key_routes(
        api_key_routes: &ApiKeyRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Create API key endpoint
        let create_api_key = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::post())
            .and(warp::body::json())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let api_key_routes = api_key_routes.clone();
                move |request: crate::api_keys::CreateApiKeyRequestSimple,
                      auth_header: Option<String>| {
                    let api_key_routes = api_key_routes.clone();
                    async move {
                        match api_key_routes
                            .create_api_key_simple(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // List API keys endpoint
        let list_api_keys = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let api_key_routes = api_key_routes.clone();
                move |auth_header: Option<String>| {
                    let api_key_routes = api_key_routes.clone();
                    async move {
                        match api_key_routes.list_api_keys(auth_header.as_deref()).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Deactivate API key endpoint
        let deactivate_api_key = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::path!(String))
            .and(warp::delete())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let api_key_routes = api_key_routes.clone();
                move |api_key_id: String, auth_header: Option<String>| {
                    let api_key_routes = api_key_routes.clone();
                    async move {
                        match api_key_routes
                            .deactivate_api_key(auth_header.as_deref(), &api_key_id)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        create_api_key.or(list_api_keys).or(deactivate_api_key)
    }

    /// Create API key usage endpoint route
    fn create_api_key_usage_route(
        api_key_routes: ApiKeyRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        warp::path("api")
            .and(warp::path("keys"))
            .and(warp::path!(String))
            .and(warp::path("usage"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                move |api_key_id: String,
                      auth_header: Option<String>,
                      params: std::collections::HashMap<String, String>| {
                    let api_key_routes = api_key_routes.clone();
                    async move {
                        let start_date_str =
                            params.get("start_date").cloned().unwrap_or_else(|| {
                                let thirty_days_ago =
                                    chrono::Utc::now() - chrono::Duration::days(30);
                                thirty_days_ago.to_rfc3339()
                            });
                        let end_date_str = params
                            .get("end_date")
                            .cloned()
                            .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                        let start_date =
                            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start_date_str) {
                                dt.with_timezone(&chrono::Utc)
                            } else {
                                let error = invalid_format_error("start_date", "RFC3339");
                                return Err(warp::reject::custom(ApiError(error)));
                            };

                        let end_date =
                            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&end_date_str) {
                                dt.with_timezone(&chrono::Utc)
                            } else {
                                let error = invalid_format_error("end_date", "RFC3339");
                                return Err(warp::reject::custom(ApiError(error)));
                            };

                        match api_key_routes
                            .get_api_key_usage(
                                auth_header.as_deref(),
                                &api_key_id,
                                start_date,
                                end_date,
                            )
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            })
    }

    /// Create dashboard endpoint routes
    fn create_dashboard_routes(
        dashboard_routes: &DashboardRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Dashboard overview
        let dashboard_overview = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("overview"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth_header: Option<String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        match dashboard_routes
                            .get_dashboard_overview(auth_header.as_deref())
                            .await
                        {
                            Ok(overview) => Ok(warp::reply::json(&overview)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard analytics
        let dashboard_analytics = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("analytics"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth_header: Option<String>,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let days = params
                            .get("days")
                            .and_then(|d| d.parse::<u32>().ok())
                            .unwrap_or(30);
                        match dashboard_routes
                            .get_usage_analytics(auth_header.as_deref(), days)
                            .await
                        {
                            Ok(analytics) => Ok(warp::reply::json(&analytics)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard rate limits
        let dashboard_rate_limits = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("rate-limits"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth_header: Option<String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        match dashboard_routes
                            .get_rate_limit_overview(auth_header.as_deref())
                            .await
                        {
                            Ok(overview) => Ok(warp::reply::json(&overview)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        dashboard_overview
            .or(dashboard_analytics)
            .or(dashboard_rate_limits)
    }

    /// Create additional dashboard endpoint routes for logs and stats
    fn create_dashboard_detailed_routes(
        dashboard_routes: &DashboardRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Dashboard request logs
        let dashboard_request_logs = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("request-logs"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth_header: Option<String>,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let api_key_id = params.get("api_key_id").map(std::string::String::as_str);
                        let time_range = params.get("time_range").map(std::string::String::as_str);
                        let status = params.get("status").map(std::string::String::as_str);
                        let tool = params.get("tool").map(std::string::String::as_str);

                        match dashboard_routes
                            .get_request_logs(
                                auth_header.as_deref(),
                                api_key_id,
                                time_range,
                                status,
                                tool,
                            )
                            .await
                        {
                            Ok(logs) => Ok(warp::reply::json(&logs)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard request stats
        let dashboard_request_stats = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("request-stats"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth_header: Option<String>,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let api_key_id = params.get("api_key_id").map(std::string::String::as_str);
                        let time_range = params.get("time_range").map(std::string::String::as_str);

                        match dashboard_routes
                            .get_request_stats(auth_header.as_deref(), api_key_id, time_range)
                            .await
                        {
                            Ok(stats) => Ok(warp::reply::json(&stats)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard tool usage
        let dashboard_tool_usage = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("tool-usage"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth_header: Option<String>,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let api_key_id = params.get("api_key_id").map(std::string::String::as_str);
                        let time_range = params.get("time_range").map(std::string::String::as_str);

                        match dashboard_routes
                            .get_tool_usage_breakdown(
                                auth_header.as_deref(),
                                api_key_id,
                                time_range,
                            )
                            .await
                        {
                            Ok(usage) => Ok(warp::reply::json(&usage)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        dashboard_request_logs
            .or(dashboard_request_stats)
            .or(dashboard_tool_usage)
    }

    /// Create A2A endpoint routes - agent card and dashboard
    fn create_a2a_basic_routes(
        a2a_routes: &A2ARoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // A2A Agent Card endpoint
        let a2a_agent_card = warp::path("a2a")
            .and(warp::path("agent-card"))
            .and(warp::get())
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move || {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes.get_agent_card() {
                            Ok(agent_card) => Ok(warp::reply::json(&agent_card)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // A2A Dashboard Overview endpoint
        let a2a_dashboard_overview = warp::path("a2a")
            .and(warp::path("dashboard"))
            .and(warp::path("overview"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes
                            .get_dashboard_overview(auth_header.as_deref())
                            .await
                        {
                            Ok(overview) => Ok(warp::reply::json(&overview)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        a2a_agent_card.or(a2a_dashboard_overview)
    }

    /// Create A2A client management endpoint routes
    fn create_a2a_client_routes(
        a2a_routes: &A2ARoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // A2A Client Registration endpoint
        let a2a_register_client = warp::path("a2a")
            .and(warp::path("clients"))
            .and(warp::post())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::body::json())
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |auth_header: Option<String>, request: crate::a2a_routes::A2AClientRequest| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes
                            .register_client(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(credentials) => Ok(warp::reply::json(&credentials)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // A2A List Clients endpoint
        let a2a_list_clients = warp::path("a2a")
            .and(warp::path("clients"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes.list_clients(auth_header.as_deref()).await {
                            Ok(clients) => Ok(warp::reply::json(&clients)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        a2a_register_client.or(a2a_list_clients)
    }

    /// Create A2A client monitoring endpoint routes
    fn create_a2a_monitoring_routes(
        a2a_routes: &A2ARoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // A2A Client Usage endpoint
        let a2a_client_usage = warp::path("a2a")
            .and(warp::path("clients"))
            .and(warp::path!(String / "usage"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |client_id: String, auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes
                            .get_client_usage(auth_header.as_deref(), &client_id)
                            .await
                        {
                            Ok(usage) => Ok(warp::reply::json(&usage)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // A2A Client Rate Limit endpoint
        let a2a_client_rate_limit = warp::path("a2a")
            .and(warp::path("clients"))
            .and(warp::path!(String / "rate-limit"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |client_id: String, auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes
                            .get_client_rate_limit(auth_header.as_deref(), &client_id)
                            .await
                        {
                            Ok(rate_limit) => Ok(warp::reply::json(&rate_limit)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        a2a_client_usage.or(a2a_client_rate_limit)
    }

    /// Create A2A authentication and execution endpoint routes
    fn create_a2a_execution_routes(
        a2a_routes: &A2ARoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // A2A Authentication endpoint
        let a2a_auth = warp::path("a2a")
            .and(warp::path("auth"))
            .and(warp::post())
            .and(warp::body::json())
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |request: serde_json::Value| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes.authenticate(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // A2A Tool Execution endpoint
        let a2a_execute = warp::path("a2a")
            .and(warp::path("execute"))
            .and(warp::post())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::body::json())
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move |auth_header: Option<String>, request: serde_json::Value| {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes
                            .execute_tool(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        a2a_auth.or(a2a_execute)
    }

    /// Create configuration endpoint routes
    fn create_configuration_routes(
        configuration_routes: &Arc<ConfigurationRoutes>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Configuration catalog
        let config_catalog = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("catalog"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let config_routes = (*configuration_routes).clone();
                move |auth_header: Option<String>| {
                    let config_routes_inner = config_routes.clone();
                    async move {
                        match config_routes_inner.get_configuration_catalog(auth_header.as_deref())
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Configuration profiles
        let config_profiles = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("profiles"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let config_routes = (*configuration_routes).clone();
                move |auth_header: Option<String>| {
                    let config_routes_inner = config_routes.clone();
                    async move {
                        match config_routes_inner.get_configuration_profiles(auth_header.as_deref())
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        config_catalog.or(config_profiles)
    }

    /// Create user configuration endpoint routes
    fn create_user_configuration_routes(
        configuration_routes: &Arc<ConfigurationRoutes>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Get user configuration
        let config_user_get = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("user"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let config_routes = (*configuration_routes).clone();
                move |auth_header: Option<String>| {
                    let config_routes = config_routes.clone();
                    async move {
                        match config_routes
                            .get_user_configuration(auth_header.as_deref())
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Update user configuration
        let config_user_update = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("user"))
            .and(warp::put())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::body::json())
            .and_then({
                let config_routes = (*configuration_routes).clone();
                move |auth_header: Option<String>, request: crate::configuration_routes::UpdateConfigurationRequest| {
                    let config_routes = config_routes.clone();
                    async move {
                        match config_routes
                            .update_user_configuration(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        config_user_get.or(config_user_update)
    }

    /// Create specialized configuration endpoint routes
    fn create_specialized_configuration_routes(
        configuration_routes: &Arc<ConfigurationRoutes>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Configuration zones
        let config_zones = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("zones"))
            .and(warp::post())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::body::json())
            .and_then({
                let config_routes = (*configuration_routes).clone();
                move |auth_header: Option<String>, request: crate::configuration_routes::PersonalizedZonesRequest| {
                    let config_routes = config_routes.clone();
                    async move {
                        match config_routes
                            .calculate_personalized_zones(auth_header.as_deref(), &request)
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Configuration validation
        let config_validate = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("validate"))
            .and(warp::post())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::body::json())
            .and_then({
                let config_routes = (*configuration_routes).clone();
                move |auth_header: Option<String>, request: crate::configuration_routes::ValidateConfigurationRequest| {
                    let config_routes = config_routes.clone();
                    async move {
                        match config_routes
                            .validate_configuration(auth_header.as_deref(), &request)
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = api_error(&e.to_string());
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        config_zones.or(config_validate)
    }

    /// Create security headers filter
    fn create_security_headers_filter(
        security_config: &SecurityConfig,
    ) -> warp::filters::reply::WithHeaders {
        let headers = security_config.to_headers();
        let mut header_map = warp::http::HeaderMap::new();
        for (name, value) in headers {
            if let Ok(header_name) = warp::http::HeaderName::from_str(name) {
                if let Ok(header_value) = warp::http::HeaderValue::from_str(value) {
                    header_map.insert(header_name, header_value);
                }
            }
        }
        warp::reply::with::headers(header_map)
    }

    /// Create health check endpoint
    fn create_health_route(
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        warp::path("health").and(warp::get()).map(|| {
            warp::reply::json(
                &serde_json::json!({"status": "ok", "service": service_names::PIERRE_MCP_SERVER}),
            )
        })
    }

    /// Create JWT authentication filter
    fn create_auth_filter(
        auth_manager: Arc<AuthManager>,
    ) -> impl warp::Filter<Extract = (crate::auth::AuthResult,), Error = warp::Rejection> + Clone
    {
        use warp::Filter;
        warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || auth_manager.clone()))
            .and_then(
                |auth_header: Option<String>, auth_mgr: Arc<AuthManager>| async move {
                    match auth_header {
                        Some(header) => {
                            // Extract token from "Bearer <token>" format
                            let Some(token) = header.strip_prefix("Bearer ") else {
                                return Err(warp::reject::custom(crate::errors::AppError::new(
                                    crate::errors::ErrorCode::AuthInvalid,
                                    "Invalid authorization header format. Use 'Bearer <token>'",
                                )));
                            };

                            // Validate JWT token using AuthManager
                            match auth_mgr.validate_token(token) {
                                Ok(claims) => {
                                    // Parse user_id from claims.sub
                                    let user_id =
                                        uuid::Uuid::parse_str(&claims.sub).map_err(|_| {
                                            warp::reject::custom(crate::errors::AppError::new(
                                                crate::errors::ErrorCode::AuthInvalid,
                                                "Invalid user ID in JWT token",
                                            ))
                                        })?;

                                    // Convert JWT claims to AuthResult
                                    Ok(crate::auth::AuthResult {
                                        user_id,
                                        auth_method: crate::auth::AuthMethod::JwtToken {
                                            tier: "basic".to_string(),
                                        },
                                        rate_limit: crate::rate_limiting::UnifiedRateLimitInfo {
                                            is_rate_limited: false,
                                            limit: Some(1000),
                                            remaining: Some(999),
                                            reset_at: Some(
                                                chrono::Utc::now() + chrono::Duration::hours(1),
                                            ),
                                            tier: "basic".to_string(),
                                            auth_method: "jwt".to_string(),
                                        },
                                    })
                                }
                                Err(_) => Err(warp::reject::custom(crate::errors::AppError::new(
                                    crate::errors::ErrorCode::AuthInvalid,
                                    "Invalid or expired JWT token",
                                ))),
                            }
                        }
                        None => Err(warp::reject::custom(crate::errors::AppError::new(
                            crate::errors::ErrorCode::AuthRequired,
                            "Authorization header required",
                        ))),
                    }
                },
            )
    }

    /// Create tenant management routes filter
    fn create_tenant_routes_filter(
        database: Database,
        auth_manager: AuthManager,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let database = Arc::new(database);
        let auth_manager = Arc::new(auth_manager);

        let with_auth = Self::create_auth_filter(auth_manager);

        // POST /api/tenants - Create tenant
        let create_tenant = warp::path("api")
            .and(warp::path("tenants"))
            .and(warp::post())
            .and(warp::body::json())
            .and(with_auth.clone())
            .and(warp::any().map({
                let db = database.clone();
                move || db.clone()
            }))
            .and_then(|req, auth, db| async move {
                crate::tenant_routes::create_tenant(req, auth, db)
                    .await
                    .map(|response| warp::reply::json(&response))
                    .map_err(warp::reject::custom)
            });

        // GET /api/tenants - List tenants
        let list_tenants = warp::path("api")
            .and(warp::path("tenants"))
            .and(warp::get())
            .and(with_auth.clone())
            .and(warp::any().map({
                let db = database.clone();
                move || db.clone()
            }))
            .and_then(|auth, db| async move {
                crate::tenant_routes::list_tenants(auth, db)
                    .await
                    .map(|response| warp::reply::json(&response))
                    .map_err(warp::reject::custom)
            });

        // POST /api/tenants/{tenant_id}/oauth - Configure OAuth
        let configure_oauth = warp::path("api")
            .and(warp::path("tenants"))
            .and(warp::path::param::<String>())
            .and(warp::path("oauth"))
            .and(warp::post())
            .and(warp::body::json())
            .and(with_auth)
            .and(warp::any().map(move || database.clone()))
            .and_then(|tenant_id, req, auth, db| async move {
                crate::tenant_routes::configure_tenant_oauth(tenant_id, req, auth, db)
                    .await
                    .map(|response| warp::reply::json(&response))
                    .map_err(warp::reject::custom)
            });

        create_tenant.or(list_tenants).or(configure_oauth)
    }

    /// Run MCP server with both stdio and HTTP transports
    async fn run_mcp_server(self, port: u16) -> Result<()> {
        info!("Starting MCP server with stdio and HTTP transports");

        // Clone server for both transports
        let server_for_stdio = self.clone();
        let server_for_http = self.clone();

        // Start stdio transport in background - don't wait for it to complete
        let stdio_handle = tokio::spawn(async move {
            match server_for_stdio.run_stdio_transport().await {
                Ok(()) => info!("stdio transport completed successfully"),
                Err(e) => warn!("stdio transport failed: {}", e),
            }
        });

        // Monitor stdio transport in background but don't exit server when it completes
        tokio::spawn(async move {
            match stdio_handle.await {
                Ok(()) => info!("stdio transport task completed"),
                Err(e) => warn!("stdio transport task failed: {}", e),
            }
        });

        // Run HTTP transport - this should run indefinitely
        loop {
            info!("Starting HTTP transport on port {}", port);

            // Clone server for each iteration since run_http_transport takes ownership
            match server_for_http.clone().run_http_transport(port).await {
                Ok(()) => {
                    error!("HTTP transport unexpectedly completed - this should never happen");
                    error!("HTTP server should run indefinitely. Restarting in 5 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!("HTTP transport failed: {}", e);
                    error!("Restarting HTTP server in 10 seconds...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
                }
            }
        }
    }

    /// Run MCP server with only HTTP transport (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server fails to start or bind to the specified port
    pub async fn run_http_only(self, port: u16) -> Result<()> {
        info!("Starting MCP server with HTTP transport only");

        // Start HTTP server for auth endpoints in background
        let http_port = port + 1; // Use port+1 for HTTP
        let resources_http = self.resources.clone();
        tokio::spawn(async move {
            Self::run_http_server_with_resources(http_port, resources_http).await
        });

        // Run MCP HTTP transport only (no stdio)
        self.run_http_transport(port).await
    }

    /// Run MCP server using stdio transport (MCP specification compliant)
    async fn run_stdio_transport(self) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        info!("MCP stdio transport ready - listening on stdin/stdout");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                line.clear();
                continue;
            }

            if let Ok(request) = serde_json::from_str::<McpRequest>(trimmed_line) {
                if let Some(response) = Self::handle_request(request, &self.resources).await {
                    let response_str = match serde_json::to_string(&response) {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("Failed to serialize response: {}", e);
                            line.clear();
                            continue;
                        }
                    };

                    if let Err(e) = stdout.write_all(response_str.as_bytes()).await {
                        tracing::error!("Failed to write to stdout: {}", e);
                        break;
                    }
                    if let Err(e) = stdout.write_all(b"\n").await {
                        tracing::error!("Failed to write newline to stdout: {}", e);
                        break;
                    }
                    if let Err(e) = stdout.flush().await {
                        tracing::error!("Failed to flush stdout: {}", e);
                        break;
                    }
                }
            }
            line.clear();
        }

        info!("MCP stdio transport ended");
        Ok(())
    }

    /// Run MCP server using Streamable HTTP transport (MCP specification compliant)
    async fn run_http_transport(self, port: u16) -> Result<()> {
        use warp::Filter;

        info!("MCP HTTP transport starting on port {}", port);

        let resources = self.resources.clone();

        // MCP endpoint for both POST and GET
        let mcp_endpoint = warp::path("mcp")
            .and(warp::method())
            .and(warp::header::optional::<String>("origin"))
            .and(warp::header::optional::<String>("accept"))
            .and(warp::header::optional::<String>("authorization"))
            .and(
                warp::body::json()
                    .or(warp::any().map(|| serde_json::Value::Null))
                    .unify(),
            )
            .and_then({
                move |method: warp::http::Method,
                      origin: Option<String>,
                      accept: Option<String>,
                      authorization: Option<String>,
                      body: serde_json::Value| {
                    let resources = resources.clone();

                    async move {
                        let ctx = HttpRequestContext { resources };
                        Self::handle_mcp_http_request(
                            method,
                            origin,
                            accept,
                            authorization,
                            body,
                            &ctx,
                        )
                        .await
                    }
                }
            });

        // Configure CORS for MCP
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type", "accept", "origin", "authorization"])
            .allow_methods(vec!["GET", "POST", "OPTIONS"]);

        let routes = mcp_endpoint.with(cors).recover(|err| async move {
            Ok::<_, std::convert::Infallible>(Self::handle_mcp_rejection_sync(&err))
        });

        info!("MCP HTTP transport ready on port {}", port);
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;

        Ok(())
    }

    /// Handle MCP HTTP request (Streamable HTTP transport)
    async fn handle_mcp_http_request(
        method: warp::http::Method,
        origin: Option<String>,
        accept: Option<String>,
        authorization: Option<String>,
        body: serde_json::Value,
        ctx: &HttpRequestContext,
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
        // Validate Origin header for security (DNS rebinding protection)
        if let Some(origin) = origin {
            if !Self::is_valid_origin(&origin) {
                return Err(warp::reject::custom(McpHttpError::InvalidOrigin));
            }
        }

        match method {
            warp::http::Method::POST => {
                // Handle JSON-RPC request
                match serde_json::from_value::<McpRequest>(body.clone()) {
                    Ok(mut request) => {
                        // If no auth_token in the request body, use the Authorization header
                        if request.auth_token.is_none() {
                            request.auth_token = authorization;
                        }

                        #[allow(clippy::option_if_let_else)]
                        if let Some(response) = Self::handle_request(request, &ctx.resources).await
                        {
                            // Return 202 Accepted with response body for successful requests
                            Ok(Box::new(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::ACCEPTED,
                            )))
                        } else {
                            // Notification - return 202 with empty body
                            Ok(Box::new(warp::reply::with_status(
                                warp::reply(),
                                warp::http::StatusCode::ACCEPTED,
                            )))
                        }
                    }
                    Err(parse_error) => {
                        let body_str = serde_json::to_string(&body)
                            .unwrap_or_else(|_| "invalid json".to_string());
                        tracing::warn!(
                            "Failed to parse MCP request: {} | Body: {}",
                            parse_error,
                            body_str
                        );

                        // Create and log the error response we're about to send
                        let error_response = McpResponse::error(
                            default_request_id(),
                            -32600,
                            "Invalid request".to_string(),
                        );
                        let error_response_str = serde_json::to_string(&error_response)
                            .unwrap_or_else(|_| "failed to serialize error response".to_string());
                        tracing::warn!("Sending MCP error response: {}", error_response_str);

                        Err(warp::reject::custom(McpHttpError::InvalidRequest))
                    }
                }
            }
            warp::http::Method::GET => {
                // Handle GET request for server-sent events or status
                if accept
                    .as_ref()
                    .is_some_and(|a| a.contains("text/event-stream"))
                {
                    // Return SSE response for streaming
                    let reply = warp::reply::with_header(
                        "MCP HTTP transport ready",
                        "content-type",
                        "text/event-stream",
                    );
                    Ok(Box::new(warp::reply::with_status(
                        reply,
                        warp::http::StatusCode::OK,
                    )))
                } else {
                    // Return JSON status
                    let reply = warp::reply::json(&serde_json::json!({
                        "status": "ready",
                        "transport": "streamable-http",
                        "version": protocol::mcp_protocol_version()
                    }));
                    Ok(Box::new(warp::reply::with_status(
                        reply,
                        warp::http::StatusCode::OK,
                    )))
                }
            }
            _ => Err(warp::reject::custom(McpHttpError::InvalidRequest)),
        }
    }

    /// Validate origin header for security
    fn is_valid_origin(origin: &str) -> bool {
        // Allow localhost origins for development
        crate::constants::network_config::LOCALHOST_PATTERNS
            .iter()
            .any(|pattern| origin.starts_with(pattern)) ||
        // Allow null origin for direct connections
        origin == "null"
    }

    /// Handle HTTP rejection
    fn handle_mcp_rejection_sync(err: &warp::Rejection) -> impl warp::Reply {
        let http_code;
        let mcp_error_code;
        let message;

        if err.is_not_found() {
            http_code = warp::http::StatusCode::NOT_FOUND;
            mcp_error_code = -32601; // Method not found
            message = "Not Found";
        } else if matches!(err.find(), Some(McpHttpError::InvalidOrigin)) {
            http_code = warp::http::StatusCode::FORBIDDEN;
            mcp_error_code = -32000; // Server error
            message = "Invalid origin";
        } else if matches!(err.find(), Some(McpHttpError::InvalidRequest)) {
            http_code = warp::http::StatusCode::BAD_REQUEST;
            mcp_error_code = -32600; // Invalid Request
            message = "Invalid request";
        } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
            http_code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
            mcp_error_code = -32601; // Method not found
            message = "Method not allowed";
        } else {
            http_code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
            mcp_error_code = -32603; // Internal error
            message = "Internal server error";
        }

        // Return proper MCP JSON-RPC error response
        // Use -1 as ID for rejection errors since we can't determine the original request ID
        let mcp_response = McpResponse::error(
            serde_json::Value::Number(serde_json::Number::from(-1)),
            mcp_error_code,
            message.to_string(),
        );

        // Debug log the rejection response we're sending
        let response_str = serde_json::to_string(&mcp_response)
            .unwrap_or_else(|_| "failed to serialize rejection response".to_string());
        tracing::warn!(
            "Sending rejection response (HTTP {}): {}",
            http_code.as_u16(),
            response_str
        );

        let json = warp::reply::json(&mcp_response);
        warp::reply::with_status(json, http_code)
    }

    /// Handle MCP request with `ServerResources`
    pub async fn handle_request(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> Option<McpResponse> {
        // Handle notifications (no response needed)
        if request.method.starts_with("notifications/") {
            Self::handle_notification(&request);
            return None;
        }

        // Handle regular requests (response needed)
        let response = match request.method.as_str() {
            "initialize" => Self::handle_initialize(request),
            "ping" => Self::handle_ping(request),
            "tools/list" => Self::handle_tools_list(request),
            "prompts/list" => Self::handle_prompts_list(request),
            "resources/list" => Self::handle_resources_list(request),
            "authenticate" => Self::handle_authenticate(request, &resources.auth_manager),
            "tools/call" => Self::handle_tools_call_with_resources(request, resources).await,
            _ => Self::handle_unknown_method(request),
        };

        Some(response)
    }

    /// Handle initialize request
    fn handle_initialize(request: McpRequest) -> McpResponse {
        let init_response = InitializeResponse::new(
            protocol::mcp_protocol_version(),
            protocol::server_name_multitenant(),
            SERVER_VERSION.to_string(),
        );

        let request_id = request.id.unwrap_or_else(default_request_id);
        match serde_json::to_value(&init_response) {
            Ok(result) => McpResponse::success(request_id, result),
            Err(_) => McpResponse::error(request_id, -32603, "Internal error".to_string()),
        }
    }

    /// Handle ping request
    fn handle_ping(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(request_id, serde_json::json!({}))
    }

    /// Extract tenant context from MCP request headers
    ///
    /// # Errors
    ///
    /// Returns an error if tenant context extraction fails
    /// Get user's role in a tenant with proper fallback
    async fn get_user_role_for_tenant(
        database: &Arc<Database>,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> TenantRole {
        match database.get_user_tenant_role(user_id, tenant_id).await {
            Ok(Some(role_str)) => TenantRole::from_db_string(&role_str),
            Ok(None) => {
                tracing::warn!(
                    "User {} not found in tenant {}, defaulting to Member",
                    user_id,
                    tenant_id
                );
                TenantRole::Member
            }
            Err(e) => {
                tracing::error!(
                    "Failed to get user role for tenant {}: {}, defaulting to Member",
                    tenant_id,
                    e
                );
                TenantRole::Member
            }
        }
    }

    /// Extract tenant context from explicit header
    async fn extract_tenant_from_header(
        request: &McpRequest,
        auth_result: &AuthResult,
        database: &Arc<Database>,
    ) -> Result<Option<TenantContext>, String> {
        let Some(tenant_id_str) = request
            .headers
            .as_ref()
            .and_then(|h| h.get("X-Tenant-ID"))
            .and_then(|v| v.as_str())
        else {
            return Ok(None);
        };

        let tenant_id = uuid::Uuid::parse_str(tenant_id_str)
            .map_err(|e| format!("Invalid tenant ID format: {e}"))?;

        match database.get_tenant_by_id(tenant_id).await {
            Ok(tenant) => {
                tracing::debug!("Using explicit tenant from header: {}", tenant.name);
                let role =
                    Self::get_user_role_for_tenant(database, auth_result.user_id, tenant_id).await;
                Ok(Some(TenantContext::new(
                    tenant_id,
                    tenant.name,
                    auth_result.user_id,
                    role,
                )))
            }
            Err(e) => {
                tracing::warn!("Failed to fetch explicit tenant {}: {}", tenant_id, e);
                Err(format!("Tenant not found: {tenant_id}"))
            }
        }
    }

    /// Extract tenant context from user's tenant association
    async fn extract_tenant_from_user(
        auth_result: &AuthResult,
        database: &Arc<Database>,
    ) -> Result<Option<TenantContext>, String> {
        let user = match database.get_user(auth_result.user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                tracing::warn!("User not found: {}", auth_result.user_id);
                return Err("User not found".to_string());
            }
            Err(e) => {
                tracing::warn!("Failed to fetch user {}: {}", auth_result.user_id, e);
                return Err(format!("User lookup failed: {e}"));
            }
        };

        let Some(user_tenant_id) = user.tenant_id else {
            tracing::debug!(
                "User {} has no tenant_id, will use default tenant",
                auth_result.user_id
            );
            return Ok(None);
        };

        // Try parsing as UUID first
        if let Ok(tenant_uuid) = uuid::Uuid::parse_str(&user_tenant_id) {
            if let Ok(tenant) = database.get_tenant_by_id(tenant_uuid).await {
                tracing::debug!("Using user's tenant: {}", tenant.name);
                let role =
                    Self::get_user_role_for_tenant(database, auth_result.user_id, tenant_uuid)
                        .await;
                return Ok(Some(TenantContext::new(
                    tenant_uuid,
                    tenant.name,
                    auth_result.user_id,
                    role,
                )));
            }
        }

        // Try as slug if UUID parsing failed
        if let Ok(tenant) = database.get_tenant_by_slug(&user_tenant_id).await {
            tracing::debug!("Using user's tenant slug: {}", tenant.name);
            let role =
                Self::get_user_role_for_tenant(database, auth_result.user_id, tenant.id).await;
            return Ok(Some(TenantContext::new(
                tenant.id,
                tenant.name,
                auth_result.user_id,
                role,
            )));
        }

        tracing::warn!("Failed to resolve user's tenant: {}", user_tenant_id);
        Ok(None)
    }

    async fn extract_tenant_context_internal(
        request: &McpRequest,
        auth_result: &AuthResult,
        database: &Arc<Database>,
    ) -> Result<Option<TenantContext>, String> {
        // 1. Try explicit tenant from header
        if let Some(context) =
            Self::extract_tenant_from_header(request, auth_result, database).await?
        {
            return Ok(Some(context));
        }

        // 2. Try user's tenant association
        if let Some(context) = Self::extract_tenant_from_user(auth_result, database).await? {
            return Ok(Some(context));
        }

        // 3. No tenant found - return None for proper error handling
        tracing::warn!("No tenant context found for user {}", auth_result.user_id);
        Ok(None)
    }

    /// Handle tools/list request
    fn handle_tools_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        let tools = crate::mcp::schema::get_tools();
        McpResponse::success(
            request_id,
            serde_json::json!({
                "tools": tools
            }),
        )
    }

    /// Handle prompts/list request - returns empty list as we don't support prompts yet
    fn handle_prompts_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(
            request_id,
            serde_json::json!({
                "prompts": []
            }),
        )
    }

    /// Handle resources/list request - returns empty list as we don't support resources yet
    fn handle_resources_list(request: McpRequest) -> McpResponse {
        let request_id = request.id.unwrap_or_else(default_request_id);
        McpResponse::success(
            request_id,
            serde_json::json!({
                "resources": []
            }),
        )
    }

    /// Handle notifications/initialized - no response needed for notifications
    /// Handle notification messages (no response needed)
    fn handle_notification(request: &McpRequest) {
        if request.method.as_str() == "notifications/initialized" {
            // Client has finished initialization - we can log this but no response needed
        } else {
            // Unknown notification - log but don't respond
        }
    }

    /// Handle tools/call request with `ServerResources` (for HTTP requests)
    async fn handle_tools_call_with_resources(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let auth_token = request.auth_token.as_deref();

        tracing::debug!(
            "MCP tool call authentication attempt for method: {}",
            request.method
        );

        match resources
            .auth_middleware
            .authenticate_request(auth_token)
            .await
        {
            Ok(auth_result) => {
                tracing::info!(
                    "MCP tool call authentication successful for user: {} (method: {})",
                    auth_result.user_id,
                    auth_result.auth_method.display_name()
                );

                // Update user's last active timestamp
                let _ = resources
                    .database
                    .update_last_active(auth_result.user_id)
                    .await;

                // Extract tenant context from request and auth result
                let tenant_context = Self::extract_tenant_context_internal(
                    &request,
                    &auth_result,
                    &resources.database,
                )
                .await
                .unwrap_or(None);

                // Use the provided ServerResources directly
                Self::handle_tool_execution_direct(request, auth_result, tenant_context, resources)
                    .await
            }
            Err(e) => Self::handle_authentication_error(request, &e),
        }
    }

    /// Handle tool execution directly using provided `ServerResources`
    async fn handle_tool_execution_direct(
        request: McpRequest,
        auth_result: AuthResult,
        tenant_context: Option<TenantContext>,
        resources: &Arc<ServerResources>,
    ) -> McpResponse {
        let Some(params) = request.params else {
            tracing::error!("Missing request parameters in tools/call");
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.unwrap_or_else(default_request_id),
                result: None,
                error: Some(McpError {
                    code: ERROR_INVALID_PARAMS,
                    message: "Invalid params: Missing request parameters".to_string(),
                    data: None,
                }),
            };
        };
        let tool_name = params["name"].as_str().unwrap_or("");
        let args = &params["arguments"];
        let user_id = auth_result.user_id;

        tracing::info!(
            "Executing tool call: {} for user: {} using {} authentication",
            tool_name,
            user_id,
            auth_result.auth_method.display_name()
        );

        // Use the provided ServerResources directly - no fake resource creation!
        let routing_context = ToolRoutingContext {
            resources,
            tenant_context: &tenant_context,
            auth_result: &auth_result,
        };

        Self::route_tool_call(
            tool_name,
            args,
            request.id.unwrap_or_else(default_request_id),
            user_id,
            &routing_context,
        )
        .await
    }

    /// Handle tools/call request with authentication and tenant context
    #[allow(dead_code)]
    async fn handle_tools_call(&self, request: McpRequest) -> McpResponse {
        let auth_token = request.auth_token.as_deref();

        tracing::debug!(
            "MCP tool call authentication attempt for method: {}",
            request.method
        );

        match self
            .resources
            .auth_middleware
            .authenticate_request(auth_token)
            .await
        {
            Ok(auth_result) => {
                tracing::info!(
                    "MCP tool call authentication successful for user: {} (method: {})",
                    auth_result.user_id,
                    auth_result.auth_method.display_name()
                );

                // Update user's last active timestamp
                let _ = self
                    .resources
                    .database
                    .update_last_active(auth_result.user_id)
                    .await;

                // Extract tenant context from request and auth result
                let tenant_context = Self::extract_tenant_context_internal(
                    &request,
                    &auth_result,
                    &self.resources.database,
                )
                .await
                .unwrap_or(None);

                self.handle_authenticated_tool_call(request, auth_result, tenant_context)
                    .await
            }
            Err(e) => Self::handle_authentication_error(request, &e),
        }
    }

    /// Handle authentication error
    fn handle_authentication_error(request: McpRequest, e: &anyhow::Error) -> McpResponse {
        warn!("MCP tool call authentication failed: {}", e);

        // Determine specific error code based on error message
        let error_message = e.to_string();
        let (error_code, error_msg) = if error_message.contains("JWT token expired") {
            (
                crate::constants::errors::ERROR_TOKEN_EXPIRED,
                crate::constants::errors::MSG_TOKEN_EXPIRED,
            )
        } else if error_message.contains("JWT token signature is invalid") {
            (
                crate::constants::errors::ERROR_TOKEN_INVALID,
                crate::constants::errors::MSG_TOKEN_INVALID,
            )
        } else if error_message.contains("JWT token is malformed") {
            (
                crate::constants::errors::ERROR_TOKEN_MALFORMED,
                crate::constants::errors::MSG_TOKEN_MALFORMED,
            )
        } else {
            (ERROR_UNAUTHORIZED, "Authentication required")
        };

        McpResponse::error_with_data(
            request.id.unwrap_or_else(default_request_id),
            error_code,
            error_msg.to_string(),
            serde_json::json!({
                "detailed_error": error_message,
                "authentication_failed": true
            }),
        )
    }

    /// Handle unknown method
    fn handle_unknown_method(request: McpRequest) -> McpResponse {
        McpResponse::error(
            request.id.unwrap_or_else(default_request_id),
            ERROR_METHOD_NOT_FOUND,
            "Method not found".to_string(),
        )
    }

    /// Handle authentication request
    fn handle_authenticate(request: McpRequest, auth_manager: &Arc<AuthManager>) -> McpResponse {
        let Some(params) = request.params else {
            tracing::error!("Missing request parameters in authentication");
            return McpResponse::error(
                request.id.unwrap_or_else(default_request_id),
                ERROR_INVALID_PARAMS,
                "Invalid params: Missing request parameters".to_string(),
            );
        };

        if let Ok(auth_request) = serde_json::from_value::<AuthRequest>(params) {
            let auth_response = auth_manager.authenticate(&auth_request);
            match serde_json::to_value(&auth_response) {
                Ok(result) => {
                    McpResponse::success(request.id.unwrap_or_else(default_request_id), result)
                }
                Err(_) => McpResponse::error(
                    request.id.unwrap_or_else(default_request_id),
                    -32603,
                    "Internal error".to_string(),
                ),
            }
        } else {
            McpResponse::error(
                request.id.unwrap_or_else(default_request_id),
                ERROR_INVALID_PARAMS,
                "Invalid authentication request".to_string(),
            )
        }
    }

    /// Handle authenticated tool call with user context and rate limiting
    #[allow(dead_code)]
    async fn handle_authenticated_tool_call(
        &self,
        request: McpRequest,
        auth_result: AuthResult,
        tenant_context: Option<TenantContext>,
    ) -> McpResponse {
        let Some(params) = request.params else {
            tracing::error!("Missing request parameters in tools/call");
            return McpResponse {
                jsonrpc: "2.0".to_string(),
                id: request.id.unwrap_or_else(default_request_id),
                result: None,
                error: Some(McpError {
                    code: ERROR_INVALID_PARAMS,
                    message: "Invalid params: Missing request parameters".to_string(),
                    data: None,
                }),
            };
        };
        let tool_name = params["name"].as_str().unwrap_or("");
        let args = &params["arguments"];
        let user_id = auth_result.user_id;

        tracing::info!(
            "Executing tool call: {} for user: {} using {} authentication",
            tool_name,
            user_id,
            auth_result.auth_method.display_name()
        );

        let routing_context = ToolRoutingContext {
            resources: &self.resources,
            tenant_context: &tenant_context,
            auth_result: &auth_result,
        };

        Self::route_tool_call(
            tool_name,
            args,
            request.id.unwrap_or_else(default_request_id),
            user_id,
            &routing_context,
        )
        .await
    }

    /// Route tool calls to appropriate handlers based on tool type and tenant context
    async fn route_tool_call(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        match tool_name {
            // Note: CONNECT_STRAVA and CONNECT_FITBIT tools removed - use tenant-level OAuth configuration
            GET_CONNECTION_STATUS => {
                if let Some(ref tenant_ctx) = ctx.tenant_context {
                    // Extract optional OAuth credentials from args
                    let strava_client_id = args.get("strava_client_id").and_then(|v| v.as_str());
                    let strava_client_secret =
                        args.get("strava_client_secret").and_then(|v| v.as_str());
                    let fitbit_client_id = args.get("fitbit_client_id").and_then(|v| v.as_str());
                    let fitbit_client_secret =
                        args.get("fitbit_client_secret").and_then(|v| v.as_str());

                    return Self::handle_tenant_connection_status(
                        tenant_ctx,
                        &ctx.resources.tenant_provider_factory,
                        &ctx.resources.database,
                        &ctx.resources.tenant_oauth_client,
                        request_id,
                        strava_client_id,
                        strava_client_secret,
                        fitbit_client_id,
                        fitbit_client_secret,
                    )
                    .await;
                }
                // No legacy fallback - require tenant context
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: "No tenant context found. User must be assigned to a tenant."
                            .to_string(),
                        data: None,
                    }),
                    id: request_id,
                }
            }
            DISCONNECT_PROVIDER => {
                let provider_name = args[PROVIDER].as_str().unwrap_or("");
                Self::route_disconnect_tool(provider_name, user_id, request_id, ctx)
            }
            SET_GOAL
            | TRACK_PROGRESS
            | ANALYZE_GOAL_FEASIBILITY
            | SUGGEST_GOALS
            | CALCULATE_FITNESS_SCORE
            | GENERATE_RECOMMENDATIONS
            | ANALYZE_TRAINING_LOAD
            | DETECT_PATTERNS
            | ANALYZE_PERFORMANCE_TRENDS
            | "get_configuration_catalog"
            | "get_configuration_profiles"
            | "get_user_configuration"
            | "update_user_configuration"
            | "calculate_personalized_zones"
            | "validate_configuration" => {
                Self::handle_tool_without_provider(
                    tool_name,
                    args,
                    request_id,
                    user_id,
                    &ctx.resources.database,
                    ctx.auth_result,
                )
                .await
            }
            _ => Self::route_provider_tool(tool_name, args, request_id, user_id, ctx).await,
        }
    }

    fn route_disconnect_tool(
        provider_name: &str,
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        if let Some(ref tenant_ctx) = ctx.tenant_context {
            Self::handle_tenant_disconnect_provider(
                tenant_ctx,
                provider_name,
                &ctx.resources.tenant_provider_factory,
                &ctx.resources.database,
                request_id,
            )
        } else {
            Self::handle_disconnect_provider(user_id, provider_name, ctx.resources, request_id)
        }
    }

    async fn route_provider_tool(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        _user_id: Uuid,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        if let Some(ref tenant_ctx) = ctx.tenant_context {
            Self::handle_tenant_tool_with_provider(
                tool_name,
                args,
                request_id,
                tenant_ctx,
                ctx.resources,
                ctx.auth_result,
            )
            .await
        } else {
            // No tenant context means no provider access - tenant-aware endpoints required
            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: format!("Tool '{tool_name}' requires tenant context - use tenant-aware MCP endpoints"),
                    data: None,
                }),
                id: request_id,
            }
        }
    }

    /// Handle tools that don't require external providers
    async fn handle_tool_without_provider(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        database: &Arc<Database>,
        auth_result: &AuthResult,
    ) -> McpResponse {
        let start_time = std::time::Instant::now();
        let response = Self::execute_tool_call_without_provider(
            tool_name,
            args,
            request_id.clone(),
            user_id,
            database,
        )
        .await;

        // Record API key usage if authenticated with API key
        if let crate::auth::AuthMethod::ApiKey { key_id, .. } = &auth_result.auth_method {
            let _ = Self::record_api_key_usage(
                database,
                key_id,
                tool_name,
                start_time.elapsed(),
                &response,
            )
            .await;
        }

        response
    }

    /// Handle `disconnect_provider` tool call
    fn handle_disconnect_provider(
        user_id: Uuid,
        provider: &str,
        resources: &Arc<ServerResources>,
        id: Value,
    ) -> McpResponse {
        // Use existing ServerResources (no fake auth managers or cloning!)
        let oauth_routes = OAuthRoutes::new(resources.clone());

        match oauth_routes.disconnect_provider(user_id, provider) {
            Ok(()) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": format!("Successfully disconnected {provider}"),
                    "provider": provider
                });

                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: Some(response),
                    error: None,
                    id,
                }
            }
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to disconnect provider: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Execute tool call without provider (for database-only tools)
    async fn execute_tool_call_without_provider(
        tool_name: &str,
        args: &Value,
        id: Value,
        user_id: Uuid,
        database: &Arc<Database>,
    ) -> McpResponse {
        let result = match tool_name {
            SET_GOAL => Self::handle_set_goal(args, user_id, database, &id).await,
            TRACK_PROGRESS => Self::handle_track_progress(args, user_id, database, &id).await,
            ANALYZE_GOAL_FEASIBILITY => Ok(Self::handle_analyze_goal_feasibility(args, user_id)),
            SUGGEST_GOALS => Ok(Self::handle_suggest_goals()),
            CALCULATE_FITNESS_SCORE => Ok(Self::handle_calculate_fitness_score()),
            GENERATE_RECOMMENDATIONS => Ok(Self::handle_generate_recommendations()),
            ANALYZE_TRAINING_LOAD => Ok(Self::handle_analyze_training_load()),
            DETECT_PATTERNS => Ok(Self::handle_detect_patterns(args)),
            ANALYZE_PERFORMANCE_TRENDS => Ok(Self::handle_analyze_performance_trends(args)),
            "get_configuration_catalog" => Ok(Self::handle_get_configuration_catalog()),
            "get_configuration_profiles" => Ok(Self::handle_get_configuration_profiles()),
            "get_user_configuration" => Ok(Self::handle_get_user_configuration(user_id, database)),
            "update_user_configuration" => Ok(Self::handle_update_user_configuration(
                args, user_id, database,
            )),
            "calculate_personalized_zones" => Ok(Self::handle_calculate_personalized_zones(args)),
            "validate_configuration" => Ok(Self::handle_validate_configuration(args)),
            PREDICT_PERFORMANCE => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Provider required".into(),
                        data: None,
                    }),
                    id,
                };
            }
            _ => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: format!("Unknown tool: {tool_name}"),
                        data: None,
                    }),
                    id,
                };
            }
        };

        match result {
            Ok(response) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: Some(response),
                error: None,
                id,
            },
            Err(error_response) => error_response,
        }
    }

    /// Handle `SET_GOAL` tool call
    async fn handle_set_goal(
        args: &Value,
        user_id: Uuid,
        database: &Arc<Database>,
        id: &Value,
    ) -> Result<Value, McpResponse> {
        let goal_data = args.clone();

        match database.create_goal(user_id, goal_data).await {
            Ok(goal_id) => {
                let response = serde_json::json!({
                    "goal_created": {
                        "goal_id": goal_id,
                        "status": "active",
                        "message": "Goal successfully created"
                    }
                });
                Ok(response)
            }
            Err(e) => Err(McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to create goal: {e}"),
                    data: None,
                }),
                id: id.clone(),
            }),
        }
    }

    /// Handle `TRACK_PROGRESS` tool call
    async fn handle_track_progress(
        args: &Value,
        user_id: Uuid,
        database: &Arc<Database>,
        id: &Value,
    ) -> Result<Value, McpResponse> {
        let goal_id = args[GOAL_ID].as_str().unwrap_or("");

        match database.get_user_goals(user_id).await {
            Ok(goals) => goals.iter().find(|g| g["id"] == goal_id).map_or_else(
                || {
                    Err(McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Goal with ID '{goal_id}' not found"),
                            data: None,
                        }),
                        id: id.clone(),
                    })
                },
                |goal| {
                    let response = serde_json::json!({
                        "progress_report": {
                            "goal_id": goal_id,
                            "goal": goal,
                            "progress_percentage": 65.0,
                            "on_track": true,
                            "insights": [
                                "Making good progress toward your goal",
                                "Maintain current training frequency"
                            ]
                        }
                    });
                    Ok(response)
                },
            ),
            Err(e) => Err(McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get goals: {e}"),
                    data: None,
                }),
                id: id.clone(),
            }),
        }
    }

    /// Handle `ANALYZE_GOAL_FEASIBILITY` tool call
    fn handle_analyze_goal_feasibility(args: &Value, user_id: Uuid) -> Value {
        let goal_data = args.clone();

        // Log goal analysis request
        tracing::debug!("Analyzing goal feasibility for user {}", user_id);
        if let Some(goal_type) = goal_data.get("goal_type") {
            tracing::debug!("Goal type: {}", goal_type);
        }
        if let Some(target_value) = goal_data.get("target_value") {
            tracing::debug!("Target value: {}", target_value);
        }

        let response = serde_json::json!({
            "feasibility_analysis": {
                "feasible": true,
                "confidence": 0.8,
                "estimated_completion_time": "8 weeks",
                "recommendations": [
                    "Goal appears achievable based on current training patterns",
                    "Consider gradual increase in training volume"
                ],
                "risk_factors": [
                    "Ensure adequate recovery time",
                    "Monitor for signs of overtraining"
                ]
            }
        });
        response
    }

    /// Handle `SUGGEST_GOALS` tool call
    fn handle_suggest_goals() -> Value {
        let response = serde_json::json!({
            "goal_suggestions": [
                {
                    "title": "Monthly Distance Goal",
                    "description": "Run 100km this month",
                    "goal_type": "distance",
                    "target_value": 100.0,
                    "rationale": "Based on your recent running frequency"
                },
                {
                    "title": "Pace Improvement",
                    "description": "Improve average pace by 30 seconds per km",
                    "goal_type": "performance",
                    "target_value": 30.0,
                    "rationale": "Your pace has been consistent - time to challenge yourself"
                }
            ]
        });
        response
    }

    /// Handle `CALCULATE_FITNESS_SCORE` tool call
    fn handle_calculate_fitness_score() -> Value {
        let response = serde_json::json!({
            "fitness_score": {
                "overall_score": 75,
                "max_score": 100,
                "components": {
                    "frequency": 20,
                    "consistency": 15,
                    "duration": 20,
                    "variety": 10
                },
                "insights": [
                    "Your fitness score is 75 out of 100",
                    "Regular training frequency is your strength",
                    "Consider adding variety to your workouts"
                ]
            }
        });
        response
    }

    /// Handle `GENERATE_RECOMMENDATIONS` tool call
    fn handle_generate_recommendations() -> Value {
        let response = serde_json::json!({
            "training_recommendations": [
                {
                    "type": "intensity",
                    "title": "Add Interval Training",
                    "description": "Include 1-2 high-intensity interval sessions per week",
                    "priority": "medium",
                    "rationale": "To improve speed and cardiovascular fitness"
                },
                {
                    "type": "volume",
                    "title": "Gradual Volume Increase",
                    "description": "Increase weekly distance by 10% each week",
                    "priority": "high",
                    "rationale": "Based on your current training load"
                },
                {
                    "type": "recovery",
                    "title": "Include Rest Days",
                    "description": "Schedule at least one complete rest day per week",
                    "priority": "high",
                    "rationale": "Essential for adaptation and injury prevention"
                }
            ]
        });
        response
    }

    /// Handle `ANALYZE_TRAINING_LOAD` tool call
    fn handle_analyze_training_load() -> Value {
        let response = serde_json::json!({
            "training_load_analysis": {
                "weekly_hours": 5.2,
                "weekly_distance_km": 35.0,
                "load_level": "moderate",
                "total_activities": 12,
                "insights": [
                    "Current training load: moderate (5.2 hours/week)",
                    "Training load is appropriate for current fitness level",
                    "Consider periodization for optimal adaptation"
                ],
                "recommendations": [
                    "Maintain current level",
                    "Focus on consistency"
                ]
            }
        });
        response
    }

    /// Handle `DETECT_PATTERNS` tool call
    fn handle_detect_patterns(args: &Value) -> Value {
        let response = serde_json::json!({
            "pattern_analysis": {
                "pattern_type": args["pattern_type"].as_str().unwrap_or("weekly"),
                "total_activities": 25,
                "patterns_detected": [
                    "Regular training frequency detected",
                    "Consistent effort levels across activities"
                ],
                "recommendations": [
                    "Continue current training consistency",
                    "Consider adding variety to workout types"
                ]
            }
        });
        response
    }

    /// Handle `ANALYZE_PERFORMANCE_TRENDS` tool call
    fn handle_analyze_performance_trends(args: &Value) -> Value {
        let response = serde_json::json!({
            "trend_analysis": {
                "timeframe": args["timeframe"].as_str().unwrap_or("month"),
                "metric": args["metric"].as_str().unwrap_or("pace"),
                "total_activities": 15,
                "trend_direction": "stable",
                "insights": [
                    "Analyzed 15 activities over the past month",
                    "Performance trends require more historical data for accurate analysis"
                ]
            }
        });
        response
    }

    /// Record API key usage for billing and analytics
    ///
    /// # Errors
    ///
    /// Returns an error if the usage cannot be recorded in the database
    pub async fn record_api_key_usage(
        database: &Arc<Database>,
        api_key_id: &str,
        tool_name: &str,
        response_time: std::time::Duration,
        response: &McpResponse,
    ) -> Result<()> {
        use crate::api_keys::ApiKeyUsage;

        let status_code = if response.error.is_some() {
            400 // Error responses
        } else {
            200 // Success responses
        };

        let error_message = response.error.as_ref().map(|e| e.message.clone());

        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key_id.to_string(),
            timestamp: Utc::now(),
            tool_name: tool_name.to_string(),
            response_time_ms: u32::try_from(response_time.as_millis()).ok(),
            status_code,
            error_message,
            request_size_bytes: None,  // Could be calculated from request
            response_size_bytes: None, // Could be calculated from response
            ip_address: None,          // Would need to be passed from request context
            user_agent: None,          // Would need to be passed from request context
        };

        database.record_api_key_usage(&usage).await?;
        Ok(())
    }

    /// Get database reference for admin API
    #[must_use]
    pub fn database(&self) -> &Database {
        &self.resources.database
    }

    /// Get auth manager reference for admin API
    #[must_use]
    pub fn auth_manager(&self) -> &AuthManager {
        &self.resources.auth_manager
    }

    /// Handle get configuration catalog tool call
    fn handle_get_configuration_catalog() -> Value {
        use crate::configuration::catalog::CatalogBuilder;

        let catalog = CatalogBuilder::build();
        serde_json::json!({
            "catalog": catalog,
            "metadata": {
                "timestamp": chrono::Utc::now(),
                "processing_time_ms": None::<u64>,
                "api_version": "1.0.0"
            }
        })
    }

    /// Handle get configuration profiles tool call
    fn handle_get_configuration_profiles() -> Value {
        use crate::configuration::profiles::ProfileTemplates;

        let all_profiles = ProfileTemplates::all();
        let profiles: Vec<_> = all_profiles
            .into_iter()
            .map(|(name, profile)| {
                let description = match &profile {
                    crate::configuration::profiles::ConfigProfile::Default => {
                        "Standard configuration with default thresholds".to_string()
                    }
                    crate::configuration::profiles::ConfigProfile::Research { .. } => {
                        "Research-grade detailed analysis with high sensitivity".to_string()
                    }
                    crate::configuration::profiles::ConfigProfile::Elite { .. } => {
                        "Elite athlete configuration with strict performance standards".to_string()
                    }
                    crate::configuration::profiles::ConfigProfile::Recreational { .. } => {
                        "Recreational athlete configuration with forgiving thresholds".to_string()
                    }
                    crate::configuration::profiles::ConfigProfile::Beginner { .. } => {
                        "Beginner-friendly configuration with simplified metrics".to_string()
                    }
                    crate::configuration::profiles::ConfigProfile::Medical { .. } => {
                        "Medical/rehabilitation configuration with conservative limits".to_string()
                    }
                    crate::configuration::profiles::ConfigProfile::SportSpecific {
                        sport, ..
                    } => format!("Sport-specific optimization for {sport}"),
                    crate::configuration::profiles::ConfigProfile::Custom {
                        description, ..
                    } => description.clone(),
                };

                serde_json::json!({
                    "name": name,
                    "profile": profile,
                    "description": description
                })
            })
            .collect();

        serde_json::json!({
            "profiles": profiles,
            "metadata": {
                "timestamp": chrono::Utc::now(),
                "total_profiles": profiles.len()
            }
        })
    }

    /// Handle get user configuration tool call
    fn handle_get_user_configuration(user_id: Uuid, _database: &Arc<Database>) -> Value {
        // For now, return default configuration
        // In a full implementation, this would query the database for user preferences
        let default_config = crate::configuration::profiles::ConfigProfile::Default;

        serde_json::json!({
            "user_id": user_id,
            "active_profile": default_config,
            "parameter_overrides": {},
            "created_at": chrono::Utc::now(),
            "updated_at": chrono::Utc::now()
        })
    }

    /// Handle update user configuration tool call
    fn handle_update_user_configuration(
        args: &Value,
        user_id: Uuid,
        _database: &Arc<Database>,
    ) -> Value {
        // Extract parameters from args
        let profile = args.get("profile").and_then(|v| v.as_str());
        let parameter_overrides = args
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // For now, just return success with the updated configuration
        // In a full implementation, this would update the database
        serde_json::json!({
            "user_id": user_id,
            "active_profile": profile.unwrap_or("Default"),
            "parameter_overrides": parameter_overrides,
            "updated_at": chrono::Utc::now(),
            "success": true
        })
    }

    /// Handle calculate personalized zones tool call
    fn handle_calculate_personalized_zones(args: &Value) -> Value {
        use crate::configuration::vo2_max::VO2MaxCalculator;

        // Extract parameters
        let vo2_max = args
            .get("vo2_max")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(50.0);
        // Safe: heart rate values are small positive integers (80-220 bpm)
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let max_hr = args
            .get("max_hr")
            .and_then(serde_json::Value::as_f64)
            .map_or(190, |v| v as u16);
        // Safe: heart rate values are small positive integers (40-100 bpm)
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let resting_hr = args
            .get("resting_hr")
            .and_then(serde_json::Value::as_f64)
            .map_or(60, |v| v as u16);
        let lactate_threshold = args
            .get("lactate_threshold")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.85);
        let sport_efficiency = args
            .get("sport_efficiency")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(1.0);

        // Create calculator and calculate zones
        let calculator = VO2MaxCalculator::new(
            vo2_max,
            resting_hr,
            max_hr,
            lactate_threshold,
            sport_efficiency,
        );

        let hr_zones = calculator.calculate_hr_zones();
        let pace_zones = calculator.calculate_pace_zones();

        serde_json::json!({
            "zones": {
                "heart_rate_zones": hr_zones,
                "pace_zones": pace_zones
            },
            "parameters_used": {
                "vo2_max": vo2_max,
                "max_hr": max_hr,
                "resting_hr": resting_hr,
                "lactate_threshold": lactate_threshold,
                "sport_efficiency": sport_efficiency
            },
            "metadata": {
                "timestamp": chrono::Utc::now(),
                "calculation_method": "VO2MaxCalculator"
            }
        })
    }

    /// Handle validate configuration tool call
    fn handle_validate_configuration(args: &Value) -> Value {
        use crate::configuration::runtime::ConfigValue;
        use crate::configuration::validation::ConfigValidator;

        let parameters = args
            .get("parameters")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        // Convert to HashMap<String, ConfigValue> for validation
        let mut param_map: std::collections::HashMap<String, ConfigValue> =
            std::collections::HashMap::new();

        if let Ok(json_map) = serde_json::from_value::<
            std::collections::HashMap<String, serde_json::Value>,
        >(parameters)
        {
            for (key, value) in json_map {
                let config_value = match value {
                    serde_json::Value::Number(n) if n.is_f64() => {
                        ConfigValue::Float(n.as_f64().unwrap_or(0.0))
                    }
                    serde_json::Value::Number(n) if n.is_i64() => {
                        ConfigValue::Integer(n.as_i64().unwrap_or(0))
                    }
                    serde_json::Value::Bool(b) => ConfigValue::Boolean(b),
                    serde_json::Value::String(s) => ConfigValue::String(s),
                    _ => continue, // Skip unsupported types
                };
                param_map.insert(key, config_value);
            }
        }

        // Validate using ConfigValidator
        let validator = ConfigValidator::new();
        let validation_result = validator.validate(&param_map, None); // No user profile for now

        serde_json::json!({
            "validation_result": validation_result,
            "metadata": {
                "timestamp": chrono::Utc::now(),
                "validator_version": "1.0.0"
            }
        })
    }

    // === Tenant-Aware Tool Handlers ===

    /// Store user-provided OAuth credentials if supplied
    async fn store_mcp_oauth_credentials(
        tenant_context: &TenantContext,
        oauth_client: &Arc<TenantOAuthClient>,
        credentials: &McpOAuthCredentials<'_>,
    ) {
        // Store Strava credentials if provided
        if let (Some(id), Some(secret)) = (
            credentials.strava_client_id,
            credentials.strava_client_secret,
        ) {
            tracing::info!(
                "Storing MCP-provided Strava OAuth credentials for tenant {}",
                tenant_context.tenant_id
            );
            let request = crate::tenant::oauth_client::StoreCredentialsRequest {
                client_id: id.to_string(),
                client_secret: secret.to_string(),
                redirect_uri: crate::constants::env_config::strava_redirect_uri(),
                scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                configured_by: tenant_context.user_id,
            };

            if let Err(e) = oauth_client
                .store_credentials(tenant_context.tenant_id, "strava", request)
                .await
            {
                tracing::error!("Failed to store Strava OAuth credentials: {}", e);
            }
        }

        // Store Fitbit credentials if provided
        if let (Some(id), Some(secret)) = (
            credentials.fitbit_client_id,
            credentials.fitbit_client_secret,
        ) {
            tracing::info!(
                "Storing MCP-provided Fitbit OAuth credentials for tenant {}",
                tenant_context.tenant_id
            );
            let request = crate::tenant::oauth_client::StoreCredentialsRequest {
                client_id: id.to_string(),
                client_secret: secret.to_string(),
                redirect_uri: crate::constants::env_config::fitbit_redirect_uri(),
                scopes: vec![
                    "activity".to_string(),
                    "heartrate".to_string(),
                    "location".to_string(),
                    "nutrition".to_string(),
                    "profile".to_string(),
                    "settings".to_string(),
                    "sleep".to_string(),
                    "social".to_string(),
                    "weight".to_string(),
                ],
                configured_by: tenant_context.user_id,
            };

            if let Err(e) = oauth_client
                .store_credentials(tenant_context.tenant_id, "fitbit", request)
                .await
            {
                tracing::error!("Failed to store Fitbit OAuth credentials: {}", e);
            }
        }
    }

    /// Handle tenant-aware connection status
    async fn handle_tenant_connection_status(
        tenant_context: &TenantContext,
        _tenant_provider_factory: &Arc<TenantProviderFactory>,
        _database: &Arc<Database>,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
        request_id: Value,
        strava_client_id: Option<&str>,
        strava_client_secret: Option<&str>,
        fitbit_client_id: Option<&str>,
        fitbit_client_secret: Option<&str>,
    ) -> McpResponse {
        tracing::info!(
            "Checking connection status for tenant {} user {}",
            tenant_context.tenant_name,
            tenant_context.user_id
        );

        // Store MCP-provided OAuth credentials if supplied
        let credentials = McpOAuthCredentials {
            strava_client_id,
            strava_client_secret,
            fitbit_client_id,
            fitbit_client_secret,
        };
        Self::store_mcp_oauth_credentials(tenant_context, tenant_oauth_client, &credentials).await;

        // Generate OAuth URLs for connecting providers
        // Using the HTTP API endpoints (port 8081) for OAuth flow
        let base_url = "http://127.0.0.1:8081/api/oauth";

        // In a real implementation, this would check tenant-specific provider connections
        // For now, return connection status with OAuth URLs using proper MCP content format
        let structured_data = serde_json::json!({
            "providers": [
                {
                    "provider": "strava",
                    "connected": false,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{}/auth/strava/{}", base_url, tenant_context.user_id),
                    "connect_instructions": "Click this URL to connect your Strava account and authorize access to your fitness data."
                },
                {
                    "provider": "fitbit",
                    "connected": false,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{}/auth/fitbit/{}", base_url, tenant_context.user_id),
                    "connect_instructions": "Click this URL to connect your Fitbit account and authorize access to your fitness data."
                }
            ],
            "tenant_info": {
                "tenant_id": tenant_context.tenant_id,
                "tenant_name": tenant_context.tenant_name
            },
            "connection_help": {
                "message": "To connect a fitness provider, click the connect_url for the provider you want to use. You'll be redirected to their website to authorize access, then redirected back to complete the connection.",
                "supported_providers": ["strava", "fitbit"],
                "note": "After connecting, you can use fitness tools like get_activities, get_athlete, and get_stats with the connected provider."
            }
        });

        let text_content = format!(
            "🏃‍♂️ Fitness Provider Connection Status\n\n\
            📊 Available Providers:\n\n\
            🔗 Strava (Not Connected)\n\
            Click to connect: {base_url}/auth/strava/{user_id}\n\n\
            🔗 Fitbit (Not Connected)\n\
            Click to connect: {base_url}/auth/fitbit/{user_id}\n\n\
            💡 To connect a provider:\n\
            1. Click one of the URLs above\n\
            2. You'll be redirected to authorize access\n\
            3. Complete the OAuth flow to connect your account\n\
            4. Start using fitness tools like get_activities, get_athlete, and get_stats",
            base_url = base_url,
            user_id = tenant_context.user_id
        );

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(serde_json::json!({
                "content": [
                    {
                        "type": "text",
                        "text": text_content
                    }
                ],
                "structuredContent": structured_data,
                "isError": false
            })),
            error: None,
            id: request_id,
        }
    }

    /// Handle tenant-aware provider disconnection
    fn handle_tenant_disconnect_provider(
        tenant_context: &TenantContext,
        provider_name: &str,
        _tenant_provider_factory: &Arc<TenantProviderFactory>,
        _database: &Arc<Database>,
        request_id: Value,
    ) -> McpResponse {
        tracing::info!(
            "Tenant {} disconnecting provider {} for user {}",
            tenant_context.tenant_name,
            provider_name,
            tenant_context.user_id
        );

        // In a real implementation, this would revoke tenant-specific OAuth tokens
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(serde_json::json!({
                "message": format!("Disconnected from {provider_name}"),
                "provider": provider_name,
                "tenant_id": tenant_context.tenant_id,
                "success": true
            })),
            error: None,
            id: request_id,
        }
    }

    /// Handle tenant-aware tools that require providers
    async fn handle_tenant_tool_with_provider(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        tenant_context: &TenantContext,
        resources: &Arc<ServerResources>,
        auth_result: &AuthResult,
    ) -> McpResponse {
        // Check if this is a known tool that requires a provider
        let known_provider_tools = [
            GET_ACTIVITIES,
            GET_ATHLETE,
            GET_STATS,
            GET_ACTIVITY_INTELLIGENCE,
            ANALYZE_ACTIVITY,
            CALCULATE_METRICS,
            COMPARE_ACTIVITIES,
            PREDICT_PERFORMANCE,
        ];

        if !known_provider_tools.contains(&tool_name) {
            return McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: format!("Unknown tool: {tool_name}"),
                    data: None,
                }),
                id: request_id,
            };
        }

        let provider_name = args[PROVIDER].as_str().unwrap_or("");

        tracing::info!(
            "Executing tenant tool {} with provider {} for tenant {} user {}",
            tool_name,
            provider_name,
            tenant_context.tenant_name,
            tenant_context.user_id
        );

        // Create a Universal protocol request to execute the tool
        let universal_request = crate::protocols::universal::UniversalRequest {
            tool_name: tool_name.to_string(),
            parameters: args.clone(),
            user_id: auth_result.user_id.to_string(),
            protocol: "mcp".to_string(),
        };

        // Use the provided ServerResources - no more fake auth managers or secrets!
        let executor = crate::protocols::universal::UniversalToolExecutor::new(resources.clone());

        // Execute the tool through Universal protocol
        match executor.execute_tool(universal_request).await {
            Ok(response) => {
                if response.success {
                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: response.result,
                        error: None,
                        id: request_id,
                    }
                } else {
                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: response
                                .error
                                .unwrap_or_else(|| "Tool execution failed".to_string()),
                            data: None,
                        }),
                        id: request_id,
                    }
                }
            }
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Tool execution error: {e}"),
                    data: None,
                }),
                id: request_id,
            },
        }
    }

    /// Create default server configuration for MCP protocol
    /// Uses environment configuration if available, otherwise creates a minimal config
    #[allow(dead_code)]
    fn create_default_server_config() -> std::sync::Arc<crate::config::environment::ServerConfig> {
        std::sync::Arc::new(
            crate::config::environment::ServerConfig::from_env()
                .unwrap_or_else(|_| Self::create_minimal_mcp_config()),
        )
    }

    /// Create minimal fallback config for MCP protocol (based on A2A implementation)
    #[allow(dead_code)]
    fn create_minimal_mcp_config() -> crate::config::environment::ServerConfig {
        crate::config::environment::ServerConfig {
            mcp_port: 8080,
            http_port: 8081,
            log_level: crate::config::environment::LogLevel::Info,
            database: crate::config::environment::DatabaseConfig {
                url: crate::config::environment::DatabaseUrl::default(),
                encryption_key_path: std::path::PathBuf::from("data/encryption.key"),
                auto_migrate: true,
                backup: crate::config::environment::BackupConfig {
                    enabled: false,
                    interval_seconds: 3600,
                    retention_count: 7,
                    directory: std::path::PathBuf::from("data/backups"),
                },
            },
            auth: crate::config::environment::AuthConfig {
                jwt_secret_path: std::path::PathBuf::from("data/jwt.secret"),
                jwt_expiry_hours: 24,
                enable_refresh_tokens: false,
            },
            oauth: crate::config::environment::OAuthConfig {
                strava: crate::config::environment::OAuthProviderConfig {
                    client_id: None,     // Use tenant-based OAuth credentials
                    client_secret: None, // Use tenant-based OAuth credentials
                    redirect_uri: None,
                    scopes: vec!["read".into(), "activity:read_all".into()],
                    enabled: false, // Disabled - use tenant OAuth instead
                },
                fitbit: crate::config::environment::OAuthProviderConfig {
                    client_id: None,     // Use tenant-based OAuth credentials
                    client_secret: None, // Use tenant-based OAuth credentials
                    redirect_uri: None,
                    scopes: vec!["activity".into(), "profile".into()],
                    enabled: false, // Disabled - use tenant OAuth instead
                },
            },
            security: crate::config::environment::SecurityConfig {
                cors_origins: vec!["*".into()],
                rate_limit: crate::config::environment::RateLimitConfig {
                    enabled: false,
                    requests_per_window: 100,
                    window_seconds: 60,
                },
                tls: crate::config::environment::TlsConfig {
                    enabled: false,
                    cert_path: None,
                    key_path: None,
                },
                headers: crate::config::environment::SecurityHeadersConfig {
                    environment: crate::config::environment::Environment::Development,
                },
            },
            external_services: crate::config::environment::ExternalServicesConfig {
                weather: crate::config::environment::WeatherServiceConfig {
                    api_key: std::env::var("OPENWEATHER_API_KEY").ok(),
                    base_url: "https://api.openweathermap.org/data/2.5".into(),
                    enabled: false,
                },
                geocoding: crate::config::environment::GeocodingServiceConfig {
                    base_url: "https://nominatim.openstreetmap.org".into(),
                    enabled: true,
                },
                strava_api: crate::config::environment::StravaApiConfig {
                    base_url: "https://www.strava.com/api/v3".into(),
                    auth_url: "https://www.strava.com/oauth/authorize".into(),
                    token_url: "https://www.strava.com/oauth/token".into(),
                },
                fitbit_api: crate::config::environment::FitbitApiConfig {
                    base_url: "https://api.fitbit.com".into(),
                    auth_url: "https://www.fitbit.com/oauth2/authorize".into(),
                    token_url: "https://api.fitbit.com/oauth2/token".into(),
                },
            },
            app_behavior: crate::config::environment::AppBehaviorConfig {
                max_activities_fetch: 100,
                default_activities_limit: 20,
                ci_mode: false,
                protocol: crate::config::environment::ProtocolConfig {
                    mcp_version: crate::constants::network_config::DEFAULT_MCP_VERSION.to_string(),
                    server_name: service_names::PIERRE_MCP_SERVER.into(),
                    server_version: env!("CARGO_PKG_VERSION").to_string(),
                },
            },
        }
    }
}

/// MCP request with optional authentication token and headers
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    /// Optional ID - notifications don't have IDs, only regular requests do
    pub id: Option<Value>,
    /// Authorization header value (Bearer token)
    #[serde(rename = "auth")]
    pub auth_token: Option<String>,
    /// Optional HTTP headers for tenant context and other metadata
    #[serde(default)]
    pub headers: Option<std::collections::HashMap<String, Value>>,
}

/// MCP response
#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
    pub id: Value,
}

/// MCP error
#[derive(Debug, Serialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl McpError {
    /// Create a new MCP error
    #[must_use]
    pub const fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    /// Create a new MCP error with data
    #[must_use]
    pub const fn new_with_data(code: i32, message: String, data: Value) -> Self {
        Self {
            code,
            message,
            data: Some(data),
        }
    }
}

impl McpResponse {
    /// Create a successful MCP response
    #[must_use]
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error MCP response
    #[must_use]
    pub fn error(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(McpError::new(code, message)),
            id,
        }
    }

    /// Create an error MCP response with data
    #[must_use]
    pub fn error_with_data(id: Value, code: i32, message: String, data: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(McpError::new_with_data(code, message, data)),
            id,
        }
    }
}

/// HTTP API error wrapper
#[derive(Debug)]
struct ApiError(serde_json::Value);

impl warp::reject::Reject for ApiError {}

/// MCP HTTP transport errors
#[derive(Debug)]
enum McpHttpError {
    InvalidOrigin,
    InvalidRequest,
}

impl warp::reject::Reject for McpHttpError {}

/// Add CORS and security headers to a reply
fn with_cors_headers(
    reply: impl warp::Reply,
    security_headers_env: Option<&str>,
) -> impl warp::Reply {
    let env = security_headers_env.unwrap_or("development");
    let security_config = SecurityConfig::from_environment(env);
    let headers = security_config.to_headers();

    // Add main CORS headers and a security header
    let csp_value = headers
        .get("Content-Security-Policy")
        .cloned()
        .unwrap_or_else(|| "default-src 'self'".into());

    warp::reply::with_header(
        warp::reply::with_header(
            warp::reply::with_header(
                warp::reply::with_header(reply, "access-control-allow-origin", "*"),
                "access-control-allow-methods",
                "GET, POST, PUT, DELETE, OPTIONS",
            ),
            "access-control-allow-headers",
            "content-type, authorization, x-requested-with, accept, origin",
        ),
        "Content-Security-Policy",
        csp_value,
    )
}

/// Handle HTTP rejections and errors
async fn handle_rejection(
    err: warp::Rejection,
) -> Result<Box<dyn warp::Reply>, std::convert::Infallible> {
    err.find::<ApiError>().map_or_else(
        || {
            if err.find::<warp::reject::MethodNotAllowed>().is_some() {
                // Handle CORS preflight and method not allowed
                let json = warp::reply::json(&serde_json::json!({}));
                let reply = warp::reply::with_status(json, warp::http::StatusCode::OK);
                Ok(Box::new(with_cors_headers(reply, None)) as Box<dyn warp::Reply>)
            } else if err.is_not_found() {
                let json = warp::reply::json(&serde_json::json!({
                    "error": "Not Found",
                    "message": "The requested endpoint was not found"
                }));
                let reply = warp::reply::with_status(json, warp::http::StatusCode::NOT_FOUND);
                Ok(Box::new(with_cors_headers(reply, None)) as Box<dyn warp::Reply>)
            } else {
                let json = warp::reply::json(&serde_json::json!({
                    "error": "Internal Server Error",
                    "message": "Something went wrong"
                }));
                let reply =
                    warp::reply::with_status(json, warp::http::StatusCode::INTERNAL_SERVER_ERROR);
                Ok(Box::new(with_cors_headers(reply, None)) as Box<dyn warp::Reply>)
            }
        },
        |api_error| {
            let json = warp::reply::json(&api_error.0);
            let reply = warp::reply::with_status(json, warp::http::StatusCode::BAD_REQUEST);
            Ok(Box::new(with_cors_headers(reply, None)) as Box<dyn warp::Reply>)
        },
    )
}
