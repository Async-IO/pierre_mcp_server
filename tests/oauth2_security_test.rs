// ABOUTME: Comprehensive OAuth 2.0 security tests for redirect URI validation and Argon2id
// ABOUTME: Validates security hardening improvements for client registration
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use pierre_mcp_server::{
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    oauth2::{client_registration::ClientRegistrationManager, models::ClientRegistrationRequest},
};
use std::sync::Arc;

/// Test redirect URI validation - HTTPS enforcement
#[tokio::test]
async fn test_redirect_uri_https_enforcement() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    // Test 1: HTTPS URI should succeed
    let https_registration = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_string()],
        client_name: Some("HTTPS Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(https_registration)
        .await;
    assert!(result.is_ok());

    // Test 2: HTTP non-localhost URI should fail
    let http_non_localhost_registration = ClientRegistrationRequest {
        redirect_uris: vec!["http://example.com/callback".to_string()],
        client_name: Some("HTTP Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(http_non_localhost_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("Invalid redirect_uri"));

    // Test 3: HTTP localhost should succeed
    let localhost_registration = ClientRegistrationRequest {
        redirect_uris: vec!["http://localhost:8080/callback".to_string()],
        client_name: Some("Localhost Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(localhost_registration)
        .await;
    assert!(result.is_ok());

    // Test 4: HTTP 127.0.0.1 should succeed
    let loopback_registration = ClientRegistrationRequest {
        redirect_uris: vec!["http://127.0.0.1:3000/callback".to_string()],
        client_name: Some("Loopback Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(loopback_registration)
        .await;
    assert!(result.is_ok());
}

/// Test redirect URI validation - fragment rejection
#[tokio::test]
async fn test_redirect_uri_fragment_rejection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    // URI with fragment should fail (security risk per RFC 6749)
    let fragment_registration = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback#fragment".to_string()],
        client_name: Some("Fragment Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(fragment_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("Invalid redirect_uri"));
}

/// Test redirect URI validation - wildcard rejection
#[tokio::test]
async fn test_redirect_uri_wildcard_rejection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    // Wildcard URI should fail (subdomain bypass attack prevention)
    let wildcard_registration = ClientRegistrationRequest {
        redirect_uris: vec!["https://*.example.com/callback".to_string()],
        client_name: Some("Wildcard Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(wildcard_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("Invalid redirect_uri"));
}

/// Test redirect URI validation - out-of-band URN
#[tokio::test]
async fn test_redirect_uri_oob_urn() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    // Out-of-band URN should succeed (for native apps per RFC 8252)
    let oob_registration = ClientRegistrationRequest {
        redirect_uris: vec!["urn:ietf:wg:oauth:2.0:oob".to_string()],
        client_name: Some("OOB Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager.register_client(oob_registration).await;
    assert!(result.is_ok());
}

/// Test Argon2id hashing and verification
#[tokio::test]
async fn test_argon2id_client_secret_hashing() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    let registration_request = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_string()],
        client_name: Some("Test Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let registration_response = registration_manager
        .register_client(registration_request)
        .await
        .unwrap();

    // Retrieve client from database to check hash format
    let client = database
        .get_oauth2_client(&registration_response.client_id)
        .await
        .unwrap()
        .unwrap();

    // Verify hash is in Argon2 PHC format
    assert!(client.client_secret_hash.starts_with("$argon2"));

    // Verify hash can be parsed and contains proper algorithm identifier
    let parsed_hash = PasswordHash::new(&client.client_secret_hash).unwrap();
    assert_eq!(parsed_hash.algorithm.as_str(), "argon2id");

    // Verify the secret can be validated
    let argon2 = Argon2::default();
    assert!(argon2
        .verify_password(registration_response.client_secret.as_bytes(), &parsed_hash)
        .is_ok());

    // Verify wrong secret fails validation
    assert!(argon2
        .verify_password(b"wrong_secret", &parsed_hash)
        .is_err());
}

/// Test client validation with correct and incorrect secrets
#[tokio::test]
async fn test_client_secret_validation() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    let registration_request = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_string()],
        client_name: Some("Test Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let registration_response = registration_manager
        .register_client(registration_request)
        .await
        .unwrap();

    // Test 1: Correct secret should succeed
    let result = registration_manager
        .validate_client(
            &registration_response.client_id,
            &registration_response.client_secret,
        )
        .await;
    assert!(result.is_ok());

    // Test 2: Wrong secret should fail
    let result = registration_manager
        .validate_client(&registration_response.client_id, "wrong_secret")
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error, "invalid_client");

    // Test 3: Non-existent client should fail
    let result = registration_manager
        .validate_client("non_existent_client", "any_secret")
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().error, "invalid_client");
}

/// Test empty redirect URI list rejection
#[tokio::test]
async fn test_empty_redirect_uri_rejection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    let empty_uri_registration = ClientRegistrationRequest {
        redirect_uris: vec![],
        client_name: Some("Empty URI Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(empty_uri_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("At least one redirect_uri is required"));
}

/// Test malformed URI rejection
#[tokio::test]
async fn test_malformed_uri_rejection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    let malformed_uri_registration = ClientRegistrationRequest {
        redirect_uris: vec!["not-a-valid-uri".to_string()],
        client_name: Some("Malformed URI Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(malformed_uri_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("Invalid redirect_uri"));
}

/// Test unsupported grant type rejection
#[tokio::test]
async fn test_unsupported_grant_type_rejection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    let unsupported_grant_registration = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_string()],
        client_name: Some("Unsupported Grant Client".to_string()),
        client_uri: None,
        grant_types: Some(vec!["implicit".to_string()]),
        response_types: None,
        scope: None,
    };

    let result = registration_manager
        .register_client(unsupported_grant_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("Unsupported grant_type"));
}

/// Test unsupported response type rejection
#[tokio::test]
async fn test_unsupported_response_type_rejection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(
        Database::new("sqlite::memory:", encryption_key)
            .await
            .unwrap(),
    );
    database.migrate().await.unwrap();

    let registration_manager = ClientRegistrationManager::new(database.clone());

    let unsupported_response_registration = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_string()],
        client_name: Some("Unsupported Response Client".to_string()),
        client_uri: None,
        grant_types: None,
        response_types: Some(vec!["token".to_string()]),
        scope: None,
    };

    let result = registration_manager
        .register_client(unsupported_response_registration)
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .error_description
        .unwrap()
        .contains("Unsupported response_type"));
}
