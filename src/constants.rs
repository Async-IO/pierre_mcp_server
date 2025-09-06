// ABOUTME: System-wide constants and configuration values for Pierre API
// ABOUTME: Contains protocol constants, error codes, and system configuration defaults
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

    /// Get `MCP` Protocol version from environment or default
    #[must_use]
    pub fn mcp_protocol_version() -> String {
        env::var("MCP_PROTOCOL_VERSION").unwrap_or_else(|_| "2025-06-18".into())
    }

    /// `JSON-RPC` version (standard, not configurable)
    pub const JSONRPC_VERSION: &str = "2.0";

    /// Get server name from environment or default
    #[must_use]
    pub fn server_name() -> String {
        env::var("SERVER_NAME").unwrap_or_else(|_| "pierre-mcp-server".into())
    }

    /// Get server name variant with specific suffix
    #[must_use]
    pub fn server_name_multitenant() -> String {
        env::var("SERVER_NAME").unwrap_or_else(|_| "pierre-mcp-server".into())
    }

    /// Server version from Cargo.toml
    pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
}

/// Environment-based configuration
pub mod env_config {
    use super::env;

    /// Get `MCP` server port from environment or default
    #[must_use]
    pub fn mcp_port() -> u16 {
        env::var("MCP_PORT")
            .unwrap_or_else(|_| crate::constants::ports::DEFAULT_MCP_PORT.to_string())
            .parse()
            .unwrap_or(crate::constants::ports::DEFAULT_MCP_PORT)
    }

    /// Get `HTTP` server port from environment or default
    #[must_use]
    pub fn http_port() -> u16 {
        env::var("HTTP_PORT")
            .unwrap_or_else(|_| crate::constants::ports::DEFAULT_HTTP_PORT.to_string())
            .parse()
            .unwrap_or(crate::constants::ports::DEFAULT_HTTP_PORT)
    }

    /// Get database `URL` from environment or default
    #[must_use]
    pub fn database_url() -> String {
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/users.db".into())
    }

    /// Get encryption key path from environment or default
    #[must_use]
    pub fn encryption_key_path() -> String {
        env::var("ENCRYPTION_KEY_PATH").unwrap_or_else(|_| "./data/encryption.key".into())
    }

    /// Get `JWT` secret path from environment or default
    #[must_use]
    pub fn jwt_secret_path() -> String {
        env::var("JWT_SECRET_PATH").unwrap_or_else(|_| "./data/jwt.secret".into())
    }

    /// Get `JWT` expiry hours from environment or default
    #[must_use]
    pub fn jwt_expiry_hours() -> i64 {
        env::var("JWT_EXPIRY_HOURS")
            .unwrap_or_else(|_| "24".into())
            .parse()
            .unwrap_or(24)
    }

