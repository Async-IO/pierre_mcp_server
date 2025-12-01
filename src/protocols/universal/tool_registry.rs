// ABOUTME: Type-safe tool registry replacing string-based routing
// ABOUTME: Eliminates string literals and provides compile-time safety for tool dispatch
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Type-safe tool identifier that replaces string-based routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    // Core Strava API tools
    /// Get user's fitness activities with optional filtering
    GetActivities,
    /// Get user's athlete profile and basic information
    GetAthlete,
    /// Get user's performance statistics and metrics
    GetStats,
    /// Analyze a specific activity with detailed performance insights
    AnalyzeActivity,
    /// Get AI-powered intelligence analysis for an activity
    GetActivityIntelligence,
    /// Check OAuth connection status for fitness providers
    GetConnectionStatus,
    /// Connect to Pierre MCP server (triggers OAuth flow)
    ConnectToPierre,
    /// Connect to a fitness data provider via OAuth
    ConnectProvider,
    /// Disconnect user from a fitness data provider
    DisconnectProvider,

    // OAuth notification tools
    /// Announce OAuth connection success to user
    AnnounceOAuthSuccess,
    /// Check for OAuth completion notifications
    CheckOAuthNotifications,
    /// Get list of OAuth notifications
    GetNotifications,
    /// Mark OAuth notifications as read
    MarkNotificationsRead,

    // Goal and planning tools
    /// Set a new fitness goal for the user
    SetGoal,
    /// Get AI-suggested fitness goals based on activity history
    SuggestGoals,
    /// Analyze whether a goal is achievable given current fitness level
    AnalyzeGoalFeasibility,
    /// Track progress towards fitness goals
    TrackProgress,

    // Analysis and intelligence tools
    /// Calculate custom fitness metrics and performance indicators
    CalculateMetrics,
    /// Analyze performance trends over time
    AnalyzePerformanceTrends,
    /// Compare two activities for performance analysis
    CompareActivities,
    /// Detect patterns and insights in activity data
    DetectPatterns,
    /// Generate personalized training recommendations
    GenerateRecommendations,
    /// Calculate overall fitness score based on recent activities
    CalculateFitnessScore,
    /// Predict future performance based on training patterns
    PredictPerformance,
    /// Analyze training load and recovery metrics
    AnalyzeTrainingLoad,

    // Configuration management tools
    /// Get the complete configuration catalog with all available parameters
    GetConfigurationCatalog,
    /// Get available configuration profiles (Research, Elite, Recreational, etc.)
    GetConfigurationProfiles,
    /// Get current user's configuration settings and overrides
    GetUserConfiguration,
    /// Update user's configuration parameters and session overrides
    UpdateUserConfiguration,
    /// Calculate personalized training zones based on user's VO2 max
    CalculatePersonalizedZones,
    /// Validate configuration parameters against safety rules
    ValidateConfiguration,

    // Sleep and recovery analysis tools
    /// Analyze sleep quality from Fitbit/Garmin data using NSF/AASM guidelines
    AnalyzeSleepQuality,
    /// Calculate holistic recovery score combining TSB, sleep quality, and HRV
    CalculateRecoveryScore,
    /// AI-powered rest day recommendation based on recovery indicators
    SuggestRestDay,
    /// Track sleep patterns and correlate with performance over time
    TrackSleepTrends,
    /// Optimize sleep duration based on training load and recovery needs
    OptimizeSleepSchedule,

    // Fitness configuration management tools
    /// Get user fitness configuration settings
    GetFitnessConfig,
    /// Set user fitness configuration settings
    SetFitnessConfig,
    /// List all fitness configuration names
    ListFitnessConfigs,
    /// Delete a fitness configuration
    DeleteFitnessConfig,

    // Nutrition analysis and USDA food database tools
    /// Calculate daily calorie and macronutrient needs using Mifflin-St Jeor BMR formula
    CalculateDailyNutrition,
    /// Get optimal pre/post-workout nutrition recommendations following ISSN guidelines
    GetNutrientTiming,
    /// Search USDA `FoodData` Central database for foods by name/description
    SearchFood,
    /// Get detailed nutritional information for a specific food from USDA database
    GetFoodDetails,
    /// Analyze total calories and macronutrients for a meal of multiple foods
    AnalyzeMealNutrition,
}

