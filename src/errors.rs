// ABOUTME: Centralized error handling and error types for Pierre API
// ABOUTME: Defines all error variants used across MCP, A2A, and REST protocols
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Unified Error Handling System
//!
//! This module provides a centralized error handling system for the Pierre MCP Server.
//! It defines standard error types, error codes, and HTTP response formatting to ensure
//! consistent error handling across all modules and APIs.

use std::array::TryFromSliceError;
#[cfg(any(feature = "postgresql", feature = "sqlite"))]
use std::error::Error;
use std::fmt::{self, Display};
use std::io;
use std::num::TryFromIntError;

use ring::error::Unspecified as RingUnspecified;
use serde::de::Error as SerdeDeError;
use serde::{Deserialize, Serialize};
use serde_json::Error as JsonError;
use thiserror::Error as ThisError;
use tracing::warn;
use uuid::Error as UuidError;

use axum::response::{IntoResponse, Response};
use chrono::{ParseError as ChronoParseError, Utc};

use crate::constants::http_status::{
    BAD_GATEWAY, BAD_REQUEST, CONFLICT, FORBIDDEN, INTERNAL_SERVER_ERROR, NOT_FOUND,
    SERVICE_UNAVAILABLE, TOO_MANY_REQUESTS, UNAUTHORIZED,
};
use crate::database::DatabaseError;
use crate::protocols::ProtocolError;
use crate::providers::errors::ProviderError;

/// Standard error codes used throughout the application
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Authentication & Authorization
    /// Authentication is required but not provided
    AuthRequired,
    /// Authentication credentials are invalid
    AuthInvalid,
    /// Authentication token has expired
    AuthExpired,
    /// Authentication format is malformed
    AuthMalformed,
    /// User lacks permission for the requested operation
    PermissionDenied,

    // Rate Limiting
    /// Rate limit has been exceeded
    RateLimitExceeded,
    /// Usage quota has been exceeded
    QuotaExceeded,

    // Validation
    /// Input validation failed
    InvalidInput,
    /// Required field is missing from request
    MissingRequiredField,
    /// Data format is invalid
    InvalidFormat,
    /// Value is outside acceptable range
    ValueOutOfRange,

    // Resource Management
    /// Requested resource was not found
    ResourceNotFound,
    /// Resource already exists (conflict)
    ResourceAlreadyExists,
    /// Resource is locked and cannot be modified
    ResourceLocked,
    /// Resource is temporarily unavailable
    ResourceUnavailable,

    // External Services
    /// External service returned an error
    ExternalServiceError,
    /// External service is unavailable
    ExternalServiceUnavailable,
    /// Authentication with external service failed
    ExternalAuthFailed,
    /// External service rate limited our request
    ExternalRateLimited,

    // Configuration
    /// Configuration error occurred
    ConfigError,
    /// Required configuration is missing
    ConfigMissing,
    /// Configuration value is invalid
    ConfigInvalid,

    // Internal Errors
    /// Internal server error
    InternalError,
    /// Database operation failed
    DatabaseError,
    /// Storage operation failed
    StorageError,
    /// Serialization/deserialization failed
    SerializationError,
}

impl ErrorCode {
    /// Get the `HTTP` status code for this error
    #[must_use]
    pub const fn http_status(self) -> u16 {
        match self {
            // 400 Bad Request
            Self::InvalidInput
            | Self::MissingRequiredField
            | Self::InvalidFormat
            | Self::ValueOutOfRange => BAD_REQUEST,

            // 401 Unauthorized - Authentication issues (missing or invalid credentials)
            Self::AuthRequired | Self::AuthInvalid => UNAUTHORIZED,

            // 403 Forbidden - Authorization issues (expired/malformed tokens, permission denied)
            Self::AuthExpired | Self::AuthMalformed | Self::PermissionDenied => FORBIDDEN,

            // 404 Not Found
            Self::ResourceNotFound => NOT_FOUND,

            // 409 Conflict
            Self::ResourceAlreadyExists | Self::ResourceLocked => CONFLICT,

            // 429 Too Many Requests
            Self::RateLimitExceeded | Self::QuotaExceeded => TOO_MANY_REQUESTS,

            // 502 Bad Gateway
            Self::ExternalServiceError | Self::ExternalServiceUnavailable => BAD_GATEWAY,

            // 503 Service Unavailable
            Self::ResourceUnavailable | Self::ExternalAuthFailed | Self::ExternalRateLimited => {
                SERVICE_UNAVAILABLE
            }

            // 500 Internal Server Error
            Self::InternalError
            | Self::DatabaseError
            | Self::StorageError
            | Self::SerializationError
            | Self::ConfigError
            | Self::ConfigMissing
            | Self::ConfigInvalid => INTERNAL_SERVER_ERROR,
        }
    }

