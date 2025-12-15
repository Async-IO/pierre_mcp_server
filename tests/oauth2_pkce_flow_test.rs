// ABOUTME: Comprehensive OAuth 2.0 PKCE and authorization flow security tests
// ABOUTME: Validates PKCE enforcement, state replay protection, and auth code security
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use base64::{engine::general_purpose, Engine as _};
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
    oauth2_server::{
        client_registration::ClientRegistrationManager,
        endpoints::OAuth2AuthorizationServer,
        models::{AuthorizeRequest, ClientRegistrationRequest, TokenRequest},
    },
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Helper to create test database and auth manager
async fn setup_test_env() -> (
    Arc<Database>,
    Arc<AuthManager>,
    OAuth2AuthorizationServer,
    String,
    String,
) {
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            encryption_key,
            &PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let auth_manager = Arc::new(AuthManager::new(24));

    // Create JWKS manager for RS256 token signing
    let jwks_manager = common::get_shared_test_jwks();

    let oauth_server =
        OAuth2AuthorizationServer::new(database.clone(), auth_manager.clone(), jwks_manager);

    // Register a test client
    let registration_manager = ClientRegistrationManager::new(database.clone());
    let registration_request = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_owned()],
        client_name: Some("Test Client".to_owned()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let registration_response = registration_manager
        .register_client(registration_request)
        .await
        .unwrap();

    (
        database,
        auth_manager,
        oauth_server,
        registration_response.client_id,
        registration_response.client_secret,
    )
}

/// Generate PKCE `code_verifier` (43-128 characters, base64url-encoded random bytes)
fn generate_code_verifier() -> String {
    use ring::rand::{SecureRandom, SystemRandom};
    let rng = SystemRandom::new();
    let mut random_bytes = [0u8; 32];
    rng.fill(&mut random_bytes).unwrap();
    general_purpose::URL_SAFE_NO_PAD.encode(random_bytes)
}

