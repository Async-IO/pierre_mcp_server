// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Constants Module
//! 
//! Application constants and environment-based configuration values.
//! This module provides both hardcoded constants and environment variable configuration.

use std::env;

/// Protocol-related constants - these can be overridden via environment variables
pub mod protocol {
    use std::env;
    
    /// Get MCP Protocol version from environment or default
    pub fn mcp_protocol_version() -> String {
        env::var("MCP_PROTOCOL_VERSION")
            .unwrap_or_else(|_| "2024-11-05".to_string())
    }
    
    /// JSON-RPC version (standard, not configurable)
    pub const JSONRPC_VERSION: &str = "2.0";
    
    /// Get server name from environment or default
    pub fn server_name() -> String {
        env::var("SERVER_NAME")
            .unwrap_or_else(|_| "pierre-mcp-server".to_string())
    }
    
    /// Get multi-tenant server name variant
    pub fn server_name_multitenant() -> String {
        env::var("SERVER_NAME")
            .map(|name| format!("{}-multitenant", name))
            .unwrap_or_else(|_| "pierre-mcp-server-multitenant".to_string())
    }
    
    /// Server version from Cargo.toml
    pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
    
    // For backward compatibility and performance, provide const versions with defaults
    pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";
    pub const SERVER_NAME: &str = "pierre-mcp-server";
    pub const SERVER_NAME_MULTITENANT: &str = "pierre-mcp-server-multitenant";
}

/// Environment-based configuration
pub mod env_config {
    use super::env;
    
    /// Get MCP server port from environment or default
    pub fn mcp_port() -> u16 {
        env::var("MCP_PORT")
            .unwrap_or_else(|_| "8080".to_string())
            .parse()
            .unwrap_or(8080)
    }
    
    /// Get HTTP server port from environment or default
    pub fn http_port() -> u16 {
        env::var("HTTP_PORT")
            .unwrap_or_else(|_| "8081".to_string())
            .parse()
            .unwrap_or(8081)
    }
    
    /// Get database URL from environment or default
    pub fn database_url() -> String {
        env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:./data/users.db".to_string())
    }
    
    /// Get encryption key path from environment or default
    pub fn encryption_key_path() -> String {
        env::var("ENCRYPTION_KEY_PATH")
            .unwrap_or_else(|_| "./data/encryption.key".to_string())
    }
    
    /// Get JWT secret path from environment or default
    pub fn jwt_secret_path() -> String {
        env::var("JWT_SECRET_PATH")
            .unwrap_or_else(|_| "./data/jwt.secret".to_string())
    }
    
    /// Get JWT expiry hours from environment or default
    pub fn jwt_expiry_hours() -> i64 {
        env::var("JWT_EXPIRY_HOURS")
            .unwrap_or_else(|_| "24".to_string())
            .parse()
            .unwrap_or(24)
    }
    
    /// Get Strava client ID from environment
    pub fn strava_client_id() -> Option<String> {
        env::var("STRAVA_CLIENT_ID").ok()
    }
    
    /// Get Strava client secret from environment
    pub fn strava_client_secret() -> Option<String> {
        env::var("STRAVA_CLIENT_SECRET").ok()
    }
    
    /// Get Strava redirect URI from environment or default
    pub fn strava_redirect_uri() -> String {
        env::var("STRAVA_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:8081/oauth/callback/strava".to_string())
    }
    
    /// Get OpenWeather API key from environment
    pub fn openweather_api_key() -> Option<String> {
        env::var("OPENWEATHER_API_KEY").ok()
    }
    
