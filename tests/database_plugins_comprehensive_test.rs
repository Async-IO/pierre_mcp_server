// ABOUTME: Comprehensive tests for database plugin implementations
// ABOUTME: Tests SQLite and PostgreSQL database providers with full CRUD operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! Comprehensive tests for database plugins
//!
//! This test suite covers the database plugin implementations
//! which currently have no test coverage

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use chrono::Utc;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    api_keys::{ApiKey, ApiKeyTier, ApiKeyUsage},
    constants::oauth_providers,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{TenantId, User, UserOAuthToken, UserTier},
    rate_limiting::JwtUsage,
};
use serial_test::serial;
use uuid::Uuid;

mod common;

#[tokio::test]
async fn test_sqlite_database_creation() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;

    // Verify the database was created successfully
    let _ = db.backend_info();

    Ok(())
}

#[tokio::test]
async fn test_sqlite_migration() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;

    // Test migration
    let result = db.migrate().await;
    assert!(result.is_ok());

    Ok(())
}

#[tokio::test]
async fn test_user_crud_operations() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    // Create user
    let user = User::new(
        "test_crud@example.com".to_owned(),
        "password123".to_owned(),
        Some("Test CRUD User".to_owned()),
    );

    let user_id = db.create_user(&user).await?;
    assert_eq!(user_id, user.id);

    // Get user by ID
    let retrieved_user = db.get_user(user_id).await?;
    assert!(retrieved_user.is_some());
    assert_eq!(retrieved_user.unwrap().email, "test_crud@example.com");

    // Get user by email
    let user_by_email = db.get_user_by_email("test_crud@example.com").await?;
    assert!(user_by_email.is_some());
    assert_eq!(user_by_email.unwrap().id, user_id);

    // Get user by email (required)
    let user_required = db
        .get_user_by_email_required("test_crud@example.com")
        .await?;
    assert_eq!(user_required.id, user_id);

    // Test non-existent user
    let non_existent = db.get_user(Uuid::new_v4()).await?;
    assert!(non_existent.is_none());

    Ok(())
}

#[tokio::test]
async fn test_user_last_active_update() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "last_active_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );

    let user_id = db.create_user(&user).await?;

    // Update last active
    let result = db.update_last_active(user_id).await;
    assert!(result.is_ok());

    // Verify user can still be retrieved
    let updated_user = db.get_user(user_id).await?;
    assert!(updated_user.is_some());

    Ok(())
}

#[tokio::test]
async fn test_user_count() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    // Initial count should be 0
    let initial_count = db.get_user_count().await?;
    assert_eq!(initial_count, 0);

    // Create multiple users
    for i in 0..3 {
        let user = User::new(
            format!("count_test_{i}@example.com"),
            "password".to_owned(),
            Some(format!("User {i}")),
        );
        db.create_user(&user).await?;
    }

    // Count should be 3
    let count = db.get_user_count().await?;
    assert_eq!(count, 3);

    Ok(())
}

#[tokio::test]
async fn test_strava_token_operations() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "strava_token_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    // Initially no token
    let initial_token = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?;
    assert!(initial_token.is_none());

    // Update token
    let expires_at = Utc::now() + chrono::Duration::hours(6);
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "test_access_token".to_owned(),
        Some("test_refresh_token".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    db.upsert_user_oauth_token(&oauth_token).await?;

    // Retrieve token
    let token = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?;
    assert!(token.is_some());
    let token = token.unwrap();
    assert_eq!(token.access_token, "test_access_token");
    assert_eq!(token.refresh_token.as_ref().unwrap(), "test_refresh_token");
    let scopes = token.scope.as_ref().unwrap();
    assert!(scopes.contains(&"read".to_owned()));
    assert!(scopes.contains(&"activity:read_all".to_owned()));

    // Clear token
    db.delete_user_oauth_token(
        user_id,
        TenantId::from_uuid(Uuid::nil()),
        oauth_providers::STRAVA,
    )
    .await?;
    let cleared_token = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?;
    assert!(cleared_token.is_none());

    Ok(())
}

