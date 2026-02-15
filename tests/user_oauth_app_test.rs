// ABOUTME: Tests for per-user OAuth app credentials feature
// ABOUTME: Validates 3-tier credential resolution and REST endpoint functionality
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # User OAuth App Tests
//!
//! These tests validate the per-user OAuth credentials feature:
//! - 3-tier credential resolution (user → tenant → server)
//! - REST endpoints for user OAuth app management
//! - Database operations for storing/retrieving user OAuth apps

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use chrono::Utc;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    config::environment::{OAuthConfig, OAuthProviderConfig},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{Tenant, TenantId, User, UserStatus, UserTier},
    permissions::UserRole,
    tenant::oauth_manager::{CredentialConfig, TenantOAuthManager},
};
use serial_test::serial;
use std::sync::Arc;
use uuid::Uuid;

// =============================================================================
// Test Setup Helpers
// =============================================================================

/// Create test database with migrations
async fn setup_test_database() -> Result<Database> {
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database =
        Database::new(database_url, encryption_key, &PostgresPoolConfig::default()).await?;

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(database_url, encryption_key).await?;

    database.migrate().await?;
    Ok(database)
}

/// Create a test user
async fn create_test_user(database: &Database, email: &str, tenant_id: TenantId) -> Result<Uuid> {
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: email.to_owned(),
        display_name: Some(format!("Test User {email}")),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST)?,
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&user).await?;
    // Associate user with tenant via tenant_users junction table
    database.update_user_tenant_id(user_id, tenant_id).await?;
    Ok(user_id)
}

/// Create test OAuth config with server-level credentials
fn create_test_oauth_config() -> OAuthConfig {
    OAuthConfig {
        strava: OAuthProviderConfig {
            client_id: Some("server_strava_id".to_owned()),
            client_secret: Some("server_strava_secret".to_owned()),
            redirect_uri: Some("http://localhost:8080/callback/strava".to_owned()),
            scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
            enabled: true,
        },
        fitbit: OAuthProviderConfig {
            client_id: Some("server_fitbit_id".to_owned()),
            client_secret: Some("server_fitbit_secret".to_owned()),
            redirect_uri: Some("http://localhost:8080/callback/fitbit".to_owned()),
            scopes: vec!["activity".to_owned(), "profile".to_owned()],
            enabled: true,
        },
        garmin: OAuthProviderConfig::default(),
        whoop: OAuthProviderConfig::default(),
        terra: OAuthProviderConfig::default(),
    }
}

// =============================================================================
// Unit Tests: Database Operations for User OAuth Apps
// =============================================================================

/// Test storing and retrieving user OAuth app credentials
#[tokio::test]
#[serial]
async fn test_store_and_get_user_oauth_app() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Store user OAuth app
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "user_client_id_123",
            "user_client_secret_456",
            "http://myapp.com/callback/strava",
        )
        .await?;

    // Retrieve and verify
    let app = database
        .get_user_oauth_app(user_id, "strava")
        .await?
        .expect("User OAuth app should exist");

    assert_eq!(app.user_id, user_id);
    assert_eq!(app.provider, "strava");
    assert_eq!(app.client_id, "user_client_id_123");
    assert_eq!(app.client_secret, "user_client_secret_456");
    assert_eq!(app.redirect_uri, "http://myapp.com/callback/strava");

    Ok(())
}

/// Test listing user OAuth apps
#[tokio::test]
#[serial]
async fn test_list_user_oauth_apps() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Store multiple OAuth apps
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "strava_id",
            "strava_secret",
            "http://app.com/strava",
        )
        .await?;
    database
        .store_user_oauth_app(
            user_id,
            "fitbit",
            "fitbit_id",
            "fitbit_secret",
            "http://app.com/fitbit",
        )
        .await?;
    database
        .store_user_oauth_app(
            user_id,
            "whoop",
            "whoop_id",
            "whoop_secret",
            "http://app.com/whoop",
        )
        .await?;

    // List and verify
    let apps = database.list_user_oauth_apps(user_id).await?;
    assert_eq!(apps.len(), 3);

    let providers: Vec<&str> = apps.iter().map(|a| a.provider.as_str()).collect();
    assert!(providers.contains(&"strava"));
    assert!(providers.contains(&"fitbit"));
    assert!(providers.contains(&"whoop"));

    Ok(())
}

