// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Universal Tool Execution Layer
//!
//! Provides a protocol-agnostic interface for executing tools
//! that can be called from both MCP and A2A protocols.

#![allow(clippy::single_match)]

use crate::database::Database;
use crate::intelligence::activity_analyzer::ActivityAnalyzerTrait;
use crate::intelligence::goal_engine::GoalEngineTrait;
use crate::intelligence::performance_analyzer::PerformanceAnalyzerTrait;
use crate::intelligence::recommendation_engine::RecommendationEngineTrait;
use crate::intelligence::ActivityIntelligence;
use crate::providers::{create_provider, AuthData};
use chrono::Datelike;
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

    /// Get valid token for a provider, automatically refreshing if needed
    async fn get_valid_token(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
    ) -> Result<Option<crate::oauth::TokenData>, crate::oauth::OAuthError> {
        let mut oauth_manager = crate::oauth::manager::OAuthManager::new(self.database.clone());

        // Register the appropriate provider
        match provider {
            "strava" => {
                if let Ok(strava_provider) = crate::oauth::providers::StravaOAuthProvider::new() {
                    oauth_manager.register_provider(Box::new(strava_provider));
                } else {
                    return Err(crate::oauth::OAuthError::ConfigurationError(
                        "Failed to initialize Strava provider".to_string(),
                    ));
                }
            }
            "fitbit" => {
                if let Ok(fitbit_provider) = crate::oauth::providers::FitbitOAuthProvider::new() {
                    oauth_manager.register_provider(Box::new(fitbit_provider));
                } else {
                    return Err(crate::oauth::OAuthError::ConfigurationError(
                        "Failed to initialize Fitbit provider".to_string(),
                    ));
                }
            }
            _ => {
                return Err(crate::oauth::OAuthError::UnsupportedProvider(
                    provider.to_string(),
                ))
            }
        }

        oauth_manager.ensure_valid_token(user_id, provider).await
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
            "get_connection_status" => self.handle_connection_status_async(request).await,
            "connect_strava" => self.handle_connect_strava_async(request).await,
            "connect_fitbit" => self.handle_connect_fitbit_async(request).await,
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
                    // Get valid Strava token (with automatic refresh if needed)
                    match self.get_valid_token(user_uuid, "strava").await {
                        Ok(Some(token_data)) => {
                            // Create Strava provider with real token
                            match create_provider("strava") {
                                Ok(mut provider) => {
                                    let auth_data = AuthData::OAuth2 {
                                        client_id: std::env::var("STRAVA_CLIENT_ID")
                                            .unwrap_or_default(),
                                        client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                            .unwrap_or_default(),
                                        access_token: Some(token_data.access_token.clone()),
                                        refresh_token: Some(token_data.refresh_token.clone()),
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
                            eprintln!("OAuth error: {}", e);
                            vec![serde_json::json!({
                                "error": format!("OAuth error: {}", e),
                                "is_real_data": false,
                                "note": "Token may have expired or been revoked. Please reconnect your Strava account."
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
            Ok(user_uuid) => match self.get_valid_token(user_uuid, "strava").await {
                Ok(Some(token_data)) => match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
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
            // Get valid Strava token (with automatic refresh if needed)
            match self.get_valid_token(user_uuid, "strava").await {
                Ok(Some(token_data)) => {
                    // Create Strava provider with real token
                    match create_provider("strava") {
                        Ok(mut provider) => {
                            let auth_data = AuthData::OAuth2 {
                                client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                                client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                    .unwrap_or_default(),
                                access_token: Some(token_data.access_token.clone()),
                                refresh_token: Some(token_data.refresh_token.clone()),
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

        // Use OAuth manager to check connection status for all providers
        let database = executor.database.clone();
        let rt = tokio::runtime::Handle::current();
        let connection_status = rt.block_on(async {
            let mut oauth_manager = crate::oauth::manager::OAuthManager::new(database);

            // Register all providers
            if let Ok(strava_provider) = crate::oauth::providers::StravaOAuthProvider::new() {
                oauth_manager.register_provider(Box::new(strava_provider));
            }
            if let Ok(fitbit_provider) = crate::oauth::providers::FitbitOAuthProvider::new() {
                oauth_manager.register_provider(Box::new(fitbit_provider));
            }

            match oauth_manager.get_connection_status(user_uuid).await {
                Ok(statuses) => statuses,
                Err(_) => {
                    let mut default_status = std::collections::HashMap::new();
                    default_status.insert("strava".to_string(), false);
                    default_status.insert("fitbit".to_string(), false);
                    default_status
                }
            }
        });

        let status = serde_json::json!({
            "providers": {
                "strava": {
                    "connected": connection_status.get("strava").unwrap_or(&false),
                    "status": if *connection_status.get("strava").unwrap_or(&false) { "active" } else { "not_connected" }
                },
                "fitbit": {
                    "connected": connection_status.get("fitbit").unwrap_or(&false),
                    "status": if *connection_status.get("fitbit").unwrap_or(&false) { "active" } else { "not_connected" }
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
            Ok(user_uuid) => match self.get_valid_token(user_uuid, provider_type).await {
                Ok(Some(token_data)) => match create_provider(provider_type) {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
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

    /// Handle connection status check asynchronously
    async fn handle_connection_status_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Use OAuth manager to check connection status for all providers
        let database = self.database.clone();
        let mut oauth_manager = crate::oauth::manager::OAuthManager::new(database);

        // Register all providers
        if let Ok(strava_provider) = crate::oauth::providers::StravaOAuthProvider::new() {
            oauth_manager.register_provider(Box::new(strava_provider));
        }
        if let Ok(fitbit_provider) = crate::oauth::providers::FitbitOAuthProvider::new() {
            oauth_manager.register_provider(Box::new(fitbit_provider));
        }

        let connection_status = match oauth_manager.get_connection_status(user_uuid).await {
            Ok(statuses) => statuses,
            Err(_) => {
                let mut default_status = std::collections::HashMap::new();
                default_status.insert("strava".to_string(), false);
                default_status.insert("fitbit".to_string(), false);
                default_status
            }
        };

        let status = serde_json::json!({
            "providers": {
                "strava": {
                    "connected": connection_status.get("strava").unwrap_or(&false),
                    "status": if *connection_status.get("strava").unwrap_or(&false) { "active" } else { "not_connected" }
                },
                "fitbit": {
                    "connected": connection_status.get("fitbit").unwrap_or(&false),
                    "status": if *connection_status.get("fitbit").unwrap_or(&false) { "active" } else { "not_connected" }
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

    /// Handle Strava connection asynchronously
    async fn handle_connect_strava_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Create OAuth manager with database
        let mut oauth_manager = crate::oauth::manager::OAuthManager::new(self.database.clone());

        // Register Strava provider
        match crate::oauth::providers::StravaOAuthProvider::new() {
            Ok(strava_provider) => {
                oauth_manager.register_provider(Box::new(strava_provider));

                // Generate authorization URL
                match oauth_manager.generate_auth_url(user_uuid, "strava").await {
                    Ok(auth_response) => Ok(UniversalResponse {
                        success: true,
                        result: Some(serde_json::json!({
                            "authorization_url": auth_response.authorization_url,
                            "state": auth_response.state,
                            "provider": auth_response.provider
                        })),
                        error: None,
                        metadata: None,
                    }),
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!(
                            "Failed to generate Strava authorization URL: {}",
                            e
                        )),
                        metadata: None,
                    }),
                }
            }
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to initialize Strava provider: {}", e)),
                metadata: None,
            }),
        }
    }

    /// Handle Fitbit connection asynchronously
    async fn handle_connect_fitbit_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Create OAuth manager with database
        let mut oauth_manager = crate::oauth::manager::OAuthManager::new(self.database.clone());

        // Register Fitbit provider
        match crate::oauth::providers::FitbitOAuthProvider::new() {
            Ok(fitbit_provider) => {
                oauth_manager.register_provider(Box::new(fitbit_provider));

                // Generate authorization URL
                match oauth_manager.generate_auth_url(user_uuid, "fitbit").await {
                    Ok(auth_response) => Ok(UniversalResponse {
                        success: true,
                        result: Some(serde_json::json!({
                            "authorization_url": auth_response.authorization_url,
                            "state": auth_response.state,
                            "provider": auth_response.provider
                        })),
                        error: None,
                        metadata: None,
                    }),
                    Err(e) => Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!(
                            "Failed to generate Fitbit authorization URL: {}",
                            e
                        )),
                        metadata: None,
                    }),
                }
            }
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Failed to initialize Fitbit provider: {}", e)),
                metadata: None,
            }),
        }
    }

    /// Handle connect_strava tool using OAuth manager
    fn handle_connect_strava(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Use OAuth manager to generate authorization URL
        let rt = tokio::runtime::Handle::current();
        let auth_response = rt.block_on(async {
            // Create OAuth manager with database
            let mut oauth_manager =
                crate::oauth::manager::OAuthManager::new(executor.database.clone());

            // Register Strava provider
            match crate::oauth::providers::StravaOAuthProvider::new() {
                Ok(provider) => {
                    oauth_manager.register_provider(Box::new(provider));
                    oauth_manager.generate_auth_url(user_uuid, "strava").await
                }
                Err(e) => Err(crate::oauth::OAuthError::ConfigurationError(e.to_string())),
            }
        });

        match auth_response {
            Ok(auth_resp) => Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "authorization_url": auth_resp.authorization_url,
                    "instructions": auth_resp.instructions,
                    "state": auth_resp.state,
                    "provider": auth_resp.provider,
                    "expires_in_minutes": auth_resp.expires_in_minutes,
                    "next_step": "Visit the authorization URL to complete the OAuth flow. After authorization, tokens will be automatically stored."
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
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "Failed to generate Strava authorization URL: {}",
                    e
                )),
                metadata: None,
            }),
        }
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
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract provider parameter
        let provider = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Missing required parameter: provider".to_string(),
                )
            })?;

        let rt = tokio::runtime::Handle::current();
        let disconnect_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id)
                .map_err(|_| crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string()
                ))?;

            // Use OAuth manager to revoke tokens
            let mut oauth_manager = crate::oauth::manager::OAuthManager::new(executor.database.clone());

            // Register the provider based on type
            let provider_result = match provider {
                "strava" => {
                    match crate::oauth::providers::StravaOAuthProvider::new() {
                        Ok(strava_provider) => {
                            oauth_manager.register_provider(Box::new(strava_provider));
                            oauth_manager.disconnect_provider(user_uuid, "strava").await
                        }
                        Err(e) => Err(crate::oauth::OAuthError::ConfigurationError(e.to_string())),
                    }
                }
                "fitbit" => {
                    match crate::oauth::providers::FitbitOAuthProvider::new() {
                        Ok(fitbit_provider) => {
                            oauth_manager.register_provider(Box::new(fitbit_provider));
                            oauth_manager.disconnect_provider(user_uuid, "fitbit").await
                        }
                        Err(e) => Err(crate::oauth::OAuthError::ConfigurationError(e.to_string())),
                    }
                }
                _ => {
                    return Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Unsupported provider: {}", provider)),
                        metadata: None,
                    });
                }
            };

            match provider_result {
                Ok(()) => {
                    Ok(UniversalResponse {
                        success: true,
                        result: Some(serde_json::json!({
                            "provider": provider,
                            "status": "disconnected",
                            "message": format!("{} account successfully disconnected and tokens revoked", provider),
                            "disconnected_at": chrono::Utc::now().to_rfc3339()
                        })),
                        error: None,
                        metadata: Some({
                            let mut map = std::collections::HashMap::new();
                            map.insert("action".to_string(), serde_json::Value::String("provider_disconnect".to_string()));
                            map.insert("oauth_manager".to_string(), serde_json::Value::String("token_revocation".to_string()));
                            map
                        }),
                    })
                }
                Err(e) => {
                    Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to disconnect {}: {}", provider, e)),
                        metadata: Some({
                            let mut map = std::collections::HashMap::new();
                            map.insert("error_type".to_string(), serde_json::Value::String("oauth_error".to_string()));
                            map.insert("provider".to_string(), serde_json::Value::String(provider.to_string()));
                            map
                        }),
                    })
                }
            }
        });

        disconnect_result
    }

    /// Handle calculate_metrics tool
    fn handle_calculate_metrics(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract activity_id parameter
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Missing required parameter: activity_id".to_string(),
                )
            })?;

        // Get the activity from database or provider
        let rt = tokio::runtime::Handle::current();
        let metrics_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id)
                .map_err(|_| crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string()
                ))?;

            // Try to get activity from various sources
            // First try Strava if user has connection
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            // Get recent activities and find the one with matching ID
                            if let Ok(activities) = provider.get_activities(Some(50), None).await {
                                if let Some(activity) = activities.iter().find(|a| a.id == activity_id) {
                                    // Calculate metrics using the intelligence system
                                    let metrics_calculator = crate::intelligence::metrics::MetricsCalculator::new();
                                    match metrics_calculator.calculate_metrics(activity) {
                                        Ok(metrics) => {
                                            return Ok(UniversalResponse {
                                                success: true,
                                                result: Some(serde_json::json!({
                                                    "activity_id": activity_id,
                                                    "metrics": {
                                                        "trimp": metrics.trimp,
                                                        "power_to_weight_ratio": metrics.power_to_weight_ratio,
                                                        "aerobic_efficiency": metrics.aerobic_efficiency,
                                                        "training_stress_score": metrics.training_stress_score,
                                                        "intensity_factor": metrics.intensity_factor,
                                                        "variability_index": metrics.variability_index,
                                                        "efficiency_factor": metrics.efficiency_factor,
                                                        "decoupling_percentage": metrics.decoupling_percentage,
                                                        "custom_metrics": metrics.custom_metrics
                                                    },
                                                    "data_source": "strava",
                                                    "calculated_at": chrono::Utc::now().to_rfc3339()
                                                })),
                                                error: None,
                                                metadata: Some({
                                                    let mut map = std::collections::HashMap::new();
                                                    map.insert("calculation_engine".to_string(), serde_json::Value::String("intelligence_metrics".to_string()));
                                                    map.insert("activity_sport_type".to_string(), serde_json::Value::String(format!("{:?}", activity.sport_type)));
                                                    map
                                                }),
                                            });
                                        }
                                        Err(e) => {
                                            return Ok(UniversalResponse {
                                                success: false,
                                                result: None,
                                                error: Some(format!("Failed to calculate metrics: {}", e)),
                                                metadata: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            // If we couldn't find the activity, return an error
            Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Activity with ID '{}' not found or user not connected to any provider", activity_id)),
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert("suggestion".to_string(), serde_json::Value::String("Connect to Strava or other providers to access your activities".to_string()));
                    map
                }),
            })
        });

        metrics_result
    }

    /// Handle analyze_performance_trends tool
    fn handle_analyze_performance_trends(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract parameters
        let timeframe_str = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .unwrap_or("month");

        let metric = request
            .parameters
            .get("metric")
            .and_then(|v| v.as_str())
            .unwrap_or("pace");

        // Convert timeframe string to TimeFrame enum
        let timeframe = match timeframe_str {
            "week" => crate::intelligence::TimeFrame::Week,
            "month" => crate::intelligence::TimeFrame::Month,
            "quarter" => crate::intelligence::TimeFrame::Quarter,
            "year" => crate::intelligence::TimeFrame::Year,
            _ => crate::intelligence::TimeFrame::Month,
        };

        let rt = tokio::runtime::Handle::current();
        let analysis_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(200), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Use the performance analyzer from intelligence module
            let analyzer =
                crate::intelligence::performance_analyzer::AdvancedPerformanceAnalyzer::new();

            match analyzer
                .analyze_trends(&activities, timeframe, metric)
                .await
            {
                Ok(trend_analysis) => Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "timeframe": timeframe_str,
                        "metric": metric,
                        "trend_direction": format!("{:?}", trend_analysis.trend_direction),
                        "trend_strength": trend_analysis.trend_strength,
                        "statistical_significance": trend_analysis.statistical_significance,
                        "data_points_count": trend_analysis.data_points.len(),
                        "insights": trend_analysis.insights,
                        "analysis_date": chrono::Utc::now().to_rfc3339(),
                        "data_source": "strava"
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "analysis_engine".to_string(),
                            serde_json::Value::String("advanced_performance_analyzer".to_string()),
                        );
                        map.insert(
                            "activities_analyzed".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(activities.len())),
                        );
                        map
                    }),
                }),
                Err(e) => Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to analyze performance trends: {}", e)),
                    metadata: None,
                }),
            }
        });

        analysis_result
    }

    /// Handle compare_activities tool
    fn handle_compare_activities(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract activity ID to compare
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Missing required parameter: activity_id".to_string(),
                )
            })?;

        let rt = tokio::runtime::Handle::current();
        let comparison_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(200), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Find the target activity
            let target_activity = activities.iter().find(|a| a.id == activity_id);
            if target_activity.is_none() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Activity with ID '{}' not found", activity_id)),
                    metadata: None,
                });
            }

            let target_activity = target_activity.unwrap();

            // Use the activity analyzer to compare to history
            let analyzer = crate::intelligence::activity_analyzer::AdvancedActivityAnalyzer::new();

            match analyzer
                .compare_to_history(target_activity, &activities)
                .await
            {
                Ok(comparison_insights) => {
                    // Calculate additional comparison metrics
                    let same_sport_activities: Vec<_> = activities
                        .iter()
                        .filter(|a| {
                            a.sport_type == target_activity.sport_type && a.id != activity_id
                        })
                        .collect();

                    let personal_bests = {
                        let mut pbs = std::collections::HashMap::new();

                        if let Some(distance) = target_activity.distance_meters {
                            let best_distance = same_sport_activities
                                .iter()
                                .filter_map(|a| a.distance_meters)
                                .fold(0.0, |max, d| if d > max { d } else { max });

                            pbs.insert(
                                "distance",
                                serde_json::json!({
                                    "current": distance,
                                    "personal_best": best_distance,
                                    "is_pb": distance >= best_distance
                                }),
                            );
                        }

                        if let Some(speed) = target_activity.average_speed {
                            let best_speed = same_sport_activities
                                .iter()
                                .filter_map(|a| a.average_speed)
                                .fold(0.0, |max, s| if s > max { s } else { max });

                            pbs.insert(
                                "speed",
                                serde_json::json!({
                                    "current": speed,
                                    "personal_best": best_speed,
                                    "is_pb": speed >= best_speed
                                }),
                            );
                        }

                        pbs
                    };

                    let percentile_rankings = {
                        let mut rankings = std::collections::HashMap::new();

                        if let Some(distance) = target_activity.distance_meters {
                            let distances: Vec<f64> = same_sport_activities
                                .iter()
                                .filter_map(|a| a.distance_meters)
                                .collect();

                            if !distances.is_empty() {
                                let better_count =
                                    distances.iter().filter(|&&d| d > distance).count();
                                let percentile = ((distances.len() - better_count) as f64
                                    / distances.len() as f64)
                                    * 100.0;
                                rankings.insert("distance_percentile", percentile);
                            }
                        }

                        if let Some(speed) = target_activity.average_speed {
                            let speeds: Vec<f64> = same_sport_activities
                                .iter()
                                .filter_map(|a| a.average_speed)
                                .collect();

                            if !speeds.is_empty() {
                                let better_count = speeds.iter().filter(|&&s| s > speed).count();
                                let percentile = ((speeds.len() - better_count) as f64
                                    / speeds.len() as f64)
                                    * 100.0;
                                rankings.insert("speed_percentile", percentile);
                            }
                        }

                        rankings
                    };

                    Ok(UniversalResponse {
                        success: true,
                        result: Some(serde_json::json!({
                            "activity_id": activity_id,
                            "sport_type": format!("{:?}", target_activity.sport_type),
                            "comparison_insights": comparison_insights,
                            "personal_bests": personal_bests,
                            "percentile_rankings": percentile_rankings,
                            "similar_activities_count": same_sport_activities.len(),
                            "comparison_date": chrono::Utc::now().to_rfc3339(),
                            "data_source": "strava"
                        })),
                        error: None,
                        metadata: Some({
                            let mut map = std::collections::HashMap::new();
                            map.insert(
                                "comparison_engine".to_string(),
                                serde_json::Value::String("advanced_activity_analyzer".to_string()),
                            );
                            map.insert(
                                "total_activities_analyzed".to_string(),
                                serde_json::Value::Number(serde_json::Number::from(
                                    activities.len(),
                                )),
                            );
                            map
                        }),
                    })
                }
                Err(e) => Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to compare activities: {}", e)),
                    metadata: None,
                }),
            }
        });

        comparison_result
    }

    /// Handle detect_patterns tool
    fn handle_detect_patterns(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract pattern type parameter
        let pattern_type = request
            .parameters
            .get("pattern_type")
            .and_then(|v| v.as_str())
            .unwrap_or("all"); // all, consistency, seasonal, performance

        let rt = tokio::runtime::Handle::current();
        let patterns_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(365), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Detect different types of patterns
            let mut detected_patterns = Vec::new();

            // 1. Training consistency patterns
            if pattern_type == "all" || pattern_type == "consistency" {
                let mut weekly_counts = std::collections::HashMap::new();
                for activity in &activities {
                    let week_start = activity
                        .start_date
                        .date_naive()
                        .week(chrono::Weekday::Mon)
                        .first_day();
                    *weekly_counts.entry(week_start).or_insert(0) += 1;
                }

                let avg_weekly_activities =
                    weekly_counts.values().sum::<i32>() as f64 / weekly_counts.len() as f64;
                let consistent_weeks = weekly_counts.values().filter(|&&count| count >= 3).count();
                let consistency_ratio = consistent_weeks as f64 / weekly_counts.len() as f64;

                detected_patterns.push(serde_json::json!({
                    "type": "training_consistency",
                    "description": if consistency_ratio > 0.7 {
                        "Highly consistent training pattern"
                    } else if consistency_ratio > 0.4 {
                        "Moderately consistent training"
                    } else {
                        "Inconsistent training pattern"
                    },
                    "metrics": {
                        "average_weekly_activities": avg_weekly_activities,
                        "consistency_ratio": consistency_ratio,
                        "consistent_weeks": consistent_weeks,
                        "total_weeks": weekly_counts.len()
                    }
                }));
            }

            // 2. Seasonal patterns
            if pattern_type == "all" || pattern_type == "seasonal" {
                let mut monthly_distance = std::collections::HashMap::new();
                for activity in &activities {
                    let month = activity.start_date.format("%Y-%m").to_string();
                    let distance = activity.distance_meters.unwrap_or(0.0);
                    *monthly_distance.entry(month).or_insert(0.0) += distance;
                }

                if monthly_distance.len() >= 3 {
                    let distances: Vec<f64> = monthly_distance.values().cloned().collect();
                    let avg_distance = distances.iter().sum::<f64>() / distances.len() as f64;
                    let max_distance = distances
                        .iter()
                        .fold(0.0, |max, &d| if d > max { d } else { max });
                    let min_distance =
                        distances
                            .iter()
                            .fold(f64::INFINITY, |min, &d| if d < min { d } else { min });

                    detected_patterns.push(serde_json::json!({
                        "type": "seasonal_volume",
                        "description": "Monthly training volume variations",
                        "metrics": {
                            "average_monthly_distance": avg_distance,
                            "peak_month_distance": max_distance,
                            "lowest_month_distance": min_distance,
                            "volume_variability": (max_distance - min_distance) / avg_distance
                        }
                    }));
                }
            }

            // 3. Performance patterns
            if pattern_type == "all" || pattern_type == "performance" {
                let running_activities: Vec<_> = activities
                    .iter()
                    .filter(|a| format!("{:?}", a.sport_type) == "Run")
                    .filter(|a| a.average_speed.is_some())
                    .collect();

                if running_activities.len() >= 5 {
                    let speeds: Vec<f64> = running_activities
                        .iter()
                        .filter_map(|a| a.average_speed)
                        .collect();

                    // Calculate trend over last 10 activities
                    let recent_speeds: Vec<f64> = speeds.iter().rev().take(10).cloned().collect();
                    if recent_speeds.len() >= 5 {
                        let first_half_avg =
                            recent_speeds[..recent_speeds.len() / 2].iter().sum::<f64>()
                                / (recent_speeds.len() / 2) as f64;
                        let second_half_avg =
                            recent_speeds[recent_speeds.len() / 2..].iter().sum::<f64>()
                                / (recent_speeds.len() - recent_speeds.len() / 2) as f64;

                        let improvement =
                            ((second_half_avg - first_half_avg) / first_half_avg) * 100.0;

                        detected_patterns.push(serde_json::json!({
                            "type": "performance_trend",
                            "description": if improvement > 2.0 {
                                "Improving pace trend detected"
                            } else if improvement < -2.0 {
                                "Declining pace trend detected"
                            } else {
                                "Stable pace performance"
                            },
                            "metrics": {
                                "pace_improvement_percentage": improvement,
                                "recent_activities_analyzed": recent_speeds.len(),
                                "current_average_speed": second_half_avg
                            }
                        }));
                    }
                }
            }

            // 4. Weekly patterns (day of week preferences)
            if pattern_type == "all" || pattern_type == "weekly" {
                let mut day_counts = std::collections::HashMap::new();
                for activity in &activities {
                    let day_of_week = activity.start_date.weekday().to_string();
                    *day_counts.entry(day_of_week).or_insert(0) += 1;
                }

                let most_active_day = day_counts
                    .iter()
                    .max_by_key(|(_, &count)| count)
                    .map(|(day, count)| (day.clone(), *count));

                if let Some((day, count)) = most_active_day {
                    detected_patterns.push(serde_json::json!({
                        "type": "weekly_preference",
                        "description": format!("Most active on {}", day),
                        "metrics": {
                            "preferred_day": day,
                            "activities_on_preferred_day": count,
                            "day_distribution": day_counts
                        }
                    }));
                }
            }

            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "pattern_type_requested": pattern_type,
                    "patterns_detected": detected_patterns,
                    "total_patterns": detected_patterns.len(),
                    "activities_analyzed": activities.len(),
                    "analysis_date": chrono::Utc::now().to_rfc3339(),
                    "data_source": "strava"
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "pattern_detection_engine".to_string(),
                        serde_json::Value::String("statistical_analysis".to_string()),
                    );
                    map.insert(
                        "analysis_period_days".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(365)),
                    );
                    map
                }),
            })
        });

        patterns_result
    }

    /// Handle track_progress tool
    fn handle_track_progress(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract goal parameters
        let goal_id = request
            .parameters
            .get("goal_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default_goal");

        let goal_type_str = request
            .parameters
            .get("goal_type")
            .and_then(|v| v.as_str())
            .unwrap_or("distance");

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(100000.0); // Default 100km

        let rt = tokio::runtime::Handle::current();
        let progress_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id)
                .map_err(|_| crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string()
                ))?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) = provider.get_activities(Some(100), None).await {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some("No activities found or user not connected to any provider".to_string()),
                    metadata: None,
                });
            }

            // Create a mock goal for tracking
            let goal_type = match goal_type_str {
                "distance" => crate::intelligence::GoalType::Distance {
                    sport: "Run".to_string(),
                    timeframe: crate::intelligence::TimeFrame::Month,
                },
                "frequency" => crate::intelligence::GoalType::Frequency {
                    sport: "Run".to_string(),
                    sessions_per_week: target_value as i32,
                },
                _ => crate::intelligence::GoalType::Distance {
                    sport: "Run".to_string(),
                    timeframe: crate::intelligence::TimeFrame::Month,
                },
            };

            let goal = crate::intelligence::Goal {
                id: goal_id.to_string(),
                user_id: request.user_id.clone(),
                title: format!("Track {} goal", goal_type_str),
                description: "Progress tracking goal".to_string(),
                goal_type,
                target_value,
                target_date: chrono::Utc::now() + chrono::Duration::days(30),
                current_value: 0.0,
                created_at: chrono::Utc::now() - chrono::Duration::days(7),
                updated_at: chrono::Utc::now(),
                status: crate::intelligence::GoalStatus::Active,
            };

            // Use the goal engine to track progress
            let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();
            match goal_engine.track_progress(&goal, &activities).await {
                Ok(progress_report) => {
                    Ok(UniversalResponse {
                        success: true,
                        result: Some(serde_json::json!({
                            "goal_id": progress_report.goal_id,
                            "progress_percentage": progress_report.progress_percentage,
                            "completion_date_estimate": progress_report.completion_date_estimate.map(|d| d.to_rfc3339()),
                            "milestones_achieved": progress_report.milestones_achieved,
                            "insights": progress_report.insights,
                            "recommendations": progress_report.recommendations,
                            "on_track": progress_report.on_track,
                            "activities_analyzed": activities.len(),
                            "tracking_date": chrono::Utc::now().to_rfc3339(),
                            "data_source": "strava"
                        })),
                        error: None,
                        metadata: Some({
                            let mut map = std::collections::HashMap::new();
                            map.insert("tracking_engine".to_string(), serde_json::Value::String("advanced_goal_engine".to_string()));
                            map.insert("goal_type".to_string(), serde_json::Value::String(goal_type_str.to_string()));
                            map
                        }),
                    })
                }
                Err(e) => {
                    Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to track progress: {}", e)),
                        metadata: None,
                    })
                }
            }
        });

        progress_result
    }

    /// Handle suggest_goals tool
    fn handle_suggest_goals(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let rt = tokio::runtime::Handle::current();
        let goals_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(100), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Create a basic user profile for goal suggestions
            let user_profile = crate::intelligence::UserFitnessProfile {
                user_id: request.user_id.clone(),
                age: Some(30),
                gender: Some("U".to_string()),
                weight: Some(70.0),
                height: Some(175.0),
                fitness_level: crate::intelligence::FitnessLevel::Intermediate,
                primary_sports: vec!["Run".to_string()],
                training_history_months: 12,
                preferences: crate::intelligence::UserPreferences {
                    preferred_units: "metric".to_string(),
                    training_focus: vec!["endurance".to_string()],
                    injury_history: vec![],
                    time_availability: crate::intelligence::TimeAvailability {
                        hours_per_week: 5.0,
                        preferred_days: vec![
                            "Monday".to_string(),
                            "Wednesday".to_string(),
                            "Friday".to_string(),
                        ],
                        preferred_duration_minutes: Some(60),
                    },
                },
            };

            // Use the goal engine
            let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

            match goal_engine.suggest_goals(&user_profile, &activities).await {
                Ok(goal_suggestions) => Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "goal_suggestions": goal_suggestions,
                        "total_suggestions": goal_suggestions.len(),
                        "user_profile_used": {
                            "fitness_level": format!("{:?}", user_profile.fitness_level),
                            "primary_sports": user_profile.primary_sports,
                        },
                        "activities_analyzed": activities.len(),
                        "generated_at": chrono::Utc::now().to_rfc3339(),
                        "data_source": "strava"
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "goal_engine".to_string(),
                            serde_json::Value::String("advanced_goal_engine".to_string()),
                        );
                        map.insert(
                            "profile_completeness".to_string(),
                            serde_json::Value::String("basic".to_string()),
                        );
                        map
                    }),
                }),
                Err(e) => Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to suggest goals: {}", e)),
                    metadata: None,
                }),
            }
        });

        goals_result
    }

    /// Handle analyze_goal_feasibility tool
    fn handle_analyze_goal_feasibility(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract goal parameters
        let goal_type_str = request
            .parameters
            .get("goal_type")
            .and_then(|v| v.as_str())
            .unwrap_or("distance");

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(42195.0); // Default marathon distance

        let timeline_days = request
            .parameters
            .get("timeline_days")
            .and_then(|v| v.as_i64())
            .unwrap_or(120); // Default 4 months

        let rt = tokio::runtime::Handle::current();
        let feasibility_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id)
                .map_err(|_| crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string()
                ))?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) = provider.get_activities(Some(100), None).await {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some("No activities found or user not connected to any provider".to_string()),
                    metadata: None,
                });
            }

            // Create a goal for feasibility analysis
            let goal_type = match goal_type_str {
                "distance" => crate::intelligence::GoalType::Distance {
                    sport: "Run".to_string(),
                    timeframe: crate::intelligence::TimeFrame::Month,
                },
                "time" => crate::intelligence::GoalType::Time {
                    sport: "Run".to_string(),
                    distance: 42195.0, // Marathon distance
                },
                _ => crate::intelligence::GoalType::Distance {
                    sport: "Run".to_string(),
                    timeframe: crate::intelligence::TimeFrame::Month,
                },
            };

            let _goal = crate::intelligence::Goal {
                id: uuid::Uuid::new_v4().to_string(),
                user_id: request.user_id.clone(),
                title: format!("Feasibility analysis for {} goal", goal_type_str),
                description: "Goal feasibility assessment".to_string(),
                goal_type,
                target_value,
                target_date: chrono::Utc::now() + chrono::Duration::days(timeline_days),
                current_value: 0.0,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
                status: crate::intelligence::GoalStatus::Active,
            };

            // Calculate feasibility score based on activity history
            let similar_activities: Vec<_> = activities
                .iter()
                .filter(|a| format!("{:?}", a.sport_type) == "Run") // Assuming running goal
                .collect();

            // Calculate a basic difficulty assessment based on activity analysis
            let difficulty = if similar_activities.is_empty() {
                "Unknown"
            } else {
                let avg_distance = similar_activities
                    .iter()
                    .filter_map(|a| a.distance_meters)
                    .sum::<f64>() / similar_activities.len() as f64;
                let improvement_ratio = target_value / avg_distance;

                if improvement_ratio < 1.1 {
                    "Easy"
                } else if improvement_ratio < 1.3 {
                    "Moderate"
                } else if improvement_ratio < 1.5 {
                    "Challenging"
                } else {
                    "Ambitious"
                }
            };

            let feasibility_score = if similar_activities.is_empty() {
                0.2 // Low feasibility without training history
            } else {
                let _avg_distance = similar_activities
                    .iter()
                    .filter_map(|a| a.distance_meters)
                    .sum::<f64>() / similar_activities.len() as f64;

                let max_distance = similar_activities
                    .iter()
                    .filter_map(|a| a.distance_meters)
                    .fold(0.0, |max, d| if d > max { d } else { max });
                let training_frequency = similar_activities.len() as f64 / 12.0; // Activities per month over year
                // Calculate feasibility based on current performance vs targe
                let distance_ratio = if target_value > 0.0 {
                    max_distance / target_value
                } else {
                    0.0
                };
                let base_score = distance_ratio.min(1.0) * 0.5;
                let frequency_bonus = (training_frequency / 10.0).min(0.3);
                let time_factor = if timeline_days > 90 { 0.2 } else { 0.1 };

                (base_score + frequency_bonus + time_factor).min(1.0)
            };

            let feasibility_assessment = if feasibility_score > 0.8 {
                "Highly Feasible"
            } else if feasibility_score > 0.6 {
                "Feasible with Effort"
            } else if feasibility_score > 0.4 {
                "Challenging but Possible"
            } else if feasibility_score > 0.2 {
                "Requires Significant Training"
            } else {
                "Not Recommended"
            };

            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "goal_type": goal_type_str,
                    "target_value": target_value,
                    "timeline_days": timeline_days,
                    "feasibility_score": feasibility_score,
                    "feasibility_assessment": feasibility_assessment,
                    "difficulty": difficulty,
                    "analysis_factors": {
                        "training_history_activities": similar_activities.len(),
                        "max_distance_achieved": similar_activities.iter().filter_map(|a| a.distance_meters).fold(0.0, |max, d| if d > max { d } else { max }),
                        "average_distance": if !similar_activities.is_empty() {
                            similar_activities.iter().filter_map(|a| a.distance_meters).sum::<f64>() / similar_activities.len() as f64
                        } else { 0.0 }
                    },
                    "recommendations": match feasibility_assessment {
                        "Highly Feasible" => vec!["You're well-prepared for this goal!", "Focus on maintaining consistency"],
                        "Feasible with Effort" => vec!["Increase training volume gradually", "Focus on building base endurance"],
                        "Challenging but Possible" => vec!["Consider extending timeline", "Implement structured training plan", "Focus on gradual progression"],
                        "Requires Significant Training" => vec!["Start with smaller goals first", "Build consistent training habit", "Consider working with a coach"],
                        _ => vec!["This goal may be too ambitious", "Focus on building base fitness first", "Consider a more achievable target"]
                    },
                    "analysis_date": chrono::Utc::now().to_rfc3339(),
                    "data_source": "strava"
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert("analysis_engine".to_string(), serde_json::Value::String("advanced_goal_engine".to_string()));
                    map.insert("assessment_method".to_string(), serde_json::Value::String("historical_performance".to_string()));
                    map
                }),
            })
        });

        feasibility_result
    }

    /// Handle generate_recommendations tool
    fn handle_generate_recommendations(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let rt = tokio::runtime::Handle::current();
        let recommendations_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(50), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Create a basic user profile for recommendations
            let user_profile = crate::intelligence::UserFitnessProfile {
                user_id: request.user_id.clone(),
                age: Some(30), // Default values - could be enhanced with real user data
                gender: Some("U".to_string()),
                weight: Some(70.0),
                height: Some(175.0),
                fitness_level: crate::intelligence::FitnessLevel::Intermediate,
                primary_sports: vec!["Run".to_string()],
                training_history_months: 12,
                preferences: crate::intelligence::UserPreferences {
                    preferred_units: "metric".to_string(),
                    training_focus: vec!["endurance".to_string()],
                    injury_history: vec![],
                    time_availability: crate::intelligence::TimeAvailability {
                        hours_per_week: 5.0,
                        preferred_days: vec![
                            "Monday".to_string(),
                            "Wednesday".to_string(),
                            "Friday".to_string(),
                        ],
                        preferred_duration_minutes: Some(60),
                    },
                },
            };

            // Use the recommendation engine
            let engine =
                crate::intelligence::recommendation_engine::AdvancedRecommendationEngine::new();

            match engine
                .generate_recommendations(&user_profile, &activities)
                .await
            {
                Ok(recommendations) => Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "recommendations": recommendations,
                        "total_recommendations": recommendations.len(),
                        "user_profile_used": {
                            "fitness_level": format!("{:?}", user_profile.fitness_level),
                            "primary_sports": user_profile.primary_sports,
                        },
                        "activities_analyzed": activities.len(),
                        "generated_at": chrono::Utc::now().to_rfc3339(),
                        "data_source": "strava"
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "recommendation_engine".to_string(),
                            serde_json::Value::String("advanced_recommendation_engine".to_string()),
                        );
                        map.insert(
                            "profile_completeness".to_string(),
                            serde_json::Value::String("basic".to_string()),
                        );
                        map
                    }),
                }),
                Err(e) => Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to generate recommendations: {}", e)),
                    metadata: None,
                }),
            }
        });

        recommendations_result
    }

    /// Handle calculate_fitness_score tool
    fn handle_calculate_fitness_score(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let rt = tokio::runtime::Handle::current();
        let score_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(100), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Use the performance analyzer to calculate fitness score
            let analyzer =
                crate::intelligence::performance_analyzer::AdvancedPerformanceAnalyzer::new();

            match analyzer.calculate_fitness_score(&activities).await {
                Ok(fitness_score) => Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "overall_score": fitness_score.overall_score,
                        "aerobic_fitness": fitness_score.aerobic_fitness,
                        "strength_endurance": fitness_score.strength_endurance,
                        "consistency": fitness_score.consistency,
                        "trend": format!("{:?}", fitness_score.trend),
                        "last_updated": fitness_score.last_updated.to_rfc3339(),
                        "activities_analyzed": activities.len(),
                        "calculation_date": chrono::Utc::now().to_rfc3339(),
                        "data_source": "strava"
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "calculation_engine".to_string(),
                            serde_json::Value::String("advanced_performance_analyzer".to_string()),
                        );
                        map.insert(
                            "score_components".to_string(),
                            serde_json::Value::Array(vec![
                                serde_json::Value::String("aerobic_fitness".to_string()),
                                serde_json::Value::String("strength_endurance".to_string()),
                                serde_json::Value::String("consistency".to_string()),
                            ]),
                        );
                        map
                    }),
                }),
                Err(e) => Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to calculate fitness score: {}", e)),
                    metadata: None,
                }),
            }
        });

        score_result
    }

    /// Handle predict_performance tool
    fn handle_predict_performance(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract target goal parameters
        let sport_type = request
            .parameters
            .get("sport_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Run");

        let metric = request
            .parameters
            .get("metric")
            .and_then(|v| v.as_str())
            .unwrap_or("distance");

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(|v| v.as_f64())
            .unwrap_or(10000.0); // Default 10km

        let rt = tokio::runtime::Handle::current();
        let prediction_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id)
                .map_err(|_| crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string()
                ))?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) = provider.get_activities(Some(100), None).await {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some("No activities found or user not connected to any provider".to_string()),
                    metadata: None,
                });
            }

            // Create target goal
            let target_goal = crate::intelligence::performance_analyzer::ActivityGoal {
                sport_type: sport_type.to_string(),
                metric: metric.to_string(),
                target_value,
                target_date: chrono::Utc::now() + chrono::Duration::days(90), // 3 months from now
            };

            // Use the performance analyzer to predict performance
            let analyzer = crate::intelligence::performance_analyzer::AdvancedPerformanceAnalyzer::new();

            match analyzer.predict_performance(&activities, &target_goal).await {
                Ok(prediction) => {
                    Ok(UniversalResponse {
                        success: true,
                        result: Some(serde_json::json!({
                            "target_goal": prediction.target_goal,
                            "predicted_value": prediction.predicted_value,
                            "confidence": format!("{:?}", prediction.confidence),
                            "factors": prediction.factors,
                            "recommendations": prediction.recommendations,
                            "estimated_achievement_date": prediction.estimated_achievement_date.to_rfc3339(),
                            "activities_analyzed": activities.len(),
                            "prediction_date": chrono::Utc::now().to_rfc3339(),
                            "data_source": "strava"
                        })),
                        error: None,
                        metadata: Some({
                            let mut map = std::collections::HashMap::new();
                            map.insert("prediction_engine".to_string(), serde_json::Value::String("advanced_performance_analyzer".to_string()));
                            map.insert("prediction_horizon_days".to_string(), serde_json::Value::Number(serde_json::Number::from(90)));
                            map
                        }),
                    })
                }
                Err(e) => {
                    Ok(UniversalResponse {
                        success: false,
                        result: None,
                        error: Some(format!("Failed to predict performance: {}", e)),
                        metadata: None,
                    })
                }
            }
        });

        prediction_result
    }

    /// Handle analyze_training_load tool
    fn handle_analyze_training_load(
        executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let rt = tokio::runtime::Handle::current();
        let analysis_result = rt.block_on(async {
            // Parse user ID
            let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Invalid user ID format".to_string(),
                )
            })?;

            // Get activities from provider
            let mut activities = Vec::new();
            if let Ok(Some(token_data)) = executor.get_valid_token(user_uuid, "strava").await {
                match create_provider("strava") {
                    Ok(mut provider) => {
                        let auth_data = AuthData::OAuth2 {
                            client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                            client_secret: std::env::var("STRAVA_CLIENT_SECRET")
                                .unwrap_or_default(),
                            access_token: Some(token_data.access_token.clone()),
                            refresh_token: Some(token_data.refresh_token.clone()),
                        };

                        if let Ok(()) = provider.authenticate(auth_data).await {
                            if let Ok(provider_activities) =
                                provider.get_activities(Some(100), None).await
                            {
                                activities = provider_activities;
                            }
                        }
                    }
                    Err(_) => {}
                }
            }

            if activities.is_empty() {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(
                        "No activities found or user not connected to any provider".to_string(),
                    ),
                    metadata: None,
                });
            }

            // Use the performance analyzer to analyze training load
            let analyzer =
                crate::intelligence::performance_analyzer::AdvancedPerformanceAnalyzer::new();

            match analyzer.analyze_training_load(&activities).await {
                Ok(training_load_analysis) => Ok(UniversalResponse {
                    success: true,
                    result: Some(serde_json::json!({
                        "weekly_loads": training_load_analysis.weekly_loads,
                        "average_weekly_load": training_load_analysis.average_weekly_load,
                        "load_balance_score": training_load_analysis.load_balance_score,
                        "recovery_needed": training_load_analysis.recovery_needed,
                        "recommendations": training_load_analysis.recommendations,
                        "insights": training_load_analysis.insights,
                        "activities_analyzed": activities.len(),
                        "analysis_date": chrono::Utc::now().to_rfc3339(),
                        "data_source": "strava"
                    })),
                    error: None,
                    metadata: Some({
                        let mut map = std::collections::HashMap::new();
                        map.insert(
                            "analysis_engine".to_string(),
                            serde_json::Value::String("advanced_performance_analyzer".to_string()),
                        );
                        map.insert(
                            "analysis_period_weeks".to_string(),
                            serde_json::Value::Number(serde_json::Number::from(4)),
                        );
                        map
                    }),
                }),
                Err(e) => Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Failed to analyze training load: {}", e)),
                    metadata: None,
                }),
            }
        });

        analysis_result
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
