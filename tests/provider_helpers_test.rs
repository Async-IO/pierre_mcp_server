// ABOUTME: Tests for provider helper functions
// ABOUTME: Verifies provider extraction and response creation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(missing_docs)]

use pierre_mcp_server::protocols::universal::handlers::provider_helpers::{
    create_no_token_response, extract_provider,
};

#[test]
fn test_extract_provider_with_value() {
    let mut params = serde_json::Map::new();
    params.insert("provider".to_owned(), serde_json::json!("garmin"));
    assert_eq!(extract_provider(&params), "garmin");
}

#[test]
fn test_extract_provider_default() {
    let params = serde_json::Map::new();
    // Default is "synthetic" unless PIERRE_DEFAULT_PROVIDER is set
    let result = extract_provider(&params);
    assert!(!result.is_empty());
}

#[test]
fn test_no_token_response() {
    let response = create_no_token_response("strava");
    assert!(!response.success);
    assert!(response
        .error
        .as_ref()
        .is_some_and(|e| e.contains("strava")));
}
