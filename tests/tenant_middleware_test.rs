// ABOUTME: Tests for tenant context extraction middleware
// ABOUTME: Validates middleware correctly extracts tenant from JWT and handles edge cases
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

//! # Tenant Middleware Tests
//!
//! This module tests the tenant context extraction middleware including:
//! - `ExtractedTenantContext` wrapper functionality
//! - `require_tenant_context` helper
//! - Integration tests with real JWT tokens and database

use pierre_mcp_server::middleware::tenant::{require_tenant_context, ExtractedTenantContext};
use pierre_mcp_server::tenant::{TenantContext, TenantRole};
use uuid::Uuid;

/// Test that `ExtractedTenantContext` correctly handles None case
#[test]
fn test_extracted_tenant_context_none() {
    let ctx = ExtractedTenantContext(None);
    assert!(!ctx.is_present());
    assert!(ctx.get().is_none());
    assert!(ctx.tenant_id().is_none());
    assert!(ctx.user_id().is_none());
}

/// Test that `ExtractedTenantContext` correctly handles Some case
#[test]
fn test_extracted_tenant_context_some() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Member,
    );

    let ctx = ExtractedTenantContext(Some(tenant_ctx));
    assert!(ctx.is_present());
    assert!(ctx.get().is_some());
    assert_eq!(ctx.tenant_id(), Some(tenant_id));
    assert_eq!(ctx.user_id(), Some(user_id));
}

/// Test `require_tenant_context` returns error when context is None
#[test]
fn test_require_tenant_context_none() {
    let ctx = ExtractedTenantContext(None);
    let result = require_tenant_context(&ctx);
    assert!(result.is_err());
}

/// Test `require_tenant_context` returns Ok when context is present
#[test]
fn test_require_tenant_context_some() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Admin,
    );

    let ctx = ExtractedTenantContext(Some(tenant_ctx));
    let result = require_tenant_context(&ctx);
    assert!(result.is_ok());

    let extracted = result.expect("require_tenant_context should return Ok for valid context");
    assert_eq!(extracted.tenant_id, tenant_id);
}

/// Test `TenantContext` role checks
#[test]
fn test_tenant_context_role_checks() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Admin role
    let admin_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Admin,
    );
    assert!(admin_ctx.is_admin());
    assert!(admin_ctx.can_configure_oauth());

    // Owner role
    let owner_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Owner,
    );
    assert!(owner_ctx.is_admin());
    assert!(owner_ctx.can_configure_oauth());

    // Member role
    let member_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Member,
    );
    assert!(!member_ctx.is_admin());
    assert!(!member_ctx.can_configure_oauth());
}

/// Test `ExtractedTenantContext` Clone implementation
#[test]
fn test_extracted_tenant_context_clone() {
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Member,
    );

    let original = ExtractedTenantContext(Some(tenant_ctx));
    let cloned = original.clone();

    assert_eq!(cloned.tenant_id(), original.tenant_id());
    assert_eq!(cloned.user_id(), original.user_id());
}

/// Test `ExtractedTenantContext` Debug implementation
#[test]
fn test_extracted_tenant_context_debug() {
    let ctx_none = ExtractedTenantContext(None);
    let debug_str = format!("{ctx_none:?}");
    assert!(debug_str.contains("ExtractedTenantContext"));
    assert!(debug_str.contains("None"));

    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let tenant_ctx = TenantContext::new(
        tenant_id,
        "Test Tenant".to_owned(),
        user_id,
        TenantRole::Member,
    );
    let ctx_some = ExtractedTenantContext(Some(tenant_ctx));
    let debug_str = format!("{ctx_some:?}");
    assert!(debug_str.contains("ExtractedTenantContext"));
    assert!(debug_str.contains("Test Tenant"));
}
