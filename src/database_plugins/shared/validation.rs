//! Input validation logic shared across database implementations
//!
//! This module provides common validation functions that eliminate duplication
//! between PostgreSQL and SQLite backends.
//!
//! Licensed under either of Apache License, Version 2.0 or MIT License at your option.
//! Copyright ©2025 Async-IO.org

use anyhow::Result;
use crate::errors::AppError;
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
/// * `Err` with AppError::InvalidInput if invalid
///
/// # Examples
/// ```
/// # use pierre_mcp_server::database_plugins::shared::validation::validate_email;
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid").is_err());
/// assert!(validate_email("@").is_err());
/// ```
pub fn validate_email(email: &str) -> Result<()> {
    if !email.contains('@') || email.len() < 3 {
        return Err(AppError::invalid_input("Invalid email format").into());
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
/// * `Err` with AppError::AuthInvalid if mismatch
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
) -> Result<()> {
    if entity_tenant_id != expected_tenant_id {
        return Err(AppError::auth_invalid(format!(
            "{entity_type} does not belong to the specified tenant"
        ))
        .into());
    }
    Ok(())
}

/// Validate expiration timestamp (OAuth codes, tokens, sessions)
///
/// Checks if a timestamp is in the future. Used for OAuth2 codes, access tokens,
/// refresh tokens, and A2A sessions.
///
/// # Arguments
/// * `expires_at` - The expiration timestamp from the database
/// * `now` - Current timestamp (injected for testability)
/// * `entity_type` - Human-readable entity type for error messages (e.g., "OAuth token", "Session")
///
/// # Returns
/// * `Ok(())` if not expired (expires_at > now)
/// * `Err` with AppError::InvalidInput if expired
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
) -> Result<()> {
    if expires_at <= now {
        return Err(AppError::invalid_input(format!("{entity_type} has expired")).into());
    }
    Ok(())
}

/// Validate scope authorization (A2A, OAuth2)
///
/// Ensures all requested scopes are present in the granted scopes list.
/// Used for OAuth2 scope validation and A2A session authorization.
///
/// # Arguments
/// * `requested_scopes` - Scopes being requested/used
/// * `granted_scopes` - Scopes that were authorized
///
/// # Returns
/// * `Ok(())` if all requested scopes are granted
/// * `Err` with AppError::AuthInvalid if any scope is missing
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
) -> Result<()> {
    for scope in requested_scopes {
        if !granted_scopes.contains(scope) {
            return Err(AppError::auth_invalid(format!("Scope '{}' not granted", scope)).into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.user+tag@subdomain.example.co.uk").is_ok());
        assert!(validate_email("a@b.c").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(validate_email("").is_err());
        assert!(validate_email("@").is_err());
        assert!(validate_email("no-at-sign").is_err());
        assert!(validate_email("a@").is_err());
        assert!(validate_email("@b").is_err());
    }

    #[test]
    fn test_validate_tenant_ownership_match() {
        assert!(validate_tenant_ownership("tenant-123", "tenant-123", "User").is_ok());
    }

    #[test]
    fn test_validate_tenant_ownership_mismatch() {
        let result = validate_tenant_ownership("tenant-123", "tenant-456", "OAuth token");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not belong to the specified tenant"));
    }

    #[test]
    fn test_validate_not_expired_future() {
        let now = Utc::now();
        let future = now + Duration::hours(1);
        assert!(validate_not_expired(future, now, "Token").is_ok());
    }

    #[test]
    fn test_validate_not_expired_past() {
        let now = Utc::now();
        let past = now - Duration::hours(1);
        let result = validate_not_expired(past, now, "Session");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("has expired"));
    }

    #[test]
    fn test_validate_not_expired_exact() {
        let now = Utc::now();
        // Expires_at <= now should fail
        let result = validate_not_expired(now, now, "Code");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_scope_granted_all_present() {
        let granted = vec!["read".to_string(), "write".to_string(), "admin".to_string()];
        let requested = vec!["read".to_string(), "write".to_string()];
        assert!(validate_scope_granted(&requested, &granted).is_ok());
    }

    #[test]
    fn test_validate_scope_granted_empty_request() {
        let granted = vec!["read".to_string()];
        let requested: Vec<String> = vec![];
        assert!(validate_scope_granted(&requested, &granted).is_ok());
    }

    #[test]
    fn test_validate_scope_granted_missing_scope() {
        let granted = vec!["read".to_string()];
        let requested = vec!["write".to_string()];
        let result = validate_scope_granted(&requested, &granted);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not granted"));
    }

    #[test]
    fn test_validate_scope_granted_partial_match() {
        let granted = vec!["read".to_string(), "write".to_string()];
        let requested = vec!["read".to_string(), "admin".to_string()];
        let result = validate_scope_granted(&requested, &granted);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("admin"));
    }
}
