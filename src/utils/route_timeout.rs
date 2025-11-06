// ABOUTME: Configurable timeout utilities for route handlers to prevent hanging operations
// ABOUTME: Provides timeout wrappers for database, API, SSE, and OAuth operations

// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::config::environment::RouteTimeoutConfig;
use crate::errors::AppError;
use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::time::timeout;

/// Global route timeout configuration
static ROUTE_TIMEOUT_CONFIG: OnceLock<RouteTimeoutConfig> = OnceLock::new();

/// Initialize route timeout configuration
///
/// Must be called once at server startup before any route handlers use timeouts.
///
/// # Panics
/// Panics if called more than once (configuration cannot be changed after initialization)
pub fn initialize_route_timeouts(config: RouteTimeoutConfig) {
    assert!(
        ROUTE_TIMEOUT_CONFIG.set(config).is_ok(),
        "Route timeout configuration already initialized"
    );
}

/// Get the current route timeout configuration with fallback to defaults
///
/// Returns defaults if route timeout configuration was not initialized at server startup
fn get_config() -> &'static RouteTimeoutConfig {
    static DEFAULT_CONFIG: OnceLock<RouteTimeoutConfig> = OnceLock::new();
    ROUTE_TIMEOUT_CONFIG
        .get()
        .unwrap_or_else(|| DEFAULT_CONFIG.get_or_init(RouteTimeoutConfig::default))
}

/// Execute a database operation with configured timeout
///
/// # Arguments
/// * `operation` - The async database operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
///
/// # Example
/// ```rust,ignore
/// use pierre_mcp_server::utils::route_timeout::with_database_timeout;
///
/// let user = with_database_timeout(async {
///     database.get_user_by_id(user_id).await
/// }).await?;
/// ```
pub async fn with_database_timeout<F, T, E>(operation: F) -> Result<T, anyhow::Error>
where
    F: Future<Output = Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.database_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Database operation timed out after {}s",
                config.database_timeout_secs
            ))
            .into())
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute a provider API operation with configured timeout
///
/// # Arguments
/// * `operation` - The async API operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
///
/// # Example
/// ```rust,ignore
/// use pierre_mcp_server::utils::route_timeout::with_provider_api_timeout;
///
/// let activities = with_provider_api_timeout(async {
///     provider.get_activities(limit, page).await
/// }).await?;
/// ```
pub async fn with_provider_api_timeout<F, T, E>(operation: F) -> Result<T, anyhow::Error>
where
    F: Future<Output = Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.provider_api_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Provider API operation timed out after {}s",
                config.provider_api_timeout_secs
            ))
            .into())
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute an SSE event operation with configured timeout
///
/// # Arguments
/// * `operation` - The async SSE operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
///
/// # Example
/// ```rust,ignore
/// use pierre_mcp_server::utils::route_timeout::with_sse_timeout;
///
/// with_sse_timeout(async {
///     sse_manager.send_event(user_id, event).await
/// }).await?;
/// ```
pub async fn with_sse_timeout<F, T, E>(operation: F) -> Result<T, anyhow::Error>
where
    F: Future<Output = Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.sse_event_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "SSE event operation timed out after {}s",
                config.sse_event_timeout_secs
            ))
            .into())
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute an OAuth operation with configured timeout
///
/// # Arguments
/// * `operation` - The async OAuth operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
///
/// # Example
/// ```rust,ignore
/// use pierre_mcp_server::utils::route_timeout::with_oauth_timeout;
///
/// let token = with_oauth_timeout(async {
///     auth_server.exchange_token(code, verifier).await
/// }).await?;
/// ```
pub async fn with_oauth_timeout<F, T, E>(operation: F) -> Result<T, anyhow::Error>
where
    F: Future<Output = Result<T, E>>,
    E: Into<anyhow::Error>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.oauth_operation_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "OAuth operation timed out after {}s",
                config.oauth_operation_timeout_secs
            ))
            .into())
        },
        |result| result.map_err(Into::into),
    )
}

/// Get database timeout duration for manual timeout handling
///
/// # Returns
/// Duration for database operations
#[must_use]
pub fn database_timeout_duration() -> Duration {
    Duration::from_secs(get_config().database_timeout_secs)
}

/// Get provider API timeout duration for manual timeout handling
///
/// # Returns
/// Duration for provider API operations
#[must_use]
pub fn provider_api_timeout_duration() -> Duration {
    Duration::from_secs(get_config().provider_api_timeout_secs)
}

/// Get SSE timeout duration for manual timeout handling
///
/// # Returns
/// Duration for SSE operations
#[must_use]
pub fn sse_timeout_duration() -> Duration {
    Duration::from_secs(get_config().sse_event_timeout_secs)
}

/// Get OAuth timeout duration for manual timeout handling
///
/// # Returns
/// Duration for OAuth operations
#[must_use]
pub fn oauth_timeout_duration() -> Duration {
    Duration::from_secs(get_config().oauth_operation_timeout_secs)
}
