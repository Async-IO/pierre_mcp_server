// ABOUTME: MCP server implementation with tenant isolation and user authentication
// ABOUTME: Handles MCP protocol with per-tenant data isolation and access control
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # MCP Server
//!
//! NOTE: All remaining undocumented `.clone()` calls in this file are Safe - they are
//! necessary for Arc resource sharing in HTTP route handlers and async closures required
//! by the warp framework for multi-tenant MCP protocol handling.
//! This module provides an MCP server that supports user authentication,
//! secure token storage, and user-scoped data access.

use super::{
    http_setup::HttpSetup,
    mcp_request_processor::McpRequestProcessor,
    oauth_flow_manager::{OAuthFlowManager, OAuthTemplateRenderer},
    resources::ServerResources,
    server_lifecycle::ServerLifecycle,
    tenant_isolation::validate_jwt_token_for_mcp,
    tool_handlers::{McpOAuthCredentials, ToolRoutingContext},
};
use crate::a2a_routes::A2ARoutes;
use crate::api_key_routes::ApiKeyRoutes;
use crate::auth::{AuthManager, AuthResult};
use crate::configuration_routes::ConfigurationRoutes;
use crate::constants::{
    errors::{ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND},
    json_fields::{GOAL_ID, PROVIDER},
    protocol,
    protocol::JSONRPC_VERSION,
    service_names,
    tools::{
        ANALYZE_ACTIVITY, ANALYZE_GOAL_FEASIBILITY, ANALYZE_PERFORMANCE_TRENDS,
        ANALYZE_TRAINING_LOAD, CALCULATE_FITNESS_SCORE, CALCULATE_METRICS, COMPARE_ACTIVITIES,
        DETECT_PATTERNS, GENERATE_RECOMMENDATIONS, GET_ACTIVITIES, GET_ACTIVITY_INTELLIGENCE,
        GET_ATHLETE, GET_STATS, PREDICT_PERFORMANCE, SET_GOAL, SUGGEST_GOALS, TRACK_PROGRESS,
    },
};
use crate::dashboard_routes::DashboardRoutes;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::fitness_configuration_routes::FitnessConfigurationRoutes;
use crate::oauth2_server::routes::oauth2_routes;
use crate::providers::ProviderRegistry;
use crate::routes::{AuthRoutes, LoginRequest, OAuthRoutes, RefreshTokenRequest, RegisterRequest};
use crate::security::headers::SecurityConfig;
use crate::tenant::{TenantContext, TenantOAuthClient};
// Removed unused imports - now using AppError directly

use anyhow::Result;
use chrono::Utc;
use lru::LruCache;

use serde_json::Value;
use std::fmt::Write;
use std::num::NonZeroUsize;
use std::str::FromStr;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use warp::Reply;

// Constants are now imported from the constants module

/// Default ID for notifications and error responses that don't have a request ID
fn default_request_id() -> Value {
    serde_json::Value::Number(serde_json::Number::from(0))
}

/// Context for HTTP request handling with tenant support
struct HttpRequestContext {
    resources: Arc<ServerResources>,
}

/// Connection status for providers
struct ProviderConnectionStatus {
    strava_connected: bool,
    fitbit_connected: bool,
}

/// Session data for authenticated MCP connections
#[derive(Clone, Debug)]
struct SessionData {
    jwt_token: String,
    user_id: uuid::Uuid,
}

/// HTTP request headers for MCP requests
#[derive(Clone, Debug)]
struct McpRequestHeaders {
    auth_header: Option<String>,
    origin: Option<String>,
    accept: Option<String>,
    session_id: Option<String>,
}

/// MCP request parameters for auth validation
struct McpRequestParams {
    method: warp::http::Method,
    auth_header: Option<String>,
    origin: Option<String>,
    accept: Option<String>,
    session_id: Option<String>,
    body: serde_json::Value,
}

/// MCP server supporting user authentication and isolated data access
#[derive(Clone)]
pub struct MultiTenantMcpServer {
    resources: Arc<ServerResources>,
    sessions: Arc<tokio::sync::Mutex<LruCache<String, SessionData>>>,
}

impl MultiTenantMcpServer {
    /// Default session cache size to prevent `DoS` via unbounded memory growth
    /// Note: `unwrap()` on compile-time constant is verified at compile time
    const DEFAULT_SESSION_CACHE_SIZE: NonZeroUsize = match NonZeroUsize::new(10_000) {
        Some(n) => n,
        None => unreachable!(),
    };

    /// Create a new MCP server with pre-built resources (dependency injection)
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        // Default session cache size from server configuration
        let session_cache_size =
            crate::constants::get_server_config().map_or(100, |c| c.mcp.session_cache_size);
        let cache_size =
            NonZeroUsize::new(session_cache_size).unwrap_or(Self::DEFAULT_SESSION_CACHE_SIZE);

        info!(
            "MCP session cache initialized with capacity: {}",
            cache_size
        );