    /// Get Strava redirect `URI` from environment or default (for tenant-based OAuth)
    #[must_use]
    pub fn strava_redirect_uri() -> String {
        env::var("STRAVA_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{}/api/oauth/callback/strava", base_url()))
    }

    /// Get Fitbit redirect `URI` from environment or default (for tenant-based OAuth)
    #[must_use]
    pub fn fitbit_redirect_uri() -> String {
        env::var("FITBIT_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{}/api/oauth/callback/fitbit", base_url()))
    }

    /// Get `OpenWeather` `API` key from environment
    #[must_use]
    pub fn openweather_api_key() -> Option<String> {
        env::var("OPENWEATHER_API_KEY").ok()
    }

    /// Get log level from environment or default
    #[must_use]
    pub fn log_level() -> String {
        env::var("RUST_LOG").unwrap_or_else(|_| "info".into())
    }

    /// Get Strava `API` base `URL` from environment or default
    #[must_use]
    pub fn strava_api_base() -> String {
        env::var("STRAVA_API_BASE").unwrap_or_else(|_| "https://www.strava.com/api/v3".into())
    }

    /// Get Strava auth `URL` from environment or default
    #[must_use]
    pub fn strava_auth_url() -> String {
        env::var("STRAVA_AUTH_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/authorize".into())
    }

    /// Get Strava token `URL` from environment or default
    #[must_use]
    pub fn strava_token_url() -> String {
        env::var("STRAVA_TOKEN_URL").unwrap_or_else(|_| "https://www.strava.com/oauth/token".into())
    }

    /// Get max activities fetch limit from environment or default
    #[must_use]
    pub fn max_activities_fetch() -> usize {
        env::var("MAX_ACTIVITIES_FETCH")
            .unwrap_or_else(|_| "100".into())
            .parse()
            .unwrap_or(100)
    }

    /// Get Fitbit auth `URL` from environment or default
    #[must_use]
    pub fn fitbit_auth_url() -> String {
        env::var("FITBIT_AUTH_URL")
            .unwrap_or_else(|_| "https://www.fitbit.com/oauth2/authorize".into())
    }

    /// Get Fitbit token `URL` from environment or default
    #[must_use]
    pub fn fitbit_token_url() -> String {
        env::var("FITBIT_TOKEN_URL")
            .unwrap_or_else(|_| "https://api.fitbit.com/oauth2/token".into())
    }

    /// Get Strava deauthorize `URL` from environment or default
    #[must_use]
    pub fn strava_deauthorize_url() -> String {
        env::var("STRAVA_DEAUTHORIZE_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/deauthorize".into())
    }

    /// Get Fitbit revoke `URL` from environment or default
    #[must_use]
    pub fn fitbit_revoke_url() -> String {
        env::var("FITBIT_REVOKE_URL")
            .unwrap_or_else(|_| "https://api.fitbit.com/oauth2/revoke".into())
    }

    /// Get default activities limit from environment or default
    #[must_use]
    pub fn default_activities_limit() -> usize {
        env::var("DEFAULT_ACTIVITIES_LIMIT")
            .unwrap_or_else(|_| "20".into())
            .parse()
            .unwrap_or(20)
    }

    /// Get `OpenWeather` `API` base `URL` from environment or default
    #[must_use]
    pub fn openweather_api_base() -> String {
        env::var("OPENWEATHER_API_BASE_URL")
            .unwrap_or_else(|_| "https://api.openweathermap.org".into())
    }

    /// Get base URL for the application from environment or construct from host and port
    #[must_use]
    pub fn base_url() -> String {
        env::var("BASE_URL").unwrap_or_else(|_| {
            let host = env::var("HOST")
                .unwrap_or_else(|_| crate::constants::network_config::DEFAULT_HOST.into());
            let port = http_port();
            format!("http://{host}:{port}")
        })
    }
}

/// `JSON-RPC` and `MCP` error codes
pub mod errors {
    /// Method not found
    pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid parameters
    pub const ERROR_INVALID_PARAMS: i32 = -32602;

    /// Internal error
    pub const ERROR_INTERNAL_ERROR: i32 = -32603;

    /// Unauthorized - using standard JSON-RPC Internal Error for better Claude Desktop integration
    pub const ERROR_UNAUTHORIZED: i32 = -32603;

    /// Token-specific error codes (using standard JSON-RPC codes for better Claude Desktop integration)
    pub const ERROR_TOKEN_EXPIRED: i32 = -32603; // Internal error - token expired
    pub const ERROR_TOKEN_INVALID: i32 = -32603; // Internal error - token invalid
    pub const ERROR_TOKEN_MALFORMED: i32 = -32602; // Invalid params - malformed token

    /// Common error messages
    pub const MSG_METHOD_NOT_FOUND: &str = "Method not found";
    pub const MSG_INVALID_PARAMS: &str = "Invalid parameters";
    pub const MSG_INTERNAL_ERROR: &str = "Internal error";
    pub const MSG_AUTH_REQUIRED: &str = "Authentication required";
    pub const MSG_AUTH_FAILED: &str = "Authentication failed";
    pub const MSG_INVALID_TOKEN: &str = "Invalid or expired token";

    /// Token-specific error messages
    pub const MSG_TOKEN_EXPIRED: &str = "JWT token has expired";
    pub const MSG_TOKEN_INVALID: &str = "JWT token signature is invalid";
    pub const MSG_TOKEN_MALFORMED: &str = "JWT token is malformed";

    /// MCP protocol version error codes
    pub const ERROR_VERSION_MISMATCH: i32 = -32602; // Invalid params - unsupported protocol version

