// ABOUTME: Test suite for provider utilities module
// ABOUTME: Tests type conversions, retry config, and authentication helpers
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::Utc;
use pierre_mcp_server::providers::core::OAuth2Credentials;
use pierre_mcp_server::providers::utils::{
    conversions, is_authenticated, needs_token_refresh, RetryConfig,
};
use reqwest::StatusCode;

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
        client_id: "test".to_string(),
        client_secret: "secret".to_string(),
        access_token: Some("token".to_string()),
        refresh_token: Some("refresh".to_string()),
        expires_at: Some(Utc::now() + chrono::Duration::minutes(1)),
        scopes: vec![],
    });
    assert!(needs_token_refresh(&expires_soon, 5));

    // Token expires in 10 minutes (threshold 5 minutes)
    let expires_later = Some(OAuth2Credentials {
        client_id: "test".to_string(),
        client_secret: "secret".to_string(),
        access_token: Some("token".to_string()),
        refresh_token: Some("refresh".to_string()),
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
        client_id: "test".to_string(),
        client_secret: "secret".to_string(),
        access_token: None,
        refresh_token: Some("refresh".to_string()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec![],
    });
    assert!(!is_authenticated(&no_token));

    // Expired token
    let expired = Some(OAuth2Credentials {
        client_id: "test".to_string(),
        client_secret: "secret".to_string(),
        access_token: Some("token".to_string()),
        refresh_token: Some("refresh".to_string()),
        expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
        scopes: vec![],
    });
    assert!(!is_authenticated(&expired));

    // Valid token
    let valid = Some(OAuth2Credentials {
        client_id: "test".to_string(),
        client_secret: "secret".to_string(),
        access_token: Some("token".to_string()),
        refresh_token: Some("refresh".to_string()),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scopes: vec![],
    });
    assert!(is_authenticated(&valid));

    // No expiry (assume valid)
    let no_expiry = Some(OAuth2Credentials {
        client_id: "test".to_string(),
        client_secret: "secret".to_string(),
        access_token: Some("token".to_string()),
        refresh_token: Some("refresh".to_string()),
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
