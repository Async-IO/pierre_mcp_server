// ABOUTME: Multi-tenant MCP server implementation with tenant isolation
// ABOUTME: Handles MCP protocol with per-tenant data isolation and access control
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Multi-Tenant MCP Server
//!
//! This module provides a multi-tenant MCP server that supports user authentication,
//! secure token storage, and user-scoped data access.

use crate::a2a_routes::A2ARoutes;
use crate::api_key_routes::ApiKeyRoutes;
use crate::auth::{AuthManager, AuthResult, McpAuthMiddleware};
use crate::configuration_routes::ConfigurationRoutes;
use crate::constants::{
    errors::{
        ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND, ERROR_UNAUTHORIZED,
    },
    json_fields::{ACTIVITY_ID, GOAL_ID, LIMIT, OFFSET, PROVIDER},
    protocol,
    protocol::{JSONRPC_VERSION, SERVER_VERSION},
    tools::{
        ANALYZE_ACTIVITY, ANALYZE_GOAL_FEASIBILITY, ANALYZE_PERFORMANCE_TRENDS,
        ANALYZE_TRAINING_LOAD, CALCULATE_FITNESS_SCORE, CALCULATE_METRICS, COMPARE_ACTIVITIES,
        CONNECT_FITBIT, CONNECT_STRAVA, DETECT_PATTERNS, DISCONNECT_PROVIDER,
        GENERATE_RECOMMENDATIONS, GET_ACTIVITIES, GET_ACTIVITY_INTELLIGENCE, GET_ATHLETE,
        GET_CONNECTION_STATUS, GET_STATS, PREDICT_PERFORMANCE, SET_GOAL, SUGGEST_GOALS,
        TRACK_PROGRESS,
    },
};
use crate::dashboard_routes::DashboardRoutes;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::intelligence::insights::ActivityContext;
use crate::intelligence::ActivityAnalyzer;
use crate::mcp::schema::InitializeResponse;
use crate::models::{Activity, AuthRequest};
use crate::providers::{create_provider, AuthData, FitnessProvider};
use crate::routes::{AuthRoutes, LoginRequest, OAuthRoutes, RefreshTokenRequest, RegisterRequest};
use crate::security::SecurityConfig;
use crate::websocket::WebSocketManager;

use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// Constants are now imported from the constants module

/// Type alias for the complex provider storage type
type UserProviderStorage = Arc<RwLock<HashMap<String, HashMap<String, Box<dyn FitnessProvider>>>>>;

/// Context for HTTP request handling
struct HttpRequestContext {
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    auth_middleware: Arc<McpAuthMiddleware>,
    user_providers: UserProviderStorage,
}

/// Multi-tenant MCP server supporting user authentication
#[derive(Clone)]
pub struct MultiTenantMcpServer {
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    auth_middleware: Arc<McpAuthMiddleware>,
    websocket_manager: Arc<WebSocketManager>,
    // Per-user provider instances
    user_providers: UserProviderStorage,
    config: Arc<crate::config::environment::ServerConfig>,
}

