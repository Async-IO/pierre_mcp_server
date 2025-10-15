// ABOUTME: Server implementation for serving users with isolated data access
// ABOUTME: Production-ready server with authentication and user isolation capabilities
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![recursion_limit = "256"]
#![forbid(unsafe_code)]

//! # Pierre Fitness API Server Binary
//!
//! This binary starts the multi-protocol Pierre Fitness API with user authentication,
//! secure token storage, and database management.

use anyhow::Result;
use clap::Parser;
use pierre_mcp_server::{
    auth::AuthManager,
    cache::factory::Cache,
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
    let args = parse_args_or_default();
    let config = setup_configuration(&args)?;
    bootstrap_server(config).await
}

/// Parse command line arguments or use defaults on failure
fn parse_args_or_default() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Argument parsing failed: {e}");
            eprintln!("Using default configuration for production mode");
            Args {
                config: None,
                http_port: None,
            }
        }
    }
}

/// Setup server configuration from environment and arguments
fn setup_configuration(args: &Args) -> Result<ServerConfig> {
    let mut config = ServerConfig::from_env()?;

    if let Some(http_port) = args.http_port {
        config.http_port = http_port;
    }

    logging::init_from_env()?;
    info!("Starting Pierre Fitness API - Production Mode");
    info!("{}", config.summary());

    Ok(config)
}

/// Bootstrap the complete server with all dependencies
async fn bootstrap_server(config: ServerConfig) -> Result<()> {
    let (database, auth_manager, jwt_secret) = initialize_core_systems(&config).await?;

    // Initialize cache from environment
    let cache = Cache::from_env().await?;
    info!("Cache initialized successfully");

    let server = create_server(database, auth_manager, &jwt_secret, &config, cache);
    run_server(server, &config).await
}

/// Initialize core systems (key management, database, auth)
async fn initialize_core_systems(config: &ServerConfig) -> Result<(Database, AuthManager, String)> {
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

    info!("Admin JWT secret ready for secure token generation");
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

    Ok((database, auth_manager, jwt_secret_string))
}

/// Create server instance with all resources
fn create_server(
    database: Database,
    auth_manager: AuthManager,
    jwt_secret: &str,
    config: &ServerConfig,
    cache: Cache,
) -> MultiTenantMcpServer {
    let resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        jwt_secret,
        Arc::new(config.clone()),
        cache,
    ));
    MultiTenantMcpServer::new(resources)
}

/// Run the server after displaying endpoints
async fn run_server(server: MultiTenantMcpServer, config: &ServerConfig) -> Result<()> {
    info!(
        "Server starting on port {} (unified MCP and HTTP)",
        config.http_port
    );
    display_available_endpoints(config);
    info!("Ready to serve fitness data!");

    if let Err(e) = server.run(config.http_port).await {
        error!("Server error: {}", e);
        return Err(e);
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
    display_oauth_callback_urls(&host, config);
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

/// Endpoint category definition for structured display
struct EndpointCategory {
    name: &'static str,
    endpoints: &'static [(&'static str, &'static str, &'static str)], // (description, method, path)
}

/// Display a category of endpoints with consistent formatting
fn display_endpoint_category(category: &EndpointCategory, host: &str, port: u16) {
    info!("{}", category.name);
    for (description, method, path) in category.endpoints {
        info!("   {description:18} {method} http://{host}:{port}{path}");
    }
}

fn display_mcp_endpoints(host: &str, port: u16) {
    let endpoints = [
        "MCP Protocol:",
        &format!("   HTTP Transport:    http://{host}:{port}/mcp"),
        &format!("   WebSocket:         ws://{host}:{port}/mcp/ws"),
        &format!("   Server-Sent Events: http://{host}:{port}/mcp/sse"),
    ];
    for line in &endpoints {
        info!("{}", line);
    }
}

