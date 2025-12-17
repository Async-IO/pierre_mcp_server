// ABOUTME: Tests for OAuth 2.0 typestate pattern implementation
// ABOUTME: Validates compile-time state transition safety for OAuth flows
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::oauth2_server::{
    AuthorizeResponse, Authorized, Initial, OAuthFlow, PkceConfig, PkceMethod, TokenResponse,
};

#[test]
fn test_initial_to_authorized_transition() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");

    assert_eq!(flow.client_id(), "client_123");
    assert_eq!(flow.redirect_uri(), "https://app.example.com/callback");

    let response = AuthorizeResponse {
        code: "auth_code_abc".to_owned(),
        state: Some("csrf_state".to_owned()),
    };

    let authorized = flow.authorize(response);

    assert_eq!(authorized.code(), "auth_code_abc");
    assert_eq!(authorized.state_param(), Some("csrf_state"));
    assert!(!authorized.is_code_expired());
}

#[test]
fn test_authorized_to_authenticated_transition() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");

    let authorized = flow.with_authorization_code("test_code", None);

    let token_response = TokenResponse {
        access_token: "access_token_xyz".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 3600,
        scope: Some("read write".to_owned()),
        refresh_token: Some("refresh_token_abc".to_owned()),
    };

    let authenticated = authorized
        .exchange(token_response)
        .expect("exchange should succeed");

    assert_eq!(authenticated.access_token(), "access_token_xyz");
    assert_eq!(authenticated.token_type(), "Bearer");
    assert_eq!(authenticated.scope(), Some("read write"));
    assert_eq!(authenticated.refresh_token(), Some("refresh_token_abc"));
    assert!(!authenticated.is_token_expired());
}

#[test]
fn test_authenticated_to_refreshable_transition() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");

    let authorized = flow.with_authorization_code("test_code", None);

    // Use a negative expiration to simulate an already-expired token
    let token_response = TokenResponse {
        access_token: "access_token_xyz".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: -1,
        scope: Some("read".to_owned()),
        refresh_token: Some("refresh_token_abc".to_owned()),
    };

    let authenticated = authorized
        .exchange(token_response)
        .expect("exchange should succeed");

    // Token should be expired
    assert!(authenticated.is_token_expired());

    // Transition to Refreshable
    let refreshable = authenticated
        .needs_refresh()
        .expect("should have refresh token");

    assert_eq!(refreshable.refresh_token(), "refresh_token_abc");
    assert_eq!(refreshable.scope(), Some("read"));
}

#[test]
fn test_refreshable_to_authenticated_transition() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");

    let authorized = flow.with_authorization_code("test_code", None);

    let token_response = TokenResponse {
        access_token: "old_access".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: -1,
        scope: Some("read".to_owned()),
        refresh_token: Some("old_refresh".to_owned()),
    };

    let authenticated = authorized
        .exchange(token_response)
        .expect("exchange should succeed");
    let refreshable = authenticated
        .needs_refresh()
        .expect("should have refresh token");

    // Refresh with new tokens
    let new_token_response = TokenResponse {
        access_token: "new_access".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 3600,
        scope: None, // Falls back to original scope
        refresh_token: Some("new_refresh".to_owned()),
    };

    let re_authenticated = refreshable.refresh(new_token_response);

    assert_eq!(re_authenticated.access_token(), "new_access");
    assert_eq!(re_authenticated.refresh_token(), Some("new_refresh"));
    assert_eq!(re_authenticated.scope(), Some("read")); // Preserved from original
    assert!(!re_authenticated.is_token_expired());
}

#[test]
fn test_pkce_flow() {
    let pkce = PkceConfig::new(
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", // Example verifier
        "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM", // Example S256 challenge
    );

    let flow =
        OAuthFlow::<Initial>::with_pkce("client_123", "https://app.example.com/callback", pkce);

    let pkce_config = flow.pkce_config().expect("should have PKCE config");
    assert_eq!(pkce_config.code_challenge_method(), PkceMethod::S256);
    assert_eq!(
        pkce_config.code_verifier(),
        "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"
    );

    let authorized = flow.with_authorization_code("test_code", None);

    // Code verifier should be available in Authorized state
    assert_eq!(
        authorized.code_verifier(),
        Some("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk")
    );
}