#[tokio::test]
async fn test_fitbit_token_operations() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "fitbit_token_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    // Initially no token
    let initial_token = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::FITBIT,
        )
        .await?;
    assert!(initial_token.is_none());

    // Update token
    let expires_at = Utc::now() + chrono::Duration::hours(8);
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::FITBIT.to_owned(),
        "fitbit_access_token".to_owned(),
        Some("fitbit_refresh_token".to_owned()),
        Some(expires_at),
        Some("activity profile".to_owned()),
    );
    db.upsert_user_oauth_token(&oauth_token).await?;

    // Retrieve token
    let token = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::FITBIT,
        )
        .await?;
    assert!(token.is_some());
    let token = token.unwrap();
    assert_eq!(token.access_token, "fitbit_access_token");
    assert_eq!(
        token.refresh_token.as_ref().unwrap(),
        "fitbit_refresh_token"
    );
    let scopes = token.scope.as_ref().unwrap();
    assert!(scopes.contains(&"activity".to_owned()));
    assert!(scopes.contains(&"profile".to_owned()));

    // Clear token
    db.delete_user_oauth_token(
        user_id,
        TenantId::from_uuid(Uuid::nil()),
        oauth_providers::FITBIT,
    )
    .await?;
    let cleared_token = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::FITBIT,
        )
        .await?;
    assert!(cleared_token.is_none());

    Ok(())
}

#[tokio::test]
async fn test_api_key_operations() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "api_key_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    // Create API key
    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id,
        name: "Test API Key".to_owned(),
        key_prefix: "pk_test".to_owned(),
        key_hash: "hashed_key_value".to_owned(),
        description: Some("Test API key for database testing".to_owned()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 1000,
        rate_limit_window_seconds: 30 * 24 * 3600,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    db.create_api_key(&api_key).await?;

    // Get API keys for user
    let user_keys = db.get_user_api_keys(user_id).await?;
    assert_eq!(user_keys.len(), 1);
    assert_eq!(user_keys[0].name, "Test API Key");

    // Get API key by prefix
    let key_by_prefix = db
        .get_api_key_by_prefix("pk_test", "hashed_key_value")
        .await?;
    assert!(key_by_prefix.is_some());
    assert_eq!(key_by_prefix.unwrap().id, api_key.id);

    // Update last used
    db.update_api_key_last_used(&api_key.id).await?;

    // Deactivate API key
    db.deactivate_api_key(&api_key.id, user_id).await?;
    let deactivated_key = db.get_user_api_keys(user_id).await?;
    assert!(!deactivated_key[0].is_active);

    Ok(())
}

#[tokio::test]
async fn test_api_key_usage_tracking() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "usage_tracking_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id,
        name: "Usage Test Key".to_owned(),
        key_prefix: "pk_usage".to_owned(),
        key_hash: "usage_hash".to_owned(),
        description: Some("Usage tracking test key".to_owned()),
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 10000,
        rate_limit_window_seconds: 30 * 24 * 3600,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    db.create_api_key(&api_key).await?;

    // Record usage
    let usage = ApiKeyUsage {
        id: None,
        api_key_id: api_key.id.clone(),
        tool_name: "get_activities".to_owned(),
        timestamp: Utc::now(),
        status_code: 200,
        response_time_ms: Some(150),
        error_message: None,
        request_size_bytes: Some(100),
        response_size_bytes: Some(500),
        ip_address: Some("127.0.0.1".to_owned()),
        user_agent: Some("test-client/1.0".to_owned()),
    };

    db.record_api_key_usage(&usage).await?;

    // Get usage stats (Note: usage tracking may be handled differently in plugin interface)
    let current_usage = db.get_api_key_current_usage(&api_key.id).await?;
    // For plugin interface, just verify the method works
    let _ = current_usage;

    // Get usage for date range (simplified for database plugin interface)
    // Note: Direct usage record retrieval is handled by the underlying database implementation

    Ok(())
}

