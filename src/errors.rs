// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Unified Error Handling System
//!
//! This module provides a centralized error handling system for the Pierre MCP Server.
//! It defines standard error types, error codes, and HTTP response formatting to ensure
//! consistent error handling across all modules and APIs.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;
use uuid::Uuid;
use warp::reject::Reject;

/// Standard error codes used throughout the application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    // Authentication & Authorization (1000-1999)
    #[serde(rename = "AUTH_REQUIRED")]
    AuthRequired = 1000,
    #[serde(rename = "AUTH_INVALID")]
    AuthInvalid = 1001,
    #[serde(rename = "AUTH_EXPIRED")]
    AuthExpired = 1002,
    #[serde(rename = "AUTH_MALFORMED")]
    AuthMalformed = 1003,
    #[serde(rename = "PERMISSION_DENIED")]
    PermissionDenied = 1004,

    // Rate Limiting (2000-2999)
    #[serde(rename = "RATE_LIMIT_EXCEEDED")]
    RateLimitExceeded = 2000,
    #[serde(rename = "QUOTA_EXCEEDED")]
    QuotaExceeded = 2001,

    // Validation (3000-3999)
    #[serde(rename = "INVALID_INPUT")]
    InvalidInput = 3000,
    #[serde(rename = "MISSING_REQUIRED_FIELD")]
    MissingRequiredField = 3001,
    #[serde(rename = "INVALID_FORMAT")]
    InvalidFormat = 3002,
    #[serde(rename = "VALUE_OUT_OF_RANGE")]
    ValueOutOfRange = 3003,

    // Resource Management (4000-4999)
    #[serde(rename = "RESOURCE_NOT_FOUND")]
    ResourceNotFound = 4000,
    #[serde(rename = "RESOURCE_ALREADY_EXISTS")]
    ResourceAlreadyExists = 4001,
    #[serde(rename = "RESOURCE_LOCKED")]
    ResourceLocked = 4002,
    #[serde(rename = "RESOURCE_UNAVAILABLE")]
    ResourceUnavailable = 4003,

    // External Services (5000-5999)
    #[serde(rename = "EXTERNAL_SERVICE_ERROR")]
    ExternalServiceError = 5000,
    #[serde(rename = "EXTERNAL_SERVICE_UNAVAILABLE")]
    ExternalServiceUnavailable = 5001,
    #[serde(rename = "EXTERNAL_AUTH_FAILED")]
    ExternalAuthFailed = 5002,
    #[serde(rename = "EXTERNAL_RATE_LIMITED")]
    ExternalRateLimited = 5003,

    // Configuration (6000-6999)
    #[serde(rename = "CONFIG_ERROR")]
    ConfigError = 6000,
    #[serde(rename = "CONFIG_MISSING")]
    ConfigMissing = 6001,
    #[serde(rename = "CONFIG_INVALID")]
    ConfigInvalid = 6002,

    // Internal Errors (9000-9999)
    #[serde(rename = "INTERNAL_ERROR")]
    InternalError = 9000,
    #[serde(rename = "DATABASE_ERROR")]
    DatabaseError = 9001,
    #[serde(rename = "STORAGE_ERROR")]
    StorageError = 9002,
    #[serde(rename = "SERIALIZATION_ERROR")]
    SerializationError = 9003,
}

impl ErrorCode {
    /// Get the HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        match self {
            // 400 Bad Request
            ErrorCode::InvalidInput
            | ErrorCode::MissingRequiredField
            | ErrorCode::InvalidFormat
            | ErrorCode::ValueOutOfRange => 400,

            // 401 Unauthorized
            ErrorCode::AuthRequired | ErrorCode::AuthInvalid => 401,

            // 403 Forbidden
            ErrorCode::AuthExpired | ErrorCode::AuthMalformed | ErrorCode::PermissionDenied => 403,

            // 404 Not Found
            ErrorCode::ResourceNotFound => 404,

