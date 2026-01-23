// ABOUTME: Unit tests for OAuth2 server models and error handling
// ABOUTME: Validates OAuth2 data structures, error types, and serialization
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::{Duration, Utc};
use pierre_mcp_server::oauth2_server::models::{
    AuthorizeRequest, AuthorizeResponse, ClientRegistrationRequest, ClientRegistrationResponse,
    OAuth2AccessToken, OAuth2AuthCode, OAuth2Client, OAuth2Error, OAuth2RefreshToken, OAuth2State,
    TokenRequest, TokenResponse, ValidateRefreshRequest, ValidateRefreshResponse, ValidationStatus,
};
use uuid::Uuid;

// =============================================================================
// OAuth2Error Tests
// =============================================================================

#[test]
fn test_oauth2_error_invalid_request() {
    let error = OAuth2Error::invalid_request("Missing required parameter");

    assert_eq!(error.error, "invalid_request");
    assert_eq!(
        error.error_description,
        Some("Missing required parameter".to_owned())
    );
    assert!(error.error_uri.is_some());
    assert!(error.error_uri.unwrap().contains("rfc6749"));
}

#[test]
fn test_oauth2_error_invalid_client() {
    let error = OAuth2Error::invalid_client();

    assert_eq!(error.error, "invalid_client");
    assert_eq!(
        error.error_description,
        Some("Client authentication failed".to_owned())
    );
    assert!(error.error_uri.is_some());
}

#[test]
fn test_oauth2_error_invalid_grant() {
    let error = OAuth2Error::invalid_grant("Authorization code expired");

    assert_eq!(error.error, "invalid_grant");
    assert_eq!(
        error.error_description,
        Some("Authorization code expired".to_owned())
    );
}

#[test]
fn test_oauth2_error_unsupported_grant_type() {
    let error = OAuth2Error::unsupported_grant_type();

    assert_eq!(error.error, "unsupported_grant_type");
    assert_eq!(
        error.error_description,
        Some("Grant type not supported".to_owned())
    );
}

#[test]
fn test_oauth2_error_serialization() {
    let error = OAuth2Error::invalid_request("Test error");

    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("\"error\":\"invalid_request\""));
    assert!(json.contains("\"error_description\":\"Test error\""));
}

// =============================================================================
// AuthorizeRequest Tests
// =============================================================================

#[test]
fn test_authorize_request_deserialization() {
    let json = r#"{
        "response_type": "code",
        "client_id": "test_client_123",
        "redirect_uri": "https://example.com/callback",
        "scope": "read write",
        "state": "random_state_value",
        "code_challenge": "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        "code_challenge_method": "S256"
    }"#;

    let request: AuthorizeRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.response_type, "code");
    assert_eq!(request.client_id, "test_client_123");
    assert_eq!(request.redirect_uri, "https://example.com/callback");
    assert_eq!(request.scope, Some("read write".to_owned()));
    assert_eq!(request.state, Some("random_state_value".to_owned()));
    assert!(request.code_challenge.is_some());
    assert_eq!(request.code_challenge_method, Some("S256".to_owned()));
}

#[test]
fn test_authorize_request_minimal() {
    let json = r#"{
        "response_type": "code",
        "client_id": "minimal_client",
        "redirect_uri": "https://example.com/cb"
    }"#;

    let request: AuthorizeRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.response_type, "code");
    assert_eq!(request.client_id, "minimal_client");
    assert!(request.scope.is_none());
    assert!(request.state.is_none());
    assert!(request.code_challenge.is_none());
}

#[test]
fn test_authorize_request_clone() {
    let request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: "client".to_owned(),
        redirect_uri: "https://example.com".to_owned(),
        scope: Some("read".to_owned()),
        state: Some("state".to_owned()),
        code_challenge: None,
        code_challenge_method: None,
    };

    let cloned = request.clone();
    assert_eq!(request.client_id, cloned.client_id);
    assert_eq!(request.scope, cloned.scope);
}

