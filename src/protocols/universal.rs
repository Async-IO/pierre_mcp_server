// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Universal Tool Execution Layer
//!
//! Provides a protocol-agnostic interface for executing tools
//! that can be called from both MCP and A2A protocols.

use crate::database::Database;
use crate::intelligence::ActivityIntelligence;
use crate::providers::{create_provider, AuthData};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
// All async operations handled natively without blocking runtime

/// Universal request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalRequest {
    pub tool_name: String,
    pub parameters: Value,
    pub user_id: String,
    pub protocol: String,
}

/// Universal response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalResponse {
    pub success: bool,
    pub result: Option<Value>,
    pub error: Option<String>,
    pub metadata: Option<HashMap<String, Value>>,
}

/// Universal tool definition
#[derive(Debug, Clone)]
pub struct UniversalTool {
    pub name: String,
    pub description: String,
    pub handler: fn(
        &UniversalToolExecutor,
        UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError>,
}

/// Universal tool executor
pub struct UniversalToolExecutor {
    pub database: Arc<Database>,
    pub intelligence: Arc<ActivityIntelligence>,
    tools: HashMap<String, UniversalTool>,
}

impl UniversalToolExecutor {
    pub fn new(database: Arc<Database>, intelligence: Arc<ActivityIntelligence>) -> Self {
        let mut executor = Self {
            database,
            intelligence,
            tools: HashMap::new(),
        };

        executor.register_default_tools();
        executor
    }

    /// Register all default tools
    fn register_default_tools(&mut self) {
        // Register sync tools
        self.register_tool(UniversalTool {
            name: "get_connection_status".to_string(),
            description: "Check provider connection status".to_string(),
            handler: Self::handle_connection_status,
        });

        self.register_tool(UniversalTool {
            name: "set_goal".to_string(),
            description: "Set a fitness goal".to_string(),
            handler: Self::handle_set_goal,
        });

        self.register_tool(UniversalTool {
            name: "connect_strava".to_string(),
            description: "Generate authorization URL to connect user's Strava account".to_string(),
            handler: Self::handle_connect_strava,
        });

        self.register_tool(UniversalTool {
            name: "connect_fitbit".to_string(),
            description: "Generate authorization URL to connect user's Fitbit account".to_string(),
            handler: Self::handle_connect_fitbit,
        });

        self.register_tool(UniversalTool {
            name: "disconnect_provider".to_string(),
            description: "Disconnect and remove stored tokens for a specific fitness provider"
                .to_string(),
            handler: Self::handle_disconnect_provider,
        });

        // Advanced analytics tools
        self.register_tool(UniversalTool {
            name: "calculate_metrics".to_string(),
            description: "Calculate advanced fitness metrics for an activity".to_string(),
            handler: Self::handle_calculate_metrics,
        });

        self.register_tool(UniversalTool {
            name: "analyze_performance_trends".to_string(),
            description: "Analyze performance trends over time".to_string(),
            handler: Self::handle_analyze_performance_trends,
        });

        self.register_tool(UniversalTool {
            name: "compare_activities".to_string(),
            description: "Compare an activity against similar activities or personal bests"
                .to_string(),
            handler: Self::handle_compare_activities,
        });

        self.register_tool(UniversalTool {
            name: "detect_patterns".to_string(),
            description: "Detect patterns in training data".to_string(),
            handler: Self::handle_detect_patterns,
        });

        self.register_tool(UniversalTool {
            name: "track_progress".to_string(),
            description: "Track progress toward a specific goal".to_string(),
            handler: Self::handle_track_progress,
        });

        self.register_tool(UniversalTool {
            name: "suggest_goals".to_string(),
            description: "Generate AI-powered goal suggestions".to_string(),
            handler: Self::handle_suggest_goals,
        });

        self.register_tool(UniversalTool {
            name: "analyze_goal_feasibility".to_string(),
            description: "Assess whether a goal is realistic and achievable".to_string(),
            handler: Self::handle_analyze_goal_feasibility,
        });

        self.register_tool(UniversalTool {
            name: "generate_recommendations".to_string(),
            description: "Generate personalized training recommendations".to_string(),
            handler: Self::handle_generate_recommendations,
        });

        self.register_tool(UniversalTool {
            name: "calculate_fitness_score".to_string(),
            description: "Calculate comprehensive fitness score".to_string(),
            handler: Self::handle_calculate_fitness_score,
        });

        self.register_tool(UniversalTool {
            name: "predict_performance".to_string(),
            description: "Predict future performance capabilities".to_string(),
            handler: Self::handle_predict_performance,
        });

        self.register_tool(UniversalTool {
            name: "analyze_training_load".to_string(),
            description: "Analyze training load balance and recovery needs".to_string(),
            handler: Self::handle_analyze_training_load,
        });
    }

