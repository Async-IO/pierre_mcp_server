// ABOUTME: Unit tests for errors functionality
// ABOUTME: Validates errors behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::Utc;
use pierre_mcp_server::errors::{AppError, ErrorCode, ErrorResponse};

#[test]
fn test_error_code_http_status() {
    assert_eq!(ErrorCode::AuthRequired.http_status(), 401);
    assert_eq!(ErrorCode::RateLimitExceeded.http_status(), 429);
    assert_eq!(ErrorCode::ResourceNotFound.http_status(), 404);
    assert_eq!(ErrorCode::InternalError.http_status(), 500);
}

#[test]
fn test_app_error_creation() {
    let error = AppError::auth_required().with_request_id("req-123");

    assert_eq!(error.code, ErrorCode::AuthRequired);
    assert!(error.request_id.is_some());
}

#[test]
fn test_error_response_serialization() {
    let error = AppError::rate_limit_exceeded(1000);
    let response = ErrorResponse::from(error);

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("RateLimitExceeded"));
}

// Comprehensive Error Code HTTP Status Tests

#[allow(clippy::cognitive_complexity)]
#[test]
fn test_error_code_http_status_comprehensive() {
    // Test 400 Bad Request errors
    assert_eq!(ErrorCode::InvalidInput.http_status(), 400);
    assert_eq!(ErrorCode::MissingRequiredField.http_status(), 400);
    assert_eq!(ErrorCode::InvalidFormat.http_status(), 400);
    assert_eq!(ErrorCode::ValueOutOfRange.http_status(), 400);

    // Test 401 Unauthorized errors
    assert_eq!(ErrorCode::AuthRequired.http_status(), 401);
    assert_eq!(ErrorCode::AuthInvalid.http_status(), 401);

    // Test 403 Forbidden errors
    assert_eq!(ErrorCode::AuthExpired.http_status(), 403);
    assert_eq!(ErrorCode::AuthMalformed.http_status(), 403);
    assert_eq!(ErrorCode::PermissionDenied.http_status(), 403);

    // Test 404 Not Found errors
    assert_eq!(ErrorCode::ResourceNotFound.http_status(), 404);

    // Test 409 Conflict errors
    assert_eq!(ErrorCode::ResourceAlreadyExists.http_status(), 409);
    assert_eq!(ErrorCode::ResourceLocked.http_status(), 409);

    // Test 429 Too Many Requests errors
    assert_eq!(ErrorCode::RateLimitExceeded.http_status(), 429);
    assert_eq!(ErrorCode::QuotaExceeded.http_status(), 429);

    // Test 502 Bad Gateway errors
    assert_eq!(ErrorCode::ExternalServiceError.http_status(), 502);
    assert_eq!(ErrorCode::ExternalServiceUnavailable.http_status(), 502);

    // Test 503 Service Unavailable errors
    assert_eq!(ErrorCode::ResourceUnavailable.http_status(), 503);
    assert_eq!(ErrorCode::ExternalAuthFailed.http_status(), 503);
    assert_eq!(ErrorCode::ExternalRateLimited.http_status(), 503);

    // Test 500 Internal Server Error errors
    assert_eq!(ErrorCode::InternalError.http_status(), 500);
    assert_eq!(ErrorCode::DatabaseError.http_status(), 500);
    assert_eq!(ErrorCode::StorageError.http_status(), 500);
    assert_eq!(ErrorCode::SerializationError.http_status(), 500);
    assert_eq!(ErrorCode::ConfigError.http_status(), 500);
    assert_eq!(ErrorCode::ConfigMissing.http_status(), 500);
    assert_eq!(ErrorCode::ConfigInvalid.http_status(), 500);
}

#[test]
fn test_error_code_description() {
    assert_eq!(
        ErrorCode::AuthRequired.description(),
        "Authentication is required to access this resource"
    );
    assert_eq!(
        ErrorCode::AuthInvalid.description(),
        "The provided authentication credentials are invalid"
    );
    assert_eq!(
        ErrorCode::RateLimitExceeded.description(),
        "Rate limit exceeded. Please slow down your requests"
    );
    assert_eq!(
        ErrorCode::ResourceNotFound.description(),
        "The requested resource was not found"
    );
    assert_eq!(
        ErrorCode::InternalError.description(),
        "An internal server error occurred"
    );
}

