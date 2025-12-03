// ABOUTME: Unit tests for the role-based permission system
// ABOUTME: Tests UserRole hierarchy, Permissions bitflags, and permission checker logic
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, missing_docs)]

use pierre_mcp_server::permissions::{
    impersonation::{ImpersonationSession, PermissionDelegation},
    Permissions, UserRole,
};
use std::str::FromStr;
use uuid::Uuid;

#[test]
fn test_role_hierarchy() {
    assert!(UserRole::SuperAdmin.has_privilege(UserRole::Admin));
    assert!(UserRole::SuperAdmin.has_privilege(UserRole::User));
    assert!(UserRole::Admin.has_privilege(UserRole::User));
    assert!(!UserRole::User.has_privilege(UserRole::Admin));
    assert!(!UserRole::Admin.has_privilege(UserRole::SuperAdmin));
}

#[test]
fn test_role_parsing() {
    assert_eq!(
        "super_admin".parse::<UserRole>().unwrap(),
        UserRole::SuperAdmin
    );
    assert_eq!("admin".parse::<UserRole>().unwrap(), UserRole::Admin);
    assert_eq!("user".parse::<UserRole>().unwrap(), UserRole::User);
    assert!(UserRole::from_str("invalid").is_err());
}

#[test]
fn test_role_display() {
    assert_eq!(UserRole::SuperAdmin.as_str(), "super_admin");
    assert_eq!(UserRole::Admin.as_str(), "admin");
    assert_eq!(UserRole::User.as_str(), "user");
}

#[test]
fn test_permissions_user_default() {
    let perms = Permissions::USER_DEFAULT;
    assert!(perms.has_all(Permissions::VIEW_OWN_DATA));
    assert!(perms.has_all(Permissions::CREATE_MCP_TOKENS));
    assert!(!perms.has_any(Permissions::VIEW_ALL_USERS));
    assert!(!perms.has_any(Permissions::IMPERSONATE_USERS));
}

#[test]
fn test_permissions_admin_default() {
    let perms = Permissions::ADMIN_DEFAULT;
    assert!(perms.has_all(Permissions::VIEW_OWN_DATA));
    assert!(perms.has_all(Permissions::VIEW_ALL_USERS));
    assert!(perms.has_all(Permissions::APPROVE_USERS));
    assert!(!perms.has_any(Permissions::IMPERSONATE_USERS));
}

#[test]
fn test_permissions_super_admin() {
    let perms = Permissions::all();
    assert!(perms.has_all(Permissions::VIEW_OWN_DATA));
    assert!(perms.has_all(Permissions::VIEW_ALL_USERS));
    assert!(perms.has_all(Permissions::IMPERSONATE_USERS));
    assert!(perms.has_all(Permissions::MANAGE_ADMINS));
}

#[test]
fn test_role_default_permissions() {
    assert_eq!(
        UserRole::User.default_permissions(),
        Permissions::USER_DEFAULT
    );
    assert_eq!(
        UserRole::Admin.default_permissions(),
        Permissions::ADMIN_DEFAULT
    );
    assert_eq!(
        UserRole::SuperAdmin.default_permissions(),
        Permissions::all()
    );
}

#[test]
fn test_impersonation_session_lifecycle() {
    let admin_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let mut session =
        ImpersonationSession::new(admin_id, user_id, Some("Testing user issue".to_owned()));

    assert!(session.is_active);
    assert!(session.ended_at.is_none());
    assert_eq!(session.impersonator_id, admin_id);
    assert_eq!(session.target_user_id, user_id);

    session.end();

    assert!(!session.is_active);
    assert!(session.ended_at.is_some());
    assert!(session.duration_seconds() >= 0);
}

#[test]
fn test_permission_delegation() {
    let grantor = Uuid::new_v4();
    let grantee = Uuid::new_v4();

    let mut delegation = PermissionDelegation::new(
        grantor, grantee, 0x1F, // Some permissions
        None,
    );

    assert!(delegation.is_active());

    delegation.revoke();

    assert!(!delegation.is_active());
}

#[test]
fn test_delegation_expiration() {
    let grantor = Uuid::new_v4();
    let grantee = Uuid::new_v4();

    // Create expired delegation
    let delegation = PermissionDelegation {
        id: Uuid::new_v4().to_string(),
        grantor_id: grantor,
        grantee_id: grantee,
        permissions: 0x1F,
        expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
        revoked_at: None,
        created_at: chrono::Utc::now() - chrono::Duration::hours(2),
    };

    assert!(!delegation.is_active());
}
