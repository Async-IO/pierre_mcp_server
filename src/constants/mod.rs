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
pub use tools::*;
// Note: protocol and protocols are kept as modules to avoid conflicts

// Alias for backward compatibility during transition
pub mod oauth_providers {
    pub use super::oauth::*;
}

// Remaining constants organized by domain

/// Environment-based configuration
pub mod env_config {
    use super::env;

    /// Get unified server port from environment or default
    /// Checks `HTTP_PORT` first (preferred), then `MCP_PORT` for backwards compatibility
    #[must_use]
    pub fn server_port() -> u16 {
        env::var("HTTP_PORT")
            .or_else(|_| env::var("MCP_PORT"))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8081)
    }

    /// Get HTTP server port (alias for `server_port` for backwards compatibility)
    #[must_use]
    #[deprecated(
        since = "0.1.0",
        note = "Use server_port() instead - server is unified"
    )]
    pub fn http_port() -> u16 {
        server_port()
    }

    /// Get MCP port (alias for `server_port` for backwards compatibility)
    #[must_use]
    #[deprecated(
        since = "0.1.0",
        note = "Use server_port() instead - server is unified"
    )]
    pub fn mcp_port() -> u16 {
        server_port()
    }

    /// Get base URL from environment or default
    #[must_use]
    pub fn base_url() -> String {
        env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
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
    /// API key live prefix (legacy name)
    pub const API_KEY_LIVE: &str = "pk_live_";
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
    pub const DAYS_PER_WEEK: u64 = 7;
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