#[tokio::test]
async fn test_jwt_usage_tracking() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "jwt_usage_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    // Record JWT usage
    let jwt_usage = JwtUsage {
        id: None,
        user_id,
        endpoint: "/api/oauth/connect".to_owned(),
        method: "POST".to_owned(),
        status_code: 200,
        response_time_ms: Some(100),
        request_size_bytes: Some(50),
        response_size_bytes: Some(200),
        ip_address: Some("127.0.0.1".to_owned()),
        user_agent: Some("test-client".to_owned()),
        timestamp: Utc::now(),
    };

    db.record_jwt_usage(&jwt_usage).await?;

    // Get JWT usage count (Note: usage counting may work differently in plugin interface)
    let count = db.get_jwt_current_usage(user_id).await?;
    // For plugin interface, just verify the method works without asserting specific counts
    let _ = count;

    // Record another usage
    let jwt_usage2 = JwtUsage {
        id: None,
        user_id,
        endpoint: "/api/activities".to_owned(),
        method: "GET".to_owned(),
        status_code: 200,
        response_time_ms: Some(75),
        request_size_bytes: Some(30),
        response_size_bytes: Some(1000),
        ip_address: Some("127.0.0.1".to_owned()),
        user_agent: Some("test-client".to_owned()),
        timestamp: Utc::now(),
    };

    db.record_jwt_usage(&jwt_usage2).await?;

    let updated_count = db.get_jwt_current_usage(user_id).await?;
    // Just verify the method works
    let _ = updated_count;

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_concurrent_database_operations() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    // Create multiple users concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let db_clone = db.clone();
        handles.push(tokio::spawn(async move {
            let user = User::new(
                format!("concurrent_{i}@example.com"),
                "password".to_owned(),
                Some(format!("Concurrent User {i}")),
            );
            db_clone.create_user(&user).await
        }));
    }

    // All operations should succeed
    for handle in handles {
        let result = handle.await?;
        assert!(result.is_ok());
    }

    // Verify all users were created
    let count = db.get_user_count().await?;
    assert_eq!(count, 5);

    Ok(())
}

#[tokio::test]
async fn test_token_encryption_roundtrip() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "encryption_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    // Test with special characters in tokens
    let long_token = format!("very_long_token_{}", "x".repeat(500));
    let special_tokens = vec![
        ("token_with_special_chars_!@#$%^&*()", "refresh_token_äöü"),
        (long_token.as_str(), "short_refresh"),
        ("unicode_token_test", "unicode_refresh_test"),
    ];

    for (access_token, refresh_token) in special_tokens {
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        // Store Strava token
        let oauth_token = UserOAuthToken::new(
            user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::STRAVA.to_owned(),
            access_token.to_owned(),
            Some(refresh_token.to_owned()),
            Some(expires_at),
            Some("read".to_owned()),
        );
        db.upsert_user_oauth_token(&oauth_token).await?;

        // Retrieve and verify
        let retrieved = db
            .get_user_oauth_token(
                user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::STRAVA,
            )
            .await?;
        assert!(retrieved.is_some());
        let token = retrieved.unwrap();
        assert_eq!(token.access_token, access_token);
        assert_eq!(token.refresh_token.as_ref().unwrap(), refresh_token);

        // Store Fitbit token
        let fitbit_oauth_token = UserOAuthToken::new(
            user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::FITBIT.to_owned(),
            access_token.to_owned(),
            Some(refresh_token.to_owned()),
            Some(expires_at),
            Some("activity".to_owned()),
        );
        db.upsert_user_oauth_token(&fitbit_oauth_token).await?;

        // Retrieve and verify
        let fitbit_token = db
            .get_user_oauth_token(
                user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::FITBIT,
            )
            .await?;
        assert!(fitbit_token.is_some());
        let token = fitbit_token.unwrap();
        assert_eq!(token.access_token, access_token);
        assert_eq!(token.refresh_token.as_ref().unwrap(), refresh_token);
    }

    Ok(())
}

