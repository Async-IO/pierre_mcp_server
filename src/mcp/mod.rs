// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

pub mod schema;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::config::{Config, FitnessConfig};
use crate::providers::{FitnessProvider, create_provider, AuthData};
use crate::mcp::schema::InitializeResponse;
use crate::intelligence::ActivityAnalyzer;
use crate::intelligence::insights::ActivityContext;
use crate::intelligence::weather::WeatherService;

// MCP Protocol Constants
const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
const JSONRPC_VERSION: &str = "2.0";

// Server Information
const SERVER_NAME: &str = "pierre-mcp-server";
const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

// JSON-RPC Error Codes (as defined in the JSON-RPC 2.0 specification)
const ERROR_METHOD_NOT_FOUND: i32 = -32601;
const ERROR_INVALID_PARAMS: i32 = -32602;
const ERROR_INTERNAL_ERROR: i32 = -32603;


pub struct McpServer {
    config: Config,
    providers: Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
}

impl McpServer {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            providers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn run(self, port: u16) -> Result<()> {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpListener;
        
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        info!("MCP server listening on port {}", port);
        
        loop {
            let (socket, addr) = listener.accept().await?;
            info!("New connection from {}", addr);
            
            let providers = self.providers.clone();
            let config = self.config.clone();
            
            tokio::spawn(async move {
                let (reader, mut writer) = socket.into_split();
                let mut reader = BufReader::new(reader);
                let mut line = String::new();
                
                while reader.read_line(&mut line).await.unwrap_or(0) > 0 {
                    if let Ok(request) = serde_json::from_str::<McpRequest>(&line) {
                        let response = handle_request(request, &providers, &config).await;
                        let response_str = serde_json::to_string(&response).unwrap();
                        writer.write_all(response_str.as_bytes()).await.ok();
                        writer.write_all(b"\n").await.ok();
                    }
                    line.clear();
                }
            });
        }
    }
}

#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    params: Option<Value>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<McpError>,
    id: Value,
}

#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
    data: Option<Value>,
}

async fn handle_request(
    request: McpRequest,
    providers: &Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
    config: &Config,
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
        "tools/call" => {
            let params = request.params.unwrap_or_default();
            let tool_name = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];
            
            handle_tool_call(tool_name, args, providers, config, request.id).await
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

async fn handle_tool_call(
    tool_name: &str,
    args: &Value,
    providers: &Arc<RwLock<HashMap<String, Box<dyn FitnessProvider>>>>,
    config: &Config,
    id: Value,
) -> McpResponse {
    let provider_name = args["provider"].as_str().unwrap_or("");
    
    let mut providers_write = providers.write().await;
    if !providers_write.contains_key(provider_name) {
        match create_provider(provider_name) {
            Ok(mut provider) => {
                if let Some(auth_config) = config.providers.get(provider_name) {
                    let auth_data = match &auth_config.auth_type[..] {
                        "oauth2" => AuthData::OAuth2 {
                            client_id: auth_config.client_id.clone().unwrap_or_default(),
                            client_secret: auth_config.client_secret.clone().unwrap_or_default(),
                            access_token: auth_config.access_token.clone(),
                            refresh_token: auth_config.refresh_token.clone(),
                        },
                        "api_key" => AuthData::ApiKey(auth_config.api_key.clone().unwrap_or_default()),
                        _ => {
                            return McpResponse {
                                jsonrpc: JSONRPC_VERSION.to_string(),
                                result: None,
                                error: Some(McpError {
                                    code: ERROR_INVALID_PARAMS,
                                    message: "Invalid auth configuration".to_string(),
                                    data: None,
                                }),
                                id,
                            };
                        }
                    };
                    
                    if let Err(e) = provider.authenticate(auth_data).await {
                        return McpResponse {
                            jsonrpc: JSONRPC_VERSION.to_string(),
                            result: None,
                            error: Some(McpError {
                                code: ERROR_INTERNAL_ERROR,
                                message: format!("Authentication failed: {}", e),
                                data: None,
                            }),
                            id,
                        };
                    }
                }
                providers_write.insert(provider_name.to_string(), provider);
            }
            Err(e) => {
                return McpResponse {
                    jsonrpc: JSONRPC_VERSION.to_string(),
                    result: None,
                    error: Some(McpError {
                        code: ERROR_INVALID_PARAMS,
                        message: format!("Invalid provider: {}", e),
                        data: None,
                    }),
                    id,
                };
            }
        }
    }
    drop(providers_write);
    
    let providers_read = providers.read().await;
    let provider = providers_read.get(provider_name).unwrap();
    
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
                            // Load weather configuration from fitness config
                            let fitness_config = FitnessConfig::load(None).unwrap_or_default();
                            
                            // Get weather data if requested
                            let weather = if include_weather {
                                let weather_config = fitness_config.weather_api.unwrap_or_default();
                                let mut weather_service = WeatherService::new(weather_config);
                                
                                // Try to get real weather data for the activity
                                weather_service.get_weather_for_activity(
                                    activity.start_latitude,
                                    activity.start_longitude,
                                    activity.start_date
                                ).await.unwrap_or(None)
                            } else {
                                None
                            };
                            
                            // Get location data if requested and GPS coordinates are available
                            let location = if include_location && activity.start_latitude.is_some() && activity.start_longitude.is_some() {
                                tracing::info!("Getting location data for coordinates: {:.6}, {:.6}", 
                                    activity.start_latitude.unwrap(), activity.start_longitude.unwrap());
                                
                                let mut location_service = crate::intelligence::location::LocationService::new();
                                
                                match location_service.get_location_from_coordinates(
                                    activity.start_latitude.unwrap(),
                                    activity.start_longitude.unwrap()
                                ).await {
                                    Ok(location_data) => {
                                        tracing::info!("Location data retrieved: {}", location_data.display_name);
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
                                        tracing::warn!("Failed to get location data: {}", e);
                                        None
                                    }
                                }
                            } else {
                                if include_location {
                                    tracing::info!("Location requested but no GPS coordinates available");
                                }
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
        _ => None,
    };
    
    McpResponse {
        jsonrpc: JSONRPC_VERSION.to_string(),
        result: result.clone(),
        error: if result.is_none() {
            Some(McpError {
                code: ERROR_METHOD_NOT_FOUND,
                message: "Unknown tool".to_string(),
                data: None,
            })
        } else {
            None
        },
        id,
    }
}