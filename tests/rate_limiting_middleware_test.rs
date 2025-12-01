// ABOUTME: Integration tests for rate limiting middleware functionality
// ABOUTME: Tests rate limit error creation and checking mechanisms
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::Utc;
use pierre_mcp_server::errors::ErrorCode;
use pierre_mcp_server::middleware::rate_limiting::{
    check_rate_limit_and_respond, create_rate_limit_error,
};
use pierre_mcp_server::rate_limiting::UnifiedRateLimitInfo;

#[test]
fn test_rate_limit_error_creation() {
    let rate_limit_info = UnifiedRateLimitInfo {
        is_rate_limited: true,
        limit: Some(1000),
        remaining: Some(0),
        reset_at: Some(Utc::now() + chrono::Duration::hours(1)),
        tier: "professional".into(),
        auth_method: "api_key".into(),
    };

    let error = create_rate_limit_error(&rate_limit_info);
    assert_eq!(error.code, ErrorCode::RateLimitExceeded);
    assert_eq!(error.http_status(), 429);

    // Check basic error properties
    assert_eq!(
        error.code,
        pierre_mcp_server::errors::ErrorCode::RateLimitExceeded
    );
    assert!(error.message.contains("1000"));
    assert!(error.message.contains("professional"));
}

#[test]
fn test_rate_limit_check() {
    // Test when not rate limited
    let info = UnifiedRateLimitInfo {
        is_rate_limited: false,
        limit: Some(1000),
        remaining: Some(500),
        reset_at: None,
        tier: "starter".into(),
        auth_method: "jwt".into(),
    };

    assert!(check_rate_limit_and_respond(&info).is_ok());

    // Test when rate limited
    let info = UnifiedRateLimitInfo {
        is_rate_limited: true,
        limit: Some(1000),
        remaining: Some(0),
        reset_at: Some(Utc::now()),
        tier: "starter".into(),
        auth_method: "jwt".into(),
    };

    assert!(check_rate_limit_and_respond(&info).is_err());
}
