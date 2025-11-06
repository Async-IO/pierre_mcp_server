// ABOUTME: Clean universal executor that coordinates authentication, routing, and execution
// ABOUTME: Replaces monolithic universal.rs with composable services and type-safe routing
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::auth_service::AuthService;
use super::handlers::{
    handle_analyze_activity, handle_analyze_goal_feasibility, handle_analyze_meal_nutrition,
    handle_analyze_performance_trends, handle_analyze_sleep_quality, handle_analyze_training_load,
    handle_calculate_daily_nutrition, handle_calculate_fitness_score, handle_calculate_metrics,
    handle_calculate_personalized_zones, handle_calculate_recovery_score,
    handle_compare_activities, handle_connect_provider, handle_detect_patterns,
    handle_disconnect_provider, handle_generate_recommendations, handle_get_activities,
    handle_get_activity_intelligence, handle_get_athlete, handle_get_configuration_catalog,
    handle_get_configuration_profiles, handle_get_connection_status, handle_get_food_details,
    handle_get_nutrient_timing, handle_get_stats, handle_get_user_configuration,
    handle_optimize_sleep_schedule, handle_predict_performance, handle_search_food,
    handle_set_goal, handle_suggest_goals, handle_suggest_rest_day, handle_track_progress,
    handle_track_sleep_trends, handle_update_user_configuration, handle_validate_configuration,
};
use super::tool_registry::{ToolId, ToolInfo, ToolRegistry};
use crate::mcp::resources::ServerResources;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::sync::Arc;

/// Intelligence service interface for analysis operations
/// Provides abstraction layer for future intelligence module integration
pub struct IntelligenceService {
    _resources: Arc<ServerResources>,
}

impl IntelligenceService {
    /// Creates a new intelligence service instance
    #[must_use]
    pub const fn new(resources: Arc<ServerResources>) -> Self {
        Self {
            _resources: resources,
        }
    }

    /// Analyze activity data with intelligence engine
    ///
    /// # Errors
    /// Returns error if intelligence analysis fails
    pub fn analyze_activity(
        &self,
        activity: &crate::models::Activity,
    ) -> Result<serde_json::Value, String> {
        // Calculate basic efficiency score
        let efficiency_score = activity.distance_meters.map_or(crate::intelligence::physiological_constants::efficiency_defaults::DEFAULT_EFFICIENCY_WITH_DISTANCE, |distance| if activity.duration_seconds > 0 && distance > f64::from(crate::intelligence::physiological_constants::business_thresholds::MIN_VALID_DISTANCE) {
                let duration_f64 = f64::from(u32::try_from(activity.duration_seconds.min(u64::from(u32::MAX))).unwrap_or(u32::MAX));
                let speed_ms = distance / duration_f64;
                (speed_ms * f64::from(crate::intelligence::physiological_constants::business_thresholds::MAX_SCORE))
                    .min(f64::from(crate::intelligence::physiological_constants::business_thresholds::MAX_SCORE))
            } else {
                crate::intelligence::physiological_constants::efficiency_defaults::DEFAULT_EFFICIENCY_SCORE
            });

        // Calculate effort score based on duration and distance
        let effort_score = if activity.duration_seconds > 0 {
            let duration_hours = f64::from(
                u32::try_from(activity.duration_seconds.min(u64::from(u32::MAX)))
                    .unwrap_or(u32::MAX),
            ) / crate::constants::time_constants::SECONDS_PER_HOUR_F64;
            let base_effort = duration_hours * f64::from(crate::intelligence::physiological_constants::business_thresholds::DURATION_SCORE_FACTOR);

            // Add distance component if available
            activity.distance_meters.map_or(base_effort, |d| {
                let distance_km = d / 1000.0;
                base_effort + (distance_km / f64::from(crate::intelligence::physiological_constants::business_thresholds::DISTANCE_SCORE_DIVISOR))
            })
        } else {
            f64::from(crate::intelligence::physiological_constants::business_thresholds::DEFAULT_HR_EFFORT_SCORE)
        };

        Ok(serde_json::json!({
            "activity_id": activity.id,
            "analysis_type": "intelligence_engine",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "efficiency_score": efficiency_score,
            "effort_score": effort_score.min(f64::from(crate::intelligence::physiological_constants::business_thresholds::MAX_SCORE)),
            "performance_insights": {
                "efficiency_rating": if efficiency_score > 75.0 { "excellent" } else if efficiency_score > 50.0 { "good" } else { "needs_improvement" },
                "effort_level": if effort_score > 80.0 { "high" } else if effort_score > 40.0 { "moderate" } else { "low" }
            },
            "recommendations": Self::generate_activity_recommendations(activity, efficiency_score, effort_score)
        }))
    }

