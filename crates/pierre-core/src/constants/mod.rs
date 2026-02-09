// ABOUTME: Constants module with domain-separated organization
// ABOUTME: Pure data constants organized by domain for the Pierre fitness platform
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Constants module
//!
//! This module organizes application constants by domain for better maintainability.
//! Constants are grouped into logical domains rather than being in a single large file.

// Domain-specific modules

/// Cache-related constants (TTL, sizes, etc.)
pub mod cache;
/// Error codes and error-related constants
pub mod errors;
/// OAuth provider constants and configuration
pub mod oauth;
/// Protocol-specific constants for MCP
pub mod protocol;
/// Multi-protocol constants (A2A, MCP, etc.)
pub mod protocols;
/// Tool identifiers and tool-related constants
pub mod tools;
/// Unit conversion and measurement constants
pub mod units;

// Re-export commonly used items for easier access
pub use errors::*;
pub use oauth::*;
/// Tool-related constants re-export
pub use tools::*;
// Note: protocol and protocols are kept as modules to avoid conflicts

/// OAuth provider constants
pub mod oauth_providers {
    /// Re-export all OAuth constants
    pub use super::oauth::*;
}

// Remaining constants organized by domain

/// API endpoints
pub mod endpoints {
    /// Health check endpoint
    pub const HEALTH_CHECK: &str = "/health";
    /// API base path
    pub const API_BASE: &str = "/api";
}

/// Network ports
pub mod ports {
    /// Default HTTP port
    pub const DEFAULT_HTTP_PORT: u16 = 8081;
    /// Default MCP port
    pub const DEFAULT_MCP_PORT: u16 = 8081;
    /// Default docs port
    pub const DEFAULT_DOCS_PORT: u16 = 8082;
    /// Default OAuth callback port (for bridge focus recovery)
    pub const DEFAULT_OAUTH_CALLBACK_PORT: u16 = 35535;
}

/// API routes
pub mod routes {
    /// Health route
    pub const HEALTH: &str = "/health";
    /// Activities route
    pub const ACTIVITIES: &str = "/activities";
    /// Stats route
    pub const STATS: &str = "/stats";
    /// Connect route
    pub const CONNECT: &str = "/connect";
    /// OAuth callback route
    pub const OAUTH_CALLBACK: &str = "/oauth/callback";
}

/// Default limits
pub mod limits {
    /// Default activities fetch limit
    pub const DEFAULT_ACTIVITIES_LIMIT: usize = 20;
    /// Maximum activities that can be fetched in one request
    pub const MAX_ACTIVITIES_FETCH: usize = 100;
    /// Minutes per hour
    pub const MINUTES_PER_HOUR: u64 = 60;
    /// Seconds per minute
    pub const SECONDS_PER_MINUTE: u64 = 60;
    /// Maximum timeframe days
    pub const MAX_TIMEFRAME_DAYS: u32 = 365;
    /// Activity capacity hint
    pub const ACTIVITY_CAPACITY_HINT: usize = 50;
    /// Meters per kilometer
    pub const METERS_PER_KILOMETER: f64 = 1000.0;
    /// Percentage multiplier
    pub const PERCENTAGE_MULTIPLIER: f64 = 100.0;
    /// Default session hours for JWT tokens
    pub const DEFAULT_SESSION_HOURS: i64 = 24;
    /// User session JWT expiry hours (24 hours for logged-in users)
    pub const USER_SESSION_EXPIRY_HOURS: i64 = 24;
    /// OAuth access token expiry hours (1 hour per RFC 8252 Security Best Practices)
    pub const OAUTH_ACCESS_TOKEN_EXPIRY_HOURS: i64 = 1;
    /// Maximum request size in bytes
    pub const MAX_REQUEST_SIZE: usize = 1_048_576; // 1MB
    /// Maximum response size in bytes
    pub const MAX_RESPONSE_SIZE: usize = 10_485_760; // 10MB
    /// Default backup interval in seconds
    pub const DEFAULT_BACKUP_INTERVAL_SECS: u64 = 86400; // 24 hours
    /// Default backup retention count
    pub const DEFAULT_BACKUP_RETENTION_COUNT: u32 = 7;
    /// Default rate limit requests
    pub const DEFAULT_RATE_LIMIT_REQUESTS: u32 = 100;
    /// Default rate limit window seconds
    pub const DEFAULT_RATE_LIMIT_WINDOW_SECS: u64 = 60;
    /// Default confidence threshold
    pub const DEFAULT_CONFIDENCE_THRESHOLD: f64 = 0.85;
}

