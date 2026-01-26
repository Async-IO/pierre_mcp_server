// ABOUTME: Model to SQL row conversion helpers for database operations.
// ABOUTME: Provides generic row parsing functions for PostgreSQL and SQLite backends.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Model ↔ SQL row conversion helpers
//!
//! This module provides generic database row parsing functions that work with both
//! PostgreSQL and SQLite, eliminating duplicate row parsing logic.

use crate::a2a::protocol::A2ATask;
use crate::admin::models::{AdminAction, AdminPermissions, AdminToken, AdminTokenUsage};
use crate::database::UserMcpToken;
use crate::errors::{AppError, AppResult};
use crate::models::User;
use crate::permissions::impersonation::ImpersonationSession;
use crate::permissions::UserRole;
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
        .unwrap_or(UserRole::User);

    // NOTE: tenant_id is no longer stored on User - use tenant_users junction table
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

/// Parse `UserMcpToken` from database row (database-agnostic)
///
/// Works with both `PostgreSQL` and `SQLite` backends.
///
/// # Errors
/// Returns error if required fields are missing or have invalid types.
pub fn parse_user_mcp_token_from_row<R>(row: &R) -> AppResult<UserMcpToken>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    i32: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    bool: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<DateTime<Utc>>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    let user_id_str: String = row
        .try_get("user_id")
        .map_err(|e| AppError::database(format!("Failed to get column 'user_id': {e}")))?;

    Ok(UserMcpToken {
        id: row
            .try_get("id")
            .map_err(|e| AppError::database(format!("Failed to get column 'id': {e}")))?,
        user_id: Uuid::parse_str(&user_id_str)
            .map_err(|e| AppError::internal(format!("Failed to parse user_id UUID: {e}")))?,
        name: row
            .try_get("name")
            .map_err(|e| AppError::database(format!("Failed to get column 'name': {e}")))?,
        token_hash: row
            .try_get("token_hash")
            .map_err(|e| AppError::database(format!("Failed to get column 'token_hash': {e}")))?,
        token_prefix: row
            .try_get("token_prefix")
            .map_err(|e| AppError::database(format!("Failed to get column 'token_prefix': {e}")))?,
        expires_at: row
            .try_get("expires_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'expires_at': {e}")))?,
        last_used_at: row
            .try_get("last_used_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'last_used_at': {e}")))?,
        usage_count: u32::try_from(
            row.try_get::<i32, _>("usage_count").map_err(|e| {
                AppError::database(format!("Failed to get column 'usage_count': {e}"))
            })?,
        )
        .map_err(|e| {
            AppError::internal(format!("Integer conversion failed for usage_count: {e}"))
        })?,
        is_revoked: row
            .try_get("is_revoked")
            .map_err(|e| AppError::database(format!("Failed to get column 'is_revoked': {e}")))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'created_at': {e}")))?,
    })
}

/// Parse `ImpersonationSession` from database row (database-agnostic)
///
/// Works with both `PostgreSQL` and `SQLite` backends.
///
/// # Errors
/// Returns error if required fields are missing or have invalid types.
pub fn parse_impersonation_session_from_row<R>(row: &R) -> AppResult<ImpersonationSession>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<String>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    bool: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<DateTime<Utc>>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    let id: String = row
        .try_get("id")
        .map_err(|e| AppError::database(format!("Failed to get column 'id': {e}")))?;
    let impersonator_id: String = row
        .try_get("impersonator_id")
        .map_err(|e| AppError::database(format!("Failed to get column 'impersonator_id': {e}")))?;
    let target_user_id: String = row
        .try_get("target_user_id")
        .map_err(|e| AppError::database(format!("Failed to get column 'target_user_id': {e}")))?;

    Ok(ImpersonationSession {
        id,
        impersonator_id: Uuid::parse_str(&impersonator_id)
            .map_err(|e| AppError::database(format!("Invalid impersonator_id UUID: {e}")))?,
        target_user_id: Uuid::parse_str(&target_user_id)
            .map_err(|e| AppError::database(format!("Invalid target_user_id UUID: {e}")))?,
        reason: row
            .try_get("reason")
            .map_err(|e| AppError::database(format!("Failed to get column 'reason': {e}")))?,
        started_at: row
            .try_get("started_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'started_at': {e}")))?,
        ended_at: row
            .try_get("ended_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'ended_at': {e}")))?,
        is_active: row
            .try_get("is_active")
            .map_err(|e| AppError::database(format!("Failed to get column 'is_active': {e}")))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'created_at': {e}")))?,
    })
}

