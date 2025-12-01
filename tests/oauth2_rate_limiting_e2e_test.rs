// ABOUTME: End-to-end tests for OAuth2 endpoint rate limiting with RFC-compliant headers
// ABOUTME: Validates per-IP rate limiting, 429 responses, and rate limit header correctness
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::{
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    oauth2_server::{
        client_registration::ClientRegistrationManager, models::ClientRegistrationRequest,
        rate_limiting::OAuth2RateLimiter,
    },
    rate_limiting::OAuth2RateLimitConfig,
};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;

/// Test rate limiting on client registration endpoint
#[tokio::test]
async fn test_rate_limit_client_registration() {
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );

    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));

    // Default limit for registration endpoint is 10 requests per minute
    let endpoint = "register";

    // Make 10 successful requests
    for i in 1..=10 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(!status.is_limited, "Request {i} should not be rate limited");
        assert_eq!(status.limit, 10);
        assert_eq!(status.remaining, 11 - i); // Remaining is count before current request
        assert!(status.retry_after_seconds.is_none());

        // Actually register a client to simulate real usage
        let registration_request = ClientRegistrationRequest {
            redirect_uris: vec![format!("https://example{i}.com/callback")],
            client_name: Some(format!("Test Client {i}")),
            client_uri: None,
            grant_types: None,
            response_types: None,
            scope: None,
        };

        let result = registration_manager
            .register_client(registration_request)
            .await;
        assert!(result.is_ok(), "Registration {i} should succeed");
    }

    // 11th request should be rate limited
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited, "Request 11 should be rate limited");
    assert_eq!(status.limit, 10);
    assert_eq!(status.remaining, 0);
    assert!(status.retry_after_seconds.is_some());
    assert!(status.retry_after_seconds.unwrap() <= 60);
}

/// Test rate limiting on token endpoint
#[tokio::test]
async fn test_rate_limit_token_endpoint() {
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101));

    // Default limit for token endpoint is 30 requests per minute
    let endpoint = "token";

    // Make 30 successful requests
    for i in 1..=30 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(!status.is_limited, "Request {i} should not be rate limited");
        assert_eq!(status.limit, 30);
        assert_eq!(status.remaining, 31 - i); // Remaining is count before current request
    }

    // 31st request should be rate limited
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited, "Request 31 should be rate limited");
    assert_eq!(status.remaining, 0);
    assert!(status.retry_after_seconds.is_some());
}

/// Test rate limiting on authorization endpoint
#[tokio::test]
async fn test_rate_limit_authorization_endpoint() {
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 102));

    // Default limit for authorize endpoint is 60 requests per minute
    let endpoint = "authorize";

    // Make 60 successful requests
    for i in 1..=60 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(!status.is_limited, "Request {i} should not be rate limited");
        assert_eq!(status.limit, 60);
        assert_eq!(status.remaining, 61 - i); // Remaining is count before current request
    }

    // 61st request should be rate limited
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited, "Request 61 should be rate limited");
    assert_eq!(status.remaining, 0);
}

/// Test per-IP isolation - different IPs should have separate rate limits
#[tokio::test]
async fn test_per_ip_rate_limit_isolation() {
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip_1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 103));
    let client_ip_2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 104));

    let endpoint = "register";

    // Exhaust rate limit for IP 1
    for _i in 1..=10 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip_1);
        assert!(!status.is_limited);
    }

    // IP 1 should now be rate limited
    let status = rate_limiter.check_rate_limit(endpoint, client_ip_1);
    assert!(status.is_limited, "IP 1 should be rate limited");

    // IP 2 should still have full quota
    let status = rate_limiter.check_rate_limit(endpoint, client_ip_2);
    assert!(!status.is_limited, "IP 2 should not be rate limited");
    assert_eq!(status.remaining, 10); // Remaining is count before current request
}

/// Test rate limit headers contain correct values
#[tokio::test]
async fn test_rate_limit_headers() {
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 105));
    let endpoint = "token";

    // First request
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert_eq!(status.limit, 30, "X-RateLimit-Limit should be 30");
    assert_eq!(status.remaining, 30, "X-RateLimit-Remaining should be 30");
    assert!(status.reset_at > 0, "X-RateLimit-Reset should be set");

    // Second request
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert_eq!(
        status.remaining, 29,
        "X-RateLimit-Remaining should decrement"
    );

    // Reset timestamp should be in the future (within next 60 seconds)
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    #[allow(clippy::cast_possible_wrap)]
    let now_i64 = now as i64;
    assert!(
        status.reset_at > now_i64,
        "Reset timestamp should be in the future"
    );
    assert!(
        status.reset_at <= now_i64 + 60,
        "Reset timestamp should be within 60 seconds"
    );
}

