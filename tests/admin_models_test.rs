// ABOUTME: Integration tests for admin models and types
// ABOUTME: Tests admin permissions, token structures, and validation types extracted from src/admin/models.rs
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::admin::models::{AdminPermission, AdminPermissions, RateLimitPeriod};

#[test]
fn test_admin_permissions_serialization() {
    let permissions = AdminPermissions::default_admin();
    let json = permissions.to_json().unwrap();
    let restored = AdminPermissions::from_json(&json).unwrap();
    assert_eq!(permissions, restored);
}

#[test]
fn test_admin_permission_string_conversion() {
    let permission = AdminPermission::ProvisionKeys;
    let string = permission.to_string();
    let restored: AdminPermission = string.parse().unwrap();
    assert_eq!(permission, restored);
}

#[test]
fn test_super_admin_permissions() {
    let super_admin = AdminPermissions::super_admin();
    assert!(super_admin.has_permission(&AdminPermission::ManageAdminTokens));
    assert!(super_admin.has_permission(&AdminPermission::ProvisionKeys));

    let regular_admin = AdminPermissions::default_admin();
    assert!(!regular_admin.has_permission(&AdminPermission::ManageAdminTokens));
    assert!(regular_admin.has_permission(&AdminPermission::ProvisionKeys));
}

#[test]
fn test_rate_limit_period_conversion() {
    let period = RateLimitPeriod::Month;
    assert_eq!(period.window_seconds(), 2_592_000);
    assert_eq!(period.to_string(), "month");
}