            // 409 Conflict
            ErrorCode::ResourceAlreadyExists | ErrorCode::ResourceLocked => 409,

            // 429 Too Many Requests
            ErrorCode::RateLimitExceeded | ErrorCode::QuotaExceeded => 429,

            // 502 Bad Gateway
            ErrorCode::ExternalServiceError | ErrorCode::ExternalServiceUnavailable => 502,

            // 503 Service Unavailable
            ErrorCode::ResourceUnavailable
            | ErrorCode::ExternalAuthFailed
            | ErrorCode::ExternalRateLimited => 503,

            // 500 Internal Server Error
            ErrorCode::InternalError
            | ErrorCode::DatabaseError
            | ErrorCode::StorageError
            | ErrorCode::SerializationError
            | ErrorCode::ConfigError
            | ErrorCode::ConfigMissing
            | ErrorCode::ConfigInvalid => 500,
        }
    }

    /// Get a user-friendly description of this error
    pub fn description(&self) -> &'static str {
        match self {
            ErrorCode::AuthRequired => "Authentication is required to access this resource",
            ErrorCode::AuthInvalid => "The provided authentication credentials are invalid",
            ErrorCode::AuthExpired => "The authentication token has expired",
            ErrorCode::AuthMalformed => "The authentication token is malformed or corrupted",
            ErrorCode::PermissionDenied => "You do not have permission to perform this action",
            ErrorCode::RateLimitExceeded => "Rate limit exceeded. Please slow down your requests",
            ErrorCode::QuotaExceeded => "Usage quota exceeded for your current plan",
            ErrorCode::InvalidInput => "The provided input is invalid",
            ErrorCode::MissingRequiredField => "A required field is missing from the request",
            ErrorCode::InvalidFormat => "The data format is invalid",
            ErrorCode::ValueOutOfRange => "The provided value is outside the acceptable range",
            ErrorCode::ResourceNotFound => "The requested resource was not found",
            ErrorCode::ResourceAlreadyExists => "A resource with this identifier already exists",
            ErrorCode::ResourceLocked => "The resource is currently locked and cannot be modified",
            ErrorCode::ResourceUnavailable => "The resource is temporarily unavailable",
            ErrorCode::ExternalServiceError => "An external service encountered an error",
            ErrorCode::ExternalServiceUnavailable => "An external service is currently unavailable",
            ErrorCode::ExternalAuthFailed => "Authentication with external service failed",
            ErrorCode::ExternalRateLimited => "External service rate limit exceeded",
            ErrorCode::ConfigError => "Configuration error encountered",
            ErrorCode::ConfigMissing => "Required configuration is missing",
            ErrorCode::ConfigInvalid => "Configuration is invalid",
            ErrorCode::InternalError => "An internal server error occurred",
            ErrorCode::DatabaseError => "Database operation failed",
            ErrorCode::StorageError => "Storage operation failed",
            ErrorCode::SerializationError => "Data serialization/deserialization failed",
        }
    }
}

/// Additional context that can be attached to errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Request ID for tracing
    pub request_id: Option<String>,
    /// User ID if available
    pub user_id: Option<Uuid>,
    /// Resource ID if applicable
    pub resource_id: Option<String>,
    /// Additional key-value context
    pub details: serde_json::Value,
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self {
            request_id: None,
            user_id: None,
            resource_id: None,
            details: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
}

