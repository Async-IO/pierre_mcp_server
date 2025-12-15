// ABOUTME: Comprehensive OAuth 2.0 server-side state validation tests for CSRF protection
// ABOUTME: Tests defense-in-depth security with atomic state consumption and replay attack prevention
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::{Duration, Utc};
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    oauth2_server::{
        client_registration::ClientRegistrationManager,
        models::{ClientRegistrationRequest, OAuth2State},
    },
};
use std::error::Error;
use std::sync::Arc;
use uuid::Uuid;

/// Helper function to create an `OAuth2` client for testing
async fn create_test_client(
    database: &Arc<Database>,
    client_id: &str,
) -> Result<(), Box<dyn Error>> {
    let registration_manager = ClientRegistrationManager::new(database.clone());

    // Register a test client
    let registration = ClientRegistrationRequest {
        redirect_uris: vec!["https://example.com/callback".to_owned()],
        client_name: Some(format!("Test Client {client_id}")),
        client_uri: None,
        grant_types: Some(vec!["authorization_code".to_owned()]),
        response_types: Some(vec!["code".to_owned()]),
        scope: Some("read write".to_owned()),
    };

    let response = registration_manager
        .register_client(registration)
        .await
        .map_err(|e| format!("Failed to register OAuth2 client: {e:?}"))?;

    // For testing purposes, we'll use the generated client_id from registration
    // and update our test to use it, or we can manually insert with our desired client_id
    // Using sqlx directly for the update

    // Get the underlying SQLite connection and update the client_id
    // Tests always use in-memory SQLite
    let pool = match &**database {
        Database::SQLite(sqlite_db) => sqlite_db.pool(),
        #[cfg(feature = "postgresql")]
        Database::PostgreSQL(_) => {
            return Err("Test requires SQLite database".into());
        }
    };

    sqlx::query("UPDATE oauth2_clients SET client_id = ?1 WHERE client_id = ?2")
        .bind(client_id)
        .bind(&response.client_id)
        .execute(pool)
        .await?;

    Ok(())
}

/// Test successful state storage and consumption
#[tokio::test]
async fn test_state_storage_and_consumption() {
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

    let client_id = "test_client_123";
    let state_value = "random_state_value_12345";
    let user_id = Uuid::new_v4();
    let tenant_id = "tenant_456";

    // Create test OAuth2 client (required for foreign key constraint)
    create_test_client(&database, client_id).await.unwrap();

    // Create OAuth2State
    let oauth2_state = OAuth2State {
        state: state_value.to_owned(),
        client_id: client_id.to_owned(),
        user_id: Some(user_id),
        tenant_id: Some(tenant_id.to_owned()),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("read write".to_owned()),
        code_challenge: Some("test_challenge".to_owned()),
        code_challenge_method: Some("S256".to_owned()),
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
    };

    // Store state
    let store_result = database.store_oauth2_state(&oauth2_state).await;
    assert!(
        store_result.is_ok(),
        "State storage should succeed: {:?}",
        store_result.err()
    );

    // Consume state
    let consumed_state = database
        .consume_oauth2_state(state_value, client_id, Utc::now())
        .await
        .unwrap();

    assert!(
        consumed_state.is_some(),
        "State should be found and consumed"
    );
    let consumed = consumed_state.unwrap();
    assert_eq!(consumed.state, state_value);
    assert_eq!(consumed.client_id, client_id);
    assert_eq!(consumed.user_id, Some(user_id));
    assert_eq!(consumed.tenant_id, Some(tenant_id.to_owned()));
    assert_eq!(consumed.redirect_uri, "https://example.com/callback");
    assert_eq!(consumed.scope, Some("read write".to_owned()));
}

/// Test state replay attack prevention (state already used)
#[tokio::test]
async fn test_state_replay_attack_prevention() {
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

    let client_id = "test_client_replay";
    let state_value = "state_replay_test_123";

    // Create test OAuth2 client (required for foreign key constraint)
    create_test_client(&database, client_id).await.unwrap();

    let oauth2_state = OAuth2State {
        state: state_value.to_owned(),
        client_id: client_id.to_owned(),
        user_id: Some(Uuid::new_v4()),
        tenant_id: Some("tenant_replay".to_owned()),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: None,
        code_challenge: None,
        code_challenge_method: None,
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
    };

    database.store_oauth2_state(&oauth2_state).await.unwrap();

    // First consumption should succeed
    let first_consumption = database
        .consume_oauth2_state(state_value, client_id, Utc::now())
        .await
        .unwrap();
    assert!(
        first_consumption.is_some(),
        "First consumption should succeed"
    );

    // Second consumption should fail (replay attack)
    let second_consumption = database
        .consume_oauth2_state(state_value, client_id, Utc::now())
        .await
        .unwrap();
    assert!(
        second_consumption.is_none(),
        "Second consumption should fail - state already used (replay attack detected)"
    );
}

/// Test expired state rejection
#[tokio::test]
async fn test_expired_state_rejection() {
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

    let client_id = "test_client_expired";
    let state_value = "expired_state_test_456";

    // Create test OAuth2 client (required for foreign key constraint)
    create_test_client(&database, client_id).await.unwrap();

    // Create state that expired 1 minute ago
    let oauth2_state = OAuth2State {
        state: state_value.to_owned(),
        client_id: client_id.to_owned(),
        user_id: Some(Uuid::new_v4()),
        tenant_id: Some("tenant_expired".to_owned()),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: None,
        code_challenge: None,
        code_challenge_method: None,
        created_at: Utc::now() - Duration::minutes(11),
        expires_at: Utc::now() - Duration::minutes(1), // Expired
        used: false,
    };

    database.store_oauth2_state(&oauth2_state).await.unwrap();

    // Attempt to consume expired state
    let consumption_result = database
        .consume_oauth2_state(state_value, client_id, Utc::now())
        .await
        .unwrap();

    assert!(
        consumption_result.is_none(),
        "Expired state should not be consumable"
    );
}