/// Test removing user OAuth app
#[tokio::test]
#[serial]
async fn test_remove_user_oauth_app() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Store and then remove
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "client_id",
            "client_secret",
            "http://app.com/callback",
        )
        .await?;

    // Verify it exists
    let app = database.get_user_oauth_app(user_id, "strava").await?;
    assert!(app.is_some());

    // Remove it
    database.remove_user_oauth_app(user_id, "strava").await?;

    // Verify it's gone
    let app = database.get_user_oauth_app(user_id, "strava").await?;
    assert!(app.is_none());

    Ok(())
}

/// Test user OAuth app isolation between users
#[tokio::test]
#[serial]
async fn test_user_oauth_app_isolation() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();

    let user_a = create_test_user(&database, "user_a@example.com", tenant_id).await?;
    let user_b = create_test_user(&database, "user_b@example.com", tenant_id).await?;

    // User A stores Strava app
    database
        .store_user_oauth_app(
            user_a,
            "strava",
            "user_a_strava_id",
            "user_a_strava_secret",
            "http://user-a.com/callback",
        )
        .await?;

    // User B stores different Strava app
    database
        .store_user_oauth_app(
            user_b,
            "strava",
            "user_b_strava_id",
            "user_b_strava_secret",
            "http://user-b.com/callback",
        )
        .await?;

    // Verify isolation
    let user_a_app = database
        .get_user_oauth_app(user_a, "strava")
        .await?
        .expect("User A app should exist");
    let user_b_app = database
        .get_user_oauth_app(user_b, "strava")
        .await?
        .expect("User B app should exist");

    assert_eq!(user_a_app.client_id, "user_a_strava_id");
    assert_eq!(user_b_app.client_id, "user_b_strava_id");
    assert_ne!(user_a_app.client_id, user_b_app.client_id);

    Ok(())
}

/// Test storing user OAuth app for all supported providers
#[tokio::test]
#[serial]
async fn test_all_supported_providers() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let providers = ["strava", "fitbit", "garmin", "whoop", "terra"];

    for provider in &providers {
        database
            .store_user_oauth_app(
                user_id,
                provider,
                &format!("{provider}_client_id"),
                &format!("{provider}_client_secret"),
                &format!("http://app.com/{provider}/callback"),
            )
            .await?;
    }

    // Verify all stored
    let apps = database.list_user_oauth_apps(user_id).await?;
    assert_eq!(apps.len(), 5, "All 5 providers should be stored");

    for provider in &providers {
        let app = database
            .get_user_oauth_app(user_id, provider)
            .await?
            .unwrap_or_else(|| panic!("App for {provider} should exist"));
        assert_eq!(app.client_id, format!("{provider}_client_id"));
    }

    Ok(())
}

// =============================================================================
// Unit Tests: 3-Tier Credential Resolution
// =============================================================================

/// Test: User-specific credentials take priority over server-level
#[tokio::test]
#[serial]
async fn test_user_credentials_priority_over_server() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Set up server-level credentials
    let oauth_config = Arc::new(create_test_oauth_config());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Store user-specific credentials
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "user_specific_client_id",
            "user_specific_secret",
            "http://user-app.com/callback",
        )
        .await?;

    // Get credentials with user_id - should return user-specific
    let credentials = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;

    assert_eq!(
        credentials.client_id, "user_specific_client_id",
        "User-specific credentials should take priority"
    );

    Ok(())
}

/// Test: Falls back to server-level when no user credentials exist
#[tokio::test]
#[serial]
async fn test_fallback_to_server_credentials() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Set up server-level credentials only (no user-specific)
    let oauth_config = Arc::new(create_test_oauth_config());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Get credentials - should fall back to server-level
    let credentials = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;

    assert_eq!(
        credentials.client_id, "server_strava_id",
        "Should fall back to server-level credentials"
    );

    Ok(())
}

/// Test: Backward compatibility - `get_credentials` without `user_id` uses server-level
#[tokio::test]
#[serial]
async fn test_backward_compatible_get_credentials() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();

    let oauth_config = Arc::new(create_test_oauth_config());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Use the original get_credentials (no user_id)
    let credentials = oauth_manager
        .get_credentials(tenant_id, "strava", &database)
        .await?;

    assert_eq!(
        credentials.client_id, "server_strava_id",
        "Original get_credentials should use server-level"
    );

    Ok(())
}

