// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! # Multi-Tenant MCP Server Binary
//!
//! This binary starts the multi-tenant Pierre MCP Server with user authentication,
//! secure token storage, and database management.

use anyhow::Result;
use clap::Parser;
use pierre_mcp_server::{
    auth::{AuthManager, generate_jwt_secret},
    database::{Database, generate_encryption_key},
    mcp::multitenant::MultiTenantMcpServer,
};
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "pierre-mcp-server-multitenant")]
#[command(about = "Multi-tenant Pierre MCP Server for fitness data aggregation")]
pub struct Args {
    /// Port to listen on
    #[arg(short, long, default_value = "8080")]
    port: u16,

    /// Database URL (SQLite file path)
    #[arg(short, long, default_value = "sqlite:./users.db")]
    database_url: String,

    /// JWT token expiry in hours
    #[arg(short, long, default_value = "24")]
    token_expiry_hours: i64,

    /// Encryption key file (will be generated if doesn't exist)
    #[arg(short, long, default_value = "./encryption.key")]
    encryption_key_file: PathBuf,

    /// JWT secret file (will be generated if doesn't exist)
    #[arg(short, long, default_value = "./jwt.secret")]
    jwt_secret_file: PathBuf,

    /// Run in single-tenant mode (no authentication required)
    #[arg(long, default_value = "false")]
    single_tenant: bool,

    /// Configuration file path for providers (required in single-tenant mode)
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    if args.single_tenant {
        info!("Starting Pierre MCP Server - Single-Tenant Mode");
        info!("Port: {}", args.port);
        
        // In single-tenant mode, use the original server
        // Pass the config path if provided, otherwise Config::load will use the default location
        let config = pierre_mcp_server::config::Config::load(args.config)?;
        let server = pierre_mcp_server::mcp::McpServer::new(config);
        
        info!("🚀 Single-tenant MCP server starting on port {}", args.port);
        info!("📊 Ready to serve fitness data!");
        
        if let Err(e) = server.run(args.port).await {
            error!("Server error: {}", e);
            return Err(e);
        }
    } else {
        info!("Starting Pierre MCP Server - Multi-Tenant Mode");
        info!("Port: {}", args.port);
        info!("Database: {}", args.database_url);
        info!("Token expiry: {} hours", args.token_expiry_hours);

        // Load or generate encryption key
        let encryption_key = load_or_generate_encryption_key(&args.encryption_key_file)?;
        info!("Encryption key loaded from: {}", args.encryption_key_file.display());

        // Load or generate JWT secret
        let jwt_secret = load_or_generate_jwt_secret(&args.jwt_secret_file)?;
        info!("JWT secret loaded from: {}", args.jwt_secret_file.display());

        // Initialize database
        let database = Database::new(&args.database_url, encryption_key.to_vec()).await?;
        info!("Database initialized successfully");

        // Initialize authentication manager
        let auth_manager = AuthManager::new(jwt_secret.to_vec(), args.token_expiry_hours);
        info!("Authentication manager initialized");

        // Create and run multi-tenant server
        let server = MultiTenantMcpServer::new(database, auth_manager);
        
        info!("🚀 Multi-tenant MCP server starting on port {}", args.port);
        info!("📊 Ready to serve fitness data with user authentication!");
        
        if let Err(e) = server.run(args.port).await {
            error!("Server error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Load encryption key from file or generate a new one
fn load_or_generate_encryption_key(key_file: &PathBuf) -> Result<[u8; 32]> {
    if key_file.exists() {
        // Load existing key
        let key_data = std::fs::read(key_file)?;
        if key_data.len() != 32 {
            return Err(anyhow::anyhow!("Invalid encryption key length: expected 32 bytes, got {}", key_data.len()));
        }
        
        let mut key = [0u8; 32];
        key.copy_from_slice(&key_data);
        Ok(key)
    } else {
        // Generate new key
        let key = generate_encryption_key();
        std::fs::write(key_file, &key)?;
        info!("Generated new encryption key: {}", key_file.display());
        Ok(key)
    }
}

/// Load JWT secret from file or generate a new one
fn load_or_generate_jwt_secret(secret_file: &PathBuf) -> Result<[u8; 64]> {
    if secret_file.exists() {
        // Load existing secret
        let secret_data = std::fs::read(secret_file)?;
        if secret_data.len() != 64 {
            return Err(anyhow::anyhow!("Invalid JWT secret length: expected 64 bytes, got {}", secret_data.len()));
        }
        
        let mut secret = [0u8; 64];
        secret.copy_from_slice(&secret_data);
        Ok(secret)
    } else {
        // Generate new secret
        let secret = generate_jwt_secret();
        std::fs::write(secret_file, &secret)?;
        info!("Generated new JWT secret: {}", secret_file.display());
        Ok(secret)
    }
}