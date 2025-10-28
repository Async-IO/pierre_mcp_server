// ABOUTME: Type-safe tool registry replacing string-based routing
// ABOUTME: Eliminates string literals and provides compile-time safety for tool dispatch
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

/// Type-safe tool identifier that replaces string-based routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolId {
    // Core Strava API tools
    GetActivities,
    GetAthlete,
    GetStats,
    AnalyzeActivity,
    GetActivityIntelligence,
    GetConnectionStatus,
    ConnectProvider,
    DisconnectProvider,

    // Goal and planning tools
    SetGoal,
    SuggestGoals,
    AnalyzeGoalFeasibility,
    TrackProgress,

    // Analysis and intelligence tools
    CalculateMetrics,
    AnalyzePerformanceTrends,
    CompareActivities,
    DetectPatterns,
    GenerateRecommendations,
    CalculateFitnessScore,
    PredictPerformance,
    AnalyzeTrainingLoad,

    // Configuration management tools
    GetConfigurationCatalog,
    GetConfigurationProfiles,
    GetUserConfiguration,
    UpdateUserConfiguration,
    CalculatePersonalizedZones,
    ValidateConfiguration,
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
            "connect_provider" => Some(Self::ConnectProvider),
            "disconnect_provider" => Some(Self::DisconnectProvider),
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
            Self::ConnectProvider => "connect_provider",
            Self::DisconnectProvider => "disconnect_provider",
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
            Self::ConnectProvider => "Connect to a fitness data provider via OAuth",
            Self::DisconnectProvider => "Disconnect user from a fitness data provider",
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
    pub id: ToolId,
    pub async_handler: Option<AsyncToolHandler>,
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
