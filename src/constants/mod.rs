// ABOUTME: Constants module with domain-separated organization
// ABOUTME: Replaces the 933-line dumping ground with organized domain modules

//! Constants module
//!
//! This module organizes application constants by domain for better maintainability.
//! Constants are grouped into logical domains rather than being in a single large file.

use std::env;

// Domain-specific modules
pub mod errors;
pub mod oauth;
pub mod protocol;
pub mod protocols;
pub mod tools;

// Re-export commonly used items for easier access
pub use errors::*;
pub use oauth::*;
pub use protocol::*;
pub use protocols::*;
pub use tools::*;

// Alias for backward compatibility during transition
pub mod oauth_providers {
    pub use super::oauth::*;
}

// Remaining constants organized by domain

/// Environment-based configuration
pub mod env_config {
    use super::env;

    /// Get HTTP server port from environment or default
    #[must_use]
    pub fn http_port() -> u16 {
        env::var("HTTP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8080)
    }

    /// Get MCP port from environment or default
    #[must_use]
    pub fn mcp_port() -> u16 {
        env::var("MCP_PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8081)
    }

    /// Get base URL from environment or default
    #[must_use]
    pub fn base_url() -> String {
        env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string())
    }

    /// Get Strava redirect URI from environment or default
    #[must_use]
    pub fn strava_redirect_uri() -> String {
        env::var("STRAVA_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{}/auth/strava/callback", base_url()))
    }

    /// Get Fitbit redirect URI from environment or default
    #[must_use]
    pub fn fitbit_redirect_uri() -> String {
        env::var("FITBIT_REDIRECT_URI")
            .unwrap_or_else(|_| format!("{}/auth/fitbit/callback", base_url()))
    }

    /// Get Strava auth URL from environment or default
    #[must_use]
    pub fn strava_auth_url() -> String {
        env::var("STRAVA_AUTH_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/authorize".to_string())
    }

    /// Get Strava token URL from environment or default
    #[must_use]
    pub fn strava_token_url() -> String {
        env::var("STRAVA_TOKEN_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/token".to_string())
    }

    /// Get Strava deauthorize URL from environment or default
    #[must_use]
    pub fn strava_deauthorize_url() -> String {
        env::var("STRAVA_DEAUTH_URL")
            .unwrap_or_else(|_| "https://www.strava.com/oauth/deauthorize".to_string())
    }

    /// Get Fitbit auth URL from environment or default
    #[must_use]
    pub fn fitbit_auth_url() -> String {
        env::var("FITBIT_AUTH_URL")
            .unwrap_or_else(|_| "https://www.fitbit.com/oauth2/authorize".to_string())
    }

    /// Get Fitbit token URL from environment or default
    #[must_use]
    pub fn fitbit_token_url() -> String {
        env::var("FITBIT_TOKEN_URL")
            .unwrap_or_else(|_| "https://api.fitbit.com/oauth2/token".to_string())
    }

    /// Get Fitbit revoke URL from environment or default
    #[must_use]
    pub fn fitbit_revoke_url() -> String {
        env::var("FITBIT_REVOKE_URL")
            .unwrap_or_else(|_| "https://api.fitbit.com/oauth2/revoke".to_string())
    }