/// Timeout configurations
pub mod timeouts {
    /// Default HTTP client request timeout in seconds
    pub const HTTP_CLIENT_TIMEOUT_SECS: u64 = 30;
    /// Default HTTP client connect timeout in seconds
    pub const HTTP_CLIENT_CONNECT_TIMEOUT_SECS: u64 = 10;
    /// OAuth client request timeout in seconds
    pub const OAUTH_CLIENT_TIMEOUT_SECS: u64 = 15;
    /// OAuth client connect timeout in seconds
    pub const OAUTH_CLIENT_CONNECT_TIMEOUT_SECS: u64 = 5;
    /// API client request timeout in seconds
    pub const API_CLIENT_TIMEOUT_SECS: u64 = 60;
    /// API client connect timeout in seconds
    pub const API_CLIENT_CONNECT_TIMEOUT_SECS: u64 = 10;
    /// Health check client timeout in seconds
    pub const HEALTH_CHECK_TIMEOUT_SECS: u64 = 5;
    /// OAuth callback notification timeout in seconds
    pub const OAUTH_CALLBACK_NOTIFICATION_TIMEOUT_SECS: u64 = 5;
    /// Database connection timeout in seconds
    pub const DATABASE_TIMEOUT_SECS: u64 = 10;
    /// OAuth callback wait timeout in seconds (for bridge flow)
    pub const OAUTH_CALLBACK_WAIT_TIMEOUT_SECS: u64 = 300; // 5 minutes
    /// SSE cleanup task interval in seconds
    pub const SSE_CLEANUP_INTERVAL_SECS: u64 = 300; // 5 minutes
    /// SSE connection timeout in seconds (inactive connections removed after this duration)
    pub const SSE_CONNECTION_TIMEOUT_SECS: u64 = 600; // 10 minutes
    /// OAuth session cookie Max-Age in seconds (matches JWT expiration)
    pub const SESSION_COOKIE_MAX_AGE_SECS: u64 = 86400; // 24 hours
}

/// Cryptographic constants
pub mod crypto {
    /// Token prefix for `API` keys
    pub const TOKEN_PREFIX: &str = "pk_";
    /// Secret key minimum length
    pub const SECRET_KEY_MIN_LENGTH: usize = 32;
}

/// Security configurations
pub mod security {
    /// CORS allowed origins
    pub const CORS_ALLOWED_ORIGINS: &str = "*";
}

/// OAuth configuration constants
pub mod oauth_config {
    /// OAuth state parameter length
    pub const STATE_LENGTH: usize = 32;
    /// OAuth code challenge length
    pub const CODE_CHALLENGE_LENGTH: usize = 128;
    /// OAuth authorization URL expiration time in minutes
    /// Authorization URLs remain valid for 10 minutes
    pub const AUTHORIZATION_EXPIRES_MINUTES: u32 = 10;
}

/// API key system configuration
pub mod system_config {
    /// Trial tier monthly limit
    pub const TRIAL_MONTHLY_LIMIT: u32 = 1_000;
    /// Starter tier monthly limit
    pub const STARTER_MONTHLY_LIMIT: u32 = 10_000;
    /// Professional tier monthly limit
    pub const PROFESSIONAL_MONTHLY_LIMIT: u32 = 100_000;
    /// Rate limit window in seconds (30 days)
    pub const RATE_LIMIT_WINDOW_SECONDS: u32 = 30 * 24 * 60 * 60;
    /// Default trial period in days
    pub const TRIAL_PERIOD_DAYS: i64 = 14;
}

/// API key prefixes
pub mod key_prefixes {
    /// Live API key prefix
    pub const LIVE: &str = "pk_live_";
    /// Trial API key prefix
    pub const TRIAL: &str = "pk_trial_";
}

/// API key tiers
pub mod tiers {
    /// Trial tier
    pub const TRIAL: &str = "trial";
    /// Starter tier
    pub const STARTER: &str = "starter";
    /// Professional tier
    pub const PROFESSIONAL: &str = "professional";
    /// Enterprise tier
    pub const ENTERPRISE: &str = "enterprise";
    /// Professional tier (short name)
    pub const PRO: &str = "professional";
    /// Enterprise tier (short name)
    pub const ENT: &str = "enterprise";
}