    /// MCP-specific error codes for better diagnostics
    pub const ERROR_TOOL_EXECUTION: i32 = -32000; // Server error - tool execution failed
    pub const ERROR_RESOURCE_ACCESS: i32 = -32001; // Server error - resource access failed
    pub const ERROR_AUTHENTICATION: i32 = -32002; // Server error - authentication failed
    pub const ERROR_AUTHORIZATION: i32 = -32003; // Server error - authorization failed
    pub const ERROR_SERIALIZATION: i32 = -32004; // Server error - data serialization failed

    /// MCP protocol version error messages
    pub const MSG_VERSION_MISMATCH: &str = "Unsupported MCP protocol version";

    /// MCP-specific error messages
    pub const MSG_TOOL_EXECUTION: &str = "Tool execution failed";
    pub const MSG_RESOURCE_ACCESS: &str = "Resource access failed";
    pub const MSG_AUTHENTICATION: &str = "Authentication failed";
    pub const MSG_AUTHORIZATION: &str = "Authorization failed";
    pub const MSG_SERIALIZATION: &str = "Data serialization failed";

    /// Progress tracking error codes
    pub const ERROR_PROGRESS_TRACKING: i32 = -32005; // Server error - progress tracking failed
    pub const ERROR_OPERATION_CANCELLED: i32 = -32006; // Server error - operation cancelled

    /// Progress tracking error messages
    pub const MSG_PROGRESS_TRACKING: &str = "Progress tracking failed";
    pub const MSG_OPERATION_CANCELLED: &str = "Operation was cancelled";
}

/// `API` endpoints and `URLs`
pub mod endpoints {
    /// Strava `API`
    pub const STRAVA_API_BASE: &str = "https://www.strava.com/api/v3";
    pub const STRAVA_AUTH_URL: &str = "https://www.strava.com/oauth/authorize";
    pub const STRAVA_TOKEN_URL: &str = "https://www.strava.com/oauth/token";

    /// Fitbit `API`
    pub const FITBIT_API_BASE: &str = "https://api.fitbit.com";
    pub const FITBIT_AUTH_URL: &str = "https://www.fitbit.com/oauth2/authorize";
    pub const FITBIT_TOKEN_URL: &str = "https://api.fitbit.com/oauth2/token";
}

/// Default port configurations
pub mod ports {
    /// Default `MCP` server port
    pub const DEFAULT_MCP_PORT: u16 = 8080;
    /// Default `HTTP` server port  
    pub const DEFAULT_HTTP_PORT: u16 = 8081;
    /// Default documentation server port
    pub const DEFAULT_DOCS_PORT: u16 = 3000;
}

/// `HTTP` routes and paths
pub mod routes {
    /// Authentication routes
    pub const AUTH_BASE: &str = "auth";
    pub const AUTH_REGISTER: &str = "register";
    pub const AUTH_LOGIN: &str = "login";

    /// `OAuth` routes
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
    pub const JWT_EXPIRY_HOURS: i64 = 168; // 1 week
    pub const AUTH_THREAD_SLEEP_MS: u64 = 1;

    /// Rate limiting defaults
    pub const DEFAULT_RATE_LIMIT_REQUESTS: u32 = 100;
    pub const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;
    pub const RATE_LIMIT_WINDOW_DAYS: i64 = 30;

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

    /// Activity processing limits
    pub const ACTIVITY_CAPACITY_HINT: usize = 100;

    /// Mathematical constants
    pub const PERCENTAGE_MULTIPLIER: f64 = 100.0;

    /// Time constants
    pub const SECONDS_PER_HOUR: u64 = 3600;
    pub const MINUTES_PER_HOUR: u64 = 60;
    pub const HOURS_PER_DAY: i64 = 24;
    pub const DAYS_PER_WEEK: i64 = 7;
    pub const DAYS_PER_MONTH: i64 = 30;
    pub const DAYS_PER_YEAR: i64 = 365;

    /// Common timeframe limits
    pub const MAX_TIMEFRAME_DAYS: u32 = 365;
    pub const DEFAULT_TRIAL_DAYS: i64 = 30;
    pub const DEFAULT_SESSION_HOURS: i64 = 24;