    /// Generate recommendations based on activity analysis
    fn generate_activity_recommendations(
        activity: &crate::models::Activity,
        efficiency_score: f64,
        effort_score: f64,
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        if efficiency_score < 50.0 {
            recommendations
                .push("Consider focusing on pacing strategy for improved efficiency".to_owned());
        }

        if effort_score > 80.0 {
            recommendations.push("High effort detected - ensure adequate recovery time".to_owned());
        }

        if activity.distance_meters.is_none() {
            recommendations.push("Track distance for more comprehensive analysis".to_owned());
        }

        if recommendations.is_empty() {
            recommendations.push("Great activity! Keep up the consistent training".to_owned());
        }

        recommendations
    }
}

/// Clean universal executor with separated concerns
/// No clippy suppressions needed - this is well-designed code
pub struct UniversalExecutor {
    /// Authentication service for handling OAuth and token validation
    pub auth_service: AuthService,
    /// Intelligence service for activity analysis and insights
    pub intelligence_service: IntelligenceService,
    /// Shared server resources (database, weather service, etc.)
    pub resources: Arc<ServerResources>,
    /// Tool registry mapping tool IDs to handlers
    registry: ToolRegistry,
}

impl UniversalExecutor {
    /// Create new executor with all services
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        let auth_service = AuthService::new(resources.clone()); // Safe: Arc clone for service creation
        let intelligence_service = IntelligenceService::new(resources.clone()); // Safe: Arc clone for service creation
        let mut registry = ToolRegistry::new();

        // Register all tools with their handlers
        Self::register_all_tools(&mut registry);