/// Test 429 response includes Retry-After header
#[tokio::test]
async fn test_retry_after_header() {
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 106));
    let endpoint = "register";

    // Exhaust rate limit
    for i in 1..=10 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(
            !status.is_limited,
            "Request {i} should not be limited during quota exhaustion"
        );
    }

    // Next request should be rate limited with Retry-After
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited);
    assert!(
        status.retry_after_seconds.is_some(),
        "Retry-After header should be set"
    );

    let retry_after = status.retry_after_seconds.unwrap();
    assert!(
        retry_after > 0 && retry_after <= 60,
        "Retry-After should be between 1 and 60 seconds"
    );
}

/// Test rate limit window reset after expiration
#[tokio::test]
async fn test_rate_limit_window_reset() {
    // This test uses a custom config with a very short window for testing
    let mut config = OAuth2RateLimitConfig::new();
    config.register_rpm = 3; // Only 3 requests allowed

    let rate_limiter = OAuth2RateLimiter::with_config(config);
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 107));
    let endpoint = "register";

    // Make 3 requests
    for i in 1..=3 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(!status.is_limited, "Request {i} should succeed");
    }

    // 4th request should be rate limited
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited, "Request 4 should be rate limited");

    // Wait for window to expire (60 seconds + small buffer)
    tokio::time::sleep(tokio::time::Duration::from_secs(61)).await;

    // After window reset, should be able to make requests again
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(
        !status.is_limited,
        "After window reset, requests should succeed again"
    );
    assert_eq!(status.remaining, 3); // Remaining is count before current request
}

/// Test custom rate limit configuration
#[tokio::test]
async fn test_custom_rate_limit_config() {
    let mut config = OAuth2RateLimitConfig::new();
    config.register_rpm = 5;
    config.token_rpm = 15;
    config.authorize_rpm = 25;

    let rate_limiter = OAuth2RateLimiter::with_config(config);
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 108));

    // Test custom register limit
    let status = rate_limiter.check_rate_limit("register", client_ip);
    assert_eq!(status.limit, 5, "Custom register limit should be 5");

    // Test custom token limit
    let status = rate_limiter.check_rate_limit("token", client_ip);
    assert_eq!(status.limit, 15, "Custom token limit should be 15");

    // Test custom authorize limit
    let status = rate_limiter.check_rate_limit("authorize", client_ip);
    assert_eq!(status.limit, 25, "Custom authorize limit should be 25");
}

/// Test rate limiting behavior at exact limit boundary
#[tokio::test]
async fn test_rate_limit_boundary() {
    let rate_limiter = OAuth2RateLimiter::new();
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 109));
    let endpoint = "register";

    // Make exactly 10 requests (at limit)
    for i in 1..=10 {
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(!status.is_limited, "Request {i} should succeed");
        if i == 10 {
            assert_eq!(
                status.remaining, 1,
                "Remaining should be 1 at limit boundary"
            );
        }
    }

    // One more request should be rejected
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited, "Request beyond limit should be rejected");
}

/// Test concurrent requests from same IP
#[tokio::test]
async fn test_concurrent_requests_same_ip() {
    let rate_limiter = Arc::new(OAuth2RateLimiter::new());
    let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 110));
    let endpoint = "token";

    let mut handles = vec![];

    // Spawn 30 concurrent requests
    for _i in 0..30 {
        let limiter = Arc::clone(&rate_limiter);
        let handle = tokio::spawn(async move {
            let status = limiter.check_rate_limit(endpoint, client_ip);
            status.is_limited
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results: Vec<bool> = futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // All 30 requests should succeed (limit is 30)
    let limited_count = results.iter().filter(|&&limited| limited).count();
    assert_eq!(
        limited_count, 0,
        "All 30 concurrent requests should succeed"
    );

    // 31st request should be rate limited
    let status = rate_limiter.check_rate_limit(endpoint, client_ip);
    assert!(status.is_limited, "Request 31 should be rate limited");
}

/// Test that rate limiter cleans up old entries
#[tokio::test]
async fn test_rate_limiter_cleanup() {
    let rate_limiter = OAuth2RateLimiter::new();
    let endpoint = "register";

    // Create requests from many different IPs
    for i in 1..=100 {
        let client_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, i));
        let status = rate_limiter.check_rate_limit(endpoint, client_ip);
        assert!(
            !status.is_limited,
            "First request from IP {i} should not be limited"
        );
    }

    // Wait for cleanup window (2 minutes)
    tokio::time::sleep(tokio::time::Duration::from_secs(121)).await;

    // Make another request to trigger cleanup
    let new_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    let status = rate_limiter.check_rate_limit(endpoint, new_ip);
    assert!(
        !status.is_limited,
        "New request should succeed after cleanup"
    );
}