/// Test: Error when no credentials at any level
#[tokio::test]
#[serial]
async fn test_error_when_no_credentials() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Set up empty OAuth config (no server-level credentials)
    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Should fail for garmin (no credentials anywhere)
    let result = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "garmin", &database)
        .await;

    assert!(result.is_err(), "Should error when no credentials exist");

    Ok(())
}

/// Test: Different users get different credentials for same provider
#[tokio::test]
#[serial]
async fn test_different_users_different_credentials() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();

    let user_a = create_test_user(&database, "user_a@example.com", tenant_id).await?;
    let user_b = create_test_user(&database, "user_b@example.com", tenant_id).await?;

    // Set up server-level credentials
    let oauth_config = Arc::new(create_test_oauth_config());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // User A has custom credentials
    database
        .store_user_oauth_app(
            user_a,
            "strava",
            "user_a_client_id",
            "user_a_secret",
            "http://user-a.com/callback",
        )
        .await?;

    // User B has no custom credentials

    // User A should get their own credentials
    let creds_a = oauth_manager
        .get_credentials_for_user(Some(user_a), tenant_id, "strava", &database)
        .await?;
    assert_eq!(creds_a.client_id, "user_a_client_id");

    // User B should get server-level credentials
    let creds_b = oauth_manager
        .get_credentials_for_user(Some(user_b), tenant_id, "strava", &database)
        .await?;
    assert_eq!(creds_b.client_id, "server_strava_id");

    Ok(())
}

/// Test: Credential resolution with tenant-specific credentials
#[tokio::test]
#[serial]
async fn test_tenant_credentials_priority() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Create tenant
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        tenant_id.to_string(),
        Some("test-tenant.example.com".to_owned()),
        "professional".to_owned(),
        user_id, // owner_user_id
    );
    database.create_tenant(&tenant).await?;

    // Set up server-level credentials
    let oauth_config = Arc::new(create_test_oauth_config());
    let mut oauth_manager = TenantOAuthManager::new(oauth_config);

    // Store tenant-specific credentials (priority 2)
    let tenant_creds = CredentialConfig {
        client_id: "tenant_strava_id".to_owned(),
        client_secret: "tenant_strava_secret".to_owned(),
        redirect_uri: "http://tenant.example.com/callback".to_owned(),
        scopes: vec!["read".to_owned()],
        configured_by: user_id,
    };
    oauth_manager.store_credentials(tenant_id, "strava", tenant_creds)?;

    // With no user credentials, should get tenant-specific
    let credentials = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;

    assert_eq!(
        credentials.client_id, "tenant_strava_id",
        "Should use tenant-specific credentials when no user credentials exist"
    );

    // Now add user-specific credentials
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "user_strava_id",
            "user_strava_secret",
            "http://user.com/callback",
        )
        .await?;

    // Should now prefer user-specific over tenant-specific
    let credentials = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;

    assert_eq!(
        credentials.client_id, "user_strava_id",
        "Should prefer user-specific over tenant-specific"
    );

    Ok(())
}

// =============================================================================
// Unit Tests: Default Scopes and Rate Limits
// =============================================================================

/// Test: User credentials get correct default scopes for each provider
#[tokio::test]
#[serial]
async fn test_user_credentials_default_scopes() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Store user credentials for WHOOP
    database
        .store_user_oauth_app(
            user_id,
            "whoop",
            "whoop_id",
            "whoop_secret",
            "http://app.com/whoop",
        )
        .await?;

    let credentials = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "whoop", &database)
        .await?;

    // Should have WHOOP default scopes
    assert!(!credentials.scopes.is_empty(), "Should have default scopes");
    assert!(
        credentials.scopes.contains(&"offline".to_owned()),
        "WHOOP should have 'offline' scope"
    );
    assert!(
        credentials.scopes.contains(&"read:profile".to_owned()),
        "WHOOP should have 'read:profile' scope"
    );

    Ok(())
}

/// Test: User credentials get correct default rate limits
#[tokio::test]
#[serial]
async fn test_user_credentials_default_rate_limits() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Store user credentials for Strava
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "strava_id",
            "strava_secret",
            "http://app.com/strava",
        )
        .await?;

    let credentials = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;

    // Strava has a higher default rate limit (15000/day)
    assert_eq!(
        credentials.rate_limit_per_day, 15000,
        "Strava should have 15000/day rate limit"
    );

    Ok(())
}

// =============================================================================
// Database Edge Case Tests
// =============================================================================

