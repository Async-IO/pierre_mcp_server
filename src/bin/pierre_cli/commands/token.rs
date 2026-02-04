// ABOUTME: Token management commands for pierre-cli
// ABOUTME: Handles generate, list, revoke, rotate, and stats operations for admin tokens
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use pierre_mcp_server::{
    admin::{
        jwks::JwksManager,
        models::{AdminPermission, CreateAdminTokenRequest},
    },
    database_plugins::{factory::Database, DatabaseProvider},
    errors::{AppError, AppResult},
};

type Result<T> = AppResult<T>;
use std::collections::HashMap;
use tracing::{error, info, warn};

use crate::helpers::display::display_generated_token;

/// Generate a new admin token
pub async fn generate(
    database: &Database,
    jwks_manager: &JwksManager,
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
            return Err(AppError::invalid_input(
                "Service already has an active token",
            ));
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

fn parse_permissions_list(permissions_str: &str) -> Result<Vec<AdminPermission>> {
    info!("List Parsing custom permissions: {}", permissions_str);
    let mut parsed_permissions = Vec::new();

    for perm_str in permissions_str.split(',') {
        let permission = parse_single_permission(perm_str.trim())?;
        info!("  Success Added permission: {}", permission);
        parsed_permissions.push(permission);
    }

    Ok(parsed_permissions)
}

fn parse_single_permission(trimmed: &str) -> Result<AdminPermission> {
    trimmed.parse::<AdminPermission>().map_err(|_| {
        error!("Error Invalid permission: '{}'", trimmed);
        print_valid_permissions();
        AppError::invalid_input(format!("Invalid permission: {trimmed}"))
    })
}

fn apply_permissions_to_request(
    request: &mut CreateAdminTokenRequest,
    parsed_permissions: Vec<AdminPermission>,
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
            "Admin JWT secret not found. Run pierre-cli user create first.",
        ));
    };

    info!("JWT signing key loaded successfully for token generation");
    Ok(jwt_secret)
}

fn log_jwt_secret_error() {
    error!("Admin JWT secret not found in database!");
    error!("Please run pierre-cli user create first:");
    error!("  pierre-cli user create --email admin@example.com --password yourpassword");
}

/// List all admin tokens
pub async fn list(database: &Database, include_inactive: bool, detailed: bool) -> Result<()> {
    info!(
        "List Listing admin tokens (include_inactive: {})",
        include_inactive
    );

    let tokens = database.list_admin_tokens(include_inactive).await?;

    if tokens.is_empty() {
        println!("No admin tokens found.");
        println!(
            "Generate your first token with: pierre-cli token generate --service your_service"
        );
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
pub async fn revoke(database: &Database, token_id: String) -> Result<()> {
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
pub async fn rotate(
    database: &Database,
    jwks_manager: &JwksManager,
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
        return Err(AppError::invalid_input("Cannot rotate inactive token"));
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

    // Load JWT secret from database (must exist - created by user create)
    let Ok(jwt_secret) = database.get_system_secret("admin_jwt_secret").await else {
        error!("Admin JWT secret not found in database!");
        return Err(AppError::config(
            "Admin JWT secret not found. Run pierre-cli user create first.",
        ));
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
pub async fn stats(database: &Database, token_id: Option<String>, days: u32) -> Result<()> {
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
        let mut action_counts = HashMap::new();
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
