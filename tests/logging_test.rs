// ABOUTME: Unit tests for logging functionality
// ABOUTME: Validates logging behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Integration tests for logging.rs module
// Tests for logging configuration and environment variable handling

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::logging::{LogFormat, LoggingConfig};
use std::env;

#[test]
fn test_logging_config_from_env() {
    // Set test environment variables
    env::set_var("RUST_LOG", "debug");
    env::set_var("LOG_FORMAT", "json");
    env::set_var("ENVIRONMENT", "production");
    env::set_var("SERVICE_NAME", "test-service");

    let config = LoggingConfig::from_env();

    assert_eq!(config.level, "debug");
    assert!(matches!(config.format, LogFormat::Json));
    assert_eq!(config.environment, "production");
    assert_eq!(config.service_name, "test-service");
    assert!(config.output.location); // Should be true for production

    // Clean up
    env::remove_var("RUST_LOG");
    env::remove_var("LOG_FORMAT");
    env::remove_var("ENVIRONMENT");
    env::remove_var("SERVICE_NAME");
}

#[test]
fn test_default_logging_config() {
    let config = LoggingConfig::default();

    assert_eq!(config.level, "info");
    assert!(matches!(config.format, LogFormat::Pretty));
    assert_eq!(config.environment, "development");
    assert_eq!(config.service_name, "pierre-mcp-server");
    assert!(!config.output.location); // Should be false for development
}