    /// Get a user-friendly description of this error
    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::AuthRequired => "Authentication is required to access this resource",
            Self::AuthInvalid => "The provided authentication credentials are invalid",
            Self::AuthExpired => "The authentication token has expired",
            Self::AuthMalformed => "The authentication token is malformed or corrupted",
            Self::PermissionDenied => "You do not have permission to perform this action",
            Self::RateLimitExceeded => "Rate limit exceeded. Please slow down your requests",
            Self::QuotaExceeded => "Usage quota exceeded for your current plan",
            Self::InvalidInput => "The provided input is invalid",
            Self::MissingRequiredField => "A required field is missing from the request",
            Self::InvalidFormat => "The data format is invalid",
            Self::ValueOutOfRange => "The provided value is outside the acceptable range",
            Self::ResourceNotFound => "The requested resource was not found",
            Self::ResourceAlreadyExists => "A resource with this identifier already exists",
            Self::ResourceLocked => "The resource is currently locked and cannot be modified",
            Self::ResourceUnavailable => "The resource is temporarily unavailable",
            Self::ExternalServiceError => "An external service encountered an error",
            Self::ExternalServiceUnavailable => "An external service is currently unavailable",
            Self::ExternalAuthFailed => "Authentication with external service failed",
            Self::ExternalRateLimited => "External service rate limit exceeded",
            Self::ConfigError => "Configuration error encountered",
            Self::ConfigMissing => "Required configuration is missing",
            Self::ConfigInvalid => "Configuration is invalid",
            Self::InternalError => "An internal server error occurred",
            Self::DatabaseError => "Database operation failed",
            Self::StorageError => "Storage operation failed",
            Self::SerializationError => "Data serialization/deserialization failed",
        }
    }
}

// Simple serialization - just use the debug representation
impl Serialize for ErrorCode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{self:?}"))
    }
}

impl<'de> Deserialize<'de> for ErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "AuthRequired" => Ok(Self::AuthRequired),
            "AuthInvalid" => Ok(Self::AuthInvalid),
            "AuthExpired" => Ok(Self::AuthExpired),
            "AuthMalformed" => Ok(Self::AuthMalformed),
            "PermissionDenied" => Ok(Self::PermissionDenied),
            "RateLimitExceeded" => Ok(Self::RateLimitExceeded),
            "QuotaExceeded" => Ok(Self::QuotaExceeded),
            "InvalidInput" => Ok(Self::InvalidInput),
            "MissingRequiredField" => Ok(Self::MissingRequiredField),
            "InvalidFormat" => Ok(Self::InvalidFormat),
            "ValueOutOfRange" => Ok(Self::ValueOutOfRange),
            "ResourceNotFound" => Ok(Self::ResourceNotFound),
            "ResourceAlreadyExists" => Ok(Self::ResourceAlreadyExists),
            "ResourceLocked" => Ok(Self::ResourceLocked),
            "ResourceUnavailable" => Ok(Self::ResourceUnavailable),
            "ExternalServiceError" => Ok(Self::ExternalServiceError),
            "ExternalServiceUnavailable" => Ok(Self::ExternalServiceUnavailable),
            "ExternalAuthFailed" => Ok(Self::ExternalAuthFailed),
            "ExternalRateLimited" => Ok(Self::ExternalRateLimited),
            "ConfigError" => Ok(Self::ConfigError),
            "ConfigMissing" => Ok(Self::ConfigMissing),
            "ConfigInvalid" => Ok(Self::ConfigInvalid),
            "InternalError" => Ok(Self::InternalError),
            "DatabaseError" => Ok(Self::DatabaseError),
            "StorageError" => Ok(Self::StorageError),
            "SerializationError" => Ok(Self::SerializationError),
            _ => Err(SerdeDeError::unknown_variant(&s, &[])),
        }
    }
}

