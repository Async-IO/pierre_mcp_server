// ABOUTME: Administrative token setup utility for configuring system admin credentials
// ABOUTME: Command-line interface for managing admin tokens and administrative access controls
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//! for the Pierre MCP Server. Admin tokens are used by admin services to
//! provision and manage API keys for users.
//!
//! Usage:
//! ```bash
//! # Create default admin user for frontend login
//! cargo run --bin admin-setup -- create-admin-user
//!
//! # Create admin user with custom credentials
//! cargo run --bin admin-setup -- create-admin-user --email admin@mycompany.com --password mypassword
//!
//! # Generate a new admin token
//! cargo run --bin admin-setup -- generate-token --service pierre_admin_service
//!
//! # Generate a super admin token
//! cargo run --bin admin-setup -- generate-token --service admin_console --super-admin
//!
//! # List all admin tokens
//! cargo run --bin admin-setup -- list-tokens
//!
//! # Revoke an admin token
//! cargo run --bin admin-setup -- revoke-token admin_token_123
//! ```

use anyhow::Result;
use bcrypt::{hash, DEFAULT_COST};
use chrono::Utc;
use clap::{Parser, Subcommand};
use pierre_mcp_server::{
    admin::{
        jwks::JwksManager,
        models::{CreateAdminTokenRequest, GeneratedAdminToken},
    },
    constants::tiers,
    database::CreateUserMcpTokenRequest,
    database_plugins::factory::Database,
    database_plugins::DatabaseProvider,
    errors::AppError,
    models::Tenant,
};
use std::env;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Generate a new RSA keypair and persist it to the database
async fn generate_and_persist_keypair(
    database: &Database,
    jwks_manager: &mut JwksManager,
) -> Result<(), AppError> {
    info!("No persisted RSA keys found, generating new keypair");
    let kid = format!("key_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    jwks_manager.generate_rsa_key_pair(&kid)?;

    let key_pair = jwks_manager.get_active_key()?;
    let private_pem = key_pair.export_private_key_pem()?;
    let public_pem = key_pair.export_public_key_pem()?;
    let created_at = chrono::Utc::now();
    database
        .save_rsa_keypair(&kid, &private_pem, &public_pem, created_at, true, 4096)
        .await?;
    info!("Generated and persisted new RSA keypair: {}", kid);
    Ok(())
}

/// Load existing RSA keypairs from database into JWKS manager
fn load_existing_keypairs(
    jwks_manager: &mut JwksManager,
    keypairs: Vec<(String, String, String, chrono::DateTime<chrono::Utc>, bool)>,
) -> Result<(), AppError> {
    info!(
        "Loading {} persisted RSA keypairs from database",
        keypairs.len()
    );
    jwks_manager.load_keys_from_database(keypairs)?;
    info!("Successfully loaded RSA keys from database");
    Ok(())
}

/// Generate ephemeral keys when database fails
fn generate_ephemeral_keys(
    jwks_manager: &mut JwksManager,
    error: &AppError,
) -> Result<(), AppError> {
    warn!("Failed to load RSA keys from database: {error}. Generating ephemeral keys.");
    jwks_manager.generate_rsa_key_pair("admin_key_ephemeral")?;
    Ok(())
}

/// Initialize JWKS manager by loading keys from database or generating new ones
/// This ensures the CLI uses the same RSA keys as the running server
async fn initialize_jwks_manager(database: &Database) -> Result<JwksManager, AppError> {
    info!("Initializing JWKS manager for RS256 admin tokens...");
    let mut jwks_manager = JwksManager::new();

    match database.load_rsa_keypairs().await {
        Ok(keypairs) if !keypairs.is_empty() => {
            load_existing_keypairs(&mut jwks_manager, keypairs)?;
        }
        Ok(_) => {
            generate_and_persist_keypair(database, &mut jwks_manager).await?;
        }
        Err(e) => {
            generate_ephemeral_keys(&mut jwks_manager, &e)?;
        }
    }
    info!("JWKS manager initialized");
    Ok(jwks_manager)
}

#[derive(Parser)]
#[command(
    name = "admin-setup",
    about = "Pierre MCP Server Admin Token Management",
    long_about = "Manage admin tokens for Pierre MCP Server. Admin tokens allow external services to provision and manage API keys."
)]
struct AdminSetupArgs {
    #[command(subcommand)]
    command: AdminCommand,