    /// Confidence thresholds  
    pub const DEFAULT_CONFIDENCE_THRESHOLD: f64 = 0.7;
    pub const HIGH_CONFIDENCE_THRESHOLD: f64 = 0.8;
    pub const LOW_CONFIDENCE_THRESHOLD: f64 = 0.6;

    /// Default heart rate
    pub const DEFAULT_RESTING_HEART_RATE: u16 = 60;

    /// Common capacity hints
    pub const DEFAULT_ACTIVITY_LIMIT: u32 = 30;
    pub const MAX_ACTIVITIES_QUERY: usize = 1000;
    pub const LARGE_CAPACITY_HINT: usize = 1000;
}

/// Timeout and duration constants
pub mod timeouts {
    /// Health check timeouts
    pub const HEALTH_CHECK_TIMEOUT_SECS: u64 = 5;
    pub const HEALTH_CACHE_TTL_SECS: u64 = 30;
    pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 3600; // 1 hour

    /// `OAuth` timeouts
    pub const OAUTH_STATE_EXPIRY_MINUTES: i64 = 10;
    pub const TOKEN_REFRESH_BUFFER_MINUTES: i64 = 5;
    pub const TOKEN_EXPIRY_WARNING_MINUTES: i64 = 5;
    pub const DEFAULT_TOKEN_EXPIRY_HOURS: i64 = 6;

    /// Auth setup and general operations
    pub const AUTH_SETUP_WAIT_SECS: u64 = 60;

    /// Admin token expiry
    pub const ADMIN_TOKEN_DEFAULT_EXPIRY_DAYS: i64 = 365;

    /// Trial period
    pub const DEFAULT_TRIAL_DAYS: i64 = 14;
}

/// Cryptographic and security constants
pub mod crypto {
    /// `JWT` and secret lengths
    pub const JWT_SECRET_LENGTH: usize = 64;

    /// `API` key configuration
    pub const API_KEY_RANDOM_LENGTH: usize = 32;
    pub const API_KEY_PREFIX_LENGTH: usize = 12;
    pub const TRIAL_KEY_LENGTH: usize = 41;
    pub const LIVE_KEY_LENGTH: usize = 40;

    /// `OAuth` and `PKCE`
    pub const PKCE_CODE_VERIFIER_LENGTH: usize = 128;
}

/// Security header constants
pub mod security {
    /// `HSTS` max-age values
    pub const HSTS_MAX_AGE_DEV: u32 = 86400; // 1 day in seconds
    pub const HSTS_MAX_AGE_PROD: u32 = 31_536_000; // 1 year in seconds
}

/// `OAuth` scopes and provider defaults
pub mod oauth {
    /// Default `OAuth` scopes for Strava
    pub const STRAVA_DEFAULT_SCOPES: &str = "activity:read_all";

    /// Default `OAuth` scopes for Fitbit  
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

    /// Default goal timeframe in days
    pub const DEFAULT_GOAL_TIMEFRAME_DAYS: i64 = 90;
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

/// `MCP` tool names
pub mod tools {
    /// Core tools
    pub const GET_ACTIVITIES: &str = "get_activities";
    pub const GET_ATHLETE: &str = "get_athlete";
    pub const GET_STATS: &str = "get_stats";
    pub const GET_ACTIVITY_INTELLIGENCE: &str = "get_activity_intelligence";

    /// Connection management
    pub const GET_CONNECTION_STATUS: &str = "get_connection_status";
    pub const DISCONNECT_PROVIDER: &str = "disconnect_provider";

    /// Notification management
    pub const MARK_NOTIFICATIONS_READ: &str = "mark_notifications_read";
    pub const GET_NOTIFICATIONS: &str = "get_notifications";
    pub const ANNOUNCE_OAUTH_SUCCESS: &str = "announce_oauth_success";
    pub const CHECK_OAUTH_NOTIFICATIONS: &str = "check_oauth_notifications";

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

/// Common `JSON` field names
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

/// System configuration constants for rates, limits and thresholds
pub mod system_config {
    /// `API` tier rate limits (requests per month)
    pub const TRIAL_MONTHLY_LIMIT: u32 = 1_000;
    pub const STARTER_MONTHLY_LIMIT: u32 = 10_000;
    pub const PROFESSIONAL_MONTHLY_LIMIT: u32 = 100_000;

