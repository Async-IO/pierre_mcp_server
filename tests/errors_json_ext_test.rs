// ABOUTME: Tests for JSON-specific error handling extensions
// ABOUTME: Validates JsonResultExt trait and JSON error helper functions
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Integration tests for JSON error handling extensions.
//!
//! This module validates the `JsonResultExt` trait and helper functions
//! for creating JSON-specific error messages with proper context.

use pierre_mcp_server::errors::{AppError, ErrorCode, JsonResultExt};
use std::error::Error;

#[test]
fn test_json_parse_error() {
    let error = AppError::json_parse_error("test context", "malformed JSON");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("test context"));
    assert!(error.message.contains("malformed JSON"));
}

#[test]
fn test_missing_field() {
    let error = AppError::missing_field("user_id");
    assert_eq!(error.code, ErrorCode::MissingRequiredField);
    assert!(error.message.contains("user_id"));
}

#[test]
fn test_invalid_field() {
    let error = AppError::invalid_field("age", "must be positive");
    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert!(error.message.contains("age"));
    assert!(error.message.contains("must be positive"));
}

#[test]
fn test_json_result_ext() -> Result<(), String> {
    let json_str = r#"{"invalid": json}"#;
    let result: Result<serde_json::Value, _> = serde_json::from_str(json_str);

    match result.json_context("test parsing") {
        Err(error) => {
            assert_eq!(error.code, ErrorCode::InvalidInput);
            assert!(
                error.message.contains("test parsing"),
                "Error message should contain context"
            );
            Ok(())
        }
        Ok(_) => Err("Should fail to parse invalid JSON".to_owned()),
    }
}

#[test]
fn test_json_result_ext_with_valid_json() -> Result<(), Box<dyn Error>> {
    let json_str = r#"{"valid": "json"}"#;
    let result: Result<serde_json::Value, _> = serde_json::from_str(json_str);

    let value = result.json_context("valid json parsing")?;

    assert_eq!(
        value.get("valid").and_then(|v| v.as_str()),
        Some("json"),
        "Should parse valid JSON correctly"
    );

    Ok(())
}
