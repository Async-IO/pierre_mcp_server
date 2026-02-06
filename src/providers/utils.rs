// ABOUTME: Shared utilities for fitness provider implementations
// ABOUTME: Type conversions, retry logic, token refresh, and common patterns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult};
use chrono::{TimeZone, Utc};
use rand::Rng;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::env;
use std::future::Future;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::core::OAuth2Credentials;
use super::errors::{ProviderError, ProviderResult};

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds
    pub initial_backoff_ms: u64,
    /// HTTP status codes that should trigger retries
    pub retryable_status_codes: Vec<StatusCode>,
    /// Estimated block duration for user-facing error messages (seconds)
    pub estimated_block_duration_secs: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            retryable_status_codes: vec![StatusCode::TOO_MANY_REQUESTS],
            estimated_block_duration_secs: 3600, // 1 hour
        }
    }
}

/// Type conversion utilities for safe float-to-integer conversions
pub mod conversions {
    use num_traits::ToPrimitive;

    /// Safely convert f64 to u64, clamping to valid range
    /// Used for duration values from APIs that return floats
    #[must_use]
    pub fn f64_to_u64(value: f64) -> u64 {
        if !value.is_finite() {
            return 0;
        }
        let t = value.trunc();
        if t.is_sign_negative() {
            return 0;
        }
        t.to_u64().map_or(u64::MAX, |v| v)
    }

    /// Safely convert f32 to u32, clamping to valid range
    /// Used for metrics like heart rate, power, cadence
    #[must_use]
    pub fn f32_to_u32(value: f32) -> u32 {
        if !value.is_finite() {
            return 0;
        }
        let t = value.trunc();
        if t.is_sign_negative() {
            return 0;
        }
        t.to_u32().map_or(u32::MAX, |v| v)
    }

    /// Safely convert f64 to u32, clamping to valid range
    /// Used for calorie values and other metrics
    #[must_use]
    pub fn f64_to_u32(value: f64) -> u32 {
        if !value.is_finite() {
            return 0;
        }
        let t = value.trunc();
        if t.is_sign_negative() {
            return 0;
        }
        t.to_u32().map_or(u32::MAX, |v| v)
    }
}

/// Result of checking if a response should be retried
enum RetryDecision {
    /// Continue with retry after backoff
    Retry { backoff_ms: u64 },
    /// Max retries reached, return error
    MaxRetriesExceeded,
    /// Not a retryable status, continue processing
    NotRetryable,
}

/// Check if a response status should trigger a retry
fn check_retry_status(
    status: StatusCode,
    attempt: u32,
    retry_config: &RetryConfig,
    provider_name: &str,
) -> RetryDecision {
    if !retry_config.retryable_status_codes.contains(&status) {
        return RetryDecision::NotRetryable;
    }

    let current_attempt = attempt + 1;
    if current_attempt >= retry_config.max_retries {
        warn!(
            "{provider_name} API rate limit exceeded - max retries ({}) reached",
            retry_config.max_retries
        );
        return RetryDecision::MaxRetriesExceeded;
    }

    let backoff_ms = retry_config.initial_backoff_ms * 2_u64.pow(current_attempt - 1);
    let status_code = status.as_u16();
    warn!(
        "{provider_name} API rate limit hit ({status_code}) - retry {current_attempt}/{} after {backoff_ms}ms backoff",
        retry_config.max_retries
    );

    RetryDecision::Retry { backoff_ms }
}

/// Create a rate limit exceeded error
fn rate_limit_error(
    status: StatusCode,
    provider_name: &str,
    retry_config: &RetryConfig,
) -> AppError {
    let minutes = retry_config.estimated_block_duration_secs / 60;
    let status_code = status.as_u16();
    let err = ProviderError::RateLimitExceeded {
        provider: provider_name.to_owned(),
        retry_after_secs: retry_config.estimated_block_duration_secs,
        limit_type: format!(
            "API rate limit ({status_code}) - max retries reached - wait ~{minutes} minutes"
        ),
    };
    AppError::external_service(provider_name, err.to_string())
}

