// ABOUTME: Test utilities and helper functions for A2A module testing
// ABOUTME: Provides common test setup, client creation, and database utilities for A2A tests
//! Test utilities for A2A module
//!
//! Common functions to reduce code duplication in A2A tests

use crate::database_plugins::factory::Database;
use std::sync::Arc;

#[cfg(test)]
/// Create test database for A2A tests
///
/// # Panics
///
/// This function will panic if the database creation fails.
pub async fn create_test_database() -> Arc<Database> {
    let database = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");
    Arc::new(database)
}

#[cfg(test)]
/// Create test database with custom encryption key
///
/// # Panics
///
/// This function will panic if the database creation fails.
pub async fn create_test_database_with_key(key: Vec<u8>) -> Arc<Database> {
    let database = Database::new("sqlite::memory:", key)
        .await
        .expect("Failed to create test database with custom key");
    Arc::new(database)
}