fn display_auth_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Authentication & OAuth:",
        endpoints: &[
            ("User Registration:", "POST", "/auth/register"),
            ("User Login:", "POST", "/auth/login"),
            ("OAuth Authorize:", "GET", "/oauth/authorize/{provider}"),
            ("OAuth Callback:", "GET", "/oauth/callback/{provider}"),
            ("OAuth Status:", "GET", "/oauth/status"),
            ("OAuth Disconnect:", "POST", "/oauth/disconnect/{provider}"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_oauth2_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "OAuth 2.0 Server:",
        endpoints: &[
            ("Authorization:", "GET", "/oauth2/authorize"),
            ("Token Exchange:", "POST", "/oauth2/token"),
            ("Client Registration:", "POST", "/oauth2/register"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_oauth_callback_urls(_host: &str, config: &ServerConfig) {
    let endpoints = [
        "OAuth Callback URLs (MCP Bridge):",
        &format!(
            "   Bridge Callback:   http://localhost:{}/oauth/callback",
            config.oauth_callback_port
        ),
        &format!(
            "   Focus Recovery:    http://localhost:{}/oauth/focus-recovery",
            config.oauth_callback_port
        ),
        &format!(
            "   Provider Callback: http://localhost:{}/oauth/provider-callback/{{provider}}",
            config.oauth_callback_port
        ),
    ];
    for line in &endpoints {
        info!("{}", line);
    }
}

fn display_admin_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Admin Management:",
        endpoints: &[
            ("Admin Setup:", "POST", "/admin/setup"),
            ("Create User:", "POST", "/admin/users"),
            ("List Users:", "GET", "/admin/users"),
            ("Generate Token:", "POST", "/admin/tokens"),
            ("List Tokens:", "GET", "/admin/tokens"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_api_key_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "API Key Management:",
        endpoints: &[
            ("Create API Key:", "POST", "/api/keys"),
            ("List API Keys:", "GET", "/api/keys"),
            ("Delete API Key:", "DELETE", "/api/keys/{key_id}"),
            ("API Key Usage:", "GET", "/api/keys/usage"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_tenant_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Tenant Management:",
        endpoints: &[
            ("Create Tenant:", "POST", "/tenants"),
            ("List Tenants:", "GET", "/tenants"),
            ("Get Tenant:", "GET", "/tenants/{tenant_id}"),
            ("Update Tenant:", "PUT", "/tenants/{tenant_id}"),
            ("Delete Tenant:", "DELETE", "/tenants/{tenant_id}"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_dashboard_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Dashboard & Monitoring:",
        endpoints: &[
            ("Health Check:", "GET", "/health"),
            ("System Status:", "GET", "/dashboard/status"),
            ("User Dashboard:", "GET", "/dashboard/user"),
            ("Admin Dashboard:", "GET", "/dashboard/admin"),
            ("Detailed Stats:", "GET", "/dashboard/detailed"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_a2a_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "A2A Protocol:",
        endpoints: &[
            ("A2A Status:", "GET", "/a2a/status"),
            ("A2A Tools:", "GET", "/a2a/tools"),
            ("A2A Execute:", "POST", "/a2a/execute"),
            ("A2A Monitoring:", "GET", "/a2a/monitoring"),
            ("Client Tools:", "GET", "/a2a/client/tools"),
            ("Client Execute:", "POST", "/a2a/client/execute"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_config_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Configuration:",
        endpoints: &[
            ("Get Config:", "GET", "/config"),
            ("Update Config:", "PUT", "/config"),
            ("User Config:", "GET", "/config/user"),
            ("Update User Config:", "PUT", "/config/user"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_fitness_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Fitness Configuration:",
        endpoints: &[
            ("Get Fitness Config:", "GET", "/fitness/config"),
            ("Update Fitness Config:", "PUT", "/fitness/config"),
            ("Delete Fitness Config:", "DELETE", "/fitness/config"),
        ],
    };
    display_endpoint_category(&category, host, port);
}

fn display_notification_endpoints(host: &str, port: u16) {
    let category = EndpointCategory {
        name: "Real-time Notifications:",
        endpoints: &[("SSE Stream:", "GET", "/notifications/sse?user_id={user_id}")],
    };
    display_endpoint_category(&category, host, port);
}
