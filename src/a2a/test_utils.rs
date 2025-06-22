//! Test utilities for A2A module
//!
//! Common functions to reduce code duplication in A2A tests

use crate::database_plugins::factory::Database;
use std::sync::Arc;

#[cfg(test)]
/// Create test database for A2A tests
pub async fn create_test_database() -> Arc<Database> {
    let database = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");
    Arc::new(database)
}

#[cfg(test)]
/// Create test database with custom encryption key
pub async fn create_test_database_with_key(key: Vec<u8>) -> Arc<Database> {
    let database = Database::new("sqlite::memory:", key)
        .await
        .expect("Failed to create test database with custom key");
    Arc::new(database)
}
