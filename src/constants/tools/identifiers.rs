// ABOUTME: MCP tool identifier constants to eliminate hardcoded tool names
// ABOUTME: Provides centralized tool name constants organized by functional groups

//! MCP tool identifier constants

/// Core tools for basic data retrieval
pub const GET_ACTIVITIES: &str = "get_activities";
pub const GET_ATHLETE: &str = "get_athlete";
pub const GET_STATS: &str = "get_stats";
pub const GET_ACTIVITY_INTELLIGENCE: &str = "get_activity_intelligence";

/// Connection management tools
pub const CONNECT_TO_PIERRE: &str = "connect_to_pierre";  // Tool ID remains snake_case for internal use
pub const GET_CONNECTION_STATUS: &str = "get_connection_status";
pub const DISCONNECT_PROVIDER: &str = "disconnect_provider";

/// Notification management tools
pub const MARK_NOTIFICATIONS_READ: &str = "mark_notifications_read";
pub const GET_NOTIFICATIONS: &str = "get_notifications";
pub const ANNOUNCE_OAUTH_SUCCESS: &str = "announce_oauth_success";
pub const CHECK_OAUTH_NOTIFICATIONS: &str = "check_oauth_notifications";

/// Analytics and performance analysis tools
pub const ANALYZE_ACTIVITY: &str = "analyze_activity";
pub const CALCULATE_METRICS: &str = "calculate_metrics";
pub const ANALYZE_PERFORMANCE_TRENDS: &str = "analyze_performance_trends";
pub const COMPARE_ACTIVITIES: &str = "compare_activities";
pub const DETECT_PATTERNS: &str = "detect_patterns";

/// Goal management tools
pub const SET_GOAL: &str = "set_goal";
pub const TRACK_PROGRESS: &str = "track_progress";

/// Weather and external data tools
pub const GET_WEATHER: &str = "get_weather";

/// Fitness configuration tools
pub const GET_FITNESS_CONFIG: &str = "get_fitness_config";
pub const SET_FITNESS_CONFIG: &str = "set_fitness_config";
pub const LIST_FITNESS_CONFIGS: &str = "list_fitness_configs";
pub const DELETE_FITNESS_CONFIG: &str = "delete_fitness_config";

/// Advanced analytics tools
pub const PREDICT_PERFORMANCE: &str = "predict_performance";
pub const ANALYZE_GOAL_FEASIBILITY: &str = "analyze_goal_feasibility";
pub const ANALYZE_TRAINING_LOAD: &str = "analyze_training_load";
pub const CALCULATE_FITNESS_SCORE: &str = "calculate_fitness_score";
pub const GENERATE_RECOMMENDATIONS: &str = "generate_recommendations";
pub const SUGGEST_GOALS: &str = "suggest_goals";