/// Create an API error for non-success responses
fn api_error(status: StatusCode, text: &str, provider_name: &str) -> AppError {
    error!(
        "{provider_name} API request failed - status: {status}, body_length: {} bytes",
        text.len()
    );
    let err = ProviderError::ApiError {
        provider: provider_name.to_owned(),
        status_code: status.as_u16(),
        message: format!("{provider_name} API request failed with status {status}: {text}"),
        retryable: false,
    };
    AppError::external_service(provider_name, err.to_string())
}

/// Make an authenticated HTTP GET request with retry logic
///
/// # Errors
///
/// Returns an error if:
/// - No access token is available
/// - All retry attempts are exhausted
/// - Network request fails
/// - Response parsing fails
pub async fn api_request_with_retry<T>(
    client: &Client,
    url: &str,
    access_token: &str,
    provider_name: &str,
    retry_config: &RetryConfig,
) -> AppResult<T>
where
    T: for<'de> Deserialize<'de>,
{
    info!("Starting {provider_name} API request to: {url}");

    let mut attempt = 0;
    loop {
        let response = client
            .get(url)
            .header("Authorization", format!("Bearer {access_token}"))
            .send()
            .await
            .map_err(|e| {
                AppError::external_service(provider_name, format!("Failed to send request: {e}"))
            })?;

        let status = response.status();
        info!("Received HTTP response with status: {status}");

        match check_retry_status(status, attempt, retry_config, provider_name) {
            RetryDecision::Retry { backoff_ms } => {
                attempt += 1;
                sleep(Duration::from_millis(backoff_ms)).await;
                continue;
            }
            RetryDecision::MaxRetriesExceeded => {
                return Err(rate_limit_error(status, provider_name, retry_config));
            }
            RetryDecision::NotRetryable => {}
        }

        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(api_error(status, &text, provider_name));
        }

        info!("Parsing JSON response from {provider_name} API");
        return response.json().await.map_err(|e| {
            error!("Failed to parse JSON response: {e}");
            AppError::external_service(provider_name, format!("Failed to parse API response: {e}"))
        });
    }
}

/// Standard token refresh response structure
#[derive(Debug, Deserialize)]
pub struct TokenRefreshResponse {
    /// New access token from the OAuth provider
    pub access_token: String,
    /// Optional new refresh token (if rotated by provider)
    pub refresh_token: Option<String>,
    /// Token expiration time in seconds from now
    #[serde(default)]
    pub expires_in: Option<i64>,
    /// Token expiration as Unix timestamp
    #[serde(default)]
    pub expires_at: Option<i64>,
}

