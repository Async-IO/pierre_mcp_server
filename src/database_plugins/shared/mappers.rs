// ABOUTME: Model to SQL row conversion helpers for database operations.
// ABOUTME: Provides generic row parsing functions for PostgreSQL and SQLite backends.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Model ↔ SQL row conversion helpers
//!
//! This module provides generic database row parsing functions that work with both
//! PostgreSQL and SQLite, eliminating duplicate row parsing logic.

use crate::a2a::protocol::A2ATask;
use crate::errors::{AppError, AppResult};
use crate::models::User;
use chrono::{DateTime, Utc};
use serde_json::Value;
use tracing::warn;
use uuid::Uuid;

/// Parse User from database row (database-agnostic)
///
/// Works with both `PostgreSQL` (`PgRow`) and `SQLite` (`SqliteRow`) using generic trait bounds.
///
/// # Arguments
/// * `row` - Database row implementing `sqlx::Row` trait
///
/// # Returns
/// * `Ok(User)` if parsing succeeds
///
/// # Errors
/// * Returns error if required fields are missing or have invalid types
///
/// # Examples
/// ```text
/// // PostgreSQL usage:
/// let user = shared::mappers::parse_user_from_row(&pg_row)?;
///
/// // SQLite usage:
/// let user = shared::mappers::parse_user_from_row(&sqlite_row)?;
/// ```
pub fn parse_user_from_row<R>(row: &R) -> AppResult<User>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    for<'a> usize: sqlx::ColumnIndex<R>,
    Uuid: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<String>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<Uuid>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    bool: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<DateTime<Utc>>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    // Parse enum fields using shared converters
    let user_status_str: String = row
        .try_get("user_status")
        .map_err(|e| AppError::database(format!("Failed to get column 'user_status': {e}")))?;
    let user_status = super::enums::str_to_user_status(&user_status_str);

    let tier_str: String = row
        .try_get("tier")
        .map_err(|e| AppError::database(format!("Failed to get column 'tier': {e}")))?;
    let tier = super::enums::str_to_user_tier(&tier_str);

    // Parse role - default to 'user' if not present (backward compatibility)
    let role = row
        .try_get::<String, _>("role")
        .map(|role_str| super::enums::str_to_user_role(&role_str))
        .unwrap_or(crate::permissions::UserRole::User);

    Ok(User {
        id: row
            .try_get("id")
            .map_err(|e| AppError::database(format!("Failed to get column 'id': {e}")))?,
        email: row
            .try_get("email")
            .map_err(|e| AppError::database(format!("Failed to get column 'email': {e}")))?,
        display_name: row
            .try_get("display_name")
            .map_err(|e| AppError::database(format!("Failed to get column 'display_name': {e}")))?,
        password_hash: row.try_get("password_hash").map_err(|e| {
            AppError::database(format!("Failed to get column 'password_hash': {e}"))
        })?,
        tier,
        tenant_id: row
            .try_get("tenant_id")
            .map_err(|e| AppError::database(format!("Failed to get column 'tenant_id': {e}")))?,
        strava_token: None, // Loaded separately via user_oauth_tokens
        fitbit_token: None,
        is_active: row
            .try_get("is_active")
            .map_err(|e| AppError::database(format!("Failed to get column 'is_active': {e}")))?,
        user_status,
        is_admin: row.try_get("is_admin").unwrap_or(false),
        role,
        approved_by: row
            .try_get("approved_by")
            .map_err(|e| AppError::database(format!("Failed to get column 'approved_by': {e}")))?,
        approved_at: row
            .try_get("approved_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'approved_at': {e}")))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'created_at': {e}")))?,
        last_active: row
            .try_get("last_active")
            .map_err(|e| AppError::database(format!("Failed to get column 'last_active': {e}")))?,
        // Firebase fields - default to None/"email" for backward compatibility
        firebase_uid: row.try_get("firebase_uid").ok().flatten(),
        auth_provider: row
            .try_get("auth_provider")
            .unwrap_or_else(|_| "email".to_owned()),
    })
}

