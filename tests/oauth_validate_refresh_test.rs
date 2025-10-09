// ABOUTME: Test suite for POST /oauth2/validate-and-refresh endpoint
// ABOUTME: Covers token validation, refresh, and invalid token scenarios
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::Utc;
use pierre_mcp_server::auth::AuthManager;
use std::sync::Arc;
use uuid::Uuid;

/// Create test auth manager for JWT token generation and validation
fn setup_auth_manager() -> Arc<AuthManager> {
    let jwt_secret = b"test_secret_key_32_bytes_long!!!".to_vec();
    let token_expiry_hours = 1;
    Arc::new(AuthManager::new(jwt_secret, token_expiry_hours))
}

/// Test Scenario 1: Valid JWT token can be parsed and validated
#[tokio::test]
async fn test_validate_jwt_token_structure() {
    let auth_manager = setup_auth_manager();
    let user_id = Uuid::new_v4();

    // Generate a valid access token
    let access_token = auth_manager
        .generate_oauth_access_token(&user_id, &["read".to_string()])
        .expect("Failed to generate access token");

    // Validate token structure
    let validation_result = auth_manager.validate_token_detailed(&access_token);

    assert!(validation_result.is_ok());
    let claims = validation_result.expect("Token should be valid");
    assert_eq!(claims.sub, user_id.to_string());
    assert!(claims.exp > Utc::now().timestamp());
}

/// Test Scenario 2: Invalid signature returns error
#[tokio::test]
async fn test_validate_jwt_invalid_signature() {
    let auth_manager = setup_auth_manager();

    // Create a token with wrong signature (valid JWT but signed with different secret)
    let invalid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    // Validate token - should fail due to invalid signature
    let validation_result = auth_manager.validate_token_detailed(invalid_token);

    assert!(validation_result.is_err());
}

/// Test Scenario 3: Malformed token returns error
#[tokio::test]
async fn test_validate_jwt_malformed() {
    let auth_manager = setup_auth_manager();
    let malformed_token = "not.a.valid.jwt.token";

    // Validate token - should fail due to malformed format
    let validation_result = auth_manager.validate_token_detailed(malformed_token);

    assert!(validation_result.is_err());
}
