// ABOUTME: Input validation logic shared across database implementations.
// ABOUTME: Provides common validation functions for PostgreSQL and SQLite backends.

// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Input validation logic shared across database implementations
//!
//! This module provides common validation functions that eliminate duplication
//! between PostgreSQL and SQLite backends.

use crate::errors::{AppError, AppResult};
use chrono::{DateTime, Utc};

/// Validate email format
///
/// Performs basic email validation (contains '@' and minimum length).
/// For production use, consider using a dedicated email validation library.
///
/// # Arguments
/// * `email` - Email address to validate
///
/// # Returns
/// * `Ok(())` if valid
///
/// # Errors
/// * Returns `AppError::InvalidInput` if invalid
///
/// # Examples
/// ```
/// # use pierre_mcp_server::database_plugins::shared::validation::validate_email;
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid").is_err());
/// assert!(validate_email("@").is_err());
/// ```
pub fn validate_email(email: &str) -> AppResult<()> {
    if !email.contains('@') || email.len() < 3 {
        return Err(AppError::invalid_input("Invalid email format"));
    }
    Ok(())
}

/// Validate that entity belongs to specified tenant (authorization check)
///
/// Used for multi-tenant isolation to ensure users can only access resources
/// within their own tenant.
///
/// # Arguments
/// * `entity_tenant_id` - The tenant ID from the database record
/// * `expected_tenant_id` - The tenant ID from the authenticated user
/// * `entity_type` - Human-readable entity type for error messages (e.g., "User", "OAuth token")
///
/// # Returns
/// * `Ok(())` if tenant IDs match
///
/// # Errors
/// * Returns `AppError::AuthInvalid` if mismatch
///
/// # Examples
/// ```
/// # use pierre_mcp_server::database_plugins::shared::validation::validate_tenant_ownership;
/// assert!(validate_tenant_ownership("tenant-123", "tenant-123", "User").is_ok());
/// assert!(validate_tenant_ownership("tenant-123", "tenant-456", "User").is_err());
/// ```
pub fn validate_tenant_ownership(
    entity_tenant_id: &str,
    expected_tenant_id: &str,
    entity_type: &str,
) -> AppResult<()> {
    if entity_tenant_id != expected_tenant_id {
        return Err(AppError::auth_invalid(format!(
            "{entity_type} does not belong to the specified tenant"
        )));
    }
    Ok(())
}

/// Validate expiration timestamp (OAuth codes, tokens, sessions)
///
/// Checks if a timestamp is in the future. Used for `OAuth2` codes, access tokens,
/// refresh tokens, and A2A sessions.
///
/// # Arguments
/// * `expires_at` - The expiration timestamp from the database
/// * `now` - Current timestamp (injected for testability)
/// * `entity_type` - Human-readable entity type for error messages (e.g., "OAuth token", "Session")
///
/// # Returns
/// * `Ok(())` if not expired (`expires_at` > now)
///
/// # Errors
/// * Returns `AppError::InvalidInput` if expired
///
/// # Examples
/// ```
/// # use pierre_mcp_server::database_plugins::shared::validation::validate_not_expired;
/// # use chrono::{Utc, Duration};
/// let now = Utc::now();
/// let future = now + Duration::hours(1);
/// let past = now - Duration::hours(1);
///
/// assert!(validate_not_expired(future, now, "Token").is_ok());
/// assert!(validate_not_expired(past, now, "Token").is_err());
/// ```
pub fn validate_not_expired(
    expires_at: DateTime<Utc>,
    now: DateTime<Utc>,
    entity_type: &str,
) -> AppResult<()> {
    if expires_at <= now {
        return Err(AppError::invalid_input(format!(
            "{entity_type} has expired"
        )));
    }
    Ok(())
}

/// Validate scope authorization (A2A, `OAuth2`)
///
/// Ensures all requested scopes are present in the granted scopes list.
/// Used for `OAuth2` scope validation and A2A session authorization.
///
/// # Arguments
/// * `requested_scopes` - Scopes being requested/used
/// * `granted_scopes` - Scopes that were authorized
///
/// # Returns
/// * `Ok(())` if all requested scopes are granted
///
/// # Errors
/// * Returns `AppError::AuthInvalid` if any scope is missing
///
/// # Examples
/// ```
/// # use pierre_mcp_server::database_plugins::shared::validation::validate_scope_granted;
/// let granted = vec!["read".to_string(), "write".to_string()];
/// let valid_request = vec!["read".to_string()];
/// let invalid_request = vec!["admin".to_string()];
///
/// assert!(validate_scope_granted(&valid_request, &granted).is_ok());
/// assert!(validate_scope_granted(&invalid_request, &granted).is_err());
/// ```
pub fn validate_scope_granted(
    requested_scopes: &[String],
    granted_scopes: &[String],
) -> AppResult<()> {
    for scope in requested_scopes {
        if !granted_scopes.contains(scope) {
            return Err(AppError::auth_invalid(format!(
                "Scope '{scope}' not granted"
            )));
        }
    }
    Ok(())
}