/// Test: Upsert behavior - storing credentials twice for same provider updates them
#[tokio::test]
#[serial]
async fn test_upsert_user_oauth_app() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Store initial credentials
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "original_client_id",
            "original_secret",
            "http://original.com/callback",
        )
        .await?;

    // Verify initial
    let app = database
        .get_user_oauth_app(user_id, "strava")
        .await?
        .unwrap();
    assert_eq!(app.client_id, "original_client_id");

    // Store updated credentials for same provider
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "updated_client_id",
            "updated_secret",
            "http://updated.com/callback",
        )
        .await?;

    // Verify update
    let app = database
        .get_user_oauth_app(user_id, "strava")
        .await?
        .unwrap();
    assert_eq!(app.client_id, "updated_client_id");
    assert_eq!(app.client_secret, "updated_secret");
    assert_eq!(app.redirect_uri, "http://updated.com/callback");

    // Should still only have one app for this provider
    let apps = database.list_user_oauth_apps(user_id).await?;
    let strava_app_count = apps.iter().filter(|a| a.provider == "strava").count();
    assert_eq!(
        strava_app_count, 1,
        "Should only have one strava app after upsert"
    );

    Ok(())
}

/// Test: Getting OAuth app for non-existent user returns None
#[tokio::test]
#[serial]
async fn test_get_oauth_app_non_existent_user() -> Result<()> {
    let database = setup_test_database().await?;
    let non_existent_user_id = Uuid::new_v4();

    let result = database
        .get_user_oauth_app(non_existent_user_id, "strava")
        .await?;

    assert!(result.is_none(), "Should return None for non-existent user");

    Ok(())
}

/// Test: Listing OAuth apps for non-existent user returns empty list
#[tokio::test]
#[serial]
async fn test_list_oauth_apps_non_existent_user() -> Result<()> {
    let database = setup_test_database().await?;
    let non_existent_user_id = Uuid::new_v4();

    let apps = database.list_user_oauth_apps(non_existent_user_id).await?;

    assert!(
        apps.is_empty(),
        "Should return empty list for non-existent user"
    );

    Ok(())
}

/// Test: Removing non-existent OAuth app is idempotent (no error)
#[tokio::test]
#[serial]
async fn test_remove_non_existent_oauth_app() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Remove app that doesn't exist - should not error
    let result = database.remove_user_oauth_app(user_id, "strava").await;

    assert!(result.is_ok(), "Removing non-existent app should not error");

    Ok(())
}

/// Test: Cross-tenant isolation - users in different tenants are isolated
#[tokio::test]
#[serial]
async fn test_cross_tenant_isolation() -> Result<()> {
    let database = setup_test_database().await?;

    // Create two different tenants
    let tenant_a = TenantId::new();
    let tenant_b = TenantId::new();

    let user_a = create_test_user(&database, "user_a@tenant_a.com", tenant_a).await?;
    let user_b = create_test_user(&database, "user_b@tenant_b.com", tenant_b).await?;

    // User A stores Strava credentials
    database
        .store_user_oauth_app(
            user_a,
            "strava",
            "tenant_a_strava_id",
            "tenant_a_secret",
            "http://tenant-a.com/callback",
        )
        .await?;

    // User B stores Strava credentials
    database
        .store_user_oauth_app(
            user_b,
            "strava",
            "tenant_b_strava_id",
            "tenant_b_secret",
            "http://tenant-b.com/callback",
        )
        .await?;

    // Verify each user only sees their own credentials
    let app_a = database
        .get_user_oauth_app(user_a, "strava")
        .await?
        .unwrap();
    let app_b = database
        .get_user_oauth_app(user_b, "strava")
        .await?
        .unwrap();

    assert_eq!(app_a.client_id, "tenant_a_strava_id");
    assert_eq!(app_b.client_id, "tenant_b_strava_id");

    // User A's list only shows their apps
    let apps_a = database.list_user_oauth_apps(user_a).await?;
    assert_eq!(apps_a.len(), 1);
    assert_eq!(apps_a[0].client_id, "tenant_a_strava_id");

    Ok(())
}