/// Refresh `OAuth2` access token using refresh token
///
/// # Errors
///
/// Returns an error if:
/// - HTTP request fails
/// - Token endpoint returns error
/// - Response parsing fails
pub async fn refresh_oauth_token(
    client: &Client,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    provider_name: &str,
) -> AppResult<OAuth2Credentials> {
    info!("Refreshing {provider_name} access token");

    let params = [
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| {
            AppError::external_service(
                provider_name,
                format!("Failed to send token refresh request: {e}"),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let err = ProviderError::AuthenticationFailed {
            provider: provider_name.to_owned(),
            reason: format!("token refresh failed with status: {status}"),
        };
        return Err(AppError::external_service(provider_name, err.to_string()));
    }

    let token_response: TokenRefreshResponse = response.json().await.map_err(|e| {
        AppError::external_service(
            provider_name,
            format!("Failed to parse token refresh response: {e}"),
        )
    })?;

    // Calculate expiry time
    let expires_at = token_response
        .expires_at
        .and_then(|ts| Utc.timestamp_opt(ts, 0).single())
        .or_else(|| {
            token_response
                .expires_in
                .map(|secs| Utc::now() + chrono::Duration::seconds(secs))
        });

    Ok(OAuth2Credentials {
        client_id: client_id.to_owned(),
        client_secret: client_secret.to_owned(),
        access_token: Some(token_response.access_token),
        refresh_token: token_response.refresh_token,
        expires_at,
        scopes: vec![], // Preserve original scopes in caller
    })
}

/// Check if `OAuth2` credentials need refresh
/// Returns `true` if token expires within the threshold
#[must_use]
pub fn needs_token_refresh(
    credentials: &Option<OAuth2Credentials>,
    refresh_threshold_minutes: i64,
) -> bool {
    credentials.as_ref().is_some_and(|creds| {
        creds.expires_at.is_some_and(|expires_at| {
            Utc::now() + chrono::Duration::minutes(refresh_threshold_minutes) > expires_at
        })
    })
}

/// Check if credentials are authenticated (has valid access token)
#[must_use]
pub fn is_authenticated(credentials: &Option<OAuth2Credentials>) -> bool {
    credentials.as_ref().is_some_and(|creds| {
        creds.access_token.is_some()
            && creds
                .expires_at
                .is_none_or(|expires_at| Utc::now() < expires_at)
    })
}

/// Environment variable name for maximum retry attempts
pub const ENV_RETRY_MAX_ATTEMPTS: &str = "PIERRE_RETRY_MAX_ATTEMPTS";
/// Environment variable name for base delay in milliseconds
pub const ENV_RETRY_BASE_DELAY_MS: &str = "PIERRE_RETRY_BASE_DELAY_MS";
/// Environment variable name for maximum delay in milliseconds
pub const ENV_RETRY_MAX_DELAY_MS: &str = "PIERRE_RETRY_MAX_DELAY_MS";
/// Environment variable name for jitter factor (0.0 to 1.0)
pub const ENV_RETRY_JITTER_FACTOR: &str = "PIERRE_RETRY_JITTER_FACTOR";

/// Parse a u32 environment variable with validation and fallback to default
fn parse_env_u32(name: &str, default: u32, min: u32, max: u32) -> u32 {
    env::var(name).map_or(default, |val| {
        val.parse::<u32>().map_or_else(
            |e| {
                warn!(
                    "Failed to parse environment variable {name}='{val}': {e}, using default {default}"
                );
                default
            },
            |parsed| {
                if parsed >= min && parsed <= max {
                    parsed
                } else {
                    warn!(
                        "Environment variable {name}={parsed} out of range [{min}, {max}], using default {default}"
                    );
                    default
                }
            },
        )
    })
}

/// Parse a u64 environment variable with validation and fallback to default
fn parse_env_u64(name: &str, default: u64, min: u64, max: u64) -> u64 {
    env::var(name).map_or(default, |val| {
        val.parse::<u64>().map_or_else(
            |e| {
                warn!(
                    "Failed to parse environment variable {name}='{val}': {e}, using default {default}"
                );
                default
            },
            |parsed| {
                if parsed >= min && parsed <= max {
                    parsed
                } else {
                    warn!(
                        "Environment variable {name}={parsed} out of range [{min}, {max}], using default {default}"
                    );
                    default
                }
            },
        )
    })
}

/// Parse an f64 environment variable with validation and fallback to default
fn parse_env_f64(name: &str, default: f64, min: f64, max: f64) -> f64 {
    env::var(name).map_or(default, |val| {
        val.parse::<f64>().map_or_else(
            |e| {
                warn!(
                    "Failed to parse environment variable {name}='{val}': {e}, using default {default}"
                );
                default
            },
            |parsed| {
                if parsed >= min && parsed <= max {
                    parsed
                } else {
                    warn!(
                        "Environment variable {name}={parsed} out of range [{min}, {max}], using default {default}"
                    );
                    default
                }
            },
        )
    })
}

/// Configuration for exponential backoff retry behavior
#[derive(Debug, Clone)]
pub struct RetryBackoffConfig {
    /// Maximum number of retry attempts (not including the initial attempt)
    pub max_attempts: u32,
    /// Base delay for exponential backoff in milliseconds
    pub base_delay_ms: u64,
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    /// Jitter factor (0.0 to 1.0) to add randomness to delays
    pub jitter_factor: f64,
}

impl Default for RetryBackoffConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 1000,
            max_delay_ms: 30000,
            jitter_factor: 0.1,
        }
    }
}