/// Simplified error type for the application
#[derive(Debug, Clone, ThisError)]
pub struct AppError {
    /// Error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Optional request `ID` for tracing
    pub request_id: Option<String>,
}

impl AppError {
    /// Create a new `AppError` with the given code and message
    #[must_use]
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            request_id: None,
        }
    }

    /// Add a request `ID` to the error
    #[must_use]
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Get the `HTTP` status code for this error
    #[must_use]
    pub const fn http_status(&self) -> u16 {
        self.code.http_status()
    }

    /// Get sanitized message safe for client exposure
    /// Internal error details are replaced with generic messages
    #[must_use]
    pub fn sanitized_message(&self) -> String {
        match self.code {
            // Validation and rate limit errors: messages are safe to expose
            // Rate limit messages help users understand wait times
            ErrorCode::InvalidInput
            | ErrorCode::MissingRequiredField
            | ErrorCode::InvalidFormat
            | ErrorCode::ValueOutOfRange
            | ErrorCode::RateLimitExceeded
            | ErrorCode::QuotaExceeded
            | ErrorCode::ExternalRateLimited => self.message.clone(),
            // JWT validation errors: expose details to help with troubleshooting
            // (key mismatches, expiry, etc. don't contain sensitive data)
            ErrorCode::AuthInvalid if self.message.contains("JWT") => self.message.clone(),
            // All other errors: use generic description (auth, database, internal)
            _ => self.code.description().to_owned(),
        }
    }

    /// Get full error details for internal logging
    /// NEVER send this to clients - contains sensitive information
    #[must_use]
    pub fn internal_details(&self) -> String {
        format!("{:?}: {}", self.code, self.message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.description(), self.message)
    }
}

/// Convert `AppError` to Axum `Response` for `HTTP` responses
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        use axum::http::StatusCode;
        use axum::Json;

        let status = StatusCode::from_u16(self.code.http_status())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let response = ErrorResponse::from(self);

        (status, Json(response)).into_response()
    }
}

/// Result type alias for convenience
pub type AppResult<T> = Result<T, AppError>;

/// Simplified `HTTP` error response format
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Error code identifying the type of error
    pub code: ErrorCode,
    /// Human-readable error message (sanitized for client)
    pub message: String,
    /// Optional request ID for error tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// RFC3339 timestamp when the error occurred
    pub timestamp: String,
}

impl From<AppError> for ErrorResponse {
    fn from(error: AppError) -> Self {
        // Log full details internally before sanitizing
        warn!("API error: {}", error.internal_details());

        Self {
            code: error.code,
            message: error.sanitized_message(), // Use sanitized message for client
            request_id: error.request_id,
            timestamp: Utc::now().to_rfc3339(),
        }
    }
}

/// Convenience functions for creating common errors
impl AppError {
    /// Authentication required
    #[must_use]
    pub fn auth_required() -> Self {
        Self::new(ErrorCode::AuthRequired, "Authentication required")
    }