// =============================================================================
// AuthorizeResponse Tests
// =============================================================================

#[test]
fn test_authorize_response_serialization() {
    let response = AuthorizeResponse {
        code: "auth_code_12345".to_owned(),
        state: Some("csrf_state".to_owned()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"code\":\"auth_code_12345\""));
    assert!(json.contains("\"state\":\"csrf_state\""));
}

#[test]
fn test_authorize_response_without_state() {
    let response = AuthorizeResponse {
        code: "auth_code".to_owned(),
        state: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"code\":\"auth_code\""));
    // state should be present as null
    assert!(json.contains("\"state\":null"));
}

// =============================================================================
// TokenRequest Tests
// =============================================================================

#[test]
fn test_token_request_authorization_code() {
    let json = r#"{
        "grant_type": "authorization_code",
        "code": "auth_code_xyz",
        "redirect_uri": "https://example.com/callback",
        "client_id": "client_123",
        "client_secret": "secret_456",
        "code_verifier": "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    }"#;

    let request: TokenRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.grant_type, "authorization_code");
    assert_eq!(request.code, Some("auth_code_xyz".to_owned()));
    assert_eq!(request.client_id, "client_123");
    assert_eq!(request.client_secret, "secret_456");
    assert!(request.code_verifier.is_some());
}

#[test]
fn test_token_request_client_credentials() {
    let json = r#"{
        "grant_type": "client_credentials",
        "client_id": "service_client",
        "client_secret": "service_secret",
        "scope": "admin"
    }"#;

    let request: TokenRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.grant_type, "client_credentials");
    assert!(request.code.is_none());
    assert_eq!(request.scope, Some("admin".to_owned()));
}

#[test]
fn test_token_request_refresh_token() {
    let json = r#"{
        "grant_type": "refresh_token",
        "client_id": "client",
        "client_secret": "secret",
        "refresh_token": "refresh_token_abc123"
    }"#;

    let request: TokenRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.grant_type, "refresh_token");
    assert_eq!(
        request.refresh_token,
        Some("refresh_token_abc123".to_owned())
    );
}

// =============================================================================
// TokenResponse Tests
// =============================================================================

#[test]
fn test_token_response_serialization() {
    let response = TokenResponse {
        access_token: "eyJhbGciOiJSUzI1NiIsInR5cCI6IkpXVCJ9...".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 3600,
        scope: Some("read write".to_owned()),
        refresh_token: Some("refresh_token_xyz".to_owned()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"token_type\":\"Bearer\""));
    assert!(json.contains("\"expires_in\":3600"));
    assert!(json.contains("refresh_token"));
}

#[test]
fn test_token_response_without_refresh() {
    let response = TokenResponse {
        access_token: "access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 1800,
        scope: None,
        refresh_token: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"expires_in\":1800"));
}

// =============================================================================
// ClientRegistrationRequest Tests
// =============================================================================

#[test]
fn test_client_registration_request() {
    let json = r#"{
        "redirect_uris": ["https://app.example.com/callback", "https://app.example.com/oauth"],
        "client_name": "My Application",
        "client_uri": "https://app.example.com",
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "scope": "read write admin"
    }"#;

    let request: ClientRegistrationRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.redirect_uris.len(), 2);
    assert_eq!(request.client_name, Some("My Application".to_owned()));
    assert_eq!(
        request.grant_types,
        Some(vec![
            "authorization_code".to_owned(),
            "refresh_token".to_owned()
        ])
    );
}

#[test]
fn test_client_registration_request_minimal() {
    let json = r#"{
        "redirect_uris": ["https://example.com/cb"]
    }"#;

    let request: ClientRegistrationRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.redirect_uris.len(), 1);
    assert!(request.client_name.is_none());
    assert!(request.grant_types.is_none());
}

// =============================================================================
// ClientRegistrationResponse Tests
// =============================================================================