/// Default values
pub mod defaults {
    /// Default page size for paginated responses
    pub const PAGE_SIZE: usize = 20;
    /// Default cache TTL in seconds
    pub const CACHE_TTL_SECS: u64 = 300; // 5 minutes
    /// Default goal timeframe in days
    pub const DEFAULT_GOAL_TIMEFRAME_DAYS: u32 = 30;
    /// Default backup directory
    pub const DEFAULT_BACKUP_DIR: &str = "./backups";
    /// Default weather cache TTL
    pub const DEFAULT_WEATHER_CACHE_TTL_SECS: u64 = 1800; // 30 minutes
    /// Default analytics cache TTL
    pub const DEFAULT_ANALYTICS_CACHE_TTL_SECS: u64 = 3600; // 1 hour
}

/// Configuration system constants
pub mod configuration_system {
    /// Number of available configuration parameters in catalog
    ///
    /// Total count of configuration options exposed via MCP configuration tools
    /// Used for catalog size reporting and validation
    pub const AVAILABLE_PARAMETERS_COUNT: usize = 25;
}

/// Database configuration
pub mod database {
    /// Connection pool minimum size
    pub const POOL_MIN_SIZE: u32 = 1;
    /// Connection pool maximum size
    pub const POOL_MAX_SIZE: u32 = 10;
    /// Connection timeout in seconds
    pub const CONNECTION_TIMEOUT_SECS: u64 = 30;
    /// Query timeout in seconds
    pub const QUERY_TIMEOUT_SECS: u64 = 30;
    /// Migration timeout in seconds
    pub const MIGRATION_TIMEOUT_SECS: u64 = 300;
    /// Default number of connection retries on startup
    pub const CONNECTION_RETRIES: u32 = 5;
    /// Initial retry delay in milliseconds for exponential backoff
    pub const INITIAL_RETRY_DELAY_MS: u64 = 500;
    /// Maximum retry delay in milliseconds
    pub const MAX_RETRY_DELAY_MS: u64 = 30_000;
    /// Exponential backoff base multiplier
    pub const RETRY_BACKOFF_MULTIPLIER: u32 = 2;
}

/// Redis connection configuration
pub mod redis {
    /// Redis connection timeout in seconds
    pub const CONNECTION_TIMEOUT_SECS: u64 = 10;
    /// Redis response timeout in seconds
    pub const RESPONSE_TIMEOUT_SECS: u64 = 5;
    /// Number of reconnection retries
    pub const RECONNECTION_RETRIES: usize = 5;
    /// Exponential backoff base for retry delays
    pub const RETRY_EXPONENT_BASE: u64 = 2;
    /// Maximum retry delay in milliseconds
    pub const MAX_RETRY_DELAY_MS: u64 = 30_000;
    /// Initial connection retry count
    pub const INITIAL_CONNECTION_RETRIES: u32 = 3;
}

/// Status codes and messages
pub mod status {
    /// HTTP status codes
    pub mod http {
        /// OK
        pub const OK: u16 = 200;
        /// Created
        pub const CREATED: u16 = 201;
        /// Bad Request
        pub const BAD_REQUEST: u16 = 400;
        /// Unauthorized
        pub const UNAUTHORIZED: u16 = 401;
        /// Forbidden
        pub const FORBIDDEN: u16 = 403;
        /// Not Found
        pub const NOT_FOUND: u16 = 404;
        /// Internal Server Error
        pub const INTERNAL_SERVER_ERROR: u16 = 500;
    }

    /// MCP status messages
    pub mod mcp {
        /// Connected
        pub const CONNECTED: &str = "connected";
        /// Disconnected
        pub const DISCONNECTED: &str = "disconnected";
        /// Error
        pub const ERROR: &str = "error";
    }
}