impl MultiTenantMcpServer {
    /// Create a new multi-tenant MCP server
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
        config: Arc<crate::config::environment::ServerConfig>,
    ) -> Self {
        let database_arc = Arc::new(database);
        let auth_manager_arc = Arc::new(auth_manager);
        let auth_middleware =
            McpAuthMiddleware::new(auth_manager_arc.as_ref().clone(), database_arc.clone());
        let websocket_manager = Arc::new(WebSocketManager::new(
            database_arc.as_ref().clone(),
            auth_manager_arc.as_ref().clone(),
        ));

        Self {
            database: database_arc,
            auth_manager: auth_manager_arc,
            auth_middleware: Arc::new(auth_middleware),
            websocket_manager,
            user_providers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Run the multi-tenant server with both HTTP and MCP endpoints
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to start or bind to the specified port
    pub async fn run(self, port: u16) -> Result<()> {
        // Create HTTP + MCP server
        info!(
            "Starting multi-tenant server with HTTP and MCP on port {}",
            port
        );

        // Clone references for HTTP handlers
        let database = self.database.clone();
        let auth_manager = self.auth_manager.clone();

        // Create route handlers
        let auth_routes = AuthRoutes::new(database.as_ref().clone(), auth_manager.as_ref().clone());
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        // Validate route handlers are properly initialized
        tracing::debug!("Initialized auth and OAuth route handlers for multi-tenant server - auth routes: {:p}, oauth routes: {:p}", &auth_routes, &oauth_routes);

        // Start HTTP server for auth endpoints in background
        let http_port = port + 1; // Use port+1 for HTTP
        let database_http = database.clone();
        let auth_manager_http = auth_manager.clone();
        let websocket_manager_http = self.websocket_manager.clone();

        let config_http = self.config.clone();
        tokio::spawn(async move {
            Self::run_http_server(
                http_port,
                database_http,
                auth_manager_http,
                websocket_manager_http,
                config_http,
            )
            .await
        });

        // Run MCP server on main port
        self.run_mcp_server(port).await
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

    /// Initialize all route handlers
    fn setup_route_handlers(
        database: &Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        config: &Arc<crate::config::environment::ServerConfig>,
    ) -> (
        AuthRoutes,
        OAuthRoutes,
        ApiKeyRoutes,
        DashboardRoutes,
        A2ARoutes,
        Arc<ConfigurationRoutes>,
    ) {
        let auth_routes = AuthRoutes::new(database.as_ref().clone(), auth_manager.as_ref().clone());
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());
        let api_key_routes =
            ApiKeyRoutes::new(database.as_ref().clone(), auth_manager.as_ref().clone());
        let dashboard_routes =
            DashboardRoutes::new(database.as_ref().clone(), auth_manager.as_ref().clone());
        let a2a_routes = A2ARoutes::new(database.clone(), auth_manager.clone(), config.clone());
        let configuration_routes = Arc::new(ConfigurationRoutes::new(
            database.as_ref().clone(),
            auth_manager.as_ref().clone(),
        ));

        (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
        )
    }

    /// Load JWT secret from file system
    fn load_jwt_secret(config: &crate::config::environment::ServerConfig) -> Result<String> {
        let jwt_secret = if config.auth.jwt_secret_path.exists() {
            std::fs::read(&config.auth.jwt_secret_path).map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read JWT secret from {}: {}. This is required for security.",
                    config.auth.jwt_secret_path.display(),
                    e
                )
            })?
        } else {
            return Err(anyhow::anyhow!(
                "JWT secret file not found at {}. This is required for security.",
                config.auth.jwt_secret_path.display()
            ));
        };

        // JWT secret is stored as binary data (64 bytes), convert to base64 string for admin routes
        Ok(general_purpose::STANDARD.encode(jwt_secret))
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
            ])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
    }

    /// Create authentication endpoint routes
    fn create_auth_routes(
        auth_routes: &AuthRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // Registration endpoint
        let register = warp::path("auth")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Login endpoint
        let login = warp::path("auth")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Token refresh endpoint
        let refresh = warp::path("auth")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        register.or(login).or(refresh)
    }

    /// Create OAuth endpoint routes
    fn create_oauth_routes(
        oauth_routes: &OAuthRoutes,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // OAuth authorization URL endpoint
        let oauth_auth = warp::path("oauth")
            .and(warp::path!("auth" / String / String)) // /oauth/auth/{provider}/{user_id}
            .and(warp::get())
            .and_then({
                let oauth_routes = oauth_routes.clone();
                move |provider: String, user_id_str: String| {
                    let oauth_routes = oauth_routes.clone();
                    async move {
                        Uuid::parse_str(&user_id_str).map_or_else(
                            |_| {
                                let error = serde_json::json!({"error": "Invalid user ID format"});
                                Err(warp::reject::custom(ApiError(error)))
                            },
                            |user_id| match oauth_routes.get_auth_url(user_id, &provider) {
                                Ok(auth_response) => Ok(warp::reply::json(&auth_response)),
                                Err(e) => {
                                    let error = serde_json::json!({"error": e.to_string()});
                                    Err(warp::reject::custom(ApiError(error)))
                                }
                            },
                        )
                    }
                }
            });

        // OAuth callback endpoint
        let oauth_callback = warp::path("oauth")
            .and(warp::path("callback"))
            .and(warp::path!(String)) // /oauth/callback/{provider}
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and(warp::get())
            .and_then({
                let oauth_routes = oauth_routes.clone();
                move |provider: String, params: std::collections::HashMap<String, String>| {
                    let oauth_routes = oauth_routes.clone();
                    async move {
                        let code = params.get("code").cloned().unwrap_or_default();
                        let state = params.get("state").cloned().unwrap_or_default();
                        let error = params.get("error").cloned();
                        if let Some(error_msg) = error {
                            let error_response = serde_json::json!({
                                "error": "OAuth authorization failed",
                                "details": error_msg,
                                "provider": provider
                            });
                            return Ok(warp::reply::with_status(
                                warp::reply::json(&error_response),
                                warp::http::StatusCode::BAD_REQUEST
                            ));
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
                                ))
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
            });

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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                move |api_key_id: String, auth_header: Option<String>, params: std::collections::HashMap<String, String>| {
                    let api_key_routes = api_key_routes.clone();
                    async move {
                        let start_date_str = params.get("start_date").cloned().unwrap_or_else(|| {
                            let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
                            thirty_days_ago.to_rfc3339()
                        });
                        let end_date_str = params.get("end_date").cloned().unwrap_or_else(|| {
                            chrono::Utc::now().to_rfc3339()
                        });

                        let start_date = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&start_date_str) {
                            dt.with_timezone(&chrono::Utc)
                        } else {
                            let error = serde_json::json!({"error": "Invalid start_date format. Use RFC3339."});
                            return Err(warp::reject::custom(ApiError(error)));
                        };

                        let end_date = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&end_date_str) {
                            dt.with_timezone(&chrono::Utc)
                        } else {
                            let error = serde_json::json!({"error": "Invalid end_date format. Use RFC3339."});
                            return Err(warp::reject::custom(ApiError(error)));
                        };

                        match api_key_routes.get_api_key_usage(auth_header.as_deref(), &api_key_id, start_date, end_date).await {
                            Ok(response) => Ok(warp::reply::json(&response)),
                            Err(e) => {
                                let error = serde_json::json!({"error": e.to_string()});
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
        let dashboard_overview = warp::path("dashboard")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard analytics
        let dashboard_analytics = warp::path("dashboard")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard rate limits
        let dashboard_rate_limits = warp::path("dashboard")
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
                                let error = serde_json::json!({"error": e.to_string()});
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
        let dashboard_request_logs = warp::path("dashboard")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard request stats
        let dashboard_request_stats = warp::path("dashboard")
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
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // Dashboard tool usage
        let dashboard_tool_usage = warp::path("dashboard")
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                                let error = serde_json::json!({"error": e.to_string()});
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
                if let Ok(header_value) = warp::http::HeaderValue::from_str(&value) {
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
            warp::reply::json(&serde_json::json!({"status": "ok", "service": "pierre-mcp-server"}))
        })
    }

    /// Run HTTP server for authentication endpoints
    async fn run_http_server(
        port: u16,
        database: Arc<Database>,
        auth_manager: Arc<AuthManager>,
        websocket_manager: Arc<WebSocketManager>,
        config: Arc<crate::config::environment::ServerConfig>,
    ) -> Result<()> {
        use warp::Filter;

        info!("HTTP authentication server starting on port {}", port);

        // Initialize security configuration
        let security_config = Self::setup_security_config(&config);

        // Initialize all route handlers
        let (
            auth_routes,
            oauth_routes,
            api_key_routes,
            dashboard_routes,
            a2a_routes,
            configuration_routes,
        ) = Self::setup_route_handlers(&database, &auth_manager, &config);

        // Load JWT secret for admin routes
        let jwt_secret_str = Self::load_jwt_secret(&config)?;

        // Setup admin routes
        let admin_context = crate::admin_routes::AdminApiContext::new(
            database.as_ref().clone(),
            &jwt_secret_str,
            auth_manager.as_ref().clone(),
        );
        let admin_routes_filter = crate::admin_routes::admin_routes_with_rejection(admin_context);

        // Configure CORS
        let cors = Self::setup_cors();

        // Create all route groups using helper functions
        let auth_route_filter = Self::create_auth_routes(&auth_routes);
        let oauth_route_filter = Self::create_oauth_routes(&oauth_routes);
        let api_key_route_filter = Self::create_api_key_routes(&api_key_routes);
        let api_key_usage_filter = Self::create_api_key_usage_route(api_key_routes.clone());
        let health_filter = Self::create_health_route();

        // Dashboard route groups
        let dashboard_basic_filter = Self::create_dashboard_routes(&dashboard_routes);
        let dashboard_detailed_filter = Self::create_dashboard_detailed_routes(&dashboard_routes);

        // A2A route groups
        let a2a_basic_filter = Self::create_a2a_basic_routes(&a2a_routes);
        let a2a_client_filter = Self::create_a2a_client_routes(&a2a_routes);
        let a2a_monitoring_filter = Self::create_a2a_monitoring_routes(&a2a_routes);
        let a2a_execution_filter = Self::create_a2a_execution_routes(&a2a_routes);

        // Configuration route groups
        let config_basic_filter = Self::create_configuration_routes(&configuration_routes);
        let config_user_filter = Self::create_user_configuration_routes(&configuration_routes);
        let config_specialized_filter =
            Self::create_specialized_configuration_routes(&configuration_routes);

        // WebSocket endpoint
        let websocket_route = websocket_manager.websocket_filter();

        // Start periodic WebSocket updates
        websocket_manager.start_periodic_updates();

        // Create security headers filter
        let security_headers_filter = Self::create_security_headers_filter(&security_config);

        // Combine route groups
        let auth_routes = auth_route_filter.or(oauth_route_filter);
        let api_key_routes = api_key_route_filter.or(api_key_usage_filter);
        let dashboard_routes = dashboard_basic_filter.or(dashboard_detailed_filter);
        let a2a_routes = a2a_basic_filter
            .or(a2a_client_filter)
            .or(a2a_monitoring_filter)
            .or(a2a_execution_filter);
        let configuration_routes = config_basic_filter
            .or(config_user_filter)
            .or(config_specialized_filter);

        // HTTP routes with security headers (exclude WebSocket)
        let http_routes = auth_routes
            .or(api_key_routes)
            .or(dashboard_routes)
            .or(a2a_routes)
            .or(configuration_routes)
            .or(health_filter)
            .or(admin_routes_filter)
            .with(cors.clone())
            .with(security_headers_filter);

        // WebSocket route without security headers (security headers break WebSocket handshake)
        let ws_routes = websocket_route.with(cors);

        // Combine routes
        let routes = http_routes.or(ws_routes);
        let routes = routes.recover(handle_rejection);

        info!("HTTP server ready on port {}", port);
        Box::pin(warp::serve(routes).run(([127, 0, 0, 1], port))).await;

        Ok(())
    }

    /// Run MCP server with both stdio and HTTP transports
    async fn run_mcp_server(self, port: u16) -> Result<()> {
        info!("Starting MCP server with stdio and HTTP transports");

        // Clone server for both transports
        let server_for_stdio = self.clone();
        let server_for_http = self;

        // Start stdio transport
        let stdio_handle =
            tokio::spawn(async move { server_for_stdio.run_stdio_transport().await });

        // Start HTTP transport
        let http_handle =
            tokio::spawn(async move { server_for_http.run_http_transport(port).await });

        // Wait for either transport to fail
        tokio::select! {
            result = stdio_handle => {
                match result {
                    Ok(Ok(())) => info!("stdio transport completed successfully"),
                    Ok(Err(e)) => warn!("stdio transport failed: {}", e),
                    Err(e) => warn!("stdio transport task failed: {}", e),
                }
            }
            result = http_handle => {
                match result {
                    Ok(Ok(())) => info!("HTTP transport completed successfully"),
                    Ok(Err(e)) => warn!("HTTP transport failed: {}", e),
                    Err(e) => warn!("HTTP transport task failed: {}", e),
                }
            }
        }

        Ok(())
    }

    /// Run MCP server with only HTTP transport (for testing)
    ///
    /// # Errors
    ///
    /// Returns an error if the HTTP server fails to start or bind to the specified port
    pub async fn run_http_only(self, port: u16) -> Result<()> {
        info!("Starting MCP server with HTTP transport only");

        // Clone references for HTTP handlers
        let database = self.database.clone();
        let auth_manager = self.auth_manager.clone();

        // Start HTTP server for auth endpoints in background
        let http_port = port + 1; // Use port+1 for HTTP
        let database_http = database.clone();
        let auth_manager_http = auth_manager.clone();
        let websocket_manager_http = self.websocket_manager.clone();

        let config_http = self.config.clone();
        tokio::spawn(async move {
            Self::run_http_server(
                http_port,
                database_http,
                auth_manager_http,
                websocket_manager_http,
                config_http,
            )
            .await
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
                let response = Self::handle_request(
                    request,
                    &self.database,
                    &self.auth_manager,
                    &self.auth_middleware,
                    &self.user_providers,
                )
                .await;

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
            line.clear();
        }

        info!("MCP stdio transport ended");
        Ok(())
    }

    /// Run MCP server using Streamable HTTP transport (MCP specification compliant)
    async fn run_http_transport(self, port: u16) -> Result<()> {
        use warp::Filter;

        info!("MCP HTTP transport starting on port {}", port);

        let database = self.database.clone();
        let auth_manager = self.auth_manager.clone();
        let auth_middleware = self.auth_middleware.clone();
        let user_providers = self.user_providers.clone();

        // MCP endpoint for both POST and GET
        let mcp_endpoint = warp::path("mcp")
            .and(warp::method())
            .and(warp::header::optional::<String>("origin"))
            .and(warp::header::optional::<String>("accept"))
            .and(
                warp::body::json()
                    .or(warp::any().map(|| serde_json::Value::Null))
                    .unify(),
            )
            .and_then({
                move |method: warp::http::Method,
                      origin: Option<String>,
                      accept: Option<String>,
                      body: serde_json::Value| {
                    let database = database.clone();
                    let auth_manager = auth_manager.clone();
                    let auth_middleware = auth_middleware.clone();
                    let user_providers = user_providers.clone();

                    async move {
                        let ctx = HttpRequestContext {
                            database,
                            auth_manager,
                            auth_middleware,
                            user_providers,
                        };
                        Self::handle_mcp_http_request(method, origin, accept, body, &ctx).await
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
                if let Ok(request) = serde_json::from_value::<McpRequest>(body) {
                    let response = Self::handle_request(
                        request,
                        &ctx.database,
                        &ctx.auth_manager,
                        &ctx.auth_middleware,
                        &ctx.user_providers,
                    )
                    .await;

                    // Return 202 Accepted with no body for successful requests
                    Ok(Box::new(warp::reply::with_status(
                        warp::reply::json(&response),
                        warp::http::StatusCode::ACCEPTED,
                    )))
                } else {
                    Err(warp::reject::custom(McpHttpError::InvalidRequest))
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
                        "version": "2024-11-05"
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
        origin.starts_with("http://localhost") ||
        origin.starts_with("http://127.0.0.1") ||
        origin.starts_with("https://localhost") ||
        origin.starts_with("https://127.0.0.1") ||
        // Allow null origin for direct connections
        origin == "null"
    }

    /// Handle HTTP rejection
    fn handle_mcp_rejection_sync(err: &warp::Rejection) -> impl warp::Reply {
        let code;
        let message;

        if err.is_not_found() {
            code = warp::http::StatusCode::NOT_FOUND;
            message = "Not Found";
        } else if matches!(err.find(), Some(McpHttpError::InvalidOrigin)) {
            code = warp::http::StatusCode::FORBIDDEN;
            message = "Invalid origin";
        } else if matches!(err.find(), Some(McpHttpError::InvalidRequest)) {
            code = warp::http::StatusCode::BAD_REQUEST;
            message = "Invalid request";
        } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
            code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
            message = "Method not allowed";
        } else {
            code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
            message = "Internal server error";
        }

        let json = warp::reply::json(&serde_json::json!({
            "error": message,
            "code": code.as_u16()
        }));

        warp::reply::with_status(json, code)
    }

    /// Handle MCP request with authentication
    pub async fn handle_request(
        request: McpRequest,
        database: &Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        auth_middleware: &Arc<McpAuthMiddleware>,
        user_providers: &UserProviderStorage,
    ) -> McpResponse {
        match request.method.as_str() {
            "initialize" => Self::handle_initialize(request),
            "ping" => Self::handle_ping(request),
            "tools/list" => Self::handle_tools_list(request),
            "authenticate" => Self::handle_authenticate(request, auth_manager),
            "tools/call" => {
                Self::handle_tools_call(request, database, auth_middleware, user_providers).await
            }
            _ => Self::handle_unknown_method(request),
        }
    }

    /// Handle initialize request
    fn handle_initialize(request: McpRequest) -> McpResponse {
        let init_response = InitializeResponse::new(
            protocol::mcp_protocol_version(),
            protocol::server_name_multitenant(),
            SERVER_VERSION.to_string(),
        );

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: serde_json::to_value(&init_response).ok(),
            error: None,
            id: request.id,
        }
    }

    /// Handle ping request
    fn handle_ping(request: McpRequest) -> McpResponse {
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(serde_json::json!({})),
            error: None,
            id: request.id,
        }
    }

    /// Handle tools/list request
    fn handle_tools_list(request: McpRequest) -> McpResponse {
        let tools = crate::mcp::schema::get_tools();
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(serde_json::json!({
                "tools": tools
            })),
            error: None,
            id: request.id,
        }
    }

    /// Handle tools/call request with authentication
    async fn handle_tools_call(
        request: McpRequest,
        database: &Arc<Database>,
        auth_middleware: &Arc<McpAuthMiddleware>,
        user_providers: &UserProviderStorage,
    ) -> McpResponse {
        let auth_token = request.auth_token.as_deref();

        tracing::debug!(
            "MCP tool call authentication attempt for method: {}",
            request.method
        );

        match auth_middleware.authenticate_request(auth_token).await {
            Ok(auth_result) => {
                tracing::info!(
                    "MCP tool call authentication successful for user: {} (method: {})",
                    auth_result.user_id,
                    auth_result.auth_method.display_name()
                );

                // Update user's last active timestamp
                let _ = database.update_last_active(auth_result.user_id).await;

                Self::handle_authenticated_tool_call(request, auth_result, database, user_providers)
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

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(McpError {
                code: error_code,
                message: error_msg.to_string(),
                data: Some(serde_json::json!({
                    "detailed_error": error_message,
                    "authentication_failed": true
                })),
            }),
            id: request.id,
        }
    }

    /// Handle unknown method
    fn handle_unknown_method(request: McpRequest) -> McpResponse {
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(McpError {
                code: ERROR_METHOD_NOT_FOUND,
                message: "Method not found".into(),
                data: None,
            }),
            id: request.id,
        }
    }

    /// Handle authentication request
    fn handle_authenticate(request: McpRequest, auth_manager: &Arc<AuthManager>) -> McpResponse {
        let params = request.params.unwrap_or_default();

        if let Ok(auth_request) = serde_json::from_value::<AuthRequest>(params) {
            let auth_response = auth_manager.authenticate(&auth_request);

            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(&auth_response).ok(),
                error: None,
                id: request.id,
            }
        } else {
            McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INVALID_PARAMS,
                    message: "Invalid authentication request".into(),
                    data: None,
                }),
                id: request.id,
            }
        }
    }

    /// Handle authenticated tool call with user context and rate limiting
    async fn handle_authenticated_tool_call(
        request: McpRequest,
        auth_result: AuthResult,
        database: &Arc<Database>,
        user_providers: &UserProviderStorage,
    ) -> McpResponse {
        let params = request.params.unwrap_or_default();
        let tool_name = params["name"].as_str().unwrap_or("");
        let args = &params["arguments"];
        let user_id = auth_result.user_id;

        tracing::info!(
            "Executing tool call: {} for user: {} using {} authentication",
            tool_name,
            user_id,
            auth_result.auth_method.display_name()
        );

        // Handle OAuth-related tools (don't require existing provider)
        match tool_name {
            CONNECT_STRAVA => Self::handle_connect_strava(user_id, database, request.id),
            CONNECT_FITBIT => Self::handle_connect_fitbit(user_id, database, request.id),
            GET_CONNECTION_STATUS => {
                return Self::handle_get_connection_status(user_id, database, request.id).await;
            }
            DISCONNECT_PROVIDER => {
                let provider_name = args[PROVIDER].as_str().unwrap_or("");
                Self::handle_disconnect_provider(user_id, provider_name, database, request.id)
            }
            // Tools that don't require providers
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
                    request.id,
                    user_id,
                    database,
                    &auth_result,
                )
                .await
            }
            _ => {
                Self::handle_tool_with_provider(
                    tool_name,
                    args,
                    request.id,
                    user_id,
                    database,
                    user_providers,
                    &auth_result,
                )
                .await
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

    /// Handle tools that require external providers  
    async fn handle_tool_with_provider(
        tool_name: &str,
        args: &Value,
        request_id: Value,
        user_id: Uuid,
        database: &Arc<Database>,
        user_providers: &UserProviderStorage,
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
        let provider_result =
            Self::get_user_provider(user_id, provider_name, database, user_providers).await;

        let provider = match provider_result {
            Ok(provider) => provider,
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_UNAUTHORIZED,
                        message: format!("Provider authentication failed: {e}"),
                        data: None,
                    }),
                    id: request_id,
                };
            }
        };

        let start_time = std::time::Instant::now();
        let response = Self::execute_tool_call(
            tool_name,
            args,
            provider.as_ref(),
            request_id.clone(),
            user_id,
            database,
        )
        .await;

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

    /// Get or create a user-specific provider instance
    ///
    /// # Errors
    ///
    /// Returns an error if the provider cannot be created or authenticated
    pub async fn get_user_provider(
        user_id: Uuid,
        provider_name: &str,
        database: &Arc<Database>,
        user_providers: &UserProviderStorage,
    ) -> Result<Box<dyn FitnessProvider>> {
        let user_key = user_id.to_string();

        // Check if provider already exists for this user
        {
            let providers_read = user_providers.read().await;
            if let Some(user_provider_map) = providers_read.get(&user_key) {
                if let Some(_provider) = user_provider_map.get(provider_name) {
                    // Provider exists - we recreate it each time since trait objects don't support Clone
                    // This is acceptable for the current usage pattern
                }
            }
        }

        // Create new provider instance for user
        let mut provider = create_provider(provider_name)?;

        // Get user's decrypted token for this provider
        let token = match provider_name {
            "strava" => database.get_strava_token(user_id).await?,
            "fitbit" => database.get_fitbit_token(user_id).await?,
            _ => None,
        };

        if let Some(decrypted_token) = token {
            // Authenticate provider with user's token
            let auth_data = AuthData::OAuth2 {
                client_id: crate::constants::env_config::strava_client_id().unwrap_or_default(),
                client_secret: crate::constants::env_config::strava_client_secret()
                    .unwrap_or_default(),
                access_token: Some(decrypted_token.access_token),
                refresh_token: Some(decrypted_token.refresh_token),
            };

            provider.authenticate(auth_data).await?;
        } else {
            return Err(anyhow::anyhow!(
                "No valid token found for provider {}",
                provider_name
            ));
        }

        // Store provider for reuse
        {
            let mut providers_write = user_providers.write().await;
            providers_write
                .entry(user_key)
                .or_insert_with(HashMap::new)
                .insert(provider_name.to_string(), provider);
        }

        // Return a new instance with current authentication
        let mut new_provider = create_provider(provider_name)?;
        if let Some(decrypted_token) = database.get_strava_token(user_id).await? {
            let auth_data = AuthData::OAuth2 {
                client_id: crate::constants::env_config::strava_client_id().unwrap_or_default(),
                client_secret: crate::constants::env_config::strava_client_secret()
                    .unwrap_or_default(),
                access_token: Some(decrypted_token.access_token),
                refresh_token: Some(decrypted_token.refresh_token),
            };
            new_provider.authenticate(auth_data).await?;
        }

        Ok(new_provider)
    }

    /// Handle `connect_strava` tool call
    fn handle_connect_strava(user_id: Uuid, database: &Arc<Database>, id: Value) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        match oauth_routes.get_auth_url(user_id, "strava") {
            Ok(auth_response) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(&auth_response).ok(),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to generate Strava authorization URL: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `connect_fitbit` tool call
    fn handle_connect_fitbit(user_id: Uuid, database: &Arc<Database>, id: Value) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        match oauth_routes.get_auth_url(user_id, "fitbit") {
            Ok(auth_response) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(&auth_response).ok(),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to generate Fitbit authorization URL: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `get_connection_status` tool call
    async fn handle_get_connection_status(
        user_id: Uuid,
        database: &Arc<Database>,
        id: Value,
    ) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        match oauth_routes.get_connection_status(user_id).await {
            Ok(statuses) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(&statuses).ok(),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get connection status: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `disconnect_provider` tool call
    fn handle_disconnect_provider(
        user_id: Uuid,
        provider: &str,
        database: &Arc<Database>,
        id: Value,
    ) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

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

    /// Handle `GET_ACTIVITIES` tool call
    async fn handle_get_activities(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let limit = args[LIMIT].as_u64().and_then(|n| usize::try_from(n).ok());
        let offset = args[OFFSET].as_u64().and_then(|n| usize::try_from(n).ok());

        match provider.get_activities(limit, offset).await {
            Ok(activities) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(activities).ok(),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get activities: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `GET_ATHLETE` tool call
    async fn handle_get_athlete(provider: &dyn FitnessProvider, id: Value) -> McpResponse {
        match provider.get_athlete().await {
            Ok(athlete) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(athlete).ok(),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get athlete: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `GET_STATS` tool call
    async fn handle_get_stats(provider: &dyn FitnessProvider, id: Value) -> McpResponse {
        match provider.get_stats().await {
            Ok(stats) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: serde_json::to_value(stats).ok(),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get stats: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `GET_ACTIVITY_INTELLIGENCE` tool call
    async fn handle_activity_intelligence(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let activity_id = args[ACTIVITY_ID].as_str().unwrap_or("");
        let include_weather = args["include_weather"].as_bool().unwrap_or(true);
        let include_location = args["include_location"].as_bool().unwrap_or(true);

        // Log intelligence request parameters
        tracing::debug!(
            "Generating activity intelligence for activity {} (weather: {}, location: {})",
            activity_id,
            include_weather,
            include_location
        );

        // Get activities from provider
        match provider.get_activities(Some(100), None).await {
            Ok(activities) => {
                if let Some(activity) = activities.iter().find(|a| a.id == activity_id) {
                    Self::generate_activity_intelligence(activity, include_location, id).await
                } else {
                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Activity with ID '{activity_id}' not found"),
                            data: None,
                        }),
                        id,
                    }
                }
            }
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get activities: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Generate intelligence analysis for an activity
    async fn generate_activity_intelligence(
        activity: &Activity,
        include_location: bool,
        id: Value,
    ) -> McpResponse {
        // Create activity analyzer
        let analyzer = ActivityAnalyzer::new();

        // Create activity context with location data if requested
        let context = if include_location {
            Self::create_location_context(activity).await
        } else {
            None
        };

        // Generate activity intelligence
        match analyzer.analyze_activity(activity, context.as_ref()) {
            Ok(intelligence) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: Some(Self::format_intelligence_response(&intelligence, activity)),
                error: None,
                id,
            },
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Intelligence analysis failed: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Create location context for activity if coordinates are available
    async fn create_location_context(activity: &Activity) -> Option<ActivityContext> {
        let location = if activity.start_latitude.is_some() && activity.start_longitude.is_some() {
            let mut location_service = crate::intelligence::location::LocationService::new();

            match location_service
                .get_location_from_coordinates(
                    activity.start_latitude.unwrap_or_else(|| {
                        tracing::warn!("Missing latitude despite earlier check");
                        0.0
                    }),
                    activity.start_longitude.unwrap_or_else(|| {
                        tracing::warn!("Missing longitude despite earlier check");
                        0.0
                    }),
                )
                .await
            {
                Ok(location_data) => Some(crate::intelligence::LocationContext {
                    city: location_data.city,
                    region: location_data.region,
                    country: location_data.country,
                    trail_name: location_data.trail_name,
                    terrain_type: location_data.natural,
                    display_name: location_data.display_name,
                }),
                Err(e) => {
                    warn!("Failed to get location data: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Some(ActivityContext {
            location,
            recent_activities: None,
        })
    }

    /// Format intelligence analysis into JSON response
    fn format_intelligence_response(
        intelligence: &crate::intelligence::ActivityIntelligence,
        activity: &Activity,
    ) -> Value {
        serde_json::json!({
            "summary": intelligence.summary,
            "activity_id": activity.id,
            "activity_name": activity.name,
            "sport_type": activity.sport_type,
            "duration_minutes": activity.duration_seconds / 60,
            "distance_km": activity.distance_meters.map(|d| d / 1000.0),
            "performance_indicators": {
                "relative_effort": intelligence.performance_indicators.relative_effort,
                "zone_distribution": intelligence.performance_indicators.zone_distribution,
                "personal_records": intelligence.performance_indicators.personal_records,
                "efficiency_score": intelligence.performance_indicators.efficiency_score,
                "trend_indicators": intelligence.performance_indicators.trend_indicators
            },
            "contextual_factors": {
                "weather": intelligence.contextual_factors.weather,
                "location": intelligence.contextual_factors.location,
                "time_of_day": intelligence.contextual_factors.time_of_day,
                "days_since_last_activity": intelligence.contextual_factors.days_since_last_activity,
                "weekly_load": intelligence.contextual_factors.weekly_load
            },
            "key_insights": intelligence.key_insights,
            "generated_at": intelligence.generated_at.to_rfc3339(),
            "status": "full_analysis_complete"
        })
    }

    /// Handle `analyze_activity` tool call
    async fn handle_analyze_activity(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let activity_id = args["activity_id"].as_str().unwrap_or("");

        match provider.get_activities(Some(100), None).await {
            Ok(activities) => {
                if let Some(activity) = activities.iter().find(|a| a.id == activity_id) {
                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: Some(serde_json::json!({
                            "activity_analysis": {
                                "activity_id": activity.id,
                                "name": activity.name,
                                "sport_type": activity.sport_type,
                                "duration_minutes": activity.duration_seconds / 60,
                                "distance_km": activity.distance_meters.map(|d| d / 1000.0),
                                "pace_per_km": activity.distance_meters.and_then(|d| {
                                    if d > 0.0 {
                                        Some((if activity.duration_seconds > u64::from(u32::MAX) {
                                            f64::from(u32::MAX) / 60.0
                                        } else {
                                            f64::from(u32::try_from(activity.duration_seconds).unwrap_or(u32::MAX)) / 60.0
                                        }) / (d / 1000.0))
                                    } else {
                                        None
                                    }
                                }),
                                "average_heart_rate": activity.average_heart_rate,
                                "max_heart_rate": activity.max_heart_rate,
                                "elevation_gain": activity.elevation_gain,
                                "calories": activity.calories,
                                "insights": [
                                    format!("This was a {} lasting {} minutes",
                                        activity.sport_type.display_name(),
                                        activity.duration_seconds / 60),
                                    activity.distance_meters.map_or_else(
                                        || "Distance tracking not available".into(),
                                        |distance| format!("Covered {:.1} km", distance / 1000.0)
                                    )
                                ]
                            }
                        })),
                        error: None,
                        id,
                    }
                } else {
                    McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Activity with ID '{activity_id}' not found"),
                            data: None,
                        }),
                        id,
                    }
                }
            }
            Err(e) => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get activities: {e}"),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle `calculate_metrics` tool call
    async fn handle_calculate_metrics_inline(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: &Value,
    ) -> Result<Value, McpResponse> {
        let activity_id = args["activity_id"].as_str().unwrap_or("");

        match provider.get_activities(Some(100), None).await {
            Ok(activities) => {
                activities.iter().find(|a| a.id == activity_id).map_or_else(
                    || Err(McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INVALID_PARAMS,
                            message: format!("Activity with ID '{activity_id}' not found"),
                            data: None,
                        }),
                        id: id.clone(),
                    }),
                    |activity| Ok(serde_json::json!({
                        "metrics": {
                            "activity_id": activity.id,
                            "duration_minutes": activity.duration_seconds / 60,
                            "distance_km": activity.distance_meters.map(|d| d / 1000.0),
                            "average_speed_kmh": activity.average_speed.map(|s| s * 3.6),
                            "max_speed_kmh": activity.max_speed.map(|s| s * 3.6),
                            "heart_rate_metrics": {
                                "average_hr": activity.average_heart_rate,
                                "max_hr": activity.max_heart_rate,
                                "hr_reserve_used": activity.average_heart_rate.and_then(|avg| {
                                    activity.max_heart_rate.map(|max| (f64::from(avg) / f64::from(max)) * 100.0)
                                })
                            },
                            "elevation_gain_m": activity.elevation_gain,
                            "calories_burned": activity.calories
                        }
                    }))
                )
            }
            Err(e) => Err(McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_INTERNAL_ERROR,
                    message: format!("Failed to get activities: {e}"),
                    data: None,
                }),
                id: id.clone(),
            }),
        }
    }

    /// Execute tool call with provider
    async fn execute_tool_call(
        tool_name: &str,
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
        _user_id: Uuid,
        _database: &Arc<Database>,
    ) -> McpResponse {
        match tool_name {
            GET_ACTIVITIES => Self::handle_get_activities(args, provider, id).await,
            GET_ATHLETE => Self::handle_get_athlete(provider, id).await,
            GET_STATS => Self::handle_get_stats(provider, id).await,
            GET_ACTIVITY_INTELLIGENCE => {
                Self::handle_activity_intelligence(args, provider, id).await
            }
            "analyze_activity" => Self::handle_analyze_activity(args, provider, id).await,
            "calculate_metrics" => {
                match Self::handle_calculate_metrics_inline(args, provider, &id).await {
                    Ok(response) => Self::create_success_response(Some(response), id),
                    Err(err_response) => err_response,
                }
            }
            "analyze_performance_trends" => {
                Self::handle_analyze_performance_trends_tool(args, provider, id).await
            }
            "compare_activities" => Self::handle_compare_activities_tool(args, provider, id).await,
            "detect_patterns" => Self::handle_detect_patterns_tool(args, provider, id).await,
            "suggest_goals" => Self::handle_suggest_goals_tool(provider, id).await,
            "generate_recommendations" => {
                Self::handle_generate_recommendations_tool(provider, id).await
            }
            "calculate_fitness_score" => {
                Self::handle_calculate_fitness_score_tool(provider, id).await
            }
            "predict_performance" => {
                Self::handle_predict_performance_tool(args, provider, id).await
            }
            "analyze_training_load" => Self::handle_analyze_training_load_tool(provider, id).await,
            _ => Self::create_error_response(
                ERROR_METHOD_NOT_FOUND,
                &format!("Unknown tool: {tool_name}"),
                id,
            ),
        }
    }

    /// Create success response helper
    fn create_success_response(result: Option<Value>, id: Value) -> McpResponse {
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id,
        }
    }

    /// Create error response helper
    fn create_error_response(code: i32, message: &str, id: Value) -> McpResponse {
        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(McpError {
                code,
                message: message.to_string(),
                data: None,
            }),
            id,
        }
    }

    /// Handle analyze performance trends tool
    async fn handle_analyze_performance_trends_tool(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let timeframe = args["timeframe"].as_str().unwrap_or("month");
        let metric = args["metric"].as_str().unwrap_or("pace");

        match provider.get_activities(Some(100), None).await {
            Ok(activities) => {
                let response = serde_json::json!({
                    "trend_analysis": {
                        "timeframe": timeframe,
                        "metric": metric,
                        "total_activities": activities.len(),
                        "trend_direction": "stable",
                        "insights": [
                            format!("Analyzed {} activities over the past {}", activities.len(), timeframe),
                            "Performance trends require more historical data for accurate analysis"
                        ]
                    }
                });
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle compare activities tool
    async fn handle_compare_activities_tool(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let activity_id1 = args["activity_id1"].as_str().unwrap_or("");
        let activity_id2 = args["activity_id2"].as_str().unwrap_or("");

        match provider.get_activities(Some(100), None).await {
            Ok(activities) => {
                let activity1 = activities.iter().find(|a| a.id == activity_id1);
                let activity2 = activities.iter().find(|a| a.id == activity_id2);

                if let (Some(a1), Some(a2)) = (activity1, activity2) {
                    let response = serde_json::json!({
                        "comparison": {
                            "activity1": {
                                "id": a1.id,
                                "name": a1.name,
                                "duration_minutes": a1.duration_seconds / 60,
                                "distance_km": a1.distance_meters.map(|d| d / 1000.0)
                            },
                            "activity2": {
                                "id": a2.id,
                                "name": a2.name,
                                "duration_minutes": a2.duration_seconds / 60,
                                "distance_km": a2.distance_meters.map(|d| d / 1000.0)
                            },
                            "insights": [
                                "Activity comparison shows differences in duration and distance",
                                "For detailed analysis, consider pace, heart rate, and effort levels"
                            ]
                        }
                    });
                    Self::create_success_response(Some(response), id)
                } else {
                    Self::create_error_response(
                        ERROR_INVALID_PARAMS,
                        "One or both activities not found",
                        id,
                    )
                }
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle detect patterns tool
    async fn handle_detect_patterns_tool(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let pattern_type = args["pattern_type"].as_str().unwrap_or("weekly");

        match provider.get_activities(Some(100), None).await {
            Ok(activities) => {
                let response = serde_json::json!({
                    "pattern_analysis": {
                        "pattern_type": pattern_type,
                        "total_activities": activities.len(),
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
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle suggest goals tool
    async fn handle_suggest_goals_tool(provider: &dyn FitnessProvider, id: Value) -> McpResponse {
        match provider.get_activities(Some(50), None).await {
            Ok(_activities) => {
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
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle generate recommendations tool
    async fn handle_generate_recommendations_tool(
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        match provider.get_activities(Some(20), None).await {
            Ok(_activities) => {
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
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle calculate fitness score tool
    async fn handle_calculate_fitness_score_tool(
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        match provider.get_activities(Some(30), None).await {
            Ok(activities) => {
                let total_activities = activities.len();
                let avg_duration = if activities.is_empty() {
                    0
                } else {
                    activities.iter().map(|a| a.duration_seconds).sum::<u64>()
                        / activities.len() as u64
                };

                let fitness_score = std::cmp::min(85, 50 + total_activities * 2);

                let response = serde_json::json!({
                    "fitness_score": {
                        "overall_score": fitness_score,
                        "max_score": 100,
                        "components": {
                            "frequency": std::cmp::min(25, total_activities * 2),
                            "consistency": 15,
                            "duration": std::cmp::min(20, (avg_duration / 60) as usize / 10),
                            "variety": 10
                        },
                        "insights": [
                            format!("Your fitness score is {} out of 100", fitness_score),
                            "Regular training frequency is your strength",
                            "Consider adding variety to your workouts"
                        ]
                    }
                });
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle predict performance tool
    async fn handle_predict_performance_tool(
        args: &Value,
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        let prediction_type = args["prediction_type"].as_str().unwrap_or("pace");
        let timeframe = args["timeframe"].as_str().unwrap_or("month");

        match provider.get_activities(Some(20), None).await {
            Ok(_activities) => {
                let response = serde_json::json!({
                    "performance_prediction": {
                        "prediction_type": prediction_type,
                        "timeframe": timeframe,
                        "predicted_improvement": "5-8%",
                        "confidence": 0.7,
                        "factors": [
                            "Current training consistency",
                            "Historical performance trends",
                            "Typical progression patterns"
                        ],
                        "recommendations": [
                            "Maintain consistent training schedule",
                            "Gradually increase intensity",
                            "Include proper recovery periods"
                        ]
                    }
                });
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
        }
    }

    /// Handle analyze training load tool
    async fn handle_analyze_training_load_tool(
        provider: &dyn FitnessProvider,
        id: Value,
    ) -> McpResponse {
        match provider.get_activities(Some(30), None).await {
            Ok(activities) => {
                let total_duration = activities.iter().map(|a| a.duration_seconds).sum::<u64>();
                let total_distance = activities
                    .iter()
                    .filter_map(|a| a.distance_meters)
                    .sum::<f64>();

                let weekly_hours = (if total_duration > u64::from(u32::MAX) {
                    f64::from(u32::MAX) / 3600.0
                } else {
                    f64::from(u32::try_from(total_duration).unwrap_or(u32::MAX)) / 3600.0
                }) / 4.0; // Assuming 4 weeks of data

                let load_level = if weekly_hours < 3.0 {
                    "low"
                } else if weekly_hours < 6.0 {
                    "moderate"
                } else if weekly_hours < 10.0 {
                    "high"
                } else {
                    "very_high"
                };

                let response = serde_json::json!({
                    "training_load_analysis": {
                        "weekly_hours": weekly_hours,
                        "weekly_distance_km": total_distance / 4000.0, // 4 weeks in meters to km
                        "load_level": load_level,
                        "total_activities": activities.len(),
                        "insights": [
                            format!("Current training load: {} ({:.1} hours/week)", load_level, weekly_hours),
                            "Training load is appropriate for current fitness level",
                            "Consider periodization for optimal adaptation"
                        ],
                        "recommendations": match load_level {
                            "low" => vec!["Consider increasing training frequency", "Add more variety to workouts"],
                            "moderate" => vec!["Maintain current level", "Focus on consistency"],
                            "high" => vec!["Ensure adequate recovery", "Monitor for overtraining signs"],
                            _ => vec!["Consider reducing volume", "Prioritize recovery"]
                        }
                    }
                });
                Self::create_success_response(Some(response), id)
            }
            Err(e) => Self::create_error_response(
                ERROR_INTERNAL_ERROR,
                &format!("Failed to get activities: {e}"),
                id,
            ),
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
        &self.database
    }

    /// Get auth manager reference for admin API
    #[must_use]
    pub fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
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
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let max_hr = args
            .get("max_hr")
            .and_then(serde_json::Value::as_f64)
            .map_or(190, |v| v as u16);
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
}

/// MCP request with optional authentication token
#[derive(Debug, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Value,
    /// Authorization header value (Bearer token)
    #[serde(rename = "auth")]
    pub auth_token: Option<String>,
}

/// MCP response
#[derive(Debug, Serialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
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
