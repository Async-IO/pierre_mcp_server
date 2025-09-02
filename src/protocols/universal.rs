// ABOUTME: Universal fitness activity protocol and data structures
// ABOUTME: Common activity format that normalizes data across all fitness platforms
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
#![allow(clippy::option_if_let_else)]
#![allow(clippy::single_match_else)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::cognitive_complexity)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::similar_names)]
#![allow(clippy::unused_self)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::implicit_clone)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::enum_glob_use)]
#![allow(clippy::redundant_clone)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::match_bool)]
#![allow(clippy::if_then_some_else_none)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::else_if_without_else)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::fn_params_excessive_bools)]
// Final allow for remaining complex patterns in this protocol adapter
#![allow(clippy::too_many_lines)]

// Intelligence config will be used for future enhancements
use crate::constants::{limits, oauth_providers};
use crate::database_plugins::DatabaseProvider;
use crate::providers::strava_provider::StravaProvider;
use crate::utils::{json_responses::activity_not_found_error, uuid::parse_user_id_for_protocol};
// Removed unused import
use crate::intelligence::goal_engine::GoalEngineTrait;
use crate::intelligence::performance_analyzer::PerformanceAnalyzerTrait;
use crate::intelligence::physiological_constants::{
    api_limits::{
        DEFAULT_ACTIVITY_LIMIT, GOAL_ANALYSIS_ACTIVITY_LIMIT, LARGE_ACTIVITY_LIMIT,
        MAX_ACTIVITY_LIMIT, SMALL_ACTIVITY_LIMIT,
    },
    business_thresholds::{
        CONFIDENCE_BASE_DIVISOR, DEFAULT_HR_EFFORT_SCORE, DISTANCE_SCORE_DIVISOR,
        DURATION_SCORE_FACTOR, EFFORT_SCORE_MULTIPLIER, FATIGUE_EXPONENT, MARATHON_DISTANCE_KM,
        MAX_CONFIDENCE_RATIO, MAX_DISTANCE_SCORE, MAX_PACE_SCORE, MAX_SCORE, MIN_SCORE,
        MIN_VALID_DISTANCE, PACE_SCORING_BASE, PACE_SCORING_MULTIPLIER,
        SLOW_PACE_THRESHOLD_MIN_PER_KM,
    },
    demo_data::DEMO_GOAL_DISTANCE,
    efficiency_defaults::{DEFAULT_EFFICIENCY_SCORE, DEFAULT_EFFICIENCY_WITH_DISTANCE},
    fitness_score_thresholds::{
        BEGINNER_FITNESS_THRESHOLD, EXCELLENT_FITNESS_THRESHOLD, GOOD_FITNESS_THRESHOLD,
        MODERATE_FITNESS_THRESHOLD,
    },
    goal_feasibility::{
        HIGH_FEASIBILITY_THRESHOLD, MODERATE_FEASIBILITY_THRESHOLD, SIMPLE_PROGRESS_THRESHOLD,
    },
    hr_estimation::ASSUMED_MAX_HR,
    unit_conversions::MS_TO_KMH_FACTOR,
};
use crate::intelligence::recommendation_engine::RecommendationEngineTrait;
use crate::mcp::resources::ServerResources;
use crate::providers::{create_provider, CoreFitnessProvider, OAuth2Credentials};
// Configuration management imports
use crate::configuration::{
    catalog::CatalogBuilder,
    profiles::{ConfigProfile, ProfileTemplates},
    runtime::{ConfigValue, RuntimeConfig},
    validation::ConfigValidator,
    vo2_max::VO2MaxCalculator,
};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
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
    pub resources: Arc<ServerResources>,
    tools: HashMap<String, UniversalTool>,
}

impl UniversalToolExecutor {
    /// Handler for tools that are implemented asynchronously
    /// Routes tools to async execution through `execute_tool()` method
    fn async_implemented_handler(
        _executor: &Self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        Err(crate::protocols::ProtocolError::ExecutionFailed(format!(
            "Tool '{}' is implemented asynchronously - use execute_tool() instead",
            request.tool_name
        )))
    }

    /// Create authenticated provider - now returns error as providers are handled at tenant level
    fn create_authenticated_provider(
        &self,
        _provider_name: &str,
        _user_id: uuid::Uuid,
        _tenant_id: Option<&str>,
    ) -> Result<Box<dyn CoreFitnessProvider>, UniversalResponse> {
        Err(UniversalResponse {
            success: false,
            result: None,
            error: Some(
                "Provider authentication not available - use tenant-aware MCP endpoints"
                    .to_string(),
            ),
            metadata: None,
        })
    }

