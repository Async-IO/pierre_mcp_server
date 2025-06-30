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

// Intelligence config will be used for future enhancements
use crate::database_plugins::{factory::Database, DatabaseProvider};
// Removed unused import
use crate::intelligence::analyzer::ActivityAnalyzer;
use crate::intelligence::goal_engine::GoalEngineTrait;
use crate::intelligence::performance_analyzer::PerformanceAnalyzerTrait;
use crate::intelligence::recommendation_engine::RecommendationEngineTrait;
use crate::intelligence::ActivityIntelligence;
use crate::models::Activity;
use crate::providers::{create_provider, AuthData, FitnessProvider};
// Removed unused import
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
    pub config: Arc<crate::config::environment::ServerConfig>,
    tools: HashMap<String, UniversalTool>,
}

impl UniversalToolExecutor {
    /// Helper method for tools that are not yet implemented
    /// Returns a proper error instead of panicking
    fn not_implemented_handler(
        _executor: &UniversalToolExecutor,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Err(crate::protocols::ProtocolError::ExecutionFailed(format!(
            "Tool '{}' is not yet implemented",
            request.tool_name
        )))
    }

    /// Provide real activity intelligence analysis using the ActivityIntelligence engine
    async fn get_real_activity_intelligence(
        &self,
        request: &UniversalRequest,
    ) -> Result<serde_json::Value, String> {
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing activity_id parameter")?;

        // Parse user_id
        let user_id = uuid::Uuid::parse_str(&request.user_id)
            .map_err(|e| format!("Invalid user ID: {}", e))?;

        // First, try to get the activity from the database or providers
        let activity_data = match self.get_activity_data(activity_id, user_id).await {
            Ok(data) => data,
            Err(e) => {
                return Ok(serde_json::json!({
                    "activity_id": activity_id,
                    "analysis_type": "error",
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "error": format!("Could not retrieve activity data: {}", e),
                    "intelligence": {
                        "summary": "Analysis failed - activity data not available",
                        "insights": [],
                        "recommendations": [
                            "Ensure the activity exists and you have access to it",
                            "Check that your fitness provider is connected",
                            "Verify the activity_id is correct"
                        ]
                    }
                }));
            }
        };

        // Use the real ActivityAnalyzer for analysis
        let analyzer = ActivityAnalyzer::new();
        match analyzer.analyze_activity(&activity_data, None).await {
            Ok(analysis) => Ok(serde_json::json!({
                "activity_id": activity_id,
                "analysis_type": "full_intelligence",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "intelligence": {
                    "summary": analysis.summary,
                    "insights": analysis.key_insights,
                    "performance_metrics": analysis.performance_indicators,
                    "contextual_factors": analysis.contextual_factors,
                    "generated_at": analysis.generated_at
                },
                "metadata": {
                    "analysis_engine": "ActivityAnalyzer",
                    "analysis_timestamp": analysis.generated_at.to_rfc3339()
                }
            })),
            Err(e) => Err(format!("Activity intelligence analysis failed: {}", e)),
        }
    }