#[test]
fn test_client_registration_response() {
    let response = ClientRegistrationResponse {
        client_id: "new_client_id".to_owned(),
        client_secret: "generated_secret".to_owned(),
        client_id_issued_at: Some(1_700_000_000),
        client_secret_expires_at: None,
        redirect_uris: vec!["https://example.com/cb".to_owned()],
        grant_types: vec!["authorization_code".to_owned()],
        response_types: vec!["code".to_owned()],
        client_name: Some("Test App".to_owned()),
        client_uri: None,
        scope: Some("read".to_owned()),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("new_client_id"));
    assert!(json.contains("generated_secret"));
}

// =============================================================================
// OAuth2Client Tests
// =============================================================================

#[test]
fn test_oauth2_client_creation() {
    let client = OAuth2Client {
        id: "internal_id_1".to_owned(),
        client_id: "public_client_id".to_owned(),
        client_secret_hash: "hashed_secret".to_owned(),
        redirect_uris: vec!["https://app.com/callback".to_owned()],
        grant_types: vec!["authorization_code".to_owned(), "refresh_token".to_owned()],
        response_types: vec!["code".to_owned()],
        client_name: Some("Test Application".to_owned()),
        client_uri: Some("https://app.com".to_owned()),
        scope: Some("read write".to_owned()),
        created_at: Utc::now(),
        expires_at: None,
    };

    assert_eq!(client.client_id, "public_client_id");
    assert_eq!(client.redirect_uris.len(), 1);
    assert_eq!(client.grant_types.len(), 2);
}

#[test]
fn test_oauth2_client_clone() {
    let client = OAuth2Client {
        id: "id".to_owned(),
        client_id: "client".to_owned(),
        client_secret_hash: "hash".to_owned(),
        redirect_uris: vec!["https://example.com".to_owned()],
        grant_types: vec!["authorization_code".to_owned()],
        response_types: vec!["code".to_owned()],
        client_name: None,
        client_uri: None,
        scope: None,
        created_at: Utc::now(),
        expires_at: Some(Utc::now() + Duration::days(365)),
    };

    let cloned = client.clone();
    assert_eq!(client.client_id, cloned.client_id);
    assert_eq!(client.redirect_uris, cloned.redirect_uris);
}

// =============================================================================
// OAuth2AuthCode Tests
// =============================================================================

#[test]
fn test_oauth2_auth_code_creation() {
    let user_id = Uuid::new_v4();
    let auth_code = OAuth2AuthCode {
        code: "auth_code_abc123".to_owned(),
        client_id: "client_123".to_owned(),
        user_id,
        tenant_id: "tenant_456".to_owned(),
        redirect_uri: "https://app.com/callback".to_owned(),
        scope: Some("read write".to_owned()),
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
        state: Some("csrf_state".to_owned()),
        code_challenge: Some("E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM".to_owned()),
        code_challenge_method: Some("S256".to_owned()),
    };

    assert_eq!(auth_code.code, "auth_code_abc123");
    assert_eq!(auth_code.user_id, user_id);
    assert!(!auth_code.used);
    assert!(auth_code.code_challenge.is_some());
}

#[test]
fn test_oauth2_auth_code_clone() {
    let auth_code = OAuth2AuthCode {
        code: "code".to_owned(),
        client_id: "client".to_owned(),
        user_id: Uuid::new_v4(),
        tenant_id: "tenant".to_owned(),
        redirect_uri: "https://example.com".to_owned(),
        scope: None,
        expires_at: Utc::now(),
        used: false,
        state: None,
        code_challenge: None,
        code_challenge_method: None,
    };

    let cloned = auth_code.clone();
    assert_eq!(auth_code.code, cloned.code);
    assert_eq!(auth_code.user_id, cloned.user_id);
}

// =============================================================================
// OAuth2AccessToken Tests
// =============================================================================