    /// Database URL override
    #[arg(long)]
    database_url: Option<String>,

    /// Encryption key override (base64 encoded)
    #[arg(long)]
    encryption_key: Option<String>,

    /// Enable debug logging
    #[arg(long, short = 'v')]
    verbose: bool,
}

#[non_exhaustive]
#[derive(Subcommand)]
enum AdminCommand {
    /// Generate a new admin token
    GenerateToken {
        /// Service name (e.g., "`pierre_admin_service`")
        #[arg(long)]
        service: String,

        /// Service description
        #[arg(long)]
        description: Option<String>,

        /// Token expiration in days (default: 365, 0 = never expires)
        #[arg(long, default_value = "365")]
        expires_days: u64,

        /// Create super admin token (never expires, all permissions)
        #[arg(long)]
        super_admin: bool,

        /// Custom permissions (comma-separated)
        #[arg(long)]
        permissions: Option<String>,
    },

    /// Create or update admin user for frontend login
    CreateAdminUser {
        /// Admin email (required)
        #[arg(long)]
        email: String,

        /// Admin password (required)
        #[arg(long)]
        password: String,

        /// Admin display name (defaults to email prefix if not specified)
        #[arg(long)]
        name: Option<String>,

        /// Force update if user already exists
        #[arg(long)]
        force: bool,

        /// Create super admin user (can impersonate other users)
        #[arg(long)]
        super_admin: bool,
    },

    /// List all admin tokens
    ListTokens {
        /// Include inactive tokens
        #[arg(long)]
        include_inactive: bool,

        /// Show detailed information
        #[arg(long, short = 'd')]
        detailed: bool,
    },

    /// Revoke an admin token
    RevokeToken {
        /// Token ID to revoke
        token_id: String,
    },

    /// Rotate an admin token (generate new token, revoke old one)
    RotateToken {
        /// Token ID to rotate
        token_id: String,

        /// New expiration in days
        #[arg(long)]
        expires_days: Option<u64>,
    },