impl ToolId {
    /// Convert from string tool name to strongly-typed ID
    /// Returns None for unknown tool names
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "get_activities" => Some(Self::GetActivities),
            "get_athlete" => Some(Self::GetAthlete),
            "get_stats" => Some(Self::GetStats),
            "analyze_activity" => Some(Self::AnalyzeActivity),
            "get_activity_intelligence" => Some(Self::GetActivityIntelligence),
            "get_connection_status" => Some(Self::GetConnectionStatus),
            "connect_to_pierre" => Some(Self::ConnectToPierre),
            "connect_provider" => Some(Self::ConnectProvider),
            "disconnect_provider" => Some(Self::DisconnectProvider),
            "announce_oauth_success" => Some(Self::AnnounceOAuthSuccess),
            "check_oauth_notifications" => Some(Self::CheckOAuthNotifications),
            "get_notifications" => Some(Self::GetNotifications),
            "mark_notifications_read" => Some(Self::MarkNotificationsRead),
            "set_goal" => Some(Self::SetGoal),
            "suggest_goals" => Some(Self::SuggestGoals),
            "analyze_goal_feasibility" => Some(Self::AnalyzeGoalFeasibility),
            "track_progress" => Some(Self::TrackProgress),
            "calculate_metrics" => Some(Self::CalculateMetrics),
            "analyze_performance_trends" => Some(Self::AnalyzePerformanceTrends),
            "compare_activities" => Some(Self::CompareActivities),
            "detect_patterns" => Some(Self::DetectPatterns),
            "generate_recommendations" => Some(Self::GenerateRecommendations),
            "calculate_fitness_score" => Some(Self::CalculateFitnessScore),
            "predict_performance" => Some(Self::PredictPerformance),
            "analyze_training_load" => Some(Self::AnalyzeTrainingLoad),
            "get_configuration_catalog" => Some(Self::GetConfigurationCatalog),
            "get_configuration_profiles" => Some(Self::GetConfigurationProfiles),
            "get_user_configuration" => Some(Self::GetUserConfiguration),
            "update_user_configuration" => Some(Self::UpdateUserConfiguration),
            "calculate_personalized_zones" => Some(Self::CalculatePersonalizedZones),
            "validate_configuration" => Some(Self::ValidateConfiguration),
            "analyze_sleep_quality" => Some(Self::AnalyzeSleepQuality),
            "calculate_recovery_score" => Some(Self::CalculateRecoveryScore),
            "suggest_rest_day" => Some(Self::SuggestRestDay),
            "track_sleep_trends" => Some(Self::TrackSleepTrends),
            "optimize_sleep_schedule" => Some(Self::OptimizeSleepSchedule),
            "get_fitness_config" => Some(Self::GetFitnessConfig),
            "set_fitness_config" => Some(Self::SetFitnessConfig),
            "list_fitness_configs" => Some(Self::ListFitnessConfigs),
            "delete_fitness_config" => Some(Self::DeleteFitnessConfig),
            "calculate_daily_nutrition" => Some(Self::CalculateDailyNutrition),
            "get_nutrient_timing" => Some(Self::GetNutrientTiming),
            "search_food" => Some(Self::SearchFood),
            "get_food_details" => Some(Self::GetFoodDetails),
            "analyze_meal_nutrition" => Some(Self::AnalyzeMealNutrition),
            _ => None,
        }
    }

    /// Get the string name for this tool ID
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::GetActivities => "get_activities",
            Self::GetAthlete => "get_athlete",
            Self::GetStats => "get_stats",
            Self::AnalyzeActivity => "analyze_activity",
            Self::GetActivityIntelligence => "get_activity_intelligence",
            Self::GetConnectionStatus => "get_connection_status",
            Self::ConnectToPierre => "connect_to_pierre",
            Self::ConnectProvider => "connect_provider",
            Self::DisconnectProvider => "disconnect_provider",
            Self::AnnounceOAuthSuccess => "announce_oauth_success",
            Self::CheckOAuthNotifications => "check_oauth_notifications",
            Self::GetNotifications => "get_notifications",
            Self::MarkNotificationsRead => "mark_notifications_read",
            Self::SetGoal => "set_goal",
            Self::SuggestGoals => "suggest_goals",
            Self::AnalyzeGoalFeasibility => "analyze_goal_feasibility",
            Self::TrackProgress => "track_progress",
            Self::CalculateMetrics => "calculate_metrics",
            Self::AnalyzePerformanceTrends => "analyze_performance_trends",
            Self::CompareActivities => "compare_activities",
            Self::DetectPatterns => "detect_patterns",
            Self::GenerateRecommendations => "generate_recommendations",
            Self::CalculateFitnessScore => "calculate_fitness_score",
            Self::PredictPerformance => "predict_performance",
            Self::AnalyzeTrainingLoad => "analyze_training_load",
            Self::GetConfigurationCatalog => "get_configuration_catalog",
            Self::GetConfigurationProfiles => "get_configuration_profiles",
            Self::GetUserConfiguration => "get_user_configuration",
            Self::UpdateUserConfiguration => "update_user_configuration",
            Self::CalculatePersonalizedZones => "calculate_personalized_zones",
            Self::ValidateConfiguration => "validate_configuration",
            Self::AnalyzeSleepQuality => "analyze_sleep_quality",
            Self::CalculateRecoveryScore => "calculate_recovery_score",
            Self::SuggestRestDay => "suggest_rest_day",
            Self::TrackSleepTrends => "track_sleep_trends",
            Self::OptimizeSleepSchedule => "optimize_sleep_schedule",
            Self::GetFitnessConfig => "get_fitness_config",
            Self::SetFitnessConfig => "set_fitness_config",
            Self::ListFitnessConfigs => "list_fitness_configs",
            Self::DeleteFitnessConfig => "delete_fitness_config",
            Self::CalculateDailyNutrition => "calculate_daily_nutrition",
            Self::GetNutrientTiming => "get_nutrient_timing",
            Self::SearchFood => "search_food",
            Self::GetFoodDetails => "get_food_details",
            Self::AnalyzeMealNutrition => "analyze_meal_nutrition",
        }
    }

    /// Get tool description for documentation and MCP schema generation
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::GetActivities => {
                "Get user's fitness activities with optional filtering and limits"
            }
            Self::GetAthlete => "Get user's athlete profile and basic information",
            Self::GetStats => "Get user's performance statistics and metrics",
            Self::AnalyzeActivity => {
                "Analyze a specific activity with detailed performance insights"
            }
            Self::GetActivityIntelligence => "Get AI-powered intelligence analysis for an activity",
            Self::GetConnectionStatus => "Check OAuth connection status for fitness providers",
            Self::ConnectToPierre => "Connect to Pierre MCP server and trigger OAuth authentication flow",
            Self::ConnectProvider => "Connect to a fitness data provider via OAuth",
            Self::DisconnectProvider => "Disconnect user from a fitness data provider",
            Self::AnnounceOAuthSuccess => "Announce OAuth connection success directly in chat",
            Self::CheckOAuthNotifications => "Check for new OAuth completion notifications",
            Self::GetNotifications => "Get list of OAuth notifications for the user",
            Self::MarkNotificationsRead => "Mark OAuth notifications as read",
            Self::SetGoal => "Set a new fitness goal for the user",
            Self::SuggestGoals => "Get AI-suggested fitness goals based on user's activity history",
            Self::AnalyzeGoalFeasibility => {
                "Analyze whether a goal is achievable given current fitness level"
            }
            Self::TrackProgress => "Track progress towards fitness goals",
            Self::CalculateMetrics => "Calculate custom fitness metrics and performance indicators",
            Self::AnalyzePerformanceTrends => "Analyze performance trends over time",
            Self::CompareActivities => "Compare two activities for performance analysis",
            Self::DetectPatterns => "Detect patterns and insights in activity data",
            Self::GenerateRecommendations => "Generate personalized training recommendations",
            Self::CalculateFitnessScore => {
                "Calculate overall fitness score based on recent activities"
            }
            Self::PredictPerformance => "Predict future performance based on training patterns",
            Self::AnalyzeTrainingLoad => "Analyze training load and recovery metrics",
            Self::GetConfigurationCatalog => {
                "Get the complete configuration catalog with all available parameters"
            }
            Self::GetConfigurationProfiles => {
                "Get available configuration profiles (Research, Elite, Recreational, etc.)"
            }
            Self::GetUserConfiguration => "Get current user's configuration settings and overrides",
            Self::UpdateUserConfiguration => {
                "Update user's configuration parameters and session overrides"
            }
            Self::CalculatePersonalizedZones => {
                "Calculate personalized training zones based on user's VO2 max and configuration"
            }
            Self::ValidateConfiguration => {
                "Validate configuration parameters against safety rules and constraints"
            }
            Self::AnalyzeSleepQuality => {
                "Analyze sleep quality from Fitbit/Garmin data using NSF/AASM guidelines"
            }
            Self::CalculateRecoveryScore => {
                "Calculate holistic recovery score combining TSB, sleep quality, and HRV"
            }
            Self::SuggestRestDay => {
                "AI-powered rest day recommendation based on recovery indicators"
            }
            Self::TrackSleepTrends => {
                "Track sleep patterns and correlate with performance over time"
            }
            Self::OptimizeSleepSchedule => {
                "Optimize sleep duration based on training load and recovery needs"
            }
            Self::GetFitnessConfig => "Get user fitness configuration settings including heart rate zones and training parameters",
            Self::SetFitnessConfig => "Save user fitness configuration settings for zones, thresholds, and training parameters",
            Self::ListFitnessConfigs => "List all available fitness configuration names for the user",
            Self::DeleteFitnessConfig => "Delete a specific fitness configuration by name",
            Self::CalculateDailyNutrition => {
                "Calculate daily calorie and macronutrient needs based on athlete biometrics, activity level, and training goal using Mifflin-St Jeor BMR formula"
            }
            Self::GetNutrientTiming => {
                "Get optimal pre/post-workout nutrition recommendations based on workout intensity and training goals following ISSN guidelines"
            }
            Self::SearchFood => {
                "Search USDA FoodData Central database for foods by name/description (free API with 24h caching)"
            }
            Self::GetFoodDetails => {
                "Get detailed nutritional information for a specific food from USDA database including all macro/micronutrients"
            }
            Self::AnalyzeMealNutrition => {
                "Analyze total calories and macronutrients for a meal composed of multiple USDA foods"
            }
        }
    }

    /// Check if this tool requires authentication
    #[must_use]
    pub const fn requires_auth(&self) -> bool {
        match self {
            // Config tools that don't need auth
            Self::GetConfigurationCatalog
            | Self::GetConfigurationProfiles
            | Self::ValidateConfiguration => false,
            // All other tools require authentication
            _ => true,
        }
    }

    /// Check if this tool is async (most are)
    #[must_use]
    pub const fn is_async(&self) -> bool {
        match self {
            // Sync tools that don't make API calls
            Self::GetActivityIntelligence
            | Self::CalculateMetrics
            | Self::GetConfigurationCatalog
            | Self::GetConfigurationProfiles
            | Self::CalculatePersonalizedZones
            | Self::ValidateConfiguration => false,
            // All other tools are async
            _ => true,
        }
    }
}

