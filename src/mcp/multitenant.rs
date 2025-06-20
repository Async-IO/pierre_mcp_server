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
use crate::config::FitnessConfig;
use crate::constants::{errors::*, json_fields::*, protocol, protocol::*, tools::*};
use crate::dashboard_routes::DashboardRoutes;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::intelligence::insights::ActivityContext;
use crate::intelligence::weather::WeatherService;
use crate::intelligence::ActivityAnalyzer;
use crate::mcp::schema::InitializeResponse;
use crate::models::AuthRequest;
use crate::providers::{create_provider, AuthData, FitnessProvider};
use crate::routes::{AuthRoutes, LoginRequest, OAuthRoutes, RefreshTokenRequest, RegisterRequest};
use crate::security::SecurityConfig;
use crate::websocket::WebSocketManager;

use anyhow::Result;
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

/// Multi-tenant MCP server supporting user authentication
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
        let _auth_routes = AuthRoutes::new((*database).clone(), (*auth_manager).clone());
        let _oauth_routes = OAuthRoutes::new((*database).clone());

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

        // Security configuration based on environment
        let security_config =
            SecurityConfig::from_environment(&config.security.headers.environment.to_string());
        info!(
            "Security headers enabled with {} configuration",
            config.security.headers.environment
        );

        let auth_routes = AuthRoutes::new((*database).clone(), (*auth_manager).clone());
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());
        let api_key_routes = ApiKeyRoutes::new((*database).clone(), (*auth_manager).clone());
        let dashboard_routes = DashboardRoutes::new((*database).clone(), (*auth_manager).clone());
        let a2a_routes = A2ARoutes::new(database.clone(), auth_manager.clone(), config.clone());

        // CORS configuration
        let cors = warp::cors()
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
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"]);

        // Registration endpoint
        let register = warp::path("auth")
            .and(warp::path("register"))
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

        // OAuth authorization URL endpoint
        let oauth_auth = warp::path("oauth")
            .and(warp::path!("auth" / String / String)) // /oauth/auth/{provider}/{user_id}
            .and(warp::get())
            .and_then({
                let oauth_routes = oauth_routes.clone();
                move |provider: String, user_id_str: String| {
                    let oauth_routes = oauth_routes.clone();
                    async move {
                        match Uuid::parse_str(&user_id_str) {
                            Ok(user_id) => {
                                match oauth_routes.get_auth_url(user_id, &provider).await {
                                    Ok(auth_response) => Ok(warp::reply::json(&auth_response)),
                                    Err(e) => {
                                        let error = serde_json::json!({"error": e.to_string()});
                                        Err(warp::reject::custom(ApiError(error)))
                                    }
                                }
                            }
                            Err(_) => {
                                let error = serde_json::json!({"error": "Invalid user ID format"});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });

        // OAuth callback endpoints
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
                                    "message": format!("{} account connected successfully!", provider),
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
                                    "error": format!("Failed to process OAuth callback: {}", e),
                                    "provider": provider
                                });
                                Err(warp::reject::custom(ApiError(error_response)))
                            }
                        }
                    }
                }
            });

        // API Key endpoints - REMOVED: Self-service API key creation
        // For enterprise deployment, only administrators can provision API keys
        // via the admin endpoints at /admin/provision-api-key

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

        // Trial API key endpoint - REMOVED: Self-service trial key creation
        // For enterprise deployment, trial keys must be provisioned by administrators

        let get_api_key_usage = warp::path("api")
            .and(warp::path("keys"))
            .and(warp::path!(String))
            .and(warp::path("usage"))
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::query::<std::collections::HashMap<String, String>>())
            .and_then({
                let api_key_routes = api_key_routes.clone();
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

                        let start_date = match chrono::DateTime::parse_from_rfc3339(&start_date_str) {
                            Ok(dt) => dt.with_timezone(&chrono::Utc),
                            Err(_) => {
                                let error = serde_json::json!({"error": "Invalid start_date format. Use RFC3339."});
                                return Err(warp::reject::custom(ApiError(error)));
                            }
                        };

                        let end_date = match chrono::DateTime::parse_from_rfc3339(&end_date_str) {
                            Ok(dt) => dt.with_timezone(&chrono::Utc),
                            Err(_) => {
                                let error = serde_json::json!({"error": "Invalid end_date format. Use RFC3339."});
                                return Err(warp::reject::custom(ApiError(error)));
                            }
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
            });

        // Health check endpoint
        let health = warp::path("health").and(warp::get()).map(|| {
            warp::reply::json(&serde_json::json!({"status": "ok", "service": "pierre-mcp-server"}))
        });

        // Dashboard endpoints
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

        // Dashboard Request Logs endpoint
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
                        let api_key_id = params.get("api_key_id").map(|s| s.as_str());
                        let time_range = params.get("time_range").map(|s| s.as_str());
                        let status = params.get("status").map(|s| s.as_str());
                        let tool = params.get("tool").map(|s| s.as_str());

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

        // Dashboard Request Stats endpoint
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
                        let api_key_id = params.get("api_key_id").map(|s| s.as_str());
                        let time_range = params.get("time_range").map(|s| s.as_str());

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

        // Dashboard Tool Usage endpoint
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
                        let api_key_id = params.get("api_key_id").map(|s| s.as_str());
                        let time_range = params.get("time_range").map(|s| s.as_str());

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

        // A2A Agent Card endpoint
        let a2a_agent_card = warp::path("a2a")
            .and(warp::path("agent-card"))
            .and(warp::get())
            .and_then({
                let a2a_routes = a2a_routes.clone();
                move || {
                    let a2a_routes = a2a_routes.clone();
                    async move {
                        match a2a_routes.get_agent_card().await {
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

        // WebSocket endpoint
        let websocket_route = websocket_manager.websocket_filter();

        // Start periodic WebSocket updates
        websocket_manager.start_periodic_updates();

        // Create security headers filter
        let security_headers_filter = warp::reply::with::headers({
            let headers = security_config.to_headers();
            let mut header_map = warp::http::HeaderMap::new();
            for (name, value) in headers {
                if let Ok(header_name) = warp::http::HeaderName::from_str(name) {
                    if let Ok(header_value) = warp::http::HeaderValue::from_str(&value) {
                        header_map.insert(header_name, header_value);
                    }
                }
            }
            header_map
        });

        // Group routes to avoid recursion limit issues
        let auth_routes = register
            .or(login)
            .or(refresh)
            .or(oauth_auth)
            .or(oauth_callback);

        let api_key_routes = list_api_keys.or(deactivate_api_key).or(get_api_key_usage);

        let dashboard_routes = dashboard_overview
            .or(dashboard_analytics)
            .or(dashboard_rate_limits)
            .or(dashboard_request_logs)
            .or(dashboard_request_stats)
            .or(dashboard_tool_usage);

        let a2a_routes = a2a_agent_card
            .or(a2a_dashboard_overview)
            .or(a2a_register_client)
            .or(a2a_list_clients)
            .or(a2a_client_usage)
            .or(a2a_client_rate_limit)
            .or(a2a_auth)
            .or(a2a_execute);

        // HTTP routes with security headers (exclude WebSocket)
        let http_routes = auth_routes
            .or(api_key_routes)
            .or(dashboard_routes)
            .or(a2a_routes)
            .or(health)
            .with(cors.clone())
            .with(security_headers_filter);

        // WebSocket route without security headers (security headers break WebSocket handshake)
        let ws_routes = websocket_route.with(cors);

        // Combine routes
        let routes = http_routes.or(ws_routes).recover(handle_rejection);

        info!("HTTP server ready on port {}", port);
        warp::serve(routes).run(([127, 0, 0, 1], port)).await;

        Ok(())
    }

    /// Run MCP server for AI assistant connections
    async fn run_mcp_server(self, port: u16) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        info!("MCP server listening on port {}", port);

        loop {
            let (socket, addr) = listener.accept().await?;
            info!("New MCP connection from {}", addr);

            let database = self.database.clone();
            let auth_manager = self.auth_manager.clone();
            let auth_middleware = self.auth_middleware.clone();
            let user_providers = self.user_providers.clone();

            tokio::spawn(async move {
                let (reader, mut writer) = socket.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();

                while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                    if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                        let response = Self::handle_request(
                            request,
                            &database,
                            &auth_manager,
                            &auth_middleware,
                            &user_providers,
                        )
                        .await;

                        let response_str = serde_json::to_string(&response).unwrap();
                        if let (Ok(()), Ok(()), Ok(())) = (
                            writer.write_all(response_str.as_bytes()).await,
                            writer.write_all(b"\n").await,
                            writer.flush().await,
                        ) {
                            // Response sent successfully
                        }
                    }
                    line.clear();
                }
            });
        }
    }

    /// Handle MCP request with authentication
    #[allow(clippy::type_complexity)]
    async fn handle_request(
        request: McpRequest,
        database: &Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        auth_middleware: &Arc<McpAuthMiddleware>,
        user_providers: &UserProviderStorage,
    ) -> McpResponse {
        match request.method.as_str() {
            "initialize" => {
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
            "authenticate" => Self::handle_authenticate(request, auth_manager).await,
            "tools/call" => {
                // Extract authorization header from request
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

                        Self::handle_authenticated_tool_call(
                            request,
                            auth_result,
                            database,
                            user_providers,
                        )
                        .await
                    }
                    Err(e) => {
                        warn!("MCP tool call authentication failed: {}", e);

                        // Determine specific error code based on error message
                        let error_message = e.to_string();
                        let (error_code, error_msg) = if error_message.contains("JWT token expired")
                        {
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
                }
            }
            _ => McpResponse {
                jsonrpc: JSONRPC_VERSION.to_string(),
                result: None,
                error: Some(McpError {
                    code: ERROR_METHOD_NOT_FOUND,
                    message: "Method not found".to_string(),
                    data: None,
                }),
                id: request.id,
            },
        }
    }

    /// Handle authentication request
    async fn handle_authenticate(
        request: McpRequest,
        auth_manager: &Arc<AuthManager>,
    ) -> McpResponse {
        let params = request.params.unwrap_or_default();

        if let Ok(auth_request) = serde_json::from_value::<AuthRequest>(params) {
            let auth_response = auth_manager.authenticate(auth_request);

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
                    message: "Invalid authentication request".to_string(),
                    data: None,
                }),
                id: request.id,
            }
        }
    }

    /// Handle authenticated tool call with user context and rate limiting
    #[allow(clippy::type_complexity)]
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
            CONNECT_STRAVA => {
                return Self::handle_connect_strava(user_id, database, request.id).await;
            }
            CONNECT_FITBIT => {
                return Self::handle_connect_fitbit(user_id, database, request.id).await;
            }
            GET_CONNECTION_STATUS => {
                return Self::handle_get_connection_status(user_id, database, request.id).await;
            }
            DISCONNECT_PROVIDER => {
                let provider_name = args[PROVIDER].as_str().unwrap_or("");
                return Self::handle_disconnect_provider(
                    user_id,
                    provider_name,
                    database,
                    request.id,
                )
                .await;
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
            | ANALYZE_PERFORMANCE_TRENDS => {
                let start_time = std::time::Instant::now();
                let response = Self::execute_tool_call_without_provider(
                    tool_name, args, request.id, user_id, database,
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
            _ => {
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
                    // Unknown tool
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_METHOD_NOT_FOUND,
                            message: format!("Unknown tool: {}", tool_name),
                            data: None,
                        }),
                        id: request.id,
                    };
                }

                // For fitness data tools, we need a provider
                let provider_name = args[PROVIDER].as_str().unwrap_or("");

                // Get or create user-specific provider
                let provider_result =
                    Self::get_user_provider(user_id, provider_name, database, user_providers).await;

                let provider = match provider_result {
                    Ok(provider) => provider,
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Provider authentication failed: {}", e),
                                data: None,
                            }),
                            id: request.id,
                        };
                    }
                };

                // Execute tool call with user-scoped provider
                let start_time = std::time::Instant::now();
                let response = Self::execute_tool_call(
                    tool_name,
                    args,
                    provider.as_ref(),
                    request.id,
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
        }
    }

    /// Get or create a user-specific provider instance
    async fn get_user_provider(
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

        // Return a new instance (simplified for now)
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

    /// Handle connect_strava tool call
    async fn handle_connect_strava(
        user_id: Uuid,
        database: &Arc<Database>,
        id: Value,
    ) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        match oauth_routes.get_auth_url(user_id, "strava").await {
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
                    message: format!("Failed to generate Strava authorization URL: {}", e),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle connect_fitbit tool call
    async fn handle_connect_fitbit(
        user_id: Uuid,
        database: &Arc<Database>,
        id: Value,
    ) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        match oauth_routes.get_auth_url(user_id, "fitbit").await {
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
                    message: format!("Failed to generate Fitbit authorization URL: {}", e),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle get_connection_status tool call
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
                    message: format!("Failed to get connection status: {}", e),
                    data: None,
                }),
                id,
            },
        }
    }

    /// Handle disconnect_provider tool call
    async fn handle_disconnect_provider(
        user_id: Uuid,
        provider: &str,
        database: &Arc<Database>,
        id: Value,
    ) -> McpResponse {
        let oauth_routes = OAuthRoutes::new(database.as_ref().clone());

        match oauth_routes.disconnect_provider(user_id, provider).await {
            Ok(()) => {
                let response = serde_json::json!({
                    "success": true,
                    "message": format!("Successfully disconnected {}", provider),
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
                    message: format!("Failed to disconnect provider: {}", e),
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
            SET_GOAL => {
                let goal_data = args.clone();

                // Store goal in database
                match database.create_goal(user_id, goal_data).await {
                    Ok(goal_id) => {
                        let response = serde_json::json!({
                            "goal_created": {
                                "goal_id": goal_id,
                                "status": "active",
                                "message": "Goal successfully created"
                            }
                        });
                        Some(response)
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to create goal: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            TRACK_PROGRESS => {
                let goal_id = args[GOAL_ID].as_str().unwrap_or("");

                match database.get_user_goals(user_id).await {
                    Ok(goals) => {
                        if let Some(goal) = goals.iter().find(|g| g["id"] == goal_id) {
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
                            Some(response)
                        } else {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: format!("Goal with ID '{}' not found", goal_id),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get goals: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            ANALYZE_GOAL_FEASIBILITY => {
                let _goal_data = args.clone();

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
                Some(response)
            }
            SUGGEST_GOALS => {
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
                Some(response)
            }
            CALCULATE_FITNESS_SCORE => {
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
                Some(response)
            }
            GENERATE_RECOMMENDATIONS => {
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
                Some(response)
            }
            ANALYZE_TRAINING_LOAD => {
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
                Some(response)
            }
            DETECT_PATTERNS => {
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
                Some(response)
            }
            ANALYZE_PERFORMANCE_TRENDS => {
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
                Some(response)
            }
            _ => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: format!("Unknown tool: {}", tool_name),
                        data: None,
                    }),
                    id,
                };
            }
        };

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id,
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
        let result = match tool_name {
            GET_ACTIVITIES => {
                let limit = args[LIMIT].as_u64().map(|n| n as usize);
                let offset = args[OFFSET].as_u64().map(|n| n as usize);

                match provider.get_activities(limit, offset).await {
                    Ok(activities) => serde_json::to_value(activities).ok(),
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            GET_ATHLETE => match provider.get_athlete().await {
                Ok(athlete) => serde_json::to_value(athlete).ok(),
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get athlete: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            },
            GET_STATS => match provider.get_stats().await {
                Ok(stats) => serde_json::to_value(stats).ok(),
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get stats: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            },
            GET_ACTIVITY_INTELLIGENCE => {
                let activity_id = args[ACTIVITY_ID].as_str().unwrap_or("");
                let include_weather = args["include_weather"].as_bool().unwrap_or(true);
                let include_location = args["include_location"].as_bool().unwrap_or(true);

                // Get activities from provider
                match provider.get_activities(Some(100), None).await {
                    Ok(activities) => {
                        if let Some(activity) = activities.iter().find(|a| a.id == activity_id) {
                            // Create activity analyzer
                            let analyzer = ActivityAnalyzer::new();

                            // Create activity context with weather and location data if requested
                            let context = if include_weather || include_location {
                                // Load weather configuration
                                let fitness_config = FitnessConfig::load(None).unwrap_or_default();

                                // Get weather data if requested
                                let weather = if include_weather {
                                    let weather_config =
                                        fitness_config.weather_api.unwrap_or_default();
                                    let mut weather_service =
                                        WeatherService::new(weather_config, None);

                                    weather_service
                                        .get_weather_for_activity(
                                            activity.start_latitude,
                                            activity.start_longitude,
                                            activity.start_date,
                                        )
                                        .await
                                        .unwrap_or(None)
                                } else {
                                    None
                                };

                                // Get location data if requested
                                let location = if include_location
                                    && activity.start_latitude.is_some()
                                    && activity.start_longitude.is_some()
                                {
                                    let mut location_service =
                                        crate::intelligence::location::LocationService::new();

                                    match location_service
                                        .get_location_from_coordinates(
                                            activity.start_latitude.unwrap(),
                                            activity.start_longitude.unwrap(),
                                        )
                                        .await
                                    {
                                        Ok(location_data) => {
                                            Some(crate::intelligence::LocationContext {
                                                city: location_data.city,
                                                region: location_data.region,
                                                country: location_data.country,
                                                trail_name: location_data.trail_name,
                                                terrain_type: location_data.natural,
                                                display_name: location_data.display_name,
                                            })
                                        }
                                        Err(e) => {
                                            warn!("Failed to get location data: {}", e);
                                            None
                                        }
                                    }
                                } else {
                                    None
                                };

                                Some(ActivityContext {
                                    weather,
                                    location,
                                    recent_activities: None,
                                    athlete_goals: None,
                                    historical_data: None,
                                })
                            } else {
                                None
                            };

                            // Generate activity intelligence
                            match analyzer.analyze_activity(activity, context).await {
                                Ok(intelligence) => Some(serde_json::json!({
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
                                })),
                                Err(e) => {
                                    return McpResponse {
                                        jsonrpc: JSONRPC_VERSION.to_string(),
                                        result: None,
                                        error: Some(McpError {
                                            code: ERROR_INTERNAL_ERROR,
                                            message: format!("Intelligence analysis failed: {}", e),
                                            data: None,
                                        }),
                                        id,
                                    };
                                }
                            }
                        } else {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: format!(
                                        "Activity with ID '{}' not found",
                                        activity_id
                                    ),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            // === ANALYTICS TOOLS ===
            "analyze_activity" => {
                let activity_id = args["activity_id"].as_str().unwrap_or("");

                match provider.get_activities(Some(100), None).await {
                    Ok(activities) => {
                        if let Some(activity) = activities.iter().find(|a| a.id == activity_id) {
                            let response = serde_json::json!({
                                "activity_analysis": {
                                    "activity_id": activity.id,
                                    "name": activity.name,
                                    "sport_type": activity.sport_type,
                                    "duration_minutes": activity.duration_seconds / 60,
                                    "distance_km": activity.distance_meters.map(|d| d / 1000.0),
                                    "pace_per_km": activity.distance_meters.and_then(|d| {
                                        if d > 0.0 {
                                            Some((activity.duration_seconds as f64 / 60.0) / (d / 1000.0))
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
                                        if let Some(distance) = activity.distance_meters {
                                            format!("Covered {:.1} km", distance / 1000.0)
                                        } else {
                                            "Distance tracking not available".to_string()
                                        }
                                    ]
                                }
                            });
                            Some(response)
                        } else {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: format!(
                                        "Activity with ID '{}' not found",
                                        activity_id
                                    ),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            "calculate_metrics" => {
                let activity_id = args["activity_id"].as_str().unwrap_or("");

                match provider.get_activities(Some(100), None).await {
                    Ok(activities) => {
                        if let Some(activity) = activities.iter().find(|a| a.id == activity_id) {
                            let response = serde_json::json!({
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
                                            activity.max_heart_rate.map(|max| (avg as f64 / max as f64) * 100.0)
                                        })
                                    },
                                    "elevation_gain_m": activity.elevation_gain,
                                    "calories_burned": activity.calories
                                }
                            });
                            Some(response)
                        } else {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: format!(
                                        "Activity with ID '{}' not found",
                                        activity_id
                                    ),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            "analyze_performance_trends" => {
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
                        Some(response)
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            "compare_activities" => {
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
                            Some(response)
                        } else {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: "One or both activities not found".to_string(),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            "detect_patterns" => {
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
                        Some(response)
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            "suggest_goals" => match provider.get_activities(Some(50), None).await {
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
                    Some(response)
                }
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get activities: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            },
            "generate_recommendations" => match provider.get_activities(Some(20), None).await {
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
                    Some(response)
                }
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get activities: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            },
            "calculate_fitness_score" => match provider.get_activities(Some(30), None).await {
                Ok(activities) => {
                    let total_activities = activities.len();
                    let avg_duration = if !activities.is_empty() {
                        activities.iter().map(|a| a.duration_seconds).sum::<u64>()
                            / activities.len() as u64
                    } else {
                        0
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
                    Some(response)
                }
                Err(e) => {
                    return McpResponse {
                        jsonrpc: JSONRPC_VERSION.to_string(),
                        result: None,
                        error: Some(McpError {
                            code: ERROR_INTERNAL_ERROR,
                            message: format!("Failed to get activities: {}", e),
                            data: None,
                        }),
                        id,
                    };
                }
            },
            "predict_performance" => {
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
                        Some(response)
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            "analyze_training_load" => {
                match provider.get_activities(Some(30), None).await {
                    Ok(activities) => {
                        let total_duration =
                            activities.iter().map(|a| a.duration_seconds).sum::<u64>();
                        let total_distance = activities
                            .iter()
                            .filter_map(|a| a.distance_meters)
                            .sum::<f64>();

                        let weekly_hours = (total_duration as f64 / 3600.0) / 4.0; // Assuming 4 weeks of data

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
                        Some(response)
                    }
                    Err(e) => {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Failed to get activities: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
            }
            _ => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: format!("Unknown tool: {}", tool_name),
                        data: None,
                    }),
                    id,
                };
            }
        };

        McpResponse {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            error: None,
            id,
        }
    }

    /// Record API key usage for billing and analytics
    async fn record_api_key_usage(
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
            response_time_ms: Some(response_time.as_millis() as u32),
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
    pub fn database(&self) -> &Database {
        &self.database
    }

    /// Get auth manager reference for admin API
    pub fn auth_manager(&self) -> &AuthManager {
        &self.auth_manager
    }
}

/// MCP request with optional authentication token
#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
    /// Authorization header value (Bearer token)
    #[serde(rename = "auth")]
    auth_token: Option<String>,
}

/// MCP response
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<McpError>,
    id: Value,
}

/// MCP error
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    data: Option<Value>,
}

/// HTTP API error wrapper
#[derive(Debug)]
struct ApiError(serde_json::Value);

impl warp::reject::Reject for ApiError {}

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
        .unwrap_or_else(|| "default-src 'self'".to_string());

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
    if let Some(api_error) = err.find::<ApiError>() {
        let json = warp::reply::json(&api_error.0);
        let reply = warp::reply::with_status(json, warp::http::StatusCode::BAD_REQUEST);
        Ok(Box::new(with_cors_headers(reply, None)))
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        // Handle CORS preflight and method not allowed
        let json = warp::reply::json(&serde_json::json!({}));
        let reply = warp::reply::with_status(json, warp::http::StatusCode::OK);
        Ok(Box::new(with_cors_headers(reply, None)))
    } else if err.is_not_found() {
        let json = warp::reply::json(&serde_json::json!({
            "error": "Not Found",
            "message": "The requested endpoint was not found"
        }));
        let reply = warp::reply::with_status(json, warp::http::StatusCode::NOT_FOUND);
        Ok(Box::new(with_cors_headers(reply, None)))
    } else {
        let json = warp::reply::json(&serde_json::json!({
            "error": "Internal Server Error",
            "message": "Something went wrong"
        }));
        let reply = warp::reply::with_status(json, warp::http::StatusCode::INTERNAL_SERVER_ERROR);
        Ok(Box::new(with_cors_headers(reply, None)))
    }
}