impl RetryBackoffConfig {
    /// Create a new retry config with custom settings
    #[must_use]
    pub const fn new(max_attempts: u32, base_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
            jitter_factor: 0.1,
        }
    }

    /// Create a retry config suitable for rate-limited APIs
    #[must_use]
    pub const fn for_rate_limited_api() -> Self {
        Self {
            max_attempts: 5,
            base_delay_ms: 2000,
            max_delay_ms: 60000,
            jitter_factor: 0.2,
        }
    }

    /// Create a retry config for quick network hiccups
    #[must_use]
    pub const fn for_transient_errors() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 500,
            max_delay_ms: 5000,
            jitter_factor: 0.1,
        }
    }

    /// Create a retry config from environment variables
    ///
    /// Reads the following environment variables:
    /// - `PIERRE_RETRY_MAX_ATTEMPTS`: Maximum retry attempts (default: 3)
    /// - `PIERRE_RETRY_BASE_DELAY_MS`: Base delay in milliseconds (default: 1000)
    /// - `PIERRE_RETRY_MAX_DELAY_MS`: Maximum delay in milliseconds (default: 30000)
    /// - `PIERRE_RETRY_JITTER_FACTOR`: Jitter factor 0.0-1.0 (default: 0.1)
    ///
    /// Invalid values are logged as warnings and fall back to defaults.
    #[must_use]
    pub fn from_env() -> Self {
        let defaults = Self::default();

        let max_attempts = parse_env_u32(ENV_RETRY_MAX_ATTEMPTS, defaults.max_attempts, 1, 100);

        let base_delay_ms = parse_env_u64(
            ENV_RETRY_BASE_DELAY_MS,
            defaults.base_delay_ms,
            100,
            300_000,
        );

        let max_delay_ms =
            parse_env_u64(ENV_RETRY_MAX_DELAY_MS, defaults.max_delay_ms, 1000, 600_000);

        let jitter_factor =
            parse_env_f64(ENV_RETRY_JITTER_FACTOR, defaults.jitter_factor, 0.0, 1.0);

        Self {
            max_attempts,
            base_delay_ms,
            max_delay_ms,
            jitter_factor,
        }
    }

    /// Calculate the delay for a given attempt using exponential backoff with jitter
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn calculate_delay(&self, attempt: u32, rate_limit_delay: Option<u64>) -> Duration {
        // If rate limit specifies a delay, use it (with a cap)
        if let Some(rate_limit_secs) = rate_limit_delay {
            let rate_limit_ms = rate_limit_secs.saturating_mul(1000);
            return Duration::from_millis(rate_limit_ms.min(self.max_delay_ms));
        }

        // Exponential backoff: base_delay * 2^attempt
        let exponential_delay = self
            .base_delay_ms
            .saturating_mul(2_u64.saturating_pow(attempt));
        let capped_delay = exponential_delay.min(self.max_delay_ms);

        // Add random jitter to prevent thundering herd
        // jitter_factor is always positive (0.0 to 1.0), and capped_delay is u64,
        // so the product is always non-negative. Truncation is acceptable for jitter.
        let jitter_range = (f64::from(u32::try_from(capped_delay).unwrap_or(u32::MAX))
            * self.jitter_factor) as u64;
        let jitter = if jitter_range > 0 {
            rand::thread_rng().gen_range(0..jitter_range)
        } else {
            0
        };

        Duration::from_millis(capped_delay.saturating_add(jitter))
    }
}

/// Decision after evaluating an operation result in retry loop
enum RetryLoopDecision<T> {
    /// Operation succeeded, return the result
    Success(T),
    /// Operation failed with non-retryable error or max attempts reached
    Failure(ProviderError),
    /// Retry the operation after the specified delay
    Retry {
        delay: Duration,
        error: ProviderError,
    },
}

/// Log success message for retried operation
fn log_retry_success(operation_name: &str, attempt: u32, max_attempts: u32) {
    info!(
        "Operation '{operation_name}' succeeded on attempt {}/{}",
        attempt + 1,
        max_attempts + 1
    );
}

/// Log and prepare retry decision
fn prepare_retry(
    err: ProviderError,
    attempt: u32,
    config: &RetryBackoffConfig,
    operation_name: &str,
) -> RetryLoopDecision<()> {
    let delay = config.calculate_delay(attempt, err.retry_after_secs());
    warn!(
        "Operation '{operation_name}' failed (attempt {}/{}), retrying in {}ms: {err}",
        attempt + 1,
        config.max_attempts + 1,
        delay.as_millis()
    );
    RetryLoopDecision::Retry { delay, error: err }
}