#[test]
fn test_no_refresh_token_fails() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");

    let authorized = flow.with_authorization_code("test_code", None);

    let token_response = TokenResponse {
        access_token: "access".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: -1,
        scope: None,
        refresh_token: None, // No refresh token
    };

    let authenticated = authorized
        .exchange(token_response)
        .expect("exchange should succeed");

    // Attempting to refresh should fail
    let result = authenticated.needs_refresh();
    assert!(result.is_err());

    let Err(err) = result else {
        panic!("Expected error")
    };
    assert_eq!(err.error, "invalid_grant");
}

#[test]
fn test_tenant_support() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback")
        .with_tenant("tenant_456");

    assert_eq!(flow.tenant_id(), Some("tenant_456"));

    let authorized = flow.with_authorization_code("test_code", None);
    assert_eq!(authorized.tenant_id(), Some("tenant_456"));
}

#[test]
fn test_force_refresh() {
    let flow = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");

    let authorized = flow.with_authorization_code("test_code", None);

    let token_response = TokenResponse {
        access_token: "access".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 3600, // Not expired
        scope: None,
        refresh_token: Some("refresh".to_owned()),
    };

    let authenticated = authorized
        .exchange(token_response)
        .expect("exchange should succeed");

    // Token is not expired, but we can force refresh
    assert!(!authenticated.is_token_expired());

    let refreshable = authenticated
        .force_refresh()
        .expect("force_refresh should succeed");
    assert_eq!(refreshable.refresh_token(), "refresh");
}

#[test]
fn test_oauth_flow_debug_trait() {
    // Verify that all state types implement Debug (required for debugging)
    let initial = OAuthFlow::<Initial>::new("client_123", "https://app.example.com/callback");
    let debug_output = format!("{initial:?}");
    assert!(debug_output.contains("client_123"));

    let authorized: OAuthFlow<Authorized> = initial.with_authorization_code("test_code", None);
    let debug_output = format!("{authorized:?}");
    assert!(debug_output.contains("test_code"));
}

#[test]
fn test_pkce_method_display() {
    assert_eq!(format!("{}", PkceMethod::S256), "S256");
    assert_eq!(PkceMethod::S256.as_str(), "S256");
}

#[test]
fn test_client_id_and_redirect_uri_preserved_through_transitions() {
    let flow = OAuthFlow::<Initial>::new("my_client", "https://my-app.com/callback")
        .with_tenant("tenant_xyz");

    // Check Initial state
    assert_eq!(flow.client_id(), "my_client");
    assert_eq!(flow.redirect_uri(), "https://my-app.com/callback");
    assert_eq!(flow.tenant_id(), Some("tenant_xyz"));

    // Transition to Authorized
    let authorized = flow.with_authorization_code("code123", Some("state456".to_owned()));
    assert_eq!(authorized.client_id(), "my_client");
    assert_eq!(authorized.redirect_uri(), "https://my-app.com/callback");
    assert_eq!(authorized.tenant_id(), Some("tenant_xyz"));

    // Transition to Authenticated
    let token_response = TokenResponse {
        access_token: "access".to_owned(),
        token_type: "Bearer".to_owned(),
        expires_in: 3600,
        scope: Some("read".to_owned()),
        refresh_token: Some("refresh".to_owned()),
    };

    let authenticated = authorized
        .exchange(token_response)
        .expect("exchange should succeed");
    assert_eq!(authenticated.client_id(), "my_client");
    assert_eq!(authenticated.redirect_uri(), "https://my-app.com/callback");
    assert_eq!(authenticated.tenant_id(), Some("tenant_xyz"));
}
