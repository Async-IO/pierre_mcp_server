// ABOUTME: Configurable timeout utilities for route handlers to prevent hanging operations
// ABOUTME: Provides timeout wrappers for database, API, SSE, and OAuth operations

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::config::environment::RouteTimeoutConfig;
use crate::errors::{AppError, AppResult};
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
/// ```rust,no_run
/// use pierre_mcp_server::utils::route_timeout::with_database_timeout;
///
/// # async fn example() -> pierre_mcp_server::errors::AppResult<()> {
/// # struct Database; impl Database { async fn get_user_by_id(&self, _: &str) -> pierre_mcp_server::errors::AppResult<String> { Ok(String::new()) } }
/// # let database = Database; let user_id = "test";
/// let user = with_database_timeout(async {
///     database.get_user_by_id(user_id).await
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_database_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.database_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Database operation timed out after {}s",
                config.database_timeout_secs
            )))
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
/// ```rust,no_run
/// use pierre_mcp_server::utils::route_timeout::with_provider_api_timeout;
///
/// # async fn example() -> pierre_mcp_server::errors::AppResult<()> {
/// # struct Provider; impl Provider { async fn get_activities(&self, _: usize, _: usize) -> pierre_mcp_server::errors::AppResult<Vec<()>> { Ok(vec![]) } }
/// # let provider = Provider; let limit = 10; let page = 1;
/// let activities = with_provider_api_timeout(async {
///     provider.get_activities(limit, page).await
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_provider_api_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.provider_api_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Provider API operation timed out after {}s",
                config.provider_api_timeout_secs
            )))
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
/// ```rust,no_run
/// use pierre_mcp_server::utils::route_timeout::with_sse_timeout;
///
/// # async fn example() -> pierre_mcp_server::errors::AppResult<()> {
/// # struct SseManager; impl SseManager { async fn send_event(&self, _: &str, _: &str) -> pierre_mcp_server::errors::AppResult<()> { Ok(()) } }
/// # let sse_manager = SseManager; let user_id = "test"; let event = "data";
/// with_sse_timeout(async {
///     sse_manager.send_event(user_id, event).await
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_sse_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.sse_event_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "SSE event operation timed out after {}s",
                config.sse_event_timeout_secs
            )))
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
/// ```rust,no_run
/// use pierre_mcp_server::utils::route_timeout::with_oauth_timeout;
///
/// # async fn example() -> pierre_mcp_server::errors::AppResult<()> {
/// # struct AuthServer; impl AuthServer { async fn exchange_token(&self, _: &str, _: &str) -> pierre_mcp_server::errors::AppResult<String> { Ok(String::new()) } }
/// # let auth_server = AuthServer; let code = "test"; let verifier = "test";
/// let token = with_oauth_timeout(async {
///     auth_server.exchange_token(code, verifier).await
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub async fn with_oauth_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.oauth_operation_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "OAuth operation timed out after {}s",
                config.oauth_operation_timeout_secs
            )))
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

/// Execute a default operation with configured timeout
///
/// # Arguments
/// * `operation` - The async operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
pub async fn with_default_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.default_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Operation timed out after {}s",
                config.default_timeout_secs
            )))
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute an upload operation with configured timeout
///
/// # Arguments
/// * `operation` - The async upload operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
pub async fn with_upload_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.upload_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Upload operation timed out after {}s",
                config.upload_timeout_secs
            )))
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute a long polling operation with configured timeout
///
/// # Arguments
/// * `operation` - The async long polling operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
pub async fn with_long_polling_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.long_polling_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Long polling operation timed out after {}s",
                config.long_polling_timeout_secs
            )))
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute an MCP sampling operation with configured timeout
///
/// # Arguments
/// * `operation` - The async MCP sampling operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
pub async fn with_mcp_sampling_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.mcp_sampling_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "MCP sampling operation timed out after {}s",
                config.mcp_sampling_timeout_secs
            )))
        },
        |result| result.map_err(Into::into),
    )
}

/// Execute a geocoding operation with configured timeout
///
/// # Arguments
/// * `operation` - The async geocoding operation to execute
///
/// # Returns
/// Result with the operation's value or a timeout error
///
/// # Errors
/// Returns an error if the operation times out or the operation itself fails
pub async fn with_geocoding_timeout<F, T, E>(operation: F) -> AppResult<T>
where
    F: Future<Output = Result<T, E>>,
    E: Into<AppError>,
{
    let config = get_config();
    let duration = Duration::from_secs(config.geocoding_timeout_secs);

    (timeout(duration, operation).await).map_or_else(
        |_| {
            Err(AppError::internal(format!(
                "Geocoding operation timed out after {}s",
                config.geocoding_timeout_secs
            )))
        },
        |result| result.map_err(Into::into),
    )
}

/// Get default timeout duration for manual timeout handling
///
/// # Returns
/// Duration for default operations
#[must_use]
pub fn default_timeout_duration() -> Duration {
    Duration::from_secs(get_config().default_timeout_secs)
}

/// Get upload timeout duration for manual timeout handling
///
/// # Returns
/// Duration for upload operations
#[must_use]
pub fn upload_timeout_duration() -> Duration {
    Duration::from_secs(get_config().upload_timeout_secs)
}

/// Get long polling timeout duration for manual timeout handling
///
/// # Returns
/// Duration for long polling operations
#[must_use]
pub fn long_polling_timeout_duration() -> Duration {
    Duration::from_secs(get_config().long_polling_timeout_secs)
}

/// Get MCP sampling timeout duration for manual timeout handling
///
/// # Returns
/// Duration for MCP sampling operations
#[must_use]
pub fn mcp_sampling_timeout_duration() -> Duration {
    Duration::from_secs(get_config().mcp_sampling_timeout_secs)
}

/// Get geocoding timeout duration for manual timeout handling
///
/// # Returns
/// Duration for geocoding operations
#[must_use]
pub fn geocoding_timeout_duration() -> Duration {
    Duration::from_secs(get_config().geocoding_timeout_secs)
}
