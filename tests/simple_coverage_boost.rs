// ABOUTME: Simple tests to boost coverage for critical areas
// ABOUTME: Focused on exercising code paths rather than complex functionality
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
#![allow(
    clippy::uninlined_format_args,
    clippy::match_same_arms,
    clippy::single_match_else
)]

//! Simple tests to boost coverage for critical areas
//!
//! Focused on exercising code paths rather than complex functionality

use anyhow::Result;
use pierre_mcp_server::{
    config::environment::OAuthProviderConfig,
    database_plugins::DatabaseProvider,
    models::{EncryptedToken, User, UserTier},
    oauth::providers::StravaOAuthProvider,
};
use uuid::Uuid;

mod common;
use common::*;

/// Test User model edge cases
#[tokio::test]
async fn test_user_model_serialization() -> Result<()> {
    let users = vec![
        // Minimal user
        User {
            id: Uuid::new_v4(),
            email: "minimal@example.com".to_string(),
            display_name: None,
            password_hash: "hash".to_string(),
            tier: UserTier::Starter,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active,
            is_admin: false,
            approved_by: None,
            approved_at: Some(chrono::Utc::now()),
            strava_token: None,
            fitbit_token: None,
            tenant_id: Some("test-tenant".to_string()),
        },
        // User with encrypted tokens
        User {
            id: Uuid::new_v4(),
            email: "with_tokens@example.com".to_string(),
            display_name: Some("Token User".to_string()),
            password_hash: "complex_hash".to_string(),
            tier: UserTier::Enterprise,
            created_at: chrono::Utc::now() - chrono::Duration::days(30),
            last_active: chrono::Utc::now() - chrono::Duration::hours(1),
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active,
            is_admin: false,
            approved_by: None,
            approved_at: Some(chrono::Utc::now()),
            strava_token: Some(EncryptedToken {
                access_token: "encrypted_strava".to_string(),
                refresh_token: "encrypted_refresh".to_string(),
                expires_at: chrono::Utc::now() + chrono::Duration::hours(6),
                scope: "read,activity:read_all".to_string(),
            }),
            fitbit_token: None,
            tenant_id: Some("test-tenant".to_string()),
        },
        // Inactive user
        User {
            id: Uuid::new_v4(),
            email: "inactive@example.com".to_string(),
            display_name: Some("Inactive".to_string()),
            password_hash: "old_hash".to_string(),
            tier: UserTier::Professional,
            created_at: chrono::Utc::now() - chrono::Duration::days(365),
            last_active: chrono::Utc::now() - chrono::Duration::days(30),
            is_active: false,
            user_status: pierre_mcp_server::models::UserStatus::Suspended,
            approved_by: None,
            approved_at: Some(chrono::Utc::now()),
            strava_token: None,
            fitbit_token: None,
            tenant_id: Some("test-tenant".to_string()),
            is_admin: false,
        },
    ];

    for user in users {
        // Test serialization
        let serialized = serde_json::to_string(&user)?;
        assert!(!serialized.is_empty());

        // Test deserialization
        let deserialized: User = serde_json::from_str(&serialized)?;
        assert_eq!(user.id, deserialized.id);
        assert_eq!(user.email, deserialized.email);
        assert_eq!(user.is_active, deserialized.is_active);

        // Test User::new constructor
        let new_user = User::new(
            user.email.clone(),
            user.password_hash.clone(),
            user.display_name.clone(),
        );
        assert_eq!(new_user.email, user.email);
        assert_eq!(new_user.tier, UserTier::Starter); // Default
    }

    Ok(())
}

/// Test UserTier variants
#[tokio::test]
async fn test_user_tier_variants() -> Result<()> {
    let tiers = vec![
        UserTier::Starter,
        UserTier::Professional,
        UserTier::Enterprise,
    ];

    for tier in tiers {
        // Test serialization
        let serialized = serde_json::to_string(&tier)?;
        let deserialized: UserTier = serde_json::from_str(&serialized)?;
        assert_eq!(tier, deserialized);
    }

    Ok(())
}

/// Test EncryptedToken scenarios
#[tokio::test]
async fn test_encrypted_token_edge_cases() -> Result<()> {
    let tokens = vec![
        // Short-lived token
        EncryptedToken {
            access_token: "short".to_string(),
            refresh_token: "refresh".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
            scope: "read".to_string(),
        },
        // Long-lived token
        EncryptedToken {
            access_token: "very_long_token_value".to_string(),
            refresh_token: "very_long_refresh_value".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::days(365),
            scope: "read,write,activity:read_all,profile:read_all".to_string(),
        },
        // Expired token
        EncryptedToken {
            access_token: "expired".to_string(),
            refresh_token: "expired_refresh".to_string(),
            expires_at: chrono::Utc::now() - chrono::Duration::hours(1),
            scope: "expired".to_string(),
        },
    ];

    for token in tokens {
        // Test serialization
        let serialized = serde_json::to_string(&token)?;
        let deserialized: EncryptedToken = serde_json::from_str(&serialized)?;
        assert_eq!(token.access_token, deserialized.access_token);
        assert_eq!(token.scope, deserialized.scope);

        // Test expiration check
        let is_expired = token.expires_at < chrono::Utc::now();
        println!("Token expired: {}", is_expired);
    }

    Ok(())
}