    /// Get log level from environment or default
    pub fn log_level() -> String {
        env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string())
    }
    
    /// Get Strava API base URL from environment or default
    pub fn strava_api_base() -> String {
        env::var("STRAVA_API_BASE")
            .unwrap_or_else(|_| "https://www.strava.com/api/v3".to_string())
    }
    
    /// Get Strava auth URL from environment or default
    pub fn strava_auth_url() -> String {
        env::var("STRAVA_AUTH_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/authorize".to_string())
    }
    
    /// Get Strava token URL from environment or default
    pub fn strava_token_url() -> String {
        env::var("STRAVA_TOKEN_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/token".to_string())
    }
    
    /// Get max activities fetch limit from environment or default
    pub fn max_activities_fetch() -> usize {
        env::var("MAX_ACTIVITIES_FETCH")
            .unwrap_or_else(|_| "100".to_string())
            .parse()
            .unwrap_or(100)
    }
    
    /// Get default activities limit from environment or default
    pub fn default_activities_limit() -> usize {
        env::var("DEFAULT_ACTIVITIES_LIMIT")
            .unwrap_or_else(|_| "20".to_string())
            .parse()
            .unwrap_or(20)
    }
}

/// JSON-RPC and MCP error codes
pub mod errors {
    /// Method not found
    pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;
    
    /// Invalid parameters
    pub const ERROR_INVALID_PARAMS: i32 = -32602;
    
    /// Internal error
    pub const ERROR_INTERNAL_ERROR: i32 = -32603;
    
    /// Unauthorized (custom error code)
    pub const ERROR_UNAUTHORIZED: i32 = -32000;
    
    /// Common error messages
    pub const MSG_METHOD_NOT_FOUND: &str = "Method not found";
    pub const MSG_INVALID_PARAMS: &str = "Invalid parameters";
    pub const MSG_INTERNAL_ERROR: &str = "Internal error";
    pub const MSG_AUTH_REQUIRED: &str = "Authentication required";
    pub const MSG_AUTH_FAILED: &str = "Authentication failed";
    pub const MSG_INVALID_TOKEN: &str = "Invalid or expired token";
}

/// API endpoints and URLs
pub mod endpoints {
    /// Strava API
    pub const STRAVA_API_BASE: &str = "https://www.strava.com/api/v3";
    pub const STRAVA_AUTH_URL: &str = "https://www.strava.com/oauth/authorize";
    pub const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";
    
    /// Fitbit API (placeholder - not implemented)
    pub const FITBIT_API_BASE: &str = "https://api.fitbit.com";
    pub const FITBIT_AUTH_URL: &str = "https://www.fitbit.com/oauth2/authorize";
    pub const FITBIT_TOKEN_URL: &str = "https://api.fitbit.com/oauth2/token";
}

/// HTTP routes and paths
pub mod routes {
    /// Authentication routes
    pub const AUTH_BASE: &str = "auth";
    pub const AUTH_REGISTER: &str = "register";
    pub const AUTH_LOGIN: &str = "login";
    
    /// OAuth routes
    pub const OAUTH_BASE: &str = "oauth";
    pub const OAUTH_AUTH: &str = "auth";
    pub const OAUTH_CALLBACK: &str = "callback";
    
    /// Health check
    pub const HEALTH: &str = "health";
}

/// Numeric limits and thresholds
pub mod limits {
    /// Activity fetch limits
    pub const MAX_ACTIVITIES_FETCH: usize = 100;
    pub const DEFAULT_ACTIVITIES_LIMIT: usize = 20;
    
    /// Authentication
    pub const MIN_PASSWORD_LENGTH: usize = 8;
    pub const JWT_EXPIRY_HOURS: i64 = 24;
    pub const AUTH_THREAD_SLEEP_MS: u64 = 1;
    
    /// Rate limiting defaults
    pub const DEFAULT_RATE_LIMIT_REQUESTS: u32 = 100;
    pub const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;
    
    /// Backup defaults
    pub const DEFAULT_BACKUP_INTERVAL_SECS: u64 = 21600; // 6 hours
    pub const DEFAULT_BACKUP_RETENTION_COUNT: usize = 7;
    
    /// Performance analysis
    pub const MIN_DATA_POINTS_FOR_TREND: usize = 2;
    pub const MIN_DATA_POINTS_FOR_ANALYSIS: usize = 5;
    pub const TREND_STRENGTH_STRONG: f64 = 0.8;
    pub const TREND_STRENGTH_MODERATE: f64 = 0.6;
    pub const TREND_STRENGTH_WEAK: f64 = 0.4;
    