/// Parse A2A Task from database row (database-agnostic)
///
/// Works with both `PostgreSQL` and `SQLite`. Handles JSON deserialization of
/// `input_data` and `result_data` with fallback to null/None on parse errors.
///
/// # Arguments
/// * `row` - Database row implementing `sqlx::Row` trait
///
/// # Returns
/// * `Ok(A2ATask)` if parsing succeeds
///
/// # Errors
/// * Returns error if required fields are missing or have invalid types
///
/// # Note
/// JSON deserialization errors for `input_data`/`result_data` are logged but don't fail
/// the parse (returns null/None instead).
pub fn parse_a2a_task_from_row<R>(row: &R) -> AppResult<A2ATask>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    for<'a> usize: sqlx::ColumnIndex<R>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<String>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<DateTime<Utc>>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    // Get task_id for logging
    let task_id: String = row
        .try_get("task_id")
        .or_else(|_| row.try_get("id"))
        .map_err(|e| AppError::database(format!("Failed to get task_id or id: {e}")))?; // Try both column names

    // Parse input_data JSON with fallback to null
    let input_str: String = row
        .try_get("input_data")
        .map_err(|e| AppError::database(format!("Failed to get column 'input_data': {e}")))?;
    let input_data: Value = serde_json::from_str(&input_str).unwrap_or_else(|e| {
        warn!(
            task_id = %task_id,
            error = %e,
            "Failed to deserialize A2A task input_data, using null"
        );
        Value::Null
    });

    // Parse result_data JSON (optional) with fallback to None
    let result_data = row
        .try_get::<Option<String>, _>("output_data") // Column is "output_data" not "result_data"
        .map_or(None, |result_str| {
            result_str.and_then(|s| {
                serde_json::from_str(&s)
                    .inspect_err(|e| {
                        warn!(
                            task_id = %task_id,
                            error = %e,
                            "Failed to deserialize A2A task output_data"
                        );
                    })
                    .ok()
            })
        });

    // Parse status using shared enum converter
    let status_str: String = row
        .try_get("status")
        .map_err(|e| AppError::database(format!("Failed to get column 'status': {e}")))?;
    let status = super::enums::str_to_task_status(&status_str);

    Ok(A2ATask {
        id: task_id,
        status,
        created_at: row
            .try_get("created_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'created_at': {e}")))?,
        completed_at: row.try_get("updated_at").ok(),
        result: result_data.clone(), // Safe: JSON value ownership for A2ATask struct
        error: row.try_get("method").ok(),
        client_id: row
            .try_get("client_id")
            .unwrap_or_else(|_| "unknown".into()),
        task_type: row
            .try_get("task_type")
            .map_err(|e| AppError::database(format!("Failed to get column 'task_type': {e}")))?,
        input_data,
        output_data: result_data,
        error_message: row.try_get("method").ok(),
        updated_at: row
            .try_get("updated_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'updated_at': {e}")))?,
    })
}

/// Helper to extract UUID from row (handles `PostgreSQL` UUID vs `SQLite` TEXT)
///
/// `PostgreSQL` stores UUIDs as a native type, while `SQLite` stores them as TEXT.
/// This helper tries the native UUID type first, then falls back to parsing a string.
///
/// # Arguments
/// * `row` - Database row
/// * `column` - Column name containing the UUID
///
/// # Returns
/// * `Ok(Uuid)` if extraction/parsing succeeds
///
/// # Errors
/// * Returns error if column is missing or value is invalid
///
/// # Examples
/// ```text
/// // Works with both:
/// let user_id = get_uuid_from_row(&pg_row, "id")?;      // PostgreSQL UUID
/// let user_id = get_uuid_from_row(&sqlite_row, "id")?;  // SQLite TEXT -> parsed
/// ```
pub fn get_uuid_from_row<R>(row: &R, column: &str) -> AppResult<Uuid>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    Uuid: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    // Try PostgreSQL UUID type first
    if let Ok(uuid) = row.try_get::<Uuid, _>(column) {
        return Ok(uuid);
    }

    // Fall back to SQLite TEXT (parse string)
    let uuid_str: String = row
        .try_get(column)
        .map_err(|e| AppError::database(format!("Failed to get column: {e}")))?;
    Ok(Uuid::parse_str(&uuid_str)?)
}
