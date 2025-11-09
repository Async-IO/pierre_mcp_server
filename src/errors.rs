// ABOUTME: Centralized error handling and error types for Pierre API
// ABOUTME: Defines all error variants used across MCP, A2A, and REST protocols
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! # Unified Error Handling System
//!
//! This module provides a centralized error handling system for the Pierre MCP Server.
//! It defines standard error types, error codes, and HTTP response formatting to ensure
//! consistent error handling across all modules and APIs.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use warp::reject::Reject;

/// Standard error codes used throughout the application
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
            | Self::ValueOutOfRange => crate::constants::http_status::BAD_REQUEST,

            // 401 Unauthorized - Authentication issues (missing or invalid credentials)
            Self::AuthRequired | Self::AuthInvalid => crate::constants::http_status::UNAUTHORIZED,

            // 403 Forbidden - Authorization issues (expired/malformed tokens, permission denied)
            Self::AuthExpired | Self::AuthMalformed | Self::PermissionDenied => {
                crate::constants::http_status::FORBIDDEN
            }

            // 404 Not Found
            Self::ResourceNotFound => crate::constants::http_status::NOT_FOUND,

            // 409 Conflict
            Self::ResourceAlreadyExists | Self::ResourceLocked => {
                crate::constants::http_status::CONFLICT
            }

            // 429 Too Many Requests
            Self::RateLimitExceeded | Self::QuotaExceeded => {
                crate::constants::http_status::TOO_MANY_REQUESTS
            }

            // 502 Bad Gateway
            Self::ExternalServiceError | Self::ExternalServiceUnavailable => {
                crate::constants::http_status::BAD_GATEWAY
            }

            // 503 Service Unavailable
            Self::ResourceUnavailable | Self::ExternalAuthFailed | Self::ExternalRateLimited => {
                crate::constants::http_status::SERVICE_UNAVAILABLE
            }

            // 500 Internal Server Error
            Self::InternalError
            | Self::DatabaseError
            | Self::StorageError
            | Self::SerializationError
            | Self::ConfigError
            | Self::ConfigMissing
            | Self::ConfigInvalid => crate::constants::http_status::INTERNAL_SERVER_ERROR,
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
            _ => Err(serde::de::Error::unknown_variant(&s, &[])),
        }
    }
}

/// Simplified error type for the application
#[derive(Debug, Clone, Error)]
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
            // Validation errors: message is already safe to expose
            ErrorCode::InvalidInput
            | ErrorCode::MissingRequiredField
            | ErrorCode::InvalidFormat
            | ErrorCode::ValueOutOfRange => self.message.clone(),
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

/// Implement Reject for Warp framework integration
impl Reject for AppError {}

/// Convert `AppError` to warp `Reply` for `HTTP` responses
impl warp::Reply for AppError {
    fn into_response(self) -> warp::reply::Response {
        let status = warp::http::StatusCode::from_u16(self.code.http_status())
            .unwrap_or(warp::http::StatusCode::INTERNAL_SERVER_ERROR);

        let response = ErrorResponse::from(self);
        let json = warp::reply::json(&response);

        warp::reply::with_status(json, status).into_response()
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
        tracing::warn!("API error: {}", error.internal_details());

        Self {
            code: error.code,
            message: error.sanitized_message(), // Use sanitized message for client
            request_id: error.request_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
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

/// Conversion from `anyhow::Error` to `AppError`
impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::new(ErrorCode::InternalError, error.to_string())
    }
}

/// Conversion from `std::io::Error` to `AppError`
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorCode::InternalError, format!("IO error: {error}"))
    }
}

/// Conversion from `serde_json::Error` to `AppError`
impl From<serde_json::Error> for AppError {
    fn from(error: serde_json::Error) -> Self {
        Self::new(ErrorCode::InvalidInput, format!("JSON error: {error}"))
    }
}

/// Conversion from `uuid::Error` to `AppError`
impl From<uuid::Error> for AppError {
    fn from(error: uuid::Error) -> Self {
        Self::new(ErrorCode::InvalidInput, format!("UUID error: {error}"))
    }
}

/// Conversion from `chrono::ParseError` to `AppError`
impl From<chrono::ParseError> for AppError {
    fn from(error: chrono::ParseError) -> Self {
        Self::new(
            ErrorCode::InvalidInput,
            format!("Date parse error: {error}"),
        )
    }
}

/// Protocol error conversion helper
impl From<crate::protocols::ProtocolError> for AppError {
    fn from(error: crate::protocols::ProtocolError) -> Self {
        match error {
            crate::protocols::ProtocolError::UnsupportedProtocol(protocol) => {
                Self::invalid_input(format!("Unsupported protocol: {protocol}"))
            }
            crate::protocols::ProtocolError::ToolNotFound(tool) => {
                Self::not_found(format!("tool '{tool}'"))
            }
            crate::protocols::ProtocolError::InvalidParameters(message)
            | crate::protocols::ProtocolError::InvalidRequest(message) => {
                Self::invalid_input(message)
            }
            crate::protocols::ProtocolError::ConfigurationError(message) => Self::config(message),
            crate::protocols::ProtocolError::ExecutionFailed(message) => {
                Self::internal(format!("Tool execution failed: {message}"))
            }
            crate::protocols::ProtocolError::ConversionFailed(message) => {
                Self::internal(format!("Protocol conversion failed: {message}"))
            }
            crate::protocols::ProtocolError::SerializationError(message) => {
                Self::internal(format!("Serialization failed: {message}"))
            }
            crate::protocols::ProtocolError::DatabaseError(message) => {
                Self::internal(format!("Database operation failed: {message}"))
            }
            crate::protocols::ProtocolError::PluginNotFound(plugin) => {
                Self::not_found(format!("plugin '{plugin}'"))
            }
            crate::protocols::ProtocolError::PluginError(message) => {
                Self::internal(format!("Plugin error: {message}"))
            }
            crate::protocols::ProtocolError::InvalidSchema(message) => {
                Self::invalid_input(format!("Invalid schema: {message}"))
            }
            crate::protocols::ProtocolError::InsufficientSubscription(message) => {
                Self::auth_invalid(message)
            }
            crate::protocols::ProtocolError::RateLimitExceeded(message) => {
                Self::invalid_input(format!("Rate limit exceeded: {message}"))
            }
            crate::protocols::ProtocolError::InternalError(message) => Self::internal(message),
        }
    }
}

/// Database error conversion helper
/// Note: This is conditional on whether `SQLx` is actually used in the database plugins
#[cfg(any(feature = "postgresql", feature = "sqlite"))]
impl From<Box<dyn std::error::Error + Send + Sync>> for AppError {
    fn from(error: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::database(error.to_string())
    }
}