#[test]
fn test_oauth2_access_token_creation() {
    let user_id = Uuid::new_v4();
    let token = OAuth2AccessToken {
        token: "access_token_xyz".to_owned(),
        client_id: "client_123".to_owned(),
        user_id: Some(user_id),
        scope: Some("read".to_owned()),
        expires_at: Utc::now() + Duration::hours(1),
        created_at: Utc::now(),
    };

    assert_eq!(token.token, "access_token_xyz");
    assert_eq!(token.user_id, Some(user_id));
}

#[test]
fn test_oauth2_access_token_client_credentials() {
    // Client credentials grant has no user_id
    let token = OAuth2AccessToken {
        token: "service_token".to_owned(),
        client_id: "service_client".to_owned(),
        user_id: None,
        scope: Some("admin".to_owned()),
        expires_at: Utc::now() + Duration::hours(2),
        created_at: Utc::now(),
    };

    assert!(token.user_id.is_none());
}

// =============================================================================
// OAuth2RefreshToken Tests
// =============================================================================

#[test]
fn test_oauth2_refresh_token_creation() {
    let user_id = Uuid::new_v4();
    let refresh_token = OAuth2RefreshToken {
        token: "refresh_token_abc".to_owned(),
        client_id: "client".to_owned(),
        user_id,
        tenant_id: "tenant".to_owned(),
        scope: Some("read write".to_owned()),
        expires_at: Utc::now() + Duration::days(30),
        created_at: Utc::now(),
        revoked: false,
    };

    assert_eq!(refresh_token.token, "refresh_token_abc");
    assert!(!refresh_token.revoked);
}

#[test]
fn test_oauth2_refresh_token_revoked() {
    let refresh_token = OAuth2RefreshToken {
        token: "revoked_token".to_owned(),
        client_id: "client".to_owned(),
        user_id: Uuid::new_v4(),
        tenant_id: "tenant".to_owned(),
        scope: None,
        expires_at: Utc::now() + Duration::days(30),
        created_at: Utc::now(),
        revoked: true,
    };

    assert!(refresh_token.revoked);
}

// =============================================================================
// OAuth2State Tests
// =============================================================================

#[test]
fn test_oauth2_state_creation() {
    let user_id = Uuid::new_v4();
    let state = OAuth2State {
        state: "random_state_value".to_owned(),
        client_id: "client_123".to_owned(),
        user_id: Some(user_id),
        tenant_id: None,
        redirect_uri: "https://app.com/callback".to_owned(),
        scope: Some("read".to_owned()),
        code_challenge: Some("challenge".to_owned()),
        code_challenge_method: Some("S256".to_owned()),
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
    };

    assert_eq!(state.state, "random_state_value");
    assert_eq!(state.user_id, Some(user_id));
    assert!(!state.used);
}

#[test]
fn test_oauth2_state_clone() {
    let state = OAuth2State {
        state: "state".to_owned(),
        client_id: "client".to_owned(),
        user_id: None,
        tenant_id: None,
        redirect_uri: "https://example.com".to_owned(),
        scope: None,
        code_challenge: None,
        code_challenge_method: None,
        created_at: Utc::now(),
        expires_at: Utc::now(),
        used: false,
    };

    let cloned = state.clone();
    assert_eq!(state.state, cloned.state);
}

// =============================================================================
// ValidationStatus Tests
// =============================================================================

#[test]
fn test_validation_status_serialization() {
    let valid_json = serde_json::to_string(&ValidationStatus::Valid).unwrap();
    assert_eq!(valid_json, "\"valid\"");

    let refreshed_json = serde_json::to_string(&ValidationStatus::Refreshed).unwrap();
    assert_eq!(refreshed_json, "\"refreshed\"");

    let invalid_json = serde_json::to_string(&ValidationStatus::Invalid).unwrap();
    assert_eq!(invalid_json, "\"invalid\"");
}

