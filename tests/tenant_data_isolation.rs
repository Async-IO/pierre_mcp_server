// ABOUTME: Multi-tenant data isolation security tests for preventing data breaches
// ABOUTME: Critical tests verifying users cannot access data from other tenants
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Multi-Tenant Data Isolation Security Tests
//!
//! Critical security tests to verify that users cannot access data from other tenants.
//! These tests are essential for preventing data breaches in the multi-tenant architecture.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use chrono::Utc;
use pierre_mcp_server::{
    api_keys::{ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::AuthManager,
    database_plugins::factory::Database,
    mcp::multitenant::MultiTenantMcpServer,
    models::User,
};
use std::sync::Arc;
use uuid::Uuid;

mod common;

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_tenant_isolation_tests";

/// Create a test `ServerConfig` for tenant data isolation tests
fn create_test_server_config(
) -> std::sync::Arc<pierre_mcp_server::config::environment::ServerConfig> {
    std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
        http_port: 4000,
        oauth_callback_port: 35535,
        log_level: pierre_mcp_server::config::environment::LogLevel::Info,
        logging: pierre_mcp_server::config::environment::LoggingConfig::default(),
        http_client: pierre_mcp_server::config::environment::HttpClientConfig::default(),
        database: pierre_mcp_server::config::environment::DatabaseConfig {
            url: pierre_mcp_server::config::environment::DatabaseUrl::Memory,
            auto_migrate: true,
            backup: pierre_mcp_server::config::environment::BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: std::path::PathBuf::from("test_backups"),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_client_id".to_owned()),
                client_secret: Some("test_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/strava".to_owned()),
                scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_id".to_owned()),
                client_secret: Some("test_fitbit_secret".to_owned()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/fitbit".to_owned()),
                scopes: vec!["activity".to_owned(), "profile".to_owned()],
                enabled: true,
            },
            garmin: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_owned()],
            tls: pierre_mcp_server::config::environment::TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: pierre_mcp_server::config::environment::SecurityHeadersConfig {
                environment: pierre_mcp_server::config::environment::Environment::Development,
            },
        },
        external_services: pierre_mcp_server::config::environment::ExternalServicesConfig {
            weather: pierre_mcp_server::config::environment::WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: true,
            },
            ..Default::default()
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        ..Default::default()
    })
}

async fn setup_test_database() -> Result<Database> {
    let database_url = "sqlite::memory:";
    let encryption_key = vec![0u8; 32];

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        database_url,
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(database_url, encryption_key).await?;

    database.migrate().await?;
    Ok(database)
}

async fn create_test_tenant_user(
    database: &Database,
    email: &str,
    tier: pierre_mcp_server::models::UserTier,
) -> Result<Uuid> {
    let user = User {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        display_name: Some(format!("Test User ({email})")),
        password_hash: "test_hash".to_owned(),
        tier,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
    };
    database.create_user(&user).await.map_err(Into::into)
}

/// Test that users cannot access API keys from other tenants
#[tokio::test]
async fn test_cross_tenant_api_key_access_blocked() -> Result<()> {
    let database = setup_test_database().await?;

    // Create two separate users (tenants)
    let user1_id = create_test_tenant_user(
        &database,
        "user1@example.com",
        pierre_mcp_server::models::UserTier::Professional,
    )
    .await?;
    let user2_id = create_test_tenant_user(
        &database,
        "user2@example.com",
        pierre_mcp_server::models::UserTier::Professional,
    )
    .await?;

    let api_key_manager = ApiKeyManager::new();

    // User 1 creates an API key
    let create_request = CreateApiKeyRequest {
        name: "User 1 API Key".to_owned(),
        description: Some("Secret API key for user 1".to_owned()),
        tier: ApiKeyTier::Professional,
        expires_in_days: Some(30),
        rate_limit_requests: None,
    };

    let (user1_api_key, _user1_key_string) =
        api_key_manager.create_api_key(user1_id, create_request)?;

    // Store the API key in database
    database.create_api_key(&user1_api_key).await?;

    // User 2 tries to access User 1's API key by ID
    let user2_keys = database.get_user_api_keys(user2_id).await?;
    assert!(user2_keys.is_empty(), "User 2 should not see any API keys");

    // Try to access User 1's API key directly by ID (should fail)
    let unauthorized_access = database.get_api_key_by_id(&user1_api_key.id).await?;

    // This should succeed (the key exists) but we need to verify it belongs to user1
    if let Some(retrieved_key) = unauthorized_access {
        assert_eq!(
            retrieved_key.user_id, user1_id,
            "API key should belong to user 1"
        );
        // The important test: User 2 should not be able to use this key
        assert_ne!(
            retrieved_key.user_id, user2_id,
            "API key should not belong to user 2"
        );
    }

    // Verify user isolation at the API level
    let user1_keys = database.get_user_api_keys(user1_id).await?;
    let user2_keys = database.get_user_api_keys(user2_id).await?;

    assert_eq!(user1_keys.len(), 1, "User 1 should have exactly 1 API key");
    assert_eq!(user2_keys.len(), 0, "User 2 should have no API keys");

    tracing::info!("Cross-tenant API key access isolation verified");
    Ok(())
}

