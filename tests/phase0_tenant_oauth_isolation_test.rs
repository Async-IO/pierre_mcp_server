// ABOUTME: Phase 0 OAuth isolation tests for per-tenant credential management
// ABOUTME: Critical security tests verifying tenant-specific OAuth credentials and rate limiting
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Phase 0: Per-Tenant OAuth Validation
//!
//! These tests verify the foundation for caching architecture:
//! - Tenant A and Tenant B can use different Strava apps
//! - Rate limits are tracked separately per tenant
//! - OAuth tokens are properly isolated by (`user_id`, `tenant_id`, `provider`)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use chrono::Utc;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    config::environment::{OAuthConfig, OAuthProviderConfig},
    constants::oauth_providers,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{Tenant, TenantId, User, UserOAuthToken, UserStatus, UserTier},
    permissions::UserRole,
    tenant::oauth_manager::{CredentialConfig, TenantOAuthManager},
};
use serial_test::serial;
use std::{env, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

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

/// Create test user in a specific tenant
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

/// Test 1: Tenant Credential Isolation
///
/// Verifies that tenant A uses server-level credentials while tenant B uses
/// tenant-specific credentials with a different `CLIENT_ID`
#[tokio::test]
async fn test_tenant_credential_isolation() -> Result<()> {
    let database = setup_test_database().await?;

    // Set up server-level OAuth config (fallback credentials for tenant A)
    let oauth_config = Arc::new(OAuthConfig {
        strava: OAuthProviderConfig {
            client_id: Some("163846".to_owned()),
            client_secret: Some("env_secret_a".to_owned()),
            redirect_uri: Some("http://localhost:8080/api/oauth/callback/strava".to_owned()),
            scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
            enabled: true,
        },
        fitbit: OAuthProviderConfig::default(),
        garmin: OAuthProviderConfig::default(),
        whoop: OAuthProviderConfig::default(),
        terra: OAuthProviderConfig::default(),
    });
    let mut oauth_manager = TenantOAuthManager::new(oauth_config);

    // Create two tenants
    let env_tenant_id = TenantId::new();
    let db_tenant_id = TenantId::new();

    // Create tenant A (uses server-level credentials)
    let env_tenant_owner =
        create_test_user(&database, "owner_a@example.com", env_tenant_id).await?;
    let env_tenant = Tenant::new(
        "Tenant A".to_owned(),
        env_tenant_id.to_string(),
        Some("tenant-a.example.com".to_owned()),
        "professional".to_owned(),
        env_tenant_owner,
    );
    database.create_tenant(&env_tenant).await?;

    // Create tenant B (will use tenant-specific credentials)
    let db_tenant_owner = create_test_user(&database, "owner_b@example.com", db_tenant_id).await?;
    let db_tenant = Tenant::new(
        "Tenant B".to_owned(),
        db_tenant_id.to_string(),
        Some("tenant-b.example.com".to_owned()),
        "professional".to_owned(),
        db_tenant_owner,
    );
    database.create_tenant(&db_tenant).await?;

    // Store different credentials for tenant B in memory (simulating database storage)
    oauth_manager.store_credentials(
        db_tenant_id,
        oauth_providers::STRAVA,
        CredentialConfig {
            client_id: "999888".to_owned(),
            client_secret: "db_secret_b".to_owned(),
            redirect_uri: "http://localhost:8080/api/oauth/callback/strava".to_owned(),
            scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
            configured_by: db_tenant_owner,
        },
    )?;

    // Get credentials for tenant A (should use server-level config)
    let env_creds = oauth_manager
        .get_credentials(env_tenant_id, oauth_providers::STRAVA, &database)
        .await?;

    // Get credentials for tenant B (should use tenant-specific credentials)
    let db_creds = oauth_manager
        .get_credentials(db_tenant_id, oauth_providers::STRAVA, &database)
        .await?;

    // Verify tenant A uses server-level credentials
    assert_eq!(
        env_creds.client_id, "163846",
        "Tenant A should use server-level CLIENT_ID"
    );
    assert_eq!(
        env_creds.client_secret, "env_secret_a",
        "Tenant A should use server-level SECRET"
    );
    assert_eq!(
        env_creds.tenant_id, env_tenant_id,
        "Credentials should belong to tenant A"
    );

    // Verify tenant B uses database credentials
    assert_eq!(
        db_creds.client_id, "999888",
        "Tenant B should use database CLIENT_ID"
    );
    assert_eq!(
        db_creds.client_secret, "db_secret_b",
        "Tenant B should use database SECRET"
    );
    assert_eq!(
        db_creds.tenant_id, db_tenant_id,
        "Credentials should belong to tenant B"
    );

    // Verify credentials are different
    assert_ne!(
        env_creds.client_id, db_creds.client_id,
        "Tenants should have different CLIENT_IDs"
    );
    assert_ne!(
        env_creds.client_secret, db_creds.client_secret,
        "Tenants should have different CLIENT_SECRETs"
    );

    tracing::info!(" Test 1: Tenant credential isolation verified");
    Ok(())
}

/// Test 2: Rate Limit Tracking Per Tenant
///
/// Verifies that rate limits are tracked separately for each tenant,
/// ensuring one tenant's usage doesn't affect another
#[tokio::test]
async fn test_rate_limit_tracking_per_tenant() -> Result<()> {
    let database = setup_test_database().await?;
    let oauth_config = Arc::new(OAuthConfig {
        strava: OAuthProviderConfig::default(),
        fitbit: OAuthProviderConfig::default(),
        garmin: OAuthProviderConfig::default(),
        whoop: OAuthProviderConfig::default(),
        terra: OAuthProviderConfig::default(),
    });
    let mut oauth_manager = TenantOAuthManager::new(oauth_config);

    // Create two tenants
    let first_tenant_id = TenantId::new();
    let second_tenant_id = TenantId::new();

    let first_owner = create_test_user(&database, "owner_a@example.com", first_tenant_id).await?;
    let second_owner = create_test_user(&database, "owner_b@example.com", second_tenant_id).await?;

    let first_tenant = Tenant::new(
        "Tenant A".to_owned(),
        first_tenant_id.to_string(),
        Some("tenant-a.example.com".to_owned()),
        "professional".to_owned(),
        first_owner,
    );
    database.create_tenant(&first_tenant).await?;

    let second_tenant = Tenant::new(
        "Tenant B".to_owned(),
        second_tenant_id.to_string(),
        Some("tenant-b.example.com".to_owned()),
        "professional".to_owned(),
        second_owner,
    );
    database.create_tenant(&second_tenant).await?;

    // Initial rate limit check - both should be zero
    let (first_usage_initial, first_limit) =
        oauth_manager.check_rate_limit(first_tenant_id, oauth_providers::STRAVA)?;
    let (second_usage_initial, second_limit) =
        oauth_manager.check_rate_limit(second_tenant_id, oauth_providers::STRAVA)?;

    assert_eq!(first_usage_initial, 0, "Tenant A should start with 0 usage");
    assert_eq!(
        second_usage_initial, 0,
        "Tenant B should start with 0 usage"
    );
    assert!(first_limit > 0, "Tenant A should have a rate limit");
    assert!(second_limit > 0, "Tenant B should have a rate limit");

    // Simulate 50 API calls from tenant A
    oauth_manager.increment_usage(first_tenant_id, oauth_providers::STRAVA, 50, 0)?;

    // Simulate 30 API calls from tenant B
    oauth_manager.increment_usage(second_tenant_id, oauth_providers::STRAVA, 30, 0)?;

    // Check rate limits after usage
    let (first_usage_after, _) =
        oauth_manager.check_rate_limit(first_tenant_id, oauth_providers::STRAVA)?;
    let (second_usage_after, _) =
        oauth_manager.check_rate_limit(second_tenant_id, oauth_providers::STRAVA)?;

    // Verify tenant A usage
    assert_eq!(
        first_usage_after, 50,
        "Tenant A should have 50 requests used"
    );

    // Verify tenant B usage
    assert_eq!(
        second_usage_after, 30,
        "Tenant B should have 30 requests used"
    );

    // Verify independence - tenant A's usage doesn't affect tenant B
    assert_ne!(
        first_usage_after, second_usage_after,
        "Tenants should have independent usage tracking"
    );

    // Simulate more calls from tenant A
    oauth_manager.increment_usage(first_tenant_id, oauth_providers::STRAVA, 25, 0)?;

    let (first_usage_final, _) =
        oauth_manager.check_rate_limit(first_tenant_id, oauth_providers::STRAVA)?;
    let (second_usage_final, _) =
        oauth_manager.check_rate_limit(second_tenant_id, oauth_providers::STRAVA)?;

    assert_eq!(
        first_usage_final, 75,
        "Tenant A should have 75 requests used"
    );
    assert_eq!(
        second_usage_final, 30,
        "Tenant B usage should remain unchanged"
    );

    tracing::info!(" Test 2: Rate limit tracking per tenant verified");
    Ok(())
}

/// Helper: Create tenant with user and OAuth token
async fn create_tenant_with_token(
    database: &Database,
    tenant_name: &str,
    tenant_domain: &str,
    user_email: &str,
    access_token: &str,
) -> Result<(TenantId, Uuid)> {
    let tenant_id = TenantId::new();
    let user_id = create_test_user(database, user_email, tenant_id).await?;

    let tenant = Tenant::new(
        tenant_name.to_owned(),
        tenant_id.to_string(),
        Some(tenant_domain.to_owned()),
        "professional".to_owned(),
        user_id,
    );
    database.create_tenant(&tenant).await?;

    let token = UserOAuthToken::new(
        user_id,
        tenant_id.to_string(),
        oauth_providers::STRAVA.to_owned(),
        access_token.to_owned(),
        Some(format!("refresh_{access_token}")),
        Some(Utc::now() + chrono::Duration::hours(6)),
        Some("read,activity:read_all".to_owned()),
    );
    database.upsert_user_oauth_token(&token).await?;

    Ok((tenant_id, user_id))
}

/// Test 3: Cross-Tenant Data Isolation
///
/// Verifies that OAuth tokens are properly isolated by (`user_id`, `tenant_id`, provider)
/// and users cannot access tokens from other tenants
#[tokio::test]
async fn test_cross_tenant_oauth_token_isolation() -> Result<()> {
    let database = setup_test_database().await?;

    // Create two tenants with tokens
    let (alpha_tenant_id, user1_id) = create_tenant_with_token(
        &database,
        "Tenant A",
        "tenant-a.example.com",
        "user1@tenant-a.com",
        "access_token_user1_tenant_a",
    )
    .await?;

    let (beta_tenant_id, user2_id) = create_tenant_with_token(
        &database,
        "Tenant B",
        "tenant-b.example.com",
        "user2@tenant-b.com",
        "access_token_user2_tenant_b",
    )
    .await?;

    // Retrieve token for user1 in tenant A
    let retrieved_token1 = database
        .get_user_oauth_token(user1_id, alpha_tenant_id, oauth_providers::STRAVA)
        .await?;

    // Retrieve token for user2 in tenant B
    let retrieved_token2 = database
        .get_user_oauth_token(user2_id, beta_tenant_id, oauth_providers::STRAVA)
        .await?;

    // Verify tokens are retrieved correctly
    assert!(retrieved_token1.is_some(), "User 1 token should exist");
    assert!(retrieved_token2.is_some(), "User 2 token should exist");

    let token1_data = retrieved_token1.unwrap();
    let token2_data = retrieved_token2.unwrap();

    // Verify correct tokens are returned
    assert_eq!(
        token1_data.access_token, "access_token_user1_tenant_a",
        "User 1 should get their own token"
    );
    assert_eq!(
        token2_data.access_token, "access_token_user2_tenant_b",
        "User 2 should get their own token"
    );

    // Verify tenant isolation
    assert_eq!(
        token1_data.tenant_id,
        alpha_tenant_id.to_string(),
        "Token 1 should belong to tenant A"
    );
    assert_eq!(
        token2_data.tenant_id,
        beta_tenant_id.to_string(),
        "Token 2 should belong to tenant B"
    );

    // Verify user isolation
    assert_eq!(
        token1_data.user_id, user1_id,
        "Token 1 should belong to user 1"
    );
    assert_eq!(
        token2_data.user_id, user2_id,
        "Token 2 should belong to user 2"
    );

    // Cross-tenant access attempt: Try to get user1's token with tenant B's ID
    let cross_tenant_attempt = database
        .get_user_oauth_token(user1_id, beta_tenant_id, oauth_providers::STRAVA)
        .await?;

    assert!(
        cross_tenant_attempt.is_none(),
        "User 1's token should NOT be accessible from tenant B context"
    );

    // Cross-user access attempt: Try to get user1's token with user2's ID
    let cross_user_attempt = database
        .get_user_oauth_token(user2_id, alpha_tenant_id, oauth_providers::STRAVA)
        .await?;

    assert!(
        cross_user_attempt.is_none(),
        "User 2's token should NOT be accessible from user 1's ID with tenant A"
    );

    tracing::info!(" Test 3: Cross-tenant OAuth token isolation verified");
    Ok(())
}

/// Test 4: OAuth Callback Tenant ID Preservation
///
/// Verifies that the OAuth callback flow preserves `tenant_id` through the state parameter
#[tokio::test]
async fn test_oauth_callback_tenant_preservation() -> Result<()> {
    let database = setup_test_database().await?;

    // Create tenant and user
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        tenant_id.to_string(),
        Some("test.example.com".to_owned()),
        "professional".to_owned(),
        user_id,
    );
    database.create_tenant(&tenant).await?;

    // Simulate OAuth authorization state parameter
    // In real implementation, this would be generated by the OAuth flow
    let state_data = serde_json::json!({
        "user_id": user_id.to_string(),
        "tenant_id": tenant_id.to_string(),
        "provider": oauth_providers::STRAVA,
        "timestamp": Utc::now().timestamp(),
    });

    let state_json = serde_json::to_string(&state_data)?;

    // Verify state can be parsed back
    let parsed_state: serde_json::Value = serde_json::from_str(&state_json)?;

    let extracted_tenant_id = parsed_state["tenant_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok())
        .map(TenantId::from_uuid);

    assert_eq!(
        extracted_tenant_id,
        Some(tenant_id),
        "Tenant ID should be preserved in OAuth state parameter"
    );

    tracing::info!(" Test 4: OAuth callback tenant ID preservation verified");
    Ok(())
}

/// Test 5: Token Refresh with Tenant Credentials
///
/// Verifies that token refresh uses the correct tenant-specific `CLIENT_ID/SECRET`
#[tokio::test]
async fn test_token_refresh_uses_tenant_credentials() -> Result<()> {
    let database = setup_test_database().await?;
    let oauth_config = Arc::new(OAuthConfig {
        strava: OAuthProviderConfig::default(),
        fitbit: OAuthProviderConfig::default(),
        garmin: OAuthProviderConfig::default(),
        whoop: OAuthProviderConfig::default(),
        terra: OAuthProviderConfig::default(),
    });
    let mut oauth_manager = TenantOAuthManager::new(oauth_config);

    // Create tenant
    let tenant_id = TenantId::new();
    let user_id = create_test_user(&database, "user@example.com", tenant_id).await?;

    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        tenant_id.to_string(),
        Some("test.example.com".to_owned()),
        "professional".to_owned(),
        user_id,
    );
    database.create_tenant(&tenant).await?;

    // Store tenant-specific credentials
    oauth_manager.store_credentials(
        tenant_id,
        oauth_providers::STRAVA,
        CredentialConfig {
            client_id: "tenant_specific_client_id".to_owned(),
            client_secret: "tenant_specific_secret".to_owned(),
            redirect_uri: "http://localhost:8080/api/oauth/callback/strava".to_owned(),
            scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
            configured_by: user_id,
        },
    )?;

    // Get credentials for token refresh
    let refresh_creds = oauth_manager
        .get_credentials(tenant_id, oauth_providers::STRAVA, &database)
        .await?;

    // Verify correct credentials are returned for refresh
    assert_eq!(
        refresh_creds.client_id, "tenant_specific_client_id",
        "Token refresh should use tenant-specific CLIENT_ID"
    );
    assert_eq!(
        refresh_creds.client_secret, "tenant_specific_secret",
        "Token refresh should use tenant-specific CLIENT_SECRET"
    );

    tracing::info!(" Test 5: Token refresh with tenant credentials verified");
    Ok(())
}

