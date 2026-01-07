// ABOUTME: Network configuration types for HTTP clients, SSE, CORS, and TLS
// ABOUTME: Handles timeouts, connection settings, and security transport options
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::constants::{network_config, timeouts};
use crate::errors::{AppError, AppResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// HTTP client timeout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpClientConfig {
    /// Shared HTTP client request timeout in seconds
    pub shared_client_timeout_secs: u64,
    /// Shared HTTP client connect timeout in seconds
    pub shared_client_connect_timeout_secs: u64,
    /// OAuth client request timeout in seconds
    pub oauth_client_timeout_secs: u64,
    /// OAuth client connect timeout in seconds
    pub oauth_client_connect_timeout_secs: u64,
    /// API client request timeout in seconds
    pub api_client_timeout_secs: u64,
    /// API client connect timeout in seconds
    pub api_client_connect_timeout_secs: u64,
    /// Health check client timeout in seconds
    pub health_check_timeout_secs: u64,
    /// OAuth callback notification timeout in seconds
    pub oauth_callback_notification_timeout_secs: u64,
    /// Enable exponential backoff retries with jitter
    pub enable_retries: bool,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay for exponential backoff in milliseconds
    pub retry_base_delay_ms: u64,
    /// Maximum delay cap for retries in milliseconds
    pub retry_max_delay_ms: u64,
    /// Enable jitter to prevent thundering herd problem
    pub retry_jitter_enabled: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            shared_client_timeout_secs: timeouts::HTTP_CLIENT_TIMEOUT_SECS,
            shared_client_connect_timeout_secs: timeouts::HTTP_CLIENT_CONNECT_TIMEOUT_SECS,
            oauth_client_timeout_secs: timeouts::OAUTH_CLIENT_TIMEOUT_SECS,
            oauth_client_connect_timeout_secs: timeouts::OAUTH_CLIENT_CONNECT_TIMEOUT_SECS,
            api_client_timeout_secs: timeouts::API_CLIENT_TIMEOUT_SECS,
            api_client_connect_timeout_secs: timeouts::API_CLIENT_CONNECT_TIMEOUT_SECS,
            health_check_timeout_secs: timeouts::HEALTH_CHECK_TIMEOUT_SECS,
            oauth_callback_notification_timeout_secs:
                timeouts::OAUTH_CALLBACK_NOTIFICATION_TIMEOUT_SECS,
            enable_retries: true,
            max_retries: 3,
            retry_base_delay_ms: 100,
            retry_max_delay_ms: 5000,
            retry_jitter_enabled: true,
        }
    }
}

impl HttpClientConfig {
    /// Load HTTP client configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            shared_client_timeout_secs: env::var("HTTP_CLIENT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            shared_client_connect_timeout_secs: env::var("HTTP_CLIENT_CONNECT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            oauth_client_timeout_secs: env::var("OAUTH_CLIENT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(15),
            oauth_client_connect_timeout_secs: env::var("OAUTH_CLIENT_CONNECT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            api_client_timeout_secs: env::var("API_CLIENT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            api_client_connect_timeout_secs: env::var("API_CLIENT_CONNECT_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            health_check_timeout_secs: env::var("HEALTH_CHECK_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            oauth_callback_notification_timeout_secs: env::var(
                "OAUTH_CALLBACK_NOTIFICATION_TIMEOUT_SECS",
            )
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5),
            enable_retries: env_var_or("HTTP_CLIENT_ENABLE_RETRIES", "true")
                .parse()
                .unwrap_or(true),
            max_retries: env_var_or("HTTP_CLIENT_MAX_RETRIES", "3")
                .parse()
                .unwrap_or(3),
            retry_base_delay_ms: env_var_or("HTTP_CLIENT_RETRY_BASE_DELAY_MS", "100")
                .parse()
                .unwrap_or(100),
            retry_max_delay_ms: env_var_or("HTTP_CLIENT_RETRY_MAX_DELAY_MS", "5000")
                .parse()
                .unwrap_or(5000),
            retry_jitter_enabled: env_var_or("HTTP_CLIENT_RETRY_JITTER", "true")
                .parse()
                .unwrap_or(true),
        }
    }
}

