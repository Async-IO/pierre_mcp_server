// ABOUTME: Server implementation for serving users with isolated data access
// ABOUTME: Production-ready server with authentication and user isolation capabilities
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Pierre Fitness API Server Binary
//!
//! This binary starts the multi-protocol Pierre Fitness API with user authentication,
//! secure token storage, and database management.

use anyhow::Result;
use clap::Parser;
use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::ServerConfig,
    database_plugins::{factory::Database, DatabaseProvider},
    logging,
    mcp::multitenant::MultiTenantMcpServer,
};
use std::sync::Arc;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "pierre-mcp-server")]
#[command(about = "Pierre Fitness API - Multi-protocol fitness data API for LLMs")]
pub struct Args {
    /// Configuration file path for providers
    #[arg(short, long)]
    config: Option<String>,

    /// Override MCP port
    #[arg(long)]
    mcp_port: Option<u16>,

    /// Override HTTP port
    #[arg(long)]
    http_port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle Docker environment where clap may not work properly
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Argument parsing failed: {e}");
            eprintln!("Using default configuration for production mode");
            // Default to production mode if argument parsing fails
            Args {
                config: None,
                mcp_port: None,
                http_port: None,
            }
        }
    };

    {
        // Load configuration from environment
        let mut config = ServerConfig::from_env()?;

        // Override ports if specified
        if let Some(mcp_port) = args.mcp_port {
            config.mcp_port = mcp_port;
        }
        if let Some(http_port) = args.http_port {
            config.http_port = http_port;
        }

        // Initialize production logging
        logging::init_from_env()?;

        info!("Starting Pierre Fitness API - Production Mode");
        info!("{}", config.summary());

        // Initialize two-tier key management system
        let (mut key_manager, database_encryption_key) =
            pierre_mcp_server::key_management::KeyManager::bootstrap()?;
        info!("Two-tier key management system bootstrapped");

        // Initialize database with DEK from key manager
        let database = Database::new(
            &config.database.url.to_connection_string(),
            database_encryption_key.to_vec(),
        )
        .await?;
        info!(
            "Database initialized successfully: {}",
            database.backend_info()
        );
        info!(
            "Database URL: {}",
            &config.database.url.to_connection_string()
        );

        // Complete key manager initialization with database
        key_manager.complete_initialization(&database).await?;
        info!("Two-tier key management system fully initialized");

        // Get JWT secret from database (must exist - created by admin-setup)
        let jwt_secret_string = if let Ok(secret) =
            database.get_system_secret("admin_jwt_secret").await
        {
            info!(
                "Admin JWT secret loaded from database (first 10 chars): {}...",
                secret.chars().take(10).collect::<String>()
            );
            secret
        } else {
            error!("Admin JWT secret not found in database!");
            error!("Please run the admin setup first:");
            error!("  cargo run --bin admin-setup -- create-admin-user --email admin@example.com --password yourpassword");
            return Err(anyhow::anyhow!(
                "Admin JWT secret not found. Run admin-setup create-admin-user first."
            ));
        };

        // Initialize authentication manager
        let auth_manager = {
            // Safe: JWT expiry hours are small positive configuration values (1-168)
            #[allow(clippy::cast_possible_wrap)]
            {
                AuthManager::new(
                    jwt_secret_string.as_bytes().to_vec(),
                    config.auth.jwt_expiry_hours as i64,
                )
            }
        };
        info!("Authentication manager initialized");

        // Create and run server
        let server = MultiTenantMcpServer::new(
            database,
            auth_manager,
            &jwt_secret_string,
            Arc::new(config.clone()),
        );

        info!(
            "MCP server starting on ports {} (MCP) and {} (HTTP)",
            config.mcp_port, config.http_port
        );
        info!("Ready to serve fitness data!");

        // Run the server (includes all routes)
        if let Err(e) = server.run(config.mcp_port).await {
            error!("Server error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