#[test]
fn test_validation_status_equality() {
    assert_eq!(ValidationStatus::Valid, ValidationStatus::Valid);
    assert_ne!(ValidationStatus::Valid, ValidationStatus::Invalid);
}

// =============================================================================
// ValidateRefreshRequest Tests
// =============================================================================

#[test]
fn test_validate_refresh_request_with_token() {
    let json = r#"{"refresh_token": "token_123"}"#;
    let request: ValidateRefreshRequest = serde_json::from_str(json).unwrap();

    assert_eq!(request.refresh_token, Some("token_123".to_owned()));
}

#[test]
fn test_validate_refresh_request_without_token() {
    let json = "{}";
    let request: ValidateRefreshRequest = serde_json::from_str(json).unwrap();

    assert!(request.refresh_token.is_none());
}

// =============================================================================
// ValidateRefreshResponse Tests
// =============================================================================

#[test]
fn test_validate_refresh_response_valid() {
    let response = ValidateRefreshResponse {
        status: ValidationStatus::Valid,
        expires_in: Some(3600),
        access_token: None,
        refresh_token: None,
        token_type: None,
        reason: None,
        requires_full_reauth: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"valid\""));
    assert!(json.contains("\"expires_in\":3600"));
    // Optional fields should not be present
    assert!(!json.contains("access_token"));
}

#[test]
fn test_validate_refresh_response_refreshed() {
    let response = ValidateRefreshResponse {
        status: ValidationStatus::Refreshed,
        expires_in: None,
        access_token: Some("new_access_token".to_owned()),
        refresh_token: Some("new_refresh_token".to_owned()),
        token_type: Some("Bearer".to_owned()),
        reason: None,
        requires_full_reauth: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"refreshed\""));
    assert!(json.contains("new_access_token"));
    assert!(json.contains("new_refresh_token"));
}

#[test]
fn test_validate_refresh_response_invalid() {
    let response = ValidateRefreshResponse {
        status: ValidationStatus::Invalid,
        expires_in: None,
        access_token: None,
        refresh_token: None,
        token_type: None,
        reason: Some("Token expired".to_owned()),
        requires_full_reauth: Some(true),
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(json.contains("\"status\":\"invalid\""));
    assert!(json.contains("\"reason\":\"Token expired\""));
    assert!(json.contains("\"requires_full_reauth\":true"));
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_oauth2_error_with_empty_description() {
    let error = OAuth2Error {
        error: "server_error".to_owned(),
        error_description: Some(String::new()),
        error_uri: None,
    };

    let json = serde_json::to_string(&error).unwrap();
    assert!(json.contains("\"error_description\":\"\""));
}

#[test]
fn test_token_response_long_expiry() {
    let response = TokenResponse {
        access_token: "token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 31_536_000, // 1 year in seconds
        scope: None,
        refresh_token: None,
    };

    assert_eq!(response.expires_in, 31_536_000);
}

#[test]
fn test_authorize_request_special_characters_in_state() {
    let json = r#"{
        "response_type": "code",
        "client_id": "client",
        "redirect_uri": "https://example.com/cb",
        "state": "abc123-._~"
    }"#;

    let request: AuthorizeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.state, Some("abc123-._~".to_owned()));
}

#[test]
fn test_multiple_redirect_uris() {
    let client = OAuth2Client {
        id: "id".to_owned(),
        client_id: "client".to_owned(),
        client_secret_hash: "hash".to_owned(),
        redirect_uris: vec![
            "https://app.com/callback".to_owned(),
            "https://app.com/oauth/callback".to_owned(),
            "http://localhost:3000/callback".to_owned(),
        ],
        grant_types: vec!["authorization_code".to_owned()],
        response_types: vec!["code".to_owned()],
        client_name: None,
        client_uri: None,
        scope: None,
        created_at: Utc::now(),
        expires_at: None,
    };

    assert_eq!(client.redirect_uris.len(), 3);
    assert!(client
        .redirect_uris
        .contains(&"http://localhost:3000/callback".to_owned()));
}