#[test]
fn test_error_code_serialization() {
    let auth_required = ErrorCode::AuthRequired;
    let json = serde_json::to_string(&auth_required).unwrap();
    assert_eq!(json, "\"AuthRequired\"");

    let database_error = ErrorCode::DatabaseError;
    let json = serde_json::to_string(&database_error).unwrap();
    assert_eq!(json, "\"DatabaseError\"");

    let external_service_error = ErrorCode::ExternalServiceError;
    let json = serde_json::to_string(&external_service_error).unwrap();
    assert_eq!(json, "\"ExternalServiceError\"");
}

#[test]
fn test_error_code_deserialization() {
    let auth_required: ErrorCode = serde_json::from_str("\"AuthRequired\"").unwrap();
    assert_eq!(auth_required, ErrorCode::AuthRequired);

    let database_error: ErrorCode = serde_json::from_str("\"DatabaseError\"").unwrap();
    assert_eq!(database_error, ErrorCode::DatabaseError);

    let invalid_input: ErrorCode = serde_json::from_str("\"InvalidInput\"").unwrap();
    assert_eq!(invalid_input, ErrorCode::InvalidInput);

    // Test invalid error code deserialization
    let result: Result<ErrorCode, _> = serde_json::from_str("\"UnknownError\"");
    assert!(result.is_err());
}

#[test]
fn test_error_code_serialization_roundtrip() {
    let original_codes = vec![
        ErrorCode::AuthRequired,
        ErrorCode::AuthInvalid,
        ErrorCode::AuthExpired,
        ErrorCode::RateLimitExceeded,
        ErrorCode::InvalidInput,
        ErrorCode::ResourceNotFound,
        ErrorCode::InternalError,
        ErrorCode::DatabaseError,
        ErrorCode::ExternalServiceError,
    ];

    for original_code in original_codes {
        let json = serde_json::to_string(&original_code).unwrap();
        let deserialized_code: ErrorCode = serde_json::from_str(&json).unwrap();
        assert_eq!(original_code, deserialized_code);
    }
}

// AppError Tests

#[test]
fn test_app_error_new() {
    let error = AppError::new(ErrorCode::AuthRequired, "Test message");

    assert_eq!(error.code, ErrorCode::AuthRequired);
    assert_eq!(error.message, "Test message");
    assert_eq!(error.request_id, None);
}

#[test]
fn test_app_error_with_request_id() {
    let error = AppError::new(ErrorCode::InvalidInput, "Test message").with_request_id("req_123");

    assert_eq!(error.code, ErrorCode::InvalidInput);
    assert_eq!(error.message, "Test message");
    assert_eq!(error.request_id, Some("req_123".to_string()));
}

#[test]
fn test_app_error_http_status() {
    let auth_error = AppError::new(ErrorCode::AuthRequired, "Test");
    assert_eq!(auth_error.http_status(), 401);

    let not_found_error = AppError::new(ErrorCode::ResourceNotFound, "Test");
    assert_eq!(not_found_error.http_status(), 404);

    let internal_error = AppError::new(ErrorCode::InternalError, "Test");
    assert_eq!(internal_error.http_status(), 500);
}

#[test]
fn test_app_error_display() {
    let error = AppError::new(ErrorCode::AuthRequired, "Please provide a valid token");
    let display_string = format!("{error}");
    assert_eq!(
        display_string,
        "Authentication is required to access this resource: Please provide a valid token"
    );

    let error = AppError::new(ErrorCode::ResourceNotFound, "User with ID 123 not found");
    let display_string = format!("{error}");
    assert_eq!(
        display_string,
        "The requested resource was not found: User with ID 123 not found"
    );
}