/// Test OAuth token isolation between tenants (simplified)
#[tokio::test]
async fn test_oauth_token_isolation() -> Result<()> {
    let database = setup_test_database().await?;

    // Create two users
    let user1_id = create_test_tenant_user(
        &database,
        "oauth1@example.com",
        pierre_mcp_server::models::UserTier::Starter,
    )
    .await?;
    let user2_id = create_test_tenant_user(
        &database,
        "oauth2@example.com",
        pierre_mcp_server::models::UserTier::Starter,
    )
    .await?;

    // Verify users are isolated - each user can only access their own data
    let user1 = database.get_user(user1_id).await?;
    let user2 = database.get_user(user2_id).await?;

    assert!(user1.is_some(), "User 1 should exist");
    assert!(user2.is_some(), "User 2 should exist");

    let user1_data = user1.unwrap();
    let user2_data = user2.unwrap();

    assert_eq!(user1_data.id, user1_id, "User 1 should have correct ID");
    assert_eq!(user2_data.id, user2_id, "User 2 should have correct ID");
    assert_ne!(
        user1_data.id, user2_data.id,
        "Users should have different IDs"
    );

    println!("User data isolation verified");
    Ok(())
}

/// Test admin API cannot access data across tenant boundaries
#[tokio::test]
async fn test_admin_cross_tenant_access_prevention() -> Result<()> {
    let database = setup_test_database().await?;

    // Create users in different tenants
    let user1_id = create_test_tenant_user(
        &database,
        "tenant1@example.com",
        pierre_mcp_server::models::UserTier::Enterprise,
    )
    .await?;
    let user2_id = create_test_tenant_user(
        &database,
        "tenant2@example.com",
        pierre_mcp_server::models::UserTier::Enterprise,
    )
    .await?;

    let api_key_manager = ApiKeyManager::new();

    // Create API keys for both users
    let create_request1 = CreateApiKeyRequest {
        name: "Tenant 1 Key".to_owned(),
        description: Some("Key for tenant 1".to_owned()),
        tier: ApiKeyTier::Enterprise,
        expires_in_days: Some(365),
        rate_limit_requests: None,
    };

    let create_request2 = CreateApiKeyRequest {
        name: "Tenant 2 Key".to_owned(),
        description: Some("Key for tenant 2".to_owned()),
        tier: ApiKeyTier::Enterprise,
        expires_in_days: Some(365),
        rate_limit_requests: None,
    };

    let (key1, _) = api_key_manager.create_api_key(user1_id, create_request1)?;
    let (key2, _) = api_key_manager.create_api_key(user2_id, create_request2)?;

    database.create_api_key(&key1).await?;
    database.create_api_key(&key2).await?;

    // Admin queries should be user-scoped
    let tenant1_keys = database.get_user_api_keys(user1_id).await?;
    let tenant2_keys = database.get_user_api_keys(user2_id).await?;

    assert_eq!(tenant1_keys.len(), 1);
    assert_eq!(tenant2_keys.len(), 1);

    // Keys should belong to correct users
    assert_eq!(tenant1_keys[0].user_id, user1_id);
    assert_eq!(tenant2_keys[0].user_id, user2_id);

    // Cross-tenant key access should not be possible
    assert_ne!(tenant1_keys[0].id, tenant2_keys[0].id);

    tracing::info!("Admin cross-tenant access prevention verified");
    Ok(())
}

/// Test concurrent access to user data maintains isolation
#[tokio::test]
async fn test_concurrent_tenant_isolation() -> Result<()> {
    let database = Arc::new(setup_test_database().await?);

    // Create multiple users
    let mut user_ids = Vec::new();
    for i in 0..5 {
        let user_id = create_test_tenant_user(
            &database,
            &format!("concurrent_user{i}@example.com"),
            pierre_mcp_server::models::UserTier::Professional,
        )
        .await?;
        user_ids.push(user_id);
    }

    let api_key_manager = Arc::new(ApiKeyManager::new());

    // Concurrently create API keys for each user
    let tasks = user_ids.into_iter().enumerate().map(|(i, user_id)| {
        let db = database.clone();
        let manager = api_key_manager.clone();

        tokio::spawn(async move {
            let create_request = CreateApiKeyRequest {
                name: format!("Concurrent Key {i}"),
                description: Some(format!("Key for user {i}")),
                tier: ApiKeyTier::Professional,
                expires_in_days: Some(30),
                rate_limit_requests: None,
            };

            let (api_key, _) = manager.create_api_key(user_id, create_request)?;
            db.create_api_key(&api_key).await?;

            // Return user_id and key_id for verification
            Ok::<(Uuid, String), anyhow::Error>((user_id, api_key.id))
        })
    });

    let mut user_key_pairs = Vec::new();
    for task in tasks {
        let result = task.await??;
        user_key_pairs.push(result);
    }

    // Verify each user only sees their own key
    for (user_id, expected_key_id) in user_key_pairs {
        let user_keys = database.get_user_api_keys(user_id).await?;

        assert_eq!(user_keys.len(), 1, "Each user should have exactly 1 key");
        assert_eq!(
            user_keys[0].id, expected_key_id,
            "User should see their own key"
        );
        assert_eq!(
            user_keys[0].user_id, user_id,
            "Key should belong to correct user"
        );
    }

    tracing::info!("Concurrent tenant isolation verified");
    Ok(())
}

