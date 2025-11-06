// ABOUTME: Test utilities for creating User structs and other test data in a consistent way
// ABOUTME: Centralizes test data creation to avoid duplication and ensure consistency across tests
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use crate::models::{User, UserStatus, UserTier};
use chrono::Utc;
use uuid::Uuid;

/// Create a test admin user with default values
#[must_use]
pub fn create_test_admin_user(email: &str, display_name: Option<String>) -> User {
    User {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        display_name,
        password_hash: "test_password_hash".to_owned(),
        tier: UserTier::Enterprise,
        tenant_id: Some("test-tenant".to_owned()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: true, // Admin user
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
    }
}

/// Create a test regular user with default values
#[must_use]
pub fn create_test_user(email: &str, display_name: Option<String>) -> User {
    User {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        display_name,
        password_hash: "test_password_hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: Some("test-tenant".to_owned()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false, // Regular user
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
    }
}

/// Create a test pending user awaiting approval
#[must_use]
pub fn create_test_pending_user(email: &str, display_name: Option<String>) -> User {
    User {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        display_name,
        password_hash: "test_password_hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: Some("test-tenant".to_owned()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Pending,
        is_admin: false, // Regular user
        approved_by: None,
        approved_at: None, // Not approved yet
        created_at: Utc::now(),
        last_active: Utc::now(),
    }
}

/// Create a test user with custom fields
#[must_use]
pub fn create_test_user_with_fields(
    email: &str,
    display_name: Option<String>,
    tier: UserTier,
    user_status: &UserStatus,
    is_admin: bool,
) -> User {
    User {
        id: Uuid::new_v4(),
        email: email.to_owned(),
        display_name,
        password_hash: "test_password_hash".to_owned(),
        tier,
        tenant_id: Some("test-tenant".to_owned()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: *user_status,
        is_admin,
        approved_by: if matches!(user_status, UserStatus::Active) {
            Some(Uuid::new_v4())
        } else {
            None
        },
        approved_at: if matches!(user_status, UserStatus::Active) {
            Some(Utc::now())
        } else {
            None
        },
        created_at: Utc::now(),
        last_active: Utc::now(),
    }
}
