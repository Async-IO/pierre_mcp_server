// ABOUTME: Generic authentication utilities for bearer token extraction and validation
// ABOUTME: Eliminates duplication in Authorization header parsing across routes and middleware

use anyhow::{anyhow, Context, Result};

/// Extract bearer token from Authorization header string
///
/// # Errors
///
/// Returns an error if:
/// - Authorization header doesn't start with "Bearer "
/// - Token is empty after extraction and trimming
/// - Header format is invalid
pub fn extract_bearer_token(auth_header: &str) -> Result<&str> {
    if !auth_header.starts_with("Bearer ") {
        return Err(anyhow!("Invalid authorization header format"));
    }

    let token = auth_header
        .strip_prefix("Bearer ")
        .context("Failed to extract bearer token")?
        .trim();

    if token.is_empty() {
        return Err(anyhow!("Empty bearer token"));
    }

    Ok(token)
}

/// Extract bearer token and return it as owned String
///
/// # Errors
///
/// Returns an error if:
/// - Authorization header doesn't start with "Bearer "  
/// - Token is empty after extraction and trimming
/// - Header format is invalid
pub fn extract_bearer_token_owned(auth_header: &str) -> Result<String> {
    extract_bearer_token(auth_header).map(str::to_string)
}

/// Extract bearer token from optional Authorization header
///
/// # Errors
///
/// Returns an error if:
/// - Authorization header is missing (None)
/// - Header format is invalid
/// - Token is empty
pub fn extract_bearer_token_from_option(auth_header: Option<&str>) -> Result<&str> {
    let header = auth_header.ok_or_else(|| anyhow!("Missing authorization header"))?;
    extract_bearer_token(header)
}

/// Extract bearer token from optional Authorization header as owned String
///
/// # Errors
///
/// Returns an error if:
/// - Authorization header is missing (None)
/// - Header format is invalid  
/// - Token is empty
pub fn extract_bearer_token_from_option_owned(auth_header: Option<&str>) -> Result<String> {
    extract_bearer_token_from_option(auth_header).map(str::to_string)
}

/// Check if authorization header is in Bearer format
#[must_use]
pub fn is_bearer_token(auth_header: &str) -> bool {
    auth_header.starts_with("Bearer ") && auth_header.len() > 7
}

/// Check if authorization header is likely an API key format
#[must_use]
pub fn is_api_key_format(auth_header: &str) -> bool {
    auth_header.starts_with("pk_live_") || auth_header.starts_with("sk_")
}

/// Determine the authorization type from header
#[derive(Debug, PartialEq, Eq)]
pub enum AuthType {
    Bearer,
    ApiKey,
    Unknown,
}

#[must_use]
pub fn detect_auth_type(auth_header: &str) -> AuthType {
    if is_bearer_token(auth_header) {
        AuthType::Bearer
    } else if is_api_key_format(auth_header) {
        AuthType::ApiKey
    } else {
        AuthType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bearer_token_valid() {
        let header = "Bearer abc123";
        let result = extract_bearer_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123");
    }

    #[test]
    fn test_extract_bearer_token_with_whitespace() {
        let header = "Bearer   abc123   ";
        let result = extract_bearer_token(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123");
    }

    #[test]
    fn test_extract_bearer_token_invalid_prefix() {
        let header = "Basic abc123";
        let result = extract_bearer_token(header);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid authorization header format"));
    }

    #[test]
    fn test_extract_bearer_token_empty_token() {
        let header = "Bearer ";
        let result = extract_bearer_token(header);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty bearer token"));
    }

    #[test]
    fn test_extract_bearer_token_only_whitespace() {
        let header = "Bearer   ";
        let result = extract_bearer_token(header);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Empty bearer token"));
    }

    #[test]
    fn test_extract_bearer_token_owned() {
        let header = "Bearer abc123";
        let result = extract_bearer_token_owned(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123".to_string());
    }

    #[test]
    fn test_extract_bearer_token_from_option_some() {
        let header = Some("Bearer abc123");
        let result = extract_bearer_token_from_option(header);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "abc123");
    }

    #[test]
    fn test_extract_bearer_token_from_option_none() {
        let header: Option<&str> = None;
        let result = extract_bearer_token_from_option(header);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Missing authorization header"));
    }

    #[test]
    fn test_is_bearer_token() {
        assert!(is_bearer_token("Bearer abc123"));
        assert!(!is_bearer_token("Basic abc123"));
        assert!(!is_bearer_token("Bearer"));
        assert!(!is_bearer_token("pk_live_abc123"));
    }

    #[test]
    fn test_is_api_key_format() {
        assert!(is_api_key_format("pk_live_abc123"));
        assert!(is_api_key_format("sk_abc123"));
        assert!(!is_api_key_format("Bearer abc123"));
        assert!(!is_api_key_format("Basic abc123"));
    }

    #[test]
    fn test_detect_auth_type() {
        assert_eq!(detect_auth_type("Bearer abc123"), AuthType::Bearer);
        assert_eq!(detect_auth_type("pk_live_abc123"), AuthType::ApiKey);
        assert_eq!(detect_auth_type("sk_abc123"), AuthType::ApiKey);
        assert_eq!(detect_auth_type("Basic abc123"), AuthType::Unknown);
        assert_eq!(detect_auth_type("random"), AuthType::Unknown);
    }
}