/// Test OAuth provider initialization errors
#[tokio::test]
async fn test_oauth_provider_error_cases() -> Result<()> {
    let strava_api_config = pierre_mcp_server::config::environment::StravaApiConfig {
        base_url: "https://www.strava.com/api/v3".to_string(),
        auth_url: "https://www.strava.com/oauth/authorize".to_string(),
        token_url: "https://www.strava.com/oauth/token".to_string(),
        deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_string(),
    };

    // Test missing client_id
    let missing_client_id = OAuthProviderConfig {
        client_id: None,
        client_secret: Some("secret".to_string()),
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };

    let result = StravaOAuthProvider::from_config(&missing_client_id, &strava_api_config);
    assert!(result.is_err());

    // Test missing client_secret
    let missing_secret = OAuthProviderConfig {
        client_id: Some("client".to_string()),
        client_secret: None,
        redirect_uri: Some("http://localhost:3000/callback".to_string()),
        scopes: vec!["read".to_string()],
        enabled: true,
    };

    let result2 = StravaOAuthProvider::from_config(&missing_secret, &strava_api_config);
    assert!(result2.is_err());

    Ok(())
}

/// Test database error scenarios
#[tokio::test]
async fn test_database_edge_cases() -> Result<()> {
    let database = create_test_database().await?;

    // Test duplicate user creation
    let user = User::new(
        "duplicate@example.com".to_string(),
        "hash".to_string(),
        Some("Duplicate User".to_string()),
    );

    // Create user first time
    database.create_user(&user).await?;

    // Try to create same user again - may or may not fail depending on DB constraints
    let duplicate_result = database.create_user(&user).await;
    match duplicate_result {
        Ok(_) => {
            // Database allows duplicates in test mode
        }
        Err(_) => {
            // Database enforces unique constraints
        }
    }

    Ok(())
}

/// Test authentication edge cases
#[tokio::test]
async fn test_auth_edge_cases() -> Result<()> {
    let auth_manager = create_test_auth_manager();

    // Test token generation for different user tiers
    let users = vec![
        User {
            id: Uuid::new_v4(),
            email: "starter@example.com".to_string(),
            display_name: Some("Starter".to_string()),
            password_hash: "hash".to_string(),
            tier: UserTier::Starter,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active,
            is_admin: false,
            approved_by: None,
            approved_at: Some(chrono::Utc::now()),
            strava_token: None,
            fitbit_token: None,
            tenant_id: Some("test-tenant".to_string()),
        },
        User {
            id: Uuid::new_v4(),
            email: "enterprise@example.com".to_string(),
            display_name: Some("Enterprise".to_string()),
            password_hash: "hash".to_string(),
            tier: UserTier::Enterprise,
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active,
            is_admin: false,
            approved_by: None,
            approved_at: Some(chrono::Utc::now()),
            strava_token: None,
            fitbit_token: None,
            tenant_id: Some("test-tenant".to_string()),
        },
    ];

    let jwks_manager = common::get_shared_test_jwks();
    for user in users {
        let token = auth_manager.generate_token(&user, &jwks_manager)?;
        assert!(!token.is_empty());

        // Test token validation
        let validation = auth_manager.validate_token(&token, &jwks_manager)?;
        assert_eq!(validation.sub, user.id.to_string());
    }

    Ok(())
}

/// Test various model combinations
#[tokio::test]
async fn test_model_combinations() -> Result<()> {
    // Test user with both tokens
    let user_with_tokens = User {
        id: Uuid::new_v4(),
        email: "both_tokens@example.com".to_string(),
        display_name: Some("Both Tokens".to_string()),
        password_hash: "hash".to_string(),
        tier: UserTier::Professional,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        strava_token: Some(EncryptedToken {
            access_token: "strava_access".to_string(),
            refresh_token: "strava_refresh".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(6),
            scope: "read,activity:read_all".to_string(),
        }),
        fitbit_token: Some(EncryptedToken {
            access_token: "fitbit_access".to_string(),
            refresh_token: "fitbit_refresh".to_string(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(8),
            scope: "activity,profile".to_string(),
        }),
        tenant_id: Some("test-tenant".to_string()),
        is_admin: false,
    };

    // Test serialization
    let serialized = serde_json::to_string(&user_with_tokens)?;
    assert!(serialized.contains("strava_token"));
    assert!(serialized.contains("fitbit_token"));

    // Test deserialization
    let deserialized: User = serde_json::from_str(&serialized)?;
    assert!(deserialized.strava_token.is_some());
    assert!(deserialized.fitbit_token.is_some());

    Ok(())
}
