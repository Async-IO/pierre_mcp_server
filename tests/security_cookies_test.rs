// Integration tests for secure cookie utilities
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(missing_docs)]

use axum::http::{header, HeaderMap};
use pierre_mcp_server::security::cookies::{
    get_cookie_value, set_auth_cookie, set_csrf_cookie, SecureCookieConfig,
};

#[test]
fn test_secure_cookie_config() {
    let config = SecureCookieConfig::new("test".to_owned(), "value".to_owned(), 3600);

    let cookie_str = config.build();

    assert!(
        cookie_str.contains("test=value"),
        "Cookie should contain name and value"
    );
    assert!(
        cookie_str.contains("Max-Age=3600"),
        "Cookie should contain max age"
    );
    assert!(
        cookie_str.contains("HttpOnly"),
        "Cookie should be HttpOnly by default"
    );
    assert!(
        cookie_str.contains("Secure"),
        "Cookie should be Secure by default"
    );
    assert!(
        cookie_str.contains("SameSite=Strict"),
        "Cookie should have SameSite=Strict by default"
    );
    assert!(cookie_str.contains("Path=/"), "Cookie should have Path=/");
}

#[test]
fn test_auth_cookie() -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    set_auth_cookie(&mut headers, "test_token", 3600);

    let cookie_header = headers
        .get(header::SET_COOKIE)
        .ok_or_else(|| anyhow::anyhow!("Cookie header should be set"))?
        .to_str()?;

    assert!(
        cookie_header.contains("auth_token=test_token"),
        "Cookie should contain auth token"
    );
    assert!(
        cookie_header.contains("HttpOnly"),
        "Auth cookie should be HttpOnly"
    );
    assert!(
        cookie_header.contains("Secure"),
        "Auth cookie should be Secure"
    );
    Ok(())
}

#[test]
fn test_csrf_cookie() -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    set_csrf_cookie(&mut headers, "csrf_test_token", 1800);

    let cookie_header = headers
        .get(header::SET_COOKIE)
        .ok_or_else(|| anyhow::anyhow!("Cookie header should be set"))?
        .to_str()?;

    assert!(
        cookie_header.contains("csrf_token=csrf_test_token"),
        "Cookie should contain CSRF token"
    );
    assert!(
        !cookie_header.contains("HttpOnly"),
        "CSRF cookie should NOT be HttpOnly"
    );
    assert!(
        cookie_header.contains("Secure"),
        "CSRF cookie should be Secure"
    );
    Ok(())
}

#[test]
fn test_get_cookie_value() -> anyhow::Result<()> {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::COOKIE,
        "auth_token=test123; csrf_token=csrf456".parse()?,
    );

    let auth_value = get_cookie_value(&headers, "auth_token");
    assert_eq!(
        auth_value,
        Some("test123".to_owned()),
        "Should extract auth token"
    );

    let csrf_value = get_cookie_value(&headers, "csrf_token");
    assert_eq!(
        csrf_value,
        Some("csrf456".to_owned()),
        "Should extract CSRF token"
    );

    let missing = get_cookie_value(&headers, "missing");
    assert_eq!(missing, None, "Should return None for missing cookie");
    Ok(())
}