/// Field names for JSON
pub mod json_fields {
    /// User ID field
    pub const USER_ID: &str = "user_id";
    /// Provider field
    pub const PROVIDER: &str = "provider";
    /// Activities field
    pub const ACTIVITIES: &str = "activities";
    /// Activity ID field
    pub const ACTIVITY_ID: &str = "activity_id";
    /// Goal ID field
    pub const GOAL_ID: &str = "goal_id";
    /// Limit field
    pub const LIMIT: &str = "limit";
    /// Offset field
    pub const OFFSET: &str = "offset";
    /// Before timestamp field (Unix epoch seconds) - get activities before this time
    pub const BEFORE: &str = "before";
    /// After timestamp field (Unix epoch seconds) - get activities after this time
    pub const AFTER: &str = "after";
    /// Mode field for response detail level (summary vs detailed)
    pub const MODE: &str = "mode";
    /// Sport type field for filtering activities by sport type
    pub const SPORT_TYPE: &str = "sport_type";
    /// Output format field for serialization format (json, toon)
    pub const FORMAT: &str = "format";
}

/// System configuration messages
pub mod messages {
    /// Startup message
    pub const STARTUP: &str = "Pierre MCP Server starting up";
    /// Shutdown message
    pub const SHUTDOWN: &str = "Pierre MCP Server shutting down";
    /// Health check message
    pub const HEALTH_OK: &str = "Service healthy";
}

/// Service names
pub mod service_names {
    /// MCP service
    pub const MCP: &str = "mcp";
    /// Auth service
    pub const AUTH: &str = "auth";
    /// OAuth service
    pub const OAUTH: &str = "oauth";
    /// Activity service
    pub const ACTIVITY: &str = "activity";
    /// Health service
    pub const HEALTH: &str = "health";
    /// Pierre MCP Server
    pub const PIERRE_MCP_SERVER: &str = "pierre-mcp-server";
    /// Admin API service
    pub const ADMIN_API: &str = "admin_api";
    /// Pierre MCP Admin API
    pub const PIERRE_MCP_ADMIN_API: &str = "pierre-mcp-admin-api";
}

/// Project metadata constants
pub mod project {
    /// Project name
    pub const NAME: &str = "Pierre MCP Server";
    /// Project version (synced from Cargo.toml at compile time)
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    /// Project repository URL (synced from Cargo.toml at compile time)
    pub const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

    /// Builds the HTTP User-Agent string for external API requests
    ///
    /// Format: `Pierre MCP Server/{version} ({repository_url})`
    #[must_use]
    pub fn user_agent() -> String {
        format!("{NAME}/{VERSION} ({REPOSITORY_URL})")
    }
}

/// Time constants
pub mod time_constants {
    /// Seconds per hour
    pub const SECONDS_PER_HOUR: u32 = 3600;
    /// Seconds per minute
    pub const SECONDS_PER_MINUTE: u64 = 60;
    /// Seconds per hour as f64
    pub const SECONDS_PER_HOUR_F64: f64 = 3600.0;
    /// Seconds per day
    pub const SECONDS_PER_DAY: u32 = 86_400;
    /// Seconds per week
    pub const SECONDS_PER_WEEK: u32 = 604_800;
    /// Seconds per month (30 days)
    pub const SECONDS_PER_MONTH: u32 = 2_592_000;
    /// Seconds per year (365 days)
    pub const SECONDS_PER_YEAR: u64 = 31_536_000;
    /// Minutes per hour
    pub const MINUTES_PER_HOUR: u64 = 60;
    /// Hours per day
    pub const HOURS_PER_DAY: u64 = 24;
    /// Days per week
    pub const DAYS_PER_WEEK: u32 = 7;
    /// Days per month (30-day approximation for calculations)
    pub const DAYS_PER_MONTH: u32 = 30;
    /// Days per quarter (90-day approximation for calculations)
    pub const DAYS_PER_QUARTER: u32 = 90;
    /// Days per year (standard calendar year)
    pub const DAYS_PER_YEAR: u32 = 365;
}

/// Error messages
pub mod error_messages {
    /// Invalid credentials message
    pub const INVALID_CREDENTIALS: &str = "Invalid credentials provided";
    /// Unauthorized access message
    pub const UNAUTHORIZED: &str = "Unauthorized access";
    /// Token expired message
    pub const TOKEN_EXPIRED: &str = "Token has expired";
    /// Invalid token message
    pub const INVALID_TOKEN: &str = "Invalid token provided";
    /// Invalid email format message
    pub const INVALID_EMAIL_FORMAT: &str = "Invalid email format";
    /// Password too weak message
    pub const PASSWORD_TOO_WEAK: &str = "Password must be at least 8 characters";
    /// User already exists message
    pub const USER_ALREADY_EXISTS: &str = "User already exists";
    /// User not found message
    pub const USER_NOT_FOUND: &str = "User not found";
    /// Database connection error
    pub const DATABASE_CONNECTION_ERROR: &str = "Database connection error";
    /// Rate limit exceeded
    pub const RATE_LIMIT_EXCEEDED: &str = "Rate limit exceeded";
}