    /// Get log level from environment or default
    #[must_use]
    pub fn log_level() -> String {
        env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string())
    }

    /// Get database URL from environment or default
    #[must_use]
    pub fn database_url() -> String {
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string())
    }

    /// Get encryption key path from environment or default
    #[must_use]
    pub fn encryption_key_path() -> String {
        env::var("ENCRYPTION_KEY_PATH").unwrap_or_else(|_| "./encryption.key".to_string())
    }

    /// Get JWT secret path from environment or default
    #[must_use]
    pub fn jwt_secret_path() -> String {
        env::var("JWT_SECRET_PATH").unwrap_or_else(|_| "./jwt.secret".to_string())
    }

    /// Get JWT expiry hours from environment or default
    #[must_use]
    pub fn jwt_expiry_hours() -> i64 {
        env::var("JWT_EXPIRY_HOURS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(24)
    }

    /// Get `OpenWeather` API base URL from environment or default
    #[must_use]
    pub fn openweather_api_base() -> String {
        env::var("OPENWEATHER_API_BASE")
            .unwrap_or_else(|_| "https://api.openweathermap.org/data/2.5".to_string())
    }

    /// Get `Strava` API base URL from environment or default
    #[must_use]
    pub fn strava_api_base() -> String {
        env::var("STRAVA_API_BASE").unwrap_or_else(|_| "https://www.strava.com/api/v3".to_string())
    }
}

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
    pub const DEFAULT_HTTP_PORT: u16 = 8080;
    /// Default MCP port
    pub const DEFAULT_MCP_PORT: u16 = 8081;
    /// Default docs port
    pub const DEFAULT_DOCS_PORT: u16 = 8082;
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
    /// JWT expiry hours
    pub const JWT_EXPIRY_HOURS: i64 = 24;
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
    /// Default HTTP client timeout in seconds
    pub const HTTP_CLIENT_TIMEOUT_SECS: u64 = 30;
    /// Database connection timeout in seconds
    pub const DATABASE_TIMEOUT_SECS: u64 = 10;
    /// OAuth callback timeout in seconds
    pub const OAUTH_CALLBACK_TIMEOUT_SECS: u64 = 300; // 5 minutes
}

/// Cryptographic constants
pub mod crypto {
    /// JWT algorithm
    pub const JWT_ALGORITHM: &str = "HS256";
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

/// System configuration
pub mod system_config {
    /// Maximum concurrent requests
    pub const MAX_CONCURRENT_REQUESTS: usize = 100;
    /// Worker thread count
    pub const WORKER_THREADS: usize = 4;
    /// Professional monthly limit
    pub const PROFESSIONAL_MONTHLY_LIMIT: u32 = 100_000;
    /// Starter monthly limit
    pub const STARTER_MONTHLY_LIMIT: u32 = 10_000;
    /// Trial monthly limit
    pub const TRIAL_MONTHLY_LIMIT: u32 = 1_000;
    /// Trial period days
    pub const TRIAL_PERIOD_DAYS: u32 = 30;
    /// Rate limit window seconds
    pub const RATE_LIMIT_WINDOW_SECONDS: u32 = 3600;
}

/// Time constants
pub mod time_constants {
    /// Seconds in a minute
    pub const SECONDS_PER_MINUTE: u64 = 60;
    /// Seconds in an hour
    pub const SECONDS_PER_HOUR: u32 = 3600;
    /// Seconds in an hour as f64
    pub const SECONDS_PER_HOUR_F64: f64 = 3600.0;
    /// Seconds in a day
    pub const SECONDS_PER_DAY: u32 = 86_400;
    /// Seconds in a week
    pub const SECONDS_PER_WEEK: u32 = 604_800;
    /// Seconds in a month (30 days)
    pub const SECONDS_PER_MONTH: u32 = 2_592_000;
    /// Seconds in a year (365 days)
    pub const SECONDS_PER_YEAR: u64 = 31_536_000;
    /// Minutes in an hour
    pub const MINUTES_PER_HOUR: u64 = 60;
    /// Hours in a day
    pub const HOURS_PER_DAY: u64 = 24;
    /// Days in a week
    pub const DAYS_PER_WEEK: u64 = 7;
}

/// Time module for backward compatibility
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
    /// `TCP` keep alive timeout in seconds
    pub const TCP_KEEP_ALIVE_SECS: u64 = 60;
    /// `TCP` no delay
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
}

