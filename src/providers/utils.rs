// ABOUTME: Shared utilities for fitness provider implementations
// ABOUTME: Type conversions, retry logic, token refresh, and common patterns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use crate::errors::{AppError, AppResult};
use chrono::{TimeZone, Utc};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{error, info, warn};

use super::core::OAuth2Credentials;
use super::errors::ProviderError;

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
    error!("{provider_name} API request failed - status: {status}, body: {text}");
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