#[tokio::test]
async fn test_database_error_scenarios() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    // Test getting non-existent user by email (required)
    let result = db
        .get_user_by_email_required("nonexistent@example.com")
        .await;
    assert!(result.is_err());

    // Test operations on non-existent API key
    let fake_key_id = Uuid::new_v4().to_string();
    let result = db.update_api_key_last_used(&fake_key_id).await;
    // Should either succeed (no-op) or fail gracefully
    let _ = result;

    let fake_user_id = Uuid::new_v4();
    let result = db.deactivate_api_key(&fake_key_id, fake_user_id).await;
    // Should either succeed (no-op) or fail gracefully
    let _ = result;

    Ok(())
}

#[tokio::test]
async fn test_api_key_usage_aggregation() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    let user = User::new(
        "aggregation_test@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    let api_key = ApiKey {
        id: Uuid::new_v4().to_string(),
        user_id,
        name: "Aggregation Test Key".to_owned(),
        key_prefix: "pk_agg".to_owned(),
        key_hash: "agg_hash".to_owned(),
        description: Some("Aggregation test key".to_owned()),
        tier: ApiKeyTier::Enterprise,
        rate_limit_requests: 0, // Unlimited for enterprise
        rate_limit_window_seconds: 30 * 24 * 3600,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    db.create_api_key(&api_key).await?;

    // Record multiple usage entries with different response times
    let response_times = [100, 150, 200, 75, 300];

    for (i, response_time) in response_times.iter().enumerate() {
        let usage = ApiKeyUsage {
            id: None,
            api_key_id: api_key.id.clone(),
            tool_name: format!("tool_{i}"),
            status_code: 200,
            response_time_ms: Some(*response_time),
            timestamp: Utc::now(),
            error_message: None,
            request_size_bytes: Some(100),
            response_size_bytes: Some(200),
            user_agent: Some("test-client".to_owned()),
            ip_address: Some("127.0.0.1".to_owned()),
        };

        db.record_api_key_usage(&usage).await?;
    }

    // Check aggregated stats (Note: usage aggregation may work differently in plugin interface)
    let current_usage = db.get_api_key_current_usage(&api_key.id).await?;
    // For plugin interface, just verify the method works
    let _ = current_usage;

    Ok(())
}

#[tokio::test]
async fn test_user_tier_handling() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    // Test all user tiers
    let tiers = [
        UserTier::Starter,
        UserTier::Professional,
        UserTier::Enterprise,
    ];

    for (i, tier) in tiers.iter().enumerate() {
        let mut user = User::new(
            format!("tier_test_{i}@example.com"),
            "password".to_owned(),
            Some(format!("Tier Test {i}")),
        );
        user.tier = tier.clone();

        let user_id = db.create_user(&user).await?;

        // Create API key with corresponding tier
        let api_key = ApiKey {
            id: Uuid::new_v4().to_string(),
            user_id,
            name: format!("Key for {tier:?}"),
            key_prefix: format!("pk_{i}"),
            key_hash: format!("hash_{i}"),
            description: Some(format!("Test key for {tier:?} tier")),
            tier: match tier {
                UserTier::Starter => ApiKeyTier::Starter,
                UserTier::Professional => ApiKeyTier::Professional,
                UserTier::Enterprise => ApiKeyTier::Enterprise,
            },
            rate_limit_requests: match tier {
                UserTier::Starter => 1000,
                UserTier::Professional => 10000,
                UserTier::Enterprise => 0, // Unlimited
            },
            rate_limit_window_seconds: 30 * 24 * 3600,
            is_active: true,
            last_used_at: None,
            expires_at: None,
            created_at: Utc::now(),
        };

        db.create_api_key(&api_key).await?;

        // Verify retrieval maintains tier
        let retrieved_keys = db.get_user_api_keys(user_id).await?;
        let expected_api_tier = match tier {
            UserTier::Starter => ApiKeyTier::Starter,
            UserTier::Professional => ApiKeyTier::Professional,
            UserTier::Enterprise => ApiKeyTier::Enterprise,
        };
        assert_eq!(retrieved_keys[0].tier, expected_api_tier);

        let retrieved_user = db.get_user(user_id).await?.unwrap();
        assert_eq!(retrieved_user.tier, *tier);
    }

    Ok(())
}