    /// Trial period duration (days)
    pub const TRIAL_PERIOD_DAYS: u32 = 14;

    /// Rate limiting window duration (seconds)
    /// 30 days converted to seconds for rate limit calculations
    pub const RATE_LIMIT_WINDOW_SECONDS: u32 = 30 * 24 * 60 * 60;

}

/// Time conversion constants for various durations
pub mod time_constants {
    /// Basic time unit conversions (seconds)
    pub const SECONDS_PER_HOUR: u32 = 3600;
    pub const SECONDS_PER_DAY: u32 = 86400;
    pub const SECONDS_PER_WEEK: u32 = 604_800;
    pub const SECONDS_PER_MONTH: u32 = 2_592_000;

    /// Hour conversion as floating point for calculations
    pub const SECONDS_PER_HOUR_F64: f64 = 3600.0;

    /// Cache duration constants
    pub const LOCATION_CACHE_DURATION_SECS: u64 = 24 * 60 * 60; // 24 hours
    pub const WEATHER_CACHE_HOUR_BUCKET: u64 = 3600; // 1 hour for cache bucketing

    /// A2A token expiry (24 hours in seconds)
    pub const DEFAULT_A2A_TOKEN_EXPIRY_SECONDS: u64 = 86400;
}

/// Network and protocol configuration
pub mod network_config {
    /// Port offset for `HTTP` server in multitenant mode
    pub const HTTP_PORT_OFFSET: u16 = 1000;

    /// Default test port for development
    pub const DEFAULT_TEST_PORT: u16 = 3000;

    /// Default `MCP` protocol version string
    pub const DEFAULT_MCP_VERSION: &str = "2025-06-18";

    /// Default host for development
    pub const DEFAULT_HOST: &str = "localhost";

    /// Development localhost patterns for CORS validation
    pub const LOCALHOST_PATTERNS: &[&str] = &[
        "http://localhost",
        "https://localhost",
        "http://127.0.0.1",
        "https://127.0.0.1",
    ];
}

/// Demo and test data constants
pub mod demo_data {
    /// Demo user profile constants
    pub const DEMO_USER_AGE: u32 = 30;
    pub const DEMO_PREFERRED_DURATION_MINUTES: u32 = 60;

    /// Demo efficiency score for examples and testing
    pub const DEMO_EFFICIENCY_SCORE: f64 = 85.0;

    /// Demo consistency score for examples and testing
    pub const DEMO_CONSISTENCY_SCORE: f64 = 88.0;

    /// Test IP address for demos and tests
    pub const TEST_IP_ADDRESS: &str = "127.0.0.1";

    /// Sample activity duration times in seconds for demos
    pub const SAMPLE_SHORT_RUN_SECONDS: u32 = 1800; // 30 minutes
    pub const SAMPLE_MEDIUM_RUN_SECONDS: u32 = 3600; // 1 hour
    pub const SAMPLE_LONG_RUN_SECONDS: u32 = 7200; // 2 hours
    pub const SAMPLE_ULTRA_RUN_SECONDS: u32 = 18000; // 5 hours
    pub const SAMPLE_MARATHON_SECONDS: u32 = 8000; // ~2.2 hours
    pub const SAMPLE_ULTRA_MARATHON_SECONDS: u32 = 19000; // ~5.3 hours
    pub const SAMPLE_IRONMAN_BIKE_SECONDS: u32 = 72000; // 20 hours
    pub const SAMPLE_IRONMAN_TOTAL_SECONDS: u32 = 80000; // ~22 hours

    /// Sample distances in meters for demos
    pub const SAMPLE_5K_METERS: f64 = 5000.0; // 5K run distance

    /// Sample athlete/user IDs for demos
    pub const SAMPLE_ATHLETE_ID: &str = "9876543210"; // Demo athlete ID

