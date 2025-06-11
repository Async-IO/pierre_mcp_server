// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Multi-Tenant MCP Server
//!
//! This module provides a multi-tenant MCP server that supports user authentication,
//! secure token storage, and user-scoped data access.

use crate::auth::{AuthManager, McpAuthMiddleware};
use crate::database::Database;
use crate::models::AuthRequest;
use crate::providers::{FitnessProvider, create_provider, AuthData};
use crate::mcp::schema::InitializeResponse;
use crate::intelligence::ActivityAnalyzer;
use crate::intelligence::insights::ActivityContext;
use crate::intelligence::weather::WeatherService;
use crate::config::FitnessConfig;
use crate::routes::{AuthRoutes, OAuthRoutes, RegisterRequest, LoginRequest};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

// MCP Protocol Constants
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const JSONRPC_VERSION: &str = "2.0";

// Server Information
const SERVER_NAME: &str = "pierre-mcp-server-multitenant";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// JSON-RPC Error Codes
const ERROR_METHOD_NOT_FOUND: i32 = -32601;
const ERROR_INVALID_PARAMS: i32 = -32602;
const ERROR_INTERNAL_ERROR: i32 = -32603;
const ERROR_UNAUTHORIZED: i32 = -32000;

/// Multi-tenant MCP server supporting user authentication
pub struct MultiTenantMcpServer {
    database: Arc<Database>,
    auth_manager: Arc<AuthManager>,
    auth_middleware: Arc<McpAuthMiddleware>,
    // Per-user provider instances
    user_providers: Arc<RwLock<HashMap<String, HashMap<String, Box<dyn FitnessProvider>>>>>,
}

