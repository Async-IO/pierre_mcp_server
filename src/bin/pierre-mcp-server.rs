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
    auth::{generate_jwt_secret, AuthManager},
    config::environment::ServerConfig,
    database::generate_encryption_key,
    database_plugins::factory::Database,
    logging,
    mcp::multitenant::MultiTenantMcpServer,
};
use std::path::PathBuf;
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

        // Load or generate encryption key
        let encryption_key = load_or_generate_key(&config.database.encryption_key_path)?;
        info!(
            "Encryption key loaded from: {}",
            config.database.encryption_key_path.display()
        );

        // Load or generate JWT secret
        let jwt_secret = load_or_generate_jwt_secret(&config.auth.jwt_secret_path)?;
        info!(
            "JWT secret loaded from: {}",
            config.auth.jwt_secret_path.display()
        );

        // Initialize database
        let database = Database::new(
            &config.database.url.to_connection_string(),
            encryption_key.to_vec(),
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

        // Initialize authentication manager
        let auth_manager = {
            // Safe: JWT expiry hours are small positive configuration values (1-168)
            #[allow(clippy::cast_possible_wrap)]
            {
                AuthManager::new(jwt_secret.to_vec(), config.auth.jwt_expiry_hours as i64)
            }
        };
        info!("Authentication manager initialized");

        // Create and run server
        let server = MultiTenantMcpServer::new(database, auth_manager, Arc::new(config.clone()));

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

/// Load encryption key from file or generate a new one
fn load_or_generate_key(key_file: &PathBuf) -> Result<[u8; 32]> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = key_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if key_file.exists() {
        // Load existing key
        let key_data = std::fs::read(key_file)?;
        if key_data.len() != 32 {
            return Err(anyhow::anyhow!(
                "Invalid encryption key length: expected 32 bytes, got {}",
                key_data.len()
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&key_data);
        Ok(key)
    } else {
        // Generate new key
        let key = generate_encryption_key();
        std::fs::write(key_file, key)?;
        info!("Generated new encryption key: {}", key_file.display());
        Ok(key)
    }
}

/// Load JWT secret from file or generate a new one
fn load_or_generate_jwt_secret(secret_file: &PathBuf) -> Result<[u8; 64]> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = secret_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if secret_file.exists() {
        // Load existing secret
        let secret_data = std::fs::read(secret_file)?;
        if secret_data.len() != 64 {
            return Err(anyhow::anyhow!(
                "Invalid JWT secret length: expected 64 bytes, got {}",
                secret_data.len()
            ));
        }

        let mut secret = [0u8; 64];
        secret.copy_from_slice(&secret_data);
        Ok(secret)
    } else {
        // Generate new secret
        let secret = generate_jwt_secret();
        std::fs::write(secret_file, secret)?;
        info!("Generated new JWT secret: {}", secret_file.display());
        Ok(secret)
    }
}