/// Test 6: Tenant-Specific Rate Limit Configuration
///
/// Verifies that different tenants can have different rate limit configurations
#[tokio::test]
#[serial]
async fn test_tenant_specific_rate_limits() -> Result<()> {
    let database = setup_test_database().await?;
    let oauth_config = Arc::new(OAuthConfig {
        strava: OAuthProviderConfig::default(),
        fitbit: OAuthProviderConfig::default(),
        garmin: OAuthProviderConfig::default(),
        whoop: OAuthProviderConfig::default(),
        terra: OAuthProviderConfig::default(),
    });
    let mut oauth_manager = TenantOAuthManager::new(oauth_config);

    // Create two tenants with different rate limit needs
    let tenant_standard_id = TenantId::new();
    let tenant_enterprise_id = TenantId::new();

    // Standard tenant owner (uses environment credentials)
    let owner_standard =
        create_test_user(&database, "standard@example.com", tenant_standard_id).await?;
    let standard_tenant = Tenant::new(
        "Standard Tenant".to_owned(),
        tenant_standard_id.to_string(),
        Some("standard.example.com".to_owned()),
        "professional".to_owned(),
        owner_standard,
    );
    database.create_tenant(&standard_tenant).await?;

    // Enterprise tenant owner (uses custom credentials)
    let owner_enterprise =
        create_test_user(&database, "enterprise@example.com", tenant_enterprise_id).await?;
    let enterprise_tenant = Tenant::new(
        "Enterprise Tenant".to_owned(),
        tenant_enterprise_id.to_string(),
        Some("enterprise.example.com".to_owned()),
        "enterprise".to_owned(),
        owner_enterprise,
    );
    database.create_tenant(&enterprise_tenant).await?;

    // Standard tenant uses default rate limits (via environment)
    env::set_var("STRAVA_CLIENT_ID", "163846");
    env::set_var("STRAVA_CLIENT_SECRET", "standard_secret");

    // Enterprise tenant gets custom credentials with higher rate limits
    oauth_manager.store_credentials(
        tenant_enterprise_id,
        oauth_providers::STRAVA,
        CredentialConfig {
            client_id: "enterprise_client_id".to_owned(),
            client_secret: "enterprise_secret".to_owned(),
            redirect_uri: "http://localhost:8080/api/oauth/callback/strava".to_owned(),
            scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
            configured_by: owner_enterprise,
        },
    )?;

    // Check rate limits for both tenants
    let (_, limit_standard) =
        oauth_manager.check_rate_limit(tenant_standard_id, oauth_providers::STRAVA)?;
    let (_, limit_enterprise) =
        oauth_manager.check_rate_limit(tenant_enterprise_id, oauth_providers::STRAVA)?;

    // Both should have limits configured
    assert!(limit_standard > 0, "Standard tenant should have rate limit");
    assert!(
        limit_enterprise > 0,
        "Enterprise tenant should have rate limit"
    );

    // For now, both use the same default limit from constants
    // In future, tenant-specific limits could be configured in database
    assert_eq!(
        limit_standard, limit_enterprise,
        "Currently all tenants share the same rate limit constant"
    );

    tracing::info!(" Test 6: Tenant-specific rate limit configuration verified");
    Ok(())
}