/// Unified error type for the application
#[derive(Debug, Error)]
pub struct AppError {
    /// Error code
    pub code: ErrorCode,
    /// Human-readable error message
    pub message: String,
    /// Additional context
    pub context: ErrorContext,
    /// Source error for error chaining
    #[source]
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl AppError {
    /// Create a new AppError with the given code and message
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            context: ErrorContext::default(),
            source: None,
        }
    }

    /// Create an AppError with additional context
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }

    /// Add a request ID to the error context
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.context.request_id = Some(request_id.into());
        self
    }

    /// Add a user ID to the error context
    pub fn with_user_id(mut self, user_id: Uuid) -> Self {
        self.context.user_id = Some(user_id);
        self
    }

    /// Add a resource ID to the error context
    pub fn with_resource_id(mut self, resource_id: impl Into<String>) -> Self {
        self.context.resource_id = Some(resource_id.into());
        self
    }

    /// Add details to the error context
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.context.details = details;
        self
    }

    /// Add a source error for error chaining
    pub fn with_source(mut self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        self.source = Some(Box::new(source));
        self
    }

    /// Get the HTTP status code for this error
    pub fn http_status(&self) -> u16 {
        self.code.http_status()
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code.description(), self.message)
    }
}

/// Implement Reject for Warp framework integration
impl Reject for AppError {}

/// Result type alias for convenience
pub type AppResult<T> = Result<T, AppError>;

/// HTTP error response format
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorResponseDetails,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponseDetails {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub details: serde_json::Value,
}

impl From<AppError> for ErrorResponse {
    fn from(error: AppError) -> Self {
        Self {
            error: ErrorResponseDetails {
                code: error.code,
                message: error.message,
                request_id: error.context.request_id,
                details: error.context.details,
            },
        }
    }
}

/// Convenience functions for creating common errors
impl AppError {
    /// Authentication required
    pub fn auth_required() -> Self {
        Self::new(ErrorCode::AuthRequired, "Authentication required")
    }

    /// Invalid authentication
    pub fn auth_invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::AuthInvalid, message)
    }

    /// Authentication expired
    pub fn auth_expired() -> Self {
        Self::new(ErrorCode::AuthExpired, "Authentication token has expired")
    }

    /// Rate limit exceeded
    pub fn rate_limit_exceeded(limit: u32, reset_at: chrono::DateTime<chrono::Utc>) -> Self {
        Self::new(
            ErrorCode::RateLimitExceeded,
            format!("Rate limit of {} requests exceeded", limit),
        )
        .with_details(serde_json::json!({
            "limit": limit,
            "reset_at": reset_at.to_rfc3339()
        }))
    }

    /// Resource not found
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ResourceNotFound,
            format!("{} not found", resource.into()),
        )
    }

    /// Invalid input
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }

    /// Internal server error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, message)
    }

    /// Database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::DatabaseError, message)
    }

    /// Configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ConfigError, message)
    }

    /// External service error
    pub fn external_service(service: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ExternalServiceError,
            format!("{}: {}", service.into(), message.into()),
        )
    }
}

/// Conversion from anyhow::Error to AppError
impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        // Extract the root cause if available for better error chaining
        match error.source() {
            Some(source) => AppError::new(ErrorCode::InternalError, error.to_string())
                .with_details(serde_json::json!({
                    "source": source.to_string()
                })),
            None => AppError::new(ErrorCode::InternalError, error.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_http_status() {
        assert_eq!(ErrorCode::AuthRequired.http_status(), 401);
        assert_eq!(ErrorCode::RateLimitExceeded.http_status(), 429);
        assert_eq!(ErrorCode::ResourceNotFound.http_status(), 404);
        assert_eq!(ErrorCode::InternalError.http_status(), 500);
    }

    #[test]
    fn test_app_error_creation() {
        let error = AppError::auth_required()
            .with_request_id("req-123")
            .with_user_id(Uuid::new_v4());

        assert_eq!(error.code, ErrorCode::AuthRequired);
        assert!(error.context.request_id.is_some());
        assert!(error.context.user_id.is_some());
    }

    #[test]
    fn test_error_response_serialization() {
        let error = AppError::rate_limit_exceeded(1000, chrono::Utc::now());
        let response = ErrorResponse::from(error);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("RATE_LIMIT_EXCEEDED"));
        assert!(json.contains("limit"));
    }
}