/// Rate limiting constants
pub mod rate_limits {
    /// Strava 15-minute rate limit
    pub const STRAVA_RATE_LIMIT_15MIN: u32 = 100;
    /// Strava daily rate limit
    pub const STRAVA_RATE_LIMIT_DAILY: u32 = 15000;
    /// Fitbit hourly rate limit
    pub const FITBIT_RATE_LIMIT_HOURLY: u32 = 150;
    /// Fitbit daily rate limit
    pub const FITBIT_RATE_LIMIT_DAILY: u32 = 1000;
    /// Fitbit default daily rate limit
    pub const FITBIT_DEFAULT_DAILY_RATE_LIMIT: u32 = 1000;
    /// Strava default daily rate limit
    pub const STRAVA_DEFAULT_DAILY_RATE_LIMIT: u32 = 15000;
    /// Garmin default daily rate limit
    pub const GARMIN_DEFAULT_DAILY_RATE_LIMIT: u32 = 1000;
    /// WHOOP default daily rate limit
    pub const WHOOP_DEFAULT_DAILY_RATE_LIMIT: u32 = 1000;
    /// Terra default daily rate limit
    pub const TERRA_DEFAULT_DAILY_RATE_LIMIT: u32 = 1000;
    /// Default burst limit
    pub const DEFAULT_BURST_LIMIT: u32 = 10;
    /// Default rate limit window
    pub const DEFAULT_RATE_LIMIT_WINDOW: u64 = 60;
    /// WebSocket channel capacity
    pub const WEBSOCKET_CHANNEL_CAPACITY: usize = 1000;
}

/// User default values
pub mod user_defaults {
    /// Default user age
    pub const DEFAULT_USER_AGE: i32 = 30;
    /// Default goal distance in kilometers
    pub const DEFAULT_GOAL_DISTANCE: f64 = 100.0;
}

/// API provider limits
pub mod api_provider_limits {
    /// Strava rate limit per 15 minutes
    pub const STRAVA_RATE_LIMIT_15MIN: u32 = 100;
    /// Strava daily rate limit
    pub const STRAVA_RATE_LIMIT_DAILY: u32 = 15000;
    /// Fitbit hourly rate limit
    pub const FITBIT_RATE_LIMIT_HOURLY: u32 = 150;
    /// Fitbit daily rate limit
    pub const FITBIT_RATE_LIMIT_DAILY: u32 = 1000;

    /// Strava specific limits
    pub mod strava {
        /// Default activities per page
        pub const DEFAULT_ACTIVITIES_PER_PAGE: usize = 30;
        /// Maximum activities per request
        pub const MAX_ACTIVITIES_PER_REQUEST: usize = 200;
    }

    /// Garmin Connect API limits
    ///
    /// **IMPORTANT**: These limits are based on community observations of unofficial API endpoints.
    /// Garmin does not publicly document rate limits for their unofficial API.
    pub mod garmin {
        /// Default number of activities per page request
        pub const DEFAULT_ACTIVITIES_PER_PAGE: usize = 20;
        /// Maximum activities per single API request
        pub const MAX_ACTIVITIES_PER_REQUEST: usize = 100;
        /// Recommended maximum requests per hour per user
        pub const RECOMMENDED_MAX_REQUESTS_PER_HOUR: usize = 100;
        /// Recommended minimum interval between login attempts (seconds)
        pub const RECOMMENDED_MIN_LOGIN_INTERVAL_SECS: u64 = 300;
        /// Rate limit HTTP status code
        pub const RATE_LIMIT_HTTP_STATUS: u16 = 429;
        /// Estimated rate limit block duration (seconds)
        pub const ESTIMATED_RATE_LIMIT_BLOCK_DURATION_SECS: u64 = 3600;
    }
}

