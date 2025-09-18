// ABOUTME: Server implementation for serving users with isolated data access
// ABOUTME: Production-ready server with authentication and user isolation capabilities
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![recursion_limit = "256"]

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
    mcp::{multitenant::MultiTenantMcpServer, resources::ServerResources},
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
                http_port: None,
            }
        }
    };

    {
        // Load configuration from environment
        let mut config = ServerConfig::from_env()?;

        // Override port if specified
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

        // Get or create JWT secret from database (for server-first bootstrap)
        let jwt_secret_string = database
            .get_or_create_system_secret("admin_jwt_secret")
            .await?;

        info!(
            "Admin JWT secret ready (first 10 chars): {}...",
            jwt_secret_string.chars().take(10).collect::<String>()
        );
        info!("Server is ready for admin setup via POST /admin/setup");

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

        // Create server resources and server
        let resources = Arc::new(ServerResources::new(
            database,
            auth_manager,
            &jwt_secret_string,
            Arc::new(config.clone()),
        ));
        let server = MultiTenantMcpServer::new(resources);

        info!(
            "Server starting on port {} (unified MCP and HTTP)",
            config.http_port
        );

        // Display all available API endpoints
        display_available_endpoints(&config);

        info!("Ready to serve fitness data!");

        // Run the server (includes all routes)
        if let Err(e) = server.run(config.http_port).await {
            error!("Server error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Display all available API endpoints with their ports
fn display_available_endpoints(config: &ServerConfig) {
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    info!("=== Available API Endpoints ===");
    display_mcp_endpoints(&host, config.http_port);
    display_auth_endpoints(&host, config.http_port);
    display_oauth2_endpoints(&host, config.http_port);
    display_admin_endpoints(&host, config.http_port);
    display_api_key_endpoints(&host, config.http_port);
    display_tenant_endpoints(&host, config.http_port);
    display_dashboard_endpoints(&host, config.http_port);
    display_a2a_endpoints(&host, config.http_port);
    display_config_endpoints(&host, config.http_port);
    display_fitness_endpoints(&host, config.http_port);
    display_notification_endpoints(&host, config.http_port);
    info!("=== End of Endpoint List ===");
}

#[allow(clippy::cognitive_complexity)]
fn display_mcp_endpoints(host: &str, port: u16) {
    info!("MCP Protocol:");
    info!("   HTTP Transport:  http://{host}:{port}/mcp");
    info!("   WebSocket:       ws://{host}:{port}/mcp/ws");
    info!("   Server-Sent Events: http://{host}:{port}/mcp/sse");
}

#[allow(clippy::cognitive_complexity)]
fn display_auth_endpoints(host: &str, port: u16) {
    info!("Authentication & OAuth:");
    info!("   User Registration: POST http://{host}:{port}/auth/register");
    info!("   User Login:        POST http://{host}:{port}/auth/login");
    info!("   OAuth Authorize:   GET  http://{host}:{port}/oauth/authorize/{{provider}}");
    info!("   OAuth Callback:    GET  http://{host}:{port}/oauth/callback/{{provider}}");
    info!("   OAuth Status:      GET  http://{host}:{port}/oauth/status");
    info!("   OAuth Disconnect:  POST http://{host}:{port}/oauth/disconnect/{{provider}}");
}

#[allow(clippy::cognitive_complexity)]
fn display_oauth2_endpoints(host: &str, port: u16) {
    info!("OAuth 2.0 Server:");
    info!("   Authorization:     GET  http://{host}:{port}/oauth2/authorize");
    info!("   Token Exchange:    POST http://{host}:{port}/oauth2/token");
    info!("   Client Registration: POST http://{host}:{port}/oauth2/register");
}

#[allow(clippy::cognitive_complexity)]
fn display_admin_endpoints(host: &str, port: u16) {
    info!("Admin Management:");
    info!("   Admin Setup:       POST http://{host}:{port}/admin/setup");
    info!("   Create User:       POST http://{host}:{port}/admin/users");
    info!("   List Users:        GET  http://{host}:{port}/admin/users");
    info!("   Generate Token:    POST http://{host}:{port}/admin/tokens");
    info!("   List Tokens:       GET  http://{host}:{port}/admin/tokens");
}

#[allow(clippy::cognitive_complexity)]
fn display_api_key_endpoints(host: &str, port: u16) {
    info!("API Key Management:");
    info!("   Create API Key:    POST http://{host}:{port}/api/keys");
    info!("   List API Keys:     GET  http://{host}:{port}/api/keys");
    info!("   Delete API Key:    DELETE http://{host}:{port}/api/keys/{{key_id}}");
    info!("   API Key Usage:     GET  http://{host}:{port}/api/keys/usage");
}

#[allow(clippy::cognitive_complexity)]
fn display_tenant_endpoints(host: &str, port: u16) {
    info!("Tenant Management:");
    info!("   Create Tenant:     POST http://{host}:{port}/tenants");
    info!("   List Tenants:      GET  http://{host}:{port}/tenants");
    info!("   Get Tenant:        GET  http://{host}:{port}/tenants/{{tenant_id}}");
    info!("   Update Tenant:     PUT  http://{host}:{port}/tenants/{{tenant_id}}");
    info!("   Delete Tenant:     DELETE http://{host}:{port}/tenants/{{tenant_id}}");
}

#[allow(clippy::cognitive_complexity)]
fn display_dashboard_endpoints(host: &str, port: u16) {
    info!("Dashboard & Monitoring:");
    info!("   Health Check:      GET  http://{host}:{port}/health");
    info!("   System Status:     GET  http://{host}:{port}/dashboard/status");
    info!("   User Dashboard:    GET  http://{host}:{port}/dashboard/user");
    info!("   Admin Dashboard:   GET  http://{host}:{port}/dashboard/admin");
    info!("   Detailed Stats:    GET  http://{host}:{port}/dashboard/detailed");
}

#[allow(clippy::cognitive_complexity)]
fn display_a2a_endpoints(host: &str, port: u16) {
    info!("A2A Protocol:");
    info!("   A2A Status:        GET  http://{host}:{port}/a2a/status");
    info!("   A2A Tools:         GET  http://{host}:{port}/a2a/tools");
    info!("   A2A Execute:       POST http://{host}:{port}/a2a/execute");
    info!("   A2A Monitoring:    GET  http://{host}:{port}/a2a/monitoring");
    info!("   Client Tools:      GET  http://{host}:{port}/a2a/client/tools");
    info!("   Client Execute:    POST http://{host}:{port}/a2a/client/execute");
}

#[allow(clippy::cognitive_complexity)]
fn display_config_endpoints(host: &str, port: u16) {
    info!("Configuration:");
    info!("   Get Config:        GET  http://{host}:{port}/config");
    info!("   Update Config:     PUT  http://{host}:{port}/config");
    info!("   User Config:       GET  http://{host}:{port}/config/user");
    info!("   Update User Config: PUT  http://{host}:{port}/config/user");
}

#[allow(clippy::cognitive_complexity)]
fn display_fitness_endpoints(host: &str, port: u16) {
    info!("Fitness Configuration:");
    info!("   Get Fitness Config:    GET  http://{host}:{port}/fitness/config");
    info!("   Update Fitness Config: PUT  http://{host}:{port}/fitness/config");
    info!("   Delete Fitness Config: DELETE http://{host}:{port}/fitness/config");
}

#[allow(clippy::cognitive_complexity)]
fn display_notification_endpoints(host: &str, port: u16) {
    info!("Real-time Notifications:");
    info!("   SSE Stream:        GET  http://{host}:{port}/notifications/sse?user_id={{user_id}}");
}
