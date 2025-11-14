//! Transaction retry patterns for database operations
//!
//! Handles deadlocks, timeouts, and exponential backoff for both PostgreSQL
//! and SQLite database operations. This module eliminates duplicate retry
//! logic across database implementations.
//!
//! Licensed under either of Apache License, Version 2.0 or MIT License at your option.
//! Copyright ©2025 Async-IO.org

use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

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
/// ```ignore
/// use crate::database_plugins::shared::transactions::retry_transaction;
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
pub async fn retry_transaction<F, Fut, T>(mut f: F, max_retries: u32) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    tracing::error!(
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
                    tracing::warn!(
                        attempt = attempts,
                        max_retries = max_retries,
                        backoff_ms = backoff_ms,
                        error = %e,
                        "Transaction failed with retryable error, retrying after backoff"
                    );
                    sleep(Duration::from_millis(backoff_ms)).await;
                } else {
                    // Non-retryable error (e.g., constraint violation, invalid data)
                    tracing::error!(
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