/// Handler function type for async tools
pub type AsyncToolHandler =
    fn(
        &crate::protocols::universal::UniversalToolExecutor,
        UniversalRequest,
    ) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>>;

/// Handler function type for sync tools
pub type SyncToolHandler = fn(
    &crate::protocols::universal::UniversalToolExecutor,
    &UniversalRequest,
) -> Result<UniversalResponse, ProtocolError>;

/// Tool metadata and handler information
pub struct ToolInfo {
    /// Strongly-typed tool identifier
    pub id: ToolId,
    /// Handler for async tools (makes network/DB calls)
    pub async_handler: Option<AsyncToolHandler>,
    /// Handler for sync tools (pure computation)
    pub sync_handler: Option<SyncToolHandler>,
}

impl ToolInfo {
    /// Create info for an async tool
    pub fn async_tool(id: ToolId, handler: AsyncToolHandler) -> Self {
        Self {
            id,
            async_handler: Some(handler),
            sync_handler: None,
        }
    }

    /// Create info for a sync tool
    pub fn sync_tool(id: ToolId, handler: SyncToolHandler) -> Self {
        Self {
            id,
            async_handler: None,
            sync_handler: Some(handler),
        }
    }
}

/// Type-safe tool registry that replaces string-based routing
/// Provides compile-time guarantees and better performance
pub struct ToolRegistry {
    /// Map of tool IDs to their handler information
    tools: HashMap<ToolId, ToolInfo>,
}

impl ToolRegistry {
    /// Create new empty registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool with its handler
    pub fn register(&mut self, tool_info: ToolInfo) {
        self.tools.insert(tool_info.id, tool_info);
    }

    /// Get tool info by ID
    #[must_use]
    pub fn get_tool(&self, id: ToolId) -> Option<&ToolInfo> {
        self.tools.get(&id)
    }

    /// Get tool ID from string name
    #[must_use]
    pub fn resolve_tool_name(&self, name: &str) -> Option<ToolId> {
        ToolId::from_name(name)
    }

    /// List all registered tool IDs
    #[must_use]
    pub fn list_tools(&self) -> Vec<ToolId> {
        self.tools.keys().copied().collect()
    }

    /// Check if a tool is registered
    #[must_use]
    pub fn has_tool(&self, id: ToolId) -> bool {
        self.tools.contains_key(&id)
    }

    /// Get all tool names for MCP schema generation
    #[must_use]
    pub fn tool_names(&self) -> Vec<&'static str> {
        self.tools.keys().map(ToolId::name).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Tests moved to tests/ directory following Rust idiomatic patterns