    /// Show admin token usage statistics
    TokenStats {
        /// Token ID (optional, shows all if omitted)
        token_id: Option<String>,

        /// Number of days to look back
        #[arg(long, default_value = "30")]
        days: u32,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = AdminSetupArgs::parse();

    // Initialize logging
    let log_level = if args.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt().with_env_filter(log_level).init();

    info!("Pierre MCP Server Admin Token Setup");

    // Load configuration
    let database_url = args
        .database_url
        .or_else(|| env::var("DATABASE_URL").ok())
        .unwrap_or_else(|| "sqlite:./data/users.db".into());

    // Initialize two-tier key management system
    let (mut key_manager, database_encryption_key) =
        pierre_mcp_server::key_management::KeyManager::bootstrap()?;
    info!("Two-tier key management system initialized for admin-setup");

    // Initialize database
    info!("Connecting to database: {}", database_url);
    let database = Database::new(
        &database_url,
        database_encryption_key.to_vec(),
        #[cfg(feature = "postgresql")]
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    // Complete key manager initialization
    key_manager.complete_initialization(&database).await?;
    info!("Two-tier key management system fully initialized for admin-setup");

    // Run database migrations to ensure admin_tokens table exists
    info!("Running database migrations...");
    database.migrate().await?;

    // Initialize JWKS manager - loads RSA keys from database for server compatibility
    let jwks_manager = initialize_jwks_manager(&database).await?;

    // Execute command
    match args.command {
        AdminCommand::GenerateToken {
            service,
            description,
            expires_days,
            super_admin,
            permissions,
        } => {
            generate_token_command(
                &database,
                &jwks_manager,
                service,
                description,
                expires_days,
                super_admin,
                permissions,
            )
            .await?;
        }
        AdminCommand::CreateAdminUser {
            email,
            password,
            name,
            force,
            super_admin,
        } => {
            // Derive display name from email prefix if not provided
            let display_name =
                name.unwrap_or_else(|| email.split('@').next().unwrap_or("Admin").to_owned());
            create_admin_user_command(&database, email, password, display_name, force, super_admin)
                .await?;
        }
        AdminCommand::ListTokens {
            include_inactive,
            detailed,
        } => {
            list_tokens_command(&database, include_inactive, detailed).await?;
        }
        AdminCommand::RevokeToken { token_id } => {
            revoke_token_command(&database, token_id).await?;
        }
        AdminCommand::RotateToken {
            token_id,
            expires_days,
        } => {
            rotate_token_command(&database, &jwks_manager, token_id, expires_days).await?;
        }
        AdminCommand::TokenStats { token_id, days } => {
            token_stats_command(&database, token_id, days).await?;
        }
    }

    Ok(())
}

/// Generate a new admin token
async fn generate_token_command(
    database: &Database,
    jwks_manager: &pierre_mcp_server::admin::jwks::JwksManager,
    service: String,
    description: Option<String>,
    expires_days: u64,
    super_admin: bool,
    permissions: Option<String>,
) -> Result<()> {
    info!("Key Generating admin token for service: {}", service);

    // Check for existing active token
    check_existing_token(database, &service).await?;

    // Build token request
    let mut request = build_token_request(service, description, expires_days, super_admin);

    // Apply custom permissions if provided
    apply_custom_permissions(&mut request, permissions, super_admin)?;

    // Load JWT secret
    let jwt_secret = load_jwt_secret(database).await?;

    // Generate and display token
    let generated_token = database
        .create_admin_token(&request, &jwt_secret, jwks_manager)
        .await?;

    display_generated_token(&generated_token);

    Ok(())
}

async fn check_existing_token(database: &Database, service: &str) -> Result<()> {
    if let Ok(existing_tokens) = database.list_admin_tokens(false).await {
        if existing_tokens
            .iter()
            .any(|t| t.service_name == service && t.is_active)
        {
            error!(
                "Error Service '{}' already has an active admin token!",
                service
            );
            info!("Use 'rotate-token' command to replace the existing token");
            return Err(AppError::invalid_input("Service already has an active token").into());
        }
    }
    Ok(())
}

fn build_token_request(
    service: String,
    description: Option<String>,
    expires_days: u64,
    super_admin: bool,
) -> CreateAdminTokenRequest {
    let mut request = if super_admin {
        CreateAdminTokenRequest::super_admin(service)
    } else {
        CreateAdminTokenRequest::new(service)
    };

    if let Some(desc) = description {
        request.service_description = Some(desc);
    }

    if expires_days == 0 {
        request.expires_in_days = None;
    } else {
        request.expires_in_days = Some(expires_days);
    }

    request
}

fn apply_custom_permissions(
    request: &mut CreateAdminTokenRequest,
    permissions: Option<String>,
    super_admin: bool,
) -> Result<()> {
    let Some(permissions_str) = permissions else {
        return Ok(());
    };

    if super_admin {
        warn!("Custom permissions ignored for super admin tokens (has all permissions)");
        return Ok(());
    }

    let parsed_permissions = parse_permissions_list(&permissions_str)?;
    apply_permissions_to_request(request, parsed_permissions);

    Ok(())
}

fn parse_permissions_list(
    permissions_str: &str,
) -> Result<Vec<pierre_mcp_server::admin::models::AdminPermission>> {
    info!("List Parsing custom permissions: {}", permissions_str);
    let mut parsed_permissions = Vec::new();

    for perm_str in permissions_str.split(',') {
        let permission = parse_single_permission(perm_str.trim())?;
        info!("  Success Added permission: {}", permission);
        parsed_permissions.push(permission);
    }

    Ok(parsed_permissions)
}

fn parse_single_permission(
    trimmed: &str,
) -> Result<pierre_mcp_server::admin::models::AdminPermission> {
    trimmed
        .parse::<pierre_mcp_server::admin::models::AdminPermission>()
        .map_err(|_| {
            error!("Error Invalid permission: '{}'", trimmed);
            print_valid_permissions();
            AppError::invalid_input(format!("Invalid permission: {trimmed}")).into()
        })
}

fn apply_permissions_to_request(
    request: &mut CreateAdminTokenRequest,
    parsed_permissions: Vec<pierre_mcp_server::admin::models::AdminPermission>,
) {
    if !parsed_permissions.is_empty() {
        let permissions_count = parsed_permissions.len();
        request.permissions = Some(parsed_permissions);
        info!("Success Applied {} custom permissions", permissions_count);
    }
}

fn print_valid_permissions() {
    let permissions = [
        "provision_keys",
        "list_keys",
        "revoke_keys",
        "update_key_limits",
        "manage_admin_tokens",
        "view_audit_logs",
        "manage_users",
    ];
    info!("Valid permissions are:");
    for perm in &permissions {
        info!("   - {}", perm);
    }
}

async fn load_jwt_secret(database: &Database) -> Result<String> {
    info!("Loading JWT secret from database for token generation...");
    let Ok(jwt_secret) = database.get_system_secret("admin_jwt_secret").await else {
        log_jwt_secret_error();
        return Err(AppError::config(
            "Admin JWT secret not found. Run admin-setup create-admin-user first.",
        )
        .into());
    };

    info!("JWT signing key loaded successfully for token generation");
    Ok(jwt_secret)
}

fn log_jwt_secret_error() {
    error!("Admin JWT secret not found in database!");
    error!("Please run admin-setup create-admin-user first:");
    error!("  cargo run --bin admin-setup -- create-admin-user --email admin@example.com --password yourpassword");
}

/// List all admin tokens
async fn list_tokens_command(
    database: &Database,
    include_inactive: bool,
    detailed: bool,
) -> Result<()> {
    info!(
        "List Listing admin tokens (include_inactive: {})",
        include_inactive
    );

    let tokens = database.list_admin_tokens(include_inactive).await?;

    if tokens.is_empty() {
        println!("No admin tokens found.");
        println!("Generate your first token with: cargo run --bin admin-setup -- generate-token --service your_service");
        return Ok(());
    }

    println!("\nList Admin Tokens:");
    println!("{}", "=".repeat(80));

    for token in tokens {
        println!("Key Token ID: {}", token.id);
        println!("   Service: {}", token.service_name);
        if let Some(desc) = &token.service_description {
            println!("   Description: {desc}");
        }
        println!(
            "   Status: {}",
            if token.is_active {
                "Active Active"
            } else {
                "Inactive Inactive"
            }
        );
        println!(
            "   Super Admin: {}",
            if token.is_super_admin {
                "Success Yes"
            } else {
                "Error No"
            }
        );
        println!(
            "   Created: {}",
            token.created_at.format("%Y-%m-%d %H:%M UTC")
        );

        if let Some(expires_at) = token.expires_at {
            println!("   Expires: {}", expires_at.format("%Y-%m-%d %H:%M UTC"));
        } else {
            println!("   Expires: Never");
        }

        if let Some(last_used) = token.last_used_at {
            println!("   Last Used: {}", last_used.format("%Y-%m-%d %H:%M UTC"));
        } else {
            println!("   Last Used: Never");
        }

        println!("   Usage Count: {}", token.usage_count);

        if detailed {
            println!("   Permissions: {:?}", token.permissions.to_vec());
            println!("   Prefix: {}", token.token_prefix);
            if let Some(ip) = &token.last_used_ip {
                println!("   Last IP: {ip}");
            }
        }

        println!("{}", "-".repeat(80));
    }

    Ok(())
}

/// Revoke an admin token
async fn revoke_token_command(database: &Database, token_id: String) -> Result<()> {
    info!("Revoking admin token: {}", token_id);

    // Check if token exists
    let token = database
        .get_admin_token_by_id(&token_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Admin token: {token_id}")))?;

    if !token.is_active {
        warn!("Token is already inactive");
        return Ok(());
    }

    // Revoke token
    database.deactivate_admin_token(&token_id).await?;

    println!("Success Admin token revoked successfully!");
    println!("   Token ID: {token_id}");
    println!("   Service: {}", token.service_name);
    println!("   The JWT token is now invalid and cannot be used");

    Ok(())
}

/// Rotate an admin token (create new, revoke old)
async fn rotate_token_command(
    database: &Database,
    jwks_manager: &pierre_mcp_server::admin::jwks::JwksManager,
    token_id: String,
    expires_days: Option<u64>,
) -> Result<()> {
    info!("Rotating admin token: {}", token_id);

    // Get existing token
    let old_token = database
        .get_admin_token_by_id(&token_id)
        .await?
        .ok_or_else(|| AppError::not_found(format!("Admin token: {token_id}")))?;

    if !old_token.is_active {
        return Err(AppError::invalid_input("Cannot rotate inactive token").into());
    }

    // Create new token with same service and permissions
    let mut request = CreateAdminTokenRequest {
        service_name: old_token.service_name.clone(),
        service_description: old_token.service_description.clone(),
        permissions: Some(old_token.permissions.to_vec()),
        expires_in_days: expires_days.or(Some(365)),
        is_super_admin: old_token.is_super_admin,
    };

    if old_token.is_super_admin {
        request.expires_in_days = None; // Super admin tokens never expire
    }

    // Load JWT secret from database (must exist - created by create-admin-user)
    let Ok(jwt_secret) = database.get_system_secret("admin_jwt_secret").await else {
        error!("Admin JWT secret not found in database!");
        return Err(AppError::config(
            "Admin JWT secret not found. Run admin-setup create-admin-user first.",
        )
        .into());
    };

    // Generate new token using RS256 asymmetric signing
    let new_token = database
        .create_admin_token(&request, &jwt_secret, jwks_manager)
        .await?;

    // Revoke old token
    database.deactivate_admin_token(&token_id).await?;

    println!("Token rotation completed successfully!");
    println!("   Old Token: {token_id} (revoked)");
    println!("   New Token: {} (active)", new_token.token_id);
    println!();

    display_generated_token(&new_token);

    Ok(())
}

/// Show token usage statistics
async fn token_stats_command(
    database: &Database,
    token_id: Option<String>,
    days: u32,
) -> Result<()> {
    let start_date = chrono::Utc::now() - chrono::Duration::days(i64::from(days));
    let end_date = chrono::Utc::now();

    if let Some(id) = token_id {
        info!("Token usage statistics for: {} ({} days)", id, days);

        let usage_history = database
            .get_admin_token_usage_history(&id, start_date, end_date)
            .await?;

        if usage_history.is_empty() {
            println!("No usage data found for token {id} in the last {days} days");
            return Ok(());
        }

        println!("\nUsage Statistics for Token: {id}");
        println!("{}", "=".repeat(60));
        println!(
            "Period: {} to {}",
            start_date.format("%Y-%m-%d"),
            end_date.format("%Y-%m-%d")
        );
        println!("Total Requests: {}", usage_history.len());

        let successful = usage_history.iter().filter(|u| u.success).count();
        let failed = usage_history.len() - successful;

        println!("Successful: {} ({}%)", successful, {
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            {
                (successful as f64 / usage_history.len() as f64 * 100.0).round() as u32
            }
        });
        println!("Failed: {} ({}%)", failed, {
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            {
                (failed as f64 / usage_history.len() as f64 * 100.0).round() as u32
            }
        });

        // Group by action
        let mut action_counts = std::collections::HashMap::new();
        for usage in &usage_history {
            *action_counts.entry(&usage.action).or_insert(0) += 1;
        }

        println!("\nActions:");
        for (action, count) in action_counts {
            println!("  {action}: {count}");
        }
    } else {
        info!("Overall admin token statistics ({} days)", days);

        let tokens = database.list_admin_tokens(true).await?;

        println!("\nAdmin Token Overview");
        println!("{}", "=".repeat(60));
        println!("Total Tokens: {}", tokens.len());
        println!(
            "Active Tokens: {}",
            tokens.iter().filter(|t| t.is_active).count()
        );
        println!(
            "Super Admin Tokens: {}",
            tokens.iter().filter(|t| t.is_super_admin).count()
        );

        let total_usage: u64 = tokens.iter().map(|t| t.usage_count).sum();
        println!("Total Usage Count: {total_usage}");
    }

    Ok(())
}

/// Create or update admin user for frontend login
async fn create_admin_user_command(
    database: &Database,
    email: String,
    password: String,
    name: String,
    force: bool,
    super_admin: bool,
) -> Result<()> {
    let role_str = if super_admin { "super admin" } else { "admin" };
    info!("User Creating {} user: {}", role_str, email);

    // Check if user already exists and handle accordingly
    if let Ok(Some(existing_user)) = database.get_user_by_email(&email).await {
        update_existing_admin_user(
            database,
            existing_user,
            &email,
            &password,
            &name,
            force,
            super_admin,
        )
        .await?;
    } else {
        create_new_admin_user(database, &email, &password, &name, super_admin).await?;
    }

    display_admin_user_success(&email, &name, &password, super_admin);
    initialize_admin_jwt_secret(database).await?;

    println!("\nSuccess Admin user is ready to use!");

    Ok(())
}

async fn update_existing_admin_user(
    database: &Database,
    existing_user: pierre_mcp_server::models::User,
    email: &str,
    password: &str,
    name: &str,
    force: bool,
    super_admin: bool,
) -> Result<()> {
    if !force {
        display_existing_user_error(&existing_user);
        return Err(AppError::invalid_input("User already exists (use --force to update)").into());
    }

    let role_str = if super_admin { "super admin" } else { "admin" };
    info!("Updating existing {} user...", role_str);

    let role = if super_admin {
        pierre_mcp_server::permissions::UserRole::SuperAdmin
    } else {
        pierre_mcp_server::permissions::UserRole::Admin
    };

    let updated_user = pierre_mcp_server::models::User {
        id: existing_user.id,
        email: email.to_owned(),
        display_name: Some(name.to_owned()),
        password_hash: hash(password, DEFAULT_COST)?,
        tier: pierre_mcp_server::models::UserTier::Enterprise,
        tenant_id: existing_user.tenant_id,
        strava_token: existing_user.strava_token,
        fitbit_token: existing_user.fitbit_token,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: true,
        role,
        approved_by: existing_user.approved_by,
        approved_at: existing_user.approved_at,
        created_at: existing_user.created_at,
        last_active: chrono::Utc::now(),
        firebase_uid: existing_user.firebase_uid,
        auth_provider: existing_user.auth_provider,
    };

    database.create_user(&updated_user).await?;
    Ok(())
}

fn display_existing_user_error(existing_user: &pierre_mcp_server::models::User) {
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
    let tenant_id = Uuid::new_v4();
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

    database
        .update_user_tenant_id(user_id, &tenant_id.to_string())
        .await?;

    Ok(())
}

/// Build admin user model with the given parameters
fn build_admin_user(
    user_id: Uuid,
    email: &str,
    password_hash: String,
    name: &str,
    role: pierre_mcp_server::permissions::UserRole,
) -> pierre_mcp_server::models::User {
    pierre_mcp_server::models::User {
        id: user_id,
        email: email.to_owned(),
        display_name: Some(name.to_owned()),
        password_hash,
        tier: pierre_mcp_server::models::UserTier::Enterprise,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
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
        pierre_mcp_server::permissions::UserRole::SuperAdmin
    } else {
        pierre_mcp_server::permissions::UserRole::Admin
    };

    let user_id = Uuid::new_v4();
    let password_hash = hash(password, DEFAULT_COST)?;
    let new_user = build_admin_user(user_id, email, password_hash, name, role);

    database.create_user(&new_user).await?;
    info!("Created {} user: {}", role_str, email);

    create_and_link_personal_tenant(database, user_id, name, "admin").await?;

    create_default_mcp_token_for_user(database, new_user.id).await;

    Ok(())
}

fn display_admin_user_success(email: &str, name: &str, password: &str, super_admin: bool) {
    let role_str = if super_admin { "Super Admin" } else { "Admin" };
    println!("\nSuccess {role_str} User Created Successfully!");
    println!("{}", "=".repeat(50));
    println!("User USER DETAILS:");
    println!("   Email: {email}");
    println!("   Name: {name}");
    println!("   Role: {role_str}");
    println!("   Tier: Enterprise (Full access)");
    println!("   Status: Active");

    if super_admin {
        println!("   Capabilities: Can impersonate other users");
    }

    println!("\nKey LOGIN CREDENTIALS:");
    println!("{}", "=".repeat(50));
    println!("   Email: {email}");
    println!("   Password: {password}");

    println!("\nWARNING IMPORTANT SECURITY NOTES:");
    println!("• Change the default password in production!");
    println!("• This user has full access to the admin interface");
    if super_admin {
        println!("• Super admin can impersonate any non-super-admin user");
    }
    println!("• Use strong passwords and enable 2FA if available");
    println!("• Consider creating additional admin users with limited permissions");

    println!("\nDocs NEXT STEPS:");
    println!("1. Start the Pierre MCP Server:");
    println!("   cargo run --bin pierre-mcp-server");
    println!("2. Open the frontend interface (usually http://localhost:8080)");
    println!("3. Login with the credentials above");
    println!("4. Generate your first admin token for API key provisioning");
}

async fn initialize_admin_jwt_secret(database: &Database) -> Result<()> {
    info!("Ensuring admin JWT secret exists...");
    database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;
    info!("Admin JWT signing key initialized successfully");
    Ok(())
}

/// Display a generated token with important security warnings
fn display_generated_token(token: &GeneratedAdminToken) {
    println!("\nComplete Admin Token Generated Successfully!");
    println!("{}", "=".repeat(80));
    println!("Key TOKEN DETAILS:");
    println!("   Service: {}", token.service_name);
    println!("   Token ID: {}", token.token_id);
    println!(
        "   Super Admin: {}",
        if token.is_super_admin {
            "Success Yes"
        } else {
            "Error No"
        }
    );

    if let Some(expires) = token.expires_at {
        println!("   Expires: {}", expires.format("%Y-%m-%d %H:%M UTC"));
    } else {
        println!("   Expires: Never");
    }

    println!("\nKey YOUR JWT TOKEN (SAVE THIS NOW):");
    println!("{}", "=".repeat(80));
    println!("{}", token.jwt_token);
    println!("{}", "=".repeat(80));

    println!("\nWARNING CRITICAL SECURITY NOTES:");
    println!("• This token is shown ONLY ONCE - save it now!");
    println!("• Store it securely in your admin service environment:");
    println!("  export PIERRE_MCP_ADMIN_TOKEN=\"{}\"", token.jwt_token);
    println!("• Never share this token or commit it to version control");
    println!("• Use this token in Authorization header: Bearer <token>");
    println!(
        "• This token allows {} API key operations",
        if token.is_super_admin {
            "ALL"
        } else {
            "limited"
        }
    );

    println!("\nDocs NEXT STEPS:");
    println!("1. Save the token to your admin service environment");
    println!("2. Configure your admin service to use this token");
    println!("3. Test the connection with your admin service");

    if !token.is_super_admin {
        println!("4. For super admin access, use --super-admin flag");
    }

    println!("\nUSAGE EXAMPLE:");
    println!("curl -H \"Authorization: Bearer {}\" \\", token.jwt_token);
    println!("     -X POST http://localhost:8080/admin/provision-api-key \\");
    println!("     -d '{{\"user_email\":\"user@example.com\",\"tier\":\"starter\"}}'");

    println!("\nSuccess Token is ready to use!");
}