    /// Invalid authentication
    #[must_use]
    pub fn auth_invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthInvalid, message)
    }

    /// Authentication expired
    #[must_use]
    pub fn auth_expired() -> Self {
        Self::new(ErrorCode::AuthExpired, "Authentication token has expired")
    }

    /// Rate limit exceeded
    #[must_use]
    pub fn rate_limit_exceeded(limit: u32) -> Self {
        Self::new(
            ErrorCode::RateLimitExceeded,
            format!("Rate limit of {limit} requests exceeded"),
        )
    }

    /// Resource not found
    #[must_use]
    pub fn not_found(resource: impl Into<String>) -> Self {
        let resource_str = resource.into();
        Self::new(
            ErrorCode::ResourceNotFound,
            format!("{resource_str} not found"),
        )
    }

    /// Invalid input
    #[must_use]
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    /// Internal server error
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Database error
    #[must_use]
    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DatabaseError, message)
    }

    /// Configuration error
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ConfigError, message)
    }

    /// Encryption key mismatch error with actionable guidance
    ///
    /// This error occurs when the Master Encryption Key (MEK) doesn't match
    /// the key that was used to encrypt the Database Encryption Key (DEK).
    #[must_use]
    pub fn encryption_key_mismatch(database_url: &str) -> Self {
        Self::new(
            ErrorCode::ConfigError,
            format!(
                "Encryption key mismatch\n\n\
                 The PIERRE_MASTER_ENCRYPTION_KEY does not match the key used to encrypt\n\
                 this database. This happens when:\n\
                 \x20\x20- The encryption key was regenerated or changed\n\
                 \x20\x20- The database was copied from another environment\n\
                 \x20\x20- The environment variables weren't loaded properly (check direnv)\n\n\
                 To fix:\n\
                 \x20\x201. Use the original PIERRE_MASTER_ENCRYPTION_KEY that created this database, OR\n\
                 \x20\x202. Delete the database file and restart (this loses all existing data):\n\
                 \x20\x20   rm {database_url}\n\n\
                 Database location: {database_url}",
                database_url = database_url.trim_start_matches("sqlite:")
            ),
        )
    }

    /// External service error
    #[must_use]
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        let service_str = service.into();
        let message_str = message.into();
        Self::new(
            ErrorCode::ExternalServiceError,
            format!("{service_str}: {message_str}"),
        )
    }
}

/// Conversion from `std::io::Error` to `AppError`
impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        Self::new(ErrorCode::InternalError, format!("IO error: {error}"))
    }
}

/// Conversion from `serde_json::Error` to `AppError`
impl From<JsonError> for AppError {
    fn from(error: JsonError) -> Self {
        Self::new(ErrorCode::InvalidInput, format!("JSON error: {error}"))
    }
}

/// Conversion from `sqlx::Error` to `AppError`
impl From<sqlx::Error> for AppError {
    fn from(error: sqlx::Error) -> Self {
        Self::database(format!("Database operation failed: {error}"))
    }
}

/// Conversion from `DatabaseError` to `AppError`
impl From<DatabaseError> for AppError {
    fn from(error: DatabaseError) -> Self {
        Self::database(format!("Database error: {error}"))
    }
}

/// Conversion from `uuid::Error` to `AppError`
impl From<UuidError> for AppError {
    fn from(error: UuidError) -> Self {
        Self::new(ErrorCode::InvalidInput, format!("UUID error: {error}"))
    }
}

/// Conversion from `chrono::ParseError` to `AppError`
impl From<ChronoParseError> for AppError {
    fn from(error: ChronoParseError) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Date parse error: {error}"),
        )
    }
}

/// Conversion from `TryFromIntError` to `AppError`
impl From<TryFromIntError> for AppError {
    fn from(error: TryFromIntError) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Integer conversion error: {error}"),
        )
    }
}

/// Conversion from `ring::error::Unspecified` to `AppError`
impl From<RingUnspecified> for AppError {
    fn from(_error: RingUnspecified) -> Self {
        Self::new(
            ErrorCode::InternalError,
            "Cryptographic operation failed".to_owned(),
        )
    }
}

/// Conversion from `base64::DecodeError` to `AppError`
impl From<base64::DecodeError> for AppError {
    fn from(error: base64::DecodeError) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Base64 decode error: {error}"),
        )
    }
}

/// Conversion from `TryFromSliceError` to `AppError`
impl From<TryFromSliceError> for AppError {
    fn from(error: TryFromSliceError) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Array conversion error: {error}"),
        )
    }
}

