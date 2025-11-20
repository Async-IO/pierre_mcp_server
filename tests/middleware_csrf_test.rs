// Integration tests for CSRF middleware
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(missing_docs)]

use axum::http::{HeaderMap, Method};
use pierre_mcp_server::middleware::csrf::CsrfMiddleware;
use pierre_mcp_server::security::csrf::CsrfTokenManager;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn test_csrf_middleware_get_request() -> anyhow::Result<()> {
    let csrf_manager = Arc::new(CsrfTokenManager::new());
    let middleware = CsrfMiddleware::new(csrf_manager);

    let headers = HeaderMap::new();
    let user_id = Uuid::new_v4();

    // GET requests should not require CSRF token
    let result = middleware
        .validate_csrf(&headers, &Method::GET, user_id)
        .await;

    assert!(result.is_ok(), "GET request should not require CSRF token");
    Ok(())
}

#[tokio::test]
async fn test_csrf_middleware_post_without_token() -> anyhow::Result<()> {
    let csrf_manager = Arc::new(CsrfTokenManager::new());
    let middleware = CsrfMiddleware::new(csrf_manager);

    let headers = HeaderMap::new();
    let user_id = Uuid::new_v4();

    // POST without CSRF token should fail
    let result = middleware
        .validate_csrf(&headers, &Method::POST, user_id)
        .await;

    assert!(
        result.is_err(),
        "POST request without CSRF token should fail"
    );
    Ok(())
}

#[tokio::test]
async fn test_csrf_middleware_post_with_valid_token() -> anyhow::Result<()> {
    let csrf_manager = Arc::new(CsrfTokenManager::new());
    let middleware = CsrfMiddleware::new(Arc::clone(&csrf_manager));

    let user_id = Uuid::new_v4();
    let token = csrf_manager.generate_token(user_id).await?;

    let mut headers = HeaderMap::new();
    headers.insert("X-CSRF-Token", token.parse()?);

    // POST with valid CSRF token should succeed
    let result = middleware
        .validate_csrf(&headers, &Method::POST, user_id)
        .await;

    assert!(
        result.is_ok(),
        "POST request with valid CSRF token should succeed"
    );
    Ok(())
}

#[tokio::test]
async fn test_csrf_middleware_post_with_invalid_token() -> anyhow::Result<()> {
    let csrf_manager = Arc::new(CsrfTokenManager::new());
    let middleware = CsrfMiddleware::new(csrf_manager);

    let user_id = Uuid::new_v4();

    let mut headers = HeaderMap::new();
    headers.insert("X-CSRF-Token", "invalid_token".parse()?);

    // POST with invalid CSRF token should fail
    let result = middleware
        .validate_csrf(&headers, &Method::POST, user_id)
        .await;

    assert!(
        result.is_err(),
        "POST request with invalid CSRF token should fail"
    );
    Ok(())
}

#[tokio::test]
async fn test_csrf_middleware_requires_validation() {
    assert!(
        CsrfMiddleware::requires_csrf_validation(&Method::POST),
        "POST should require CSRF validation"
    );
    assert!(
        CsrfMiddleware::requires_csrf_validation(&Method::PUT),
        "PUT should require CSRF validation"
    );
    assert!(
        CsrfMiddleware::requires_csrf_validation(&Method::DELETE),
        "DELETE should require CSRF validation"
    );
    assert!(
        CsrfMiddleware::requires_csrf_validation(&Method::PATCH),
        "PATCH should require CSRF validation"
    );
    assert!(
        !CsrfMiddleware::requires_csrf_validation(&Method::GET),
        "GET should not require CSRF validation"
    );
    assert!(
        !CsrfMiddleware::requires_csrf_validation(&Method::HEAD),
        "HEAD should not require CSRF validation"
    );
    assert!(
        !CsrfMiddleware::requires_csrf_validation(&Method::OPTIONS),
        "OPTIONS should not require CSRF validation"
    );
}