/// Parse `AdminToken` from database row (database-agnostic)
///
/// Works with both `PostgreSQL` and `SQLite` backends.
///
/// # Errors
/// Returns error if required fields are missing or have invalid types.
pub fn parse_admin_token_from_row<R>(row: &R) -> AppResult<AdminToken>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<String>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    bool: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    i64: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<DateTime<Utc>>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    let permissions_json: String = row
        .try_get("permissions")
        .map_err(|e| AppError::database(format!("Failed to get column 'permissions': {e}")))?;
    let permissions = AdminPermissions::from_json(&permissions_json)?;

    Ok(AdminToken {
        id: row
            .try_get("id")
            .map_err(|e| AppError::database(format!("Failed to get column 'id': {e}")))?,
        service_name: row
            .try_get("service_name")
            .map_err(|e| AppError::database(format!("Failed to get column 'service_name': {e}")))?,
        service_description: row.try_get("service_description").map_err(|e| {
            AppError::database(format!("Failed to get column 'service_description': {e}"))
        })?,
        token_hash: row
            .try_get("token_hash")
            .map_err(|e| AppError::database(format!("Failed to get column 'token_hash': {e}")))?,
        token_prefix: row
            .try_get("token_prefix")
            .map_err(|e| AppError::database(format!("Failed to get column 'token_prefix': {e}")))?,
        jwt_secret_hash: row.try_get("jwt_secret_hash").map_err(|e| {
            AppError::database(format!("Failed to get column 'jwt_secret_hash': {e}"))
        })?,
        permissions,
        is_super_admin: row.try_get("is_super_admin").map_err(|e| {
            AppError::database(format!("Failed to get column 'is_super_admin': {e}"))
        })?,
        is_active: row
            .try_get("is_active")
            .map_err(|e| AppError::database(format!("Failed to get column 'is_active': {e}")))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'created_at': {e}")))?,
        expires_at: row
            .try_get("expires_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'expires_at': {e}")))?,
        last_used_at: row
            .try_get("last_used_at")
            .map_err(|e| AppError::database(format!("Failed to get column 'last_used_at': {e}")))?,
        last_used_ip: row
            .try_get("last_used_ip")
            .map_err(|e| AppError::database(format!("Failed to get column 'last_used_ip': {e}")))?,
        #[allow(clippy::cast_sign_loss)]
        usage_count: u64::try_from(
            row.try_get::<i64, _>("usage_count")
                .map_err(|e| {
                    AppError::database(format!("Failed to get column 'usage_count': {e}"))
                })?
                .max(0),
        )
        .unwrap_or(0),
    })
}

/// Parse `AdminTokenUsage` from database row (database-agnostic)
///
/// Works with both `PostgreSQL` and `SQLite` backends.
///
/// # Errors
/// Returns error if required fields are missing or have invalid types.
pub fn parse_admin_token_usage_from_row<R>(row: &R) -> AppResult<AdminTokenUsage>
where
    R: sqlx::Row,
    for<'a> &'a str: sqlx::ColumnIndex<R>,
    String: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<String>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    bool: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    i64: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    Option<i32>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
    DateTime<Utc>: for<'a> sqlx::Type<R::Database> + for<'a> sqlx::Decode<'a, R::Database>,
{
    let action_str: String = row
        .try_get("action")
        .map_err(|e| AppError::database(format!("Failed to get column 'action': {e}")))?;
    let action = action_str
        .parse::<AdminAction>()
        .unwrap_or(AdminAction::ProvisionKey);

    Ok(AdminTokenUsage {
        id: Some(
            row.try_get::<i64, _>("id")
                .map_err(|e| AppError::database(format!("Failed to get column 'id': {e}")))?,
        ),
        admin_token_id: row.try_get("admin_token_id").map_err(|e| {
            AppError::database(format!("Failed to get column 'admin_token_id': {e}"))
        })?,
        timestamp: row
            .try_get("timestamp")
            .map_err(|e| AppError::database(format!("Failed to get column 'timestamp': {e}")))?,
        action,
        target_resource: row.try_get("target_resource").map_err(|e| {
            AppError::database(format!("Failed to get column 'target_resource': {e}"))
        })?,
        ip_address: row
            .try_get("ip_address")
            .map_err(|e| AppError::database(format!("Failed to get column 'ip_address': {e}")))?,
        user_agent: row
            .try_get("user_agent")
            .map_err(|e| AppError::database(format!("Failed to get column 'user_agent': {e}")))?,
        #[allow(clippy::cast_sign_loss)]
        request_size_bytes: row
            .try_get::<Option<i32>, _>("request_size_bytes")
            .map_err(|e| {
                AppError::database(format!("Failed to get column 'request_size_bytes': {e}"))
            })?
            .map(|v| u32::try_from(v.max(0)).unwrap_or(0)),
        success: row
            .try_get("success")
            .map_err(|e| AppError::database(format!("Failed to get column 'success': {e}")))?,
        error_message: None,
        #[allow(clippy::cast_sign_loss)]
        response_time_ms: row
            .try_get::<Option<i32>, _>("response_time_ms")
            .map_err(|e| {
                AppError::database(format!("Failed to get column 'response_time_ms': {e}"))
            })?
            .map(|v| u32::try_from(v.max(0)).unwrap_or(0)),
    })
}
