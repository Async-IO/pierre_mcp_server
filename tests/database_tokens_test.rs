// ABOUTME: Unit tests for database tokens functionality
// ABOUTME: Validates database tokens behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::constants::oauth_providers;
use pierre_mcp_server::database::{user_oauth_tokens::OAuthTokenData, Database};
use pierre_mcp_server::models::{DecryptedToken, TenantId, User, UserStatus, UserTier};
use pierre_mcp_server::permissions::UserRole;
use uuid::Uuid;

#[tokio::test]
async fn test_strava_token_storage() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    // Create a test user
    let user = User {
        id: Uuid::new_v4(),
        email: format!("strava_{}@example.com", Uuid::new_v4()),
        display_name: None,
        password_hash: "hashed".into(),
        tier: UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };

    db.create_user(&user).await.expect("Failed to create user");

    // Create test token with timestamp precision truncated to seconds
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(3600);
    let expires_at_truncated =
        chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).expect("Valid timestamp");
    let token = DecryptedToken {
        access_token: "test_access_token".into(),
        refresh_token: "test_refresh_token".into(),
        expires_at: expires_at_truncated,
        scope: "read,activity:read_all".into(),
    };

    // Store token
    let token_id = uuid::Uuid::new_v4().to_string();
    let oauth_token_data = OAuthTokenData {
        id: &token_id,
        user_id: user.id,
        tenant_id: TenantId::from_uuid(Uuid::nil()),
        provider: oauth_providers::STRAVA,
        access_token: &token.access_token,
        refresh_token: Some(&token.refresh_token),
        token_type: "Bearer",
        expires_at: Some(token.expires_at),
        scope: &token.scope,
    };
    db.upsert_user_oauth_token(&oauth_token_data)
        .await
        .expect("Failed to update Strava token");

    // Retrieve token
    let retrieved_oauth = db
        .get_user_oauth_token(
            user.id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await
        .expect("Failed to get Strava token")
        .expect("Token not found");

    let retrieved = DecryptedToken {
        access_token: retrieved_oauth.access_token,
        refresh_token: retrieved_oauth.refresh_token.unwrap_or_default(),
        expires_at: retrieved_oauth.expires_at.unwrap_or_else(chrono::Utc::now),
        scope: retrieved_oauth.scope.unwrap_or_default(),
    };

    assert_eq!(retrieved.access_token, token.access_token);
    assert_eq!(retrieved.refresh_token, token.refresh_token);
    assert_eq!(retrieved.expires_at, token.expires_at);
    assert_eq!(retrieved.scope, token.scope);

    // Clear token
    db.delete_user_oauth_token(
        user.id,
        TenantId::from_uuid(Uuid::nil()),
        oauth_providers::STRAVA,
    )
    .await
    .expect("Failed to clear Strava token");

    // Verify cleared
    let cleared = db
        .get_user_oauth_token(
            user.id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await
        .expect("Failed to get Strava token");
    assert!(cleared.is_none());
}

#[tokio::test]
async fn test_fitbit_token_storage() {
    let db = Database::new("sqlite::memory:", vec![0u8; 32])
        .await
        .expect("Failed to create test database");

    // Create a test user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: format!("fitbit_{user_id}@example.com"),
        display_name: None,
        password_hash: "hashed".into(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };

    db.create_user(&user).await.expect("Failed to create user");

    // Create test token with timestamp precision truncated to seconds
    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(7200);
    let expires_at_truncated =
        chrono::DateTime::from_timestamp(expires_at.timestamp(), 0).expect("Valid timestamp");
    let token = DecryptedToken {
        access_token: "fitbit_access_token".into(),
        refresh_token: "fitbit_refresh_token".into(),
        expires_at: expires_at_truncated,
        scope: "activity heartrate location".into(),
    };

    // Store token
    let token_id = uuid::Uuid::new_v4().to_string();
    let oauth_token_data = OAuthTokenData {
        id: &token_id,
        user_id,
        tenant_id: TenantId::from_uuid(Uuid::nil()),
        provider: oauth_providers::FITBIT,
        access_token: &token.access_token,
        refresh_token: Some(&token.refresh_token),
        token_type: "Bearer",
        expires_at: Some(token.expires_at),
        scope: &token.scope,
    };
    db.upsert_user_oauth_token(&oauth_token_data)
        .await
        .expect("Failed to update Fitbit token");

    // Retrieve token
    let retrieved_oauth = db
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::FITBIT,
        )
        .await
        .expect("Failed to get Fitbit token")
        .expect("Token not found");

    let retrieved = DecryptedToken {
        access_token: retrieved_oauth.access_token,
        refresh_token: retrieved_oauth.refresh_token.unwrap_or_default(),
        expires_at: retrieved_oauth.expires_at.unwrap_or_else(chrono::Utc::now),
        scope: retrieved_oauth.scope.unwrap_or_default(),
    };

    assert_eq!(retrieved.access_token, token.access_token);
    assert_eq!(retrieved.refresh_token, token.refresh_token);
    assert_eq!(retrieved.expires_at, token.expires_at);
    assert_eq!(retrieved.scope, token.scope);
}