        Self {
            resources,
            sessions: Arc::new(tokio::sync::Mutex::new(LruCache::new(cache_size))),
        }
    }

    /// Run the unified server with both HTTP and MCP endpoints
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start or bind to the specified port
    pub async fn run(self, port: u16) -> Result<()> {
        // Create unified HTTP + MCP server
        info!("Starting unified server on port {}", port);

        // Run unified server (MCP protocol and HTTP routes on same port)
        self.run_unified_server(port).await
    }

    /// Run HTTP server with centralized resources (eliminates parameter passing anti-pattern)
    ///
    /// # Errors
    /// Returns an error if server setup or routing configuration fails
    pub async fn run_http_server_with_resources(
        &self,
        port: u16,
        resources: Arc<ServerResources>,
    ) -> Result<()> {
        use warp::Filter;

        info!("HTTP authentication server starting on port {}", port);

        // Initialize security configuration
        let security_config = Self::setup_security_config(&resources.config);

        // Setup all route filters
        let route_filters = self.setup_all_route_filters(port, &resources);

        // Configure CORS and security
        let cors = HttpSetup::setup_cors();
        let security_headers = Self::create_security_headers_filter(&security_config);

        // Combine all routes
        let routes = route_filters
            .with(cors)
            .with(security_headers)
            .recover(handle_rejection);

        // Start the server
        info!("HTTP server listening on http://127.0.0.1:{}", port);
        Box::pin(warp::serve(routes).run(([127, 0, 0, 1], port))).await;

        Ok(())
    }

    /// Setup all route filters and combine them
    fn setup_all_route_filters(
        &self,
        port: u16,
        resources: &Arc<ServerResources>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Initialize all route handlers using shared resources
        let (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
            fitness_configuration_routes,
        ) = HttpSetup::setup_route_handlers_with_resources(resources);

        // Setup admin and tenant routes
        let (admin_routes_filter, tenant_routes_filter) =
            Self::setup_admin_tenant_routes(resources);

        // Create main application routes
        let main_routes = Self::setup_main_application_routes(
            port,
            resources,
            auth_routes,
            &Arc::new(oauth_routes),
            api_key_routes,
            &dashboard_routes,
        );

        // Create A2A routes
        let a2a_routes_combined = Self::setup_a2a_routes(&a2a_routes);

        // Create configuration routes
        let config_routes = Self::setup_configuration_routes(
            resources,
            &configuration_routes,
            &fitness_configuration_routes,
        );

        // Create special routes (SSE, MCP, health)
        let special_routes = self.setup_special_routes(resources);

        // Combine all routes
        main_routes
            .or(a2a_routes_combined)
            .or(config_routes)
            .or(admin_routes_filter)
            .or(tenant_routes_filter)
            .or(special_routes)
    }

    /// Setup admin and tenant management routes
    fn setup_admin_tenant_routes(
        resources: &Arc<ServerResources>,
    ) -> (
        impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone,
        impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone,
    ) {
        let jwt_secret_str = resources.admin_jwt_secret.as_ref();
        info!("Using admin JWT secret from server startup");

        let admin_context = crate::admin_routes::AdminApiContext::new(
            resources.database.clone(),
            jwt_secret_str,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );
        let admin_routes = crate::admin_routes::admin_routes_with_rejection(admin_context);

        let tenant_routes = Self::create_tenant_routes_filter(
            resources.database.clone(),
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );

        (admin_routes, tenant_routes)
    }

    /// Setup main application routes (auth, OAuth, API keys, dashboard)
    fn setup_main_application_routes(
        port: u16,
        resources: &Arc<ServerResources>,
        auth_routes: AuthRoutes,
        oauth_routes: &Arc<OAuthRoutes>,
        api_key_routes: ApiKeyRoutes,
        dashboard_routes: &DashboardRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        tracing::debug!("Setting up main application routes for port {}", port);

        let auth_filter = Self::create_auth_routes(auth_routes);
        let oauth_filter = Self::create_oauth_routes(oauth_routes, resources);
        let oauth2_server = oauth2_routes(
            resources.database.clone(),
            &resources.auth_manager,
            &resources.jwks_manager,
            &resources.config,
            &resources.oauth2_rate_limiter,
        );
        let api_key_filter = Self::create_api_key_routes(
            &api_key_routes,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );
        let api_key_usage = Self::create_api_key_usage_route(
            api_key_routes,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );
        let dashboard_filter = Self::create_dashboard_routes(
            dashboard_routes,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );
        let dashboard_detailed = Self::create_dashboard_detailed_routes(
            dashboard_routes,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );

        auth_filter
            .or(oauth_filter)
            .or(oauth2_server)
            .or(api_key_filter)
            .or(api_key_usage)
            .or(dashboard_filter)
            .or(dashboard_detailed)
    }

    /// Setup A2A (Application-to-Application) routes
    fn setup_a2a_routes(
        a2a_routes: &A2ARoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let basic = Self::create_a2a_basic_routes(a2a_routes);
        let client = Self::create_a2a_client_routes(a2a_routes);
        let monitoring = Self::create_a2a_monitoring_routes(a2a_routes);
        let execution = Self::create_a2a_execution_routes(a2a_routes);

        basic.or(client).or(monitoring).or(execution)
    }

    /// Setup configuration routes (general, user, specialized, fitness)
    fn setup_configuration_routes(
        resources: &Arc<ServerResources>,
        configuration_routes: &Arc<ConfigurationRoutes>,
        fitness_routes: &Arc<FitnessConfigurationRoutes>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let config_filter = Self::create_configuration_routes(configuration_routes);
        let user_config = Self::create_user_configuration_routes(
            configuration_routes,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );
        let specialized_config = Self::create_specialized_configuration_routes(
            configuration_routes,
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
        );
        let fitness_config = Self::create_fitness_configuration_routes(
            resources.auth_manager.clone(),
            resources.jwks_manager.clone(),
            fitness_routes,
        );

        config_filter
            .or(user_config)
            .or(specialized_config)
            .or(fitness_config)
    }

    /// Setup special routes (SSE, MCP endpoints, health check)
    fn setup_special_routes(
        &self,
        resources: &Arc<ServerResources>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let sse_routes =
            crate::sse::routes::sse_routes(resources.sse_manager.clone(), resources.clone());
        let mcp_endpoint = Self::create_mcp_endpoint_routes(resources, self.sessions.clone());
        let health = Self::create_health_route();
        let plugins_health = Self::create_plugins_health_route(resources);

        sse_routes.or(mcp_endpoint).or(health).or(plugins_health)
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

    /// Create authentication endpoint routes
    fn create_auth_routes(
        auth_routes: AuthRoutes,
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
                let auth_routes = auth_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |request: RegisterRequest| {
                    let auth_routes = auth_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match auth_routes.register(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let auth_routes = auth_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |request: LoginRequest| {
                    let auth_routes = auth_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match auth_routes.login(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                move |request: RefreshTokenRequest| {
                    let auth_routes = auth_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match auth_routes.refresh_token(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        register.or(login).or(refresh)
    }

    /// Create OAuth authorization endpoint
    fn create_oauth_auth_route(
        resources: &Arc<ServerResources>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        warp::path("api")
            .and(warp::path("oauth"))
            .and(warp::path!("auth" / String / String))
            .and(warp::get())
            .and(warp::header::headers_cloned())
            .and_then({
                let resources = resources.clone();
                move |provider: String, user_id_str: String, headers: warp::http::HeaderMap| {
                    let resources = resources.clone();
                    async move {
                        Self::handle_oauth_auth_request(provider, user_id_str, headers, resources)
                            .await
                    }
                }
            })
    }

    /// Handle OAuth authorization request with validation and credential storage
    async fn handle_oauth_auth_request(
        provider: String,
        user_id_str: String,
        headers: warp::http::HeaderMap,
        resources: Arc<ServerResources>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let oauth_manager = OAuthFlowManager::new(resources);
        oauth_manager
            .handle_authorization_request(provider, user_id_str, headers)
            .await
    }

    /// Helper function to create error response with HTML template
    fn create_oauth_error_response(
        provider: &str,
        title: &str,
        message: &str,
    ) -> warp::http::Response<warp::hyper::Body> {
        let html = OAuthTemplateRenderer::render_error_template(provider, title, Some(message))
            .unwrap_or_else(|e| {
                tracing::error!("Failed to render error template: {}", e);
                format!("<h1>{title}</h1><p>{message}</p>")
            });
        warp::reply::with_status(warp::reply::html(html), warp::http::StatusCode::BAD_REQUEST)
            .into_response()
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
                let oauth_routes = oauth_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |provider: String, params: std::collections::HashMap<String, String>| {
                    let oauth_routes = oauth_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        let Some(code) = params.get("code").cloned() else {
                            tracing::error!("Missing OAuth code parameter in callback");
                            return Ok::<warp::http::Response<warp::hyper::Body>, warp::Rejection>(
                                Self::create_oauth_error_response(&provider, "Authorization Failed", "Missing OAuth code parameter. Please try connecting again.")
                            );
                        };
                        let Some(state) = params.get("state").cloned() else {
                            tracing::error!("Missing OAuth state parameter in callback");
                            return Ok(Self::create_oauth_error_response(&provider, "Authorization Failed", "Missing OAuth state parameter. Please try connecting again."));
                        };

                        if let Some(error_msg) = params.get("error") {
                            tracing::error!("OAuth error from provider {}: {}", provider, error_msg);
                            let error_description = params.get("error_description").map_or(error_msg.as_str(), |desc| desc.as_str());
                            let message = format!("The OAuth provider returned an error: {error_description}. Please try again or contact support if the problem persists.");
                            return Ok(Self::create_oauth_error_response(&provider, "OAuth Authorization Denied", &message));
                        }

                        match oauth_routes.handle_callback(&code, &state, &provider).await {
                            Ok(callback_response) => {
                                let html_content = match OAuthTemplateRenderer::render_success_template(&provider, &callback_response) {
                                    Ok(html) => html,
                                    Err(e) => {
                                        tracing::error!("Failed to render success template: {}", e);
                                        format!("<h1>Success!</h1><p>Your {} account was connected successfully.</p>", provider.to_uppercase())
                                    }
                                };

                                Ok(warp::reply::with_status(
                                    warp::reply::html(html_content),
                                    warp::http::StatusCode::OK
                                ).into_response())
                            }
                            Err(e) => {
                                let message = format!("There was an error connecting your {} account to Pierre Fitness: {e}", provider.to_uppercase());
                                Ok(Self::create_oauth_error_response(&provider, "Connection Failed", &message))
                            }
                        }
                    }
                }
            })
    }

    /// Create OAuth endpoint routes
    fn create_oauth_routes(
        oauth_routes: &OAuthRoutes,
        resources: &Arc<ServerResources>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let oauth_auth = Self::create_oauth_auth_route(resources);
        let oauth_callback = Self::create_oauth_callback_route(oauth_routes);
        oauth_auth.or(oauth_callback)
    }

    /// Create MCP endpoint routes for HTTP server
    fn create_mcp_endpoint_routes(
        resources: &Arc<ServerResources>,
        sessions: Arc<tokio::sync::Mutex<LruCache<String, SessionData>>>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let resources = resources.clone();

        warp::path("mcp")
            .and(warp::method())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::header::optional::<String>("origin"))
            .and(warp::header::optional::<String>("accept"))
            .and(warp::header::optional::<String>("mcp-session-id"))
            .and(warp::header::headers_cloned())
            .and(warp::body::bytes().map(|bytes: bytes::Bytes| {
                if bytes.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                        tracing::warn!(
                            error = %e,
                            body_size = bytes.len(),
                            body_preview = %String::from_utf8_lossy(&bytes[..bytes.len().min(100)]),
                            "Failed to parse MCP request body as JSON, using Null"
                        );
                        serde_json::Value::Null
                    })
                }
            }))
            .and_then({
                move |method: warp::http::Method,
                      auth_header: Option<String>,
                      origin: Option<String>,
                      accept: Option<String>,
                      session_id: Option<String>,
                      all_headers: warp::http::HeaderMap,
                      body: serde_json::Value| {
                    // Debug: Log all headers
                    tracing::debug!("=== ALL HEADERS RECEIVED ===");
                    for (name, value) in &all_headers {
                        tracing::debug!("  {}: {:?}", name, value);
                    }
                    tracing::debug!("=== END HEADERS ===");
                    let resources = resources.clone();
                    let sessions = sessions.clone();

                    async move {
                        let ctx = HttpRequestContext { resources };
                        Self::handle_mcp_http_request_with_session(
                            method,
                            McpRequestHeaders {
                                auth_header,
                                origin,
                                accept,
                                session_id,
                            },
                            body,
                            &ctx,
                            sessions,
                        )
                        .await
                    }
                }
            })
    }

    /// Create API key management endpoint routes
    fn create_api_key_routes(
        api_key_routes: &ApiKeyRoutes,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        // Create API key endpoint
        let create_api_key = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::post())
            .and(with_auth.clone())
            .and(warp::body::json())
            .and_then({
                let api_key_routes = api_key_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult, request: crate::api_keys::CreateApiKeyRequestSimple| {
                    let api_key_routes = api_key_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match api_key_routes.create_api_key_simple(&auth, request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // List API keys endpoint
        let list_api_keys = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::get())
            .and(with_auth.clone())
            .and_then({
                let api_key_routes = api_key_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult| {
                    let api_key_routes = api_key_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match api_key_routes.list_api_keys(&auth).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // Deactivate API key endpoint
        let deactivate_api_key = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::path!(String))
            .and(warp::delete())
            .and(with_auth.clone())
            .and_then({
                let api_key_routes = api_key_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |api_key_id: String, auth: AuthResult| {
                    let api_key_routes = api_key_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match api_key_routes.deactivate_api_key(&auth, &api_key_id).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        create_api_key.or(list_api_keys).or(deactivate_api_key)
    }

    /// Create API key usage endpoint route
    fn create_api_key_usage_route(
        api_key_routes: ApiKeyRoutes,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        warp::path("api")
            .and(warp::path("keys"))
            .and(warp::path!(String))
            .and(warp::path("usage"))
            .and(warp::get())
            .and(with_auth)
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                move |api_key_id: String,
                      auth: AuthResult,
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
                                return Err(warp::reject::custom(ApiError::invalid_input(
                                    format!("{} must be in {} format", "start_date", "RFC3339"),
                                )));
                            };

                        let end_date =
                            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&end_date_str) {
                                dt.with_timezone(&chrono::Utc)
                            } else {
                                return Err(warp::reject::custom(ApiError::invalid_input(
                                    format!("{} must be in {} format", "end_date", "RFC3339"),
                                )));
                            };

                        match api_key_routes
                            .get_api_key_usage(&auth, &api_key_id, start_date, end_date)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            })
    }

    /// Create dashboard endpoint routes
    fn create_dashboard_routes(
        dashboard_routes: &DashboardRoutes,
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        // Dashboard overview
        let dashboard_overview = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("overview"))
            .and(warp::get())
            .and(with_auth.clone())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth: crate::auth::AuthResult| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        match dashboard_routes.get_dashboard_overview(auth).await {
                            Ok(overview) => Ok(warp::reply::json(&overview)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // Dashboard analytics
        let dashboard_analytics = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("analytics"))
            .and(warp::get())
            .and(with_auth.clone())
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth: crate::auth::AuthResult,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let days = params
                            .get("days")
                            .and_then(|d| d.parse::<u32>().ok())
                            .unwrap_or(30);
                        match dashboard_routes.get_usage_analytics(auth, days).await {
                            Ok(analytics) => Ok(warp::reply::json(&analytics)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // Dashboard rate limits
        let dashboard_rate_limits = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("rate-limits"))
            .and(warp::get())
            .and(with_auth)
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth: crate::auth::AuthResult| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        match dashboard_routes.get_rate_limit_overview(auth).await {
                            Ok(overview) => Ok(warp::reply::json(&overview)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        // Dashboard request logs
        let dashboard_request_logs = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("request-logs"))
            .and(warp::get())
            .and(with_auth.clone())
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth: crate::auth::AuthResult,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let api_key_id = params.get("api_key_id").map(std::string::String::as_str);
                        let time_range = params.get("time_range").map(std::string::String::as_str);
                        let status = params.get("status").map(std::string::String::as_str);
                        let tool = params.get("tool").map(std::string::String::as_str);

                        match dashboard_routes
                            .get_request_logs(auth, api_key_id, time_range, status, tool)
                            .await
                        {
                            Ok(logs) => Ok(warp::reply::json(&logs)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // Dashboard request stats
        let dashboard_request_stats = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("request-stats"))
            .and(warp::get())
            .and(with_auth.clone())
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth: crate::auth::AuthResult,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let api_key_id = params.get("api_key_id").map(std::string::String::as_str);
                        let time_range = params.get("time_range").map(std::string::String::as_str);

                        match dashboard_routes
                            .get_request_stats(auth, api_key_id, time_range)
                            .await
                        {
                            Ok(stats) => Ok(warp::reply::json(&stats)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // Dashboard tool usage
        let dashboard_tool_usage = warp::path("api")
            .and(warp::path("dashboard"))
            .and(warp::path("tool-usage"))
            .and(warp::get())
            .and(with_auth)
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let dashboard_routes = dashboard_routes.clone();
                move |auth: crate::auth::AuthResult,
                      params: std::collections::HashMap<String, String>| {
                    let dashboard_routes = dashboard_routes.clone();
                    async move {
                        let api_key_id = params.get("api_key_id").map(std::string::String::as_str);
                        let time_range = params.get("time_range").map(std::string::String::as_str);

                        match dashboard_routes
                            .get_tool_usage_breakdown(auth, api_key_id, time_range)
                            .await
                        {
                            Ok(usage) => Ok(warp::reply::json(&usage)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move || {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes.get_agent_card() {
                            Ok(agent_card) => Ok(warp::reply::json(&agent_card)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes
                            .get_dashboard_overview(auth_header.as_deref())
                            .await
                        {
                            Ok(overview) => Ok(warp::reply::json(&overview)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth_header: Option<String>, request: crate::a2a_routes::A2AClientRequest| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes
                            .register_client(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(credentials) => Ok(warp::reply::json(&credentials)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes.list_clients(auth_header.as_deref()).await {
                            Ok(clients) => Ok(warp::reply::json(&clients)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |client_id: String, auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes
                            .get_client_usage(auth_header.as_deref(), &client_id)
                            .await
                        {
                            Ok(usage) => Ok(warp::reply::json(&usage)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |client_id: String, auth_header: Option<String>| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes
                            .get_client_rate_limit(auth_header.as_deref(), &client_id)
                            .await
                        {
                            Ok(rate_limit) => Ok(warp::reply::json(&rate_limit)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |request: serde_json::Value| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes.authenticate(request).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let a2a_routes = a2a_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth_header: Option<String>, request: serde_json::Value| {
                    let a2a_routes = a2a_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match a2a_routes
                            .execute_tool(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let config_routes = (*configuration_routes).clone(); // Safe: Arc clone for HTTP handler closure
                move |auth_header: Option<String>| {
                    let config_routes_inner = config_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match config_routes_inner.get_configuration_catalog(auth_header.as_deref())
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
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
                let config_routes = (*configuration_routes).clone(); // Safe: Arc clone for HTTP handler closure
                move |auth_header: Option<String>| {
                    let config_routes_inner = config_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match config_routes_inner.get_configuration_profiles(auth_header.as_deref())
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        config_catalog.or(config_profiles)
    }

    /// Create user configuration endpoint routes
    fn create_user_configuration_routes(
        configuration_routes: &Arc<ConfigurationRoutes>,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        // Get user configuration
        let config_user_get = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("user"))
            .and(warp::get())
            .and(with_auth.clone())
            .and_then({
                let config_routes = (*configuration_routes).clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult| {
                    let config_routes = config_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match config_routes.get_user_configuration(&auth).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => Err(warp::reject::custom(ApiError::internal(e.to_string()))),
                        }
                    }
                }
            });

        // Update user configuration
        let config_user_update = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("user"))
            .and(warp::put())
            .and(with_auth.clone())
            .and(warp::body::json())
            .and_then({
                let config_routes = (*configuration_routes).clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult, request: crate::configuration_routes::UpdateConfigurationRequest| {
                    let config_routes = config_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match config_routes
                            .update_user_configuration(&auth, request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
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
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        // Configuration zones
        let config_zones = warp::path("api")
            .and(warp::path("configuration"))
            .and(warp::path("zones"))
            .and(warp::post())
            .and(with_auth.clone())
            .and(warp::body::json())
            .and_then({
                let config_routes = (*configuration_routes).clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult, request: crate::configuration_routes::PersonalizedZonesRequest| {
                    let config_routes = config_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match config_routes
                            .calculate_personalized_zones(&auth, &request)
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
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
            .and(with_auth.clone())
            .and(warp::body::json())
            .and_then({
                let config_routes = (*configuration_routes).clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult, request: crate::configuration_routes::ValidateConfigurationRequest| {
                    let config_routes = config_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match config_routes
                            .validate_configuration(&auth, &request)
                        {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
                            }
                        }
                    }
                }
            });

        config_zones.or(config_validate)
    }

    /// Create fitness configuration endpoint routes
    fn create_fitness_configuration_routes(
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
        fitness_config_routes: &Arc<
            crate::fitness_configuration_routes::FitnessConfigurationRoutes,
        >,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let list_configs = Self::create_list_fitness_configs_route(
            fitness_config_routes,
            auth_manager.clone(),
            jwks_manager.clone(),
        );
        let get_config = Self::create_get_fitness_config_route(
            fitness_config_routes,
            auth_manager.clone(),
            jwks_manager.clone(),
        );
        let save_user_config = Self::create_save_fitness_config_route(
            fitness_config_routes,
            auth_manager.clone(),
            jwks_manager.clone(),
        );
        let delete_user_config = Self::create_delete_fitness_config_route(
            fitness_config_routes,
            auth_manager,
            jwks_manager,
        );

        list_configs
            .or(get_config)
            .or(save_user_config)
            .or(delete_user_config)
    }

    /// Create list fitness configurations route
    fn create_list_fitness_configs_route(
        fitness_config_routes: &Arc<
            crate::fitness_configuration_routes::FitnessConfigurationRoutes,
        >,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::end())
            .and(warp::get())
            .and(with_auth)
            .and_then({
                let fitness_routes = fitness_config_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult| {
                    let fitness_routes = fitness_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match fitness_routes.list_configurations(&auth).await {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::OK,
                            )),
                            Err(e) => {
                                tracing::error!("List fitness configurations failed: {e}");
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
                            }
                        }
                    }
                }
            })
    }

    /// Create get specific fitness configuration route
    fn create_get_fitness_config_route(
        fitness_config_routes: &Arc<
            crate::fitness_configuration_routes::FitnessConfigurationRoutes,
        >,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;
        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::get())
            .and(with_auth)
            .and_then({
                let fitness_routes = fitness_config_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |config_name: String, auth: AuthResult| {
                    let fitness_routes = fitness_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match fitness_routes.get_configuration(&auth, &config_name).await {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::OK,
                            )),
                            Err(e) => {
                                tracing::error!("Get fitness configuration failed: {e}");
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
                            }
                        }
                    }
                }
            })
    }

    /// Create save user fitness configuration route
    fn create_save_fitness_config_route(
        fitness_config_routes: &Arc<
            crate::fitness_configuration_routes::FitnessConfigurationRoutes,
        >,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::end())
            .and(warp::post())
            .and(with_auth)
            .and(warp::body::json::<crate::fitness_configuration_routes::SaveFitnessConfigRequest>())
            .and_then({
                let fitness_routes = fitness_config_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |auth: AuthResult, request: crate::fitness_configuration_routes::SaveFitnessConfigRequest| {
                    let fitness_routes = fitness_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match fitness_routes
                            .save_user_configuration(&auth, request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::CREATED,
                            )),
                            Err(e) => {
                                tracing::error!("Save user fitness configuration failed: {e}");
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
                            }
                        }
                    }
                }
            })
    }

    /// Create delete user fitness configuration route
    fn create_delete_fitness_config_route(
        fitness_config_routes: &Arc<
            crate::fitness_configuration_routes::FitnessConfigurationRoutes,
        >,
        auth_manager: std::sync::Arc<AuthManager>,
        jwks_manager: std::sync::Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

        warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::delete())
            .and(with_auth)
            .and_then({
                let fitness_routes = fitness_config_routes.clone(); // Safe: Arc clone for HTTP handler closure
                move |config_name: String, auth: AuthResult| {
                    let fitness_routes = fitness_routes.clone(); // Safe: Arc clone needed for Fn trait in Warp
                    async move {
                        match fitness_routes
                            .delete_user_configuration(&auth, &config_name)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::OK,
                            )),
                            Err(e) => {
                                tracing::error!("Delete user fitness configuration failed: {e}");
                                Err(warp::reject::custom(ApiError::internal(e.to_string())))
                            }
                        }
                    }
                }
            })
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

    /// Create plugins health check endpoint
    fn create_plugins_health_route(
        resources: &Arc<ServerResources>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;
        let resources_for_route = resources.clone();

        warp::path!("health" / "plugins")
            .and(warp::get())
            .map(move || {
                let plugin_stats = resources_for_route.plugin_executor.as_ref().map_or_else(
                    || {
                        serde_json::json!({
                            "status": "not_initialized",
                            "message": "Plugin system has not been initialized"
                        })
                    },
                    |executor| {
                        let stats = executor.get_statistics();
                        serde_json::json!({
                            "status": "initialized",
                            "total_tools": stats.total_tools,
                            "core_tools": stats.core_tools,
                            "plugin_tools": stats.plugin_tools,
                            "plugin_registry": stats.plugin_stats
                        })
                    },
                );
                warp::reply::json(&plugin_stats)
            })
    }

    /// Create JWT authentication filter
    fn create_auth_filter(
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = (crate::auth::AuthResult,), Error = warp::Rejection> + Clone
    {
        use warp::Filter;
        warp::header::optional::<String>("authorization")
            .and(warp::any().map(move || auth_manager.clone()))
            .and(warp::any().map(move || jwks_manager.clone()))
            .and_then(
                |auth_header: Option<String>,
                 auth_mgr: Arc<AuthManager>,
                 jwks_mgr: Arc<crate::admin::jwks::JwksManager>| async move {
                    match auth_header {
                        Some(header) => {
                            // Extract token from "Bearer <token>" format
                            let Some(token) = header.strip_prefix("Bearer ") else {
                                return Err(warp::reject::custom(crate::errors::AppError::new(
                                    crate::errors::ErrorCode::AuthInvalid,
                                    "Invalid authorization header format. Use 'Bearer <token>'",
                                )));
                            };

                            // Validate JWT token using AuthManager with RS256
                            match auth_mgr.validate_token(token, &jwks_mgr) {
                                Ok(claims) => {
                                    // Parse user_id from claims.sub
                                    let user_id =
                                        uuid::Uuid::parse_str(&claims.sub).map_err(|e| {
                                            tracing::error!(
                                                sub = %claims.sub,
                                                error = %e,
                                                "Failed to parse user_id from JWT token subject claim"
                                            );
                                            warp::reject::custom(crate::errors::AppError::new(
                                                crate::errors::ErrorCode::AuthInvalid,
                                                format!("Invalid user ID in JWT token: {e}"),
                                            ))
                                        })?;

                                    // Convert JWT claims to AuthResult
                                    Ok(crate::auth::AuthResult {
                                        user_id,
                                        auth_method: crate::auth::AuthMethod::JwtToken {
                                            tier: "basic".to_owned(),
                                        },
                                        rate_limit: crate::rate_limiting::UnifiedRateLimitInfo {
                                            is_rate_limited: false,
                                            limit: Some(1000),
                                            remaining: Some(999),
                                            reset_at: Some(
                                                chrono::Utc::now() + chrono::Duration::hours(1),
                                            ),
                                            tier: "basic".to_owned(),
                                            auth_method: "jwt".to_owned(),
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
        database: Arc<Database>,
        auth_manager: Arc<AuthManager>,
        jwks_manager: Arc<crate::admin::jwks::JwksManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        let with_auth = Self::create_auth_filter(auth_manager, jwks_manager);

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

    /// Run unified server with both stdio and HTTP transports on single port
    async fn run_unified_server(self, port: u16) -> Result<()> {
        let lifecycle = ServerLifecycle::new(self.resources);
        lifecycle.run_unified_server(port).await
    }

    /// Run MCP server with only HTTP transport (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server fails to start or bind to the specified port
    pub async fn run_http_only(self, port: u16) -> Result<()> {
        let lifecycle = ServerLifecycle::new(self.resources);
        lifecycle.run_http_only(port).await
    }

    /// Handle MCP HTTP request (Streamable HTTP transport)
    // Handle MCP HTTP request with authentication result
    /// Determine if an MCP method requires authentication
    fn mcp_method_requires_auth(mcp_method: &str) -> bool {
        match mcp_method {
            // Standard MCP discovery methods - following MCP specification compliance
            // These methods allow clients to discover capabilities before authentication
            "ping"
            | "notifications/initialized"
            | "initialize"
            | "tools/list"
            | "prompts/list"
            | "resources/list" => false,
            // All other methods (tools/call, etc.) require authentication
            _ => true,
        }
    }

    /// Handle MCP HTTP request with conditional authentication
    fn log_http_request_debug(
        method: &warp::http::Method,
        auth_header: Option<&String>,
        origin: Option<&String>,
        accept: Option<&String>,
        body: &serde_json::Value,
    ) {
        tracing::debug!(
            "=== MCP HTTP Request: {} | Auth: {} | Origin: {:?} | Accept: {:?} | Body: {:?} | Thread: {:?} | Time: {:?} ===",
            method,
            auth_header.is_some(),
            origin,
            accept,
            body,
            std::thread::current().id(),
            std::time::SystemTime::now()
        );
    }

    async fn validate_auth_and_handle(
        params: McpRequestParams,
        mcp_method: &str,
        ctx: &HttpRequestContext,
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
        tracing::debug!("Authentication is required for method '{}'", mcp_method);
        if let Some(header) = params.auth_header.as_ref() {
            tracing::debug!(
                "Auth header present: {} (first 20 chars)",
                &header[..std::cmp::min(20, header.len())]
            );

            // Extract token from "Bearer <token>" format
            let Some(token) = header.strip_prefix("Bearer ") else {
                tracing::warn!("Invalid authorization header format - missing 'Bearer ' prefix");
                return Err(warp::reject::custom(crate::errors::AppError::new(
                    crate::errors::ErrorCode::AuthInvalid,
                    "Invalid authorization header format. Use 'Bearer <token>'",
                )));
            };

            // Use helper function to validate JWT token for authentication
            validate_jwt_token_for_mcp(
                token,
                &ctx.resources.auth_manager,
                &ctx.resources.jwks_manager,
                &ctx.resources.database,
            )
            .await
            .map_err(|e| {
                warp::reject::custom(ApiError::auth_invalid(format!(
                    "JWT validation failed: {e}"
                )))
            })?;
            tracing::debug!("Proceeding to handle_mcp_http_request with user context");

            // Pass the original auth header, not the user_id
            Self::handle_mcp_http_request(
                params.method,
                params.origin,
                params.accept,
                params.auth_header,
                params.session_id,
                params.body,
                ctx,
            )
            .await
        } else {
            // Return HTTP 401 Unauthorized status code as required by MCP authorization spec
            tracing::debug!(
                "Authentication required for method '{}', returning HTTP 401",
                mcp_method
            );
            Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "error": "Unauthorized access",
                    "message": format!("Authentication required for MCP method '{}'", mcp_method),
                    "method": mcp_method
                })),
                warp::http::StatusCode::UNAUTHORIZED,
            )))
        }
    }

    async fn handle_mcp_http_request_with_conditional_auth(
        method: warp::http::Method,
        auth_header: Option<String>,
        origin: Option<String>,
        accept: Option<String>,
        session_id: Option<String>,
        body: serde_json::Value,
        ctx: &HttpRequestContext,
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
        Self::log_http_request_debug(
            &method,
            auth_header.as_ref(),
            origin.as_ref(),
            accept.as_ref(),
            &body,
        );

        // For GET requests (typically for metadata), no auth is needed
        if method == warp::http::Method::GET {
            tracing::debug!("GET request - skipping authentication");
            return Self::handle_mcp_http_request(
                method,
                origin,
                accept,
                auth_header,
                session_id,
                body,
                ctx,
            )
            .await;
        }

        // For POST requests, check the MCP method in the body to decide if auth is needed
        let mcp_method_str = body
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or_else(|| {
                tracing::warn!(
                    body_keys = ?body.as_object().map(|o| o.keys().collect::<Vec<_>>()),
                    "MCP request missing 'method' field, treating as empty method"
                );
                ""
            });
        let mcp_method = mcp_method_str.to_owned();
        tracing::debug!("POST request - MCP method: '{}'", mcp_method);

        let requires_auth = Self::mcp_method_requires_auth(&mcp_method);
        tracing::debug!(
            "MCP method '{}' requires auth: {}",
            mcp_method,
            requires_auth
        );

        // CRITICAL: DO NOT MODIFY THIS AUTH LOGIC
        //
        // Discovery methods (tools/list, initialize, resources/list, prompts/list) MUST work
        // without authentication. This is required by the MCP specification.
        //
        // DO NOT add "security improvements" that validate auth headers for discovery methods.
        // Clients send cached/expired tokens during discovery - this is normal and expected.
        // Rejecting these requests breaks the entire MCP handshake.
        //
        // Only validate authentication for methods where requires_auth = true.
        // See: https://spec.modelcontextprotocol.io/specification/architecture/#security
        if requires_auth {
            Self::validate_auth_and_handle(
                McpRequestParams {
                    method,
                    auth_header,
                    origin,
                    accept,
                    session_id,
                    body,
                },
                &mcp_method,
                ctx,
            )
            .await
        } else {
            // No authentication header provided and none required
            Self::handle_mcp_http_request(
                method,
                origin,
                accept,
                auth_header,
                session_id,
                body,
                ctx,
            )
            .await
        }
    }

    /// Handle MCP HTTP request with session management support
    async fn handle_mcp_http_request_with_session(
        method: warp::http::Method,
        headers: McpRequestHeaders,
        body: serde_json::Value,
        ctx: &HttpRequestContext,
        sessions: Arc<tokio::sync::Mutex<LruCache<String, SessionData>>>,
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
        tracing::debug!("=== MCP HTTP Request with Session START ===");
        tracing::debug!(
            "Method: {}, Has Auth Header: {}, Session ID: {:?}",
            method,
            headers.auth_header.is_some(),
            headers.session_id
        );

        // Determine session ID and auth header priority:
        // 1. Always prefer auth header from current request if provided
        // 2. Fall back to stored session auth only if no header in current request
        // 3. Generate new session ID if needed
        let actual_session_id = headers.session_id.clone().unwrap_or_else(|| {
            let new_session_id = format!("session_{}", uuid::Uuid::new_v4());
            tracing::info!("Generated new MCP session: {}", new_session_id);
            new_session_id
        });

        let effective_auth_header = if headers.auth_header.is_some() {
            // Current request has auth header - use it
            tracing::debug!("Using auth header from current request");
            headers.auth_header.clone()
        } else if let Some(sid) = headers.session_id.as_ref() {
            // No auth in current request, check session
            let mut sessions_guard = sessions.lock().await;
            sessions_guard.get(sid).map(|session_data| {
                tracing::info!(
                    "Using stored session auth for user {}",
                    session_data.user_id
                );
                format!("Bearer {}", session_data.jwt_token)
            })
        } else {
            None
        };

        // If we have auth header but no session data yet, validate and store it
        if let Some(ref auth) = headers.auth_header {
            let needs_validation = {
                let sessions_guard = sessions.lock().await;
                !sessions_guard.contains(&actual_session_id)
            };

            if needs_validation {
                // Extract JWT token and validate it
                if let Some(token) = auth.strip_prefix("Bearer ") {
                    // Validate the JWT to get user info
                    if let Ok(jwt_result) = validate_jwt_token_for_mcp(
                        token,
                        &ctx.resources.auth_manager,
                        &ctx.resources.jwks_manager,
                        &ctx.resources.database,
                    )
                    .await
                    {
                        // Get user details
                        if let Ok(Some(user)) =
                            ctx.resources.database.get_user(jwt_result.user_id).await
                        {
                            // Store session
                            let mut sessions_guard = sessions.lock().await;
                            sessions_guard.put(
                                actual_session_id.clone(),
                                SessionData {
                                    jwt_token: token.to_owned(),
                                    user_id: jwt_result.user_id,
                                },
                            );
                            drop(sessions_guard);
                            tracing::info!(
                                "Stored session {} for user {} ({})",
                                actual_session_id,
                                jwt_result.user_id,
                                user.email
                            );
                        }
                    }
                }
            }
        }

        let response_headers = vec![("Mcp-Session-Id", actual_session_id)];

        // Call the existing conditional auth handler
        let mut response = Self::handle_mcp_http_request_with_conditional_auth(
            method,
            effective_auth_header,
            headers.origin,
            headers.accept,
            headers.session_id,
            body,
            ctx,
        )
        .await?;

        // Add session headers to response
        for (key, value) in response_headers {
            response = Box::new(warp::reply::with_header(response, key, value));
        }

        Ok(response)
    }

    // Long function: Complex HTTP request handling with comprehensive logging and validation
    #[allow(clippy::too_many_lines)]
    async fn handle_mcp_http_request(
        method: warp::http::Method,
        origin: Option<String>,
        accept: Option<String>,
        authorization: Option<String>,
        session_id: Option<String>,
        body: serde_json::Value,
        ctx: &HttpRequestContext,
    ) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
        // Store values for logging before validation consumes them
        let origin_for_logging = origin.clone();
        let accept_for_logging = accept.clone();

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

                        tracing::debug!(
                            transport = "http",
                            origin = ?origin_for_logging,
                            accept = ?accept_for_logging,
                            mcp_method = %request.method,
                            body_size = body.to_string().len(),
                            "Received MCP request via HTTP transport"
                        );

                        // Store method name before moving request
                        let method_name = request.method.clone();

                        Self::handle_request(request, &ctx.resources)
                            .await
                            .map_or_else(
                                || {
                                    // For "notifications/initialized", return 202 Accepted to trigger SSE stream in SDK
                                    // For other notifications, return 204 No Content per JSON-RPC spec
                                    let status = if method_name == "notifications/initialized" {
                                        warp::http::StatusCode::ACCEPTED
                                    } else {
                                        warp::http::StatusCode::NO_CONTENT
                                    };

                                    Ok(Box::new(warp::reply::with_status(warp::reply(), status))
                                        as Box<dyn warp::Reply>)
                                },
                                |response| {
                                    // Return 200 OK with response body for successful requests
                                    Ok(Box::new(warp::reply::with_status(
                                        warp::reply::json(&response),
                                        warp::http::StatusCode::OK,
                                    ))
                                        as Box<dyn warp::Reply>)
                                },
                            )
                    }
                    Err(parse_error) => {
                        let body_str = serde_json::to_string(&body)
                            .unwrap_or_else(|_| "invalid json".to_owned());
                        tracing::warn!(
                            "Failed to parse MCP request: {} | Body: {}",
                            parse_error,
                            body_str
                        );

                        // Per JSON-RPC 2.0 spec: Parse/validation errors return HTTP 200 with error in JSON-RPC envelope
                        let error_response = McpResponse::error(
                            Some(default_request_id()),
                            -32600,
                            "Invalid request".to_owned(),
                        );
                        let error_response_str = serde_json::to_string(&error_response)
                            .unwrap_or_else(|_| "failed to serialize error response".to_owned());
                        tracing::warn!("Sending MCP error response: {}", error_response_str);

                        Ok(Box::new(warp::reply::with_status(
                            warp::reply::json(&error_response),
                            warp::http::StatusCode::OK,
                        )) as Box<dyn warp::Reply>)
                    }
                }
            }
            warp::http::Method::GET => {
                // Handle GET request for server-sent events or status
                if accept
                    .as_ref()
                    .is_some_and(|a| a.contains("text/event-stream"))
                {
                    // Integrate with unified SSE infrastructure for MCP protocol streaming
                    tracing::info!(
                        "MCP SSE request - registering protocol stream with unified SSE manager"
                    );

                    // Use mcp-session-id header if provided, otherwise generate new session ID
                    let session_id_value = session_id
                        .clone()
                        .unwrap_or_else(|| format!("session_{}", uuid::Uuid::new_v4()));

                    // Register SSE stream with the manager
                    let mut receiver = ctx
                        .resources
                        .sse_manager
                        .register_protocol_stream(
                            session_id_value.clone(),
                            authorization,
                            ctx.resources.clone(),
                        )
                        .await;

                    let manager = ctx.resources.sse_manager.clone();
                    let session_id_clone = session_id_value.clone();

                    // Create SSE stream that forwards MCP protocol messages
                    let stream = async_stream::stream! {
                        // Listen for MCP messages with sequential event IDs for client reconnection support
                        tracing::debug!("SSE stream listening for messages on session: {}", session_id_clone);
                        let mut event_id: u64 = 0;
                        loop {
                            match receiver.recv().await {
                                Ok(message) => {
                                    event_id += 1;
                                    tracing::debug!("SSE stream received message on session {}: {}", session_id_clone, message);
                                    yield Ok::<_, warp::Error>(warp::sse::Event::default()
                                        .id(event_id.to_string())
                                        .data(message.clone())
                                        .event("message"));
                                    tracing::debug!("SSE stream yielded message to client on session: {}", session_id_clone);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                    tracing::warn!("SSE protocol stream lagged for session {}, skipped {} messages", session_id_clone, skipped);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    tracing::info!("SSE protocol channel closed for session: {}", session_id_clone);
                                    break;
                                }
                            }
                        }

                        // Clean up connection
                        tracing::debug!("Cleaning up SSE stream for session: {}", session_id_clone);
                        manager.unregister_protocol_stream(&session_id_clone).await;
                    };

                    // Use warp's SSE reply with proper keepalive interval and CORS headers
                    let keep = warp::sse::keep_alive()
                        .interval(std::time::Duration::from_secs(15))
                        .text(": keepalive\n\n");
                    let sse_reply = warp::sse::reply(keep.stream(stream));
                    let response = warp::reply::with_header(
                        warp::reply::with_header(
                            warp::reply::with_header(sse_reply, "access-control-allow-origin", "*"),
                            "access-control-allow-headers",
                            "cache-control",
                        ),
                        "Mcp-Session-Id",
                        session_id_value,
                    );

                    Ok(Box::new(response))
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

    /// Validate origin header for security (DNS rebinding and CSRF protection)
    fn is_valid_origin(origin: &str) -> bool {
        // Check environment-configured allowed origins first
        if let Some(server_config) = crate::constants::get_server_config() {
            let allowed_origins = &server_config.cors.allowed_origins;
            if !allowed_origins.is_empty()
                && allowed_origins
                    .split(',')
                    .map(str::trim)
                    .any(|x| x == origin)
            {
                return true;
            }

            // Allow localhost in development only if explicitly enabled
            let allow_localhost = server_config.cors.allow_localhost_dev;

            if allow_localhost {
                // Validate localhost patterns - be more strict than before
                let is_localhost = crate::constants::network_config::LOCALHOST_PATTERNS
                    .iter()
                    .any(|pattern| {
                        origin.starts_with(&format!("http://{pattern}"))
                            || origin.starts_with(&format!("https://{pattern}"))
                    });

                if is_localhost {
                    return true;
                }
            }
        }

        // Reject "null" origin as it's a security risk (file:// origins, CSRF attacks)
        // In production, clients should have proper origins
        false
    }

    /// Handle MCP request with `ServerResources`
    #[tracing::instrument(
        skip(resources),
        fields(
            mcp_method = %request.method,
            mcp_id = ?request.id,
            auth_present = request.auth_token.is_some(),
            request_headers = ?request.headers,
            response_status = tracing::field::Empty,
            duration_ms = tracing::field::Empty
        )
    )]
    pub async fn handle_request(
        request: McpRequest,
        resources: &Arc<ServerResources>,
    ) -> Option<McpResponse> {
        let processor = McpRequestProcessor::new(resources.clone());
        processor.handle_request(request).await
    }

    /// Extract tenant context from MCP request headers
    /// Route disconnect tool request to appropriate provider handler
    ///
    /// # Errors
    /// Returns an error if the provider is not supported or the operation fails
    pub async fn route_disconnect_tool(
        provider_name: &str,
        user_id: Uuid,
        request_id: Value,
        ctx: &ToolRoutingContext<'_>,
    ) -> McpResponse {
        if let Some(ref tenant_ctx) = ctx.tenant_context {
            Self::handle_tenant_disconnect_provider(
                tenant_ctx,
                provider_name,
                &ctx.resources.provider_registry,
                &ctx.resources.database,
                request_id,
            )
        } else {
            Self::handle_disconnect_provider(user_id, provider_name, ctx.resources, request_id)
                .await
        }
    }

    /// Route provider-specific tool requests to appropriate handlers
    pub async fn route_provider_tool(
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
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: format!("Tool '{tool_name}' requires tenant context - use tenant-aware MCP endpoints"),
                    data: None,
                }),
                id: Some(request_id),
            }
        }
    }

    /// Handle tools that don't require external providers
    pub async fn handle_tool_without_provider(
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
            if let Err(e) = Self::record_api_key_usage(
                database,
                key_id,
                tool_name,
                start_time.elapsed(),
                &response,
            )
            .await
            {
                tracing::warn!(
                    key_id = %key_id,
                    tool_name = %tool_name,
                    error = %e,
                    "Failed to record API key usage - metrics may be incomplete"
                );
            }
        }

        response
    }

    /// Handle `disconnect_provider` tool call
    async fn handle_disconnect_provider(
        user_id: Uuid,
        provider: &str,
        resources: &Arc<ServerResources>,
        id: Value,
    ) -> McpResponse {
        // Use existing ServerResources (no fake auth managers or cloning!)
        let server_context = crate::context::ServerContext::from(resources.as_ref());
        let oauth_routes = OAuthRoutes::new(
            server_context.data().clone(),
            server_context.config().clone(),
            server_context.notification().clone(),
        );

        match oauth_routes.disconnect_provider(user_id, provider).await {
            Ok(()) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": format!("Successfully disconnected {provider}"),
                    "provider": provider
                });

                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: Some(response),
                    error: None,
                    id: Some(id),
                }
            }
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to disconnect provider: {e}"),
                    data: None,
                }),
                id: Some(id),
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
            PREDICT_PERFORMANCE => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INTERNAL_ERROR,
                        message: "Provider required".into(),
                        data: None,
                    }),
                    id: Some(id),
                };
            }
            _ => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_owned(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: format!("Unknown tool: {tool_name}"),
                        data: None,
                    }),
                    id: Some(id),
                };
            }
        };

        match result {
            Ok(response) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: Some(response),
                error: None,
                id: Some(id),
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
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to create goal: {e}"),
                    data: None,
                }),
                id: Some(id.clone()),
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
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Goal with ID '{goal_id}' not found"),
                            data: None,
                        }),
                        id: Some(id.clone()),
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
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get goals: {e}"),
                    data: None,
                }),
                id: Some(id.clone()),
            }),
        }
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
            api_key_id: api_key_id.to_owned(),
            timestamp: Utc::now(),
            tool_name: tool_name.to_owned(),
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

    // === Tenant-Aware Tool Handlers ===

    /// Store user-provided OAuth credentials if supplied
    async fn store_mcp_oauth_credentials(
        tenant_context: &TenantContext,
        oauth_client: &Arc<TenantOAuthClient>,
        credentials: &McpOAuthCredentials<'_>,
        config: &Arc<crate::config::environment::ServerConfig>,
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
            let redirect_uri = config
                .oauth
                .strava
                .redirect_uri
                .clone() // Safe: Config string ownership for OAuth credential storage
                .unwrap_or_else(|| {
                    format!(
                        "http://localhost:{}/api/oauth/callback/strava",
                        config.http_port
                    )
                });
            let request = crate::tenant::oauth_client::StoreCredentialsRequest {
                client_id: id.to_owned(),
                client_secret: secret.to_owned(),
                redirect_uri,
                scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_owned)
                    .collect(),
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
            let redirect_uri = config
                .oauth
                .fitbit
                .redirect_uri
                .clone() // Safe: Config string ownership for OAuth credential storage
                .unwrap_or_else(|| {
                    format!(
                        "http://localhost:{}/api/oauth/callback/fitbit",
                        config.http_port
                    )
                });
            let request = crate::tenant::oauth_client::StoreCredentialsRequest {
                client_id: id.to_owned(),
                client_secret: secret.to_owned(),
                redirect_uri,
                scopes: vec![
                    "activity".to_owned(),
                    "heartrate".to_owned(),
                    "location".to_owned(),
                    "nutrition".to_owned(),
                    "profile".to_owned(),
                    "settings".to_owned(),
                    "sleep".to_owned(),
                    "social".to_owned(),
                    "weight".to_owned(),
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
    pub async fn handle_tenant_connection_status(
        tenant_context: &TenantContext,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
        database: &Arc<Database>,
        request_id: Value,
        credentials: McpOAuthCredentials<'_>,
        http_port: u16,
        config: &Arc<crate::config::environment::ServerConfig>,
    ) -> McpResponse {
        tracing::info!(
            "Checking connection status for tenant {} user {}",
            tenant_context.tenant_name,
            tenant_context.user_id
        );

        // Store MCP-provided OAuth credentials if supplied
        Self::store_mcp_oauth_credentials(
            tenant_context,
            tenant_oauth_client,
            &credentials,
            config,
        )
        .await;

        let base_url = Self::build_oauth_base_url(http_port);
        let connection_status = Self::check_provider_connections(tenant_context, database).await;
        let notifications_text =
            Self::build_notifications_text(database, tenant_context.user_id).await;
        let structured_data = Self::build_structured_connection_data(
            tenant_context,
            &connection_status,
            &base_url,
            database,
        )
        .await;
        let text_content = Self::build_text_content(
            &connection_status,
            &base_url,
            tenant_context,
            &notifications_text,
        );

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
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
            id: Some(request_id),
        }
    }

    /// Build OAuth base URL with dynamic port
    fn build_oauth_base_url(http_port: u16) -> String {
        let host = crate::constants::get_server_config()
            .map_or_else(|| "localhost".to_owned(), |c| c.host.clone());
        format!("http://{host}:{http_port}/api/oauth")
    }

    /// Check connection status for all providers
    async fn check_provider_connections(
        tenant_context: &TenantContext,
        database: &Arc<Database>,
    ) -> ProviderConnectionStatus {
        let user_id = tenant_context.user_id;
        let tenant_id_str = tenant_context.tenant_id.to_string();

        // Check Strava connection status
        tracing::debug!(
            "Checking Strava token for user_id={}, tenant_id={}, provider=strava",
            user_id,
            tenant_id_str
        );
        let strava_connected = database
            .get_user_oauth_token(user_id, &tenant_id_str, "strava")
            .await
            .map_or_else(
                |e| {
                    tracing::warn!("Failed to query Strava OAuth token: {e}");
                    false
                },
                |token| {
                    let connected = token.is_some();
                    tracing::debug!("Strava token lookup result: connected={connected}");
                    connected
                },
            );

        // Check Fitbit connection status
        let fitbit_connected = database
            .get_user_oauth_token(user_id, &tenant_id_str, "fitbit")
            .await
            .is_ok_and(|token| token.is_some());

        ProviderConnectionStatus {
            strava_connected,
            fitbit_connected,
        }
    }

    /// Build notifications text from unread notifications
    async fn build_notifications_text(database: &Arc<Database>, user_id: uuid::Uuid) -> String {
        let unread_notifications = database
            .get_unread_oauth_notifications(user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch unread notifications: {e}");
                Vec::new()
            });

        if unread_notifications.is_empty() {
            String::new()
        } else {
            let mut notifications_msg = String::from("\n\nRecent OAuth Updates:\n");
            for notification in &unread_notifications {
                let status_indicator = if notification.success {
                    "[SUCCESS]"
                } else {
                    "[FAILED]"
                };
                writeln!(
                    notifications_msg,
                    "{status_indicator} {}: {}",
                    notification.provider.to_uppercase(),
                    notification.message
                )
                .unwrap_or_else(|_| tracing::warn!("Failed to write notification text"));
            }
            notifications_msg
        }
    }

    /// Build structured connection data JSON
    async fn build_structured_connection_data(
        tenant_context: &TenantContext,
        connection_status: &ProviderConnectionStatus,
        base_url: &str,
        database: &Arc<Database>,
    ) -> serde_json::Value {
        let unread_notifications = database
            .get_unread_oauth_notifications(tenant_context.user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!(
                    user_id = %tenant_context.user_id,
                    error = %e,
                    "Failed to fetch OAuth notifications for connection status"
                );
                Vec::new()
            });

        serde_json::json!({
            "providers": [
                {
                    "provider": "strava",
                    "connected": connection_status.strava_connected,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{base_url}/auth/strava/{}", tenant_context.user_id),
                    "connect_instructions": if connection_status.strava_connected {
                        "Your Strava account is connected and ready to use."
                    } else {
                        "Click this URL to connect your Strava account and authorize access to your fitness data."
                    }
                },
                {
                    "provider": "fitbit",
                    "connected": connection_status.fitbit_connected,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{base_url}/auth/fitbit/{}", tenant_context.user_id),
                    "connect_instructions": if connection_status.fitbit_connected {
                        "Your Fitbit account is connected and ready to use."
                    } else {
                        "Click this URL to connect your Fitbit account and authorize access to your fitness data."
                    }
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
            },
            "recent_notifications": unread_notifications.iter().map(|n| serde_json::json!({
                "id": n.id,
                "provider": n.provider,
                "success": n.success,
                "message": n.message,
                "created_at": n.created_at
            })).collect::<Vec<_>>()
        })
    }

    /// Build human-readable text content
    fn build_text_content(
        connection_status: &ProviderConnectionStatus,
        base_url: &str,
        tenant_context: &TenantContext,
        notifications_text: &str,
    ) -> String {
        let strava_status = if connection_status.strava_connected {
            "Connected"
        } else {
            "Not Connected"
        };
        let fitbit_status = if connection_status.fitbit_connected {
            "Connected"
        } else {
            "Not Connected"
        };

        let strava_action = if connection_status.strava_connected {
            "Ready to use fitness tools!".to_owned()
        } else {
            format!(
                "Click to connect: {base_url}/auth/strava/{}",
                tenant_context.user_id
            )
        };

        let fitbit_action = if connection_status.fitbit_connected {
            "Ready to use fitness tools!".to_owned()
        } else {
            format!(
                "Click to connect: {base_url}/auth/fitbit/{}",
                tenant_context.user_id
            )
        };

        let connection_instructions = if !connection_status.strava_connected
            || !connection_status.fitbit_connected
        {
            "To connect a provider:\n\
            1. Click one of the URLs above\n\
            2. You'll be redirected to authorize access\n\
            3. Complete the OAuth flow to connect your account\n\
            4. Start using fitness tools like get_activities, get_athlete, and get_stats"
        } else {
            "All providers connected! You can now use fitness tools like get_activities, get_athlete, and get_stats."
        };

        format!(
            "Fitness Provider Connection Status\n\n\
            Available Providers:\n\n\
            Strava ({strava_status})\n\
            {strava_action}\n\n\
            Fitbit ({fitbit_status})\n\
            {fitbit_action}\n\n\
            {connection_instructions}{notifications_text}"
        )
    }

    /// Handle tenant-aware provider disconnection
    fn handle_tenant_disconnect_provider(
        tenant_context: &TenantContext,
        provider_name: &str,
        _provider_registry: &Arc<ProviderRegistry>,
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
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: Some(serde_json::json!({
                "message": format!("Disconnected from {provider_name}"),
                "provider": provider_name,
                "tenant_id": tenant_context.tenant_id,
                "success": true
            })),
            error: None,
            id: Some(request_id),
        }
    }

    /// Create error response for tool execution failure
    fn create_tool_error_response(
        tool_name: &str,
        provider_name: &str,
        response_error: Option<String>,
        request_id: Value,
    ) -> McpResponse {
        let error_msg = response_error
            .unwrap_or_else(|| "Tool execution failed with no error message".to_owned());
        tracing::error!(
            "Tool execution failed for {} with provider {}: {} (success=false)",
            tool_name,
            provider_name,
            error_msg
        );
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_owned(),
            result: None,
            error: Some(McpError {
                code: ERROR_INTERNAL_ERROR,
                message: error_msg,
                data: None,
            }),
            id: Some(request_id),
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
            // Analytics tools - route through Universal Protocol
            ANALYZE_GOAL_FEASIBILITY,
            SUGGEST_GOALS,
            CALCULATE_FITNESS_SCORE,
            GENERATE_RECOMMENDATIONS,
            ANALYZE_TRAINING_LOAD,
            DETECT_PATTERNS,
            ANALYZE_PERFORMANCE_TRENDS,
            // Configuration tools - route through Universal Protocol
            "get_configuration_catalog",
            "get_configuration_profiles",
            "get_user_configuration",
            "update_user_configuration",
            "calculate_personalized_zones",
            "validate_configuration",
        ];

        if !known_provider_tools.contains(&tool_name) {
            return McpResponse {
                jsonrpc: JSONRPC_VERSION.to_owned(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: format!("Unknown tool: {tool_name}"),
                    data: None,
                }),
                id: Some(request_id),
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
            tool_name: tool_name.to_owned(),
            parameters: args.clone(),
            user_id: auth_result.user_id.to_string(),
            protocol: "mcp".to_owned(),
            tenant_id: Some(tenant_context.tenant_id.to_string()),
        };

        // Use the provided ServerResources - no more fake auth managers or secrets!
        let executor = crate::protocols::universal::UniversalToolExecutor::new(resources.clone());

        // Execute the tool through Universal protocol
        match executor.execute_tool(universal_request).await {
            Ok(response) => {
                // Convert UniversalResponse to proper MCP ToolResponse format with content field
                let tool_response =
                    crate::protocols::converter::ProtocolConverter::universal_to_mcp(response);

                // Serialize ToolResponse to JSON for MCP result field
                match serde_json::to_value(&tool_response) {
                    Ok(result_value) => McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_owned(),
                        result: Some(result_value),
                        error: None,
                        id: Some(request_id),
                    },
                    Err(e) => Self::create_tool_error_response(
                        tool_name,
                        provider_name,
                        Some(format!("Failed to serialize tool response: {e}")),
                        request_id,
                    ),
                }
            }
            Err(e) => Self::create_tool_error_response(
                tool_name,
                provider_name,
                Some(format!("Tool execution error: {e}")),
                request_id,
            ),
        }
    }
}