/// Time-related constants
pub mod time {
    /// Default token expiry in seconds (1 hour)
    pub const DEFAULT_TOKEN_EXPIRY_SECONDS: i64 = 3600;
    /// Seconds in a minute
    pub const MINUTE_SECONDS: i64 = 60;
    /// Seconds in an hour
    pub const HOUR_SECONDS: i64 = 3600;
    /// Seconds in a day
    pub const DAY_SECONDS: i64 = 86_400;
    /// Unix epoch start
    pub const UNIX_EPOCH: &str = "1970-01-01T00:00:00Z";
    /// ISO 8601 format
    pub const ISO_8601_FORMAT: &str = "%Y-%m-%dT%H:%M:%SZ";
    /// Date format
    pub const DATE_FORMAT: &str = "%Y-%m-%d";
    /// Time format
    pub const TIME_FORMAT: &str = "%H:%M:%S";
}

/// Network configuration
pub mod network_config {
    /// TCP keep alive timeout in seconds
    pub const TCP_KEEP_ALIVE_SECS: u64 = 60;
    /// TCP no delay
    pub const TCP_NODELAY: bool = true;
    /// `SO_REUSEADDR`
    pub const SO_REUSEADDR: bool = true;
    /// OAuth code verifier length
    pub const OAUTH_CODE_VERIFIER_LENGTH: usize = 128;
    /// Localhost patterns for validation
    pub const LOCALHOST_PATTERNS: &[&str] = &["localhost", "127.0.0.1", "::1", "0.0.0.0"];
    /// HTTP client error threshold
    pub const HTTP_CLIENT_ERROR_THRESHOLD: u16 = 400;
    /// SSE broadcast channel size
    pub const SSE_BROADCAST_CHANNEL_SIZE: usize = 1000;
    /// Maximum concurrent SSE connections per user (`DoS` prevention)
    pub const SSE_MAX_CONNECTIONS_PER_USER: usize = 5;
}

/// Physiology constants
pub mod physiology {
    /// Minimum good ground contact time in milliseconds
    pub const MIN_GOOD_GCT_MS: f64 = 180.0;
    /// Maximum good ground contact time in milliseconds
    pub const MAX_GOOD_GCT_MS: f64 = 250.0;
    /// Optimal ground contact time in milliseconds
    pub const OPTIMAL_GCT_MS: f64 = 215.0;
    /// Default resting heart rate
    pub const DEFAULT_RESTING_HR: u16 = 60;
    /// Default maximum heart rate
    pub const DEFAULT_MAX_HR: u16 = 190;
}

/// API tier request limits
pub mod api_tier_limits {
    /// Trial requests per month
    pub const TRIAL_REQUESTS_PER_MONTH: u32 = 100;
    /// Starter requests per month
    pub const STARTER_REQUESTS_PER_MONTH: u32 = 1000;
    /// Pro requests per month
    pub const PRO_REQUESTS_PER_MONTH: u32 = 10000;
}

/// HTTP status codes
pub mod http_status {
    /// HTTP 200 OK (success range minimum)
    pub const SUCCESS_MIN: u16 = 200;
    /// HTTP 299 (success range maximum)
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
    /// HTTP 429 Too Many Requests
    pub const TOO_MANY_REQUESTS: u16 = 429;
    /// HTTP 500 Internal Server Error
    pub const INTERNAL_SERVER_ERROR: u16 = 500;
    /// HTTP 502 Bad Gateway
    pub const BAD_GATEWAY: u16 = 502;
    /// HTTP 503 Service Unavailable
    pub const SERVICE_UNAVAILABLE: u16 = 503;
}

/// System monitoring constants
pub mod system_monitoring {
    /// Bytes to MB divisor
    pub const BYTES_TO_MB_DIVISOR: u64 = 1_048_576;
    /// Bytes to GB divisor
    pub const BYTES_TO_GB_DIVISOR: u64 = 1_073_741_824;
    /// KB to MB divisor
    pub const KB_TO_MB_DIVISOR: u64 = 1024;
    /// Memory warning threshold percentage
    pub const MEMORY_WARNING_THRESHOLD: f64 = 80.0;
    /// Disk warning threshold percentage
    pub const DISK_WARNING_THRESHOLD: f64 = 85.0;
}

/// Rate limiting tier burst configurations
pub mod rate_limiting_bursts {
    /// Free tier burst limit
    pub const FREE_TIER_BURST: u32 = 100;
    /// Professional tier burst limit
    pub const PROFESSIONAL_BURST: u32 = 500;
    /// Enterprise tier burst limit
    pub const ENTERPRISE_BURST: u32 = 2000;
}

