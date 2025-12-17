// ABOUTME: Unit tests for TransactionGuard RAII wrapper
// ABOUTME: Validates auto-rollback behavior, commit semantics, and retry patterns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use std::sync::atomic::{AtomicU32, Ordering};

use pierre_mcp_server::database_plugins::shared::transactions::{
    retry_transaction, SqliteTransactionGuard,
};
use pierre_mcp_server::errors::AppError;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::Row;

/// Create a test `SQLite` pool with a simple table for testing
async fn create_test_pool() -> sqlx::SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test pool");

    // Create a simple test table
    sqlx::query(
        r"CREATE TABLE IF NOT EXISTS test_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            value INTEGER NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create test table");

    pool
}

/// Count rows in the `test_items` table
async fn count_items(pool: &sqlx::SqlitePool) -> i64 {
    let row = sqlx::query("SELECT COUNT(*) as count FROM test_items")
        .fetch_one(pool)
        .await
        .expect("Failed to count items");
    row.get::<i64, _>("count")
}

#[tokio::test]
async fn test_transaction_guard_commit_persists_changes() {
    let pool = create_test_pool().await;

    // Start a transaction and insert data
    let tx = pool.begin().await.expect("Failed to begin transaction");
    let mut guard = SqliteTransactionGuard::new(tx);

    sqlx::query("INSERT INTO test_items (name, value) VALUES ('item1', 100)")
        .execute(guard.executor().expect("Guard should have executor"))
        .await
        .expect("Failed to insert");

    // Commit the transaction
    guard.commit().await.expect("Commit should succeed");

    // Verify data was persisted
    assert_eq!(count_items(&pool).await, 1);
}

#[tokio::test]
async fn test_transaction_guard_drop_without_commit_rolls_back() {
    let pool = create_test_pool().await;

    // Start a transaction and insert data, but don't commit
    {
        let tx = pool.begin().await.expect("Failed to begin transaction");
        let mut guard = SqliteTransactionGuard::new(tx);

        sqlx::query("INSERT INTO test_items (name, value) VALUES ('item2', 200)")
            .execute(guard.executor().expect("Guard should have executor"))
            .await
            .expect("Failed to insert");

        // Guard dropped here without commit - should rollback
    }

    // Verify data was NOT persisted (rolled back)
    assert_eq!(count_items(&pool).await, 0);
}

#[tokio::test]
async fn test_transaction_guard_explicit_rollback() {
    let pool = create_test_pool().await;

    let tx = pool.begin().await.expect("Failed to begin transaction");
    let mut guard = SqliteTransactionGuard::new(tx);

    sqlx::query("INSERT INTO test_items (name, value) VALUES ('item3', 300)")
        .execute(guard.executor().expect("Guard should have executor"))
        .await
        .expect("Failed to insert");

    // Explicit rollback
    guard.rollback().await.expect("Rollback should succeed");

    // Verify data was NOT persisted
    assert_eq!(count_items(&pool).await, 0);
}

#[tokio::test]
async fn test_transaction_guard_is_committed_before_commit() {
    let pool = create_test_pool().await;

    let tx = pool.begin().await.expect("Failed to begin transaction");
    let guard = SqliteTransactionGuard::new(tx);

    // Before commit, is_committed should be false
    assert!(!guard.is_committed());

    // Commit consumes self, so we can't check afterwards - just verify commit succeeds
    guard.commit().await.expect("Commit should succeed");
}

#[tokio::test]
async fn test_transaction_guard_multiple_operations() {
    let pool = create_test_pool().await;

    let tx = pool.begin().await.expect("Failed to begin transaction");
    let mut guard = SqliteTransactionGuard::new(tx);

    // Multiple inserts in same transaction
    sqlx::query("INSERT INTO test_items (name, value) VALUES ('a', 1)")
        .execute(guard.executor().expect("Guard should have executor"))
        .await
        .expect("Failed to insert");

    sqlx::query("INSERT INTO test_items (name, value) VALUES ('b', 2)")
        .execute(guard.executor().expect("Guard should have executor"))
        .await
        .expect("Failed to insert");

    sqlx::query("INSERT INTO test_items (name, value) VALUES ('c', 3)")
        .execute(guard.executor().expect("Guard should have executor"))
        .await
        .expect("Failed to insert");

    guard.commit().await.expect("Commit should succeed");

    // All three items should be persisted
    assert_eq!(count_items(&pool).await, 3);
}

