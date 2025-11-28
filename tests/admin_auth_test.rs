// ABOUTME: Integration tests for admin authentication and authorization system
// ABOUTME: Tests authentication flow, permissions, and token validation using real database connections
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use pierre_mcp_server::admin::{
    auth::AdminAuthService,
    jwt::AdminJwtManager,
    models::{AdminPermission, AdminPermissions},
};
use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::factory::Database;

#[tokio::test]
async fn test_admin_authentication_flow() {
    // Create test database
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        "sqlite::memory:",
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await
    .unwrap();

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();

    // Create JWKS manager for RS256 and generate keys
    let jwks_manager = common::get_shared_test_jwks();

    // Create auth service
    let jwt_secret = "test_jwt_secret_for_admin_auth";
    let auth_service = AdminAuthService::new(
        database.clone(),
        jwks_manager.clone(),
        AdminAuthService::DEFAULT_CACHE_TTL_SECS,
    );

    // Manually create an RS256 token with a known secret and store it in database
    let jwt_manager = AdminJwtManager::new();
    let test_token = jwt_manager
        .generate_token(
            "test_token_123",
            "test_service",
            &AdminPermissions::default_admin(),
            false,
            Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            &jwks_manager,
        )
        .unwrap();

    // Generate token hash and prefix for storage
    let token_prefix = AdminJwtManager::generate_token_prefix(&test_token);
    let token_hash = AdminJwtManager::hash_token_for_storage(&test_token).unwrap();
    let jwt_secret_hash = AdminJwtManager::hash_secret(jwt_secret);

    // Store token in database manually
    let permissions_json = AdminPermissions::default_admin().to_json().unwrap();
    match &database {
        Database::SQLite(sqlite_db) => {
            sqlx::query(
                r"
                INSERT INTO admin_tokens (
                    id, service_name, token_hash, token_prefix,
                    jwt_secret_hash, permissions, is_super_admin, is_active,
                    created_at, usage_count
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ",
            )
            .bind("test_token_123")
            .bind("test_service")
            .bind(&token_hash)
            .bind(&token_prefix)
            .bind(&jwt_secret_hash)
            .bind(&permissions_json)
            .bind(false)
            .bind(true)
            .bind(chrono::Utc::now())
            .bind(0)
            .execute(sqlite_db.pool())
            .await
            .unwrap();
        }
        #[cfg(feature = "postgresql")]
        Database::PostgreSQL(_) => {
            panic!("PostgreSQL not supported in this test");
        }
    }

    // Test authentication
    let result = auth_service
        .authenticate_and_authorize(
            &test_token,
            AdminPermission::ProvisionKeys,
            Some("127.0.0.1"),
        )
        .await;

    if result.is_err() {
        println!("Auth test error: {}", result.as_ref().unwrap_err());
    }
    assert!(result.is_ok());
    let validated = result.unwrap();
    assert_eq!(validated.service_name, "test_service");
    assert!(validated
        .permissions
        .has_permission(&AdminPermission::ProvisionKeys));
}