    /// Unit conversion constants
    pub const METERS_TO_KM: f64 = 1000.0; // Convert meters to kilometers
}

/// External API provider limits and configuration constants
///
/// These constants define the technical limits and constraints imposed by external
/// fitness tracking APIs like Strava and Fitbit. They should match the actual
/// API documentation and be updated when providers change their limits.
pub mod api_provider_limits {
    /// Strava API rate limiting and pagination constants
    /// Reference: <https://developers.strava.com/docs/rate-limits/>
    pub mod strava {
        /// Maximum activities that can be requested in a single API call
        /// This is enforced by Strava's `per_page` parameter limit
        pub const MAX_ACTIVITIES_PER_REQUEST: usize = 200;

        /// Default number of activities per page when no limit is specified
        /// This matches Strava's API default behavior
        pub const DEFAULT_ACTIVITIES_PER_PAGE: usize = 30;

        /// Rate limit for "non-upload" endpoints (activities, athlete data, etc.)
        /// These endpoints have lower limits than upload endpoints
        pub const RATE_LIMIT_REQUESTS_PER_15MIN: u32 = 100;
        pub const RATE_LIMIT_REQUESTS_PER_DAY: u32 = 1000;

        /// Overall API rate limits (applies to all endpoints combined)
        /// These are higher limits that apply when you haven't hit the non-upload limits
        pub const OVERALL_RATE_LIMIT_REQUESTS_PER_15MIN: u32 = 200;
        pub const OVERALL_RATE_LIMIT_REQUESTS_PER_DAY: u32 = 2000;

        /// Rate limit reset intervals (in minutes)
        /// Strava resets rate limits at natural 15-minute intervals
        pub const RATE_LIMIT_RESET_INTERVAL_MINUTES: u32 = 15;
    }

    /// Fitbit API limits and constraints
    /// Reference: <https://dev.fitbit.com/build/reference/web-api/developer-guide/application-design/>
    pub mod fitbit {
        /// Fitbit has different rate limiting structure than Strava
        pub const RATE_LIMIT_REQUESTS_PER_HOUR: u32 = 150;
        pub const RATE_LIMIT_REQUESTS_PER_DAY: u32 = 3600; // 150 * 24
    }
}

/// API tier constants to eliminate hardcoded strings throughout the codebase
pub mod tiers {
    /// Trial tier identifier
    pub const TRIAL: &str = "trial";

    /// Starter tier identifier
    pub const STARTER: &str = "starter";

    /// Professional tier identifier
    pub const PROFESSIONAL: &str = "professional";

    /// Enterprise tier identifier
    pub const ENTERPRISE: &str = "enterprise";

    /// Alternative professional tier identifier
    pub const PRO: &str = "pro";

    /// Alternative enterprise tier identifier
    pub const ENT: &str = "ent";
}

/// API key prefix constants to eliminate hardcoded key format strings
pub mod key_prefixes {
    /// Production API key prefix
    pub const API_KEY_LIVE: &str = "pk_live_";

    /// Test API key prefix  
    pub const API_KEY_TEST: &str = "pk_test_";

    /// Production secret key prefix
    pub const SECRET_KEY_LIVE: &str = "sk_live_";

    /// Test secret key prefix
    pub const SECRET_KEY_TEST: &str = "sk_test_";
}

/// Service name constants to eliminate hardcoded service identifiers
pub mod service_names {
    /// Main MCP server service name
    pub const PIERRE_MCP_SERVER: &str = "pierre-mcp-server";

    /// Admin API service identifier
    pub const ADMIN_API: &str = "admin-api";

    /// Pierre MCP admin API service name
    pub const PIERRE_MCP_ADMIN_API: &str = "pierre-mcp-admin-api";
}

/// Error message constants for consistent error handling
pub mod error_messages {
    // Validation errors
    pub const INVALID_EMAIL_FORMAT: &str = "Invalid email format";
    pub const PASSWORD_TOO_WEAK: &str = "Password must be at least 8 characters long";
    pub const INVALID_CREDENTIALS: &str = "Invalid email or password";

    // User state errors
    pub const USER_ALREADY_EXISTS: &str = "User with this email already exists";
    pub const USER_NOT_FOUND: &str = "User not found";

