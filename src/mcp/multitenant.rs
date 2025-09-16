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

use super::{
    http_setup::HttpSetup,
    protocol::ProtocolHandler,
    resources::ServerResources,
    tool_handlers::{McpOAuthCredentials, ToolHandlers, ToolRoutingContext},
};
use crate::a2a_routes::A2ARoutes;
use crate::api_key_routes::ApiKeyRoutes;
use crate::auth::{AuthManager, AuthResult};
use crate::configuration_routes::ConfigurationRoutes;
use crate::constants::{
    errors::{ERROR_INTERNAL_ERROR, ERROR_INVALID_PARAMS, ERROR_METHOD_NOT_FOUND},
    json_fields::{GOAL_ID, PROVIDER},
    oauth_providers, protocol,
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
use crate::oauth2::routes::oauth2_routes;
use crate::providers::ProviderRegistry;
use crate::routes::{AuthRoutes, LoginRequest, OAuthRoutes, RefreshTokenRequest, RegisterRequest};
use crate::security::headers::SecurityConfig;
use crate::tenant::{TenantContext, TenantOAuthClient, TenantRole};
use crate::utils::json_responses::{api_error, invalid_format_error, oauth_error};

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::Write;
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

/// Context for HTTP request handling with tenant support
struct HttpRequestContext {
    resources: Arc<ServerResources>,
}

/// MCP server supporting user authentication and isolated data access
#[derive(Clone)]
pub struct MultiTenantMcpServer {
    resources: Arc<ServerResources>,
}

impl MultiTenantMcpServer {
    /// Create a new MCP server with pre-built resources (dependency injection)
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
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

        // Run MCP server on main port (this sets up notification system and starts HTTP with shared resources)
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
            fitness_configuration_routes,
        ) = HttpSetup::setup_route_handlers_with_resources(&resources);

        // Use JWT secret from resources
        let jwt_secret_str = resources.admin_jwt_secret.as_ref();
        info!("Using admin JWT secret from server startup");

        // Setup admin routes - API requires owned values
        let admin_context = crate::admin_routes::AdminApiContext::new(
            resources.database.clone(),
            jwt_secret_str,
            resources.auth_manager.clone(),
        );
        let admin_routes_filter = crate::admin_routes::admin_routes_with_rejection(admin_context);

        // Setup tenant management routes - API requires owned values
        let tenant_routes_filter = Self::create_tenant_routes_filter(
            resources.database.clone(),
            resources.auth_manager.clone(),
        );

        // Configure CORS
        let cors = HttpSetup::setup_cors();

        // Create all route groups using helper functions
        let auth_route_filter = Self::create_auth_routes(&auth_routes);
        let oauth_route_filter = Self::create_oauth_routes(
            &oauth_routes,
            &resources.database,
            &resources.tenant_oauth_client,
        );

        // Create OAuth 2.0 server routes for mcp-remote compatibility
        let oauth2_server_routes =
            oauth2_routes(resources.database.clone(), resources.auth_manager.clone());
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

        // Create fitness configuration routes
        let fitness_configuration_filter =
            Self::create_fitness_configuration_routes(&fitness_configuration_routes);

        // Security headers middleware
        let security_headers = Self::create_security_headers_filter(&security_config);

        // Health check route
        let health_route = Self::create_health_route();

        // SSE notification routes
        let sse_routes = crate::notifications::sse::sse_routes(resources.sse_manager.clone());

        // Create websocket route (method needs to exist)
        // let websocket_route = Self::create_websocket_route(resources.websocket_manager.clone());

        // Combine all routes
        let routes = auth_route_filter
            .or(oauth_route_filter)
            .or(oauth2_server_routes)
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
            .or(fitness_configuration_filter)
            .or(admin_routes_filter)
            .or(tenant_routes_filter)
            .or(sse_routes)
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
                                oauth_providers::STRAVA => crate::constants::env_config::strava_redirect_uri(),
                                oauth_providers::FITBIT => crate::constants::env_config::fitbit_redirect_uri(),
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
                                    oauth_providers::STRAVA => crate::constants::oauth::STRAVA_DEFAULT_SCOPES.split(',').map(str::to_string).collect(),
                                    oauth_providers::FITBIT => vec!["activity".to_string(), "heartrate".to_string(), "location".to_string(), "nutrition".to_string(), "profile".to_string(), "settings".to_string(), "sleep".to_string(), "social".to_string(), "weight".to_string()],
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

                        // Get tenant information from database
                        let tenant_name = match database.get_tenant_by_id(tenant_id).await {
                            Ok(tenant) => tenant.name,
                            Err(_) => "Organization".to_string(), // Fallback if tenant lookup fails
                        };

                        // Use tenant context to get authorization URL  
                        let tenant_context = TenantContext {
                            tenant_id,
                            user_id,
                            tenant_name,
                            user_role: TenantRole::Member,
                        };

                        // Generate state for CSRF protection (mimics the routes implementation)
                        let new_uuid = Uuid::new_v4();
                        let state = format!("{user_id}:{new_uuid}");

                        match tenant_oauth_client
                            .get_authorization_url(&tenant_context, &provider, &state, database.as_ref())
                            .await
                        {
                            Ok(auth_url) => {
                                tracing::info!("Redirecting user {} to {} OAuth authorization URL", user_id, provider);
                                Ok(warp::reply::with_status(
                                    warp::reply::with_header(
                                        warp::reply(),
                                        "location",
                                        auth_url,
                                    ),
                                    warp::http::StatusCode::FOUND,
                                ))
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
                                let html_content = match Self::render_oauth_error_template(
                                    "Authorization Failed",
                                    "Missing OAuth code parameter. Please try connecting again.",
                                    None
                                ) {
                                    Ok(html) => html,
                                    Err(e) => {
                                        tracing::error!("Failed to render error template: {}", e);
                                        "<h1>Authorization Failed</h1><p>Missing OAuth code parameter. Please try connecting again.</p>".to_string()
                                    }
                                };
                                return Ok::<warp::http::Response<warp::hyper::Body>, warp::Rejection>(warp::reply::with_status(
                                    warp::reply::html(html_content),
                                    warp::http::StatusCode::BAD_REQUEST
                                ).into_response());
                            };
                        let Some(state) = params.get("state").cloned() else {
                                tracing::error!("Missing OAuth state parameter in callback");
                                let html_content = match Self::render_oauth_error_template(
                                    "Authorization Failed",
                                    "Missing OAuth state parameter. Please try connecting again.",
                                    None
                                ) {
                                    Ok(html) => html,
                                    Err(e) => {
                                        tracing::error!("Failed to render error template: {}", e);
                                        "<h1>Authorization Failed</h1><p>Missing OAuth state parameter. Please try connecting again.</p>".to_string()
                                    }
                                };
                                return Ok(warp::reply::with_status(
                                    warp::reply::html(html_content),
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
                                let html_content = match Self::render_oauth_success_template(&provider, &callback_response) {
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
                                let html_content = match Self::render_oauth_error_template(
                                    "Connection Failed",
                                    &format!("There was an error connecting your {} account to Pierre Fitness.", provider.to_uppercase()),
                                    Some(&e.to_string())
                                ) {
                                    Ok(html) => html,
                                    Err(template_err) => {
                                        tracing::error!("Failed to render error template: {}", template_err);
                                        format!("<h1>Error</h1><p>Failed to connect {} account: {}</p>", provider.to_uppercase(), e)
                                    }
                                };

                                Ok(warp::reply::with_status(
                                    warp::reply::html(html_content),
                                    warp::http::StatusCode::BAD_REQUEST
                                ).into_response())
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

    /// Create fitness configuration endpoint routes
    // Long function: Defines complete fitness configuration API route schema
    #[allow(clippy::too_many_lines)]
    fn create_fitness_configuration_routes(
        fitness_config_routes: &Arc<
            crate::fitness_configuration_routes::FitnessConfigurationRoutes,
        >,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

        // List fitness configurations
        let list_configs = warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let fitness_routes = fitness_config_routes.clone();
                move |auth_header: Option<String>| {
                    let fitness_routes = fitness_routes.clone();
                    async move {
                        match fitness_routes
                            .list_configurations(auth_header.as_deref())
                            .await
                        {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::OK,
                            )),
                            Err(e) => {
                                tracing::error!("List fitness configurations failed: {}", e);
                                Err(warp::reject::custom(crate::mcp::multitenant::ApiError(
                                    serde_json::json!({"error": e.to_string()}),
                                )))
                            }
                        }
                    }
                }
            });

        // Get specific fitness configuration
        let get_config = warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::get())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let fitness_routes = fitness_config_routes.clone();
                move |config_name: String, auth_header: Option<String>| {
                    let fitness_routes = fitness_routes.clone();
                    async move {
                        match fitness_routes
                            .get_configuration(auth_header.as_deref(), &config_name)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::OK,
                            )),
                            Err(e) => {
                                tracing::error!("Get fitness configuration failed: {}", e);
                                Err(warp::reject::custom(crate::mcp::multitenant::ApiError(
                                    serde_json::json!({"error": e.to_string()}),
                                )))
                            }
                        }
                    }
                }
            });

        // Save user fitness configuration
        let save_user_config = warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::end())
            .and(warp::post())
            .and(warp::header::optional::<String>("authorization"))
            .and(warp::body::json::<crate::fitness_configuration_routes::SaveFitnessConfigRequest>())
            .and_then({
                let fitness_routes = fitness_config_routes.clone();
                move |auth_header: Option<String>, request: crate::fitness_configuration_routes::SaveFitnessConfigRequest| {
                    let fitness_routes = fitness_routes.clone();
                    async move {
                        match fitness_routes
                            .save_user_configuration(auth_header.as_deref(), request)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::CREATED,
                            )),
                            Err(e) => {
                                tracing::error!("Save user fitness configuration failed: {}", e);
                                Err(warp::reject::custom(crate::mcp::multitenant::ApiError(
                                    serde_json::json!({"error": e.to_string()}),
                                )))
                            }
                        }
                    }
                }
            });

        // Delete user fitness configuration
        let delete_user_config = warp::path("api")
            .and(warp::path("fitness-configurations"))
            .and(warp::path::param::<String>())
            .and(warp::path::end())
            .and(warp::delete())
            .and(warp::header::optional::<String>("authorization"))
            .and_then({
                let fitness_routes = fitness_config_routes.clone();
                move |config_name: String, auth_header: Option<String>| {
                    let fitness_routes = fitness_routes.clone();
                    async move {
                        match fitness_routes
                            .delete_user_configuration(auth_header.as_deref(), &config_name)
                            .await
                        {
                            Ok(response) => Ok(warp::reply::with_status(
                                warp::reply::json(&response),
                                warp::http::StatusCode::OK,
                            )),
                            Err(e) => {
                                tracing::error!("Delete user fitness configuration failed: {}", e);
                                Err(warp::reject::custom(crate::mcp::multitenant::ApiError(
                                    serde_json::json!({"error": e.to_string()}),
                                )))
                            }
                        }
                    }
                }
            });

        list_configs
            .or(get_config)
            .or(save_user_config)
            .or(delete_user_config)
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
        database: Arc<Database>,
        auth_manager: Arc<AuthManager>,
    ) -> impl warp::Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        use warp::Filter;

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

        // Create notification channels for both transports using broadcast for multiple receivers
        let (notification_sender, _): (
            tokio::sync::broadcast::Sender<crate::mcp::schema::OAuthCompletedNotification>,
            tokio::sync::broadcast::Receiver<crate::mcp::schema::OAuthCompletedNotification>,
        ) = tokio::sync::broadcast::channel(100); // Buffer up to 100 notifications

        // Create separate receivers for stdio and SSE
        let notification_receiver = notification_sender.subscribe();
        let sse_notification_receiver = notification_sender.subscribe();

        // Set up notification sender in resources for OAuth callbacks
        let mut resources_clone = (*self.resources).clone();
        resources_clone.set_oauth_notification_sender(notification_sender);
        let shared_resources = Arc::new(resources_clone);

        // Clone server for both transports with shared notification resources
        let mut server_for_stdio = self.clone();
        server_for_stdio.resources = shared_resources.clone();

        let mut server_for_http = self.clone();
        server_for_http.resources = shared_resources.clone();

        let mut server_for_sse = self.clone();
        server_for_sse.resources = shared_resources.clone();

        // Start HTTP server for auth endpoints in background with shared resources (includes notification sender)
        let http_port = port + 1; // Use port+1 for HTTP
        let shared_resources_for_http = shared_resources.clone();
        tokio::spawn(async move {
            Box::pin(Self::run_http_server_with_resources(
                http_port,
                shared_resources_for_http,
            ))
            .await
        });

        // Start stdio transport in background - don't wait for it to complete
        let stdio_handle = tokio::spawn(async move {
            match server_for_stdio
                .run_stdio_transport(notification_receiver)
                .await
            {
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

        // Start SSE notification forwarder task
        let sse_manager_for_notifications = server_for_sse.resources.sse_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = server_for_sse
                .run_sse_notification_forwarder(
                    sse_notification_receiver,
                    sse_manager_for_notifications,
                )
                .await
            {
                error!("SSE notification forwarder failed: {}", e);
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

    /// Spawn background task to handle OAuth notifications via stdio
    fn spawn_oauth_notification_handler(
        mut notification_receiver: tokio::sync::broadcast::Receiver<
            crate::mcp::schema::OAuthCompletedNotification,
        >,
        stdout: Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) {
        tokio::spawn(async move {
            loop {
                match notification_receiver.recv().await {
                    Ok(notification) => {
                        Self::handle_oauth_notification(notification, &stdout).await;
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                        tracing::warn!(
                            "Stdio notification handler lagged, skipped {} notifications",
                            skipped
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        tracing::info!("Stdio notification channel closed, ending handler");
                        break;
                    }
                }
            }
        });
    }

    /// Handle a single OAuth notification by writing it to stdout
    async fn handle_oauth_notification(
        notification: crate::mcp::schema::OAuthCompletedNotification,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) {
        use tokio::io::AsyncWriteExt;

        tracing::info!(
            "Received OAuth notification for MCP client: {}",
            notification.params.provider
        );

        let notification_json = match serde_json::to_string(&notification) {
            Ok(json) => json,
            Err(e) => {
                tracing::error!("Failed to serialize OAuth notification: {}", e);
                return;
            }
        };

        {
            let mut stdout_guard = stdout.lock().await;
            if let Err(e) = stdout_guard.write_all(notification_json.as_bytes()).await {
                tracing::error!("Failed to write OAuth notification to stdout: {}", e);
                return;
            }
            if let Err(e) = stdout_guard.write_all(b"\n").await {
                tracing::error!("Failed to write newline after OAuth notification: {}", e);
                return;
            }
            if let Err(e) = stdout_guard.flush().await {
                tracing::error!("Failed to flush OAuth notification to stdout: {}", e);
                return;
            }
        } // stdout_guard is dropped here, releasing the mutex

        tracing::info!("Successfully sent OAuth notification to MCP client via stdout");
    }

    /// Write MCP response to stdout
    async fn write_response_to_stdout(
        response: &McpResponse,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let response_str = serde_json::to_string(response)
            .map_err(|e| anyhow::anyhow!("Failed to serialize response: {}", e))?;

        {
            let mut stdout_guard = stdout.lock().await;
            stdout_guard
                .write_all(response_str.as_bytes())
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write to stdout: {}", e))?;
            stdout_guard
                .write_all(b"\n")
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write newline to stdout: {}", e))?;
            stdout_guard
                .flush()
                .await
                .map_err(|e| anyhow::anyhow!("Failed to flush stdout: {}", e))?;
            drop(stdout_guard);
        }

        Ok(())
    }

    /// Process a single MCP request and send response
    async fn process_mcp_request(
        request: McpRequest,
        resources: &Arc<ServerResources>,
        stdout: &Arc<tokio::sync::Mutex<tokio::io::Stdout>>,
    ) -> Result<()> {
        tracing::debug!(
            transport = "stdio",
            mcp_method = %request.method,
            "Processing MCP request via stdio transport"
        );

        if let Some(response) = Self::handle_request(request, resources).await {
            Self::write_response_to_stdout(&response, stdout).await?;
        }

        Ok(())
    }

    /// Run MCP server using stdio transport (MCP specification compliant)
    async fn run_stdio_transport(
        self,
        notification_receiver: tokio::sync::broadcast::Receiver<
            crate::mcp::schema::OAuthCompletedNotification,
        >,
    ) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, BufReader};

        info!("MCP stdio transport ready - listening on stdin/stdout");

        let stdin = tokio::io::stdin();
        let stdout = Arc::new(tokio::sync::Mutex::new(tokio::io::stdout()));
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        // Spawn background task to handle OAuth notifications
        Self::spawn_oauth_notification_handler(notification_receiver, stdout.clone());

        while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
            let trimmed_line = line.trim();
            if trimmed_line.is_empty() {
                line.clear();
                continue;
            }

            if let Ok(request) = serde_json::from_str::<McpRequest>(trimmed_line) {
                if let Err(e) = Self::process_mcp_request(request, &self.resources, &stdout).await {
                    tracing::error!("Failed to process MCP request: {}", e);
                    break;
                }
            }
            line.clear();
        }

        info!("MCP stdio transport ended");
        Ok(())
    }

    /// Run SSE notification forwarder for OAuth notifications
    async fn run_sse_notification_forwarder(
        &self,
        mut notification_receiver: tokio::sync::broadcast::Receiver<
            crate::mcp::schema::OAuthCompletedNotification,
        >,
        sse_manager: Arc<crate::notifications::sse::SseConnectionManager>,
    ) -> Result<()> {
        info!("SSE notification forwarder ready - waiting for OAuth notifications");

        loop {
            match notification_receiver.recv().await {
                Ok(notification) => {
                    tracing::info!(
                        "Received OAuth notification for SSE delivery: {}",
                        notification.params.provider
                    );

                    // Convert OAuth notification to database format for SSE
                    let oauth_notification =
                        crate::database::oauth_notifications::OAuthNotification {
                            id: format!(
                                "oauth-{}-{}",
                                notification.params.provider,
                                chrono::Utc::now().timestamp()
                            ),
                            user_id: notification
                                .params
                                .user_id
                                .unwrap_or_else(|| "unknown".to_string()),
                            provider: notification.params.provider.clone(),
                            success: notification.params.success,
                            message: notification.params.message.clone(),
                            expires_at: None,
                            created_at: chrono::Utc::now(),
                            read_at: None,
                        };

                    // Send to all connected SSE clients for this user
                    if let Err(e) = sse_manager
                        .send_notification(&oauth_notification.user_id, &oauth_notification)
                        .await
                    {
                        tracing::warn!("Failed to send OAuth notification via SSE: {}", e);
                    } else {
                        tracing::info!(
                            "Successfully sent OAuth notification via SSE for user: {}",
                            oauth_notification.user_id
                        );
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                    tracing::warn!(
                        "SSE notification forwarder lagged, skipped {} notifications",
                        skipped
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::info!("SSE notification channel closed, ending forwarder");
                    break;
                }
            }
        }

        info!("SSE notification forwarder ended");
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

                        Self::handle_request(request, &ctx.resources)
                            .await
                            .map_or_else(
                                || {
                                    // Notification - return 204 No Content with empty body (JSON-RPC notifications expect no response)
                                    Ok(Box::new(warp::reply::with_status(
                                        warp::reply(),
                                        warp::http::StatusCode::NO_CONTENT,
                                    ))
                                        as Box<dyn warp::Reply>)
                                },
                                |response| {
                                    // Return 202 Accepted with response body for successful requests
                                    Ok(Box::new(warp::reply::with_status(
                                        warp::reply::json(&response),
                                        warp::http::StatusCode::ACCEPTED,
                                    ))
                                        as Box<dyn warp::Reply>)
                                },
                            )
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
            mcp_error_code = -32603; // Internal error - invalid origin
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
        let start_time = std::time::Instant::now();

        // Log request (with optional truncation)
        let should_truncate = std::env::var("MCP_LOG_TRUNCATE")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        if should_truncate {
            let request_summary = format!("{}(id={:?})", request.method, request.id);
            tracing::debug!(
                mcp_method = %request.method,
                mcp_id = ?request.id,
                mcp_params_preview = ?request.params.as_ref().map(|p| {
                    let s = p.to_string();
                    if s.len() > 100 { format!("{}...[truncated]", &s[..100]) } else { s }
                }),
                auth_present = request.auth_token.is_some(),
                "Received MCP request: {}",
                request_summary
            );
        } else {
            tracing::debug!(
                mcp_request = ?request,
                "Received MCP request (full)"
            );
        }

        // Handle notifications (no response needed)
        if request.method.starts_with("notifications/") {
            Self::handle_notification(&request);
            let duration = start_time.elapsed();
            tracing::debug!(
                // Safe: duration will be much less than u64::MAX milliseconds for request processing
                duration_ms = {
                    #[allow(clippy::cast_possible_truncation)]
                    {
                        duration.as_millis() as u64
                    }
                },
                "MCP notification processed"
            );
            return None;
        }

        // Handle regular requests (response needed)
        let response = match request.method.as_str() {
            "initialize" => {
                // Use OAuth-aware initialization if authentication is provided
                if request.auth_token.is_some() {
                    ProtocolHandler::handle_initialize_with_oauth(request, resources).await
                } else {
                    ProtocolHandler::handle_initialize(request)
                }
            }
            "ping" => ProtocolHandler::handle_ping(request),
            "tools/list" => ProtocolHandler::handle_tools_list(request),
            "prompts/list" => ProtocolHandler::handle_prompts_list(request),
            "resources/list" => ProtocolHandler::handle_resources_list(request, resources),
            "resources/read" => ProtocolHandler::handle_resources_read(request, resources).await,
            "authenticate" => {
                ProtocolHandler::handle_authenticate(request, &resources.auth_manager)
            }
            "tools/call" => {
                ToolHandlers::handle_tools_call_with_resources(request, resources).await
            }
            _ => ProtocolHandler::handle_unknown_method(request),
        };

        let duration = start_time.elapsed();
        let duration_ms = u64::try_from(duration.as_millis()).unwrap_or(u64::MAX);

        // Log response (with optional truncation)
        let should_truncate = std::env::var("MCP_LOG_TRUNCATE")
            .map(|v| v != "false" && v != "0")
            .unwrap_or(true);

        if should_truncate {
            let response_preview = response.result.as_ref().map(|r| {
                let s = r.to_string();
                if s.len() > 150 {
                    format!("{}...[truncated]", &s[..150])
                } else {
                    s
                }
            });
            tracing::debug!(
                mcp_id = ?response.id,
                duration_ms = duration_ms,
                success = response.error.is_none(),
                result_preview = ?response_preview,
                error = ?response.error.as_ref().map(|e| &e.message),
                "Sending MCP response"
            );
        } else {
            tracing::debug!(
                mcp_response = ?response,
                duration_ms = duration_ms,
                "Sending MCP response (full)"
            );
        }

        // Record metrics in span
        tracing::Span::current()
            .record("duration_ms", duration_ms)
            .record("response_status", response.error.is_none());

        Some(response)
    }

    /// Extract tenant context from MCP request headers
    ///
    /// # Errors
    ///
    /// Returns an error if tenant context extraction fails
    /// Get user's role in a tenant - returns error if user not found in tenant
    async fn get_user_role_for_tenant(
        database: &Arc<Database>,
        user_id: uuid::Uuid,
        tenant_id: uuid::Uuid,
    ) -> Result<TenantRole, String> {
        match database.get_user_tenant_role(user_id, tenant_id).await {
            Ok(Some(role_str)) => Ok(TenantRole::from_db_string(&role_str)),
            Ok(None) => {
                tracing::error!(
                    "User {} not found in tenant {} - access denied",
                    user_id,
                    tenant_id
                );
                Err("User has no valid tenant".to_string())
            }
            Err(e) => {
                tracing::error!("Failed to get user role for tenant {}: {}", tenant_id, e);
                Err(format!("Database error checking tenant membership: {e}"))
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
                    match Self::get_user_role_for_tenant(database, auth_result.user_id, tenant_id)
                        .await
                    {
                        Ok(role) => role,
                        Err(e) => {
                            tracing::error!("Access denied for tenant {}: {}", tenant_id, e);
                            return Err(e);
                        }
                    };
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
        eprintln!(
            "DEBUG: Extracting tenant context for user {}",
            auth_result.user_id
        );
        let user = match database.get_user(auth_result.user_id).await {
            Ok(Some(user)) => {
                eprintln!(
                    "DEBUG: Found user {} with tenant_id: {:?}",
                    user.email, user.tenant_id
                );
                user
            }
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
            eprintln!(
                "DEBUG: User {user_id} has no tenant_id",
                user_id = auth_result.user_id
            );
            return Ok(None);
        };

        eprintln!("DEBUG: Attempting to parse tenant_id as UUID: {user_tenant_id}");
        // Try parsing as UUID first
        if let Ok(tenant_uuid) = uuid::Uuid::parse_str(&user_tenant_id) {
            eprintln!("DEBUG: Successfully parsed UUID: {tenant_uuid}");
            match database.get_tenant_by_id(tenant_uuid).await {
                Ok(tenant) => {
                    eprintln!("DEBUG: Found tenant by ID: {name}", name = tenant.name);
                    tracing::debug!("Using user's tenant: {}", tenant.name);
                    eprintln!("DEBUG: Getting user role for tenant");
                    let role = match Self::get_user_role_for_tenant(
                        database,
                        auth_result.user_id,
                        tenant_uuid,
                    )
                    .await
                    {
                        Ok(role) => {
                            eprintln!("DEBUG: Got user role: {role:?}");
                            role
                        }
                        Err(e) => {
                            eprintln!("DEBUG: Failed to get user role: {e}");
                            tracing::error!("Access denied for user tenant {}: {}", tenant_uuid, e);
                            return Err(e);
                        }
                    };
                    let context =
                        TenantContext::new(tenant_uuid, tenant.name, auth_result.user_id, role);
                    tracing::debug!("Created tenant context: {:?}", context);
                    return Ok(Some(context));
                }
                Err(e) => {
                    eprintln!("DEBUG: Failed to get tenant by ID {tenant_uuid}: {e}");
                }
            }
        } else {
            eprintln!("DEBUG: Failed to parse tenant_id as UUID: {user_tenant_id}");
        }

        // Try as slug if UUID parsing failed
        if let Ok(tenant) = database.get_tenant_by_slug(&user_tenant_id).await {
            tracing::debug!("Using user's tenant slug: {}", tenant.name);
            let role = match Self::get_user_role_for_tenant(
                database,
                auth_result.user_id,
                tenant.id,
            )
            .await
            {
                Ok(role) => role,
                Err(e) => {
                    tracing::error!(
                        "Access denied for user tenant slug '{}': {}",
                        user_tenant_id,
                        e
                    );
                    return Err(e);
                }
            };
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

    /// Extract tenant context from request and auth result
    ///
    /// # Errors
    ///
    /// Returns an error if tenant context extraction fails
    pub async fn extract_tenant_context_internal(
        request: &McpRequest,
        auth_result: &AuthResult,
        database: &Arc<Database>,
    ) -> Result<Option<TenantContext>, String> {
        eprintln!(
            "DEBUG: Starting tenant context extraction for user {}",
            auth_result.user_id
        );

        // 1. Try explicit tenant from header
        if let Some(context) =
            Self::extract_tenant_from_header(request, auth_result, database).await?
        {
            eprintln!(
                "DEBUG: Found tenant from header: {name}",
                name = context.tenant_name
            );
            return Ok(Some(context));
        }

        // 2. Try user's tenant association
        if let Some(context) = Self::extract_tenant_from_user(auth_result, database).await? {
            eprintln!(
                "DEBUG: Found tenant from user association: {}",
                context.tenant_name
            );
            return Ok(Some(context));
        }

        // 3. No tenant found - return None for proper error handling
        eprintln!(
            "DEBUG: No tenant context found for user {}",
            auth_result.user_id
        );
        Ok(None)
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

    #[must_use]
    pub fn route_disconnect_tool(
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
        }
    }

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
                scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                    .split(',')
                    .map(str::to_string)
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
    // Long function: Complex transactional function that checks OAuth status, notifications, and generates comprehensive connection status response
    #[allow(clippy::too_many_lines)]
    pub async fn handle_tenant_connection_status(
        tenant_context: &TenantContext,
        tenant_oauth_client: &Arc<TenantOAuthClient>,
        database: &Arc<Database>,
        request_id: Value,
        credentials: McpOAuthCredentials<'_>,
    ) -> McpResponse {
        tracing::info!(
            "Checking connection status for tenant {} user {}",
            tenant_context.tenant_name,
            tenant_context.user_id
        );

        // Store MCP-provided OAuth credentials if supplied
        Self::store_mcp_oauth_credentials(tenant_context, tenant_oauth_client, &credentials).await;

        // Generate OAuth URLs for connecting providers
        // Using the HTTP API endpoints (port 8081) for OAuth flow
        let base_url = "http://127.0.0.1:8081/api/oauth";

        // Check actual OAuth token status from database
        // tenant_context.user_id is already a Uuid, no need to parse
        let user_id = tenant_context.user_id;

        // Get tenant_id as string for database queries
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
                    tracing::warn!("Failed to query Strava OAuth token: {}", e);
                    false
                },
                |token| {
                    let connected = token.is_some();
                    tracing::debug!("Strava token lookup result: connected={}", connected);
                    connected
                },
            );

        // Check Fitbit connection status from stored OAuth tokens
        let fitbit_connected = database
            .get_user_oauth_token(user_id, &tenant_id_str, "fitbit")
            .await
            .is_ok_and(|token| token.is_some());

        // Get unread OAuth notifications for automatic announcement
        let unread_notifications = database
            .get_unread_oauth_notifications(user_id)
            .await
            .unwrap_or_else(|e| {
                tracing::warn!("Failed to fetch unread notifications: {}", e);
                Vec::new()
            });

        // Build notifications text for announcement
        let notifications_text = if unread_notifications.is_empty() {
            String::new()
        } else {
            let mut notifications_msg = String::from("\n\n🎉 Recent OAuth Updates:\n");
            for notification in &unread_notifications {
                let status_emoji = if notification.success { "✅" } else { "❌" };
                writeln!(
                    notifications_msg,
                    "{} {}: {}",
                    status_emoji,
                    notification.provider.to_uppercase(),
                    notification.message
                )
                .unwrap_or_else(|_| tracing::warn!("Failed to write notification text"));
            }
            notifications_msg
        };

        let structured_data = serde_json::json!({
            "providers": [
                {
                    "provider": "strava",
                    "connected": strava_connected,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{}/auth/strava/{}", base_url, tenant_context.user_id),
                    "connect_instructions": if strava_connected {
                        "Your Strava account is connected and ready to use."
                    } else {
                        "Click this URL to connect your Strava account and authorize access to your fitness data."
                    }
                },
                {
                    "provider": "fitbit",
                    "connected": fitbit_connected,
                    "tenant_id": tenant_context.tenant_id,
                    "last_sync": null,
                    "connect_url": format!("{}/auth/fitbit/{}", base_url, tenant_context.user_id),
                    "connect_instructions": if fitbit_connected {
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
        });

        let strava_status = if strava_connected {
            "Connected ✅"
        } else {
            "Not Connected"
        };
        let fitbit_status = if fitbit_connected {
            "Connected ✅"
        } else {
            "Not Connected"
        };

        let text_content = format!(
            "Fitness Provider Connection Status\n\n\
            Available Providers:\n\n\
            Strava ({strava_status})\n\
            {strava_action}\n\n\
            Fitbit ({fitbit_status})\n\
            {fitbit_action}\n\n\
            {connection_instructions}{notifications_text}",
            strava_status = strava_status,
            strava_action = if strava_connected {
                "✅ Ready to use fitness tools!".to_string()
            } else {
                format!(
                    "Click to connect: {}/auth/strava/{}",
                    base_url, tenant_context.user_id
                )
            },
            fitbit_status = fitbit_status,
            fitbit_action = if fitbit_connected {
                "✅ Ready to use fitness tools!".to_string()
            } else {
                format!(
                    "Click to connect: {}/auth/fitbit/{}",
                    base_url, tenant_context.user_id
                )
            },
            connection_instructions = if !strava_connected || !fitbit_connected {
                "To connect a provider:\n\
                1. Click one of the URLs above\n\
                2. You'll be redirected to authorize access\n\
                3. Complete the OAuth flow to connect your account\n\
                4. Start using fitness tools like get_activities, get_athlete, and get_stats"
            } else {
                "All providers connected! You can now use fitness tools like get_activities, get_athlete, and get_stats."
            },
            notifications_text = notifications_text
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
            tenant_id: Some(tenant_context.tenant_id.to_string()),
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
}

/// MCP request with optional authentication token and headers
#[derive(Debug, Clone, Deserialize)]
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

impl MultiTenantMcpServer {
    /// Render OAuth success template
    fn render_oauth_success_template(
        provider: &str,
        callback_response: &crate::routes::OAuthCallbackResponse,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let template_content = std::fs::read_to_string("templates/oauth_success.html")?;

        let html = template_content
            .replace("{{provider}}", &provider.to_uppercase())
            .replace("{{user_id}}", &callback_response.user_id)
            .replace("{{expires_at}}", &callback_response.expires_at);

        Ok(html)
    }

    /// Render OAuth error template
    fn render_oauth_error_template(
        title: &str,
        message: &str,
        error: Option<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let template_content = std::fs::read_to_string("templates/oauth_error.html")?;

        let html = template_content
            .replace("{{title}}", title)
            .replace("{{message}}", message);

        let html = error.map_or_else(
            || {
                // Remove the error details section if no error
                let start = html.find("{{#if error}}").unwrap_or(0);
                let end = html.find("{{/if}}").map_or(html.len(), |i| i + 7);
                format!("{}{}", &html[..start], &html[end..])
            },
            |error_msg| {
                html.replace("{{#if error}}", "")
                    .replace("{{/if}}", "")
                    .replace("{{error}}", error_msg)
            },
        );

        Ok(html)
    }
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