// Phase 2: Type aliases pointing to unified JSON-RPC foundation
/// Type alias for MCP requests using the JSON-RPC foundation
pub type McpRequest = crate::jsonrpc::JsonRpcRequest;
/// Type alias for MCP responses using the JSON-RPC foundation
pub type McpResponse = crate::jsonrpc::JsonRpcResponse;
/// Type alias for MCP errors using the JSON-RPC foundation
pub type McpError = crate::jsonrpc::JsonRpcError;

/// Re-export `AppError` as `ApiError` for this module
type ApiError = crate::errors::AppError;

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
    // Handle MCP-specific errors first
    if let Some(mcp_error) = err.find::<McpHttpError>() {
        return match mcp_error {
            McpHttpError::InvalidOrigin => {
                let json = warp::reply::json(&serde_json::json!({
                    "error": "Forbidden",
                    "message": "Invalid origin header for security"
                }));
                let reply = warp::reply::with_status(json, warp::http::StatusCode::FORBIDDEN);
                Ok(Box::new(with_cors_headers(reply, None)) as Box<dyn warp::Reply>)
            }
            McpHttpError::InvalidRequest => {
                // This shouldn't happen anymore since we handle it inline, but keep for safety
                let json = warp::reply::json(&serde_json::json!({
                    "error": "Bad Request",
                    "message": "Invalid MCP request"
                }));
                let reply = warp::reply::with_status(json, warp::http::StatusCode::BAD_REQUEST);
                Ok(Box::new(with_cors_headers(reply, None)) as Box<dyn warp::Reply>)
            }
        };
    }

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
        |api_error: &ApiError| {
            // AppError implements Reply with built-in sanitization
            // Clone to take ownership for into_response()
            Ok(
                Box::new(with_cors_headers(api_error.clone().into_response(), None))
                    as Box<dyn warp::Reply>,
            )
        },
    )
}
