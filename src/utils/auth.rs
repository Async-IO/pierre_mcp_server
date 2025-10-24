// ABOUTME: Generic authentication utilities for bearer token extraction and validation
// ABOUTME: Eliminates duplication in Authorization header parsing across routes and middleware
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::constants::key_prefixes;
use crate::errors::AppError;
use anyhow::{Context, Result};

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
        return Err(AppError::auth_invalid("Invalid authorization header format").into());
    }

    let token = auth_header
        .strip_prefix("Bearer ")
        .context("Failed to extract bearer token")?
        .trim();

    if token.is_empty() {
        return Err(AppError::auth_invalid("Empty bearer token").into());
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
    let header = auth_header.ok_or_else(AppError::auth_required)?;
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
    auth_header.starts_with(key_prefixes::API_KEY_LIVE) || auth_header.starts_with("sk_")
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