    // Authentication errors
    pub const TOKEN_EXCHANGE_FAILED: &str = "OAuth token exchange failed";
    pub const INVALID_TOKEN: &str = "Invalid or expired token";
}

/// Time-related constants in seconds
pub mod time {
    /// One minute in seconds
    pub const MINUTE_SECONDS: i64 = 60;

    /// One hour in seconds
    pub const HOUR_SECONDS: i64 = 3600;

    /// One day in seconds  
    pub const DAY_SECONDS: i64 = 86400;

    /// One week in seconds
    pub const WEEK_SECONDS: i64 = 604_800;

    /// Default OAuth token expiry (1 hour)
    pub const DEFAULT_TOKEN_EXPIRY_SECONDS: i64 = HOUR_SECONDS;

    /// Default rate limit window (1 minute)
    pub const DEFAULT_RATE_LIMIT_WINDOW_SECONDS: i64 = MINUTE_SECONDS;
}

/// HTTP status code constants
pub mod http_status {
    /// HTTP 200 OK range minimum
    pub const SUCCESS_MIN: u16 = 200;

    /// HTTP 299 OK range maximum  
    pub const SUCCESS_MAX: u16 = 299;

    /// HTTP 400 Bad Request
    pub const BAD_REQUEST: u16 = 400;

    /// HTTP 401 Unauthorized
    pub const UNAUTHORIZED: u16 = 401;

    /// HTTP 403 Forbidden
    pub const FORBIDDEN: u16 = 403;

    /// HTTP 404 Not Found
    pub const NOT_FOUND: u16 = 404;

    /// HTTP 409 Conflict
    pub const CONFLICT: u16 = 409;

    /// HTTP 422 Unprocessable Entity
    pub const UNPROCESSABLE_ENTITY: u16 = 422;

    /// HTTP 429 Too Many Requests
    pub const TOO_MANY_REQUESTS: u16 = 429;

    /// HTTP 500 Internal Server Error
    pub const INTERNAL_SERVER_ERROR: u16 = 500;

    /// HTTP 502 Bad Gateway
    pub const BAD_GATEWAY: u16 = 502;

    /// HTTP 503 Service Unavailable
    pub const SERVICE_UNAVAILABLE: u16 = 503;
}

/// Rate limiting constants
pub mod rate_limits {
    /// Default burst limit for rate limiting
    pub const DEFAULT_BURST_LIMIT: u32 = 100;

    /// WebSocket broadcast channel capacity
    pub const WEBSOCKET_CHANNEL_CAPACITY: usize = 1000;

    /// Database query result limit
    pub const DEFAULT_QUERY_LIMIT: u32 = 1000;
}

/// Physiological constants  
pub mod physiology {
    /// Default resting heart rate (BPM)
    pub const DEFAULT_RESTING_HR: u16 = 60;

    /// Default max heart rate (BPM)  
    pub const DEFAULT_MAX_HR: u16 = 190;

    /// Normal cadence range maximum (RPM)
    pub const MAX_NORMAL_CADENCE: u16 = 300;

    /// Meters per kilometer conversion
    pub const METERS_PER_KM: f64 = 1000.0;

    /// Maximum normal heart rate (BPM) for validation
    pub const MAX_NORMAL_HR: u16 = 300;

    /// Minimum good ground contact time in milliseconds
    pub const MIN_GOOD_GCT_MS: f64 = 200.0;

    /// Maximum good ground contact time in milliseconds
    pub const MAX_GOOD_GCT_MS: f64 = 300.0;

    /// Optimal ground contact time in milliseconds
    pub const OPTIMAL_GCT_MS: f64 = 250.0;
}

/// OAuth provider constants to eliminate hardcoded provider names
pub mod oauth_providers {
    /// Strava fitness provider identifier
    pub const STRAVA: &str = "strava";

    /// Fitbit fitness provider identifier  
    pub const FITBIT: &str = "fitbit";

    /// Get all supported OAuth providers
    #[must_use]
    pub const fn all() -> &'static [&'static str] {
        &[STRAVA, FITBIT]
    }

    /// Check if a provider is supported
    #[must_use]
    pub fn is_supported(provider: &str) -> bool {
        all().contains(&provider)
    }
}