/// User defaults
pub mod user_defaults {
    /// Default timezone
    pub const DEFAULT_TIMEZONE: &str = "UTC";
    /// Default user age
    pub const DEFAULT_USER_AGE: i32 = 30;
    /// Default goal distance in meters
    pub const DEFAULT_GOAL_DISTANCE: f64 = 5000.0;
    /// Default locale
    pub const DEFAULT_LOCALE: &str = "en_US";
    /// Default measurement system
    pub const DEFAULT_MEASUREMENT_SYSTEM: &str = "metric";
}

/// API provider limits
pub mod api_provider_limits {
    /// Strava-specific limits
    pub mod strava {
        /// Default activities per page
        pub const DEFAULT_ACTIVITIES_PER_PAGE: usize = 30;
        /// Maximum activities per request
        pub const MAX_ACTIVITIES_PER_REQUEST: usize = 200;
        /// Rate limit requests per 15 minutes
        pub const RATE_LIMIT_15MIN: u32 = 100;
        /// Rate limit requests per day
        pub const RATE_LIMIT_DAILY: u32 = 1000;
    }

    /// Fitbit-specific limits
    pub mod fitbit {
        /// Default activities per page
        pub const DEFAULT_ACTIVITIES_PER_PAGE: usize = 20;
        /// Maximum activities per request
        pub const MAX_ACTIVITIES_PER_REQUEST: usize = 100;
        /// Rate limit requests per hour
        pub const RATE_LIMIT_HOURLY: u32 = 150;
        /// Rate limit requests per day
        pub const RATE_LIMIT_DAILY: u32 = 1000;
    }

    /// General provider limits
    pub const STRAVA_RATE_LIMIT_15MIN: u32 = 100;
    pub const STRAVA_RATE_LIMIT_DAILY: u32 = 1000;
    pub const FITBIT_RATE_LIMIT_HOURLY: u32 = 150;
    pub const FITBIT_RATE_LIMIT_DAILY: u32 = 1000;
}

/// Service tiers
pub mod tiers {
    /// Trial tier
    pub const TRIAL: &str = "trial";
    /// Free tier
    pub const FREE: &str = "free";
    /// Starter tier
    pub const STARTER: &str = "starter";
    /// Pro tier
    pub const PRO: &str = "pro";
    /// Professional tier
    pub const PROFESSIONAL: &str = "professional";
    /// Enterprise tier
    pub const ENTERPRISE: &str = "enterprise";
    /// ENT tier (enterprise abbreviation)
    pub const ENT: &str = "ent";
}

/// Key prefixes
pub mod key_prefixes {
    /// `API` key prefix
    pub const API_KEY: &str = "pk_";
    /// Live `API` key prefix
    pub const API_KEY_LIVE: &str = "pk_live_";
    /// Secret key prefix
    pub const SECRET_KEY: &str = "sk_";
    /// Test key prefix
    pub const TEST_KEY: &str = "tk_";
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
    /// Pierre MCP Server service name
    pub const PIERRE_MCP_SERVER: &str = "pierre_mcp_server";
    /// Admin `API` service name
    pub const ADMIN_API: &str = "admin_api";
    /// Pierre `MCP` Admin `API` service name
    pub const PIERRE_MCP_ADMIN_API: &str = "pierre_mcp_admin_api";
}

/// Error messages
pub mod error_messages {
    /// Invalid credentials
    pub const INVALID_CREDENTIALS: &str = "Invalid credentials provided";
    /// Account not found
    pub const ACCOUNT_NOT_FOUND: &str = "Account not found";
    /// Insufficient permissions
    pub const INSUFFICIENT_PERMISSIONS: &str = "Insufficient permissions";
    /// Rate limit exceeded
    pub const RATE_LIMIT_EXCEEDED: &str = "Rate limit exceeded";
    /// Service unavailable
    pub const SERVICE_UNAVAILABLE: &str = "Service temporarily unavailable";
    /// Invalid email format
    pub const INVALID_EMAIL_FORMAT: &str = "Invalid email format";
    /// Password too weak
    pub const PASSWORD_TOO_WEAK: &str = "Password does not meet requirements";
    /// User already exists
    pub const USER_ALREADY_EXISTS: &str = "User with this email already exists";
}