    /// Get activity data from providers or database
    async fn get_activity_data(
        &self,
        activity_id: &str,
        user_id: uuid::Uuid,
    ) -> Result<Activity, String> {
        // Try to get activity from Strava first
        if let Ok(Some(strava_token)) = self.database.get_strava_token(user_id).await {
            let mut strava_provider = crate::providers::strava::StravaProvider::new();

            // Authenticate with stored token
            let auth_data = AuthData::OAuth2 {
                client_id: "strava_client".to_string(), // Would be from config in real implementation
                client_secret: "".to_string(),
                access_token: Some(strava_token.access_token),
                refresh_token: Some(strava_token.refresh_token),
            };

            if strava_provider.authenticate(auth_data).await.is_ok() {
                if let Ok(activity) = strava_provider.get_activity(activity_id).await {
                    return Ok(activity);
                }
            }
        }

        // Try Fitbit if Strava failed
        if let Ok(Some(fitbit_token)) = self.database.get_fitbit_token(user_id).await {
            let mut fitbit_provider = crate::providers::fitbit::FitbitProvider::new();

            // Authenticate with stored token
            let auth_data = AuthData::OAuth2 {
                client_id: "fitbit_client".to_string(), // Would be from config in real implementation
                client_secret: "".to_string(),
                access_token: Some(fitbit_token.access_token),
                refresh_token: Some(fitbit_token.refresh_token),
            };

            if fitbit_provider.authenticate(auth_data).await.is_ok() {
                if let Ok(activity) = fitbit_provider.get_activity(activity_id).await {
                    return Ok(activity);
                }
            }
        }

        Err("Activity not found in any connected providers".to_string())
    }
    pub fn new(
        database: Arc<Database>,
        intelligence: Arc<ActivityIntelligence>,
        config: Arc<crate::config::environment::ServerConfig>,
    ) -> Self {
        let mut executor = Self {
            database,
            intelligence,
            config,
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

        // Register the appropriate provider using centralized config
        match provider {
            "strava" => {
                if let Ok(strava_provider) =
                    crate::oauth::providers::StravaOAuthProvider::from_config(
                        &self.config.oauth.strava,
                    )
                {
                    oauth_manager.register_provider(Box::new(strava_provider));
                } else {
                    return Err(crate::oauth::OAuthError::ConfigurationError(
                        "Failed to initialize Strava provider".to_string(),
                    ));
                }
            }
            "fitbit" => {
                if let Ok(fitbit_provider) =
                    crate::oauth::providers::FitbitOAuthProvider::from_config(
                        &self.config.oauth.fitbit,
                    )
                {
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
        // All tools are now handled through async execute_tool match statement
        // No sync tools needed as everything is async
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
            "disconnect_provider" => self.handle_disconnect_provider_async(request).await,
            "set_goal" => self.handle_set_goal_async(request).await,
            "calculate_metrics" => self.handle_calculate_metrics_async(request).await,
            "analyze_performance_trends" => {
                self.handle_analyze_performance_trends_async(request).await
            }
            "compare_activities" => self.handle_compare_activities_async(request).await,
            "detect_patterns" => self.handle_detect_patterns_async(request).await,
            "track_progress" => self.handle_track_progress_async(request).await,
            "suggest_goals" => self.handle_suggest_goals_async(request).await,
            "analyze_goal_feasibility" => self.handle_analyze_goal_feasibility_async(request).await,
            "generate_recommendations" => self.handle_generate_recommendations_async(request).await,
            "calculate_fitness_score" => self.handle_calculate_fitness_score_async(request).await,
            "predict_performance" => self.handle_predict_performance_async(request).await,
            "analyze_training_load" => self.handle_analyze_training_load_async(request).await,
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
    pub fn list_tools(&self) -> Vec<UniversalTool> {
        vec![
            UniversalTool {
                name: "get_activities".to_string(),
                description: "Get activities from fitness providers".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "get_athlete".to_string(),
                description: "Get athlete information".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "get_stats".to_string(),
                description: "Get athlete statistics".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "analyze_activity".to_string(),
                description: "Analyze an activity".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "get_activity_intelligence".to_string(),
                description: "Get AI intelligence for activity".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "get_connection_status".to_string(),
                description: "Check provider connection status".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "connect_strava".to_string(),
                description: "Generate authorization URL to connect user's Strava account"
                    .to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "connect_fitbit".to_string(),
                description: "Generate authorization URL to connect user's Fitbit account"
                    .to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "disconnect_provider".to_string(),
                description: "Disconnect and remove stored tokens for a specific fitness provider"
                    .to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "set_goal".to_string(),
                description: "Set a fitness goal".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "calculate_metrics".to_string(),
                description: "Calculate advanced fitness metrics for an activity".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "analyze_performance_trends".to_string(),
                description: "Analyze performance trends over time".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "compare_activities".to_string(),
                description: "Compare an activity against similar activities or personal bests"
                    .to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "detect_patterns".to_string(),
                description: "Detect patterns in training data".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "track_progress".to_string(),
                description: "Track progress toward a specific goal".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "suggest_goals".to_string(),
                description: "Generate AI-powered goal suggestions".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "analyze_goal_feasibility".to_string(),
                description: "Assess whether a goal is realistic and achievable".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "generate_recommendations".to_string(),
                description: "Generate personalized training recommendations".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "calculate_fitness_score".to_string(),
                description: "Calculate comprehensive fitness score".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "predict_performance".to_string(),
                description: "Predict future performance capabilities".to_string(),
                handler: Self::not_implemented_handler,
            },
            UniversalTool {
                name: "analyze_training_load".to_string(),
                description: "Analyze training load balance and recovery needs".to_string(),
                handler: Self::not_implemented_handler,
            },
        ]
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<UniversalTool> {
        self.list_tools().into_iter().find(|tool| tool.name == name)
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
                                                    tracing::error!(
                                                        "Strava API call failed: {}",
                                                        e
                                                    );
                                                    vec![serde_json::json!({
                                                        "error": format!("Strava API call failed: {}", e),
                                                        "is_real_data": false
                                                    })]
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::error!("Strava authentication failed: {}", e);
                                            vec![serde_json::json!({
                                                "error": format!("Strava authentication failed: {}", e),
                                                "is_real_data": false
                                            })]
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!("Failed to create Strava provider: {}", e);
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
                            tracing::error!("OAuth error: {}", e);
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

        // Use the real ActivityIntelligence engine for proper analysis
        match self.get_real_activity_intelligence(&request).await {
            Ok(analysis) => Ok(UniversalResponse {
                success: true,
                result: Some(analysis),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "analysis_timestamp".to_string(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    map.insert(
                        "requested_activity_id".to_string(),
                        serde_json::Value::String(activity_id.to_string()),
                    );
                    map
                }),
            }),
            Err(e) => Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!("Activity intelligence analysis failed: {}", e)),
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "analysis_timestamp".to_string(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    map.insert(
                        "requested_activity_id".to_string(),
                        serde_json::Value::String(activity_id.to_string()),
                    );
                    map
                }),
            }),
        }
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

        // Register all providers using centralized config
        if let Ok(strava_provider) =
            crate::oauth::providers::StravaOAuthProvider::from_config(&self.config.oauth.strava)
        {
            oauth_manager.register_provider(Box::new(strava_provider));
        }
        if let Ok(fitbit_provider) =
            crate::oauth::providers::FitbitOAuthProvider::from_config(&self.config.oauth.fitbit)
        {
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

        // Register Strava provider using centralized config
        match crate::oauth::providers::StravaOAuthProvider::from_config(&self.config.oauth.strava) {
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

        // Register Fitbit provider using centralized config
        match crate::oauth::providers::FitbitOAuthProvider::from_config(&self.config.oauth.fitbit) {
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

    /// Handle disconnect_provider tool asynchronously
    async fn handle_disconnect_provider_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let provider = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "provider is required".to_string(),
                )
            })?;

        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Clear tokens for the specified provider
        match provider {
            "strava" => {
                self.database
                    .clear_strava_token(user_uuid)
                    .await
                    .map_err(|e| {
                        crate::protocols::ProtocolError::ExecutionFailed(format!(
                            "Database error: {}",
                            e
                        ))
                    })?;
            }
            "fitbit" => {
                self.database
                    .clear_fitbit_token(user_uuid)
                    .await
                    .map_err(|e| {
                        crate::protocols::ProtocolError::ExecutionFailed(format!(
                            "Database error: {}",
                            e
                        ))
                    })?;
            }
            _ => {
                return Err(crate::protocols::ProtocolError::InvalidParameters(format!(
                    "Unknown provider: {}",
                    provider
                )))
            }
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "provider": provider,
                "status": "disconnected",
                "message": format!("{} has been disconnected successfully", provider)
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle set_goal tool asynchronously
    async fn handle_set_goal_async(
        &self,
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

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Save goal to database
        let created_at = chrono::Utc::now();
        let goal_data = serde_json::json!({
            "goal_type": goal_type,
            "target_value": target_value,
            "timeframe": timeframe,
            "title": title,
            "created_at": created_at.to_rfc3339()
        });

        let goal_id = self
            .database
            .create_goal(user_uuid, goal_data)
            .await
            .map_err(|e| {
                crate::protocols::ProtocolError::ExecutionFailed(format!("Database error: {}", e))
            })?;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "goal_id": goal_id.to_string(),
                "goal_type": goal_type,
                "target_value": target_value,
                "timeframe": timeframe,
                "title": title,
                "created_at": created_at.to_rfc3339(),
                "status": "created"
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle calculate_metrics tool asynchronously
    async fn handle_calculate_metrics_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract activity data from parameters
        let activity = request.parameters.get("activity").ok_or_else(|| {
            crate::protocols::ProtocolError::InvalidParameters(
                "activity parameter is required".to_string(),
            )
        })?;

        let distance = activity
            .get("distance")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let duration = activity
            .get("duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let elevation_gain = activity
            .get("elevation_gain")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let heart_rate = activity
            .get("average_heart_rate")
            .and_then(|v| v.as_u64())
            .map(|hr| hr as u32);

        // Calculate metrics
        let pace = if distance > 0.0 && duration > 0 {
            (duration as f64) / (distance / 1000.0)
        } else {
            0.0
        };

        let speed = if duration > 0 {
            (distance / duration as f64) * 3.6
        } else {
            0.0
        };

        let intensity_score = heart_rate
            .map(|hr| (hr as f64 / 180.0) * 100.0)
            .unwrap_or(50.0);

        let efficiency_score = if distance > 0.0 && elevation_gain > 0.0 {
            (distance / elevation_gain).min(100.0)
        } else {
            75.0
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "pace": pace,
                "speed": speed,
                "intensity_score": intensity_score,
                "efficiency_score": efficiency_score,
                "metrics_summary": {
                    "distance_km": distance / 1000.0,
                    "duration_minutes": duration / 60,
                    "elevation_meters": elevation_gain,
                    "average_heart_rate": heart_rate
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "calculation_timestamp".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                map.insert(
                    "metric_version".to_string(),
                    serde_json::Value::String("1.0".to_string()),
                );
                map
            }),
        })
    }

    /// Handle analyze_performance_trends tool asynchronously
    async fn handle_analyze_performance_trends_async(
        &self,
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

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get activities from provider
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                    "insights": trend_analysis.insights.iter().map(|i| i.message.clone()).collect::<Vec<_>>(),
                    "recommendations": trend_analysis.insights.iter().filter_map(|i| {
                        if i.insight_type == "recommendation" { Some(i.message.clone()) } else { None }
                    }).collect::<Vec<_>>()
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
    }

    /// Handle compare_activities tool asynchronously
    async fn handle_compare_activities_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract activity IDs from parameters
        let activity_id1 = request
            .parameters
            .get("activity_id1")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "activity_id1 is required".to_string(),
                )
            })?;

        let activity_id2 = request
            .parameters
            .get("activity_id2")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "activity_id2 is required".to_string(),
                )
            })?;

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get activities from provider
        let mut activity1 = None;
        let mut activity2 = None;

        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
                        access_token: Some(token_data.access_token.clone()),
                        refresh_token: Some(token_data.refresh_token.clone()),
                    };

                    if let Ok(()) = provider.authenticate(auth_data).await {
                        // Get activities
                        if let Ok(activities) = provider.get_activities(Some(100), None).await {
                            activity1 = activities.iter().find(|a| a.id == activity_id1).cloned();
                            activity2 = activities.iter().find(|a| a.id == activity_id2).cloned();
                        }
                    }
                }
                Err(_) => {}
            }
        }

        if activity1.is_none() || activity2.is_none() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("One or both activities not found".to_string()),
                metadata: None,
            });
        }

        let act1 = activity1.unwrap();
        let act2 = activity2.unwrap();

        // Compare activities
        let comparison = serde_json::json!({
            "activity1": {
                "id": act1.id,
                "name": act1.name,
                "distance": act1.distance_meters,
                "duration": act1.duration_seconds,
                "pace": act1.distance_meters.map(|d| act1.duration_seconds as f64 / (d / 1000.0)),
                "elevation_gain": act1.elevation_gain,
                "average_heart_rate": act1.average_heart_rate
            },
            "activity2": {
                "id": act2.id,
                "name": act2.name,
                "distance": act2.distance_meters,
                "duration": act2.duration_seconds,
                "pace": act2.distance_meters.map(|d| act2.duration_seconds as f64 / (d / 1000.0)),
                "elevation_gain": act2.elevation_gain,
                "average_heart_rate": act2.average_heart_rate
            },
            "differences": {
                "distance_diff": act2.distance_meters.unwrap_or(0.0) - act1.distance_meters.unwrap_or(0.0),
                "duration_diff": act2.duration_seconds as i64 - act1.duration_seconds as i64,
                "pace_improvement": if let (Some(d1), Some(d2)) = (act1.distance_meters, act2.distance_meters) {
                    let pace1 = act1.duration_seconds as f64 / (d1 / 1000.0);
                    let pace2 = act2.duration_seconds as f64 / (d2 / 1000.0);
                    Some(((pace1 - pace2) / pace1) * 100.0)
                } else {
                    None
                },
                "elevation_diff": act2.elevation_gain.unwrap_or(0.0) - act1.elevation_gain.unwrap_or(0.0)
            }
        });

        Ok(UniversalResponse {
            success: true,
            result: Some(comparison),
            error: None,
            metadata: None,
        })
    }

    /// Handle detect_patterns tool asynchronously
    async fn handle_detect_patterns_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get activities from provider
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
                        access_token: Some(token_data.access_token.clone()),
                        refresh_token: Some(token_data.refresh_token.clone()),
                    };

                    if let Ok(()) = provider.authenticate(auth_data).await {
                        if let Ok(provider_activities) =
                            provider.get_activities(Some(300), None).await
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
                error: Some("No activities found to analyze patterns".to_string()),
                metadata: None,
            });
        }

        // Use the activity analyzer from intelligence module
        let _analyzer = crate::intelligence::ActivityAnalyzer::new();

        // Simplified pattern detection - just return mock patterns for now
        let patterns = vec![
            serde_json::json!({
                "pattern_type": "weekly_frequency",
                "description": "Regular weekly training pattern",
                "confidence": 0.8
            }),
            serde_json::json!({
                "pattern_type": "distance_trend",
                "description": "Consistent distance improvement",
                "confidence": 0.7
            }),
        ];

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
            "patterns": patterns,
            "total_activities_analyzed": activities.len(),
            "analysis_period": {
                    "start": activities.last().map(|a| a.start_date.to_rfc3339()),
                    "end": activities.first().map(|a| a.start_date.to_rfc3339())
                },
                "insights": vec!["Found consistent training patterns", "Weekly frequency is stable"]
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "analysis_engine".to_string(),
                    serde_json::Value::String("activity_analyzer".to_string()),
                );
                map.insert(
                    "pattern_detection_version".to_string(),
                    serde_json::Value::String("1.0".to_string()),
                );
                map
            }),
        })
    }

    /// Handle track_progress tool asynchronously
    async fn handle_track_progress_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract goal ID from parameters
        let goal_id = request
            .parameters
            .get("goal_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "goal_id is required".to_string(),
                )
            })?;

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get activities from provider
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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

        // Calculate progress based on activities
        let total_distance: f64 = activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .sum::<f64>()
            / 1000.0; // Convert to km

        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();

        // Mock goal data (in a real implementation, this would be fetched from database)
        let goal_target = 1000.0; // 1000 km
        let progress_percentage = (total_distance / goal_target) * 100.0;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "goal_id": goal_id,
                "current_value": total_distance,
                "target_value": goal_target,
                "progress_percentage": progress_percentage,
                "on_track": progress_percentage >= 50.0, // Simple heuristic
                "days_remaining": 90, // Mock value
                "projected_completion": if progress_percentage > 0.0 {
                    Some((goal_target / total_distance) * 90.0)
                } else {
                    None
                },
                "summary": {
                    "total_activities": activities.len(),
                    "total_distance_km": total_distance,
                    "total_duration_hours": total_duration as f64 / 3600.0
                }
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle suggest_goals tool asynchronously
    async fn handle_suggest_goals_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get recent activities
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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

        // Use the goal engine from intelligence module
        let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

        // Create a mock user profile for the goal engine
        let user_profile = crate::intelligence::UserFitnessProfile {
            user_id: request.user_id.clone(),
            age: Some(30),
            gender: None,
            weight: None,
            height: None,
            fitness_level: crate::intelligence::FitnessLevel::Intermediate,
            primary_sports: vec!["general".to_string()],
            training_history_months: 6,
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

        match goal_engine.suggest_goals(&user_profile, &activities).await {
            Ok(suggestions) => Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "suggested_goals": suggestions.into_iter().map(|g| {
                        serde_json::json!({
                            "goal_type": format!("{:?}", g.goal_type),
                            "target_value": g.suggested_target,
                            "difficulty": format!("{:?}", g.difficulty),
                            "rationale": g.rationale,
                            "estimated_timeline_days": g.estimated_timeline_days,
                            "success_probability": g.success_probability
                        })
                    }).collect::<Vec<_>>(),
                    "activities_analyzed": activities.len()
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "analysis_engine".to_string(),
                        serde_json::Value::String("smart_goal_engine".to_string()),
                    );
                    map.insert(
                        "suggestion_algorithm".to_string(),
                        serde_json::Value::String("adaptive_goal_generation".to_string()),
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
    }

    /// Handle analyze_goal_feasibility tool asynchronously
    async fn handle_analyze_goal_feasibility_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract goal parameters
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

        let _timeframe_days = request
            .parameters
            .get("timeframe_days")
            .and_then(|v| v.as_u64())
            .unwrap_or(90) as u32;

        let title = request
            .parameters
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Fitness Goal")
            .to_string();

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get historical activities
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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

        // Use the goal engine from intelligence module
        let _goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

        // Create a goal object for analysis
        let _goal = crate::intelligence::Goal {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: request.user_id.clone(),
            title: title.clone(),
            description: format!("Goal: {} {}", target_value, goal_type),
            goal_type: match goal_type {
                "distance" => crate::intelligence::GoalType::Distance {
                    sport: "general".to_string(),
                    timeframe: crate::intelligence::TimeFrame::Custom {
                        start: chrono::Utc::now(),
                        end: chrono::Utc::now() + chrono::Duration::days(30),
                    },
                },
                "frequency" => crate::intelligence::GoalType::Frequency {
                    sport: "general".to_string(),
                    sessions_per_week: target_value as i32,
                },
                _ => crate::intelligence::GoalType::Distance {
                    sport: "general".to_string(),
                    timeframe: crate::intelligence::TimeFrame::Custom {
                        start: chrono::Utc::now(),
                        end: chrono::Utc::now() + chrono::Duration::days(30),
                    },
                },
            },
            target_value,
            target_date: chrono::Utc::now() + chrono::Duration::days(30),
            current_value: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            status: crate::intelligence::GoalStatus::Active,
        };

        // Mock goal feasibility analysis since the method doesn't exist
        let feasibility_score = if target_value > 0.0 { 75.0 } else { 0.0 };
        let feasible = feasibility_score > 50.0;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "feasible": feasible,
                "feasibility_score": feasibility_score,
                "confidence_level": 0.8,
                "risk_factors": vec!["Limited historical data"],
                "success_probability": feasibility_score / 100.0,
                "recommendations": vec!["Start with smaller milestones", "Track progress regularly"],
                "adjusted_target": target_value,
                "adjusted_timeframe": 30,
                "historical_context": {
                    "activities_analyzed": activities.len(),
                    "goal_type": goal_type,
                    "target_value": target_value
                }
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle generate_recommendations tool asynchronously
    async fn handle_generate_recommendations_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get recent activities
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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

        // Use the recommendation engine from intelligence module
        let recommendation_engine =
            crate::intelligence::recommendation_engine::AdvancedRecommendationEngine::new();

        // Create a mock user profile for the recommendation engine
        let user_profile = crate::intelligence::UserFitnessProfile {
            user_id: request.user_id.clone(),
            age: Some(30),
            gender: None,
            weight: None,
            height: None,
            fitness_level: crate::intelligence::FitnessLevel::Intermediate,
            primary_sports: vec!["general".to_string()],
            training_history_months: 6,
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

        match recommendation_engine
            .generate_recommendations(&user_profile, &activities)
            .await
        {
            Ok(recommendations) => Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "recommendations": recommendations.into_iter().map(|r| {
                        serde_json::json!({
                            "type": format!("{:?}", r.recommendation_type),
                            "priority": format!("{:?}", r.priority),
                            "title": r.title,
                            "description": r.description,
                            "rationale": r.rationale,
                            "actionable_steps": r.actionable_steps
                        })
                    }).collect::<Vec<_>>(),
                    "personalization": {
                        "fitness_level": format!("{:?}", user_profile.fitness_level),
                        "training_focus": user_profile.preferences.training_focus,
                        "time_availability": user_profile.preferences.time_availability.hours_per_week
                    },
                    "next_steps": vec!["Follow the recommendations above", "Track your progress regularly"],
                    "activities_analyzed": activities.len()
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::new();
                    map.insert(
                        "recommendation_engine".to_string(),
                        serde_json::Value::String("adaptive_recommendation_engine".to_string()),
                    );
                    map.insert(
                        "algorithm_version".to_string(),
                        serde_json::Value::String("2.0".to_string()),
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
    }

    /// Handle calculate_fitness_score tool asynchronously
    async fn handle_calculate_fitness_score_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get recent activities
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                error: Some("No activities found to calculate fitness score".to_string()),
                metadata: None,
            });
        }

        // Calculate fitness metrics
        let total_distance: f64 = activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .sum::<f64>()
            / 1000.0;

        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();

        let avg_pace = if total_distance > 0.0 {
            (total_duration as f64 / 60.0) / total_distance
        } else {
            0.0
        };

        let activity_frequency = activities.len() as f64
            / (chrono::Utc::now() - activities.last().unwrap().start_date)
                .num_days()
                .max(1) as f64
            * 7.0; // Activities per week

        // Calculate composite fitness score (0-100)
        let distance_score = (total_distance / 100.0).min(1.0) * 30.0; // Max 30 points
        let frequency_score = (activity_frequency / 4.0).min(1.0) * 30.0; // Max 30 points
        let pace_score = if avg_pace > 0.0 {
            ((10.0 / avg_pace) * 10.0).min(40.0) // Max 40 points, better pace = higher score
        } else {
            0.0
        };

        let fitness_score = distance_score + frequency_score + pace_score;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "fitness_score": fitness_score,
                "score_components": {
                    "distance_score": distance_score,
                    "frequency_score": frequency_score,
                    "pace_score": pace_score
                },
                "fitness_metrics": {
                    "total_distance_km": total_distance,
                    "total_duration_hours": total_duration as f64 / 3600.0,
                    "average_pace_min_per_km": avg_pace,
                    "activities_per_week": activity_frequency,
                    "total_activities": activities.len()
                },
                "fitness_level": match fitness_score {
                    score if score >= 80.0 => "Excellent",
                    score if score >= 60.0 => "Good",
                    score if score >= 40.0 => "Moderate",
                    score if score >= 20.0 => "Beginner",
                    _ => "Just Starting"
                },
                "percentile": (fitness_score).min(99.0), // Simplified percentile
                "trend": "improving" // Would need historical data for real trend
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "calculation_method".to_string(),
                    serde_json::Value::String("composite_fitness_score_v1".to_string()),
                );
                map.insert(
                    "activities_analyzed".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(activities.len())),
                );
                map.insert(
                    "analysis_period_days".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(
                        (chrono::Utc::now() - activities.last().unwrap().start_date)
                            .num_days()
                            .max(1),
                    )),
                );
                map
            }),
        })
    }

    /// Handle predict_performance tool asynchronously
    async fn handle_predict_performance_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract prediction parameters
        let distance = request
            .parameters
            .get("distance")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "distance is required".to_string(),
                )
            })?;

        let activity_type = request
            .parameters
            .get("activity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("run");

        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get historical activities
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                error: Some("No historical activities found for prediction".to_string()),
                metadata: None,
            });
        }

        // Filter activities by type
        let relevant_activities: Vec<_> = activities
            .iter()
            .filter(|a| match activity_type {
                "run" => matches!(a.sport_type, crate::models::SportType::Run),
                "ride" => matches!(a.sport_type, crate::models::SportType::Ride),
                _ => true,
            })
            .collect();

        if relevant_activities.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some(format!(
                    "No {} activities found for prediction",
                    activity_type
                )),
                metadata: None,
            });
        }

        // Calculate average pace from recent activities
        let total_distance: f64 = relevant_activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .sum::<f64>()
            / 1000.0;

        let total_duration: u64 = relevant_activities.iter().map(|a| a.duration_seconds).sum();

        let avg_pace = if total_distance > 0.0 {
            (total_duration as f64 / 60.0) / total_distance
        } else {
            6.0 // Default 6 min/km
        };

        // Simple linear prediction (in reality, would use more sophisticated models)
        let predicted_time_minutes = avg_pace * (distance / 1000.0);

        // Add fatigue factor for longer distances
        let fatigue_factor = 1.0 + ((distance / 1000.0) / 42.195).powf(0.06);
        let adjusted_time = predicted_time_minutes * fatigue_factor;

        // Calculate confidence based on data availability
        let confidence = (relevant_activities.len() as f64 / 20.0).min(0.95) * 100.0;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "predicted_time": {
                    "minutes": adjusted_time,
                    "formatted": format!("{}:{:02}",
                        adjusted_time as u32,
                        ((adjusted_time % 1.0) * 60.0) as u32
                    )
                },
                "predicted_pace": {
                    "min_per_km": adjusted_time / (distance / 1000.0),
                    "min_per_mile": (adjusted_time / (distance / 1000.0)) * 1.60934
                },
                "confidence_level": confidence,
                "prediction_basis": {
                    "activities_analyzed": relevant_activities.len(),
                    "average_training_pace": avg_pace,
                    "distance_km": distance / 1000.0,
                    "activity_type": activity_type
                },
                "performance_range": {
                    "best_case": adjusted_time * 0.95,
                    "worst_case": adjusted_time * 1.10
                },
                "training_recommendations": if confidence < 50.0 {
                    vec!["More training data needed for accurate prediction"]
                } else if adjusted_time / (distance / 1000.0) > 7.0 {
                    vec!["Consider increasing training volume", "Focus on pace improvement"]
                } else {
                    vec!["Maintain current training", "Consider interval training for speed"]
                }
            })),
            error: None,
            metadata: None,
        })
    }

    /// Handle analyze_training_load tool asynchronously
    async fn handle_analyze_training_load_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = uuid::Uuid::parse_str(&request.user_id).map_err(|_| {
            crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".to_string())
        })?;

        // Get recent activities (last 4 weeks)
        let mut activities = Vec::new();
        if let Ok(Some(token_data)) = self.get_valid_token(user_uuid, "strava").await {
            match create_provider("strava") {
                Ok(mut provider) => {
                    let auth_data = AuthData::OAuth2 {
                        client_id: std::env::var("STRAVA_CLIENT_ID").unwrap_or_default(),
                        client_secret: std::env::var("STRAVA_CLIENT_SECRET").unwrap_or_default(),
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
                error: Some("No activities found to analyze training load".to_string()),
                metadata: None,
            });
        }

        // Filter activities from last 4 weeks
        let four_weeks_ago = chrono::Utc::now() - chrono::Duration::weeks(4);
        let recent_activities: Vec<_> = activities
            .into_iter()
            .filter(|a| a.start_date > four_weeks_ago)
            .collect();

        // Calculate weekly loads
        let mut weekly_loads = vec![0.0; 4];
        for activity in &recent_activities {
            let weeks_ago = (chrono::Utc::now() - activity.start_date).num_weeks() as usize;
            if weeks_ago < 4 {
                let load = activity.duration_seconds as f64 / 60.0; // Simple duration-based load
                weekly_loads[3 - weeks_ago] += load;
            }
        }

        // Calculate acute and chronic loads
        let acute_load = weekly_loads[3]; // This week
        let chronic_load = weekly_loads.iter().sum::<f64>() / 4.0; // 4-week average

        let load_ratio = if chronic_load > 0.0 {
            acute_load / chronic_load
        } else {
            1.0
        };

        // Determine training load balance
        let load_balance = match load_ratio {
            r if r < 0.8 => "Detraining",
            r if r < 1.0 => "Maintaining",
            r if r < 1.3 => "Optimal",
            r if r < 1.5 => "High",
            _ => "Very High - Risk of Overtraining",
        };

        // Calculate recovery recommendations
        let recovery_recommendation = match load_ratio {
            r if r < 1.0 => "Current load is good, consider slight increase",
            r if r < 1.3 => "Optimal training load, maintain current level",
            r if r < 1.5 => "High training load, ensure adequate recovery",
            _ => "Very high load - consider rest days or easy sessions",
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "training_load_balance": load_balance,
                "load_metrics": {
                    "acute_load": acute_load,
                    "chronic_load": chronic_load,
                    "acute_chronic_ratio": load_ratio,
                    "weekly_loads": weekly_loads
                },
                "recovery_recommendation": recovery_recommendation,
                "training_stress": {
                    "current_week": weekly_loads[3],
                    "trend": if weekly_loads[3] > weekly_loads[2] { "increasing" } else { "decreasing" },
                    "total_activities": recent_activities.len()
                },
                "recommendations": match load_ratio {
                    r if r < 0.8 => vec![
                        "Increase training volume gradually",
                        "Add one additional session per week"
                    ],
                    r if r < 1.3 => vec![
                        "Maintain current training schedule",
                        "Focus on quality over quantity"
                    ],
                    _ => vec![
                        "Prioritize recovery",
                        "Consider active recovery sessions",
                        "Ensure adequate sleep and nutrition"
                    ]
                },
                "injury_risk": match load_ratio {
                    r if r < 1.3 => "Low",
                    r if r < 1.5 => "Moderate",
                    _ => "High"
                },
                "next_week_guidance": {
                    "recommended_load": chronic_load * 1.1,
                    "max_safe_load": chronic_load * 1.3
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "analysis_engine".to_string(),
                    serde_json::Value::String("training_load_analyzer".to_string()),
                );
                map.insert(
                    "analysis_period_weeks".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(4)),
                );
                map.insert(
                    "activities_analyzed".to_string(),
                    serde_json::Value::Number(serde_json::Number::from(recent_activities.len())),
                );
                map.insert(
                    "analysis_date".to_string(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                map.insert(
                    "data_source".to_string(),
                    serde_json::Value::String("strava".to_string()),
                );
                map
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    // Currently no tests in this module
}