#[tokio::test]
async fn test_transaction_guard_error_causes_rollback() {
    let pool = create_test_pool().await;

    // Insert one item first
    sqlx::query("INSERT INTO test_items (name, value) VALUES ('existing', 999)")
        .execute(&pool)
        .await
        .expect("Failed to insert initial item");

    assert_eq!(count_items(&pool).await, 1);

    // Try a transaction that will fail in the middle
    let result: Result<(), AppError> = async {
        let tx = pool
            .begin()
            .await
            .map_err(|e| AppError::database(e.to_string()))?;
        let mut guard = SqliteTransactionGuard::new(tx);

        sqlx::query("INSERT INTO test_items (name, value) VALUES ('new_item', 100)")
            .execute(guard.executor()?)
            .await
            .map_err(|e| AppError::database(e.to_string()))?;

        // Simulate an error condition
        return Err(AppError::internal("Simulated business logic error"));

        // This unreachable code would have committed
        #[allow(unreachable_code)]
        {
            guard.commit().await?;
            Ok(())
        }
    }
    .await;

    // The transaction should have failed
    assert!(result.is_err());

    // Only the original item should exist (new insert rolled back)
    assert_eq!(count_items(&pool).await, 1);
}

#[tokio::test]
async fn test_retry_transaction_succeeds_first_try() {
    let pool = create_test_pool().await;

    let result = retry_transaction(
        || async {
            sqlx::query("INSERT INTO test_items (name, value) VALUES ('retry_test', 42)")
                .execute(&pool)
                .await
                .map_err(|e| AppError::database(e.to_string()))?;
            Ok("success")
        },
        3,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");
    assert_eq!(count_items(&pool).await, 1);
}

#[tokio::test]
async fn test_retry_transaction_non_retryable_error_fails_immediately() {
    let pool = create_test_pool().await;

    // Create a unique constraint to trigger a non-retryable error
    sqlx::query(
        r"CREATE TABLE IF NOT EXISTS unique_test (
            id INTEGER PRIMARY KEY,
            unique_name TEXT UNIQUE NOT NULL
        )",
    )
    .execute(&pool)
    .await
    .expect("Failed to create unique_test table");

    // Insert initial row
    sqlx::query("INSERT INTO unique_test (id, unique_name) VALUES (1, 'duplicate')")
        .execute(&pool)
        .await
        .expect("Failed to insert initial row");

    let attempt_count = AtomicU32::new(0);

    let result = retry_transaction(
        || {
            attempt_count.fetch_add(1, Ordering::SeqCst);
            async {
                // This will fail with a UNIQUE constraint violation
                sqlx::query("INSERT INTO unique_test (id, unique_name) VALUES (2, 'duplicate')")
                    .execute(&pool)
                    .await
                    .map_err(|e| AppError::database(e.to_string()))?;
                Ok(())
            }
        },
        5,
    )
    .await;

    // Should fail without retrying (constraint violation is not retryable)
    assert!(result.is_err());
    // Should only attempt once since constraint violations are not retryable
    assert_eq!(
        attempt_count.load(Ordering::SeqCst),
        1,
        "Non-retryable errors should not be retried"
    );
}

#[tokio::test]
async fn test_transaction_guard_with_retry_pattern() {
    let pool = create_test_pool().await;

    let result = retry_transaction(
        || {
            let pool = pool.clone();
            async move {
                let tx = pool
                    .begin()
                    .await
                    .map_err(|e| AppError::database(e.to_string()))?;
                let mut guard = SqliteTransactionGuard::new(tx);

                sqlx::query("INSERT INTO test_items (name, value) VALUES ('guarded', 777)")
                    .execute(guard.executor()?)
                    .await
                    .map_err(|e| AppError::database(e.to_string()))?;

                sqlx::query("INSERT INTO test_items (name, value) VALUES ('guarded2', 888)")
                    .execute(guard.executor()?)
                    .await
                    .map_err(|e| AppError::database(e.to_string()))?;

                guard.commit().await?;
                Ok(2_i32)
            }
        },
        3,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 2);
    assert_eq!(count_items(&pool).await, 2);
}
