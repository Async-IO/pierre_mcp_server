// ABOUTME: UUID parsing and validation utilities to eliminate duplication across the codebase
// ABOUTME: Provides safe UUID parsing with consistent error handling and format validation

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_uuid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_uuid(uuid_str);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().to_string(), uuid_str);
    }

    #[test]
    fn test_parse_invalid_uuid() {
        let result = parse_uuid("not-a-uuid");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid UUID format"));
    }

    #[test]
    fn test_parse_uuid_with_custom_message() {
        let result = parse_uuid_with_message("invalid", "Custom error message");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Custom error message");
    }

    #[test]
    fn test_parse_user_id() {
        let user_id = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_user_id(user_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_optional_uuid_none() {
        let result = parse_optional_uuid(None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_parse_optional_uuid_some_valid() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let result = parse_optional_uuid(Some(uuid_str));
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[test]
    fn test_parse_optional_uuid_some_invalid() {
        let result = parse_optional_uuid(Some("invalid"));
        assert!(result.is_err());
    }

    #[test]
    fn test_is_valid_uuid() {
        assert!(is_valid_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(!is_valid_uuid("not-a-uuid"));
        assert!(!is_valid_uuid(""));
    }

    #[test]
    fn test_parse_user_id_from_state() {
        let state = "550e8400-e29b-41d4-a716-446655440000:random-data";
        let result = parse_user_id_from_state(state);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap().to_string(),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn test_parse_user_id_from_state_invalid_format() {
        let result = parse_user_id_from_state("no-colon");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid state parameter"));
    }

    #[test]
    fn test_format_uuid() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format_uuid(&uuid), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn test_format_uuid_simple() {
        let uuid = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(
            format_uuid_simple(&uuid),
            "550e8400e29b41d4a716446655440000"
        );
    }

    #[test]
    fn test_new_uuid() {
        let uuid1 = new_uuid();
        let uuid2 = new_uuid();
        assert_ne!(uuid1, uuid2); // Should generate different UUIDs
    }

    #[test]
    fn test_new_uuid_string() {
        let uuid_str = new_uuid_string();
        assert!(is_valid_uuid(&uuid_str));
    }
}