impl MultiTenantMcpServer {
    /// Create a new multi-tenant MCP server
    pub fn new(
        database: Database,
        auth_manager: AuthManager,
    ) -> Self {
        let auth_middleware = McpAuthMiddleware::new(auth_manager.clone());
        
        Self {
            database: Arc::new(database),
            auth_manager: Arc::new(auth_manager),
            auth_middleware: Arc::new(auth_middleware),
            user_providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Run the multi-tenant server with both HTTP and MCP endpoints
    pub async fn run(self, port: u16) -> Result<()> {
        // Create HTTP + MCP server
        info!("Starting multi-tenant server with HTTP and MCP on port {}", port);
        
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
        
        tokio::spawn(async move {
            Self::run_http_server(http_port, database_http, auth_manager_http).await
        });
        
        // Run MCP server on main port
        self.run_mcp_server(port).await
    }

    /// Run HTTP server for authentication endpoints
    async fn run_http_server(
        port: u16,
        database: Arc<Database>,
        auth_manager: Arc<AuthManager>,
    ) -> Result<()> {
        use warp::Filter;
        
        info!("HTTP authentication server starting on port {}", port);
        
        let auth_routes = AuthRoutes::new((*database).clone(), (*auth_manager).clone());
        let oauth_routes = OAuthRoutes::new((*database).clone());
        
        // CORS configuration
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST", "OPTIONS"]);
        
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
        
        // OAuth authorization URL endpoint
        let oauth_auth = warp::path("oauth")
            .and(warp::path!("auth" / String / String)) // /oauth/auth/{provider}/{user_id}
            .and(warp::get())
            .and_then({
                let oauth_routes = oauth_routes.clone();
                move |provider: String, user_id: String| {
                    let oauth_routes = oauth_routes.clone();
                    async move {
                        match oauth_routes.get_auth_url(&user_id, &provider).await {
                            Ok(url) => {
                                let response = serde_json::json!({"auth_url": url});
                                Ok(warp::reply::json(&response))
                            }
                            Err(e) => {
                                let error = serde_json::json!({"error": e.to_string()});
                                Err(warp::reject::custom(ApiError(error)))
                            }
                        }
                    }
                }
            });
        
        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| {
                warp::reply::json(&serde_json::json!({"status": "ok", "service": "pierre-mcp-server"}))
            });
        
        let routes = register
            .or(login)
            .or(oauth_auth)
            .or(health)
            .with(cors)
            .recover(handle_rejection);
        
        info!("HTTP server ready on port {}", port);
        warp::serve(routes)
            .run(([127, 0, 0, 1], port))
            .await;
            
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
                        ).await;
                        
                        let response_str = serde_json::to_string(&response).unwrap();
                        writer.write_all(response_str.as_bytes()).await.ok();
                        writer.write_all(b"\n").await.ok();
                    }
                    line.clear();
                }
            });
        }
    }

    /// Handle MCP request with authentication
    async fn handle_request(
        request: McpRequest,
        database: &Arc<Database>,
        auth_manager: &Arc<AuthManager>,
        auth_middleware: &Arc<McpAuthMiddleware>,
        user_providers: &Arc<RwLock<HashMap<String, HashMap<String, Box<dyn FitnessProvider>>>>>,
    ) -> McpResponse {
        match request.method.as_str() {
            "initialize" => {
                let init_response = InitializeResponse::new(
                    MCP_PROTOCOL_VERSION.to_string(),
                    SERVER_NAME.to_string(),
                    SERVER_VERSION.to_string(),
                );
                
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: serde_json::to_value(&init_response).ok(),
                    error: None,
                    id: request.id,
                }
            }
            "authenticate" => {
                Self::handle_authenticate(request, auth_manager).await
            }
            "tools/call" => {
                // Extract authorization header from request
                let auth_token = request.auth_token.as_deref();
                
                match auth_middleware.authenticate_request(auth_token) {
                    Ok(user_id) => {
                        // Update user's last active timestamp
                        let _ = database.update_last_active(user_id).await;
                        
                        Self::handle_authenticated_tool_call(
                            request,
                            user_id,
                            database,
                            user_providers,
                        ).await
                    }
                    Err(e) => {
                        warn!("Authentication failed: {}", e);
                        McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_UNAUTHORIZED,
                                message: "Authentication required".to_string(),
                                data: Some(serde_json::json!({"error": e.to_string()})),
                            }),
                            id: request.id,
                        }
                    }
                }
            }
            _ => {
                McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_METHOD_NOT_FOUND,
                        message: "Method not found".to_string(),
                        data: None,
                    }),
                    id: request.id,
                }
            }
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

    /// Handle authenticated tool call with user context
    async fn handle_authenticated_tool_call(
        request: McpRequest,
        user_id: Uuid,
        database: &Arc<Database>,
        user_providers: &Arc<RwLock<HashMap<String, HashMap<String, Box<dyn FitnessProvider>>>>>,
    ) -> McpResponse {
        let params = request.params.unwrap_or_default();
        let tool_name = params["name"].as_str().unwrap_or("");
        let args = &params["arguments"];
        let provider_name = args["provider"].as_str().unwrap_or("");
        
        // Get or create user-specific provider
        let provider_result = Self::get_user_provider(
            user_id,
            provider_name,
            database,
            user_providers,
        ).await;

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
        Self::execute_tool_call(tool_name, args, &provider, request.id).await
    }

    /// Get or create a user-specific provider instance
    async fn get_user_provider(
        user_id: Uuid,
        provider_name: &str,
        database: &Arc<Database>,
        user_providers: &Arc<RwLock<HashMap<String, HashMap<String, Box<dyn FitnessProvider>>>>>,
    ) -> Result<Box<dyn FitnessProvider>> {
        let user_key = user_id.to_string();
        
        // Check if provider already exists for this user
        {
            let providers_read = user_providers.read().await;
            if let Some(user_provider_map) = providers_read.get(&user_key) {
                if let Some(_provider) = user_provider_map.get(provider_name) {
                    // TODO: Clone provider - this is tricky with trait objects
                    // For now, we'll recreate the provider each time
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
                client_id: String::new(), // Will be set from config
                client_secret: String::new(), // Will be set from config
                access_token: Some(decrypted_token.access_token),
                refresh_token: Some(decrypted_token.refresh_token),
            };

            provider.authenticate(auth_data).await?;
        } else {
            return Err(anyhow::anyhow!("No valid token found for provider {}", provider_name));
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
                client_id: String::new(),
                client_secret: String::new(),
                access_token: Some(decrypted_token.access_token),
                refresh_token: Some(decrypted_token.refresh_token),
            };
            new_provider.authenticate(auth_data).await?;
        }
        
        Ok(new_provider)
    }

    /// Execute tool call with provider
    async fn execute_tool_call(
        tool_name: &str,
        args: &Value,
        provider: &Box<dyn FitnessProvider>,
        id: Value,
    ) -> McpResponse {
        let result = match tool_name {
            "get_activities" => {
                let limit = args["limit"].as_u64().map(|n| n as usize);
                let offset = args["offset"].as_u64().map(|n| n as usize);
                
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
            "get_athlete" => {
                match provider.get_athlete().await {
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
                }
            }
            "get_stats" => {
                match provider.get_stats().await {
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
                }
            }
            "get_activity_intelligence" => {
                let activity_id = args["activity_id"].as_str().unwrap_or("");
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
                                    let weather_config = fitness_config.weather_api.unwrap_or_default();
                                    let mut weather_service = WeatherService::new(weather_config);
                                    
                                    weather_service.get_weather_for_activity(
                                        activity.start_latitude,
                                        activity.start_longitude,
                                        activity.start_date
                                    ).await.unwrap_or(None)
                                } else {
                                    None
                                };
                                
                                // Get location data if requested
                                let location = if include_location && activity.start_latitude.is_some() && activity.start_longitude.is_some() {
                                    let mut location_service = crate::intelligence::location::LocationService::new();
                                    
                                    match location_service.get_location_from_coordinates(
                                        activity.start_latitude.unwrap(),
                                        activity.start_longitude.unwrap()
                                    ).await {
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
                                Ok(intelligence) => {
                                    Some(serde_json::json!({
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
                                    }))
                                }
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
                                    message: format!("Activity with ID '{}' not found", activity_id),
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

/// Handle HTTP rejections and errors
async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, std::convert::Infallible> {
    if let Some(api_error) = err.find::<ApiError>() {
        let json = warp::reply::json(&api_error.0);
        Ok(warp::reply::with_status(json, warp::http::StatusCode::BAD_REQUEST))
    } else if err.is_not_found() {
        let json = warp::reply::json(&serde_json::json!({
            "error": "Not Found",
            "message": "The requested endpoint was not found"
        }));
        Ok(warp::reply::with_status(json, warp::http::StatusCode::NOT_FOUND))
    } else {
        let json = warp::reply::json(&serde_json::json!({
            "error": "Internal Server Error",
            "message": "Something went wrong"
        }));
        Ok(warp::reply::with_status(json, warp::http::StatusCode::INTERNAL_SERVER_ERROR))
    }
}