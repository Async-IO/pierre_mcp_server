// ABOUTME: Test suite for provider utilities module
// ABOUTME: Tests type conversions, retry config, authentication helpers, and retry logic
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::providers::core::OAuth2Credentials;
use pierre_mcp_server::providers::errors::ProviderError;
use pierre_mcp_server::providers::utils::{
    conversions, is_authenticated, needs_token_refresh, with_retry, with_retry_default,
    RetryBackoffConfig, RetryConfig,
};
use reqwest::StatusCode;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[test]
fn test_f64_to_u64_conversion() {
    assert_eq!(conversions::f64_to_u64(0.0), 0);
    assert_eq!(conversions::f64_to_u64(100.5), 100);
    assert_eq!(conversions::f64_to_u64(1000.9), 1000);
    assert_eq!(conversions::f64_to_u64(-10.0), 0); // Clamps negative to 0
}

#[test]
fn test_f32_to_u32_conversion() {
    assert_eq!(conversions::f32_to_u32(0.0), 0);
    assert_eq!(conversions::f32_to_u32(150.7), 150);
    assert_eq!(conversions::f32_to_u32(-5.0), 0); // Clamps negative to 0
}

#[test]
fn test_f64_to_u32_conversion() {
    assert_eq!(conversions::f64_to_u32(0.0), 0);
    assert_eq!(conversions::f64_to_u32(500.5), 500);
    assert_eq!(conversions::f64_to_u32(-10.0), 0); // Clamps negative to 0
}

#[test]
fn test_needs_token_refresh() {
    // No credentials
    assert!(!needs_token_refresh(&None, 5));

    // Token expires in 1 minute (threshold 5 minutes)
    let expires_soon = Some(OAuth2Credentials {
        client_id: "test".to_owned(),
        client_secret: "secret".to_owned(),
        access_token: Some("token".to_owned()),
        refresh_token: Some("refresh".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::minutes(1)),
        scopes: vec![],
    });
    assert!(needs_token_refresh(&expires_soon, 5));

    // Token expires in 10 minutes (threshold 5 minutes)
    let expires_later = Some(OAuth2Credentials {
        client_id: "test".to_owned(),
        client_secret: "secret".to_owned(),
        access_token: Some("token".to_owned()),
        refresh_token: Some("refresh".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::minutes(10)),
        scopes: vec![],
    });
    assert!(!needs_token_refresh(&expires_later, 5));
}

#[test]
fn test_is_authenticated() {
    // No credentials
    assert!(!is_authenticated(&None));

    // No access token
    let no_token = Some(OAuth2Credentials {
        client_id: "test".to_owned(),
        client_secret: "secret".to_owned(),
        access_token: None,
        refresh_token: Some("refresh".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec![],
    });
    assert!(!is_authenticated(&no_token));

    // Expired token
    let expired = Some(OAuth2Credentials {
        client_id: "test".to_owned(),
        client_secret: "secret".to_owned(),
        access_token: Some("token".to_owned()),
        refresh_token: Some("refresh".to_owned()),
        expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
        scopes: vec![],
    });
    assert!(!is_authenticated(&expired));

    // Valid token
    let valid = Some(OAuth2Credentials {
        client_id: "test".to_owned(),
        client_secret: "secret".to_owned(),
        access_token: Some("token".to_owned()),
        refresh_token: Some("refresh".to_owned()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec![],
    });
    assert!(is_authenticated(&valid));

    // No expiry (assume valid)
    let no_expiry = Some(OAuth2Credentials {
        client_id: "test".to_owned(),
        client_secret: "secret".to_owned(),
        access_token: Some("token".to_owned()),
        refresh_token: Some("refresh".to_owned()),
        expires_at: None,
        scopes: vec![],
    });
    assert!(is_authenticated(&no_expiry));
}

#[test]
fn test_retry_config_default() {
    let config = RetryConfig::default();
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.initial_backoff_ms, 1000);
    assert_eq!(config.estimated_block_duration_secs, 3600);
    assert!(config
        .retryable_status_codes
        .contains(&StatusCode::TOO_MANY_REQUESTS));
}

#[test]
fn test_retry_config_custom() {
    let config = RetryConfig {
        max_retries: 5,
        initial_backoff_ms: 500,
        retryable_status_codes: vec![
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::SERVICE_UNAVAILABLE,
        ],
        estimated_block_duration_secs: 7200,
    };

    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_backoff_ms, 500);
    assert_eq!(config.estimated_block_duration_secs, 7200);
    assert!(config
        .retryable_status_codes
        .contains(&StatusCode::TOO_MANY_REQUESTS));
    assert!(config
        .retryable_status_codes
        .contains(&StatusCode::SERVICE_UNAVAILABLE));
}

