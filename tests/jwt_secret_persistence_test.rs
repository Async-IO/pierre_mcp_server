// ABOUTME: Test to verify JWT secret persistence across server restarts
// ABOUTME: Ensures admin tokens remain valid after server restart - fixes the 12-hour issue
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use anyhow::Result;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    admin::{jwt::AdminJwtManager, models::CreateAdminTokenRequest},
    database_plugins::{factory::Database, DatabaseProvider},
    key_management::KeyManager,
};
use serial_test::serial;
use std::env;
use tempfile::TempDir;

#[tokio::test]
#[serial]
async fn test_jwt_secret_persistence_across_restarts() -> Result<()> {
    // Create temporary directory for test database
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_jwt_persistence.db");
    let db_url = format!("sqlite:{}", db_path.display());

    // Set consistent MEK for test (32 bytes base64 encoded) - required for KeyManager::bootstrap()
    env::set_var(
        "PIERRE_MASTER_ENCRYPTION_KEY",
        "YmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmJiYmI=",
    );

    // Initialize JWKS manager for RS256 admin token signing (shared across "restarts")
    let jwks_manager = common::get_shared_test_jwks();

    // Step 1: First initialization - simulate admin-setup
    let jwt_secret_1 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        #[cfg(feature = "postgresql")]
        let mut database = Database::new(
            &db_url,
            database_key.to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await?;
        #[cfg(not(feature = "postgresql"))]
        let mut database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&mut database).await?;

        // Get/create JWT secret (simulating admin-setup)
        let jwt_secret = database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?;

        // Create admin token with this secret
        let request = CreateAdminTokenRequest {
            service_name: "test_service".into(),
            service_description: Some("Test token".into()),
            permissions: None,
            expires_in_days: Some(1),
            is_super_admin: true,
        };

        let generated_token = database
            .create_admin_token(&request, &jwt_secret, &jwks_manager)
            .await?;
        println!("Generated token: {}", generated_token.jwt_token);

        (jwt_secret, generated_token.jwt_token)
    };

    // Step 2: Second initialization - simulate server restart
    let jwt_secret_2 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        #[cfg(feature = "postgresql")]
        let mut database = Database::new(
            &db_url,
            database_key.to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await?;
        #[cfg(not(feature = "postgresql"))]
        let mut database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&mut database).await?;

        // Get JWT secret again (simulating server restart)
        database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?
    };

    // Step 3: Verify JWT secrets are identical
    assert_eq!(
        jwt_secret_1.0, jwt_secret_2,
        "JWT secret changed between restarts! This causes admin token invalidation."
    );

    // Step 4: Verify admin token can be validated with persistent secret using RS256
    let jwt_manager = AdminJwtManager::new();

    // This should NOT fail with InvalidSignature (using RS256 validation)
    let validation_result = jwt_manager.validate_token(&jwt_secret_1.1, &jwks_manager);
    assert!(
        validation_result.is_ok(),
        "Admin token validation failed after restart: {:?}",
        validation_result.err()
    );

    // Clean up test environment variable
    env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    println!(" JWT secret persistence test PASSED");
    println!(" Admin tokens survive server restarts");
    println!(" No more 12-hour InvalidSignature issue");

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_mek_ensures_consistent_jwt_storage() -> Result<()> {
    // This test verifies that the MEK properly encrypts/decrypts JWT secrets
    // ensuring they remain consistent across restarts

    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_mek_jwt.db");
    let db_url = format!("sqlite:{}", db_path.display());

    // Set consistent MEK for test (32 bytes base64 encoded)
    env::set_var(
        "PIERRE_MASTER_ENCRYPTION_KEY",
        "YWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWFhYWE=",
    );

    let jwt_secret_1 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        #[cfg(feature = "postgresql")]
        let mut database = Database::new(
            &db_url,
            database_key.to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await?;
        #[cfg(not(feature = "postgresql"))]
        let mut database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&mut database).await?;
        database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?
    };

    let jwt_secret_2 = {
        let (mut key_manager, database_key) = KeyManager::bootstrap()?;
        #[cfg(feature = "postgresql")]
        let mut database = Database::new(
            &db_url,
            database_key.to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await?;
        #[cfg(not(feature = "postgresql"))]
        let mut database = Database::new(&db_url, database_key.to_vec()).await?;
        key_manager.complete_initialization(&mut database).await?;
        database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?
    };

    assert_eq!(
        jwt_secret_1, jwt_secret_2,
        "MEK-encrypted JWT secret storage failed - secrets differ across restarts"
    );

    // Clean up test environment variable
    env::remove_var("PIERRE_MASTER_ENCRYPTION_KEY");

    println!(" MEK-based JWT secret storage test PASSED");

    Ok(())
}
