// ABOUTME: Transaction management with RAII guards and retry patterns for database operations.
// ABOUTME: Provides automatic rollback on drop and exponential backoff for PostgreSQL and SQLite.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Transaction management with RAII guards and retry patterns
//!
//! This module provides:
//! - `TransactionGuard`: RAII wrapper ensuring automatic rollback if not committed
//! - `retry_transaction`: Exponential backoff for deadlock and timeout recovery
//!
//! ## RAII Transaction Guard
//!
//! The `TransactionGuard` ensures database transactions are properly handled:
//! - Automatic rollback on drop if not explicitly committed
//! - Type-safe commit that consumes the guard
//! - Works with both SQLite and PostgreSQL via SQLx generics
//!
//! ## Example Usage
//!
//! ```text
//! use pierre_mcp_server::database_plugins::shared::transactions::TransactionGuard;
//!
//! async fn create_user_with_profile(pool: &SqlitePool) -> AppResult<()> {
//!     let tx = pool.begin().await?;
//!     let mut guard = TransactionGuard::new(tx);
//!
//!     // Multiple database operations
//!     sqlx::query("INSERT INTO users ...").execute(guard.executor()?).await?;
//!     sqlx::query("INSERT INTO profiles ...").execute(guard.executor()?).await?;
//!
//!     // Explicit commit - if this line isn't reached, transaction rolls back
//!     guard.commit().await?;
//!     Ok(())
//! }
//! ```
//!
//! ## Combining with Retry Logic
//!
//! For operations that may encounter transient failures (deadlocks, timeouts),
//! combine `TransactionGuard` with `retry_transaction`:
//!
//! ```text
//! use pierre_mcp_server::database_plugins::shared::transactions::{
//!     TransactionGuard, retry_transaction
//! };
//!
//! async fn create_user_with_retry(pool: &SqlitePool, user: &User) -> AppResult<String> {
//!     retry_transaction(|| async {
//!         let tx = pool.begin().await?;
//!         let mut guard = TransactionGuard::new(tx);
//!
//!         let user_id = sqlx::query("INSERT INTO users ...")
//!             .execute(guard.executor()?)
//!             .await?;
//!         sqlx::query("INSERT INTO profiles ...")
//!             .execute(guard.executor()?)
//!             .await?;
//!
//!         guard.commit().await?;
//!         Ok(user_id)
//!     }, 3).await
//! }
//! ```

use std::future::Future;
use std::time::Duration;

use sqlx::{Database, Transaction};
use tokio::time::sleep;
use tracing::{debug, error, warn};

use crate::errors::{AppError, AppResult};

/// Retry a transaction operation if it fails due to deadlock or timeout
///
/// This function implements exponential backoff for retryable database errors:
/// - `SQLite`: "database is locked" errors (database-level locking)
/// - `PostgreSQL`: Deadlock detection errors (row-level locking)
/// - Both: Connection timeout errors
///
/// Non-retryable errors (constraint violations, invalid data, etc.) are
/// propagated immediately without retry.
///
/// # Arguments
/// * `f` - Async closure that performs the database operation
/// * `max_retries` - Maximum number of retry attempts (typically 3-5)
///
/// # Returns
/// * `Ok(T)` - Operation succeeded
///
/// # Errors
/// * Returns error if operation failed after max retries or non-retryable error
///
/// # Exponential Backoff
/// - Attempt 1: 10ms
/// - Attempt 2: 20ms
/// - Attempt 3: 40ms
/// - Attempt 4: 80ms
/// - Attempt 5: 160ms
///
/// # Examples
/// ```text
/// use pierre_mcp_server::database_plugins::shared::transactions::retry_transaction;
///
/// let result = retry_transaction(
///     || async {
///         // Database operation that might deadlock
///         db.create_user(&user).await
///     },
///     3 // max retries
/// ).await?;
/// ```
///
/// # Use Cases
/// - Concurrent user creation (username unique constraint contention)
/// - OAuth token updates (same user, multiple devices)
/// - A2A task status updates (worker contention)
/// - `SQLite` write operations under load
pub async fn retry_transaction<F, Fut, T>(mut f: F, max_retries: u32) -> AppResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = AppResult<T>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    error!(
                        attempts = attempts,
                        max_retries = max_retries,
                        error = %e,
                        "Transaction failed after max retries"
                    );
                    return Err(e);
                }

                // Check if error is retryable (deadlock, database locked, timeout)
                let error_msg = format!("{e:?}");
                if is_retryable_error(&error_msg) {
                    // Exponential backoff: 10ms, 20ms, 40ms, 80ms, 160ms, ...
                    let backoff_ms = 10 * (1 << attempts);
                    warn!(
                        attempt = attempts,
                        max_retries = max_retries,
                        backoff_ms = backoff_ms,
                        error = %e,
                        "Transaction failed with retryable error, retrying after backoff"
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                } else {
                    // Non-retryable error (e.g., constraint violation, invalid data)
                    error!(
                        attempts = attempts,
                        error = %e,
                        "Transaction failed with non-retryable error"
                    );
                    return Err(e);
                }
            }
        }
    }
}

