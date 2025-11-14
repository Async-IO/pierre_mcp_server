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
/// - SQLite: "database is locked" errors (database-level locking)
/// - PostgreSQL: Deadlock detection errors (row-level locking)
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
/// * `Err` - Operation failed after max retries or non-retryable error
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
/// - SQLite write operations under load
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
                let error_msg = format!("{:?}", e);
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
/// - Deadlock detection (PostgreSQL)
/// - Database locked (SQLite)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_retryable_deadlock() {
        assert!(is_retryable_error("deadlock detected"));
        assert!(is_retryable_error("DEADLOCK"));
    }

    #[test]
    fn test_is_retryable_locked() {
        assert!(is_retryable_error("database is locked"));
        assert!(is_retryable_error("Database Locked"));
        assert!(is_retryable_error("table is locked"));
        assert!(is_retryable_error("busy timeout"));
    }

    #[test]
    fn test_is_retryable_timeout() {
        assert!(is_retryable_error("connection timeout"));
        assert!(is_retryable_error("operation timed out"));
        assert!(is_retryable_error("TIMEOUT"));
    }

    #[test]
    fn test_is_retryable_serialization() {
        assert!(is_retryable_error("serialization failure"));
        assert!(is_retryable_error("could not serialize access"));
    }

    #[test]
    fn test_is_not_retryable_constraint() {
        assert!(!is_retryable_error("unique constraint violation"));
        assert!(!is_retryable_error("foreign key constraint failed"));
        assert!(!is_retryable_error("check constraint violation"));
        assert!(!is_retryable_error("not null constraint"));
    }

    #[test]
    fn test_is_not_retryable_connection() {
        assert!(!is_retryable_error("connection refused"));
        assert!(!is_retryable_error("permission denied"));
        assert!(!is_retryable_error("authentication failed"));
    }

    #[test]
    fn test_is_not_retryable_unknown() {
        assert!(!is_retryable_error("some random error"));
        assert!(!is_retryable_error("invalid input"));
    }

    #[tokio::test]
    async fn test_retry_transaction_succeeds_first_try() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = retry_transaction(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, anyhow::Error>(42)
                }
            },
            3,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_transaction_succeeds_after_retry() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = retry_transaction(
            move || {
                let count = call_count_clone.clone();
                async move {
                    let current = count.fetch_add(1, Ordering::SeqCst) + 1;
                    if current < 3 {
                        Err(anyhow::anyhow!("database is locked"))
                    } else {
                        Ok::<i32, anyhow::Error>(42)
                    }
                }
            },
            5,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_transaction_fails_after_max_retries() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = retry_transaction(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, anyhow::Error>(anyhow::anyhow!("deadlock detected"))
                }
            },
            3,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 3);
        assert!(result.unwrap_err().to_string().contains("deadlock"));
    }

    #[tokio::test]
    async fn test_retry_transaction_non_retryable_fails_immediately() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        let result = retry_transaction(
            move || {
                let count = call_count_clone.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, anyhow::Error>(anyhow::anyhow!("unique constraint violation"))
                }
            },
            5,
        )
        .await;

        assert!(result.is_err());
        assert_eq!(call_count.load(Ordering::SeqCst), 1); // Should not retry
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("unique constraint"));
    }
}
