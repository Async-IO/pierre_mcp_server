// ABOUTME: User management commands for pierre-cli
// ABOUTME: Handles creation and management of admin users for frontend login
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use pierre_mcp_server::{
    constants::tiers,
    database::CreateUserMcpTokenRequest,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, AppResult},
    models::{Tenant, TenantId, User, UserStatus, UserTier},
    permissions::UserRole,
};

type Result<T> = AppResult<T>;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::helpers::display::display_admin_user_success;

/// Create or update admin user for frontend login
pub async fn create(
    database: &Database,
    email: String,
    password: String,
    name: Option<String>,
    force: bool,
    super_admin: bool,
) -> Result<()> {
    // Derive display name from email prefix if not provided
    let display_name =
        name.unwrap_or_else(|| email.split('@').next().unwrap_or("Admin").to_owned());

    let role_str = if super_admin { "super admin" } else { "admin" };
    info!("User Creating {} user: {}", role_str, email);

    // Check if user already exists and handle accordingly
    if let Ok(Some(existing_user)) = database.get_user_by_email(&email).await {
        update_existing_admin_user(
            database,
            existing_user,
            &email,
            &password,
            &display_name,
            force,
            super_admin,
        )
        .await?;
    } else {
        create_new_admin_user(database, &email, &password, &display_name, super_admin).await?;
    }

    display_admin_user_success(&email, &display_name, &password, super_admin);
    initialize_admin_jwt_secret(database).await?;

    println!("\nSuccess Admin user is ready to use!");

    Ok(())
}

async fn update_existing_admin_user(
    database: &Database,
    existing_user: User,
    email: &str,
    password: &str,
    name: &str,
    force: bool,
    super_admin: bool,
) -> Result<()> {
    if !force {
        display_existing_user_error(&existing_user);
        return Err(AppError::invalid_input(
            "User already exists (use --force to update)",
        ));
    }

    let role_str = if super_admin { "super admin" } else { "admin" };
    info!("Updating existing {} user...", role_str);

    let role = if super_admin {
        UserRole::SuperAdmin
    } else {
        UserRole::Admin
    };

    // NOTE: tenant_id is managed via tenant_users junction table, not on User struct
    let updated_user = User {
        id: existing_user.id,
        email: email.to_owned(),
        display_name: Some(name.to_owned()),
        password_hash: hash(password, DEFAULT_COST)
            .map_err(|e| AppError::internal(format!("bcrypt error: {e}")))?,
        tier: UserTier::Enterprise,
        strava_token: existing_user.strava_token,
        fitbit_token: existing_user.fitbit_token,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: true,
        role,
        approved_by: existing_user.approved_by,
        approved_at: existing_user.approved_at,
        created_at: existing_user.created_at,
        last_active: Utc::now(),
        firebase_uid: existing_user.firebase_uid,
        auth_provider: existing_user.auth_provider,
    };

    database.create_user(&updated_user).await?;
    Ok(())
}

fn display_existing_user_error(existing_user: &User) {
    let details = format!(
        "Email: {}\nName: {:?}\nCreated: {}",
        existing_user.email,
        existing_user.display_name,
        existing_user.created_at.format("%Y-%m-%d %H:%M UTC")
    );

    error!("Error User '{}' already exists!", existing_user.email);
    info!("Use --force flag to update existing user");
    info!(
        "   Current user details:\n   - {}",
        details.replace('\n', "\n   - ")
    );
}

/// Auto-create a default MCP token for a newly activated user.
/// This is a non-fatal operation - failure is logged but does not propagate.
async fn create_default_mcp_token_for_user(database: &Database, user_id: Uuid) {
    let token_request = CreateUserMcpTokenRequest {
        name: "Default Token".to_owned(),
        expires_in_days: None, // Never expires
    };

    match database
        .create_user_mcp_token(user_id, &token_request)
        .await
    {
        Ok(token_result) => {
            info!(
                user_id = %user_id,
                token_id = %token_result.token.id,
                "Auto-created default MCP token for admin user"
            );
        }
        Err(e) => {
            // Log error but don't fail - user can create token manually
            warn!(
                user_id = %user_id,
                error = %e,
                "Failed to auto-create MCP token for admin user (non-fatal)"
            );
        }
    }
}

/// Create a personal tenant for a user and link them to it
async fn create_and_link_personal_tenant(
    database: &Database,
    user_id: Uuid,
    name: &str,
    slug_prefix: &str,
) -> Result<()> {
    let tenant_id = TenantId::new();
    let tenant_slug = format!("{slug_prefix}-{}", user_id.as_simple());
    let tenant = Tenant {
        id: tenant_id,
        name: format!("{name}'s Workspace"),
        slug: tenant_slug,
        domain: None,
        plan: tiers::ENTERPRISE.to_owned(),
        owner_user_id: user_id,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    database.create_tenant(&tenant).await?;
    info!("Created personal tenant: {} ({})", tenant.name, tenant_id);

    database.update_user_tenant_id(user_id, tenant_id).await?;

    Ok(())
}

/// Build admin user model with the given parameters
fn build_admin_user(
    user_id: Uuid,
    email: &str,
    password_hash: String,
    name: &str,
    role: UserRole,
) -> User {
    // NOTE: tenant_id is managed via tenant_users junction table, not on User struct
    User {
        id: user_id,
        email: email.to_owned(),
        display_name: Some(name.to_owned()),
        password_hash,
        tier: UserTier::Enterprise,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: true,
        role,
        approved_by: None,
        approved_at: Some(Utc::now()),
        created_at: Utc::now(),
        last_active: Utc::now(),
        firebase_uid: None,
        auth_provider: "email".to_owned(),
    }
}

async fn create_new_admin_user(
    database: &Database,
    email: &str,
    password: &str,
    name: &str,
    super_admin: bool,
) -> Result<()> {
    let role_str = if super_admin { "super admin" } else { "admin" };
    info!("Creating new {} user...", role_str);

    let role = if super_admin {
        UserRole::SuperAdmin
    } else {
        UserRole::Admin
    };

    let user_id = Uuid::new_v4();
    let password_hash = hash(password, DEFAULT_COST)
        .map_err(|e| AppError::internal(format!("bcrypt error: {e}")))?;
    let new_user = build_admin_user(user_id, email, password_hash, name, role);

    database.create_user(&new_user).await?;
    info!("Created {} user: {}", role_str, email);

    create_and_link_personal_tenant(database, user_id, name, "admin").await?;

    create_default_mcp_token_for_user(database, new_user.id).await;

    Ok(())
}

async fn initialize_admin_jwt_secret(database: &Database) -> Result<()> {
    info!("Ensuring admin JWT secret exists...");
    database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;
    info!("Admin JWT signing key initialized successfully");
    Ok(())
}
