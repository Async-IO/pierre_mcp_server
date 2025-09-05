// Integration tests for health.rs module
// Tests for health check functionality and system monitoring

use pierre_mcp_server::{
    database::generate_encryption_key,
    database_plugins::factory::Database,
    health::{HealthChecker, HealthStatus},
};
use std::sync::Arc;

#[tokio::test]
async fn test_basic_health_check() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let health_checker = HealthChecker::new(Arc::new(database));

    let response = health_checker.basic_health();

    assert_eq!(response.status, HealthStatus::Healthy);
    assert_eq!(response.service.name, "pierre-mcp-server");
    assert!(!response.checks.is_empty());
}

#[tokio::test]
async fn test_comprehensive_health_check() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let health_checker = HealthChecker::new(Arc::new(database));

    let response = health_checker.comprehensive_health().await;

    // Should have multiple checks
    assert!(response.checks.len() > 1);

    // Should include database check
    assert!(response.checks.iter().any(|c| c.name == "database"));
}

#[tokio::test]
async fn test_readiness_check() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let health_checker = HealthChecker::new(Arc::new(database));

    let response = health_checker.readiness().await;

    // Should include database check for readiness
    assert!(response.checks.iter().any(|c| c.name == "database"));
}