        Self {
            auth_service,
            intelligence_service,
            resources,
            registry,
        }
    }

    /// Register all tools with type-safe handlers
    fn register_strava_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::async_tool(
            ToolId::GetActivities,
            |executor, request| Box::pin(handle_get_activities(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetAthlete,
            |executor, request| Box::pin(handle_get_athlete(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetStats,
            |executor, request| Box::pin(handle_get_stats(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::AnalyzeActivity,
            |executor, request| Box::pin(handle_analyze_activity(executor, request)),
        ));
    }

    fn register_connection_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::async_tool(
            ToolId::GetConnectionStatus,
            |executor, request| Box::pin(handle_get_connection_status(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::ConnectProvider,
            |executor, request| Box::pin(handle_connect_provider(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::DisconnectProvider,
            |executor, request| Box::pin(handle_disconnect_provider(executor, request)),
        ));
    }

    fn register_configuration_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::sync_tool(
            ToolId::GetConfigurationCatalog,
            handle_get_configuration_catalog,
        ));
        registry.register(ToolInfo::sync_tool(
            ToolId::GetConfigurationProfiles,
            handle_get_configuration_profiles,
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetUserConfiguration,
            |executor, request| Box::pin(handle_get_user_configuration(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::UpdateUserConfiguration,
            |executor, request| Box::pin(handle_update_user_configuration(executor, request)),
        ));
        registry.register(ToolInfo::sync_tool(
            ToolId::CalculatePersonalizedZones,
            handle_calculate_personalized_zones,
        ));
        registry.register(ToolInfo::sync_tool(
            ToolId::ValidateConfiguration,
            handle_validate_configuration,
        ));
    }

    fn register_intelligence_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::sync_tool(
            ToolId::CalculateMetrics,
            handle_calculate_metrics,
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetActivityIntelligence,
            |executor, request| Box::pin(handle_get_activity_intelligence(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::AnalyzePerformanceTrends,
            |executor, request| Box::pin(handle_analyze_performance_trends(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::CompareActivities,
            |executor, request| Box::pin(handle_compare_activities(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::DetectPatterns,
            |executor, request| Box::pin(handle_detect_patterns(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GenerateRecommendations,
            |executor, request| Box::pin(handle_generate_recommendations(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::CalculateFitnessScore,
            |executor, request| Box::pin(handle_calculate_fitness_score(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::PredictPerformance,
            |executor, request| Box::pin(handle_predict_performance(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::AnalyzeTrainingLoad,
            |executor, request| Box::pin(handle_analyze_training_load(executor, request)),
        ));
    }

    fn register_goal_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::async_tool(
            ToolId::SetGoal,
            |executor, request| Box::pin(handle_set_goal(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::SuggestGoals,
            |executor, request| Box::pin(handle_suggest_goals(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::AnalyzeGoalFeasibility,
            |executor, request| Box::pin(handle_analyze_goal_feasibility(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::TrackProgress,
            |executor, request| Box::pin(handle_track_progress(executor, request)),
        ));
    }

    fn register_sleep_recovery_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::async_tool(
            ToolId::AnalyzeSleepQuality,
            |executor, request| Box::pin(handle_analyze_sleep_quality(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::CalculateRecoveryScore,
            |executor, request| Box::pin(handle_calculate_recovery_score(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::SuggestRestDay,
            |executor, request| Box::pin(handle_suggest_rest_day(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::TrackSleepTrends,
            |executor, request| Box::pin(handle_track_sleep_trends(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::OptimizeSleepSchedule,
            |executor, request| Box::pin(handle_optimize_sleep_schedule(executor, request)),
        ));
    }

    fn register_nutrition_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::async_tool(
            ToolId::CalculateDailyNutrition,
            |executor, request| Box::pin(handle_calculate_daily_nutrition(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetNutrientTiming,
            |executor, request| Box::pin(handle_get_nutrient_timing(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::SearchFood,
            |executor, request| Box::pin(handle_search_food(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetFoodDetails,
            |executor, request| Box::pin(handle_get_food_details(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::AnalyzeMealNutrition,
            |executor, request| Box::pin(handle_analyze_meal_nutrition(executor, request)),
        ));
    }

    fn register_all_tools(registry: &mut ToolRegistry) {
        Self::register_strava_tools(registry);
        Self::register_connection_tools(registry);
        Self::register_configuration_tools(registry);
        Self::register_intelligence_tools(registry);
        Self::register_goal_tools(registry);
        Self::register_sleep_recovery_tools(registry);
        Self::register_nutrition_tools(registry);
    }

    /// Execute a tool with type-safe routing (no string matching!)
    ///
    /// # Errors
    /// Returns `ProtocolError` if tool is not found or execution fails
    pub async fn execute_tool(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, ProtocolError> {
        // Convert string tool name to type-safe ID
        let tool_id = self
            .registry
            .resolve_tool_name(&request.tool_name)
            .ok_or_else(|| ProtocolError::ToolNotFound(request.tool_name.clone()))?; // Safe: String ownership needed for error message

        // Get registered tool info
        let tool_info = self.registry.get_tool(tool_id).ok_or_else(|| {
            ProtocolError::InternalError(format!("Tool {tool_id:?} not registered"))
        })?;

        // Convert to legacy UniversalToolExecutor for handler compatibility
        let legacy_executor = Self::new(self.resources.clone()); // Safe: Arc clone for legacy executor creation

        // Execute based on tool type
        match (tool_info.async_handler, tool_info.sync_handler) {
            (Some(async_handler), None) => {
                // Execute async handler
                async_handler(&legacy_executor, request).await
            }
            (None, Some(sync_handler)) => {
                // Execute sync handler
                sync_handler(&legacy_executor, &request)
            }
            _ => Err(ProtocolError::InternalError(format!(
                "Tool {tool_id:?} has invalid handler configuration"
            ))),
        }
    }

    /// List all available tools for MCP schema generation
    #[must_use]
    pub fn list_tools(&self) -> Vec<ToolId> {
        self.registry.list_tools()
    }

    /// Get tool metadata for documentation
    #[must_use]
    pub fn get_tool_info(&self, tool_id: ToolId) -> Option<(String, String, bool, bool)> {
        if self.registry.has_tool(tool_id) {
            Some((
                tool_id.name().to_owned(),
                tool_id.description().to_owned(),
                tool_id.requires_auth(),
                tool_id.is_async(),
            ))
        } else {
            None
        }
    }

    /// Check if executor has a specific tool
    #[must_use]
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.registry.resolve_tool_name(tool_name).is_some()
    }
}