/// OAuth 2.0 rate limiting configurations
pub mod oauth_rate_limiting {
    /// Authorization endpoint rate limit (requests per minute)
    pub const AUTHORIZE_RPM: u32 = 60;
    /// Token endpoint rate limit (requests per minute)
    pub const TOKEN_RPM: u32 = 30;
    /// Registration endpoint rate limit (requests per minute)
    pub const REGISTER_RPM: u32 = 10;
    /// Rate limit window duration in seconds
    pub const WINDOW_SECS: u64 = 60;
    /// Rate limiter cleanup threshold
    pub const CLEANUP_THRESHOLD: usize = 1000;
    /// Stale entry timeout in seconds
    pub const STALE_ENTRY_TIMEOUT_SECS: u64 = 120;
    /// Default retry-after header value in seconds
    pub const DEFAULT_RETRY_AFTER_SECS: u64 = 60;
}

/// Cache configuration constants
pub mod cache_config {
    /// Default cache capacity for LRU cache
    pub const DEFAULT_CAPACITY: usize = 1000;
    /// Rate limiter cleanup threshold (reused for cache cleanup)
    pub const CLEANUP_THRESHOLD: usize = 1000;
}

/// MCP transport configuration
pub mod mcp_transport {
    /// Notification broadcast channel size
    pub const NOTIFICATION_CHANNEL_SIZE: usize = 100;
    /// Maximum allowed MCP request body size in bytes (5 MB)
    pub const MAX_REQUEST_BODY_BYTES: usize = 5 * 1024 * 1024;
}

/// Rate limit header constants
pub mod rate_limit_headers {
    /// Rate limit window in seconds for HTTP headers (30 days)
    pub const WINDOW_SECS: &str = "2592000";
}

/// Sleep analysis and recovery constants
pub mod sleep_recovery {
    /// Number of recent activities to fetch for sleep/recovery analysis
    pub const ACTIVITY_LIMIT: u32 = 42;
    /// Minimum number of days of sleep history required for trend analysis
    pub const TREND_MIN_DAYS: usize = 7;
    /// Sleep trend improving threshold (hours increase over previous period)
    pub const TREND_IMPROVING_THRESHOLD: f64 = 5.0;
    /// Sleep trend declining threshold (hours decrease below previous period)
    pub const TREND_DECLINING_THRESHOLD: f64 = 5.0;
    /// Additional sleep hours recommended when athlete is fatigued (TSB negative)
    pub const FATIGUE_BONUS_HOURS: f64 = 0.5;
    /// ATL (Acute Training Load) threshold indicating high training load
    pub const HIGH_LOAD_ATL_THRESHOLD: f64 = 100.0;
    /// Additional sleep hours recommended during high training load periods
    pub const HIGH_LOAD_BONUS_HOURS: f64 = 0.25;
    /// Buffer time in minutes before target sleep time for wind-down routine
    pub const WIND_DOWN_MINUTES: i64 = 15;
    /// Minutes per day constant for time calculations and day wrapping
    pub const MINUTES_PER_DAY: i64 = 1440;
}

/// Goal management and feasibility constants
pub mod goal_management {
    /// Minimum number of activities required to establish training history
    pub const MIN_ACTIVITIES_FOR_TRAINING_HISTORY: usize = 2;
    /// Activities per week threshold for advanced fitness level classification
    pub const ADVANCED_FITNESS_ACTIVITIES_PER_WEEK: f64 = 5.0;
    /// Minimum training weeks required for advanced fitness level
    pub const ADVANCED_FITNESS_MIN_WEEKS: f64 = 26.0;
    /// Activities per week threshold for intermediate fitness level classification
    pub const INTERMEDIATE_FITNESS_ACTIVITIES_PER_WEEK: f64 = 3.0;
    /// Minimum training weeks required for intermediate fitness level
    pub const INTERMEDIATE_FITNESS_MIN_WEEKS: f64 = 12.0;
    /// Default training time availability per week (hours)
    pub const DEFAULT_TIME_AVAILABILITY_HOURS: f64 = 3.0;
    /// Default preferred activity duration (minutes)
    pub const DEFAULT_PREFERRED_DURATION_MINUTES: u32 = 30;
    /// Average days per month for monthly calculations (365.25/12)
    pub const DAYS_PER_MONTH_AVERAGE: f64 = 30.44;
}