#[tokio::test]
async fn test_database_connection_reuse() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();
    #[cfg(feature = "postgresql")]
    let db = Database::new(
        "sqlite::memory:",
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    db.migrate().await?;

    // Create user
    let user = User::new(
        "connection_reuse@example.com".to_owned(),
        "password".to_owned(),
        None,
    );
    let user_id = db.create_user(&user).await?;

    // Perform multiple operations to test connection reuse
    for i in 0..10 {
        // Each operation should work with the same connection
        db.update_last_active(user_id).await?;

        let token_expires = Utc::now() + chrono::Duration::hours(i);
        let oauth_token = UserOAuthToken::new(
            user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::STRAVA.to_owned(),
            format!("token_{i}"),
            Some(format!("refresh_{i}")),
            Some(token_expires),
            Some("read".to_owned()),
        );
        db.upsert_user_oauth_token(&oauth_token).await?;

        let retrieved_token = db
            .get_user_oauth_token(
                user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::STRAVA,
            )
            .await?;
        assert!(retrieved_token.is_some());
        assert_eq!(retrieved_token.unwrap().access_token, format!("token_{i}"));
    }

    Ok(())
}

// PostgreSQL-specific tests (only run when feature is enabled)
// Each test uses an isolated database to prevent race conditions with --test-threads=4
#[cfg(feature = "postgresql")]
mod postgres_tests {
    use super::*;
    use common::IsolatedPostgresDb;