/// Generate PKCE `code_challenge` from `code_verifier` using S256 method
fn generate_code_challenge(code_verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let hash = hasher.finalize();
    general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

/// Test PKCE enforcement - authorization without `code_challenge` should fail
#[tokio::test]
async fn test_pkce_enforcement_no_code_challenge() {
    let (database, _auth_manager, oauth_server, client_id, _client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Attempt authorization WITHOUT code_challenge (PKCE required)
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id,
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: None, // No PKCE
        code_challenge_method: None,
    };

    let result = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_request");
    assert!(error
        .error_description
        .unwrap()
        .contains("code_challenge is required"));
}

/// Test PKCE enforcement - valid authorization with S256 `code_challenge`
#[tokio::test]
async fn test_pkce_valid_s256_flow() {
    let (database, _auth_manager, oauth_server, client_id, client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Authorization with PKCE
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();
    assert!(!auth_response.code.is_empty());

    // Token exchange with valid code_verifier
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example.com/callback".to_owned()),
        client_id,
        client_secret,
        scope: None,
        refresh_token: None,
        code_verifier: Some(code_verifier),
    };

    let token_response = oauth_server.token(token_request).await;
    assert!(token_response.is_ok());
}

/// Test PKCE verification - wrong `code_verifier` should fail
#[tokio::test]
async fn test_pkce_invalid_code_verifier() {
    let (database, _auth_manager, oauth_server, client_id, client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Authorization with PKCE
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();

    // Token exchange with WRONG code_verifier
    let wrong_verifier = generate_code_verifier(); // Different verifier
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example.com/callback".to_owned()),
        client_id,
        client_secret,
        scope: None,
        refresh_token: None,
        code_verifier: Some(wrong_verifier),
    };

    let result = oauth_server.token(token_request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_grant");
    assert!(error.error_description.unwrap().contains("code_verifier"));
}

/// Test PKCE - missing `code_verifier` when `code_challenge` was provided
#[tokio::test]
async fn test_pkce_missing_code_verifier() {
    let (database, _auth_manager, oauth_server, client_id, client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Authorization with PKCE
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();

    // Token exchange WITHOUT code_verifier (should fail)
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example.com/callback".to_owned()),
        client_id,
        client_secret,
        scope: None,
        refresh_token: None,
        code_verifier: None, // Missing verifier
    };

    let result = oauth_server.token(token_request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_grant");
    assert!(error.error_description.unwrap().contains("code_verifier"));
}

/// Test authorization code replay attack - code can only be used once
#[tokio::test]
async fn test_auth_code_replay_prevention() {
    let (database, _auth_manager, oauth_server, client_id, client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Authorization with PKCE
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();

    // First token exchange - should succeed
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code.clone()),
        redirect_uri: Some("https://example.com/callback".to_owned()),
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        scope: None,
        refresh_token: None,
        code_verifier: Some(code_verifier.clone()),
    };

    let first_result = oauth_server.token(token_request).await;
    assert!(first_result.is_ok());

    // Second token exchange with SAME code - should fail (replay attack)
    let replay_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example.com/callback".to_owned()),
        client_id,
        client_secret,
        scope: None,
        refresh_token: None,
        code_verifier: Some(code_verifier),
    };

    let result = oauth_server.token(replay_request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_grant");
    assert!(error
        .error_description
        .unwrap()
        .contains("Invalid or expired"));
}

/// Test authorization code bound to specific client - cross-client attack prevention
#[tokio::test]
async fn test_auth_code_client_binding() {
    let (database, _auth_manager, oauth_server, client_id, _client_secret) = setup_test_env().await;

    // Register a SECOND client
    let registration_manager = ClientRegistrationManager::new(database.clone());
    let second_client_request = ClientRegistrationRequest {
        redirect_uris: vec!["https://example2.com/callback".to_owned()],
        client_name: Some("Second Client".to_owned()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };
    let second_client_response = registration_manager
        .register_client(second_client_request)
        .await
        .unwrap();

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Authorization for FIRST client
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();

    // Attempt token exchange with SECOND client (should fail - cross-client attack)
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example2.com/callback".to_owned()),
        client_id: second_client_response.client_id,
        client_secret: second_client_response.client_secret,
        scope: None,
        refresh_token: None,
        code_verifier: Some(code_verifier),
    };

    let result = oauth_server.token(token_request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_grant");
    assert!(error
        .error_description
        .unwrap()
        .contains("Invalid or expired"));
}

/// Test `redirect_uri` exact match requirement
#[tokio::test]
async fn test_redirect_uri_exact_match() {
    let (database, _auth_manager, oauth_server, client_id, client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Authorization with exact redirect_uri
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();

    // Token exchange with DIFFERENT redirect_uri (should fail - must match exactly)
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example.com/callback2".to_owned()), // Different path
        client_id,
        client_secret,
        scope: None,
        refresh_token: None,
        code_verifier: Some(code_verifier),
    };

    let result = oauth_server.token(token_request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_grant");
    assert!(error
        .error_description
        .unwrap()
        .contains("Invalid or expired"));
}

/// Test refresh token rotation - old token cannot be reused
#[tokio::test]
async fn test_refresh_token_rotation() {
    let (database, _auth_manager, oauth_server, client_id, client_secret) = setup_test_env().await;

    // Create test user
    let user = User::new(
        "test@example.com".to_owned(),
        "hash".to_owned(),
        Some("Test User".to_owned()),
    );
    database.create_user(&user).await.unwrap();

    // Generate PKCE parameters
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Get authorization code
    let auth_request = AuthorizeRequest {
        response_type: "code".to_owned(),
        client_id: client_id.clone(),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("fitness:read".to_owned()),
        state: Some("test_state".to_owned()),
        code_challenge: Some(code_challenge),
        code_challenge_method: Some("S256".to_owned()),
    };

    let auth_response = oauth_server
        .authorize(auth_request, Some(user.id), None)
        .await
        .unwrap();

    // Exchange for initial token
    let token_request = TokenRequest {
        grant_type: "authorization_code".to_owned(),
        code: Some(auth_response.code),
        redirect_uri: Some("https://example.com/callback".to_owned()),
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        scope: None,
        refresh_token: None,
        code_verifier: Some(code_verifier),
    };

    let token_response = oauth_server.token(token_request).await.unwrap();
    let old_refresh_token = token_response.refresh_token.unwrap();

    // Use refresh token to get new tokens (first refresh - should succeed)
    let refresh_request = TokenRequest {
        grant_type: "refresh_token".to_owned(),
        code: None,
        redirect_uri: None,
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        scope: None,
        refresh_token: Some(old_refresh_token.clone()),
        code_verifier: None,
    };

    let refresh_response = oauth_server.token(refresh_request).await;
    assert!(refresh_response.is_ok());

    // Attempt to reuse OLD refresh token (should fail - token rotation)
    let replay_refresh_request = TokenRequest {
        grant_type: "refresh_token".to_owned(),
        code: None,
        redirect_uri: None,
        client_id,
        client_secret,
        scope: None,
        refresh_token: Some(old_refresh_token),
        code_verifier: None,
    };

    let result = oauth_server.token(replay_refresh_request).await;
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert_eq!(error.error, "invalid_grant");
}