#[test]
fn test_app_error_convenience_methods() {
    let auth_required = AppError::auth_required();
    assert_eq!(auth_required.code, ErrorCode::AuthRequired);
    assert_eq!(auth_required.message, "Authentication required");

    let auth_invalid = AppError::auth_invalid("Invalid token");
    assert_eq!(auth_invalid.code, ErrorCode::AuthInvalid);
    assert_eq!(auth_invalid.message, "Invalid token");

    let auth_expired = AppError::auth_expired();
    assert_eq!(auth_expired.code, ErrorCode::AuthExpired);
    assert_eq!(auth_expired.message, "Authentication token has expired");

    let rate_limit = AppError::rate_limit_exceeded(100);
    assert_eq!(rate_limit.code, ErrorCode::RateLimitExceeded);
    assert_eq!(rate_limit.message, "Rate limit of 100 requests exceeded");

    let not_found = AppError::not_found("User");
    assert_eq!(not_found.code, ErrorCode::ResourceNotFound);
    assert_eq!(not_found.message, "User not found");

    let invalid_input = AppError::invalid_input("Invalid email format");
    assert_eq!(invalid_input.code, ErrorCode::InvalidInput);
    assert_eq!(invalid_input.message, "Invalid email format");

    let internal = AppError::internal("Database connection failed");
    assert_eq!(internal.code, ErrorCode::InternalError);
    assert_eq!(internal.message, "Database connection failed");

    let database = AppError::database("Query timeout");
    assert_eq!(database.code, ErrorCode::DatabaseError);
    assert_eq!(database.message, "Query timeout");

    let config = AppError::config("Missing API key");
    assert_eq!(config.code, ErrorCode::ConfigError);
    assert_eq!(config.message, "Missing API key");

    let external = AppError::external_service("Strava", "API unavailable");
    assert_eq!(external.code, ErrorCode::ExternalServiceError);
    assert_eq!(external.message, "Strava: API unavailable");
}

// ErrorResponse Tests

#[test]
fn test_error_response_from_app_error() {
    let app_error =
        AppError::new(ErrorCode::AuthRequired, "Test message").with_request_id("req_123");

    let error_response = ErrorResponse::from(app_error);

    assert_eq!(error_response.code, ErrorCode::AuthRequired);
    // Auth errors use generic description for security, not the specific message
    assert_eq!(error_response.message, "Authentication is required to access this resource");
    assert_eq!(error_response.request_id, Some("req_123".to_string()));

    // Check that timestamp is valid RFC3339 format
    let parsed_timestamp = chrono::DateTime::parse_from_rfc3339(&error_response.timestamp);
    assert!(parsed_timestamp.is_ok());

    // Timestamp should be recent (within last minute)
    let timestamp_utc = parsed_timestamp.unwrap().with_timezone(&Utc);
    let now = Utc::now();
    let diff = now.signed_duration_since(timestamp_utc);
    assert!(diff.num_seconds() < 60);
}

#[test]
fn test_error_response_serialization_comprehensive() {
    let app_error = AppError::new(ErrorCode::InvalidInput, "Field 'email' is required")
        .with_request_id("req_456");

    let error_response = ErrorResponse::from(app_error);
    let json = serde_json::to_string(&error_response).unwrap();

    // Check that all fields are present in JSON
    assert!(json.contains("\"code\":\"InvalidInput\""));
    assert!(json.contains("\"message\":\"Field 'email' is required\""));
    assert!(json.contains("\"request_id\":\"req_456\""));
    assert!(json.contains("\"timestamp\":"));
}

#[test]
fn test_error_response_without_request_id() {
    let app_error = AppError::new(ErrorCode::InternalError, "Something went wrong");
    let error_response = ErrorResponse::from(app_error);

    assert_eq!(error_response.request_id, None);

    let json = serde_json::to_string(&error_response).unwrap();
    // request_id should not appear in JSON when None
    assert!(!json.contains("request_id"));
}

// Error Conversion Tests