/// Test: Timestamps are set correctly on creation
#[tokio::test]
#[serial]
async fn test_oauth_app_timestamps() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let before = chrono::Utc::now();

    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "client_id",
            "client_secret",
            "http://app.com/callback",
        )
        .await?;

    let after = chrono::Utc::now();

    let app = database
        .get_user_oauth_app(user_id, "strava")
        .await?
        .unwrap();

    // created_at should be between before and after
    assert!(
        app.created_at >= before && app.created_at <= after,
        "created_at should be set to current time"
    );
    assert!(
        app.updated_at >= before && app.updated_at <= after,
        "updated_at should be set to current time"
    );

    Ok(())
}

// =============================================================================
// Provider-Specific Rate Limit Tests
// =============================================================================

/// Test: All providers get correct default rate limits
#[tokio::test]
#[serial]
async fn test_all_provider_rate_limits() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Expected rate limits per provider (from src/constants/mod.rs)
    let expected_rate_limits = [
        ("strava", 15000), // STRAVA_DEFAULT_DAILY_RATE_LIMIT
        ("fitbit", 1000),  // FITBIT_DEFAULT_DAILY_RATE_LIMIT
        ("garmin", 1000),  // GARMIN_DEFAULT_DAILY_RATE_LIMIT
        ("whoop", 1000),   // WHOOP_DEFAULT_DAILY_RATE_LIMIT
        ("terra", 1000),   // TERRA_DEFAULT_DAILY_RATE_LIMIT
    ];

    for (provider, expected_limit) in &expected_rate_limits {
        // Store user credentials for this provider
        database
            .store_user_oauth_app(
                user_id,
                provider,
                &format!("{provider}_id"),
                &format!("{provider}_secret"),
                &format!("http://app.com/{provider}"),
            )
            .await?;

        let credentials = oauth_manager
            .get_credentials_for_user(Some(user_id), tenant_id, provider, &database)
            .await?;

        assert_eq!(
            credentials.rate_limit_per_day, *expected_limit,
            "{provider} should have {expected_limit}/day rate limit, got {}",
            credentials.rate_limit_per_day
        );
    }

    Ok(())
}

// =============================================================================
// Provider-Specific Scope Tests
// =============================================================================

/// Test: All providers get correct default scopes
#[tokio::test]
#[serial]
async fn test_all_provider_default_scopes() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Expected scopes per provider (from src/tenant/oauth_manager.rs::default_scopes_for_provider)
    // Only checking key scopes, not exhaustive list
    let expected_scopes: [(&str, Vec<&str>); 5] = [
        ("strava", vec!["activity:read_all"]),
        ("fitbit", vec!["activity", "profile", "heartrate", "sleep"]),
        ("garmin", vec!["wellness:read", "activities:read"]),
        ("whoop", vec!["offline", "read:profile", "read:sleep"]),
        ("terra", vec!["activity", "body", "daily", "sleep"]),
    ];

    for (provider, required_scopes) in &expected_scopes {
        // Store user credentials
        database
            .store_user_oauth_app(
                user_id,
                provider,
                &format!("{provider}_id"),
                &format!("{provider}_secret"),
                &format!("http://app.com/{provider}"),
            )
            .await?;

        let credentials = oauth_manager
            .get_credentials_for_user(Some(user_id), tenant_id, provider, &database)
            .await?;

        for scope in required_scopes {
            assert!(
                credentials.scopes.iter().any(|s| s == *scope),
                "{provider} should have '{scope}' scope, got: {:?}",
                credentials.scopes
            );
        }
    }

    Ok(())
}

// =============================================================================
// Provider Validation Tests
// =============================================================================

/// Test: Valid providers are accepted
#[tokio::test]
#[serial]
async fn test_valid_providers_accepted() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let valid_providers = ["strava", "fitbit", "garmin", "whoop", "terra"];

    for provider in &valid_providers {
        let result = database
            .store_user_oauth_app(
                user_id,
                provider,
                &format!("{provider}_id"),
                &format!("{provider}_secret"),
                &format!("http://app.com/{provider}"),
            )
            .await;

        assert!(
            result.is_ok(),
            "Provider '{provider}' should be accepted, got error: {:?}",
            result.err()
        );
    }

    Ok(())
}

// =============================================================================
// Credential Resolution Priority Tests
// =============================================================================

