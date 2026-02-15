// ABOUTME: Tenant administration business logic for slug validation and tenant provisioning
// ABOUTME: Extracted from route handlers to enable reuse across REST, MCP, and A2A protocols
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use chrono::Utc;
use tracing::{error, info};
use uuid::Uuid;

use crate::constants::tiers;
use crate::database_plugins::{factory::Database, DatabaseProvider};
use crate::errors::{AppError, AppResult};
use crate::models::{Tenant, TenantId};

/// Reserved tenant slugs that cannot be used for user-created tenants
const RESERVED_SLUGS: &[&str] = &[
    "admin",
    "api",
    "www",
    "app",
    "dashboard",
    "auth",
    "oauth",
    "login",
    "logout",
    "signup",
    "system",
    "root",
    "public",
    "static",
    "assets",
];

/// Maximum allowed length for tenant slugs
const MAX_SLUG_LENGTH: usize = 63;

/// Validate a tenant slug against naming rules
///
/// Slugs must:
/// - Be non-empty
/// - Be 63 characters or fewer
/// - Contain only ASCII alphanumeric characters and hyphens
/// - Not start or end with a hyphen
/// - Not be a reserved slug
///
/// # Errors
///
/// Returns an error describing which validation rule failed
pub fn validate_tenant_slug(slug: &str) -> AppResult<()> {
    if slug.is_empty() {
        return Err(AppError::invalid_input("Tenant slug cannot be empty"));
    }

    if slug.len() > MAX_SLUG_LENGTH {
        return Err(AppError::invalid_input(
            "Tenant slug must be 63 characters or less",
        ));
    }

    if !slug.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(AppError::invalid_input(
            "Tenant slug can only contain letters, numbers, and hyphens",
        ));
    }

    if slug.starts_with('-') || slug.ends_with('-') {
        return Err(AppError::invalid_input(
            "Tenant slug cannot start or end with a hyphen",
        ));
    }

    if RESERVED_SLUGS.contains(&slug) {
        return Err(AppError::invalid_input(format!(
            "Tenant slug '{slug}' is reserved and cannot be used",
        )));
    }

    Ok(())
}

/// Create a default tenant for a user with validated slug
///
/// Validates the slug, checks for duplicates, and creates the tenant in the database.
///
/// # Errors
///
/// Returns error if slug validation fails, slug is already in use, or database operation fails
pub async fn create_tenant_for_user(
    database: &Database,
    owner_user_id: Uuid,
    tenant_name: &str,
    tenant_slug: &str,
) -> AppResult<Tenant> {
    let slug = tenant_slug.trim().to_lowercase();
    validate_tenant_slug(&slug)?;

    if database.get_tenant_by_slug(&slug).await.is_ok() {
        return Err(AppError::invalid_input(format!(
            "Tenant slug '{slug}' is already in use",
        )));
    }

    let tenant_data = Tenant {
        id: TenantId::new(),
        name: tenant_name.to_owned(),
        slug,
        domain: None,
        plan: tiers::STARTER.to_owned(),
        owner_user_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    database
        .create_tenant(&tenant_data)
        .await
        .map_err(|e| AppError::database(format!("Failed to create tenant: {e}")))?;

    Ok(tenant_data)
}

/// Provision a tenant during user approval and link the user to it
///
/// Orchestrates tenant creation and user-tenant linking as part of the user approval
/// workflow. Generates default tenant name and slug if not provided.
///
/// # Errors
///
/// Returns error if tenant creation or user linking fails
pub async fn provision_tenant_for_approval(
    database: &Database,
    user_id: Uuid,
    user_email: &str,
    display_name: Option<&str>,
    tenant_name: Option<&str>,
    tenant_slug: Option<&str>,
) -> AppResult<Tenant> {
    let name = tenant_name.map_or_else(
        || format!("{}'s Organization", display_name.unwrap_or(user_email)),
        ToOwned::to_owned,
    );

    let slug = tenant_slug.map_or_else(
        || format!("user-{}", user_id.as_simple()),
        ToOwned::to_owned,
    );

    let tenant = create_tenant_for_user(database, user_id, &name, &slug)
        .await
        .map_err(|e| {
            error!(
                "Failed to create default tenant for user {}: {}",
                user_email, e
            );
            AppError::internal(format!("Failed to create tenant: {e}"))
        })?;

    info!(
        "Created default tenant '{}' for user {}",
        tenant.name, user_email
    );

    database
        .update_user_tenant_id(user_id, tenant.id)
        .await
        .map_err(|e| {
            error!(
                "Failed to link user {} to tenant {}: {}",
                user_email, tenant.id, e
            );
            AppError::internal(format!("Failed to link user to created tenant: {e}"))
        })?;

    Ok(tenant)
}