/// HTTP status codes
pub mod http_status {
    /// Success status range minimum
    pub const SUCCESS_MIN: u16 = 200;
    /// Success status range maximum
    pub const SUCCESS_MAX: u16 = 299;
    /// Continue
    pub const CONTINUE: u16 = 100;
    /// Switching protocols
    pub const SWITCHING_PROTOCOLS: u16 = 101;
    /// OK
    pub const OK: u16 = 200;
    /// Created
    pub const CREATED: u16 = 201;
    /// Accepted
    pub const ACCEPTED: u16 = 202;
    /// No content
    pub const NO_CONTENT: u16 = 204;
    /// Moved permanently
    pub const MOVED_PERMANENTLY: u16 = 301;
    /// Found
    pub const FOUND: u16 = 302;
    /// Not modified
    pub const NOT_MODIFIED: u16 = 304;
    /// Bad request
    pub const BAD_REQUEST: u16 = 400;
    /// Unauthorized
    pub const UNAUTHORIZED: u16 = 401;
    /// Forbidden
    pub const FORBIDDEN: u16 = 403;
    /// Not found
    pub const NOT_FOUND: u16 = 404;
    /// Method not allowed
    pub const METHOD_NOT_ALLOWED: u16 = 405;
    /// Conflict
    pub const CONFLICT: u16 = 409;
    /// Unprocessable entity
    pub const UNPROCESSABLE_ENTITY: u16 = 422;
    /// Too many requests
    pub const TOO_MANY_REQUESTS: u16 = 429;
    /// Internal server error
    pub const INTERNAL_SERVER_ERROR: u16 = 500;
    /// Not implemented
    pub const NOT_IMPLEMENTED: u16 = 501;
    /// Bad gateway
    pub const BAD_GATEWAY: u16 = 502;
    /// Service unavailable
    pub const SERVICE_UNAVAILABLE: u16 = 503;
    /// Gateway timeout
    pub const GATEWAY_TIMEOUT: u16 = 504;
}

/// Rate limiting configuration
pub mod rate_limits {
    /// Default burst limit for rate limiting
    pub const DEFAULT_BURST_LIMIT: u32 = 10;
    /// Default rate limit per minute
    pub const DEFAULT_PER_MINUTE: u32 = 60;
    /// Default rate limit per hour
    pub const DEFAULT_PER_HOUR: u32 = 1000;
    /// Default rate limit per day
    pub const DEFAULT_PER_DAY: u32 = 10_000;
    /// Strava default daily rate limit
    pub const STRAVA_DEFAULT_DAILY_RATE_LIMIT: u32 = 1000;
    /// Fitbit default daily rate limit
    pub const FITBIT_DEFAULT_DAILY_RATE_LIMIT: u32 = 1000;
    /// WebSocket channel capacity
    pub const WEBSOCKET_CHANNEL_CAPACITY: usize = 100;
}

/// Logging configuration
pub mod logging {
    /// Default log level
    pub const DEFAULT_LEVEL: &str = "info";
}

/// System monitoring
pub mod system_monitoring {
    /// Health check interval in seconds
    pub const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;
    /// Metrics collection interval in seconds
    pub const METRICS_INTERVAL_SECS: u64 = 60;
    /// Memory usage warning threshold percentage
    pub const MEMORY_WARNING_THRESHOLD: f64 = 85.0;
    /// CPU usage warning threshold percentage
    pub const CPU_WARNING_THRESHOLD: f64 = 90.0;
    /// Bytes to MB divisor
    pub const BYTES_TO_MB_DIVISOR: u64 = 1_048_576;
    /// Log rotation interval in hours
    pub const LOG_ROTATION_HOURS: u64 = 24;
    /// Disk space warning threshold percentage
    pub const DISK_SPACE_WARNING_THRESHOLD: f64 = 80.0;
}