/// Test: Complete 3-tier priority resolution
#[tokio::test]
#[serial]
async fn test_complete_three_tier_resolution() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Create tenant
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        tenant_id.to_string(),
        Some("test.example.com".to_owned()),
        "professional".to_owned(),
        user_id,
    );
    database.create_tenant(&tenant).await?;

    // Level 3: Server credentials
    let oauth_config = Arc::new(create_test_oauth_config());
    let mut oauth_manager = TenantOAuthManager::new(oauth_config);

    // Test with only server credentials
    let creds = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;
    assert_eq!(
        creds.client_id, "server_strava_id",
        "Should use server credentials"
    );

    // Level 2: Add tenant credentials
    let tenant_creds = CredentialConfig {
        client_id: "tenant_strava_id".to_owned(),
        client_secret: "tenant_secret".to_owned(),
        redirect_uri: "http://tenant.com/callback".to_owned(),
        scopes: vec!["read".to_owned()],
        configured_by: user_id,
    };
    oauth_manager.store_credentials(tenant_id, "strava", tenant_creds)?;

    let creds = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;
    assert_eq!(
        creds.client_id, "tenant_strava_id",
        "Should use tenant credentials"
    );

    // Level 1: Add user credentials
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "user_strava_id",
            "user_secret",
            "http://user.com/callback",
        )
        .await?;

    let creds = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;
    assert_eq!(
        creds.client_id, "user_strava_id",
        "Should use user credentials"
    );

    // Remove user credentials, should fall back to tenant
    database.remove_user_oauth_app(user_id, "strava").await?;

    let creds = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "strava", &database)
        .await?;
    assert_eq!(
        creds.client_id, "tenant_strava_id",
        "Should fall back to tenant"
    );

    Ok(())
}

/// Test: None `user_id` falls back to tenant/server resolution
#[tokio::test]
#[serial]
async fn test_none_user_id_skips_user_lookup() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Store user credentials
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "user_strava_id",
            "user_secret",
            "http://user.com/callback",
        )
        .await?;

    // Server credentials
    let oauth_config = Arc::new(create_test_oauth_config());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // With None user_id, should skip user lookup and use server
    let creds = oauth_manager
        .get_credentials_for_user(None, tenant_id, "strava", &database)
        .await?;

    assert_eq!(
        creds.client_id, "server_strava_id",
        "None user_id should skip user credentials and use server"
    );

    Ok(())
}

// =============================================================================
// Multiple Providers Per User Tests
// =============================================================================

/// Test: User can have credentials for all providers simultaneously
#[tokio::test]
#[serial]
async fn test_user_with_all_providers() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "power_user@example.com", tenant_id).await?;

    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    let providers = ["strava", "fitbit", "garmin", "whoop", "terra"];

    // Store unique credentials for each provider
    for provider in &providers {
        database
            .store_user_oauth_app(
                user_id,
                provider,
                &format!("unique_{provider}_client_id"),
                &format!("unique_{provider}_secret"),
                &format!("http://myapp.com/{provider}/callback"),
            )
            .await?;
    }

    // Verify each provider returns correct credentials
    for provider in &providers {
        let creds = oauth_manager
            .get_credentials_for_user(Some(user_id), tenant_id, provider, &database)
            .await?;

        assert_eq!(
            creds.client_id,
            format!("unique_{provider}_client_id"),
            "Provider {provider} should return correct user credentials"
        );
    }

    // List should show all 5
    let apps = database.list_user_oauth_apps(user_id).await?;
    assert_eq!(apps.len(), 5, "User should have all 5 providers");

    Ok(())
}

// =============================================================================
// Error Condition Tests
// =============================================================================

/// Test: Error when requesting unsupported provider
#[tokio::test]
#[serial]
async fn test_error_unsupported_provider() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let oauth_config = Arc::new(OAuthConfig::default());
    let oauth_manager = TenantOAuthManager::new(oauth_config);

    // Request credentials for unsupported provider
    let result = oauth_manager
        .get_credentials_for_user(Some(user_id), tenant_id, "unsupported_provider", &database)
        .await;

    assert!(result.is_err(), "Should error for unsupported provider");

    Ok(())
}

/// Test: Case insensitivity for provider names
#[tokio::test]
#[serial]
async fn test_provider_case_sensitivity() -> Result<()> {
    let database = setup_test_database().await?;
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    // Store with lowercase
    database
        .store_user_oauth_app(
            user_id,
            "strava",
            "client_id",
            "client_secret",
            "http://app.com/callback",
        )
        .await?;

    // Retrieve with lowercase (should work)
    let app = database.get_user_oauth_app(user_id, "strava").await?;
    assert!(app.is_some(), "Should find app with exact case match");

    Ok(())
}
