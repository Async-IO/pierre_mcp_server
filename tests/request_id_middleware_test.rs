// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Integration tests for request ID middleware
//!
//! Tests the request ID middleware functionality including:
//! - UUID generation for each request
//! - Request ID propagation through request/response lifecycle
//! - Request ID availability in handlers via extensions

#![allow(clippy::unwrap_used, clippy::expect_used)]

use axum::{
    body::{to_bytes, Body},
    http::{Request as HttpRequest, StatusCode},
    middleware,
    routing::get,
    Extension, Router,
};
use pierre_mcp_server::middleware::request_id::{request_id_middleware, RequestId};
use std::error::Error;
use tower::ServiceExt;
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "x-request-id";

async fn test_handler(Extension(request_id): Extension<RequestId>) -> String {
    format!("Request ID: {}", request_id.as_str())
}

#[tokio::test]
async fn test_request_id_middleware_generates_id() -> Result<(), Box<dyn Error>> {
    let app = Router::new()
        .route("/", get(test_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = HttpRequest::builder().uri("/").body(Body::empty())?;

    let response = app.oneshot(request).await?;

    // Check that response has request ID header
    let request_id_header = response.headers().get(REQUEST_ID_HEADER);
    assert!(request_id_header.is_some(), "Request ID header not present");

    // Verify it's a valid UUID format
    if let Some(header_value) = request_id_header {
        let request_id_str = header_value.to_str()?;
        assert!(
            Uuid::parse_str(request_id_str).is_ok(),
            "Request ID is not a valid UUID"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_request_id_available_in_handler() -> Result<(), Box<dyn Error>> {
    let app = Router::new()
        .route("/", get(test_handler))
        .layer(middleware::from_fn(request_id_middleware));

    let request = HttpRequest::builder().uri("/").body(Body::empty())?;

    let response = app.oneshot(request).await?;

    assert_eq!(response.status(), StatusCode::OK);

    // The handler should successfully access the request ID
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let body_str = String::from_utf8(body.to_vec())?;
    assert!(body_str.starts_with("Request ID: "));

    Ok(())
}