    /// Register a new tool
    pub fn register_tool(&mut self, tool: UniversalTool) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Execute a tool by name
    pub async fn execute_tool(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Handle async tools that need database or API access
        match request.tool_name.as_str() {
            "get_activities" => self.handle_get_activities_async(request).await,
            "get_athlete" => self.handle_get_athlete_async(request).await,
            "get_stats" => self.handle_get_stats_async(request).await,
            "analyze_activity" => self.handle_analyze_activity_async(request).await,
            "get_activity_intelligence" => {
                self.handle_get_activity_intelligence_async(request).await
            }
            _ => {
                // Handle synchronous tools
                let tool = self.tools.get(&request.tool_name).ok_or_else(|| {
                    crate::protocols::ProtocolError::ToolNotFound(request.tool_name.clone())
                })?;
                (tool.handler)(self, request)
            }
        }
    }

    /// List available tools
    pub fn list_tools(&self) -> Vec<&UniversalTool> {
        self.tools.values().collect()
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<&UniversalTool> {
        self.tools.get(name)
    }

    /// Handle get_activities with async Strava API calls
    async fn handle_get_activities_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract parameters
        let limit = request
            .parameters
            .get("limit")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let provider_type = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("strava");

        // Get REAL Strava data
        let activities = if provider_type == "strava" {
            match uuid::Uuid::parse_str(&request.user_id) {
                Ok(user_uuid) => {
                    // Try to get Strava token from database
                    match self.database.get_strava_token(user_uuid).await {
                        Ok(Some(strava_token)) => {
                            // Create Strava provider with real token
                            match create_provider("strava") {
                                Ok(mut provider) => {
                                    let auth_data = AuthData::OAuth2 {
                                        client_id: std::env::var("STRAVA_CLIENT_ID")
                                            .unwrap_or_default(),
                                        client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                            .unwrap_or_default(),
                                        access_token: Some(strava_token.access_token),
                                        refresh_token: Some(strava_token.refresh_token),
                                    };

                                    // Authenticate and get REAL activities
                                    match provider.authenticate(auth_data).await {
                                        Ok(()) => {
                                            match provider.get_activities(Some(limit), None).await {
                                                Ok(real_activities) => {
                                                    // Convert REAL activities to JSON
                                                    real_activities.into_iter().map(|activity| {
                                                        serde_json::json!({
                                                            "id": activity.id,
                                                            "name": activity.name,
                                                            "sport_type": format!("{:?}", activity.sport_type),
                                                            "start_date": activity.start_date.to_rfc3339(),
                                                            "duration_seconds": activity.duration_seconds,
                                                            "distance_meters": activity.distance_meters.unwrap_or(0.0),
                                                            "elevation_gain": activity.elevation_gain.unwrap_or(0.0),
                                                            "average_heart_rate": activity.average_heart_rate,
                                                            "max_heart_rate": activity.max_heart_rate,
                                                            "calories": activity.calories,
                                                            "start_latitude": activity.start_latitude,
                                                            "start_longitude": activity.start_longitude,
                                                            "city": activity.city,
                                                            "country": activity.country,
                                                            "provider": "strava",
                                                            "is_real_data": true
                                                        })
                                                    }).collect()
                                                }
                                                Err(e) => {
                                                    eprintln!("Strava API call failed: {}", e);
                                                    vec![serde_json::json!({
                                                        "error": format!("Strava API call failed: {}", e),
                                                        "is_real_data": false
                                                    })]
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!("Strava authentication failed: {}", e);
                                            vec![serde_json::json!({
                                                "error": format!("Strava authentication failed: {}", e),
                                                "is_real_data": false
                                            })]
                                        }
                                    }
                                }
                                Err(e) => {
                                    eprintln!("Failed to create Strava provider: {}", e);
                                    vec![serde_json::json!({
                                        "error": format!("Failed to create Strava provider: {}", e),
                                        "is_real_data": false
                                    })]
                                }
                            }
                        }
                        Ok(None) => {
                            vec![serde_json::json!({
                                "error": "No Strava token found for user - please connect your Strava account first",
                                "is_real_data": false,
                                "note": "Connect your Strava account via the OAuth flow to get real data"
                            })]
                        }
                        Err(e) => {
                            eprintln!("Database error: {}", e);
                            vec![serde_json::json!({
                                "error": format!("Database error: {}", e),
                                "is_real_data": false
                            })]
                        }
                    }
                }
                Err(e) => {
                    vec![serde_json::json!({
                        "error": format!("Invalid user ID format: {}", e),
                        "is_real_data": false
                    })]
                }
            }
        } else {
            vec![]
        };

        let result = serde_json::json!({
            "activities": activities,
            "total_count": activities.len(),
            "provider": provider_type
        });

        Ok(UniversalResponse {
            success: true,
            result: Some(result),
            error: None,
            metadata: Some({
                let mut meta = std::collections::HashMap::new();
                meta.insert("limit".to_string(), serde_json::Value::Number(limit.into()));
                meta
            }),
        })
    }

    /// Handle get_athlete with async Strava API calls
    async fn handle_get_athlete_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Get REAL athlete data
        let athlete_data = match uuid::Uuid::parse_str(&request.user_id) {
            Ok(user_uuid) => match self.database.get_strava_token(user_uuid).await {
                Ok(Some(strava_token)) => match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(strava_token.access_token),
                            refresh_token: Some(strava_token.refresh_token),
                        };

                        match provider.authenticate(auth_data).await {
                            Ok(()) => match provider.get_athlete().await {
                                Ok(athlete) => serde_json::json!({
                                    "id": athlete.id,
                                    "username": athlete.username,
                                    "firstname": athlete.firstname,
                                    "lastname": athlete.lastname,
                                    "profile_picture": athlete.profile_picture,
                                    "provider": athlete.provider,
                                    "is_real_data": true
                                }),
                                Err(e) => serde_json::json!({
                                    "error": format!("Failed to get athlete data: {}", e),
                                    "is_real_data": false
                                }),
                            },
                            Err(e) => serde_json::json!({
                                "error": format!("Authentication failed: {}", e),
                                "is_real_data": false
                            }),
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to create provider: {}", e),
                        "is_real_data": false
                    }),
                },
                Ok(None) => serde_json::json!({
                    "error": "No Strava token found - please connect your Strava account first",
                    "is_real_data": false,
                    "note": "Connect your Strava account via the OAuth flow to get real data"
                }),
                Err(e) => serde_json::json!({
                    "error": format!("Database error: {}", e),
                    "is_real_data": false
                }),
            },
            Err(e) => serde_json::json!({
                "error": format!("Invalid user ID: {}", e),
                "is_real_data": false
            }),
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(athlete_data),
            error: None,
            metadata: None,
        })
    }

    // Legacy sync tool handlers for non-async tools

    async fn handle_analyze_activity_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "activity_id is required".to_string(),
                )
            })?;

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get real activity data async
        let activity_result = {
            // Get Strava token from database
            match self.database.get_strava_token(user_uuid).await {
                Ok(Some(strava_token)) => {
                    // Create Strava provider with real token
                    match create_provider("strava") {
                        Ok(mut provider) => {
                            let auth_data = AuthData::OAuth2 {
                                client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                                client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                    .unwrap_or_default(),
                                access_token: Some(strava_token.access_token),
                                refresh_token: Some(strava_token.refresh_token),
                            };

                            // Authenticate and get real activity
                            match provider.authenticate(auth_data).await {
                                Ok(()) => {
                                    // Get all activities and find the specific one
                                    match provider.get_activities(Some(100), None).await {
                                        Ok(activities) => {
                                            activities.into_iter().find(|a| a.id == activity_id)
                                        }
                                        Err(_) => None,
                                    }
                                }
                                Err(_) => None,
                            }
                        }
                        Err(_) => None,
                    }
                }
                _ => None,
            }
        };

        match activity_result {
            Some(activity) => {
                // Use the intelligence module for real analysis
                let intelligence = &self.intelligence;

                // Generate basic analysis
                let efficiency_score = if let Some(distance) = activity.distance_meters {
                    if activity.duration_seconds > 0 && distance > 0.0 {
                        // Simple efficiency calculation: distance/time ratio normalized
                        let speed_ms = distance / activity.duration_seconds as f64;
                        (speed_ms * 100.0).clamp(0.0, 100.0)
                    } else {
                        50.0
                    }
                } else {
                    50.0
                };

                let relative_effort = activity
                    .average_heart_rate
                    .map(|hr| (hr as f64 / 180.0) * 10.0)
                    .unwrap_or(5.0);

                let result = serde_json::json!({
                    "activity_id": activity_id,
                    "activity": {
                        "id": activity.id,
                        "name": activity.name,
                        "sport_type": format!("{:?}", activity.sport_type),
                        "duration_seconds": activity.duration_seconds,
                        "distance_meters": activity.distance_meters,
                        "average_heart_rate": activity.average_heart_rate,
                        "start_date": activity.start_date.to_rfc3339(),
                        "city": activity.city,
                        "country": activity.country
                    },
                    "analysis": {
                        "efficiency_score": efficiency_score,
                        "relative_effort": relative_effort,
                        "performance_summary": intelligence.performance_indicators.clone()
                    },
                    "insights": intelligence.key_insights.clone(),
                    "is_real_data": true
                });

                Ok(UniversalResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                    metadata: None,
                })
            }
            None => {
                let error_result = serde_json::json!({
                    "error": "Activity not found or user not connected to Strava",
                    "activity_id": activity_id,
                    "is_real_data": false
                });

                Ok(UniversalResponse {
                    success: false,
                    result: Some(error_result),
                    error: Some("Activity not found".to_string()),
                    metadata: None,
                })
            }
        }
    }

    fn handle_connection_status(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Use blocking runtime to check database for real connection status
        let database = executor.database.clone();
        let rt = tokio::runtime::Handle::current();
        let strava_connected = rt.block_on(async {
            match database.get_strava_token(user_uuid).await {
                Ok(Some(_)) => true,
                Ok(None) => false,
                Err(_) => false,
            }
        });

        // Check for other providers if needed
        let fitbit_connected = false; // No Fitbit implementation yet

        let status = serde_json::json!({
            "providers": {
                "strava": {
                    "connected": strava_connected,
                    "status": if strava_connected { "active" } else { "not_connected" }
                },
                "fitbit": {
                    "connected": fitbit_connected,
                    "status": "not_connected"
                }
            }
        });

        Ok(UniversalResponse {
            success: true,
            result: Some(status),
            error: None,
            metadata: None,
        })
    }

    fn handle_set_goal(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let goal_type = request
            .parameters
            .get("goal_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "goal_type is required".to_string(),
                )
            })?;

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "target_value is required".to_string(),
                )
            })?;

        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "timeframe is required".to_string(),
                )
            })?;

        let title = request
            .parameters
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Fitness Goal");

        let description = request
            .parameters
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Use blocking runtime to store goal in database
        let database = executor.database.clone();
        let rt = tokio::runtime::Handle::current();

        let goal_result = rt.block_on(async {
            let goal_data = serde_json::json!({
                "title": title,
                "description": description,
                "goal_type": goal_type,
                "target_value": target_value,
                "timeframe": timeframe,
                "status": "active",
                "current_value": 0.0
            });
            database.create_goal(user_uuid, goal_data).await
        });

        match goal_result {
            Ok(goal_id) => {
                let result = serde_json::json!({
                    "goal_id": goal_id,
                    "title": title,
                    "description": description,
                    "goal_type": goal_type,
                    "target_value": target_value,
                    "timeframe": timeframe,
                    "status": "active",
                    "progress": 0.0,
                    "created_at": chrono::Utc::now().to_rfc3339(),
                    "is_real_data": true
                });

                Ok(UniversalResponse {
                    success: true,
                    result: Some(result),
                    error: None,
                    metadata: None,
                })
            }
            Err(e) => {
                let error_result = serde_json::json!({
                    "error": format!("Failed to create goal: {}", e),
                    "is_real_data": false
                });

                Ok(UniversalResponse {
                    success: false,
                    result: Some(error_result),
                    error: Some(format!("Database error: {}", e)),
                    metadata: None,
                })
            }
        }
    }

    // Async handlers for tools that need API access
    async fn handle_get_stats_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let provider_type = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("strava");

        // Get REAL stats from the provider
        let stats = match uuid::Uuid::parse_str(&request.user_id) {
            Ok(user_uuid) => match self.database.get_strava_token(user_uuid).await {
                Ok(Some(strava_token)) => match create_provider(provider_type) {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(strava_token.access_token),
                            refresh_token: Some(strava_token.refresh_token),
                        };

                        match provider.authenticate(auth_data).await {
                            Ok(()) => match provider.get_stats().await {
                                Ok(stats) => serde_json::to_value(&stats).unwrap_or_else(|_| {
                                    serde_json::json!({
                                        "error": "Failed to serialize stats",
                                        "is_real_data": false
                                    })
                                }),
                                Err(e) => serde_json::json!({
                                    "error": format!("Failed to get stats: {}", e),
                                    "is_real_data": false
                                }),
                            },
                            Err(e) => serde_json::json!({
                                "error": format!("Authentication failed: {}", e),
                                "is_real_data": false
                            }),
                        }
                    }
                    Err(e) => serde_json::json!({
                        "error": format!("Failed to create provider: {}", e),
                        "is_real_data": false
                    }),
                },
                Ok(None) => serde_json::json!({
                    "error": "No provider token found - please connect your account first",
                    "is_real_data": false
                }),
                Err(e) => serde_json::json!({
                    "error": format!("Database error: {}", e),
                    "is_real_data": false
                }),
            },
            Err(e) => serde_json::json!({
                "error": format!("Invalid user ID: {}", e),
                "is_real_data": false
            }),
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(stats),
            error: None,
            metadata: None,
        })
    }
    /// Handle get_activity_intelligence tool (async)
    async fn handle_get_activity_intelligence_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract activity_id from parameters
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Missing required parameter: activity_id".to_string(),
                )
            })?;

        // Activity intelligence analysis is not yet fully implemented
        // This would require implementing database methods for activity retrieval
        // and intelligence methods for analysis
        Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some("Activity intelligence analysis is not yet fully implemented. Database methods for activity retrieval are required.".to_string()),
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert("analysis_timestamp".to_string(), serde_json::Value::String(chrono::Utc::now().to_rfc3339()));
                map.insert("requested_activity_id".to_string(), serde_json::Value::String(activity_id.to_string()));
                map
            }),
        })
    }

    /// Handle connect_strava tool
    fn handle_connect_strava(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Strava connection requires OAuth flow which cannot be completed
        // directly through a tool call. Return authorization URL instead.
        let client_id = std::env::var("STRAVA_CLIENT_ID").map_err(|_| {
            crate::protocols::ProtocolError::ConfigurationError(
                "STRAVA_CLIENT_ID environment variable not set".to_string(),
            )
        })?;

        let redirect_uri = std::env::var("STRAVA_REDIRECT_URI").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}/oauth/callback/strava",
                crate::constants::ports::DEFAULT_HTTP_PORT
            )
        });

        let scope = "read,activity:read_all";
        let state = uuid::Uuid::new_v4().to_string();

        let auth_url = format!(
            "https://www.strava.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            client_id, redirect_uri, scope, state
        );

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "authorization_url": auth_url,
                "instructions": "Visit the authorization URL to connect your Strava account. Complete the OAuth flow through your web browser.",
                "state": state,
                "next_step": "After authorization, use the returned code to complete the connection process"
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "protocol".to_string(),
                    serde_json::Value::String(request.protocol),
                );
                map.insert(
                    "requires_browser".to_string(),
                    serde_json::Value::Bool(true),
                );
                map.insert(
                    "oauth_provider".to_string(),
                    serde_json::Value::String("strava".to_string()),
                );
                map
            }),
        })
    }

    /// Handle connect_fitbit tool
    fn handle_connect_fitbit(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Fitbit connection requires OAuth flow which cannot be completed
        // directly through a tool call. Return authorization URL instead.
        let client_id = std::env::var("FITBIT_CLIENT_ID").map_err(|_| {
            crate::protocols::ProtocolError::ConfigurationError(
                "FITBIT_CLIENT_ID environment variable not set".to_string(),
            )
        })?;

        let redirect_uri = std::env::var("FITBIT_REDIRECT_URI").unwrap_or_else(|_| {
            format!(
                "http://localhost:{}/oauth/callback/fitbit",
                crate::constants::ports::DEFAULT_HTTP_PORT
            )
        });

        let scope = "activity heartrate location nutrition profile settings sleep social weight";
        let state = uuid::Uuid::new_v4().to_string();

        let auth_url = format!(
            "https://www.fitbit.com/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}",
            client_id, redirect_uri, scope, state
        );

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "authorization_url": auth_url,
                "instructions": "Visit the authorization URL to connect your Fitbit account. Complete the OAuth flow through your web browser.",
                "state": state,
                "next_step": "After authorization, use the returned code to complete the connection process"
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "protocol".to_string(),
                    serde_json::Value::String(request.protocol),
                );
                map.insert(
                    "requires_browser".to_string(),
                    serde_json::Value::Bool(true),
                );
                map.insert(
                    "oauth_provider".to_string(),
                    serde_json::Value::String("fitbit".to_string()),
                );
                map
            }),
        })
    }

    /// Handle disconnect_provider tool
    fn handle_disconnect_provider(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Provider disconnection not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle calculate_metrics tool
    fn handle_calculate_metrics(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Metrics calculation not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle analyze_performance_trends tool
    fn handle_analyze_performance_trends(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Performance trend analysis not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle compare_activities tool
    fn handle_compare_activities(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Activity comparison not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle detect_patterns tool
    fn handle_detect_patterns(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Pattern detection not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle track_progress tool
    fn handle_track_progress(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Progress tracking not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle suggest_goals tool
    fn handle_suggest_goals(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Goal suggestions not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle analyze_goal_feasibility tool
    fn handle_analyze_goal_feasibility(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Goal feasibility analysis not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle generate_recommendations tool
    fn handle_generate_recommendations(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Recommendation generation not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle calculate_fitness_score tool
    fn handle_calculate_fitness_score(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Fitness score calculation not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle predict_performance tool
    fn handle_predict_performance(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Performance prediction not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle analyze_training_load tool
    fn handle_analyze_training_load(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "message": "Training load analysis not yet implemented in Universal Tool Executor",
                "tool": request.tool_name,
                "parameters": request.parameters
            })),
            error: None,
            metadata: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::Database;

    async fn create_test_executor() -> UniversalToolExecutor {
        // Use in-memory database for tests to avoid file system issues
        let database = Arc::new(Database::new(":memory:", vec![0u8; 32]).await.unwrap());
        let intelligence = Arc::new(ActivityIntelligence::new(
            "Test intelligence".to_string(),
            vec![],
            crate::intelligence::PerformanceMetrics {
                relative_effort: Some(7.5),
                zone_distribution: None,
                personal_records: vec![],
                efficiency_score: Some(75.0),
                trend_indicators: crate::intelligence::TrendIndicators {
                    pace_trend: crate::intelligence::TrendDirection::Stable,
                    effort_trend: crate::intelligence::TrendDirection::Improving,
                    distance_trend: crate::intelligence::TrendDirection::Stable,
                    consistency_score: 85.0,
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

        UniversalToolExecutor::new(database, intelligence)
    }

    #[tokio::test]
    async fn test_tool_registration() {
        let executor = create_test_executor().await;

        // Should have default tools registered
        assert!(executor.get_tool("get_connection_status").is_some());
        assert!(executor.get_tool("set_goal").is_some());
        // analyze_activity is handled async in execute_tool method

        let tools = executor.list_tools();
        assert_eq!(tools.len(), 16); // All sync tools registered, plus async tools handled in execute_tool
    }

    #[tokio::test]
    async fn test_get_activities_tool() {
        let executor = create_test_executor().await;

        let request = UniversalRequest {
            tool_name: "get_activities".to_string(),
            parameters: serde_json::json!({"limit": 5}),
            user_id: uuid::Uuid::new_v4().to_string(),
            protocol: "a2a".to_string(),
        };

        let response = executor.execute_tool(request).await.unwrap();
        assert!(response.success);
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert!(result.get("activities").is_some());
        assert!(result.get("total_count").is_some());
    }

    #[tokio::test]
    async fn test_analyze_activity_tool() {
        let executor = create_test_executor().await;

        let request = UniversalRequest {
            tool_name: "analyze_activity".to_string(),
            parameters: serde_json::json!({"activity_id": "123456"}),
            user_id: uuid::Uuid::new_v4().to_string(),
            protocol: "a2a".to_string(),
        };

        let response = executor.execute_tool(request).await.unwrap();

        // In test environment, we don't have a real Strava token, so the tool should return an error response
        // but the execution itself should succeed
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        // Should have an error about no Strava connection since we're in test mode
        // OR have insights if the mock works
        assert!(result.get("error").is_some() || result.get("insights").is_some());
    }

    #[tokio::test]
    async fn test_invalid_tool() {
        let executor = create_test_executor().await;

        let request = UniversalRequest {
            tool_name: "nonexistent_tool".to_string(),
            parameters: serde_json::json!({}),
            user_id: uuid::Uuid::new_v4().to_string(),
            protocol: "a2a".to_string(),
        };

        let result = executor.execute_tool(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::protocols::ProtocolError::ToolNotFound(_)
        ));
    }

    #[tokio::test]
    async fn test_invalid_parameters() {
        let executor = create_test_executor().await;

        let request = UniversalRequest {
            tool_name: "analyze_activity".to_string(),
            parameters: serde_json::json!({}), // Missing required activity_id
            user_id: uuid::Uuid::new_v4().to_string(),
            protocol: "a2a".to_string(),
        };

        let result = executor.execute_tool(request).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::protocols::ProtocolError::InvalidParameters(_)
        ));
    }
}
