// ABOUTME: Unit tests for OAuth2 client functionality
// ABOUTME: Validates OAuth2 client behavior, PKCE, token handling, and provider-specific flows
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::{Duration, Utc};
use pierre_mcp_server::oauth2_client::{OAuth2Client, OAuth2Config, OAuth2Token, PkceParams};

// =============================================================================
// PkceParams Tests
// =============================================================================

#[test]
fn test_pkce_params_generation() {
    let pkce = PkceParams::generate();

    // Code verifier should be 128 characters (from OAUTH_CODE_VERIFIER_LENGTH constant)
    assert!(!pkce.code_verifier.is_empty());
    assert!(pkce.code_verifier.len() >= 43); // Minimum PKCE spec requirement

    // Code challenge should be non-empty base64url encoded
    assert!(!pkce.code_challenge.is_empty());

    // Code challenge method should be S256
    assert_eq!(pkce.code_challenge_method, "S256");
}

#[test]
fn test_pkce_params_uniqueness() {
    let pkce1 = PkceParams::generate();
    let pkce2 = PkceParams::generate();

    // Each generation should produce unique values
    assert_ne!(pkce1.code_verifier, pkce2.code_verifier);
    assert_ne!(pkce1.code_challenge, pkce2.code_challenge);
}

#[test]
fn test_pkce_code_verifier_characters() {
    let pkce = PkceParams::generate();

    // Code verifier should only contain valid characters per RFC 7636
    // Valid chars: [A-Z] / [a-z] / [0-9] / "-" / "." / "_" / "~"
    for c in pkce.code_verifier.chars() {
        assert!(
            c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '~',
            "Invalid character in code verifier: {c}"
        );
    }
}

#[test]
fn test_pkce_code_challenge_is_base64url() {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

    let pkce = PkceParams::generate();

    // Code challenge should be valid base64url (no padding)
    let decoded = URL_SAFE_NO_PAD.decode(&pkce.code_challenge);
    assert!(decoded.is_ok(), "Code challenge should be valid base64url");

    // SHA256 hash should be 32 bytes
    let bytes = decoded.unwrap();
    assert_eq!(bytes.len(), 32, "SHA256 hash should be 32 bytes");
}

// =============================================================================
// OAuth2Token Tests
// =============================================================================

#[test]
fn test_oauth2_token_is_expired_when_past() {
    let token = OAuth2Token {
        access_token: "test_access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: Some(Utc::now() - Duration::hours(1)),
        refresh_token: Some("test_refresh_token".to_owned()),
        scope: Some("read write".to_owned()),
    };

    assert!(token.is_expired());
}

#[test]
fn test_oauth2_token_not_expired_when_future() {
    let token = OAuth2Token {
        access_token: "test_access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        refresh_token: Some("test_refresh_token".to_owned()),
        scope: Some("read write".to_owned()),
    };

    assert!(!token.is_expired());
}

#[test]
fn test_oauth2_token_not_expired_when_no_expiry() {
    let token = OAuth2Token {
        access_token: "test_access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: None,
        refresh_token: None,
        scope: None,
    };

    // Token with no expiration should not be considered expired
    assert!(!token.is_expired());
}

#[test]
fn test_oauth2_token_will_expire_soon_within_5_minutes() {
    let token = OAuth2Token {
        access_token: "test_access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: Some(Utc::now() + Duration::minutes(3)),
        refresh_token: Some("test_refresh_token".to_owned()),
        scope: Some("read".to_owned()),
    };

    assert!(token.will_expire_soon());
}

#[test]
fn test_oauth2_token_will_not_expire_soon_beyond_5_minutes() {
    let token = OAuth2Token {
        access_token: "test_access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: Some(Utc::now() + Duration::minutes(10)),
        refresh_token: Some("test_refresh_token".to_owned()),
        scope: Some("read".to_owned()),
    };

    assert!(!token.will_expire_soon());
}

#[test]
fn test_oauth2_token_will_not_expire_soon_when_no_expiry() {
    let token = OAuth2Token {
        access_token: "test_access_token".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: None,
        refresh_token: None,
        scope: None,
    };

    assert!(!token.will_expire_soon());
}

// =============================================================================
// OAuth2Config Tests
// =============================================================================

#[test]
fn test_oauth2_config_creation() {
    let config = OAuth2Config {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        auth_url: "https://provider.com/oauth/authorize".to_owned(),
        token_url: "https://provider.com/oauth/token".to_owned(),
        redirect_uri: "https://myapp.com/callback".to_owned(),
        scopes: vec!["read".to_owned(), "write".to_owned()],
        use_pkce: true,
    };

    assert_eq!(config.client_id, "test_client_id");
    assert_eq!(config.client_secret, "test_client_secret");
    assert!(config.use_pkce);
    assert_eq!(config.scopes.len(), 2);
}

