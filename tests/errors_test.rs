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
