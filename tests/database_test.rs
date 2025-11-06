// ABOUTME: Unit tests for database functionality
// ABOUTME: Validates database behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::database::Database;

/// Create a test database instance
///
/// # Errors
///
/// Returns an error if database initialization fails
pub async fn create_test_db() -> Result<Database> {
    // Use a simple in-memory database - each connection gets its own isolated instance
    let database_url = "sqlite::memory:";
    Database::new(database_url, vec![0u8; 32]).await
}