#[test]
fn test_app_error_from_anyhow_error() {
    let anyhow_error = anyhow::anyhow!("Test anyhow error");
    let app_error = AppError::from(anyhow_error);

    assert_eq!(app_error.code, ErrorCode::InternalError);
    assert_eq!(app_error.message, "Test anyhow error");
}

#[test]
fn test_app_error_from_io_error() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let app_error = AppError::from(io_error);

    assert_eq!(app_error.code, ErrorCode::InternalError);
    assert!(app_error.message.contains("IO error"));
    assert!(app_error.message.contains("File not found"));
}

// Result Type Alias Test

#[test]
fn test_app_result_type_alias() {
    #[allow(clippy::unnecessary_wraps)]
    fn successful_operation() -> pierre_mcp_server::errors::AppResult<String> {
        Ok("Success".to_string())
    }

    fn failed_operation() -> pierre_mcp_server::errors::AppResult<String> {
        Err(AppError::auth_required())
    }

    let success_result = successful_operation();
    assert!(success_result.is_ok());
    assert_eq!(success_result.unwrap(), "Success");

    let error_result = failed_operation();
    assert!(error_result.is_err());
    let error = error_result.unwrap_err();
    assert_eq!(error.code, ErrorCode::AuthRequired);
}

// Debug and Display Tests

#[test]
fn test_app_error_debug_format() {
    let error =
        AppError::new(ErrorCode::DatabaseError, "Connection failed").with_request_id("req_789");

    let debug_string = format!("{error:?}");
    assert!(debug_string.contains("AppError"));
    assert!(debug_string.contains("DatabaseError"));
    assert!(debug_string.contains("Connection failed"));
    assert!(debug_string.contains("req_789"));
}

#[test]
fn test_error_code_equality() {
    assert_eq!(ErrorCode::AuthRequired, ErrorCode::AuthRequired);
    assert_ne!(ErrorCode::AuthRequired, ErrorCode::AuthInvalid);
    assert_ne!(ErrorCode::DatabaseError, ErrorCode::InternalError);
}

#[test]
fn test_error_code_clone() {
    let original = ErrorCode::ExternalServiceError;
    let cloned = original;
    assert_eq!(original, cloned);
}

#[test]
fn test_error_code_copy() {
    let original = ErrorCode::InvalidInput;
    let copied = original;
    // Both should be usable after copy
    assert_eq!(original, ErrorCode::InvalidInput);
    assert_eq!(copied, ErrorCode::InvalidInput);
}

// Edge Case Tests

#[test]
fn test_app_error_chaining() {
    let error = AppError::auth_required()
        .with_request_id("req_1")
        .with_request_id("req_2"); // Should overwrite

    assert_eq!(error.request_id, Some("req_2".to_string()));
}

#[test]
fn test_error_response_timestamp_format() {
    let app_error = AppError::internal("Test timestamp");
    let error_response = ErrorResponse::from(app_error);

    // Timestamp should be in RFC3339 format
    let parsed = chrono::DateTime::parse_from_rfc3339(&error_response.timestamp);
    assert!(parsed.is_ok(), "Timestamp should be valid RFC3339 format");

    // Should be a recent timestamp
    let parsed_utc = parsed.unwrap().with_timezone(&Utc);
    let now = Utc::now();
    let duration_since = now.signed_duration_since(parsed_utc);
    assert!(
        duration_since.num_seconds() < 10,
        "Timestamp should be very recent"
    );
}

#[test]
fn test_multiple_error_types_serialization() {
    let errors = vec![
        AppError::auth_required(),
        AppError::rate_limit_exceeded(1000),
        AppError::not_found("Resource"),
        AppError::internal("Server error"),
        AppError::database("Connection timeout"),
    ];

    for error in errors {
        let response = ErrorResponse::from(error);
        let json = serde_json::to_string(&response).unwrap();

        // Just verify the JSON contains expected fields (can't deserialize ErrorResponse)
        assert!(json.contains("\"code\":"));
        assert!(json.contains("\"message\":"));
        assert!(json.contains("\"timestamp\":"));
    }
}
