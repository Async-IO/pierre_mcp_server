// ABOUTME: Rate limiting middleware for HTTP requests
// ABOUTME: Enforces request rate limits and prevents API abuse
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Rate Limiting Middleware with HTTP Headers
//!
//! This module provides utilities for adding standard HTTP rate limiting headers
//! to responses and creating proper 429 status codes when limits are exceeded.

use crate::errors::{AppError, ErrorCode};
use crate::rate_limiting::UnifiedRateLimitInfo;
use http::{HeaderMap, HeaderValue};

/// HTTP header names for rate limiting
pub mod headers {
    /// HTTP header name for maximum requests allowed in the current window
    pub const X_RATE_LIMIT_LIMIT: &str = "X-RateLimit-Limit";
    /// HTTP header name for remaining requests in the current window
    pub const X_RATE_LIMIT_REMAINING: &str = "X-RateLimit-Remaining";
    /// HTTP header name for Unix timestamp when rate limit resets
    pub const X_RATE_LIMIT_RESET: &str = "X-RateLimit-Reset";
    /// HTTP header name for rate limit window duration in seconds
    pub const X_RATE_LIMIT_WINDOW: &str = "X-RateLimit-Window";
    /// HTTP header name for rate limit tier information
    pub const X_RATE_LIMIT_TIER: &str = "X-RateLimit-Tier";
    /// HTTP header name for authentication method used
    pub const X_RATE_LIMIT_AUTH_METHOD: &str = "X-RateLimit-AuthMethod";
    /// HTTP header name for retry-after duration in seconds
    pub const RETRY_AFTER: &str = "Retry-After";
}

/// Create a `HeaderMap` with rate limit headers
#[must_use]
pub fn create_rate_limit_headers(rate_limit_info: &UnifiedRateLimitInfo) -> HeaderMap {
    let mut headers = HeaderMap::new();

    // Add rate limit headers if we have the information
    if let Some(limit) = rate_limit_info.limit {
        if let Ok(header_value) = HeaderValue::from_str(&limit.to_string()) {
            headers.insert(headers::X_RATE_LIMIT_LIMIT, header_value);
        }
    }

    if let Some(remaining) = rate_limit_info.remaining {
        if let Ok(header_value) = HeaderValue::from_str(&remaining.to_string()) {
            headers.insert(headers::X_RATE_LIMIT_REMAINING, header_value);
        }
    }

    if let Some(reset_at) = rate_limit_info.reset_at {
        // Add reset timestamp as Unix epoch
        let reset_timestamp = reset_at.timestamp();
        if let Ok(header_value) = HeaderValue::from_str(&reset_timestamp.to_string()) {
            headers.insert(headers::X_RATE_LIMIT_RESET, header_value);
        }

        // Add Retry-After header (seconds until reset)
        let retry_after = (reset_at - chrono::Utc::now()).num_seconds().max(0);
        if let Ok(header_value) = HeaderValue::from_str(&retry_after.to_string()) {
            headers.insert(headers::RETRY_AFTER, header_value);
        }
    }

    // Add tier and authentication method information
    if let Ok(header_value) = HeaderValue::from_str(&rate_limit_info.tier) {
        headers.insert(headers::X_RATE_LIMIT_TIER, header_value);
    }

    if let Ok(header_value) = HeaderValue::from_str(&rate_limit_info.auth_method) {
        headers.insert(headers::X_RATE_LIMIT_AUTH_METHOD, header_value);
    }

    // Add rate limit window (always 30 days for monthly limits)
    headers.insert(
        headers::X_RATE_LIMIT_WINDOW,
        HeaderValue::from_static("2592000"), // 30 days in seconds
    );

    headers
}

/// Create a rate limit exceeded error response with proper headers
#[must_use]
pub fn create_rate_limit_error(rate_limit_info: &UnifiedRateLimitInfo) -> AppError {
    let limit = rate_limit_info.limit.unwrap_or(0);

    AppError::new(
        ErrorCode::RateLimitExceeded,
        format!(
            "Rate limit exceeded. You have reached your limit of {} requests for the {} tier",
            limit, rate_limit_info.tier
        ),
    )
}

/// Helper function to check rate limits and return appropriate response
///
/// # Errors
///
/// Returns an error if the rate limit has been exceeded
pub fn check_rate_limit_and_respond(
    rate_limit_info: &UnifiedRateLimitInfo,
) -> Result<(), AppError> {
    if rate_limit_info.is_rate_limited {
        Err(create_rate_limit_error(rate_limit_info))
    } else {
        Ok(())
    }
}
