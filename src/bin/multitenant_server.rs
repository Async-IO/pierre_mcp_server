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
    constants::env_config,
    database::{Database, generate_encryption_key},
    mcp::multitenant::MultiTenantMcpServer,
    config::environment::ServerConfig,
    logging,
    health::HealthChecker,
};
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Parser)]
#[command(name = "pierre-mcp-server")]
#[command(about = "Pierre MCP Server for fitness data aggregation")]
pub struct Args {
    /// Run in single-tenant mode (no authentication required)
    #[arg(long, default_value = "false")]
    single_tenant: bool,

    /// Configuration file path for providers (required in single-tenant mode)
    #[arg(short, long)]
    config: Option<String>,

    /// Override MCP port (multi-tenant mode only)
    #[arg(long)]
    mcp_port: Option<u16>,

    /// Override HTTP port (multi-tenant mode only)  
    #[arg(long)]
    http_port: Option<u16>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Handle Docker environment where clap may not work properly
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Argument parsing failed: {}", e);
            eprintln!("Using default configuration for production mode");
            // Default to production mode if argument parsing fails
            Args {
                single_tenant: false,
                config: None,
                mcp_port: None,
                http_port: None,
            }
        }
    };

    if args.single_tenant {
        // Legacy mode with simple logging
        tracing_subscriber::fmt::init();
        
        info!("Starting Pierre MCP Server - Single-Tenant Mode");
        
        // In single-tenant mode, use the original server
        let config = pierre_mcp_server::config::Config::load(args.config)?;
        let server = pierre_mcp_server::mcp::McpServer::new(config);
        
        let port = args.mcp_port.unwrap_or_else(env_config::mcp_port);
        info!("🚀 Single-tenant MCP server starting on port {}", port);
        info!("📊 Ready to serve fitness data!");
        
        if let Err(e) = server.run(port).await {
            error!("Server error: {}", e);
            return Err(e);
        }
    } else {
        // Production mode with full configuration
        
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
        
        info!("🚀 Starting Pierre MCP Server - Production Mode");
        info!("{}", config.summary());

        // Load or generate encryption key
        let encryption_key = load_or_generate_key(&config.database.encryption_key_path)?;
        info!("Encryption key loaded from: {}", config.database.encryption_key_path.display());

        // Load or generate JWT secret
        let jwt_secret = load_or_generate_jwt_secret(&config.auth.jwt_secret_path)?;
        info!("JWT secret loaded from: {}", config.auth.jwt_secret_path.display());

        // Initialize database
        let database = Database::new(&config.database.url, encryption_key.to_vec()).await?;
        info!("Database initialized successfully");

        // Initialize authentication manager
        let auth_manager = AuthManager::new(jwt_secret.to_vec(), config.auth.jwt_expiry_hours as i64);
        info!("Authentication manager initialized");

        // Initialize health checker
        let health_checker = HealthChecker::new(database.clone());
        info!("Health checker initialized");

        // Create and run multi-tenant server with health checks
        let server = MultiTenantMcpServer::new(database, auth_manager);
        
        info!("🚀 Multi-tenant MCP server starting on ports {} (MCP) and {} (HTTP)", 
              config.mcp_port, config.http_port);
        info!("📊 Ready to serve fitness data with user authentication!");
        
        // Run server with health check integration
        if let Err(e) = run_production_server(server, config, health_checker).await {
            error!("Server error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Run the production server with health checks and graceful shutdown
async fn run_production_server(
    server: MultiTenantMcpServer,
    config: ServerConfig,
    health_checker: HealthChecker,
) -> Result<()> {
    
    // Setup HTTP routes with health checks  
    let routes = pierre_mcp_server::health::middleware::routes(health_checker);
    
    // Run HTTP server and MCP server concurrently
    let http_server = warp::serve(routes)
        .run(([0, 0, 0, 0], config.http_port));
    
    let mcp_server = server.run(config.mcp_port);
    
    // Wait for either server to complete (or fail)
    tokio::select! {
        _result = http_server => {
            info!("HTTP server completed");
            Ok(())
        }
        result = mcp_server => {
            if let Err(ref e) = result {
                error!("MCP server error: {}", e);
            }
            result
        }
    }
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
    // Create parent directory if it doesn't exist
    if let Some(parent) = secret_file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
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