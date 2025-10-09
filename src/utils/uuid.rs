// ABOUTME: UUID parsing and validation utilities to eliminate duplication across the codebase
// ABOUTME: Provides safe UUID parsing with consistent error handling and format validation
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use anyhow::{anyhow, Context, Result};
use uuid::Uuid;

/// Parse a UUID from a string with consistent error handling
///
/// # Errors
///
/// Returns an error if the string is not a valid UUID format
pub fn parse_uuid(uuid_str: &str) -> Result<Uuid> {
    Uuid::parse_str(uuid_str).with_context(|| format!("Invalid UUID format: '{uuid_str}'"))
}

/// Parse a UUID from a string, returning a custom error message
///
/// # Errors
///
/// Returns an error with the provided custom message if parsing fails
pub fn parse_uuid_with_message(uuid_str: &str, error_msg: &str) -> Result<Uuid> {
    Uuid::parse_str(uuid_str).map_err(|_| anyhow!("{error_msg}"))
}

/// Parse a UUID for a user ID with specific error handling
///
/// # Errors
///
/// Returns an error if the user ID is not a valid UUID format
pub fn parse_user_id(user_id_str: &str) -> Result<Uuid> {
    Uuid::parse_str(user_id_str).with_context(|| format!("Invalid user ID format: '{user_id_str}'"))
}

/// Parse an optional UUID string
///
/// Returns None if the input is None, otherwise attempts to parse the UUID
///
/// # Errors
///
/// Returns an error if the string is Some but not a valid UUID
pub fn parse_optional_uuid(uuid_str: Option<&str>) -> Result<Option<Uuid>> {
    uuid_str.map(parse_uuid).transpose()
}

/// Parse an optional UUID string with a custom error message
///
/// # Errors
///
/// Returns an error with the custom message if the string is Some but not a valid UUID
pub fn parse_optional_uuid_with_message(
    uuid_str: Option<&str>,
    error_msg: &str,
) -> Result<Option<Uuid>> {
    uuid_str
        .map(|s| parse_uuid_with_message(s, error_msg))
        .transpose()
}

/// Check if a string is a valid UUID format without allocating
#[must_use]
pub fn is_valid_uuid(uuid_str: &str) -> bool {
    Uuid::parse_str(uuid_str).is_ok()
}

/// Parse a UUID from a string owned value
///
/// # Errors
///
/// Returns an error if the string is not a valid UUID format
pub fn parse_uuid_owned(uuid_str: &str) -> Result<Uuid> {
    Uuid::parse_str(uuid_str).with_context(|| format!("Invalid UUID format: '{uuid_str}'"))
}

/// Parse a user ID from state parameter (format: "`user_id:random_uuid`")
///
/// # Errors
///
/// Returns an error if the state format is invalid or `user_id` is not a valid UUID
pub fn parse_user_id_from_state(state: &str) -> Result<Uuid> {
    let parts: Vec<&str> = state.split(':').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid state parameter format"));
    }
    parse_user_id(parts[0])
}

/// Parse a user ID for protocol requests with `ProtocolError`
///
/// # Errors
///
/// Returns a `ProtocolError::InvalidParameters` if the user ID is not a valid UUID
pub fn parse_user_id_for_protocol(
    user_id_str: &str,
) -> Result<Uuid, crate::protocols::ProtocolError> {
    Uuid::parse_str(user_id_str).map_err(|_| {
        crate::protocols::ProtocolError::InvalidParameters("Invalid user ID format".into())
    })
}

/// Format a UUID as a hyphenated string
#[must_use]
pub fn format_uuid(uuid: &Uuid) -> String {
    uuid.hyphenated().to_string()
}

/// Format a UUID as a simple string (no hyphens)
#[must_use]
pub fn format_uuid_simple(uuid: &Uuid) -> String {
    uuid.simple().to_string()
}

/// Create a new random UUID v4
#[must_use]
pub fn new_uuid() -> Uuid {
    Uuid::new_v4()
}

/// Create a new random UUID v4 as a string
#[must_use]
pub fn new_uuid_string() -> String {
    Uuid::new_v4().to_string()
}
