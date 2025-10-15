// ABOUTME: Unit tests for health functionality
// ABOUTME: Validates health behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Integration tests for health.rs module
// Tests for health check functionality and system monitoring

use pierre_mcp_server::{
    database::generate_encryption_key,
    database_plugins::factory::Database,
    health::{HealthChecker, HealthStatus},
};
use std::sync::{Arc, Once};

/// Ensure HTTP clients are initialized only once across all tests
static INIT_HTTP_CLIENTS: Once = Once::new();

fn ensure_http_clients_initialized() {
    INIT_HTTP_CLIENTS.call_once(|| {
        pierre_mcp_server::utils::http_client::initialize_http_clients(
            pierre_mcp_server::config::environment::HttpClientConfig::default(),
        );
    });
}

#[tokio::test]
async fn test_basic_health_check() {
    ensure_http_clients_initialized();
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let health_checker = HealthChecker::new(
        Arc::new(database),
        "https://www.strava.com/api/v3".to_string(),
    );

    let response = health_checker.basic_health();

    assert_eq!(response.status, HealthStatus::Healthy);
    assert_eq!(response.service.name, "pierre-mcp-server");
    assert!(!response.checks.is_empty());
}

#[tokio::test]
async fn test_comprehensive_health_check() {
    ensure_http_clients_initialized();
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let health_checker = HealthChecker::new(
        Arc::new(database),
        "https://www.strava.com/api/v3".to_string(),
    );

    let response = health_checker.comprehensive_health().await;

    // Should have multiple checks
    assert!(response.checks.len() > 1);

    // Should include database check
    assert!(response.checks.iter().any(|c| c.name == "database"));
}

#[tokio::test]
async fn test_readiness_check() {
    ensure_http_clients_initialized();
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let health_checker = HealthChecker::new(
        Arc::new(database),
        "https://www.strava.com/api/v3".to_string(),
    );

    let response = health_checker.readiness().await;

    // Should include database check for readiness
    assert!(response.checks.iter().any(|c| c.name == "database"));
}