#[test]
fn test_conversions_boundary_values() {
    // Test maximum values
    assert_eq!(conversions::f64_to_u64(f64::MAX), u64::MAX);
    assert_eq!(conversions::f32_to_u32(f32::MAX), u32::MAX);
    assert_eq!(conversions::f64_to_u32(f64::from(u32::MAX) + 1.0), u32::MAX);

    // Test zero
    assert_eq!(conversions::f64_to_u64(0.0), 0);
    assert_eq!(conversions::f32_to_u32(0.0), 0);
    assert_eq!(conversions::f64_to_u32(0.0), 0);

    // Test negative values (should clamp to 0)
    assert_eq!(conversions::f64_to_u64(-100.0), 0);
    assert_eq!(conversions::f32_to_u32(-100.0), 0);
    assert_eq!(conversions::f64_to_u32(-100.0), 0);
}

// Tests for RetryBackoffConfig

#[test]
fn test_retry_backoff_config_default() {
    let config = RetryBackoffConfig::default();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.base_delay_ms, 1000);
    assert_eq!(config.max_delay_ms, 30000);
    assert!((config.jitter_factor - 0.1).abs() < 0.001);
}

#[test]
fn test_retry_backoff_config_new() {
    let config = RetryBackoffConfig::new(5, 500, 10000);
    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.base_delay_ms, 500);
    assert_eq!(config.max_delay_ms, 10000);
    assert!((config.jitter_factor - 0.1).abs() < 0.001);
}

#[test]
fn test_retry_backoff_config_for_rate_limited_api() {
    let config = RetryBackoffConfig::for_rate_limited_api();
    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.base_delay_ms, 2000);
    assert_eq!(config.max_delay_ms, 60000);
    assert!((config.jitter_factor - 0.2).abs() < 0.001);
}

#[test]
fn test_retry_backoff_config_for_transient_errors() {
    let config = RetryBackoffConfig::for_transient_errors();
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.base_delay_ms, 500);
    assert_eq!(config.max_delay_ms, 5000);
    assert!((config.jitter_factor - 0.1).abs() < 0.001);
}

#[test]
fn test_calculate_delay_exponential_backoff() {
    let config = RetryBackoffConfig::new(5, 1000, 60000);

    // Attempt 0: 1000ms * 2^0 = 1000ms (plus jitter)
    let delay0 = config.calculate_delay(0, None);
    assert!(delay0.as_millis() >= 1000);
    assert!(delay0.as_millis() < 1200); // With 10% jitter

    // Attempt 1: 1000ms * 2^1 = 2000ms (plus jitter)
    let delay1 = config.calculate_delay(1, None);
    assert!(delay1.as_millis() >= 2000);
    assert!(delay1.as_millis() < 2400);

    // Attempt 2: 1000ms * 2^2 = 4000ms (plus jitter)
    let delay2 = config.calculate_delay(2, None);
    assert!(delay2.as_millis() >= 4000);
    assert!(delay2.as_millis() < 4800);
}

#[test]
fn test_calculate_delay_max_cap() {
    let config = RetryBackoffConfig::new(10, 1000, 5000);

    // High attempt number should cap at max_delay_ms
    let delay = config.calculate_delay(10, None);
    // With jitter, should be between 5000 and 5500
    assert!(delay.as_millis() <= 5600);
}

#[test]
fn test_calculate_delay_rate_limit_override() {
    let config = RetryBackoffConfig::new(5, 1000, 60000);

    // Rate limit delay should override exponential backoff
    let delay = config.calculate_delay(0, Some(30));
    assert_eq!(delay.as_millis(), 30000); // 30 seconds in milliseconds

    // Rate limit delay should be capped at max_delay_ms
    let delay_capped = config.calculate_delay(0, Some(120));
    assert_eq!(delay_capped.as_millis(), 60000); // Capped at max
}

// Async tests for with_retry

