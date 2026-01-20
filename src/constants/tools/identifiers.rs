// ABOUTME: MCP tool identifier constants to eliminate hardcoded tool names
// ABOUTME: Provides centralized tool name constants organized by functional groups
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! MCP tool identifier constants

/// Core tools for basic data retrieval
pub const GET_ACTIVITIES: &str = "get_activities";
/// Tool identifier for retrieving athlete profile information
pub const GET_ATHLETE: &str = "get_athlete";
/// Tool identifier for retrieving athlete statistics
pub const GET_STATS: &str = "get_stats";
/// Tool identifier for retrieving AI-powered activity insights
pub const GET_ACTIVITY_INTELLIGENCE: &str = "get_activity_intelligence";

/// Connection management tools
/// Tool identifier for unified Pierre and fitness provider OAuth connection
pub const CONNECT_PROVIDER: &str = "connect_provider"; // Unified Pierre + Provider OAuth flow
/// Tool identifier for checking connection status with fitness providers
pub const GET_CONNECTION_STATUS: &str = "get_connection_status";
/// Tool identifier for disconnecting from fitness providers
pub const DISCONNECT_PROVIDER: &str = "disconnect_provider";

/// Analytics and performance analysis tools
pub const ANALYZE_ACTIVITY: &str = "analyze_activity";
/// Tool identifier for calculating custom fitness metrics
pub const CALCULATE_METRICS: &str = "calculate_metrics";
/// Tool identifier for analyzing performance trends over time
pub const ANALYZE_PERFORMANCE_TRENDS: &str = "analyze_performance_trends";
/// Tool identifier for comparing multiple activities
pub const COMPARE_ACTIVITIES: &str = "compare_activities";
/// Tool identifier for detecting patterns in training data
pub const DETECT_PATTERNS: &str = "detect_patterns";

/// Goal management tools
pub const SET_GOAL: &str = "set_goal";
/// Tool identifier for tracking progress toward fitness goals
pub const TRACK_PROGRESS: &str = "track_progress";

/// Fitness configuration tools
pub const GET_FITNESS_CONFIG: &str = "get_fitness_config";
/// Tool identifier for updating fitness configuration settings
pub const SET_FITNESS_CONFIG: &str = "set_fitness_config";
/// Tool identifier for listing available fitness configurations
pub const LIST_FITNESS_CONFIGS: &str = "list_fitness_configs";
/// Tool identifier for deleting fitness configurations
pub const DELETE_FITNESS_CONFIG: &str = "delete_fitness_config";

/// Advanced analytics tools
pub const PREDICT_PERFORMANCE: &str = "predict_performance";
/// Tool identifier for analyzing whether fitness goals are achievable
pub const ANALYZE_GOAL_FEASIBILITY: &str = "analyze_goal_feasibility";
/// Tool identifier for analyzing training load and recovery needs
pub const ANALYZE_TRAINING_LOAD: &str = "analyze_training_load";
/// Tool identifier for calculating overall fitness score
pub const CALCULATE_FITNESS_SCORE: &str = "calculate_fitness_score";
/// Tool identifier for generating personalized training recommendations
pub const GENERATE_RECOMMENDATIONS: &str = "generate_recommendations";
/// Tool identifier for goal suggestion functionality
pub const SUGGEST_GOALS: &str = "suggest_goals";

/// Recipe management tools (Combat des Chefs)
pub const GET_RECIPE_CONSTRAINTS: &str = "get_recipe_constraints";
/// Tool identifier for listing user recipes
pub const LIST_RECIPES: &str = "list_recipes";
/// Tool identifier for retrieving a specific recipe
pub const GET_RECIPE: &str = "get_recipe";
/// Tool identifier for deleting a recipe
pub const DELETE_RECIPE: &str = "delete_recipe";
/// Tool identifier for searching recipes
pub const SEARCH_RECIPES: &str = "search_recipes";
/// Tool identifier for saving a new recipe
pub const SAVE_RECIPE: &str = "save_recipe";
/// Tool identifier for validating recipe nutrition
pub const VALIDATE_RECIPE: &str = "validate_recipe";

/// Coach management tools (custom AI personas)
pub const LIST_COACHES: &str = "list_coaches";
/// Tool identifier for creating a new coach
pub const CREATE_COACH: &str = "create_coach";
/// Tool identifier for retrieving a specific coach
pub const GET_COACH: &str = "get_coach";
/// Tool identifier for updating a coach
pub const UPDATE_COACH: &str = "update_coach";
/// Tool identifier for deleting a coach
pub const DELETE_COACH: &str = "delete_coach";
/// Tool identifier for toggling coach favorite status
pub const TOGGLE_COACH_FAVORITE: &str = "toggle_coach_favorite";
/// Tool identifier for searching coaches
pub const SEARCH_COACHES: &str = "search_coaches";
/// Tool identifier for activating a coach for the session
pub const ACTIVATE_COACH: &str = "activate_coach";
/// Tool identifier for deactivating the current coach
pub const DEACTIVATE_COACH: &str = "deactivate_coach";
/// Tool identifier for getting the currently active coach
pub const GET_ACTIVE_COACH: &str = "get_active_coach";
/// Tool identifier for hiding a coach from user's view
pub const HIDE_COACH: &str = "hide_coach";
/// Tool identifier for showing (unhiding) a coach
pub const SHOW_COACH: &str = "show_coach";
/// Tool identifier for listing hidden coaches
pub const LIST_HIDDEN_COACHES: &str = "list_hidden_coaches";

/// Admin coach management tools (system coaches)
/// Tool identifier for listing system coaches (admin only)
pub const ADMIN_LIST_SYSTEM_COACHES: &str = "admin_list_system_coaches";
/// Tool identifier for creating a system coach (admin only)
pub const ADMIN_CREATE_SYSTEM_COACH: &str = "admin_create_system_coach";
/// Tool identifier for getting a system coach (admin only)
pub const ADMIN_GET_SYSTEM_COACH: &str = "admin_get_system_coach";
/// Tool identifier for updating a system coach (admin only)
pub const ADMIN_UPDATE_SYSTEM_COACH: &str = "admin_update_system_coach";
/// Tool identifier for deleting a system coach (admin only)
pub const ADMIN_DELETE_SYSTEM_COACH: &str = "admin_delete_system_coach";
/// Tool identifier for assigning a coach to users (admin only)
pub const ADMIN_ASSIGN_COACH: &str = "admin_assign_coach";
/// Tool identifier for unassigning a coach from users (admin only)
pub const ADMIN_UNASSIGN_COACH: &str = "admin_unassign_coach";
/// Tool identifier for listing coach assignments (admin only)
pub const ADMIN_LIST_COACH_ASSIGNMENTS: &str = "admin_list_coach_assignments";

/// Mobility tools (stretching exercises, yoga poses)
/// Tool identifier for listing stretching exercises
pub const LIST_STRETCHING_EXERCISES: &str = "list_stretching_exercises";
/// Tool identifier for getting a specific stretching exercise
pub const GET_STRETCHING_EXERCISE: &str = "get_stretching_exercise";
/// Tool identifier for suggesting stretches for a specific activity
pub const SUGGEST_STRETCHES_FOR_ACTIVITY: &str = "suggest_stretches_for_activity";
/// Tool identifier for listing yoga poses
pub const LIST_YOGA_POSES: &str = "list_yoga_poses";
/// Tool identifier for getting a specific yoga pose
pub const GET_YOGA_POSE: &str = "get_yoga_pose";
/// Tool identifier for suggesting a yoga sequence
pub const SUGGEST_YOGA_SEQUENCE: &str = "suggest_yoga_sequence";