/// Check if a database error is retryable
///
/// Retryable errors are transient and may succeed on retry:
/// - Deadlock detection (`PostgreSQL`)
/// - Database locked (`SQLite`)
/// - Connection timeout
/// - Busy timeout
///
/// Non-retryable errors indicate fundamental issues:
/// - Constraint violations (UNIQUE, FOREIGN KEY, CHECK)
/// - Invalid data/type errors
/// - Permission errors
/// - Connection refused (server down)
fn is_retryable_error(error_msg: &str) -> bool {
    let error_lower = error_msg.to_lowercase();

    // Retryable: Deadlock and locking errors
    if error_lower.contains("deadlock")
        || error_lower.contains("database is locked")
        || error_lower.contains("locked")
        || error_lower.contains("busy")
    {
        return true;
    }

    // Retryable: Timeout errors
    if error_lower.contains("timeout") || error_lower.contains("timed out") {
        return true;
    }

    // Retryable: Serialization failures (PostgreSQL)
    if error_lower.contains("serialization failure") || error_lower.contains("could not serialize")
    {
        return true;
    }

    // Non-retryable: Constraint violations
    if error_lower.contains("unique constraint")
        || error_lower.contains("foreign key constraint")
        || error_lower.contains("check constraint")
        || error_lower.contains("not null constraint")
    {
        return false;
    }

    // Non-retryable: Connection/permission errors
    if error_lower.contains("connection refused")
        || error_lower.contains("permission denied")
        || error_lower.contains("authentication failed")
    {
        return false;
    }

    // Default: Non-retryable (conservative - don't retry unknown errors)
    false
}

/// RAII guard for database transactions ensuring automatic rollback on drop
///
/// This guard wraps a `SQLx` `Transaction` and provides:
/// - Automatic rollback if the guard is dropped without calling `commit()`
/// - Type-safe commit that consumes the guard (prevents double-commit)
/// - `Deref`/`DerefMut` for transparent access to the underlying transaction
///
/// # Usage Pattern
///
/// ```text
/// let tx = pool.begin().await?;
/// let mut guard = TransactionGuard::new(tx);
///
/// // Perform multiple operations
/// sqlx::query("INSERT INTO table1 ...").execute(guard.as_mut()).await?;
/// sqlx::query("INSERT INTO table2 ...").execute(guard.as_mut()).await?;
///
/// // Explicit commit - transaction is committed and guard is consumed
/// guard.commit().await?;
/// ```
///
/// If an error occurs before `commit()`, the guard is dropped and the
/// transaction is automatically rolled back by `SQLx`.
///
/// # Type Parameters
///
/// * `DB` - The database type (e.g., `Sqlite`, `Postgres`)
pub struct TransactionGuard<'c, DB: Database> {
    transaction: Option<Transaction<'c, DB>>,
    committed: bool,
}