/// Physiological constants for fitness analysis
pub mod physiology {
    /// Maximum heart rate calculation constant
    pub const MAX_HR_CONSTANT: f64 = 220.0;
    /// Resting heart rate average
    pub const AVERAGE_RESTING_HR: f64 = 60.0;
    /// Default resting heart rate
    pub const DEFAULT_RESTING_HR: u16 = 60;
    /// Default maximum heart rate
    pub const DEFAULT_MAX_HR: u16 = 180;
    /// Maximum VO2 for elite athletes
    pub const ELITE_VO2_MAX: f64 = 80.0;
    /// Average VO2 max for adults
    pub const AVERAGE_VO2_MAX: f64 = 35.0;
    /// Calories per gram of fat
    pub const CALORIES_PER_GRAM_FAT: f64 = 9.0;
    /// Calories per gram of carbohydrate
    pub const CALORIES_PER_GRAM_CARB: f64 = 4.0;
    /// Minimum good ground contact time in milliseconds
    pub const MIN_GOOD_GCT_MS: f64 = 150.0;
    /// Maximum good ground contact time in milliseconds
    pub const MAX_GOOD_GCT_MS: f64 = 300.0;
    /// Calories per gram of protein
    pub const CALORIES_PER_GRAM_PROTEIN: f64 = 4.0;
    /// Optimal cadence for running (steps per minute)
    pub const OPTIMAL_RUNNING_CADENCE: f64 = 180.0;
    /// Optimal cadence for cycling (RPM)
    pub const OPTIMAL_CYCLING_CADENCE: f64 = 90.0;
    /// Optimal ground contact time in milliseconds
    pub const OPTIMAL_GCT_MS: f64 = 250.0;
}

/// A2A Agent card example data constants
pub mod a2a_examples {
    /// Example activity ID for A2A documentation
    pub const EXAMPLE_ACTIVITY_ID: &str = "123456";
    /// Example activity date for A2A documentation (ISO 8601)
    pub const EXAMPLE_ACTIVITY_DATE: &str = "2024-01-15T07:00:00Z";
    /// Example activity duration for A2A documentation (30 minutes in seconds)
    pub const EXAMPLE_ACTIVITY_DURATION: u64 = 1800;
    /// Example activity duration for A2A documentation (seconds)
    pub const EXAMPLE_ACTIVITY_DURATION_SECONDS: u64 = 1800;
    /// Example activity distance for A2A documentation (5 km in meters)
    pub const EXAMPLE_ACTIVITY_DISTANCE: f64 = 5000.0;
    /// Example activity distance for A2A documentation (meters)
    pub const EXAMPLE_ACTIVITY_DISTANCE_METERS: f64 = 5000.0;
    /// Example activity average speed for A2A documentation (m/s)
    pub const EXAMPLE_ACTIVITY_AVG_SPEED: f64 = 2.78; // 10 km/h
    /// Example activity calories for A2A documentation
    pub const EXAMPLE_ACTIVITY_CALORIES: u32 = 300;
    /// Trial requests per month for documentation
    pub const TRIAL_REQUESTS_PER_MONTH: u32 = 100;
    /// Starter requests per month for documentation
    pub const STARTER_REQUESTS_PER_MONTH: u32 = 500;
}

/// `OAuth2` client constants
pub mod oauth2_client {
    /// Default `OAuth2` timeout in seconds
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
    /// Authorization code flow
    pub const FLOW_AUTHORIZATION_CODE: &str = "authorization_code";
    /// Client credentials flow
    pub const FLOW_CLIENT_CREDENTIALS: &str = "client_credentials";
}

/// Tenant management constants
pub mod tenant {
    /// Default tenant isolation level
    pub const DEFAULT_ISOLATION_LEVEL: &str = "strict";
    /// Maximum tenants per instance
    pub const MAX_TENANTS_PER_INSTANCE: usize = 1000;
}