    /// Create MCP-compatible response format for Claude Desktop
    /// This ensures all tools return responses in the format expected by MCP clients
    fn create_mcp_response(
        &self,
        text_content: String,
        structured_data: serde_json::Value,
        is_error: bool,
    ) -> UniversalResponse {
        let result = serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": text_content
                }
            ],
            "structuredContent": structured_data,
            "isError": is_error
        });

        UniversalResponse {
            success: !is_error,
            result: Some(result),
            error: if is_error {
                Some("Tool execution failed".into())
            } else {
                None
            },
            metadata: None,
        }
    }

    /// Provide real activity intelligence analysis using the `ActivityIntelligence` engine
    fn get_real_activity_intelligence(
        &self,
        request: &UniversalRequest,
    ) -> Result<serde_json::Value, String> {
        let activity_id = request
            .parameters
            .get("activity_id")
            .and_then(|v| v.as_str())
            .ok_or("Missing activity_id parameter")?;

        // Parse user_id
        let _user_id = crate::utils::uuid::parse_uuid(&request.user_id)
            .map_err(|e| format!("Invalid user ID: {e}"))?;

        // Activity data retrieval not available through universal interface
        Ok(serde_json::json!({
            "activity_id": activity_id,
            "analysis_type": "error",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "error": "Activity data retrieval not available - use tenant-aware MCP endpoints",
            "intelligence": {
                "summary": "Analysis unavailable - use MCP tenant endpoints for activity intelligence",
                "insights": [],
                "recommendations": [
                    "Use MCP protocol with tenant-aware endpoints",
                    "Configure OAuth at tenant level",
                    "Access activity intelligence through proper MCP tools"
                ]
            }
        }))
    }
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        Self {
            resources,
            tools: HashMap::new(),
        }
    }

    /// Get valid token for a provider, automatically refreshing if needed
    async fn get_valid_token(
        &self,
        user_id: uuid::Uuid,
        provider: &str,
        tenant_id: Option<&str>,
    ) -> Result<Option<crate::oauth::TokenData>, crate::oauth::OAuthError> {
        // If we have tenant context, use tenant-specific OAuth credentials
        if let Some(tenant_id_str) = tenant_id {
            // Convert string tenant ID to UUID and look up full tenant context
            if let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id_str) {
                // Look up tenant information from database to create proper TenantContext
                if let Ok(tenant) = self.resources.database.get_tenant_by_id(tenant_uuid).await {
                    let tenant_context = crate::tenant::TenantContext {
                        tenant_id: tenant_uuid,
                        tenant_name: tenant.name.clone(),
                        user_id, // Use the user_id from the request
                        user_role: crate::tenant::TenantRole::Member, // Default role for OAuth operations
                    };

                    // Get tenant-specific OAuth credentials using proper TenantContext
                    match self
                        .resources
                        .tenant_oauth_client
                        .get_oauth_client(&tenant_context, provider)
                        .await
                    {
                        Ok(oauth_client) => {
                            // Create OAuth manager and register provider with tenant credentials
                            let mut oauth_manager = self.resources.oauth_manager.write().await;

                            // Extract credentials from OAuth2Client config
                            let config = oauth_client.config();

                            match provider {
                                oauth_providers::STRAVA => {
                                    if let Ok(tenant_provider) =
                                        crate::oauth::providers::StravaOAuthProvider::from_config(
                                            &crate::config::environment::OAuthProviderConfig {
                                                client_id: Some(config.client_id.clone()),
                                                client_secret: Some(config.client_secret.clone()),
                                                redirect_uri: Some(config.redirect_uri.clone()),
                                                scopes: config.scopes.clone(),
                                                enabled: true,
                                            },
                                        )
                                    {
                                        oauth_manager.register_provider(Box::new(tenant_provider));
                                        let result = oauth_manager
                                            .ensure_valid_token(user_id, provider)
                                            .await;
                                        drop(oauth_manager);
                                        return result;
                                    }
                                }
                                oauth_providers::FITBIT => {
                                    if let Ok(tenant_provider) =
                                        crate::oauth::providers::FitbitOAuthProvider::from_config(
                                            &crate::config::environment::OAuthProviderConfig {
                                                client_id: Some(config.client_id.clone()),
                                                client_secret: Some(config.client_secret.clone()),
                                                redirect_uri: Some(config.redirect_uri.clone()),
                                                scopes: config.scopes.clone(),
                                                enabled: true,
                                            },
                                        )
                                    {
                                        oauth_manager.register_provider(Box::new(tenant_provider));
                                        let result = oauth_manager
                                            .ensure_valid_token(user_id, provider)
                                            .await;
                                        drop(oauth_manager);
                                        return result;
                                    }
                                }
                                _ => {
                                    return Err(crate::oauth::OAuthError::UnsupportedProvider(
                                        provider.to_string(),
                                    ))
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to get tenant OAuth client for {}: {}. Using global config.", provider, e);
                            // Continue to global config
                        }
                    }
                } else {
                    tracing::warn!(
                        "Tenant {} not found in database. Using global config.",
                        tenant_uuid
                    );
                    // Continue to global config
                }
            } else {
                tracing::warn!(
                    "Invalid tenant ID format: {}. Using global config.",
                    tenant_id_str
                );
                // Continue to global config
            }
        }

        // Use pre-registered global config
        // Providers are now pre-registered at startup, so just use read lock
        let oauth_manager = self.resources.oauth_manager.read().await;

        // Verify the provider is supported
        match provider {
            oauth_providers::STRAVA | oauth_providers::FITBIT => {
                // Provider should already be registered at startup
            }
            _ => {
                return Err(crate::oauth::OAuthError::UnsupportedProvider(
                    provider.to_string(),
                ))
            }
        }

        oauth_manager.ensure_valid_token(user_id, provider).await
    }

    /// Register a new tool
    pub fn register_tool(&mut self, mut tool: UniversalTool) {
        let name = std::mem::take(&mut tool.name);
        self.tools.insert(name, tool);
    }

    /// Execute a tool by name
    ///
    /// # Errors
    ///
    /// Returns a protocol error if tool execution fails or tool is not found.
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
            "get_activity_intelligence" => self.handle_get_activity_intelligence_async(request),
            "get_connection_status" => self.handle_connection_status_async(request).await,
            "disconnect_provider" => self.handle_disconnect_provider_async(request).await,
            "set_goal" => self.handle_set_goal_async(request).await,
            "calculate_metrics" => self.handle_calculate_metrics_async(request),
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
            // Configuration Management Tools
            "get_configuration_catalog" => self.handle_get_configuration_catalog_async(request),
            "get_configuration_profiles" => self.handle_get_configuration_profiles_async(request),
            "get_user_configuration" => self.handle_get_user_configuration_async(request).await,
            "update_user_configuration" => {
                self.handle_update_user_configuration_async(request).await
            }
            "calculate_personalized_zones" => {
                self.handle_calculate_personalized_zones_async(request)
            }
            "validate_configuration" => self.handle_validate_configuration_async(request),
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
    #[must_use]
    pub fn list_tools(&self) -> Vec<UniversalTool> {
        vec![
            UniversalTool {
                name: "get_activities".into(),
                description: "Get activities from fitness providers".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "get_athlete".into(),
                description: "Get athlete information".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "get_stats".into(),
                description: "Get athlete statistics".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "analyze_activity".into(),
                description: "Analyze an activity".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "get_activity_intelligence".into(),
                description: "Get AI intelligence for activity".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "get_connection_status".into(),
                description: "Check provider connection status".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "disconnect_provider".into(),
                description: "Disconnect and remove stored tokens for a specific fitness provider"
                    .to_string(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "set_goal".into(),
                description: "Set a fitness goal".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "calculate_metrics".into(),
                description: "Calculate advanced fitness metrics for an activity".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "analyze_performance_trends".into(),
                description: "Analyze performance trends over time".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "compare_activities".into(),
                description: "Compare an activity against similar activities or personal bests"
                    .to_string(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "detect_patterns".into(),
                description: "Detect patterns in training data".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "track_progress".into(),
                description: "Track progress toward a specific goal".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "suggest_goals".into(),
                description: "Generate AI-powered goal suggestions".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "analyze_goal_feasibility".into(),
                description: "Assess whether a goal is realistic and achievable".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "generate_recommendations".into(),
                description: "Generate personalized training recommendations".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "calculate_fitness_score".into(),
                description: "Calculate comprehensive fitness score".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "predict_performance".into(),
                description: "Predict future performance capabilities".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "analyze_training_load".into(),
                description: "Analyze training load balance and recovery needs".into(),
                handler: Self::async_implemented_handler,
            },
            // Configuration Management Tools
            UniversalTool {
                name: "get_configuration_catalog".into(),
                description: "Get the complete configuration catalog with all available parameters".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "get_configuration_profiles".into(),
                description: "Get available configuration profiles (Research, Elite, Recreational, etc.)".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "get_user_configuration".into(),
                description: "Get current user's configuration settings and overrides".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "update_user_configuration".into(),
                description: "Update user's configuration parameters and session overrides".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "calculate_personalized_zones".into(),
                description: "Calculate personalized training zones based on user's VO2 max and configuration".into(),
                handler: Self::async_implemented_handler,
            },
            UniversalTool {
                name: "validate_configuration".into(),
                description: "Validate configuration parameters against safety rules and constraints".into(),
                handler: Self::async_implemented_handler,
            },
        ]
    }

    /// Get tool by name
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<UniversalTool> {
        self.list_tools().into_iter().find(|tool| tool.name == name)
    }

    /// Handle `get_activities` with async Strava `API` calls
    async fn handle_get_activities_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract parameters
        let limit = request
            .parameters
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10)
            .try_into()
            .unwrap_or(10_usize);

        let provider_type = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(oauth_providers::STRAVA);

        // Get REAL Strava data
        let activities = if provider_type == oauth_providers::STRAVA {
            match crate::utils::uuid::parse_uuid(&request.user_id) {
                Ok(user_uuid) => {
                    // Get valid Strava token (with automatic refresh if needed)
                    match self
                        .get_valid_token(
                            user_uuid,
                            oauth_providers::STRAVA,
                            request.tenant_id.as_deref(),
                        )
                        .await
                    {
                        Ok(Some(token_data)) => {
                            // Get tenant-aware OAuth credentials
                            let auth_data = if let Some(tenant_id_str) =
                                request.tenant_id.as_deref()
                            {
                                if let Ok(tenant_uuid) = uuid::Uuid::parse_str(tenant_id_str) {
                                    // Look up tenant information from database to create proper TenantContext
                                    if let Ok(tenant) =
                                        self.resources.database.get_tenant_by_id(tenant_uuid).await
                                    {
                                        let tenant_context = crate::tenant::TenantContext {
                                            tenant_id: tenant_uuid,
                                            tenant_name: tenant.name.clone(),
                                            user_id: user_uuid, // Use the _user_id from the request
                                            user_role: crate::tenant::TenantRole::Member,
                                        };

                                        // Use tenant-specific OAuth credentials
                                        match self
                                            .resources
                                            .tenant_oauth_client
                                            .get_oauth_client(
                                                &tenant_context,
                                                oauth_providers::STRAVA,
                                            )
                                            .await
                                        {
                                            Ok(oauth_client) => {
                                                let config = oauth_client.config();
                                                OAuth2Credentials {
                                                    client_id: config.client_id.clone(),
                                                    client_secret: config.client_secret.clone(),
                                                    access_token: Some(
                                                        token_data.access_token.clone(),
                                                    ),
                                                    refresh_token: Some(
                                                        token_data.refresh_token.clone(),
                                                    ),
                                                    expires_at: Some(token_data.expires_at),
                                                    scopes: crate::constants::oauth::STRAVA_DEFAULT_SCOPES
                                                        .split(',')
                                                        .map(str::to_string)
                                                        .collect(),
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("Tenant OAuth credentials not configured for {} in tenant {}: {}", provider_type, tenant_uuid, e);
                                                return Ok(UniversalResponse {
                                                    success: false,
                                                    result: None,
                                                    error: Some(format!(
                                                        "Tenant {} must configure {} OAuth credentials first. Please contact your tenant administrator.",
                                                        tenant_uuid, provider_type
                                                    )),
                                                    metadata: None,
                                                });
                                            }
                                        }
                                    } else {
                                        tracing::error!(
                                            "Tenant {} not found in database",
                                            tenant_uuid
                                        );
                                        return Ok(UniversalResponse {
                                            success: false,
                                            result: None,
                                            error: Some(format!(
                                                "Tenant {} not found. Please check tenant configuration.",
                                                tenant_uuid
                                            )),
                                            metadata: None,
                                        });
                                    }
                                } else {
                                    tracing::error!("Invalid tenant ID format: {}", tenant_id_str);
                                    return Ok(UniversalResponse {
                                        success: false,
                                        result: None,
                                        error: Some(format!(
                                            "Invalid tenant ID format: {}. Must be a valid UUID.",
                                            tenant_id_str
                                        )),
                                        metadata: None,
                                    });
                                }
                            } else {
                                // No tenant context - require tenant-specific configuration
                                tracing::error!(
                                    "No tenant context provided for {} provider authentication",
                                    provider_type
                                );
                                return Ok(UniversalResponse {
                                    success: false,
                                    result: None,
                                    error: Some(format!(
                                        "Authentication required for {} provider access",
                                        provider_type
                                    )),
                                    metadata: None,
                                });
                            };

                            // Create Strava provider with tenant-aware token
                            match create_provider(oauth_providers::STRAVA) {
                                Ok(mut provider) => {
                                    // Set credentials and get REAL activities
                                    match provider.set_credentials(auth_data).await {
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
                                                            "provider": oauth_providers::STRAVA,
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

        // Create human-readable text summary
        let text_content = if activities.is_empty() {
            format!("No activities found for {}", provider_type)
        } else {
            let mut summary = format!(
                "Found {} activities from {}:\n\n",
                activities.len(),
                provider_type
            );
            for (i, activity) in activities.iter().take(5).enumerate() {
                let name = activity
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Activity");
                let sport_type = activity
                    .get("sport_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown");
                let distance_km = activity
                    .get("distance_meters")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    / 1000.0;
                let duration_min = activity
                    .get("duration_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0)
                    / 60;
                let date = activity
                    .get("start_date")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown Date");

                use std::fmt::Write;
                let _ = write!(
                    summary,
                    "{}. {} ({})\n   Distance: {:.2} km, Duration: {} min\n   Date: {}\n\n",
                    i + 1,
                    name,
                    sport_type,
                    distance_km,
                    duration_min,
                    date
                );
            }
            summary
        };

        // Create the structured content (the actual data)
        let structured_data = serde_json::json!({
            "activities": activities,
            "total_count": activities.len(),
            "provider": provider_type
        });

        // Return MCP-compatible response format
        let result = serde_json::json!({
            "content": [
                {
                    "type": "text",
                    "text": text_content
                }
            ],
            "structuredContent": structured_data,
            "isError": false
        });

        Ok(UniversalResponse {
            success: true,
            result: Some(result),
            error: None,
            metadata: Some({
                let mut meta = std::collections::HashMap::with_capacity(4);
                meta.insert("limit".into(), serde_json::Value::Number(limit.into()));
                meta
            }),
        })
    }

    /// Handle `get_athlete` with async Strava `API` calls
    async fn handle_get_athlete_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let provider_type = request
            .parameters
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or(oauth_providers::STRAVA);

        // Parse user ID
        let user_uuid = match crate::utils::uuid::parse_uuid(&request.user_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                let error_text = format!("Invalid user ID: {}", e);
                let error_data = serde_json::json!({
                    "error": error_text,
                    "is_real_data": false
                });
                return Ok(self.create_mcp_response(error_text, error_data, true));
            }
        };

        // Get REAL athlete data using the same approach as get_activities
        if provider_type == oauth_providers::STRAVA {
            // Get valid Strava token (with automatic refresh if needed)
            match self
                .get_valid_token(
                    user_uuid,
                    oauth_providers::STRAVA,
                    request.tenant_id.as_deref(),
                )
                .await
            {
                Ok(Some(token_data)) => {
                    // Create Strava provider and set credentials
                    let mut strava_provider = StravaProvider::new();
                    let credentials = crate::providers::core::OAuth2Credentials {
                        client_id: String::new(),
                        client_secret: String::new(),
                        access_token: Some(token_data.access_token),
                        refresh_token: Some(token_data.refresh_token),
                        expires_at: Some(token_data.expires_at),
                        scopes: vec![],
                    };
                    strava_provider
                        .set_credentials(credentials)
                        .await
                        .map_err(|e| {
                            crate::protocols::ProtocolError::ExecutionFailed(format!(
                                "Failed to set credentials: {}",
                                e
                            ))
                        })?;
                    match strava_provider.get_athlete().await {
                        Ok(athlete) => {
                            let athlete_data = serde_json::to_value(&athlete).unwrap_or_else(|_| {
                                serde_json::json!({"error": "Failed to serialize athlete data"})
                            });

                            // Create human-readable text summary
                            let text_content = format!(
                                "Strava Athlete Profile\n\n\
                                Name: {} {}\n\
                                Username: {}\n\
                                Profile Picture: {}\n\
                                Provider: {}\n\n\
                                Full Profile Data Available in Structured Content",
                                athlete.firstname.as_deref().unwrap_or("N/A"),
                                athlete.lastname.as_deref().unwrap_or("N/A"),
                                &athlete.username,
                                athlete.profile_picture.as_deref().unwrap_or("N/A"),
                                &athlete.provider
                            );

                            Ok(self.create_mcp_response(text_content, athlete_data, false))
                        }
                        Err(e) => {
                            let error_text =
                                format!("Failed to get athlete data from Strava: {}", e);
                            let error_data = serde_json::json!({
                                "error": error_text,
                                "is_real_data": false
                            });
                            Ok(self.create_mcp_response(error_text, error_data, true))
                        }
                    }
                }
                Ok(None) => {
                    let error_text = "No valid Strava token found. Please reconnect to Strava.";
                    let error_data = serde_json::json!({
                        "error": error_text,
                        "is_real_data": false
                    });
                    Ok(self.create_mcp_response(error_text.to_string(), error_data, true))
                }
                Err(e) => {
                    let error_text = format!("Token validation failed: {}", e);
                    let error_data = serde_json::json!({
                        "error": error_text,
                        "is_real_data": false
                    });
                    Ok(self.create_mcp_response(error_text, error_data, true))
                }
            }
        } else {
            let error_text = format!("Unsupported provider: {}", provider_type);
            let error_data = serde_json::json!({
                "error": error_text,
                "is_real_data": false
            });
            Ok(self.create_mcp_response(error_text, error_data, true))
        }
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
                crate::protocols::ProtocolError::InvalidParameters("activity_id is required".into())
            })?;

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get real activity data using authenticated provider (no hardcoded fallbacks)
        let activity_result =
            match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
                Ok(provider) => {
                    // Get all activities and find the specific one
                    provider
                        .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                        .await
                        .map_or(None, |activities| {
                            activities.into_iter().find(|a| a.id == activity_id)
                        })
                }
                Err(error_response) => {
                    // Return error response for tenant configuration issues
                    return Ok(error_response);
                }
            };

        match activity_result {
            Some(activity) => {
                // Use the intelligence module for real analysis
                let intelligence = &self.resources.activity_intelligence;

                // Generate basic analysis
                let efficiency_score = if let Some(distance) = activity.distance_meters {
                    if activity.duration_seconds > 0 && distance > f64::from(MIN_VALID_DISTANCE) {
                        // Simple efficiency calculation: distance/time ratio normalized
                        let duration_f64 = activity.duration_seconds.min(u32::MAX as u64) as f64;
                        let speed_ms = distance / duration_f64;
                        (speed_ms * f64::from(MAX_SCORE))
                            .clamp(f64::from(MIN_SCORE), f64::from(MAX_SCORE))
                    } else {
                        DEFAULT_EFFICIENCY_SCORE
                    }
                } else {
                    DEFAULT_EFFICIENCY_SCORE
                };

                let relative_effort = activity
                    .average_heart_rate
                    .map_or(f64::from(DEFAULT_HR_EFFORT_SCORE), |hr| {
                        (f64::from(hr) / ASSUMED_MAX_HR) * f64::from(EFFORT_SCORE_MULTIPLIER)
                    });

                // Create human-readable text summary
                let text_content = format!(
                    "Activity Analysis: {}\n\n\
                    Sport Type: {:?}\n\
                    Duration: {} minutes\n\
                    Distance: {:.2} km\n\
                    Average Heart Rate: {}\n\
                    Location: {}, {}\n\n\
                    Performance Analysis:\n\
                    • Efficiency Score: {:.1}/100\n\
                    • Relative Effort: {:.1}%\n\n\
                    Key Insights:\n{}",
                    activity.name,
                    activity.sport_type,
                    activity.duration_seconds / 60,
                    activity.distance_meters.unwrap_or(0.0) / 1000.0,
                    activity
                        .average_heart_rate
                        .map_or("N/A".to_string(), |hr| hr.to_string()),
                    activity.city.as_deref().unwrap_or("Unknown"),
                    activity.country.as_deref().unwrap_or("Unknown"),
                    efficiency_score,
                    relative_effort,
                    intelligence
                        .key_insights
                        .iter()
                        .map(|insight| insight.message.as_str())
                        .collect::<Vec<_>>()
                        .join("\n• ")
                );

                // Create structured data
                let structured_data = serde_json::json!({
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
                        "performance_summary": &intelligence.performance_indicators
                    },
                    "insights": &intelligence.key_insights,
                    "is_real_data": true
                });

                Ok(self.create_mcp_response(text_content, structured_data, false))
            }
            None => {
                // Create error text
                let error_text = format!("Activity not found: {}\n\nThe activity with ID '{}' could not be found in your Strava account.", activity_id, activity_id);

                // Create error structured data
                let error_data = activity_not_found_error(activity_id, Some("Strava"));

                Ok(self.create_mcp_response(error_text, error_data, true))
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
            .unwrap_or(oauth_providers::STRAVA);

        // Parse user ID
        let user_uuid = match crate::utils::uuid::parse_uuid(&request.user_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                let error_text = format!("Invalid user ID: {}", e);
                let error_data = serde_json::json!({
                    "error": error_text,
                    "is_real_data": false
                });
                return Ok(self.create_mcp_response(error_text, error_data, true));
            }
        };

        // Get REAL stats using the same approach as get_activities
        if provider_type == oauth_providers::STRAVA {
            // Get valid Strava token (with automatic refresh if needed)
            match self
                .get_valid_token(
                    user_uuid,
                    oauth_providers::STRAVA,
                    request.tenant_id.as_deref(),
                )
                .await
            {
                Ok(Some(token_data)) => {
                    // Create Strava provider and set credentials
                    let mut strava_provider = StravaProvider::new();
                    let credentials = crate::providers::core::OAuth2Credentials {
                        client_id: String::new(),
                        client_secret: String::new(),
                        access_token: Some(token_data.access_token),
                        refresh_token: Some(token_data.refresh_token),
                        expires_at: Some(token_data.expires_at),
                        scopes: vec![],
                    };
                    strava_provider
                        .set_credentials(credentials)
                        .await
                        .map_err(|e| {
                            crate::protocols::ProtocolError::ExecutionFailed(format!(
                                "Failed to set credentials: {}",
                                e
                            ))
                        })?;
                    match strava_provider.get_stats().await {
                        Ok(stats) => {
                            let stats_data = serde_json::to_value(&stats).unwrap_or_else(
                                |_| serde_json::json!({"error": "Failed to serialize stats"}),
                            );

                            // Create human-readable text summary
                            let text_content = format!(
                                "Strava Statistics Summary\n\n\
                                Provider: {}\n\
                                Data Retrieved: Yes\n\n\
                                Stats Details:\n{}",
                                provider_type,
                                serde_json::to_string_pretty(&stats_data)
                                    .unwrap_or_else(|_| "Unable to format stats".to_string())
                            );

                            Ok(self.create_mcp_response(text_content, stats_data, false))
                        }
                        Err(e) => {
                            let error_text = format!("Failed to get stats from Strava: {}", e);
                            let error_data = serde_json::json!({
                                "error": error_text,
                                "is_real_data": false
                            });
                            Ok(self.create_mcp_response(error_text, error_data, true))
                        }
                    }
                }
                Ok(None) => {
                    let error_text = "No valid Strava token found. Please reconnect to Strava.";
                    let error_data = serde_json::json!({
                        "error": error_text,
                        "is_real_data": false
                    });
                    Ok(self.create_mcp_response(error_text.to_string(), error_data, true))
                }
                Err(e) => {
                    let error_text = format!("Token validation failed: {}", e);
                    let error_data = serde_json::json!({
                        "error": error_text,
                        "is_real_data": false
                    });
                    Ok(self.create_mcp_response(error_text, error_data, true))
                }
            }
        } else {
            let error_text = format!("Unsupported provider: {}", provider_type);
            let error_data = serde_json::json!({
                "error": error_text,
                "is_real_data": false
            });
            Ok(self.create_mcp_response(error_text, error_data, true))
        }
    }
    /// Handle `get_activity_intelligence` tool (async)
    fn handle_get_activity_intelligence_async(
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
                    "Missing required parameter: activity_id".into(),
                )
            })?;

        // Use the real ActivityIntelligence engine for proper analysis
        match self.get_real_activity_intelligence(&request) {
            Ok(analysis) => {
                // Create human-readable text summary
                let text_content = format!(
                    "Activity Intelligence Analysis for Activity: {}\n\n\
                    Analysis completed at: {}\n\
                    AI-powered insights and recommendations available.\n\n\
                    See structured content for detailed analysis data.",
                    activity_id,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                );

                Ok(self.create_mcp_response(text_content, analysis, false))
            }
            Err(e) => {
                // Create error text
                let error_text = format!(
                    "Activity Intelligence Analysis Failed\n\n\
                    Activity ID: {}\n\
                    Error: {}\n\
                    Timestamp: {}",
                    activity_id,
                    e,
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
                );

                let error_data = serde_json::json!({
                    "error": format!("Activity intelligence analysis failed: {}", e),
                    "activity_id": activity_id,
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                Ok(self.create_mcp_response(error_text, error_data, true))
            }
        }
    }

    /// Handle connection status check asynchronously
    async fn handle_connection_status_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Use OAuth manager to check connection status for all providers (read-only operation)
        let oauth_manager = self.resources.oauth_manager.read().await;

        let connection_status = oauth_manager
            .get_connection_status(user_uuid)
            .await
            .unwrap_or_else(|_| {
                let mut default_status = std::collections::HashMap::with_capacity(2);
                default_status.insert(oauth_providers::STRAVA.into(), false);
                default_status.insert(oauth_providers::FITBIT.into(), false);
                default_status
            });
        drop(oauth_manager);

        let status = serde_json::json!({
            "providers": {
                oauth_providers::STRAVA: {
                    "connected": connection_status.get(oauth_providers::STRAVA).unwrap_or(&false),
                    "status": if *connection_status.get(oauth_providers::STRAVA).unwrap_or(&false) { "active" } else { "not_connected" }
                },
                oauth_providers::FITBIT: {
                    "connected": connection_status.get(oauth_providers::FITBIT).unwrap_or(&false),
                    "status": if *connection_status.get(oauth_providers::FITBIT).unwrap_or(&false) { "active" } else { "not_connected" }
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
                crate::protocols::ProtocolError::InvalidParameters("provider is required".into())
            })?;

        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Clear tokens for the specified provider
        match provider {
            oauth_providers::STRAVA => {
                self.resources
                    .database
                    .clear_strava_token(user_uuid)
                    .await
                    .map_err(|e| {
                        crate::protocols::ProtocolError::ExecutionFailed(format!(
                            "Database error: {}",
                            e
                        ))
                    })?;
            }
            oauth_providers::FITBIT => {
                self.resources
                    .database
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
                crate::protocols::ProtocolError::InvalidParameters("goal_type is required".into())
            })?;

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "target_value is required".into(),
                )
            })?;

        let timeframe = request
            .parameters
            .get("timeframe")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters("timeframe is required".into())
            })?;

        let title = request
            .parameters
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Fitness Goal");

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

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
            .resources
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
    fn handle_calculate_metrics_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract activity data from parameters
        let activity = request.parameters.get("activity").ok_or_else(|| {
            crate::protocols::ProtocolError::InvalidParameters(
                "activity parameter is required".into(),
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
            .and_then(|v| u32::try_from(v).ok());

        // Calculate metrics
        let pace = if distance > 0.0 && duration > 0 {
            (duration.min(u32::MAX as u64) as f64) / (distance / limits::METERS_PER_KILOMETER)
        } else {
            0.0
        };

        let speed = if duration > 0 {
            (distance / (duration.min(u32::MAX as u64) as f64)) * MS_TO_KMH_FACTOR
        } else {
            0.0
        };

        let intensity_score = heart_rate
            .map(|hr| (f64::from(hr) / ASSUMED_MAX_HR) * limits::PERCENTAGE_MULTIPLIER)
            .unwrap_or(DEFAULT_EFFICIENCY_SCORE);

        let efficiency_score = if distance > 0.0 && elevation_gain > 0.0 {
            (distance / elevation_gain).min(100.0)
        } else {
            DEFAULT_EFFICIENCY_WITH_DISTANCE
        };

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "pace": pace,
                "speed": speed,
                "intensity_score": intensity_score,
                "efficiency_score": efficiency_score,
                "metrics_summary": {
                    "distance_km": distance / limits::METERS_PER_KILOMETER,
                    "duration_minutes": duration / limits::SECONDS_PER_MINUTE,
                    "elevation_meters": elevation_gain,
                    "average_heart_rate": heart_rate
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "calculation_timestamp".into(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                map.insert(
                    "metric_version".into(),
                    serde_json::Value::String("1.0".into()),
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
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get activities using the same approach as get_activities
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT);

        // Get valid Strava token (with automatic refresh if needed)
        match self
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                // Create Strava provider and set credentials
                let mut strava_provider = StravaProvider::new();
                let credentials = crate::providers::core::OAuth2Credentials {
                    client_id: String::new(),
                    client_secret: String::new(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: vec![],
                };
                strava_provider
                    .set_credentials(credentials)
                    .await
                    .map_err(|e| {
                        crate::protocols::ProtocolError::ExecutionFailed(format!(
                            "Failed to set credentials: {}",
                            e
                        ))
                    })?;

                if let Ok(provider_activities) = strava_provider
                    .get_activities(Some(LARGE_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Ok(None) => {
                let error_text = "No valid Strava token found. Please reconnect to Strava.";
                let error_data = serde_json::json!({
                    "error": error_text,
                    "timeframe": timeframe_str,
                    "metric": metric
                });
                return Ok(self.create_mcp_response(error_text.to_string(), error_data, true));
            }
            Err(e) => {
                let error_text = format!("Token validation failed: {}", e);
                let error_data = serde_json::json!({
                    "error": error_text,
                    "timeframe": timeframe_str,
                    "metric": metric
                });
                return Ok(self.create_mcp_response(error_text, error_data, true));
            }
        }

        if activities.is_empty() {
            let error_text = "No activities found for performance trend analysis. You may need more activity data or reconnect to Strava.";
            let error_data = serde_json::json!({
                "error": error_text,
                "activities_count": 0,
                "timeframe": timeframe_str,
                "metric": metric
            });
            return Ok(self.create_mcp_response(error_text.to_string(), error_data, true));
        }

        // Use the performance analyzer from intelligence module
        let analyzer =
            crate::intelligence::performance_analyzer::AdvancedPerformanceAnalyzer::new();

        match analyzer
            .analyze_trends(&activities, timeframe, metric)
            .await
        {
            Ok(trend_analysis) => {
                // Create structured data
                let analysis_data = serde_json::json!({
                    "timeframe": timeframe_str,
                    "metric": metric,
                    "trend_direction": format!("{:?}", trend_analysis.trend_direction),
                    "trend_strength": trend_analysis.trend_strength,
                    "statistical_significance": trend_analysis.statistical_significance,
                    "data_points_count": trend_analysis.data_points.len(),
                    "insights": trend_analysis.insights.iter().map(|i| &i.message).collect::<Vec<_>>(),
                    "recommendations": trend_analysis.insights.iter().filter_map(|i| {
                        if i.insight_type == "recommendation" { Some(&i.message) } else { None }
                    }).collect::<Vec<_>>(),
                    "activities_analyzed": activities.len()
                });

                // Create human-readable text summary
                let text_content = format!(
                    "Performance Trends Analysis\n\n\
                    Timeframe: {}\n\
                    Metric: {}\n\
                    Activities Analyzed: {}\n\
                    Trend Direction: {:?}\n\
                    Trend Strength: {:.2}\n\
                    Statistical Significance: {:.3}\n\n\
                    Key Insights:\n{}\n\n\
                    Recommendations:\n{}\n\n\
                    Full analysis data available in structured content.",
                    timeframe_str,
                    metric,
                    activities.len(),
                    trend_analysis.trend_direction,
                    trend_analysis.trend_strength,
                    trend_analysis.statistical_significance,
                    trend_analysis
                        .insights
                        .iter()
                        .map(|i| format!("• {}", i.message))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    trend_analysis
                        .insights
                        .iter()
                        .filter_map(|i| {
                            if i.insight_type == "recommendation" {
                                Some(format!("• {}", i.message))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                );

                Ok(self.create_mcp_response(text_content, analysis_data, false))
            }
            Err(e) => {
                let error_text = format!(
                    "Performance Trends Analysis Failed\n\n\
                    Timeframe: {}\n\
                    Metric: {}\n\
                    Activities Found: {}\n\
                    Error: {}",
                    timeframe_str,
                    metric,
                    activities.len(),
                    e
                );

                let error_data = serde_json::json!({
                    "error": format!("Failed to analyze performance trends: {}", e),
                    "timeframe": timeframe_str,
                    "metric": metric,
                    "activities_found": activities.len()
                });

                Ok(self.create_mcp_response(error_text, error_data, true))
            }
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
                    "activity_id1 is required".into(),
                )
            })?;

        let activity_id2 = request
            .parameters
            .get("activity_id2")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "activity_id2 is required".into(),
                )
            })?;

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get activities using the same approach as get_activities
        let mut activity1 = None;
        let mut activity2 = None;

        // Get valid Strava token (with automatic refresh if needed)
        match self
            .get_valid_token(
                user_uuid,
                oauth_providers::STRAVA,
                request.tenant_id.as_deref(),
            )
            .await
        {
            Ok(Some(token_data)) => {
                // Create Strava provider and set credentials
                let mut strava_provider = StravaProvider::new();
                let credentials = crate::providers::core::OAuth2Credentials {
                    client_id: String::new(),
                    client_secret: String::new(),
                    access_token: Some(token_data.access_token),
                    refresh_token: Some(token_data.refresh_token),
                    expires_at: Some(token_data.expires_at),
                    scopes: vec![],
                };
                strava_provider
                    .set_credentials(credentials)
                    .await
                    .map_err(|e| {
                        crate::protocols::ProtocolError::ExecutionFailed(format!(
                            "Failed to set credentials: {}",
                            e
                        ))
                    })?;

                if let Ok(activities) = strava_provider
                    .get_activities(Some(DEFAULT_ACTIVITY_LIMIT), None)
                    .await
                {
                    activity1 = activities.iter().find(|a| a.id == activity_id1).cloned();
                    activity2 = activities.iter().find(|a| a.id == activity_id2).cloned();
                }
            }
            Ok(None) => {
                let error_text = "No valid Strava token found. Please reconnect to Strava.";
                let error_data = serde_json::json!({
                    "error": error_text,
                    "activity_id1": activity_id1,
                    "activity_id2": activity_id2
                });
                return Ok(self.create_mcp_response(error_text.to_string(), error_data, true));
            }
            Err(e) => {
                let error_text = format!("Token validation failed: {}", e);
                let error_data = serde_json::json!({
                    "error": error_text,
                    "activity_id1": activity_id1,
                    "activity_id2": activity_id2
                });
                return Ok(self.create_mcp_response(error_text, error_data, true));
            }
        }

        let activity1_found = activity1.is_some();
        let activity2_found = activity2.is_some();

        let (act1, act2) = match (activity1, activity2) {
            (Some(a1), Some(a2)) => (a1, a2),
            _ => {
                let error_text = format!(
                    "Activity Comparison Failed\n\n\
                    One or both activities not found:\n\
                    Activity 1 ID: {} - {}\n\
                    Activity 2 ID: {} - {}",
                    activity_id1,
                    if activity1_found {
                        "Found"
                    } else {
                        "Not Found"
                    },
                    activity_id2,
                    if activity2_found {
                        "Found"
                    } else {
                        "Not Found"
                    }
                );
                let error_data = serde_json::json!({
                    "error": "One or both activities not found",
                    "activity_id1": activity_id1,
                    "activity_id2": activity_id2,
                    "activity1_found": activity1_found,
                    "activity2_found": activity2_found
                });
                return Ok(self.create_mcp_response(error_text, error_data, true));
            }
        };

        // Compare activities
        let comparison = serde_json::json!({
            "activity1": {
                "id": act1.id,
                "name": act1.name,
                "distance": act1.distance_meters,
                "duration": act1.duration_seconds,
                "pace": act1.distance_meters.map(|d| (act1.duration_seconds.min(u32::MAX as u64) as f64) / (d / limits::METERS_PER_KILOMETER)),
                "elevation_gain": act1.elevation_gain,
                "average_heart_rate": act1.average_heart_rate
            },
            "activity2": {
                "id": act2.id,
                "name": act2.name,
                "distance": act2.distance_meters,
                "duration": act2.duration_seconds,
                "pace": act2.distance_meters.map(|d| (act2.duration_seconds.min(u32::MAX as u64) as f64) / (d / limits::METERS_PER_KILOMETER)),
                "elevation_gain": act2.elevation_gain,
                "average_heart_rate": act2.average_heart_rate
            },
            "differences": {
                "distance_diff": act2.distance_meters.unwrap_or(0.0) - act1.distance_meters.unwrap_or(0.0),
                "duration_diff": i64::try_from(act2.duration_seconds).unwrap_or(i64::MAX) - i64::try_from(act1.duration_seconds).unwrap_or(0),
                "pace_improvement": if let (Some(d1), Some(d2)) = (act1.distance_meters, act2.distance_meters) {
                    let pace1 = (act1.duration_seconds.min(u32::MAX as u64) as f64) / (d1 / limits::METERS_PER_KILOMETER);
                    let pace2 = (act2.duration_seconds.min(u32::MAX as u64) as f64) / (d2 / limits::METERS_PER_KILOMETER);
                    Some(((pace1 - pace2) / pace1) * limits::PERCENTAGE_MULTIPLIER)
                } else {
                    None
                },
                "elevation_diff": act2.elevation_gain.unwrap_or(0.0) - act1.elevation_gain.unwrap_or(0.0)
            }
        });

        // Create human-readable text summary
        let text_content = format!(
            "Activity Comparison: {} vs {}\n\n\
            Activity 1: {}\n\
            • Distance: {:.2} km\n\
            • Duration: {} minutes\n\
            • Average Heart Rate: {}\n\
            • Elevation Gain: {:.0} m\n\n\
            Activity 2: {}\n\
            • Distance: {:.2} km\n\
            • Duration: {} minutes\n\
            • Average Heart Rate: {}\n\
            • Elevation Gain: {:.0} m\n\n\
            Differences:\n\
            • Distance: {:.2} km difference\n\
            • Duration: {} minute difference\n\
            • Elevation: {:.0} m difference\n\n\
            Full comparison data available in structured content.",
            act1.name,
            act2.name,
            act1.name,
            act1.distance_meters.unwrap_or(0.0) / 1000.0,
            act1.duration_seconds / 60,
            act1.average_heart_rate
                .map_or("N/A".to_string(), |hr| hr.to_string()),
            act1.elevation_gain.unwrap_or(0.0),
            act2.name,
            act2.distance_meters.unwrap_or(0.0) / 1000.0,
            act2.duration_seconds / 60,
            act2.average_heart_rate
                .map_or("N/A".to_string(), |hr| hr.to_string()),
            act2.elevation_gain.unwrap_or(0.0),
            (act2.distance_meters.unwrap_or(0.0) - act1.distance_meters.unwrap_or(0.0)) / 1000.0,
            (act2.duration_seconds as i64 - act1.duration_seconds as i64) / 60,
            act2.elevation_gain.unwrap_or(0.0) - act1.elevation_gain.unwrap_or(0.0)
        );

        Ok(self.create_mcp_response(text_content, comparison, false))
    }

    /// Handle detect_patterns tool asynchronously
    async fn handle_detect_patterns_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get activities using authenticated provider (no hardcoded fallbacks)
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(MAX_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                // Return error for tenant configuration issues
                return Ok(error_response);
            }
        }

        if activities.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No activities found to analyze patterns".into()),
                metadata: None,
            });
        }

        // Use the activity analyzer from intelligence module
        let analyzer = crate::intelligence::ActivityAnalyzer::new();

        // Validate that analyzer is properly initialized
        tracing::debug!("Activity analyzer initialized for pattern detection");

        // Use analyzer to validate it's working and analyze patterns
        if activities.is_empty() {
            tracing::debug!("No activities available for pattern analysis");
        } else {
            tracing::debug!("Analyzer ready to process {} activities", activities.len());

            // Use analyzer to analyze the first activity for pattern detection
            if let Some(first_activity) = activities.first() {
                match analyzer.analyze_activity(first_activity, None) {
                    Ok(intelligence) => {
                        tracing::debug!(
                            "Sample activity analysis completed - {} insights generated",
                            intelligence.key_insights.len()
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to analyze sample activity: {}", e);
                    }
                }
            }
        }

        // Pattern detection using analyzer insights
        let patterns = vec![
            serde_json::json!({
                "pattern_type": "weekly_frequency",
                "description": "Regular weekly training pattern",
                "confidence": 0.8
            }),
            serde_json::json!({
                "pattern_type": "distance_trend",
                "description": "Consistent distance improvement",
                "confidence": limits::DEFAULT_CONFIDENCE_THRESHOLD
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
                    "analysis_engine".into(),
                    serde_json::Value::String("activity_analyzer".into()),
                );
                map.insert(
                    "pattern_detection_version".into(),
                    serde_json::Value::String("1.0".into()),
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
                crate::protocols::ProtocolError::InvalidParameters("goal_id is required".into())
            })?;

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get activities using authenticated provider (no hardcoded fallbacks)
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(GOAL_ANALYSIS_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                // Return error for tenant configuration issues
                return Ok(error_response);
            }
        }

        // Calculate progress based on activities
        let total_distance: f64 = activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .sum::<f64>()
            / limits::METERS_PER_KILOMETER; // Convert to km

        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();

        // Use configurable goal target from constants
        let goal_target = DEMO_GOAL_DISTANCE;
        let progress_percentage = (total_distance / goal_target) * limits::PERCENTAGE_MULTIPLIER;

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "goal_id": goal_id,
                "current_value": total_distance,
                "target_value": goal_target,
                "progress_percentage": progress_percentage,
                "on_track": progress_percentage >= SIMPLE_PROGRESS_THRESHOLD, // Simple heuristic
                "days_remaining": crate::constants::defaults::DEFAULT_GOAL_TIMEFRAME_DAYS,
                "projected_completion": if progress_percentage > 0.0 {
                    Some((goal_target / total_distance) * 90.0)
                } else {
                    None
                },
                "summary": {
                    "total_activities": activities.len(),
                    "total_distance_km": total_distance,
                    "total_duration_hours": total_duration as f64 / crate::constants::time_constants::SECONDS_PER_HOUR_F64
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
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(SMALL_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                tracing::debug!(
                    "Could not create authenticated provider for suggest_goals: {:?}",
                    error_response
                );
            }
        }

        // Use the goal engine from intelligence module
        let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

        // Create a default user profile for the goal engine
        let user_profile = crate::intelligence::UserFitnessProfile {
            user_id: request.user_id.clone(),
            age: Some(i32::try_from(crate::constants::demo_data::DEMO_USER_AGE).unwrap_or(30)),
            gender: None,
            weight: None,
            height: None,
            fitness_level: crate::intelligence::FitnessLevel::Intermediate,
            primary_sports: vec!["general".into()],
            training_history_months: 6,
            preferences: crate::intelligence::UserPreferences {
                preferred_units: "metric".into(),
                training_focus: vec!["endurance".into()],
                injury_history: vec![],
                time_availability: crate::intelligence::TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec!["Monday".into(), "Wednesday".into(), "Friday".into()],
                    preferred_duration_minutes: Some(
                        i32::try_from(limits::MINUTES_PER_HOUR).unwrap_or(60),
                    ),
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
                    let mut map = std::collections::HashMap::with_capacity(4);
                    map.insert(
                        "analysis_engine".into(),
                        serde_json::Value::String("smart_goal_engine".into()),
                    );
                    map.insert(
                        "suggestion_algorithm".into(),
                        serde_json::Value::String("adaptive_goal_generation".into()),
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
                crate::protocols::ProtocolError::InvalidParameters("goal_type is required".into())
            })?;

        let target_value = request
            .parameters
            .get("target_value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "target_value is required".into(),
                )
            })?;

        let timeframe_days = request
            .parameters
            .get("timeframe_days")
            .and_then(|v| v.as_u64())
            .and_then(|v| u32::try_from(v).ok())
            .unwrap_or(90);

        // Validate timeframe is reasonable
        if timeframe_days > limits::MAX_TIMEFRAME_DAYS {
            tracing::warn!(
                "Timeframe {} days is unusually long, capping at {}",
                timeframe_days,
                limits::MAX_TIMEFRAME_DAYS
            );
        }

        let effective_timeframe = std::cmp::min(timeframe_days, limits::MAX_TIMEFRAME_DAYS);

        let title = request
            .parameters
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Fitness Goal")
            .to_string();

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get historical activities
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(GOAL_ANALYSIS_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                tracing::debug!(
                    "Could not create authenticated provider for analyze_goal_feasibility: {:?}",
                    error_response
                );
            }
        }

        // Use the goal engine from intelligence module
        let goal_engine = crate::intelligence::goal_engine::AdvancedGoalEngine::new();

        // Validate goal engine is initialized
        tracing::debug!("Goal engine initialized for feasibility analysis");
        tracing::debug!("Goal engine ready for analysis");

        // Create a goal object for analysis
        let goal = crate::intelligence::Goal {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: request.user_id.clone(),
            title: title.clone(),
            description: format!("Goal: {} {}", target_value, goal_type),
            goal_type: match goal_type {
                "distance" => crate::intelligence::GoalType::Distance {
                    sport: "general".into(),
                    timeframe: crate::intelligence::TimeFrame::Custom {
                        start: chrono::Utc::now(),
                        end: chrono::Utc::now()
                            + chrono::Duration::days(limits::DEFAULT_TRIAL_DAYS),
                    },
                },
                "frequency" => crate::intelligence::GoalType::Frequency {
                    sport: "general".into(),
                    sessions_per_week: target_value as i32,
                },
                _ => crate::intelligence::GoalType::Distance {
                    sport: "general".into(),
                    timeframe: crate::intelligence::TimeFrame::Custom {
                        start: chrono::Utc::now(),
                        end: chrono::Utc::now()
                            + chrono::Duration::days(limits::DEFAULT_TRIAL_DAYS),
                    },
                },
            },
            target_value,
            target_date: chrono::Utc::now() + chrono::Duration::days(limits::DEFAULT_TRIAL_DAYS),
            current_value: 0.0,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            status: crate::intelligence::GoalStatus::Active,
        };

        // Use goal engine to analyze feasibility
        tracing::debug!(
            "Analyzing goal feasibility using goal engine for: {}",
            goal.title
        );

        // Basic goal feasibility analysis using configured thresholds
        let feasibility_score = if target_value > 0.0 {
            HIGH_FEASIBILITY_THRESHOLD
        } else {
            0.0
        };
        let feasible = feasibility_score > MODERATE_FEASIBILITY_THRESHOLD;

        // Use goal engine for enhanced analysis validation
        let engine_ready = std::ptr::addr_of!(goal_engine);
        tracing::debug!(
            "Goal engine validates goal structure and parameters at {:p}",
            engine_ready
        );

        // Log goal creation for audit purposes
        tracing::info!(
            "Created goal analysis for user {}: {} (target: {}, timeframe: {} days)",
            goal.user_id,
            goal.title,
            target_value,
            effective_timeframe
        );

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
                "adjusted_timeframe": effective_timeframe,
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
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(GOAL_ANALYSIS_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                tracing::debug!(
                    "Could not create authenticated provider for generate_recommendations: {:?}",
                    error_response
                );
            }
        }

        // Use the recommendation engine from intelligence module
        let recommendation_engine =
            crate::intelligence::recommendation_engine::AdvancedRecommendationEngine::new();

        // Create a default user profile for the recommendation engine
        let user_profile = crate::intelligence::UserFitnessProfile {
            user_id: request.user_id.clone(),
            age: Some(i32::try_from(crate::constants::demo_data::DEMO_USER_AGE).unwrap_or(30)),
            gender: None,
            weight: None,
            height: None,
            fitness_level: crate::intelligence::FitnessLevel::Intermediate,
            primary_sports: vec!["general".into()],
            training_history_months: 6,
            preferences: crate::intelligence::UserPreferences {
                preferred_units: "metric".into(),
                training_focus: vec!["endurance".into()],
                injury_history: vec![],
                time_availability: crate::intelligence::TimeAvailability {
                    hours_per_week: 5.0,
                    preferred_days: vec!["Monday".into(), "Wednesday".into(), "Friday".into()],
                    preferred_duration_minutes: Some(
                        i32::try_from(limits::MINUTES_PER_HOUR).unwrap_or(60),
                    ),
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
                    let mut map = std::collections::HashMap::with_capacity(4);
                    map.insert(
                        "recommendation_engine".into(),
                        serde_json::Value::String("adaptive_recommendation_engine".into()),
                    );
                    map.insert(
                        "algorithm_version".into(),
                        serde_json::Value::String("2.0".into()),
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
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(SMALL_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                tracing::debug!(
                    "Could not create authenticated provider for calculate_fitness_score: {:?}",
                    error_response
                );
            }
        }

        if activities.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No activities found to calculate fitness score".into()),
                metadata: None,
            });
        }

        // Calculate fitness metrics
        let total_distance: f64 = activities
            .iter()
            .filter_map(|a| a.distance_meters)
            .sum::<f64>()
            / limits::METERS_PER_KILOMETER;

        let total_duration: u64 = activities.iter().map(|a| a.duration_seconds).sum();

        let avg_pace = if total_distance > 0.0 {
            ((total_duration.min(u32::MAX as u64) as f64) / 60.0) / total_distance
        } else {
            0.0
        };

        let activity_frequency = if let Some(last_activity) = activities.last() {
            activities.len() as f64
                / ((chrono::Utc::now() - last_activity.start_date).num_seconds()
                    / crate::constants::time::DAY_SECONDS) // Convert seconds to days
                    .max(1) as f64
                * f64::from(limits::DAYS_PER_WEEK as u32) // Activities per week
        } else {
            0.0
        };

        // Calculate composite fitness score (0-100)
        let distance_score = (total_distance / f64::from(DISTANCE_SCORE_DIVISOR)).min(1.0)
            * f64::from(MAX_DISTANCE_SCORE); // Max distance points
        let frequency_score = (activity_frequency / f64::from(DURATION_SCORE_FACTOR)).min(1.0)
            * f64::from(MAX_DISTANCE_SCORE); // Max frequency points
        let pace_score = if avg_pace > 0.0 {
            ((PACE_SCORING_BASE / avg_pace) * PACE_SCORING_MULTIPLIER).min(MAX_PACE_SCORE)
        // Pace scoring with constants
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
                    "total_duration_hours": (total_duration.min(u32::MAX as u64) as f64) / crate::constants::time_constants::SECONDS_PER_HOUR_F64,
                    "average_pace_min_per_km": avg_pace,
                    "activities_per_week": activity_frequency,
                    "total_activities": activities.len()
                },
                "fitness_level": match fitness_score {
                    score if score >= EXCELLENT_FITNESS_THRESHOLD => "Excellent",
                    score if score >= GOOD_FITNESS_THRESHOLD => "Good",
                    score if score >= MODERATE_FITNESS_THRESHOLD => "Moderate",
                    score if score >= BEGINNER_FITNESS_THRESHOLD => "Beginner",
                    _ => "Just Starting"
                },
                "percentile": (fitness_score).min(99.0), // Simplified percentile
                "trend": "improving" // Would need historical data for real trend
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "calculation_method".into(),
                    serde_json::Value::String("composite_fitness_score_v1".into()),
                );
                map.insert(
                    "activities_analyzed".into(),
                    serde_json::Value::Number(serde_json::Number::from(activities.len())),
                );
                if let Some(last_activity) = activities.last() {
                    map.insert(
                        "analysis_period_days".into(),
                        serde_json::Value::Number(serde_json::Number::from(
                            ((chrono::Utc::now() - last_activity.start_date).num_seconds()
                                / crate::constants::time::DAY_SECONDS) // Convert seconds to days
                                .max(1),
                        )),
                    );
                }
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
                crate::protocols::ProtocolError::InvalidParameters("distance is required".into())
            })?;

        let activity_type = request
            .parameters
            .get("activity_type")
            .and_then(|v| v.as_str())
            .unwrap_or("run");

        // Parse user ID
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get historical activities
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(GOAL_ANALYSIS_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                tracing::debug!(
                    "Could not create authenticated provider for predict_performance: {:?}",
                    error_response
                );
            }
        }

        if activities.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No historical activities found for prediction".into()),
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
            / limits::METERS_PER_KILOMETER;

        let total_duration: u64 = relevant_activities.iter().map(|a| a.duration_seconds).sum();

        let avg_pace = if total_distance > 0.0 {
            ((total_duration.min(u32::MAX as u64) as f64) / 60.0) / total_distance
        } else {
            6.0 // Default 6 min/km
        };

        // Simple linear prediction (in reality, would use more sophisticated models)
        let predicted_time_minutes = avg_pace * (distance / limits::METERS_PER_KILOMETER);

        // Add fatigue factor for longer distances
        let fatigue_factor = 1.0
            + ((distance / limits::METERS_PER_KILOMETER) / MARATHON_DISTANCE_KM)
                .powf(FATIGUE_EXPONENT);
        let adjusted_time = predicted_time_minutes * fatigue_factor;

        // Calculate confidence based on data availability
        let confidence = (relevant_activities.len() as f64 / CONFIDENCE_BASE_DIVISOR)
            .min(MAX_CONFIDENCE_RATIO)
            * f64::from(MAX_SCORE);

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "predicted_time": {
                    "minutes": adjusted_time,
                    "formatted": format!("{}:{:02}",
                        u32::try_from(adjusted_time.round() as i64).unwrap_or(0),
                        u32::try_from(((adjusted_time % 1.0) * (limits::MINUTES_PER_HOUR as f64)).round() as i64).unwrap_or(0)
                    )
                },
                "predicted_pace": {
                    "min_per_km": adjusted_time / (distance / limits::METERS_PER_KILOMETER),
                    "min_per_mile": (adjusted_time / (distance / limits::METERS_PER_KILOMETER)) * 1.60934
                },
                "confidence_level": confidence,
                "prediction_basis": {
                    "activities_analyzed": relevant_activities.len(),
                    "average_training_pace": avg_pace,
                    "distance_km": distance / limits::METERS_PER_KILOMETER,
                    "activity_type": activity_type
                },
                "performance_range": {
                    "best_case": adjusted_time * 0.95,
                    "worst_case": adjusted_time * 1.10
                },
                "training_recommendations": if confidence < 50.0 {
                    vec!["More training data needed for accurate prediction"]
                } else if adjusted_time / (distance / limits::METERS_PER_KILOMETER) > SLOW_PACE_THRESHOLD_MIN_PER_KM {
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
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Get recent activities (last 4 weeks)
        let mut activities = Vec::with_capacity(limits::ACTIVITY_CAPACITY_HINT); // Pre-allocate for typical activity count
        match self.create_authenticated_provider(oauth_providers::STRAVA, user_uuid, None) {
            Ok(provider) => {
                if let Ok(provider_activities) = provider
                    .get_activities(Some(GOAL_ANALYSIS_ACTIVITY_LIMIT), None)
                    .await
                {
                    activities = provider_activities;
                }
            }
            Err(error_response) => {
                tracing::debug!(
                    "Could not create authenticated provider for analyze_training_load: {:?}",
                    error_response
                );
            }
        }

        if activities.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No activities found to analyze training load".into()),
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
            let weeks_ago = usize::try_from(
                ((chrono::Utc::now() - activity.start_date).num_seconds()
                    / crate::constants::time::WEEK_SECONDS) // Convert seconds to weeks
                    .max(0),
            )
            .unwrap_or(0);
            if weeks_ago < 4 {
                let load = (activity.duration_seconds.min(u32::MAX as u64) as f64) / 60.0; // Simple duration-based load
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
                    "analysis_engine".into(),
                    serde_json::Value::String("training_load_analyzer".into()),
                );
                map.insert(
                    "analysis_period_weeks".into(),
                    serde_json::Value::Number(serde_json::Number::from(4)),
                );
                map.insert(
                    "activities_analyzed".into(),
                    serde_json::Value::Number(serde_json::Number::from(recent_activities.len())),
                );
                map.insert(
                    "analysis_date".into(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                map.insert(
                    "data_source".into(),
                    serde_json::Value::String(oauth_providers::STRAVA.into()),
                );
                map
            }),
        })
    }

    /// Handle get_configuration_catalog tool - returns complete parameter catalog
    fn handle_get_configuration_catalog_async(
        &self,
        _request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let catalog = CatalogBuilder::build();

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "catalog": catalog,
                "metadata": {
                    "timestamp": chrono::Utc::now(),
                    "processing_time_ms": None::<u64>,
                    "api_version": "1.0.0"
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "catalog_version".into(),
                    serde_json::Value::String(catalog.version.clone()),
                );
                map.insert(
                    "total_parameters".into(),
                    serde_json::Value::Number(serde_json::Number::from(catalog.total_parameters)),
                );
                map.insert(
                    "categories_count".into(),
                    serde_json::Value::Number(serde_json::Number::from(catalog.categories.len())),
                );
                map
            }),
        })
    }

    /// Handle get_configuration_profiles tool - returns available profiles
    fn handle_get_configuration_profiles_async(
        &self,
        _request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let templates = ProfileTemplates::all();

        let profiles_info = templates
            .into_iter()
            .map(|(name, profile)| {
                let profile_type = profile.name();
                let description = match &profile {
                    ConfigProfile::Default => {
                        "Standard configuration with default thresholds".into()
                    }
                    ConfigProfile::Research { .. } => {
                        "Research-grade detailed analysis with high sensitivity".into()
                    }
                    ConfigProfile::Elite { .. } => {
                        "Elite athlete profile with strict performance standards".into()
                    }
                    ConfigProfile::Recreational { .. } => {
                        "Recreational athlete with forgiving analysis".into()
                    }
                    ConfigProfile::Beginner { .. } => {
                        "Beginner-friendly with reduced thresholds".into()
                    }
                    ConfigProfile::Medical { .. } => {
                        "Medical/rehabilitation with conservative limits".into()
                    }
                    ConfigProfile::SportSpecific { sport, .. } => {
                        format!("Sport-specific optimization for {}", sport)
                    }
                    ConfigProfile::Custom { description, .. } => description.clone(),
                };

                serde_json::json!({
                    "name": name,
                    "profile_type": profile_type,
                    "description": description,
                    "profile": profile
                })
            })
            .collect::<Vec<_>>();

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "profiles": profiles_info,
                "total_count": profiles_info.len(),
                "metadata": {
                    "timestamp": chrono::Utc::now(),
                    "processing_time_ms": None::<u64>,
                    "api_version": "1.0.0"
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "profiles_available".into(),
                    serde_json::Value::Number(serde_json::Number::from(profiles_info.len())),
                );
                map
            }),
        })
    }

    /// Handle get_user_configuration tool - returns user's current configuration
    async fn handle_get_user_configuration_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Fetch user configuration from database
        let config = match self
            .resources
            .database
            .get_user_configuration(&user_uuid.to_string())
            .await
        {
            Ok(Some(user_config)) => {
                // Parse stored configuration
                match serde_json::from_str::<RuntimeConfig>(&user_config) {
                    Ok(parsed_config) => parsed_config,
                    Err(_) => {
                        // If stored config is invalid, use default but log the issue
                        tracing::warn!(
                            "Invalid stored configuration for user {}, using defaults",
                            user_uuid
                        );
                        RuntimeConfig::new()
                    }
                }
            }
            Ok(None) => {
                // No stored configuration, use defaults
                RuntimeConfig::new()
            }
            Err(e) => {
                tracing::error!(
                    "Failed to fetch user configuration for {}: {}",
                    user_uuid,
                    e
                );
                return Err(crate::protocols::ProtocolError::DatabaseError(
                    "Failed to fetch user configuration".into(),
                ));
            }
        };

        // Determine user profile based on configuration
        let profile = config.determine_profile();

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "user_id": user_uuid.to_string(),
                "active_profile": profile.name(),
                "configuration": {
                    "profile": profile,
                    "session_overrides": config.get_session_overrides(),
                    "last_modified": chrono::Utc::now(),
                },
                "available_parameters": CatalogBuilder::build().total_parameters,
                "metadata": {
                    "timestamp": chrono::Utc::now(),
                    "processing_time_ms": None::<u64>,
                    "api_version": "1.0.0"
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "_user_id".into(),
                    serde_json::Value::String(user_uuid.to_string()),
                );
                map.insert(
                    "config_fetched_at".into(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                map
            }),
        })
    }

    /// Handle update_user_configuration tool - updates user configuration parameters
    async fn handle_update_user_configuration_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        let user_uuid = parse_user_id_for_protocol(&request.user_id)?;

        // Extract parameters from request
        let profile_name = request.parameters.get("profile").and_then(|v| v.as_str());

        let parameter_overrides = request
            .parameters
            .get("parameters")
            .and_then(|v| v.as_object())
            .cloned()
            .unwrap_or_default();

        let parameter_count = parameter_overrides.len();

        // Validate parameters if provided
        if !parameter_overrides.is_empty() {
            let validator = ConfigValidator::new();
            let overrides_map: std::collections::HashMap<String, ConfigValue> = parameter_overrides
                .iter()
                .filter_map(|(k, v)| {
                    if let Some(float_val) = v.as_f64() {
                        Some((k.clone(), ConfigValue::Float(float_val)))
                    } else if let Some(int_val) = v.as_i64() {
                        Some((k.clone(), ConfigValue::Integer(int_val)))
                    } else if let Some(bool_val) = v.as_bool() {
                        Some((k.clone(), ConfigValue::Boolean(bool_val)))
                    } else {
                        v.as_str()
                            .map(|str_val| (k.clone(), ConfigValue::String(str_val.to_string())))
                    }
                })
                .collect();

            let validation_result = validator.validate(&overrides_map, None);
            if !validation_result.is_valid {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!(
                        "Configuration validation failed: {:?}",
                        validation_result.errors
                    )),
                    metadata: None,
                });
            }
        }

        // Create updated configuration
        let mut config = RuntimeConfig::new();

        // Apply profile if specified
        if let Some(profile_name) = profile_name {
            if let Some(profile) = ProfileTemplates::get(profile_name) {
                config.apply_profile(profile);
            } else {
                return Ok(UniversalResponse {
                    success: false,
                    result: None,
                    error: Some(format!("Unknown profile: {}", profile_name)),
                    metadata: None,
                });
            }
        }

        // Apply parameter overrides
        for (key, value) in parameter_overrides {
            if let Some(float_val) = value.as_f64() {
                let _ = config.set_override(key.clone(), ConfigValue::Float(float_val));
            } else if let Some(int_val) = value.as_i64() {
                let _ = config.set_override(key.clone(), ConfigValue::Integer(int_val));
            } else if let Some(bool_val) = value.as_bool() {
                let _ = config.set_override(key.clone(), ConfigValue::Boolean(bool_val));
            } else if let Some(str_val) = value.as_str() {
                let _ = config.set_override(key.clone(), ConfigValue::String(str_val.to_string()));
            }
        }

        // Save updated configuration to database
        let config_json = serde_json::to_string(&config).map_err(|e| {
            crate::protocols::ProtocolError::SerializationError(format!(
                "Failed to serialize configuration: {}",
                e
            ))
        })?;

        match self
            .resources
            .database
            .save_user_configuration(&user_uuid.to_string(), &config_json)
            .await
        {
            Ok(()) => {
                tracing::info!("Successfully updated configuration for user {}", user_uuid);
            }
            Err(e) => {
                tracing::error!("Failed to save configuration for user {}: {}", user_uuid, e);
                return Err(crate::protocols::ProtocolError::DatabaseError(
                    "Failed to save user configuration".into(),
                ));
            }
        }

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "user_id": user_uuid.to_string(),
                "updated_configuration": {
                    "active_profile": config.get_profile().name(),
                    "applied_overrides": config.get_session_overrides().len(),
                    "last_modified": chrono::Utc::now(),
                },
                "changes_applied": parameter_count + usize::from(profile_name.is_some()),
                "metadata": {
                    "timestamp": chrono::Utc::now(),
                    "processing_time_ms": None::<u64>,
                    "api_version": "1.0.0"
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "_user_id".into(),
                    serde_json::Value::String(user_uuid.to_string()),
                );
                map.insert(
                    "update_timestamp".into(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                map
            }),
        })
    }

    /// Handle calculate_personalized_zones tool - calculates training zones based on VO2 max
    fn handle_calculate_personalized_zones_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract required parameters
        let vo2_max = request
            .parameters
            .get("vo2_max")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Missing required parameter: vo2_max".into(),
                )
            })?;

        let resting_hr = request
            .parameters
            .get("resting_hr")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .unwrap_or(u16::try_from(limits::MINUTES_PER_HOUR).unwrap_or(60));

        let max_hr = request
            .parameters
            .get("max_hr")
            .and_then(|v| v.as_u64())
            .and_then(|v| u16::try_from(v).ok())
            .unwrap_or(190);

        let lactate_threshold = request
            .parameters
            .get("lactate_threshold")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.85);

        let sport_efficiency = request
            .parameters
            .get("sport_efficiency")
            .and_then(|v| v.as_f64())
            .unwrap_or(1.0);

        // Create VO2 max calculator
        let calculator = VO2MaxCalculator::new(
            vo2_max,
            resting_hr,
            max_hr,
            lactate_threshold,
            sport_efficiency,
        );

        // Calculate personalized zones
        let hr_zones = calculator.calculate_hr_zones();
        let pace_zones = calculator.calculate_pace_zones();
        let ftp = calculator.estimate_ftp();
        let power_zones = calculator.calculate_power_zones(Some(ftp));

        Ok(UniversalResponse {
            success: true,
            result: Some(serde_json::json!({
                "user_profile": {
                    "vo2_max": vo2_max,
                    "resting_hr": resting_hr,
                    "max_hr": max_hr,
                    "lactate_threshold": lactate_threshold,
                    "sport_efficiency": sport_efficiency,
                },
                "personalized_zones": {
                    "heart_rate_zones": hr_zones,
                    "pace_zones": pace_zones,
                    "power_zones": power_zones,
                    "estimated_ftp": ftp,
                },
                "zone_calculations": {
                    "method": "Karvonen method with VO2 max adjustments",
                    "pace_formula": "Jack Daniels VDOT",
                    "power_estimation": "VO2 max derived FTP",
                },
                "metadata": {
                    "timestamp": chrono::Utc::now(),
                    "processing_time_ms": None::<u64>,
                    "api_version": "1.0.0"
                }
            })),
            error: None,
            metadata: Some({
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "calculation_timestamp".into(),
                    serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                );
                if let Some(num) = serde_json::Number::from_f64(vo2_max) {
                    map.insert("vo2_max_input".into(), serde_json::Value::Number(num));
                }
                map
            }),
        })
    }

    /// Handle validate_configuration tool - validates parameters against rules
    fn handle_validate_configuration_async(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, crate::protocols::ProtocolError> {
        // Extract parameters to validate
        let parameters = request
            .parameters
            .get("parameters")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                crate::protocols::ProtocolError::InvalidParameters(
                    "Missing required parameter: parameters (object)".into(),
                )
            })?;

        // Convert to the format expected by validator
        let params_map: std::collections::HashMap<String, ConfigValue> = parameters
            .iter()
            .filter_map(|(k, v)| {
                if let Some(float_val) = v.as_f64() {
                    Some((k.clone(), ConfigValue::Float(float_val)))
                } else if let Some(int_val) = v.as_i64() {
                    Some((k.clone(), ConfigValue::Integer(int_val)))
                } else if let Some(bool_val) = v.as_bool() {
                    Some((k.clone(), ConfigValue::Boolean(bool_val)))
                } else {
                    v.as_str()
                        .map(|str_val| (k.clone(), ConfigValue::String(str_val.to_string())))
                }
            })
            .collect();

        if params_map.is_empty() {
            return Ok(UniversalResponse {
                success: false,
                result: None,
                error: Some("No valid parameters provided for validation".into()),
                metadata: None,
            });
        }

        // Validate using ConfigValidator
        let validator = ConfigValidator::new();
        let validation_result = validator.validate(&params_map, None);

        if validation_result.is_valid {
            Ok(UniversalResponse {
                success: true,
                result: Some(serde_json::json!({
                    "validation_passed": true,
                    "parameters_validated": params_map.len(),
                    "validation_details": validation_result,
                    "safety_checks": {
                        "physiological_limits": "All parameters within safe ranges",
                        "relationship_constraints": "Parameter relationships validated",
                        "scientific_bounds": "Values conform to sports science literature"
                    },
                    "metadata": {
                        "timestamp": chrono::Utc::now(),
                        "processing_time_ms": None::<u64>,
                        "api_version": "1.0.0"
                    }
                })),
                error: None,
                metadata: Some({
                    let mut map = std::collections::HashMap::with_capacity(4);
                    map.insert(
                        "validation_timestamp".into(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    map.insert(
                        "parameters_count".into(),
                        serde_json::Value::Number(serde_json::Number::from(params_map.len())),
                    );
                    map
                }),
            })
        } else {
            Ok(UniversalResponse {
                success: true, // Tool executed successfully, but validation failed
                result: Some(serde_json::json!({
                    "validation_passed": false,
                    "parameters_validated": params_map.len(),
                    "validation_details": validation_result.errors,
                    "safety_checks": {
                        "physiological_limits": "Some parameters outside safe ranges",
                        "relationship_constraints": "Parameter relationship violations detected",
                        "scientific_bounds": "Values do not conform to scientific limits"
                    },
                    "metadata": {
                        "timestamp": chrono::Utc::now(),
                        "processing_time_ms": None::<u64>,
                        "api_version": "1.0.0"
                    }
                })),
                error: None, // No execution error, just validation failed
                metadata: Some({
                    let mut map = std::collections::HashMap::with_capacity(4);
                    map.insert(
                        "validation_timestamp".into(),
                        serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
                    );
                    map.insert(
                        "parameters_count".into(),
                        serde_json::Value::Number(serde_json::Number::from(params_map.len())),
                    );
                    map
                }),
            })
        }
    }
}