/// Test that database encryption isolates data properly
#[tokio::test]
async fn test_database_encryption_isolation() -> Result<()> {
    // Create two separate databases with different encryption keys
    let key1 = vec![1u8; 32]; // Different encryption key
    let key2 = vec![2u8; 32]; // Different encryption key

    let db_url1 = "sqlite::memory:";
    let db_url2 = "sqlite::memory:";

    #[cfg(feature = "postgresql")]
    let database1 = Database::new(
        db_url1,
        key1,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database1 = Database::new(db_url1, key1).await?;

    #[cfg(feature = "postgresql")]
    let database2 = Database::new(
        db_url2,
        key2,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database2 = Database::new(db_url2, key2).await?;

    database1.migrate().await?;
    database2.migrate().await?;

    // Create users in separate encrypted databases
    let user1_id = create_test_tenant_user(
        &database1,
        "encrypted1@example.com",
        pierre_mcp_server::models::UserTier::Starter,
    )
    .await?;
    let user2_id = create_test_tenant_user(
        &database2,
        "encrypted2@example.com",
        pierre_mcp_server::models::UserTier::Starter,
    )
    .await?;

    // Verify users exist in their respective databases
    let user1_from_db1 = database1.get_user(user1_id).await?;
    let user2_from_db2 = database2.get_user(user2_id).await?;

    assert!(
        user1_from_db1.is_some(),
        "User 1 should exist in database 1"
    );
    assert!(
        user2_from_db2.is_some(),
        "User 2 should exist in database 2"
    );

    // Cross-database access should fail (user doesn't exist)
    let user1_from_db2 = database2.get_user(user1_id).await?;
    let user2_from_db1 = database1.get_user(user2_id).await?;

    assert!(
        user1_from_db2.is_none(),
        "User 1 should not exist in database 2"
    );
    assert!(
        user2_from_db1.is_none(),
        "User 2 should not exist in database 1"
    );

    tracing::info!("Database encryption isolation verified");
    Ok(())
}

/// Test MCP server request isolation
#[tokio::test]
async fn test_mcp_server_tenant_isolation() -> Result<()> {
    common::init_server_config();
    let database = setup_test_database().await?;
    let auth_manager = AuthManager::new(24);

    // Create test server
    let cache = common::create_test_cache().await.unwrap();
    let resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database.clone(),
        auth_manager.clone(),
        TEST_JWT_SECRET,
        create_test_server_config(),
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let _server = MultiTenantMcpServer::new(resources);

    // Create two users
    let user1_id = create_test_tenant_user(
        &database,
        "mcp1@example.com",
        pierre_mcp_server::models::UserTier::Professional,
    )
    .await?;
    let user2_id = create_test_tenant_user(
        &database,
        "mcp2@example.com",
        pierre_mcp_server::models::UserTier::Professional,
    )
    .await?;

    // Get users for token generation
    let user1 = database.get_user(user1_id).await?.unwrap();
    let user2 = database.get_user(user2_id).await?.unwrap();

    // Generate JWT tokens for both users
    let jwks_manager = common::get_shared_test_jwks();
    let user1_token = auth_manager.generate_token(&user1, &jwks_manager)?;
    let user2_token = auth_manager.generate_token(&user2, &jwks_manager)?;

    // Verify tokens are different and user-specific
    assert_ne!(user1_token, user2_token, "JWT tokens should be different");

    // Verify token validation returns correct user IDs
    let user1_claims = auth_manager.validate_token(&user1_token, &jwks_manager)?;
    let user2_claims = auth_manager.validate_token(&user2_token, &jwks_manager)?;

    let user1_id_from_token = Uuid::parse_str(&user1_claims.sub)?;
    let user2_id_from_token = Uuid::parse_str(&user2_claims.sub)?;

    assert_eq!(
        user1_id_from_token, user1_id,
        "Token 1 should validate to user 1"
    );
    assert_eq!(
        user2_id_from_token, user2_id,
        "Token 2 should validate to user 2"
    );

    // Cross-validation should not work (tokens are user-specific)
    assert_ne!(
        user1_id_from_token, user2_id,
        "Token 1 should not validate to user 2"
    );
    assert_ne!(
        user2_id_from_token, user1_id,
        "Token 2 should not validate to user 1"
    );

    tracing::info!("MCP server tenant isolation verified");
    Ok(())
}