/// SSE connection management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseConfig {
    /// Cleanup task interval in seconds
    pub cleanup_interval_secs: u64,
    /// Connection timeout in seconds (connections inactive for this duration will be removed)
    pub connection_timeout_secs: u64,
    /// OAuth session cookie Max-Age in seconds
    pub session_cookie_max_age_secs: u64,
    /// Enable Secure flag on cookies (requires HTTPS)
    pub session_cookie_secure: bool,
    /// Maximum buffer size for SSE event queue per connection
    pub max_buffer_size: usize,
    /// Behavior when buffer is full
    pub buffer_overflow_strategy: SseBufferStrategy,
    /// Broadcast channel size for SSE events
    pub broadcast_channel_size: usize,
    /// Maximum SSE connections per user
    pub max_connections_per_user: usize,
}

impl Default for SseConfig {
    fn default() -> Self {
        Self {
            cleanup_interval_secs: timeouts::SSE_CLEANUP_INTERVAL_SECS,
            connection_timeout_secs: timeouts::SSE_CONNECTION_TIMEOUT_SECS,
            session_cookie_max_age_secs: timeouts::SESSION_COOKIE_MAX_AGE_SECS,
            session_cookie_secure: false, // Default to false for development, override in production
            max_buffer_size: 1000,
            buffer_overflow_strategy: SseBufferStrategy::default(),
            broadcast_channel_size: network_config::SSE_BROADCAST_CHANNEL_SIZE,
            max_connections_per_user: network_config::SSE_MAX_CONNECTIONS_PER_USER,
        }
    }
}

impl SseConfig {
    /// Load SSE configuration from environment
    ///
    /// # Errors
    ///
    /// Returns an error if SSE environment variables are invalid
    pub fn from_env() -> AppResult<Self> {
        let strategy_str = env_var_or("SSE_BUFFER_OVERFLOW_STRATEGY", "drop_oldest");
        let buffer_overflow_strategy = match strategy_str.as_str() {
            "drop_new" => SseBufferStrategy::DropNew,
            "close_connection" => SseBufferStrategy::CloseConnection,
            _ => SseBufferStrategy::DropOldest, // Default fallback (including "drop_oldest")
        };

        Ok(Self {
            cleanup_interval_secs: env_var_or(
                "SSE_CLEANUP_INTERVAL_SECS",
                &timeouts::SSE_CLEANUP_INTERVAL_SECS.to_string(),
            )
            .parse()
            .map_err(|e| {
                AppError::invalid_input(format!("Invalid SSE_CLEANUP_INTERVAL_SECS value: {e}"))
            })?,
            connection_timeout_secs: env_var_or(
                "SSE_CONNECTION_TIMEOUT_SECS",
                &timeouts::SSE_CONNECTION_TIMEOUT_SECS.to_string(),
            )
            .parse()
            .map_err(|e| {
                AppError::invalid_input(format!("Invalid SSE_CONNECTION_TIMEOUT_SECS value: {e}"))
            })?,
            session_cookie_max_age_secs: env_var_or(
                "SESSION_COOKIE_MAX_AGE_SECS",
                &timeouts::SESSION_COOKIE_MAX_AGE_SECS.to_string(),
            )
            .parse()
            .map_err(|e| {
                AppError::invalid_input(format!("Invalid SESSION_COOKIE_MAX_AGE_SECS value: {e}"))
            })?,
            session_cookie_secure: env_var_or("SESSION_COOKIE_SECURE", "false")
                .parse()
                .map_err(|e| {
                    AppError::invalid_input(format!("Invalid SESSION_COOKIE_SECURE value: {e}"))
                })?,
            max_buffer_size: env_var_or("SSE_MAX_BUFFER_SIZE", "1000")
                .parse()
                .map_err(|e| {
                    AppError::invalid_input(format!("Invalid SSE_MAX_BUFFER_SIZE value: {e}"))
                })?,
            buffer_overflow_strategy,
            broadcast_channel_size: env_var_or(
                "SSE_BROADCAST_CHANNEL_SIZE",
                &network_config::SSE_BROADCAST_CHANNEL_SIZE.to_string(),
            )
            .parse()
            .map_err(|e| {
                AppError::invalid_input(format!("Invalid SSE_BROADCAST_CHANNEL_SIZE value: {e}"))
            })?,
            max_connections_per_user: env_var_or(
                "SSE_MAX_CONNECTIONS_PER_USER",
                &network_config::SSE_MAX_CONNECTIONS_PER_USER.to_string(),
            )
            .parse()
            .map_err(|e| {
                AppError::invalid_input(format!("Invalid SSE_MAX_CONNECTIONS_PER_USER value: {e}"))
            })?,
        })
    }
}

/// Strategy for handling SSE buffer overflow
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SseBufferStrategy {
    /// Drop oldest event when buffer is full
    #[default]
    DropOldest,
    /// Drop new event when buffer is full
    DropNew,
    /// Close SSE connection when buffer is full
    CloseConnection,
}