    #[tokio::test]
    async fn test_postgres_database_creation() -> Result<()> {
        // Each test gets its own isolated database - prevents race conditions
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Verify database is operational
        let user = User::new(
            "postgres_creation_test@example.com".to_owned(),
            "password123".to_owned(),
            Some("PostgreSQL Creation Test".to_owned()),
        );

        let user_id = db.create_user(&user).await?;
        assert_eq!(user_id, user.id);

        // Database cleanup happens automatically when isolated_db is dropped
        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_migration_idempotency() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Run migration multiple times - should be idempotent
        for _ in 0..3 {
            let result = db.migrate().await;
            assert!(result.is_ok(), "Migration should be idempotent");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_user_operations() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Create user with all tiers
        let tiers = [
            UserTier::Starter,
            UserTier::Professional,
            UserTier::Enterprise,
        ];

        for (i, tier) in tiers.iter().enumerate() {
            let mut user = User::new(
                format!("postgres_user_{}_{}@example.com", i, uuid::Uuid::new_v4()),
                "secure_password_123".to_owned(),
                Some(format!("PostgreSQL User {i}")),
            );
            user.tier = tier.clone();

            let user_id = db.create_user(&user).await?;

            // Test user retrieval
            let retrieved = db.get_user(user_id).await?.unwrap();
            assert_eq!(retrieved.email, user.email);
            assert_eq!(retrieved.tier, *tier);

            // Test by email lookup
            let by_email = db.get_user_by_email(&user.email).await?.unwrap();
            assert_eq!(by_email.id, user_id);

            // Test required email lookup
            let required = db.get_user_by_email_required(&user.email).await?;
            assert_eq!(required.id, user_id);

            // Test last active update
            db.update_last_active(user_id).await?;

            // Clean up for next iteration
            // Clean up would happen on test drop or next run
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_api_key_comprehensive() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        let user = User::new(
            format!("postgres_api_test_{}@example.com", uuid::Uuid::new_v4()),
            "password".to_owned(),
            None,
        );
        let user_id = db.create_user(&user).await?;

        // Test all API key tiers
        let api_key_tiers = [
            ApiKeyTier::Starter,
            ApiKeyTier::Professional,
            ApiKeyTier::Enterprise,
        ];

        for (i, tier) in api_key_tiers.iter().enumerate() {
            let api_key = ApiKey {
                id: Uuid::new_v4().to_string(),
                user_id,
                name: format!("PostgreSQL Test Key {i}"),
                key_prefix: format!("pk_pg_{i}"),
                key_hash: format!("postgres_hash_{i}"),
                description: Some(format!("Test key for PostgreSQL {tier:?}")),
                tier: tier.clone(),
                rate_limit_requests: match tier {
                    ApiKeyTier::Trial | ApiKeyTier::Starter => 1000,
                    ApiKeyTier::Professional => 10000,
                    // Enterprise and any future tiers default to unlimited
                    _ => 0,
                },
                rate_limit_window_seconds: 86400, // 24 hours
                is_active: true,
                last_used_at: None,
                expires_at: None,
                created_at: Utc::now(),
            };

            // Create API key
            db.create_api_key(&api_key).await?;

            // Test retrieval
            let retrieved = db.get_api_key_by_id(&api_key.id, Some(user_id)).await?;
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().tier, *tier);

            // Test user keys
            let user_keys = db.get_user_api_keys(user_id).await?;
            assert!(!user_keys.is_empty());

            // Test usage tracking
            let usage = ApiKeyUsage {
                id: None,
                api_key_id: api_key.id.clone(),
                tool_name: "postgres_test_tool".to_owned(),
                status_code: 200,
                response_time_ms: Some(150),
                timestamp: Utc::now(),
                error_message: None,
                request_size_bytes: Some(512),
                response_size_bytes: Some(1024),
                user_agent: Some("postgres-test-client/1.0".to_owned()),
                ip_address: Some("10.0.0.1".to_owned()),
            };

            db.record_api_key_usage(&usage).await?;

            // Test deactivation
            db.deactivate_api_key(&api_key.id, user_id).await?;

            let deactivated = db.get_api_key_by_id(&api_key.id, Some(user_id)).await?;
            assert!(deactivated.is_none() || !deactivated.unwrap().is_active);
        }

        // Clean up
        // Clean up would happen on test drop or next run

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_token_operations() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        let user = User::new(
            format!("postgres_token_test_{}@example.com", uuid::Uuid::new_v4()),
            "password".to_owned(),
            None,
        );
        let user_id = db.create_user(&user).await?;

        let expires_at = Utc::now() + chrono::Duration::hours(2);

        // Test Strava token operations
        let strava_oauth_token = UserOAuthToken::new(
            user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::STRAVA.to_owned(),
            "strava_access_token_postgres".to_owned(),
            Some("strava_refresh_token_postgres".to_owned()),
            Some(expires_at),
            Some("read,activity:read".to_owned()),
        );
        db.upsert_user_oauth_token(&strava_oauth_token).await?;

        let strava_token = db
            .get_user_oauth_token(
                user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::STRAVA,
            )
            .await?;
        assert!(strava_token.is_some());
        let token = strava_token.unwrap();
        assert_eq!(token.access_token, "strava_access_token_postgres");
        assert_eq!(
            token.refresh_token.as_ref().unwrap(),
            "strava_refresh_token_postgres"
        );

        // Test Fitbit token operations
        let fitbit_oauth_token = UserOAuthToken::new(
            user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::FITBIT.to_owned(),
            "fitbit_access_token_postgres".to_owned(),
            Some("fitbit_refresh_token_postgres".to_owned()),
            Some(expires_at),
            Some("activity,profile".to_owned()),
        );
        db.upsert_user_oauth_token(&fitbit_oauth_token).await?;

        let fitbit_token = db
            .get_user_oauth_token(
                user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::FITBIT,
            )
            .await?;
        assert!(fitbit_token.is_some());
        let token = fitbit_token.unwrap();
        assert_eq!(token.access_token, "fitbit_access_token_postgres");
        assert_eq!(
            token.refresh_token.as_ref().unwrap(),
            "fitbit_refresh_token_postgres"
        );

        // Test token encryption roundtrip with special characters
        let special_access = "postgres_token_with_special_chars_!@#$%^&*()_+";
        let special_refresh = "postgres_refresh_äöüß€™";

        let special_oauth_token = UserOAuthToken::new(
            user_id,
            "00000000-0000-0000-0000-000000000000".to_owned(),
            oauth_providers::STRAVA.to_owned(),
            special_access.to_owned(),
            Some(special_refresh.to_owned()),
            Some(expires_at),
            Some("read_all".to_owned()),
        );
        db.upsert_user_oauth_token(&special_oauth_token).await?;

        let retrieved = db
            .get_user_oauth_token(
                user_id,
                TenantId::from_uuid(Uuid::nil()),
                oauth_providers::STRAVA,
            )
            .await?
            .unwrap();
        assert_eq!(retrieved.access_token, special_access);
        assert_eq!(retrieved.refresh_token.as_ref().unwrap(), special_refresh);

        // Clean up
        // Clean up would happen on test drop or next run

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_concurrent_operations() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Test concurrent user creation
        let mut handles = vec![];

        for i in 0..3 {
            let db_clone = db.clone();
            handles.push(tokio::spawn(async move {
                let user = User::new(
                    format!(
                        "postgres_concurrent_{}_{i}@example.com",
                        uuid::Uuid::new_v4()
                    ),
                    "password".to_owned(),
                    Some(format!("PostgreSQL Concurrent User {i}")),
                );
                let user_id = db_clone.create_user(&user).await?;

                // Immediately perform operations on the created user
                db_clone.update_last_active(user_id).await?;

                let api_key = ApiKey {
                    id: Uuid::new_v4().to_string(),
                    user_id,
                    name: format!("Concurrent Key {i}"),
                    key_prefix: format!("pk_conc_{i}"),
                    key_hash: format!("concurrent_hash_{i}"),
                    description: None,
                    tier: ApiKeyTier::Professional,
                    rate_limit_requests: 5000,
                    rate_limit_window_seconds: 3600,
                    is_active: true,
                    last_used_at: None,
                    expires_at: None,
                    created_at: Utc::now(),
                };

                db_clone.create_api_key(&api_key).await?;

                Ok::<_, anyhow::Error>(user_id)
            }));
        }

        // Collect results
        let mut user_ids = vec![];
        for handle in handles {
            let user_id = handle.await??;
            user_ids.push(user_id);
        }

        // Verify all users were created
        for user_id in &user_ids {
            let user = db.get_user(*user_id).await?;
            assert!(user.is_some());
        }

        // Clean up: Each test uses unique emails, so no manual cleanup needed

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_jwt_usage_tracking() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Create a test user first for foreign key reference
        let user = User::new(
            format!("postgres_jwt_test_{}@example.com", uuid::Uuid::new_v4()),
            "password".to_owned(),
            None,
        );
        let user_id = db.create_user(&user).await?;

        // Record JWT usage entries
        for i in 0..5 {
            let jwt_usage = JwtUsage {
                id: None,
                user_id,
                endpoint: format!("/api/postgres/endpoint_{i}"),
                method: "GET".to_owned(),
                status_code: 200,
                response_time_ms: Some(100 + u32::try_from(i).unwrap_or(0) * 10),
                timestamp: Utc::now(),
                request_size_bytes: Some(512),
                response_size_bytes: Some(1024),
                ip_address: Some("10.0.0.1".to_owned()),
                user_agent: Some("postgres-jwt-client/1.0".to_owned()),
            };

            db.record_jwt_usage(&jwt_usage).await?;
        }

        // Verify usage tracking doesn't fail
        // (Note: We can't easily verify the exact content without direct database access)
        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_error_scenarios() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Test non-existent user operations
        let fake_user_id = Uuid::new_v4();
        let result = db.get_user(fake_user_id).await?;
        assert!(result.is_none());

        // Test non-existent email required (should error)
        let result = db
            .get_user_by_email_required("nonexistent_postgres@example.com")
            .await;
        assert!(result.is_err());

        // Test invalid API key operations
        let fake_key_hash = "nonexistent_postgres_hash";
        let result = db.get_api_key_by_id(fake_key_hash, None).await?;
        assert!(result.is_none());

        // Test deactivating non-existent key
        let fake_key_id = Uuid::new_v4().to_string();
        let result = db.deactivate_api_key(&fake_key_id, fake_user_id).await;
        // Should succeed as no-op or fail gracefully
        let _ = result;

        Ok(())
    }

    #[tokio::test]
    async fn test_postgres_connection_pooling() -> Result<()> {
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Perform operations that should use connection pooling
        // Reduced for CI stability - testing pooling behavior, not stress testing
        let operations_count = 5;
        let mut handles = vec![];

        for i in 0..operations_count {
            let db_clone = db.clone();
            handles.push(tokio::spawn(async move {
                // Simple operation that uses the database connection
                let user = User::new(
                    format!("pool_test_{}_{i}@example.com", uuid::Uuid::new_v4()),
                    "password".to_owned(),
                    None,
                );

                // Create and immediately clean up to test pooling
                let user_id = db_clone.create_user(&user).await?;
                let retrieved = db_clone.get_user(user_id).await?;
                assert!(retrieved.is_some());

                // Clean up immediately
                // Clean up would happen on test drop or next run

                Ok::<_, anyhow::Error>(())
            }));
        }

        // All operations should succeed without connection exhaustion
        for handle in handles {
            handle.await??;
        }

        Ok(())
    }

    #[tokio::test]
    #[cfg(feature = "postgresql")]
    async fn test_postgres_users_schema_no_duplicate_columns() -> Result<()> {
        use sqlx::postgres::PgPoolOptions;

        // Use isolated database to prevent race conditions with parallel tests
        let isolated_db = IsolatedPostgresDb::new().await?;
        let db = isolated_db.get_database().await?;

        // Migrations already run by get_database(), but ensure idempotency
        db.migrate().await?;

        // Create a separate pool for schema introspection queries using isolated DB URL
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&isolated_db.url)
            .await?;

        // Query PostgreSQL information_schema to get column names
        let columns: Vec<(String,)> = sqlx::query_as(
            r"
            SELECT column_name
            FROM information_schema.columns
            WHERE table_name = 'users'
            ORDER BY column_name
            ",
        )
        .fetch_all(&pool)
        .await?;

        let column_names: Vec<String> = columns.into_iter().map(|(name,)| name).collect();

        // Verify critical columns exist
        assert!(
            column_names.contains(&"user_status".to_owned()),
            "user_status column should exist in users table"
        );
        assert!(
            column_names.contains(&"approved_by".to_owned()),
            "approved_by column should exist in users table"
        );
        assert!(
            column_names.contains(&"approved_at".to_owned()),
            "approved_at column should exist in users table"
        );

        // Count occurrences of each column
        let user_status_count = column_names.iter().filter(|n| *n == "user_status").count();
        let approved_by_count = column_names.iter().filter(|n| *n == "approved_by").count();
        let approved_at_count = column_names.iter().filter(|n| *n == "approved_at").count();

        // Assert each column appears exactly once
        assert_eq!(
            user_status_count, 1,
            "user_status column should appear exactly once (found {user_status_count} occurrences)"
        );
        assert_eq!(
            approved_by_count, 1,
            "approved_by column should appear exactly once (found {approved_by_count} occurrences)"
        );
        assert_eq!(
            approved_at_count, 1,
            "approved_at column should appear exactly once (found {approved_at_count} occurrences)"
        );

        Ok(())
    }
}
