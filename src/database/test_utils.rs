// ABOUTME: Test utilities for database operations and in-memory test database creation
// ABOUTME: Provides helper functions for creating isolated test database instances
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
use crate::database_plugins::factory::Database;
use crate::errors::AppResult;

/// Create a test database instance
///
/// # Errors
///
/// Returns an error if database initialization fails
pub async fn create_test_db() -> AppResult<Database> {
    // Use a simple in-memory database - each connection gets its own isolated instance
    let database_url = "sqlite::memory:";

    #[cfg(feature = "postgresql")]
    {
        Database::new(
            database_url,
            vec![0u8; 32],
            &crate::config::environment::PostgresPoolConfig::default(),
        )
        .await
    }

    #[cfg(not(feature = "postgresql"))]
    {
        Database::new(database_url, vec![0u8; 32]).await
    }
}