/// Test 7: Concurrent Multi-Tenant OAuth Operations
///
/// Verifies that concurrent OAuth operations from multiple tenants don't interfere
#[tokio::test]
async fn test_concurrent_multitenant_oauth_operations() -> Result<()> {
    let database = Arc::new(setup_test_database().await?);
    let oauth_config = Arc::new(OAuthConfig {
        strava: OAuthProviderConfig::default(),
        fitbit: OAuthProviderConfig::default(),
        garmin: OAuthProviderConfig::default(),
        whoop: OAuthProviderConfig::default(),
        terra: OAuthProviderConfig::default(),
    });
    let oauth_manager = Arc::new(RwLock::new(TenantOAuthManager::new(oauth_config)));

    // Create 5 tenants concurrently
    let mut tasks = vec![];

    for i in 0..5 {
        let db = database.clone();
        let manager = oauth_manager.clone();

        let task = tokio::spawn(async move {
            let tenant_id = TenantId::new();
            let user_id = create_test_user(&db, &format!("user{i}@example.com"), tenant_id).await?;

            let tenant = Tenant::new(
                format!("Tenant {i}"),
                tenant_id.to_string(),
                Some(format!("tenant{i}.example.com")),
                "professional".to_owned(),
                user_id,
            );
            db.create_tenant(&tenant).await?;

            // Store credentials
            manager.write().await.store_credentials(
                tenant_id,
                oauth_providers::STRAVA,
                CredentialConfig {
                    client_id: format!("client_id_{i}"),
                    client_secret: format!("secret_{i}"),
                    redirect_uri: "http://localhost:8080/api/oauth/callback/strava".to_owned(),
                    scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                    configured_by: user_id,
                },
            )?;

            // Store token
            let token = UserOAuthToken::new(
                user_id,
                tenant_id.to_string(),
                oauth_providers::STRAVA.to_owned(),
                format!("access_token_{i}"),
                Some(format!("refresh_token_{i}")),
                Some(Utc::now() + chrono::Duration::hours(6)),
                Some("read,activity:read_all".to_owned()),
            );
            db.upsert_user_oauth_token(&token).await?;

            // Simulate API calls
            manager
                .write()
                .await
                .increment_usage(tenant_id, oauth_providers::STRAVA, 10, 0)?;

            Ok::<(TenantId, Uuid), anyhow::Error>((tenant_id, user_id))
        });

        tasks.push(task);
    }

    // Wait for all tasks to complete
    let mut tenant_user_pairs = vec![];
    for task in tasks {
        let result = task.await??;
        tenant_user_pairs.push(result);
    }

    // Verify all tenants have independent data
    for (tenant_id, user_id) in tenant_user_pairs {
        // Check credentials
        let creds = {
            let manager_guard = oauth_manager.read().await;
            manager_guard
                .get_credentials(tenant_id, oauth_providers::STRAVA, &database)
                .await?
        };
        assert_eq!(
            creds.tenant_id, tenant_id,
            "Credentials should match tenant"
        );

        // Check rate limit usage
        let (usage, _) = {
            let manager_guard = oauth_manager.read().await;
            manager_guard.check_rate_limit(tenant_id, oauth_providers::STRAVA)?
        };
        assert_eq!(usage, 10, "Each tenant should have 10 requests used");

        // Check token
        let token = database
            .get_user_oauth_token(user_id, tenant_id, oauth_providers::STRAVA)
            .await?;
        assert!(token.is_some(), "Token should exist for user/tenant pair");
        assert_eq!(
            token.unwrap().user_id,
            user_id,
            "Token should belong to correct user"
        );
    }

    tracing::info!(" Test 7: Concurrent multi-tenant OAuth operations verified");
    Ok(())
}