    /// Unit conversions
    pub const SECONDS_PER_MINUTE: u64 = 60;
    pub const METERS_PER_KILOMETER: f64 = 1000.0;
    pub const METERS_PER_MILE: f64 = 1609.34;
}

/// OAuth scopes and provider defaults
pub mod oauth {
    /// Default OAuth scopes for Strava
    pub const STRAVA_DEFAULT_SCOPES: &str = "read,activity:read_all";
    
    /// Default OAuth scopes for Fitbit  
    pub const FITBIT_DEFAULT_SCOPES: &str = "activity,profile";
}

/// User and application defaults
pub mod defaults {
    /// Default backup directory
    pub const DEFAULT_BACKUP_DIR: &str = "./backups";
    
    /// Default fitness level for new users
    pub const DEFAULT_FITNESS_LEVEL: &str = "beginner";
    
    /// Default unit system
    pub const DEFAULT_UNITS: &str = "metric";
}

/// Database schema constants
pub mod database {
    /// Table names
    pub const TABLE_USERS: &str = "users";
    pub const TABLE_USER_PROFILES: &str = "user_profiles";
    pub const TABLE_GOALS: &str = "goals";
    pub const TABLE_GOAL_MILESTONES: &str = "goal_milestones";
    pub const TABLE_ANALYTICS_INSIGHTS: &str = "analytics_insights";
    
    /// Index names
    pub const INDEX_USERS_EMAIL: &str = "idx_users_email";
    pub const INDEX_GOALS_USER_ID: &str = "idx_goals_user_id";
    pub const INDEX_MILESTONES_GOAL_ID: &str = "idx_goal_milestones_goal_id";
    pub const INDEX_INSIGHTS_USER_ID: &str = "idx_analytics_insights_user_id";
    
    /// Column defaults
    pub const DEFAULT_USER_ACTIVE: bool = true;
    pub const DEFAULT_GOAL_STATUS: &str = "active";
    pub const DEFAULT_MILESTONE_ACHIEVED: bool = false;
}

/// Status and state enums
pub mod status {
    /// Goal status values
    pub const GOAL_STATUS_ACTIVE: &str = "active";
    pub const GOAL_STATUS_COMPLETED: &str = "completed";
    pub const GOAL_STATUS_PAUSED: &str = "paused";
    pub const GOAL_STATUS_CANCELLED: &str = "cancelled";
    
    /// Goal types
    pub const GOAL_TYPE_DISTANCE: &str = "distance";
    pub const GOAL_TYPE_TIME: &str = "time";
    pub const GOAL_TYPE_FREQUENCY: &str = "frequency";
    pub const GOAL_TYPE_PERFORMANCE: &str = "performance";
    pub const GOAL_TYPE_CUSTOM: &str = "custom";
    
    /// Trend directions
    pub const TREND_IMPROVING: &str = "improving";
    pub const TREND_DECLINING: &str = "declining";
    pub const TREND_STABLE: &str = "stable";
    pub const TREND_VOLATILE: &str = "volatile";
    
    /// Fitness levels
    pub const FITNESS_LEVEL_BEGINNER: &str = "beginner";
    pub const FITNESS_LEVEL_INTERMEDIATE: &str = "intermediate";
    pub const FITNESS_LEVEL_ADVANCED: &str = "advanced";
    pub const FITNESS_LEVEL_ELITE: &str = "elite";
    
    /// Training load levels
    pub const LOAD_LEVEL_LOW: &str = "low";
    pub const LOAD_LEVEL_MODERATE: &str = "moderate";
    pub const LOAD_LEVEL_HIGH: &str = "high";
    pub const LOAD_LEVEL_VERY_HIGH: &str = "very_high";
}