/// Protocol error conversion helper
impl From<ProtocolError> for AppError {
    fn from(error: ProtocolError) -> Self {
        match error {
            ProtocolError::UnsupportedProtocol { protocol } => {
                Self::invalid_input(format!("Unsupported protocol: {protocol:?}"))
            }
            ProtocolError::ToolNotFound { tool_id, .. } => {
                Self::not_found(format!("tool '{tool_id}'"))
            }
            ProtocolError::InvalidParameter {
                tool_id,
                parameter,
                reason,
            } => Self::invalid_input(format!(
                "Invalid parameter '{parameter}' for tool '{tool_id}': {reason}"
            )),
            ProtocolError::MissingParameter { tool_id, parameter } => Self::invalid_input(format!(
                "Missing required parameter '{parameter}' for tool '{tool_id}'"
            )),
            ProtocolError::InvalidParameters(message) => Self::invalid_input(message),
            ProtocolError::InvalidRequestDetailed { reason, .. }
            | ProtocolError::InvalidRequest(reason) => Self::invalid_input(reason),
            ProtocolError::ConfigMissing { key } => {
                Self::config(format!("Missing configuration: {key}"))
            }
            ProtocolError::ConfigurationErrorDetailed { message }
            | ProtocolError::ConfigurationError(message) => Self::config(message),
            ProtocolError::ExecutionFailedDetailed { tool_id, .. } => {
                Self::internal(format!("Tool '{tool_id}' execution failed"))
            }
            ProtocolError::ExecutionFailed(message) | ProtocolError::InternalError(message) => {
                Self::internal(message)
            }
            ProtocolError::ConversionFailed { from, to, reason } => Self::internal(format!(
                "Protocol conversion failed from {from:?} to {to:?}: {reason}"
            )),
            ProtocolError::Serialization { context, .. } => {
                Self::internal(format!("Serialization failed for {context}"))
            }
            ProtocolError::SerializationErrorDetailed { message }
            | ProtocolError::SerializationError(message) => {
                Self::internal(format!("Serialization failed: {message}"))
            }
            ProtocolError::Database { source } => {
                Self::internal(format!("Database error: {source}"))
            }
            ProtocolError::PluginNotFound { plugin_id } => {
                Self::not_found(format!("plugin '{plugin_id}'"))
            }
            ProtocolError::PluginError { plugin_id, details } => {
                Self::internal(format!("Plugin '{plugin_id}' error: {details}"))
            }
            ProtocolError::InvalidSchema { entity, reason } => {
                Self::invalid_input(format!("Invalid schema for {entity}: {reason}"))
            }
            ProtocolError::InsufficientSubscription { required, current } => Self::auth_invalid(
                format!("Insufficient subscription tier: requires {required}, has {current}"),
            ),
            ProtocolError::RateLimitExceeded {
                requests,
                window_secs,
            } => Self::invalid_input(format!(
                "Rate limit exceeded: {requests} requests in {window_secs}s"
            )),
            ProtocolError::OperationCancelled(message) => {
                Self::invalid_input(format!("Operation cancelled: {message}"))
            }
        }
    }
}

/// Convert `ProviderError` to `AppError`
impl From<ProviderError> for AppError {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::ApiError {
                provider, message, ..
            } => Self::external_service(&provider, message),
            ProviderError::RateLimitExceeded {
                provider,
                retry_after_secs,
                limit_type,
            } => Self::external_service(
                &provider,
                format!("Rate limit exceeded ({limit_type}): retry after {retry_after_secs}s"),
            ),
            ProviderError::AuthenticationFailed { provider, reason } => {
                Self::auth_invalid(format!("{provider} authentication failed: {reason}"))
            }
            ProviderError::TokenRefreshFailed { provider, details } => {
                Self::auth_invalid(format!("{provider} token refresh failed: {details}"))
            }
            ProviderError::NotFound {
                provider,
                resource_type,
                resource_id,
            } => Self::not_found(format!("{provider} {resource_type} '{resource_id}'")),
            ProviderError::InvalidData {
                provider,
                field,
                reason,
            } => Self::invalid_input(format!("{provider} invalid data in '{field}': {reason}")),
            ProviderError::NetworkError(details) => {
                Self::external_service("provider", format!("Network error: {details}"))
            }
            ProviderError::ConfigurationError { provider, details } => {
                Self::config(format!("{provider} configuration error: {details}"))
            }
            ProviderError::UnsupportedFeature { provider, feature } => {
                Self::invalid_input(format!("{provider} does not support {feature}"))
            }
            ProviderError::HttpError {
                provider,
                status,
                body,
            } => Self::external_service(&provider, format!("HTTP {status}: {body}")),
            ProviderError::ParseError {
                provider,
                field,
                source,
            } => Self::internal(format!("{provider} failed to parse '{field}': {source}")),
            ProviderError::Reqwest { provider, source } => {
                Self::external_service(&provider, format!("Request failed: {source}"))
            }
            ProviderError::Timeout {
                provider,
                operation,
                timeout_secs,
            } => Self::external_service(
                &provider,
                format!("{operation} timed out after {timeout_secs}s"),
            ),
            ProviderError::QuotaExceeded {
                provider,
                quota_type,
            } => Self::external_service(&provider, format!("Quota exceeded: {quota_type}")),
            ProviderError::CircuitBreakerOpen {
                provider,
                retry_after_secs,
            } => Self::external_service(
                &provider,
                format!("Service temporarily unavailable: retry after {retry_after_secs}s"),
            ),
        }
    }
}