/// Log final failure message
fn log_final_failure(operation_name: &str, attempt: u32, err: &ProviderError) {
    warn!(
        "Operation '{operation_name}' failed after {} attempts: {err}",
        attempt + 1
    );
}

/// Evaluate the result of an operation attempt and decide next action
fn evaluate_retry_attempt<T>(
    result: ProviderResult<T>,
    attempt: u32,
    config: &RetryBackoffConfig,
    operation_name: &str,
) -> RetryLoopDecision<T> {
    match result {
        Ok(value) => {
            if attempt > 0 {
                log_retry_success(operation_name, attempt, config.max_attempts);
            }
            RetryLoopDecision::Success(value)
        }
        Err(err) => {
            let should_retry = err.is_retryable() && attempt < config.max_attempts;
            if should_retry {
                let RetryLoopDecision::Retry { delay, error } =
                    prepare_retry(err, attempt, config, operation_name)
                else {
                    unreachable!()
                };
                RetryLoopDecision::Retry { delay, error }
            } else {
                if attempt > 0 {
                    log_final_failure(operation_name, attempt, &err);
                }
                RetryLoopDecision::Failure(err)
            }
        }
    }
}

/// Execute an async operation with automatic retry on retryable errors
///
/// This function wraps any async operation that returns `ProviderResult<T>` and
/// automatically retries on transient failures using exponential backoff.
///
/// # Retry Behavior
///
/// - Uses `ProviderError::is_retryable()` to determine if an error should trigger a retry
/// - Respects `retry_after_secs()` from `RateLimitExceeded` errors
/// - Applies exponential backoff with configurable jitter
/// - Logs each retry attempt with context
///
/// # Arguments
///
/// * `operation_name` - A descriptive name for the operation (used in logs)
/// * `config` - Retry configuration including max attempts and backoff settings
/// * `operation` - The async closure to execute and potentially retry
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::providers::utils::{with_retry, RetryBackoffConfig};
/// use pierre_mcp_server::providers::errors::ProviderResult;
///
/// async fn fetch_data_with_retry() -> ProviderResult<String> {
///     with_retry(
///         "fetch_athlete_data",
///         &RetryBackoffConfig::default(),
///         || async {
///             // Your provider API call here
///             Ok("data".to_owned())
///         },
///     ).await
/// }
/// ```
///
/// # Errors
///
/// Returns the last error if all retry attempts are exhausted or if a non-retryable
/// error is encountered.
pub async fn with_retry<T, F, Fut>(
    operation_name: &str,
    config: &RetryBackoffConfig,
    operation: F,
) -> ProviderResult<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = ProviderResult<T>>,
{
    let mut last_error: Option<ProviderError> = None;

    for attempt in 0..=config.max_attempts {
        let decision = evaluate_retry_attempt(operation().await, attempt, config, operation_name);

        match decision {
            RetryLoopDecision::Success(result) => return Ok(result),
            RetryLoopDecision::Failure(err) => return Err(err),
            RetryLoopDecision::Retry { delay, error } => {
                sleep(delay).await;
                last_error = Some(error);
            }
        }
    }

    // This should be unreachable due to the loop logic, but handle it gracefully
    Err(last_error.unwrap_or_else(|| {
        ProviderError::NetworkError(format!(
            "Operation '{operation_name}' failed: max retries exceeded"
        ))
    }))
}

/// Execute an async operation with retry, using default configuration
///
/// Convenience wrapper around `with_retry` using `RetryBackoffConfig::default()`.
///
/// # Errors
///
/// Returns the last error if all retry attempts are exhausted or if a non-retryable
/// error is encountered.
pub async fn with_retry_default<T, F, Fut>(operation_name: &str, operation: F) -> ProviderResult<T>
where
    F: Fn() -> Fut,
    Fut: Future<Output = ProviderResult<T>>,
{
    with_retry(operation_name, &RetryBackoffConfig::default(), operation).await
}