/// CORS (Cross-Origin Resource Sharing) configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CorsConfig {
    /// Comma-separated list of allowed origins
    pub allowed_origins: String,
    /// Allow localhost in development mode
    pub allow_localhost_dev: bool,
}

impl CorsConfig {
    /// Load CORS configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            allowed_origins: env::var("CORS_ALLOWED_ORIGINS").unwrap_or_default(),
            allow_localhost_dev: env::var("CORS_ALLOW_LOCALHOST_DEV")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
        }
    }
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Enable TLS
    pub enabled: bool,
    /// Path to TLS certificate
    pub cert_path: Option<PathBuf>,
    /// Path to TLS private key
    pub key_path: Option<PathBuf>,
}

impl TlsConfig {
    /// Load TLS configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            enabled: env_var_or("TLS_ENABLED", "false").parse().unwrap_or(false),
            cert_path: env::var("TLS_CERT_PATH").ok().map(PathBuf::from),
            key_path: env::var("TLS_KEY_PATH").ok().map(PathBuf::from),
        }
    }
}

/// Per-route timeout configuration for database, API, and SSE operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTimeoutConfig {
    /// Database operation timeout in seconds
    pub database_timeout_secs: u64,
    /// Provider API call timeout in seconds
    pub provider_api_timeout_secs: u64,
    /// SSE event send timeout in seconds
    pub sse_event_timeout_secs: u64,
    /// OAuth token operations timeout in seconds
    pub oauth_operation_timeout_secs: u64,
    /// Default timeout for general operations in seconds
    pub default_timeout_secs: u64,
    /// Upload operation timeout in seconds (longer for large files)
    pub upload_timeout_secs: u64,
    /// Long polling operation timeout in seconds
    pub long_polling_timeout_secs: u64,
    /// MCP sampling operation timeout in seconds
    pub mcp_sampling_timeout_secs: u64,
    /// Geocoding/location lookup timeout in seconds
    pub geocoding_timeout_secs: u64,
}

impl Default for RouteTimeoutConfig {
    fn default() -> Self {
        Self {
            database_timeout_secs: 30,
            provider_api_timeout_secs: 60,
            sse_event_timeout_secs: 5,
            oauth_operation_timeout_secs: 15,
            default_timeout_secs: 30,
            upload_timeout_secs: 300,
            long_polling_timeout_secs: 300,
            mcp_sampling_timeout_secs: 30,
            geocoding_timeout_secs: 10,
        }
    }
}

impl RouteTimeoutConfig {
    /// Load per-route timeout configuration from environment
    #[must_use]
    pub fn from_env() -> Self {
        Self {
            database_timeout_secs: env_var_or("ROUTE_TIMEOUT_DATABASE_SECS", "30")
                .parse()
                .unwrap_or(30),
            provider_api_timeout_secs: env_var_or("ROUTE_TIMEOUT_PROVIDER_API_SECS", "60")
                .parse()
                .unwrap_or(60),
            sse_event_timeout_secs: env_var_or("ROUTE_TIMEOUT_SSE_EVENT_SECS", "5")
                .parse()
                .unwrap_or(5),
            oauth_operation_timeout_secs: env_var_or("ROUTE_TIMEOUT_OAUTH_SECS", "15")
                .parse()
                .unwrap_or(15),
            default_timeout_secs: env_var_or("ROUTE_TIMEOUT_DEFAULT_SECS", "30")
                .parse()
                .unwrap_or(30),
            upload_timeout_secs: env_var_or("ROUTE_TIMEOUT_UPLOAD_SECS", "300")
                .parse()
                .unwrap_or(300),
            long_polling_timeout_secs: env_var_or("ROUTE_TIMEOUT_LONG_POLLING_SECS", "300")
                .parse()
                .unwrap_or(300),
            mcp_sampling_timeout_secs: env_var_or("ROUTE_TIMEOUT_MCP_SAMPLING_SECS", "30")
                .parse()
                .unwrap_or(30),
            geocoding_timeout_secs: env_var_or("ROUTE_TIMEOUT_GEOCODING_SECS", "10")
                .parse()
                .unwrap_or(10),
        }
    }
}

/// Parse comma-separated CORS origins
#[must_use]
pub fn parse_origins(origins_str: &str) -> Vec<String> {
    if origins_str == "*" {
        vec!["*".into()]
    } else {
        origins_str
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect()
    }
}

/// Get environment variable or default value
fn env_var_or(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}
