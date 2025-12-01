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
pub const CONNECT_TO_PIERRE: &str = "connect_to_pierre"; // Tool ID remains snake_case for internal use
/// Tool identifier for unified Pierre and fitness provider OAuth connection
pub const CONNECT_PROVIDER: &str = "connect_provider"; // Unified Pierre + Provider OAuth flow
/// Tool identifier for checking connection status with fitness providers
pub const GET_CONNECTION_STATUS: &str = "get_connection_status";
/// Tool identifier for disconnecting from fitness providers
pub const DISCONNECT_PROVIDER: &str = "disconnect_provider";

/// Notification management tools
pub const MARK_NOTIFICATIONS_READ: &str = "mark_notifications_read";
/// Tool identifier for retrieving user notifications
pub const GET_NOTIFICATIONS: &str = "get_notifications";
/// Tool identifier for announcing successful OAuth completion
pub const ANNOUNCE_OAUTH_SUCCESS: &str = "announce_oauth_success";
/// Tool identifier for checking pending OAuth notifications
pub const CHECK_OAUTH_NOTIFICATIONS: &str = "check_oauth_notifications";

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

/// Weather and external data tools
pub const GET_WEATHER: &str = "get_weather";

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