/// Database error conversion helper
/// Note: This is conditional on whether `SQLx` is actually used in the database plugins
#[cfg(any(feature = "postgresql", feature = "sqlite"))]
impl From<Box<dyn Error + Send + Sync>> for AppError {
    fn from(error: Box<dyn Error + Send + Sync>) -> Self {
        Self::database(error.to_string())
    }
}

// ============================================================================
// JSON Error Handling Extensions
// ============================================================================

impl AppError {
    /// Create error for JSON deserialization failures with context
    ///
    /// This provides better error messages than the generic From<`serde_json::Error`>
    /// implementation by including context about what was being parsed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::errors::AppError;
    /// use serde::Deserialize;
    /// # fn example() -> Result<(), AppError> {
    /// # #[derive(Deserialize)] struct MyParams;
    /// # let json = serde_json::json!({});
    /// let params: MyParams = serde_json::from_value(json)
    ///     .map_err(|e| AppError::json_parse_error("tool parameters", e))?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn json_parse_error<E: Display>(context: &str, error: E) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Failed to parse JSON in {context}: {error}"),
        )
    }

    /// Create error for missing required JSON fields
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::errors::AppError;
    /// # fn example() -> Result<(), AppError> {
    /// # let obj = serde_json::json!({});
    /// let field_value = obj.get("required_field")
    ///     .ok_or_else(|| AppError::missing_field("required_field"))?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn missing_field(field: &str) -> Self {
        Self::new(
            ErrorCode::MissingRequiredField,
            format!("Missing required field: {field}"),
        )
    }

    /// Create error for invalid JSON field values
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use pierre_mcp_server::errors::AppError;
    /// # fn example(value: i32) -> Result<(), AppError> {
    /// if value < 0 {
    ///     return Err(AppError::invalid_field("age", "must be non-negative"));
    /// }
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn invalid_field(field: &str, reason: &str) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Invalid value for field '{field}': {reason}"),
        )
    }
}

/// Extension trait for adding context to JSON parsing errors
///
/// This trait provides convenient methods to convert `serde_json` errors
/// into `AppError` with meaningful context about what failed to parse.
///
/// # Example
///
/// ```rust,no_run
/// use pierre_mcp_server::errors::{AppError, JsonResultExt};
/// use serde::Deserialize;
///
/// # fn example() -> Result<(), AppError> {
/// # #[derive(Deserialize)] struct MyParams;
/// # let json_value = serde_json::json!({});
/// let params: MyParams = serde_json::from_value(json_value)
///     .json_context("tool parameters")?;
/// # Ok(())
/// # }
/// ```
pub trait JsonResultExt<T> {
    /// Add context to a JSON parsing error
    ///
    /// # Errors
    /// Returns `AppError` with context if parsing fails
    fn json_context(self, context: &str) -> Result<T, AppError>;
}

impl<T> JsonResultExt<T> for Result<T, JsonError> {
    fn json_context(self, context: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::json_parse_error(context, e))
    }
}
