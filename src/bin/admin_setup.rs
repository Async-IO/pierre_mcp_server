// ABOUTME: Administrative token setup utility for configuring system admin credentials
// ABOUTME: Command-line interface for managing admin tokens and administrative access controls
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
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
use clap::{Parser, Subcommand};
use pierre_mcp_server::{
    admin::models::{CreateAdminTokenRequest, GeneratedAdminToken},
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppError,
};
use std::env;
use tracing::{error, info, warn};
use uuid::Uuid;

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

        /// Admin display name
        #[arg(long, default_value = "Pierre Admin")]
        name: String,

        /// Force update if user already exists
        #[arg(long)]
        force: bool,
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

    // Initialize JWKS manager for RS256 admin token signing
    info!("Initializing JWKS manager for RS256 admin tokens...");
    let mut jwks_manager = pierre_mcp_server::admin::jwks::JwksManager::new();
    jwks_manager.generate_rsa_key_pair("admin_key_1")?;
    info!("JWKS manager initialized with RSA-4096 key pair");

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
        } => {
            create_admin_user_command(&database, email, password, name, force).await?;
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

    // Check if service already has an active token
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
    } else {
        // Ignore error - might be first time setup
    }

    // Create token request
    let mut request = if super_admin {
        CreateAdminTokenRequest::super_admin(service.clone()) // Safe: String ownership for request
    } else {
        CreateAdminTokenRequest::new(service.clone()) // Safe: String ownership for request
    };

    if let Some(desc) = description {
        request.service_description = Some(desc);
    }

    if expires_days == 0 {
        request.expires_in_days = None; // Never expires
    } else {
        request.expires_in_days = Some(expires_days);
    }

    // Parse custom permissions if provided
    if let Some(permissions_str) = permissions {
        if super_admin {
            warn!("Custom permissions ignored for super admin tokens (has all permissions)");
        } else {
            info!("List Parsing custom permissions: {}", permissions_str);
            let mut parsed_permissions = Vec::new();

            for perm_str in permissions_str.split(',') {
                let trimmed = perm_str.trim();
                if let Ok(permission) =
                    trimmed.parse::<pierre_mcp_server::admin::models::AdminPermission>()
                {
                    info!("  Success Added permission: {}", permission);
                    parsed_permissions.push(permission);
                } else {
                    error!("Error Invalid permission: '{}'", trimmed);
                    info!("Valid permissions are:");
                    info!("   - provision_keys");
                    info!("   - list_keys");
                    info!("   - revoke_keys");
                    info!("   - update_key_limits");
                    info!("   - manage_admin_tokens");
                    info!("   - view_audit_logs");
                    info!("   - manage_users");
                    return Err(
                        AppError::invalid_input(format!("Invalid permission: {trimmed}")).into(),
                    );
                }
            }

            if !parsed_permissions.is_empty() {
                let permissions_count = parsed_permissions.len();
                request.permissions = Some(parsed_permissions);
                info!("Success Applied {} custom permissions", permissions_count);
            }
        }
    }

    // Load JWT secret from database (must exist - created by create-admin-user)
    info!("Loading JWT secret from database for token generation...");
    let Ok(jwt_secret) = database.get_system_secret("admin_jwt_secret").await else {
        error!("Admin JWT secret not found in database!");
        error!("Please run admin-setup create-admin-user first:");
        error!("  cargo run --bin admin-setup -- create-admin-user --email admin@example.com --password yourpassword");
        return Err(AppError::config(
            "Admin JWT secret not found. Run admin-setup create-admin-user first.",
        )
        .into());
    };

    info!(
        "JWT secret loaded for token generation (first 10 chars): {}...",
        jwt_secret.chars().take(10).collect::<String>()
    );

    // Generate token using RS256 asymmetric signing
    let generated_token = database
        .create_admin_token(&request, &jwt_secret, jwks_manager)
        .await?;

    // Display results
    display_generated_token(&generated_token);

    Ok(())
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
) -> Result<()> {
    info!("User Creating admin user: {}", email);

    // Check if user already exists
    if let Ok(Some(existing_user)) = database.get_user_by_email(&email).await {
        if !force {
            error!("Error User '{}' already exists!", email);
            info!("Use --force flag to update existing user");
            info!("   Current user details:");
            info!("   - Email: {}", existing_user.email);
            info!("   - Name: {:?}", existing_user.display_name);
            info!(
                "   - Created: {}",
                existing_user.created_at.format("%Y-%m-%d %H:%M UTC")
            );
            return Err(
                AppError::invalid_input("User already exists (use --force to update)").into(),
            );
        }

        info!("Updating existing admin user...");

        // Update existing user
        let updated_user = pierre_mcp_server::models::User {
            id: existing_user.id,
            email: email.clone(),
            display_name: Some(name.clone()),
            password_hash: {
                use argon2::{
                    password_hash::{PasswordHasher, SaltString},
                    Argon2,
                };
                let salt = SaltString::generate(&mut rand::thread_rng());
                Argon2::default()
                    .hash_password(password.as_bytes(), &salt)
                    .map_err(|e| anyhow::anyhow!("Password hashing failed: {e}"))?
                    .to_string()
            },
            tier: pierre_mcp_server::models::UserTier::Enterprise, // Admin gets enterprise tier
            tenant_id: existing_user.tenant_id, // Preserve existing tenant_id or None for system admins
            strava_token: existing_user.strava_token,
            fitbit_token: existing_user.fitbit_token,
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active, // Admin is always active
            is_admin: true,                                             // Mark as admin user
            approved_by: existing_user.approved_by, // Preserve existing approval
            approved_at: existing_user.approved_at, // Preserve existing approval
            created_at: existing_user.created_at,
            last_active: chrono::Utc::now(),
        };

        database.create_user(&updated_user).await?;
    } else {
        info!("➕ Creating new admin user...");

        // Create new user
        let new_user = pierre_mcp_server::models::User {
            id: Uuid::new_v4(),
            email: email.clone(),
            display_name: Some(name.clone()),
            password_hash: {
                use argon2::{
                    password_hash::{PasswordHasher, SaltString},
                    Argon2,
                };
                let salt = SaltString::generate(&mut rand::thread_rng());
                Argon2::default()
                    .hash_password(password.as_bytes(), &salt)
                    .map_err(|e| anyhow::anyhow!("Password hashing failed: {e}"))?
                    .to_string()
            },
            tier: pierre_mcp_server::models::UserTier::Enterprise, // Admin gets enterprise tier
            tenant_id: None, // System admins don't belong to specific tenants
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active, // Admin is always active
            is_admin: true,                                             // Mark as admin user
            approved_by: None,                     // Admin doesn't need approval
            approved_at: Some(chrono::Utc::now()), // Auto-approved
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        };

        database.create_user(&new_user).await?;
    }

    println!("\nSuccess Admin User Created Successfully!");
    println!("{}", "=".repeat(50));
    println!("User USER DETAILS:");
    println!("   Email: {email}");
    println!("   Name: {name}");
    println!("   Tier: Enterprise (Full access)");
    println!("   Status: Active");

    println!("\nKey LOGIN CREDENTIALS:");
    println!("{}", "=".repeat(50));
    println!("   Email: {email}");
    println!("   Password: {password}");

    println!("\nWARNING IMPORTANT SECURITY NOTES:");
    println!("• Change the default password in production!");
    println!("• This user has full access to the admin interface");
    println!("• Use strong passwords and enable 2FA if available");
    println!("• Consider creating additional admin users with limited permissions");

    println!("\nDocs NEXT STEPS:");
    println!("1. Start the Pierre MCP Server:");
    println!("   cargo run --bin pierre-mcp-server");
    println!("2. Open the frontend interface (usually http://localhost:8080)");
    println!("3. Login with the credentials above");
    println!("4. Generate your first admin token for API key provisioning");

    // Generate and store admin JWT secret if it doesn't exist
    info!("Ensuring admin JWT secret exists...");
    let jwt_secret = database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;
    info!(
        "Admin JWT secret is ready (first 10 chars): {}...",
        jwt_secret.chars().take(10).collect::<String>()
    );

    println!("\nSuccess Admin user is ready to use!");

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