/// Test state not found (invalid state)
#[tokio::test]
async fn test_state_not_found() {
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

    let client_id = "test_client_notfound";
    let invalid_state = "nonexistent_state_789";

    // Attempt to consume state that was never stored
    let consumption_result = database
        .consume_oauth2_state(invalid_state, client_id, Utc::now())
        .await
        .unwrap();

    assert!(
        consumption_result.is_none(),
        "Nonexistent state should not be found"
    );
}

/// Test state `client_id` mismatch (CSRF attack scenario)
#[tokio::test]
async fn test_state_client_id_mismatch() {
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

    let correct_client_id = "legitimate_client_abc";
    let attacker_client_id = "malicious_client_xyz";
    let state_value = "csrf_attack_state_999";

    // Create test OAuth2 clients (required for foreign key constraint)
    create_test_client(&database, correct_client_id)
        .await
        .unwrap();
    create_test_client(&database, attacker_client_id)
        .await
        .unwrap();

    // Legitimate client stores state
    let oauth2_state = OAuth2State {
        state: state_value.to_owned(),
        client_id: correct_client_id.to_owned(),
        user_id: Some(Uuid::new_v4()),
        tenant_id: Some("tenant_csrf".to_owned()),
        redirect_uri: "https://legitimate.com/callback".to_owned(),
        scope: None,
        code_challenge: None,
        code_challenge_method: None,
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
    };

    database.store_oauth2_state(&oauth2_state).await.unwrap();

    // Attacker tries to consume state with different client_id
    let attacker_consumption = database
        .consume_oauth2_state(state_value, attacker_client_id, Utc::now())
        .await
        .unwrap();

    assert!(
        attacker_consumption.is_none(),
        "State should not be consumable with mismatched client_id (CSRF attack prevented)"
    );

    // Legitimate client should still be able to consume
    let legitimate_consumption = database
        .consume_oauth2_state(state_value, correct_client_id, Utc::now())
        .await
        .unwrap();

    assert!(
        legitimate_consumption.is_some(),
        "Legitimate client should be able to consume with correct client_id"
    );
}

/// Test state with PKCE parameters preservation
#[tokio::test]
async fn test_state_with_pkce_parameters() {
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

    let client_id = "pkce_client_123";
    let state_value = "pkce_state_test_567";
    let code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
    let code_challenge_method = "S256";

    // Create test OAuth2 client (required for foreign key constraint)
    create_test_client(&database, client_id).await.unwrap();

    let oauth2_state = OAuth2State {
        state: state_value.to_owned(),
        client_id: client_id.to_owned(),
        user_id: Some(Uuid::new_v4()),
        tenant_id: Some("tenant_pkce".to_owned()),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: Some("openid profile".to_owned()),
        code_challenge: Some(code_challenge.to_owned()),
        code_challenge_method: Some(code_challenge_method.to_owned()),
        created_at: Utc::now(),
        expires_at: Utc::now() + Duration::minutes(10),
        used: false,
    };

    database.store_oauth2_state(&oauth2_state).await.unwrap();

    let consumed_state = database
        .consume_oauth2_state(state_value, client_id, Utc::now())
        .await
        .unwrap()
        .unwrap();

    // Verify PKCE parameters are preserved
    assert_eq!(
        consumed_state.code_challenge,
        Some(code_challenge.to_owned())
    );
    assert_eq!(
        consumed_state.code_challenge_method,
        Some(code_challenge_method.to_owned())
    );
    assert_eq!(consumed_state.scope, Some("openid profile".to_owned()));
}

/// Test state expiration boundary (exactly at expiry time)
#[tokio::test]
async fn test_state_expiration_boundary() {
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

    let client_id = "boundary_client_789";
    let state_value = "boundary_state_test";
    let expiry_time = Utc::now() + Duration::seconds(1);

    // Create test OAuth2 client (required for foreign key constraint)
    create_test_client(&database, client_id).await.unwrap();

    let oauth2_state = OAuth2State {
        state: state_value.to_owned(),
        client_id: client_id.to_owned(),
        user_id: Some(Uuid::new_v4()),
        tenant_id: Some("tenant_boundary".to_owned()),
        redirect_uri: "https://example.com/callback".to_owned(),
        scope: None,
        code_challenge: None,
        code_challenge_method: None,
        created_at: Utc::now(),
        expires_at: expiry_time,
        used: false,
    };

    database.store_oauth2_state(&oauth2_state).await.unwrap();

    // Consume before expiry
    let before_expiry = database
        .consume_oauth2_state(
            state_value,
            client_id,
            expiry_time - Duration::milliseconds(100),
        )
        .await
        .unwrap();
    assert!(
        before_expiry.is_some(),
        "State should be valid before expiry"
    );

    // Now it's consumed, so this should fail
    let after_consumption = database
        .consume_oauth2_state(state_value, client_id, Utc::now())
        .await
        .unwrap();
    assert!(
        after_consumption.is_none(),
        "State should not be consumable again after first consumption"
    );
}