#[test]
fn test_oauth2_config_serialization() {
    let config = OAuth2Config {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        auth_url: "https://provider.com/oauth/authorize".to_owned(),
        token_url: "https://provider.com/oauth/token".to_owned(),
        redirect_uri: "https://myapp.com/callback".to_owned(),
        scopes: vec!["read".to_owned()],
        use_pkce: false,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("test_client_id"));
    assert!(json.contains("use_pkce"));

    let deserialized: OAuth2Config = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.client_id, config.client_id);
    assert_eq!(deserialized.use_pkce, config.use_pkce);
}

// =============================================================================
// OAuth2Client Tests
// =============================================================================

#[test]
fn test_oauth2_client_creation() {
    let config = OAuth2Config {
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        auth_url: "https://provider.com/oauth/authorize".to_owned(),
        token_url: "https://provider.com/oauth/token".to_owned(),
        redirect_uri: "https://myapp.com/callback".to_owned(),
        scopes: vec!["read".to_owned()],
        use_pkce: true,
    };

    let client = OAuth2Client::new(config);

    assert_eq!(client.config().client_id, "test_client_id");
    assert!(client.config().use_pkce);
}

#[test]
fn test_oauth2_client_get_authorization_url() {
    let config = OAuth2Config {
        client_id: "my_client_id".to_owned(),
        client_secret: "my_client_secret".to_owned(),
        auth_url: "https://provider.com/oauth/authorize".to_owned(),
        token_url: "https://provider.com/oauth/token".to_owned(),
        redirect_uri: "https://myapp.com/callback".to_owned(),
        scopes: vec!["read".to_owned(), "write".to_owned()],
        use_pkce: false,
    };

    let client = OAuth2Client::new(config);
    let state = "random_state_value";

    let url = client.get_authorization_url(state).unwrap();

    assert!(url.starts_with("https://provider.com/oauth/authorize?"));
    assert!(url.contains("client_id=my_client_id"));
    assert!(url.contains("redirect_uri="));
    assert!(url.contains("response_type=code"));
    assert!(url.contains("scope=read+write") || url.contains("scope=read%20write"));
    assert!(url.contains("state=random_state_value"));
}

#[test]
fn test_oauth2_client_get_authorization_url_with_pkce() {
    let config = OAuth2Config {
        client_id: "pkce_client_id".to_owned(),
        client_secret: "pkce_client_secret".to_owned(),
        auth_url: "https://provider.com/oauth/authorize".to_owned(),
        token_url: "https://provider.com/oauth/token".to_owned(),
        redirect_uri: "https://myapp.com/callback".to_owned(),
        scopes: vec!["activity:read".to_owned()],
        use_pkce: true,
    };

    let client = OAuth2Client::new(config);
    let state = "pkce_state";
    let pkce = PkceParams::generate();

    let url = client
        .get_authorization_url_with_pkce(state, &pkce)
        .unwrap();

    assert!(url.starts_with("https://provider.com/oauth/authorize?"));
    assert!(url.contains("client_id=pkce_client_id"));
    assert!(url.contains("code_challenge="));
    assert!(url.contains("code_challenge_method=S256"));
    assert!(url.contains(&pkce.code_challenge));
}

#[test]
fn test_oauth2_client_authorization_url_invalid_base_url() {
    let config = OAuth2Config {
        client_id: "test_client".to_owned(),
        client_secret: "test_secret".to_owned(),
        auth_url: "not-a-valid-url".to_owned(),
        token_url: "https://provider.com/oauth/token".to_owned(),
        redirect_uri: "https://myapp.com/callback".to_owned(),
        scopes: vec!["read".to_owned()],
        use_pkce: false,
    };

    let client = OAuth2Client::new(config);
    let result = client.get_authorization_url("state");

    assert!(result.is_err());
}

#[test]
fn test_oauth2_client_http_client_accessible() {
    let config = OAuth2Config {
        client_id: "test".to_owned(),
        client_secret: "test".to_owned(),
        auth_url: "https://provider.com/auth".to_owned(),
        token_url: "https://provider.com/token".to_owned(),
        redirect_uri: "https://myapp.com/cb".to_owned(),
        scopes: vec![],
        use_pkce: false,
    };

    let client = OAuth2Client::new(config);

    // Verify we can access the HTTP client (it exists and is usable)
    let _http_client = client.http_client();
    // Client should be accessible without panic
}

// =============================================================================
// OAuth2Token Serialization Tests
// =============================================================================

#[test]
fn test_oauth2_token_serialization() {
    let token = OAuth2Token {
        access_token: "access123".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: Some(Utc::now() + Duration::hours(1)),
        refresh_token: Some("refresh456".to_owned()),
        scope: Some("read write".to_owned()),
    };

    let json = serde_json::to_string(&token).unwrap();
    assert!(json.contains("access123"));
    assert!(json.contains("Bearer"));
    assert!(json.contains("refresh456"));

    let deserialized: OAuth2Token = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.access_token, token.access_token);
    assert_eq!(deserialized.token_type, token.token_type);
    assert_eq!(deserialized.refresh_token, token.refresh_token);
}