#[tokio::test]
async fn test_with_retry_success_first_try() {
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let result: Result<String, ProviderError> =
        with_retry("test_op", &RetryBackoffConfig::new(3, 10, 100), || {
            let count = Arc::clone(&call_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok("success".to_owned())
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(call_count.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_with_retry_success_after_retries() {
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let result: Result<String, ProviderError> =
        with_retry("test_op", &RetryBackoffConfig::new(3, 10, 100), || {
            let count = Arc::clone(&call_count_clone);
            async move {
                let current = count.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    // Fail first 2 attempts with retryable error
                    Err(ProviderError::NetworkError("transient failure".to_owned()))
                } else {
                    // Succeed on third attempt
                    Ok("success after retries".to_owned())
                }
            }
        })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success after retries");
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_with_retry_non_retryable_error() {
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let result: Result<String, ProviderError> =
        with_retry("test_op", &RetryBackoffConfig::new(3, 10, 100), || {
            let count = Arc::clone(&call_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                // Non-retryable error should not trigger retry
                Err(ProviderError::AuthenticationFailed {
                    provider: "test".to_owned(),
                    reason: "invalid credentials".to_owned(),
                })
            }
        })
        .await;

    assert!(result.is_err());
    // Should only call once since error is non-retryable
    assert_eq!(call_count.load(Ordering::SeqCst), 1);

    if let Err(ProviderError::AuthenticationFailed { provider, reason }) = result {
        assert_eq!(provider, "test");
        assert_eq!(reason, "invalid credentials");
    } else {
        panic!("Expected AuthenticationFailed error");
    }
}

#[tokio::test]
async fn test_with_retry_max_retries_exceeded() {
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    let result: Result<String, ProviderError> =
        with_retry("test_op", &RetryBackoffConfig::new(2, 10, 100), || {
            let count = Arc::clone(&call_count_clone);
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                // Always fail with retryable error
                Err(ProviderError::NetworkError("persistent failure".to_owned()))
            }
        })
        .await;

    assert!(result.is_err());
    // Initial attempt + 2 retries = 3 total calls
    assert_eq!(call_count.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn test_with_retry_default_convenience() {
    let result: Result<i32, ProviderError> =
        with_retry_default("simple_op", || async { Ok(42) }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_with_retry_rate_limit_error() {
    let call_count = Arc::new(AtomicU32::new(0));
    let call_count_clone = Arc::clone(&call_count);

    // Use very short delays for testing
    let config = RetryBackoffConfig::new(2, 10, 100);

    let result: Result<String, ProviderError> = with_retry("test_op", &config, || {
        let count = Arc::clone(&call_count_clone);
        async move {
            let current = count.fetch_add(1, Ordering::SeqCst);
            if current == 0 {
                // First attempt: rate limit with retry_after
                Err(ProviderError::RateLimitExceeded {
                    provider: "test".to_owned(),
                    retry_after_secs: 1, // Would wait 1 second, but capped by config
                    limit_type: "hourly".to_owned(),
                })
            } else {
                Ok("recovered".to_owned())
            }
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "recovered");
    assert_eq!(call_count.load(Ordering::SeqCst), 2);
}

#[test]
fn test_provider_error_is_retryable() {
    // Retryable errors
    assert!(ProviderError::NetworkError("network issue".to_owned()).is_retryable());
    assert!(ProviderError::RateLimitExceeded {
        provider: "test".to_owned(),
        retry_after_secs: 60,
        limit_type: "hourly".to_owned(),
    }
    .is_retryable());
    assert!(ProviderError::Timeout {
        provider: "test".to_owned(),
        operation: "fetch",
        timeout_secs: 30,
    }
    .is_retryable());
    assert!(ProviderError::HttpError {
        provider: "test".to_owned(),
        status: 503,
        body: "service unavailable".to_owned(),
    }
    .is_retryable());
    assert!(ProviderError::ApiError {
        provider: "test".to_owned(),
        status_code: 500,
        message: "internal error".to_owned(),
        retryable: true,
    }
    .is_retryable());

    // Non-retryable errors
    assert!(!ProviderError::AuthenticationFailed {
        provider: "test".to_owned(),
        reason: "invalid token".to_owned(),
    }
    .is_retryable());
    assert!(!ProviderError::NotFound {
        provider: "test".to_owned(),
        resource_type: "activity".to_owned(),
        resource_id: "123".to_owned(),
    }
    .is_retryable());
    assert!(!ProviderError::ConfigurationError {
        provider: "test".to_owned(),
        details: "missing client_id".to_owned(),
    }
    .is_retryable());
}

#[test]
fn test_provider_error_retry_after_secs() {
    let rate_limit = ProviderError::RateLimitExceeded {
        provider: "test".to_owned(),
        retry_after_secs: 120,
        limit_type: "daily".to_owned(),
    };
    assert_eq!(rate_limit.retry_after_secs(), Some(120));

    let network = ProviderError::NetworkError("issue".to_owned());
    assert_eq!(network.retry_after_secs(), None);
}