/// MCP tool names
pub mod tools {
    /// Core tools
    pub const GET_ACTIVITIES: &str = "get_activities";
    pub const GET_ATHLETE: &str = "get_athlete";
    pub const GET_STATS: &str = "get_stats";
    pub const GET_ACTIVITY_INTELLIGENCE: &str = "get_activity_intelligence";
    
    /// Connection management
    pub const CONNECT_STRAVA: &str = "connect_strava";
    pub const CONNECT_FITBIT: &str = "connect_fitbit";
    pub const GET_CONNECTION_STATUS: &str = "get_connection_status";
    pub const DISCONNECT_PROVIDER: &str = "disconnect_provider";
    
    /// Analytics tools
    pub const ANALYZE_ACTIVITY: &str = "analyze_activity";
    pub const CALCULATE_METRICS: &str = "calculate_metrics";
    pub const ANALYZE_PERFORMANCE_TRENDS: &str = "analyze_performance_trends";
    pub const COMPARE_ACTIVITIES: &str = "compare_activities";
    pub const DETECT_PATTERNS: &str = "detect_patterns";
    
    /// Goal management
    pub const SET_GOAL: &str = "set_goal";
    pub const TRACK_PROGRESS: &str = "track_progress";
    pub const SUGGEST_GOALS: &str = "suggest_goals";
    pub const ANALYZE_GOAL_FEASIBILITY: &str = "analyze_goal_feasibility";
    
    /// Advanced analytics
    pub const GENERATE_RECOMMENDATIONS: &str = "generate_recommendations";
    pub const CALCULATE_FITNESS_SCORE: &str = "calculate_fitness_score";
    pub const PREDICT_PERFORMANCE: &str = "predict_performance";
    pub const ANALYZE_TRAINING_LOAD: &str = "analyze_training_load";
}

/// Common JSON field names
pub mod json_fields {
    /// Request/Response fields
    pub const JSONRPC: &str = "jsonrpc";
    pub const METHOD: &str = "method";
    pub const PARAMS: &str = "params";
    pub const RESULT: &str = "result";
    pub const ERROR: &str = "error";
    pub const ID: &str = "id";
    pub const AUTH: &str = "auth";
    
    /// Error fields
    pub const CODE: &str = "code";
    pub const MESSAGE: &str = "message";
    pub const DATA: &str = "data";
    
    /// Tool parameters
    pub const NAME: &str = "name";
    pub const ARGUMENTS: &str = "arguments";
    pub const PROVIDER: &str = "provider";
    pub const LIMIT: &str = "limit";
    pub const OFFSET: &str = "offset";
    pub const ACTIVITY_ID: &str = "activity_id";
    pub const GOAL_ID: &str = "goal_id";
    pub const TIMEFRAME: &str = "timeframe";
    pub const METRIC: &str = "metric";
}

/// User-facing messages
pub mod messages {
    /// Authentication messages
    pub const INVALID_EMAIL_FORMAT: &str = "Invalid email format";
    pub const PASSWORD_TOO_SHORT: &str = "Password must be at least 8 characters long";
    pub const USER_ALREADY_EXISTS: &str = "User with this email already exists";
    pub const INVALID_CREDENTIALS: &str = "Invalid email or password";
    pub const REGISTRATION_SUCCESS: &str = "User registered successfully";
    
    /// Provider messages
    pub const PROVIDER_NOT_CONNECTED: &str = "Provider not connected";
    pub const PROVIDER_CONNECTION_SUCCESS: &str = "Provider connected successfully";
    pub const PROVIDER_DISCONNECTED: &str = "Provider disconnected successfully";
    
    /// Goal messages
    pub const GOAL_CREATED: &str = "Goal successfully created";
    pub const GOAL_NOT_FOUND: &str = "Goal not found";
    pub const GOAL_UPDATED: &str = "Goal updated successfully";
    
    /// Analysis messages
    pub const INSUFFICIENT_DATA: &str = "Insufficient data for analysis";
    pub const ANALYSIS_COMPLETE: &str = "Analysis completed successfully";
}