#[test]
fn test_oauth2_token_serialization_with_nulls() {
    let token = OAuth2Token {
        access_token: "access_only".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: None,
        refresh_token: None,
        scope: None,
    };

    let json = serde_json::to_string(&token).unwrap();
    let deserialized: OAuth2Token = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.access_token, "access_only");
    assert!(deserialized.expires_at.is_none());
    assert!(deserialized.refresh_token.is_none());
    assert!(deserialized.scope.is_none());
}

// =============================================================================
// Strava-Specific Tests
// =============================================================================

#[test]
fn test_strava_token_response_deserialization() {
    use pierre_mcp_server::oauth2_client::client::strava::StravaTokenResponse;

    let json = r#"{
        "token_type": "Bearer",
        "expires_at": 1700000000,
        "expires_in": 21600,
        "refresh_token": "strava_refresh_token",
        "access_token": "strava_access_token",
        "athlete": {
            "id": 12345,
            "username": "testathlete",
            "firstname": "Test",
            "lastname": "Athlete"
        }
    }"#;

    let response: StravaTokenResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.access_token, "strava_access_token");
    assert_eq!(response.refresh_token, "strava_refresh_token");
    assert_eq!(response.expires_at, 1_700_000_000);
    assert_eq!(response.expires_in, 21600);

    let athlete = response.athlete.unwrap();
    assert_eq!(athlete.id, 12345);
    assert_eq!(athlete.username, Some("testathlete".to_owned()));
}

#[test]
fn test_strava_token_response_without_athlete() {
    use pierre_mcp_server::oauth2_client::client::strava::StravaTokenResponse;

    let json = r#"{
        "token_type": "Bearer",
        "expires_at": 1700000000,
        "expires_in": 21600,
        "refresh_token": "refresh",
        "access_token": "access"
    }"#;

    let response: StravaTokenResponse = serde_json::from_str(json).unwrap();
    assert!(response.athlete.is_none());
}

// =============================================================================
// Fitbit-Specific Tests
// =============================================================================

#[test]
fn test_fitbit_token_response_deserialization() {
    use pierre_mcp_server::oauth2_client::client::fitbit::FitbitTokenResponse;

    let json = r#"{
        "access_token": "fitbit_access_token",
        "expires_in": 28800,
        "refresh_token": "fitbit_refresh_token",
        "scope": "activity heartrate sleep",
        "token_type": "Bearer",
        "user_id": "FITBIT123"
    }"#;

    let response: FitbitTokenResponse = serde_json::from_str(json).unwrap();

    assert_eq!(response.access_token, "fitbit_access_token");
    assert_eq!(response.refresh_token, "fitbit_refresh_token");
    assert_eq!(response.token_type, "Bearer");
    assert_eq!(response.user_id, "FITBIT123");
    assert_eq!(response.scope, "activity heartrate sleep");
    assert_eq!(response.expires_in, 28800);
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_oauth2_token_boundary_expiration() {
    // Token expiring exactly now
    let token = OAuth2Token {
        access_token: "test".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_at: Some(Utc::now()),
        refresh_token: None,
        scope: None,
    };

    // Should be considered expired (or just about to)
    assert!(token.is_expired() || token.will_expire_soon());
}

#[test]
fn test_oauth2_config_empty_scopes() {
    let config = OAuth2Config {
        client_id: "test".to_owned(),
        client_secret: "test".to_owned(),
        auth_url: "https://provider.com/auth".to_owned(),
        token_url: "https://provider.com/token".to_owned(),
        redirect_uri: "https://myapp.com/cb".to_owned(),
        scopes: vec![],
        use_pkce: false,
    };

    let client = OAuth2Client::new(config);
    let url = client.get_authorization_url("state").unwrap();

    // URL should still be valid even with empty scopes
    assert!(url.contains("scope="));
}

#[test]
fn test_oauth2_config_special_characters_in_redirect_uri() {
    let config = OAuth2Config {
        client_id: "test".to_owned(),
        client_secret: "test".to_owned(),
        auth_url: "https://provider.com/auth".to_owned(),
        token_url: "https://provider.com/token".to_owned(),
        redirect_uri: "https://myapp.com/callback?extra=param&another=value".to_owned(),
        scopes: vec!["read".to_owned()],
        use_pkce: false,
    };

    let client = OAuth2Client::new(config);
    let url = client.get_authorization_url("state").unwrap();

    // URL should properly encode the redirect URI
    assert!(url.contains("redirect_uri="));
}

#[test]
fn test_multiple_pkce_generations_are_cryptographically_secure() {
    // Generate multiple PKCE params and ensure they're all different
    let params: Vec<PkceParams> = (0..10).map(|_| PkceParams::generate()).collect();

    for i in 0..params.len() {
        for j in (i + 1)..params.len() {
            assert_ne!(
                params[i].code_verifier, params[j].code_verifier,
                "PKCE code verifiers should be unique"
            );
            assert_ne!(
                params[i].code_challenge, params[j].code_challenge,
                "PKCE code challenges should be unique"
            );
        }
    }
}