impl<'c, DB: Database> TransactionGuard<'c, DB> {
    /// Create a new transaction guard from an existing `SQLx` transaction
    ///
    /// # Arguments
    ///
    /// * `transaction` - A `SQLx` transaction obtained from `pool.begin().await`
    ///
    /// # Example
    ///
    /// ```text
    /// let tx = pool.begin().await?;
    /// let guard = TransactionGuard::new(tx);
    /// ```
    #[must_use]
    pub fn new(transaction: Transaction<'c, DB>) -> Self {
        debug!("TransactionGuard created - transaction will auto-rollback if not committed");
        Self {
            transaction: Some(transaction),
            committed: false,
        }
    }

    /// Commit the transaction and consume the guard
    ///
    /// This method commits the transaction to the database and consumes the guard,
    /// preventing any further operations on the transaction.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The transaction was already committed or rolled back
    /// - The database commit operation fails
    ///
    /// # Example
    ///
    /// ```text
    /// guard.commit().await?; // Transaction committed, guard consumed
    /// // guard is no longer accessible here
    /// ```
    pub async fn commit(mut self) -> AppResult<()> {
        match self.transaction.take() {
            Some(tx) => {
                tx.commit()
                    .await
                    .map_err(|e| AppError::database(format!("Transaction commit failed: {e}")))?;
                self.committed = true;
                debug!("TransactionGuard committed successfully");
                Ok(())
            }
            None => {
                // Transaction was already taken (should not happen in normal usage)
                Err(AppError::internal(
                    "Transaction already consumed - cannot commit",
                ))
            }
        }
    }

    /// Explicitly rollback the transaction and consume the guard
    ///
    /// While dropping the guard without committing will also rollback,
    /// this method allows explicit rollback with error handling.
    ///
    /// # Errors
    ///
    /// Returns an error if the rollback operation fails
    ///
    /// # Example
    ///
    /// ```text
    /// if validation_failed {
    ///     guard.rollback().await?; // Explicit rollback
    ///     return Err(validation_error);
    /// }
    /// ```
    pub async fn rollback(mut self) -> AppResult<()> {
        match self.transaction.take() {
            Some(tx) => {
                tx.rollback()
                    .await
                    .map_err(|e| AppError::database(format!("Transaction rollback failed: {e}")))?;
                debug!("TransactionGuard rolled back explicitly");
                Ok(())
            }
            None => {
                // Transaction was already taken
                Err(AppError::internal(
                    "Transaction already consumed - cannot rollback",
                ))
            }
        }
    }

    /// Check if the transaction has been committed
    #[must_use]
    pub const fn is_committed(&self) -> bool {
        self.committed
    }

    /// Get a mutable reference to the underlying connection for executing queries
    ///
    /// This is the primary way to execute queries within the transaction:
    ///
    /// ```text
    /// sqlx::query("INSERT INTO ...").execute(guard.executor()?).await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the transaction has already been committed or rolled back.
    /// This indicates a programming error where the guard is used after being consumed.
    pub fn executor(&mut self) -> AppResult<&mut <DB as Database>::Connection> {
        self.transaction.as_deref_mut().ok_or_else(|| {
            AppError::internal("Transaction already consumed - guard used after commit/rollback")
        })
    }
}

impl<DB: Database> Drop for TransactionGuard<'_, DB> {
    fn drop(&mut self) {
        if self.transaction.is_some() && !self.committed {
            // Transaction was not committed - SQLx will automatically rollback
            // when the Transaction is dropped, but we log it for observability
            warn!(
                "TransactionGuard dropped without commit - transaction will be rolled back automatically"
            );
        }
    }
}

/// Type alias for `SQLite` transaction guard
pub type SqliteTransactionGuard<'c> = TransactionGuard<'c, sqlx::Sqlite>;

/// Type alias for `PostgreSQL` transaction guard
#[cfg(feature = "postgresql")]
pub type PostgresTransactionGuard<'c> = TransactionGuard<'c, sqlx::Postgres>;
