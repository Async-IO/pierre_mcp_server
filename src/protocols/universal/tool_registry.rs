// ABOUTME: Type-safe tool registry replacing string-based routing
// ABOUTME: Eliminates string literals and provides compile-time safety for tool dispatch
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::tools::{
    ACTIVATE_COACH, ADMIN_ASSIGN_COACH, ADMIN_CREATE_SYSTEM_COACH, ADMIN_DELETE_SYSTEM_COACH,
    ADMIN_GET_SYSTEM_COACH, ADMIN_LIST_COACH_ASSIGNMENTS, ADMIN_LIST_SYSTEM_COACHES,
    ADMIN_UNASSIGN_COACH, ADMIN_UPDATE_SYSTEM_COACH, CREATE_COACH, DEACTIVATE_COACH, DELETE_COACH,
    DELETE_RECIPE, GET_ACTIVE_COACH, GET_COACH, GET_RECIPE, GET_RECIPE_CONSTRAINTS, HIDE_COACH,
    LIST_COACHES, LIST_HIDDEN_COACHES, LIST_RECIPES, SAVE_RECIPE, SEARCH_COACHES, SEARCH_RECIPES,
    SHOW_COACH, TOGGLE_COACH_FAVORITE, UPDATE_COACH, VALIDATE_RECIPE,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
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
    /// Connect to a fitness data provider via OAuth
    ConnectProvider,
    /// Disconnect user from a fitness data provider
    DisconnectProvider,

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

    // Recipe management tools ("Combat des Chefs" architecture)
    /// Get macro targets and constraints for LLM recipe generation
    GetRecipeConstraints,
    /// Validate a recipe's nutrition against USDA database
    ValidateRecipe,
    /// Save a validated recipe to user's collection
    SaveRecipe,
    /// List user's saved recipes with optional filtering
    ListRecipes,
    /// Get a specific recipe by ID
    GetRecipe,
    /// Delete a recipe from user's collection
    DeleteRecipe,
    /// Search user's recipes by name, tags, or description
    SearchRecipes,

    // Coach management tools (custom AI personas)
    /// List user's coaches with optional filtering
    ListCoaches,
    /// Create a new custom coach with system prompt
    CreateCoach,
    /// Get a specific coach by ID
    GetCoach,
    /// Update an existing coach
    UpdateCoach,
    /// Delete a coach from user's collection
    DeleteCoach,
    /// Toggle favorite status of a coach
    ToggleCoachFavorite,
    /// Search coaches by query
    SearchCoaches,
    /// Activate a coach for the session
    ActivateCoach,
    /// Deactivate the currently active coach
    DeactivateCoach,
    /// Get the currently active coach
    GetActiveCoach,
    /// Hide a system or assigned coach from user's view
    HideCoach,
    /// Show (unhide) a previously hidden coach
    ShowCoach,
    /// List all hidden coaches for the user
    ListHiddenCoaches,

    // Admin coach management tools (system coaches - admin only)
    /// List system coaches in tenant (admin only)
    AdminListSystemCoaches,
    /// Create a system coach (admin only)
    AdminCreateSystemCoach,
    /// Get a specific system coach (admin only)
    AdminGetSystemCoach,
    /// Update a system coach (admin only)
    AdminUpdateSystemCoach,
    /// Delete a system coach (admin only)
    AdminDeleteSystemCoach,
    /// Assign coach to users (admin only)
    AdminAssignCoach,
    /// Unassign coach from users (admin only)
    AdminUnassignCoach,
    /// List coach assignments (admin only)
    AdminListCoachAssignments,
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
            // Note: connect_to_pierre removed - SDK bridge handles it locally via RFC 8414 discovery
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
            // Recipe management tools
            GET_RECIPE_CONSTRAINTS => Some(Self::GetRecipeConstraints),
            VALIDATE_RECIPE => Some(Self::ValidateRecipe),
            SAVE_RECIPE => Some(Self::SaveRecipe),
            LIST_RECIPES => Some(Self::ListRecipes),
            GET_RECIPE => Some(Self::GetRecipe),
            DELETE_RECIPE => Some(Self::DeleteRecipe),
            SEARCH_RECIPES => Some(Self::SearchRecipes),
            // Coach management tools
            LIST_COACHES => Some(Self::ListCoaches),
            CREATE_COACH => Some(Self::CreateCoach),
            GET_COACH => Some(Self::GetCoach),
            UPDATE_COACH => Some(Self::UpdateCoach),
            DELETE_COACH => Some(Self::DeleteCoach),
            TOGGLE_COACH_FAVORITE => Some(Self::ToggleCoachFavorite),
            SEARCH_COACHES => Some(Self::SearchCoaches),
            ACTIVATE_COACH => Some(Self::ActivateCoach),
            DEACTIVATE_COACH => Some(Self::DeactivateCoach),
            GET_ACTIVE_COACH => Some(Self::GetActiveCoach),
            HIDE_COACH => Some(Self::HideCoach),
            SHOW_COACH => Some(Self::ShowCoach),
            LIST_HIDDEN_COACHES => Some(Self::ListHiddenCoaches),
            // Admin coach management tools (system coaches)
            ADMIN_LIST_SYSTEM_COACHES => Some(Self::AdminListSystemCoaches),
            ADMIN_CREATE_SYSTEM_COACH => Some(Self::AdminCreateSystemCoach),
            ADMIN_GET_SYSTEM_COACH => Some(Self::AdminGetSystemCoach),
            ADMIN_UPDATE_SYSTEM_COACH => Some(Self::AdminUpdateSystemCoach),
            ADMIN_DELETE_SYSTEM_COACH => Some(Self::AdminDeleteSystemCoach),
            ADMIN_ASSIGN_COACH => Some(Self::AdminAssignCoach),
            ADMIN_UNASSIGN_COACH => Some(Self::AdminUnassignCoach),
            ADMIN_LIST_COACH_ASSIGNMENTS => Some(Self::AdminListCoachAssignments),
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
            // Recipe management tools
            Self::GetRecipeConstraints => GET_RECIPE_CONSTRAINTS,
            Self::ValidateRecipe => VALIDATE_RECIPE,
            Self::SaveRecipe => SAVE_RECIPE,
            Self::ListRecipes => LIST_RECIPES,
            Self::GetRecipe => GET_RECIPE,
            Self::DeleteRecipe => DELETE_RECIPE,
            Self::SearchRecipes => SEARCH_RECIPES,
            // Coach management tools
            Self::ListCoaches => LIST_COACHES,
            Self::CreateCoach => CREATE_COACH,
            Self::GetCoach => GET_COACH,
            Self::UpdateCoach => UPDATE_COACH,
            Self::DeleteCoach => DELETE_COACH,
            Self::ToggleCoachFavorite => TOGGLE_COACH_FAVORITE,
            Self::SearchCoaches => SEARCH_COACHES,
            Self::ActivateCoach => ACTIVATE_COACH,
            Self::DeactivateCoach => DEACTIVATE_COACH,
            Self::GetActiveCoach => GET_ACTIVE_COACH,
            Self::HideCoach => HIDE_COACH,
            Self::ShowCoach => SHOW_COACH,
            Self::ListHiddenCoaches => LIST_HIDDEN_COACHES,
            // Admin coach management tools
            Self::AdminListSystemCoaches => ADMIN_LIST_SYSTEM_COACHES,
            Self::AdminCreateSystemCoach => ADMIN_CREATE_SYSTEM_COACH,
            Self::AdminGetSystemCoach => ADMIN_GET_SYSTEM_COACH,
            Self::AdminUpdateSystemCoach => ADMIN_UPDATE_SYSTEM_COACH,
            Self::AdminDeleteSystemCoach => ADMIN_DELETE_SYSTEM_COACH,
            Self::AdminAssignCoach => ADMIN_ASSIGN_COACH,
            Self::AdminUnassignCoach => ADMIN_UNASSIGN_COACH,
            Self::AdminListCoachAssignments => ADMIN_LIST_COACH_ASSIGNMENTS,
        }
    }

    /// Get tool description for documentation and MCP schema generation
    #[must_use]
    pub const fn description(&self) -> &'static str {
        match self {
            Self::GetActivities => "Get user's fitness activities with optional filtering",
            Self::GetAthlete => "Get user's athlete profile and basic information",
            Self::GetStats => "Get user's performance statistics and metrics",
            Self::AnalyzeActivity => "Analyze a specific activity with performance insights",
            Self::GetActivityIntelligence => "Get AI-powered intelligence for an activity",
            Self::GetConnectionStatus => "Check OAuth connection status for providers",
            Self::ConnectProvider => "Connect to a fitness data provider via OAuth",
            Self::DisconnectProvider => "Disconnect user from a fitness data provider",
            Self::SetGoal => "Set a new fitness goal for the user",
            Self::SuggestGoals => "Get AI-suggested goals based on activity history",
            Self::AnalyzeGoalFeasibility => "Analyze if a goal is achievable",
            Self::TrackProgress => "Track progress towards fitness goals",
            Self::CalculateMetrics => "Calculate custom fitness metrics and indicators",
            Self::AnalyzePerformanceTrends => "Analyze performance trends over time",
            Self::CompareActivities => "Compare two activities for performance analysis",
            Self::DetectPatterns => "Detect patterns and insights in activity data",
            Self::GenerateRecommendations => "Generate personalized training recommendations",
            Self::CalculateFitnessScore => "Calculate overall fitness score",
            Self::PredictPerformance => "Predict future performance based on training",
            Self::AnalyzeTrainingLoad => "Analyze training load and recovery metrics",
            Self::GetConfigurationCatalog => "Get configuration catalog with parameters",
            Self::GetConfigurationProfiles => "Get available configuration profiles",
            Self::GetUserConfiguration => "Get current user's configuration settings",
            Self::UpdateUserConfiguration => "Update user's configuration parameters",
            Self::CalculatePersonalizedZones => "Calculate training zones based on VO2 max",
            Self::ValidateConfiguration => "Validate configuration against safety rules",
            Self::AnalyzeSleepQuality => "Analyze sleep quality using NSF/AASM",
            Self::CalculateRecoveryScore => "Calculate recovery score combining TSB and HRV",
            Self::SuggestRestDay => "AI-powered rest day recommendation",
            Self::TrackSleepTrends => "Track sleep patterns and correlate with performance",
            Self::OptimizeSleepSchedule => "Optimize sleep based on training load",
            Self::GetFitnessConfig => "Get user fitness config including HR zones",
            Self::SetFitnessConfig => "Save user fitness config for zones and params",
            Self::ListFitnessConfigs => "List all fitness configuration names for user",
            Self::DeleteFitnessConfig => "Delete a fitness configuration by name",
            Self::CalculateDailyNutrition => "Calculate daily calories using Mifflin-St Jeor",
            Self::GetNutrientTiming => "Get pre/post-workout nutrition per ISSN",
            Self::SearchFood => "Search USDA FoodData Central for foods",
            Self::GetFoodDetails => "Get nutritional info from USDA database",
            Self::AnalyzeMealNutrition => "Analyze calories and macros for a meal",
            Self::GetRecipeConstraints => "Get macro targets for recipe generation",
            Self::ValidateRecipe => "Validate recipe nutrition against USDA",
            Self::SaveRecipe => "Save validated recipe with nutrition data",
            Self::ListRecipes => "List saved recipes with optional filtering",
            Self::GetRecipe => "Get a specific recipe by ID",
            Self::DeleteRecipe => "Delete a recipe from collection",
            Self::SearchRecipes => "Search recipes by name, tags, or description",
            Self::ListCoaches => "List user's coaches with optional filtering",
            Self::CreateCoach => "Create a new custom coach with system prompt",
            Self::GetCoach => "Get a specific coach by ID",
            Self::UpdateCoach => "Update an existing coach's properties",
            Self::DeleteCoach => "Delete a coach from user's collection",
            Self::ToggleCoachFavorite => "Toggle favorite status of a coach",
            Self::SearchCoaches => "Search coaches by title, description, or tags",
            Self::ActivateCoach => "Set a coach as active (only one can be active)",
            Self::DeactivateCoach => "Deactivate the currently active coach",
            Self::GetActiveCoach => "Get the currently active coach",
            Self::HideCoach => "Hide a system or assigned coach",
            Self::ShowCoach => "Show (unhide) a hidden coach",
            Self::ListHiddenCoaches => "List all hidden coaches for the user",
            Self::AdminListSystemCoaches => "List all system coaches (admin only)",
            Self::AdminCreateSystemCoach => "Create a system coach (admin only)",
            Self::AdminGetSystemCoach => "Get a specific system coach (admin only)",
            Self::AdminUpdateSystemCoach => "Update a system coach (admin only)",
            Self::AdminDeleteSystemCoach => "Delete a system coach (admin only)",
            Self::AdminAssignCoach => "Assign coach to users (admin only)",
            Self::AdminUnassignCoach => "Remove coach assignment (admin only)",
            Self::AdminListCoachAssignments => "List coach assignments (admin only)",
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
        &UniversalToolExecutor,
        UniversalRequest,
    ) -> Pin<Box<dyn Future<Output = Result<UniversalResponse, ProtocolError>> + Send + '_>>;

/// Handler function type for sync tools
pub type SyncToolHandler =
    fn(&UniversalToolExecutor, &UniversalRequest) -> Result<UniversalResponse, ProtocolError>;

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
