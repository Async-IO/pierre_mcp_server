// ABOUTME: Server implementation for serving users with isolated data access
// ABOUTME: Production-ready server with authentication and user isolation capabilities
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![recursion_limit = "256"]
#![deny(unsafe_code)]

//! # Pierre Fitness API Server Binary
//!
//! This binary starts the multi-protocol Pierre Fitness API with user authentication,
//! secure token storage, and database management.

use anyhow::Result;
use clap::{error::ErrorKind, Parser};
#[cfg(feature = "provider-synthetic")]
use pierre_mcp_server::providers::set_synthetic_database_pool;
use pierre_mcp_server::{
    auth::AuthManager,
    cache::factory::Cache,
    config::environment::{ServerConfig, TokioRuntimeConfig},
    constants::init_server_config,
    database_plugins::{factory::Database, DatabaseProvider},
    errors::AppError,
    features::FeatureConfig,
    key_management::KeyManager,
    logging,
    mcp::{
        multitenant::MultiTenantMcpServer, resources::ServerResources,
        transport_manager::TransportManager,
    },
    plugins::executor::PluginToolExecutor,
    utils::{http_client::initialize_http_clients, route_timeout::initialize_route_timeouts},
};
use std::{env, sync::Arc};
use tokio::runtime::{Builder, Runtime};
use tracing::{error, info};

/// Command-line arguments for the Pierre MCP server
#[derive(Parser)]
#[command(name = "pierre-mcp-server")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "Pierre Fitness API - Multi-protocol fitness data API for LLMs")]
pub struct Args {
    /// Configuration file path for providers
    #[arg(short, long)]
    config: Option<String>,

    /// Override HTTP port
    #[arg(long)]
    http_port: Option<u16>,

    /// Run in stdio-only mode (disables HTTP/SSE transports)
    #[arg(long)]
    stdio: bool,
}

fn main() -> Result<()> {
    let args = parse_args_or_default();

    // Load runtime config first to build the Tokio runtime
    let runtime_config = TokioRuntimeConfig::from_env();
    let runtime = build_tokio_runtime(&runtime_config)?;

    // Run the async server on our configured runtime
    runtime.block_on(async {
        let config = setup_configuration(&args)?;
        bootstrap_server(config, args.stdio).await
    })
}

/// Build a Tokio runtime with configurable worker threads
fn build_tokio_runtime(config: &TokioRuntimeConfig) -> Result<Runtime> {
    let mut builder = Builder::new_multi_thread();

    // Configure worker threads if specified
    if let Some(workers) = config.worker_threads {
        builder.worker_threads(workers);
        eprintln!("Tokio runtime: {workers} worker threads (from TOKIO_WORKER_THREADS)");
    }

    // Configure thread stack size if specified
    if let Some(stack_size) = config.thread_stack_size {
        builder.thread_stack_size(stack_size);
        let stack_kb = stack_size / 1024;
        eprintln!("Tokio runtime: {stack_kb}KB thread stack (from TOKIO_THREAD_STACK_SIZE)");
    }

    // Configure thread naming
    builder.thread_name(&config.thread_name);

    // Enable drivers
    if config.enable_io {
        builder.enable_io();
    }
    if config.enable_time {
        builder.enable_time();
    }

    builder
        .build()
        .map_err(|e| AppError::internal(format!("Failed to build Tokio runtime: {e}")).into())
}

/// Parse command line arguments or use defaults on failure
fn parse_args_or_default() -> Args {
    match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            // Handle --version and --help specially - they should print and exit
            if e.kind() == ErrorKind::DisplayVersion || e.kind() == ErrorKind::DisplayHelp {
                e.exit();
            }
            // For actual errors, use defaults
            eprintln!("Argument parsing failed: {e}");
            eprintln!("Using default configuration for production mode");
            Args {
                config: None,
                http_port: None,
                stdio: false,
            }
        }
    }
}

/// Setup server configuration from environment and arguments
fn setup_configuration(args: &Args) -> Result<ServerConfig> {
    // Validate required environment variables before loading config
    validate_required_environment()?;

    let mut config = ServerConfig::from_env()?;

    if let Some(http_port) = args.http_port {
        config.http_port = http_port;
    }

    logging::init_from_env()?;
    info!("Starting Pierre Fitness API - Production Mode");
    info!("{}", config.summary());

    validate_oauth_providers(&config);

    Ok(config)
}

/// Environment variable validation result
struct EnvValidation {
    name: &'static str,
    value: Option<String>,
    required: bool,
    description: &'static str,
}

/// Validate required environment variables at startup
///
/// Fails fast with clear error messages if critical variables are missing.
/// This prevents cryptic errors later in the startup process.
fn validate_required_environment() -> Result<()> {
    let validations = vec![
        EnvValidation {
            name: "DATABASE_URL",
            value: env::var("DATABASE_URL").ok(),
            required: true,
            description: "Database connection string (e.g., sqlite:./data/users.db)",
        },
        EnvValidation {
            name: "PIERRE_MASTER_ENCRYPTION_KEY",
            value: env::var("PIERRE_MASTER_ENCRYPTION_KEY").ok(),
            required: true, // Required for multi-instance deployments - prevents data corruption
            description:
                "Base64-encoded 32-byte master encryption key (generate: openssl rand -base64 32)",
        },
        EnvValidation {
            name: "STRAVA_CLIENT_ID",
            value: env::var("STRAVA_CLIENT_ID").ok(),
            required: false,
            description: "Strava OAuth application client ID",
        },
        EnvValidation {
            name: "STRAVA_CLIENT_SECRET",
            value: env::var("STRAVA_CLIENT_SECRET").ok(),
            required: false,
            description: "Strava OAuth application client secret",
        },
    ];

    let mut missing_required = Vec::new();
    let mut missing_optional = Vec::new();

    for validation in &validations {
        if validation.value.is_none() {
            if validation.required {
                missing_required.push(validation);
            } else {
                missing_optional.push(validation);
            }
        }
    }

    // Print warnings for missing optional variables
    for validation in &missing_optional {
        eprintln!(
            "WARNING: {} not set - {}",
            validation.name, validation.description
        );
    }

    // Fail fast if required variables are missing
    if !missing_required.is_empty() {
        eprintln!();
        eprintln!("ERROR: Required environment variables are missing!");
        eprintln!();
        for validation in &missing_required {
            eprintln!("  {} - {}", validation.name, validation.description);
        }
        eprintln!();
        eprintln!("Have you sourced your .envrc? Run: source .envrc");
        eprintln!();
        let missing_names = missing_required
            .iter()
            .map(|v| v.name)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(AppError::config(format!(
            "Missing required environment variables: {missing_names}"
        ))
        .into());
    }

    Ok(())
}

/// Validate OAuth provider credentials at startup
fn validate_oauth_providers(config: &ServerConfig) {
    info!("Validating OAuth provider credentials...");
    let all_valid = validate_all_providers(config);
    log_validation_result(all_valid);
}

/// Validate all OAuth providers and return combined result
fn validate_all_providers(config: &ServerConfig) -> bool {
    let strava_valid = config.oauth.strava.validate_and_log("strava");
    let fitbit_valid = config.oauth.fitbit.validate_and_log("fitbit");
    let garmin_valid = config.oauth.garmin.validate_and_log("garmin");
    strava_valid && fitbit_valid && garmin_valid
}

/// Log OAuth validation result
fn log_validation_result(all_valid: bool) {
    if all_valid {
        info!("OAuth credential validation passed for all enabled providers");
    } else {
        error!("OAuth credential validation failed - check client_id/client_secret configuration");
        error!("Hint: Compare secret_fingerprint with known good values to detect mismatches");
    }
}

/// Bootstrap the complete server with all dependencies
async fn bootstrap_server(config: ServerConfig, stdio_only: bool) -> Result<()> {
    // Validate feature configuration before any expensive initialization
    // The validate() function logs all enabled features
    FeatureConfig::validate()?;

    initialize_global_configs(&config)?;
    let (database, auth_manager, jwt_secret) = initialize_core_systems(&config).await?;
    let cache = initialize_cache().await?;
    let server = create_server(database, auth_manager, &jwt_secret, &config, cache).await;
    run_server(server, &config, stdio_only).await
}

fn initialize_global_configs(config: &ServerConfig) -> Result<()> {
    initialize_http_clients(config.http_client.clone());
    info!("HTTP client configuration initialized");

    initialize_route_timeouts(config.route_timeouts.clone());
    info!("Route timeout configuration initialized");

    init_server_config()?;
    info!("Static server configuration initialized");

    Ok(())
}

async fn initialize_cache() -> Result<Cache> {
    let cache = Cache::from_env().await?;
    info!("Cache initialized successfully");
    Ok(cache)
}

/// Initialize core systems (key management, database, auth)
async fn initialize_core_systems(config: &ServerConfig) -> Result<(Database, AuthManager, String)> {
    let (mut key_manager, database_encryption_key) = bootstrap_key_management()?;
    let mut database = initialize_database(config, database_encryption_key).await?;
    key_manager.complete_initialization(&mut database).await?;
    info!("Two-tier key management system fully initialized");

    let jwt_secret_string = initialize_jwt_secret(&database, config).await?;
    let auth_manager = create_auth_manager(config);

    Ok((database, auth_manager, jwt_secret_string))
}

fn bootstrap_key_management() -> Result<(KeyManager, [u8; 32])> {
    let (key_manager, database_encryption_key) = KeyManager::bootstrap()?;
    info!("Two-tier key management system bootstrapped");
    Ok((key_manager, database_encryption_key))
}

async fn initialize_database(
    config: &ServerConfig,
    database_encryption_key: [u8; 32],
) -> Result<Database> {
    let database = Database::new(
        &config.database.url.to_connection_string(),
        database_encryption_key.to_vec(),
        #[cfg(feature = "postgresql")]
        &config.database.postgres_pool,
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
    Ok(database)
}

/// Log startup warnings when no admin user is configured
fn log_missing_admin_warning(auto_approve: bool) {
    let oauth_behavior = if auto_approve {
        "auto-approve"
    } else {
        "pending"
    };
    error!(
        "No admin user configured! Email/password login unavailable. \
         Firebase/OAuth creates {oauth_behavior} users. Fix: cargo run --bin pierre-cli -- user create --email admin@example.com --password <password>"
    );
}

/// Check admin user status and log appropriate startup message
async fn check_admin_status(database: &Database, auto_approve: bool) {
    match database.get_users_by_status("active").await {
        Ok(users) => {
            let admin_exists = users.iter().any(|u| u.is_admin);
            if admin_exists {
                info!("Admin user configured - server ready for authentication");
            } else {
                log_missing_admin_warning(auto_approve);
            }
        }
        Err(e) => {
            error!("Failed to check admin user status: {e}");
        }
    }
}

async fn initialize_jwt_secret(database: &Database, config: &ServerConfig) -> Result<String> {
    let jwt_secret_string = database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;

    info!("Admin JWT secret ready for secure token generation");

    // Check auto-approval setting
    // Precedence: env var (if set) > database > default
    let auto_approve = if config.app_behavior.auto_approve_users_from_env {
        config.app_behavior.auto_approve_users
    } else {
        database
            .is_auto_approval_enabled()
            .await
            .ok()
            .flatten()
            .unwrap_or(config.app_behavior.auto_approve_users)
    };

    check_admin_status(database, auto_approve).await;

    Ok(jwt_secret_string)
}

fn create_auth_manager(config: &ServerConfig) -> AuthManager {
    #[allow(clippy::cast_possible_wrap)]
    {
        let auth_manager = AuthManager::new(config.auth.jwt_expiry_hours as i64);
        info!("Authentication manager initialized with RS256");
        auth_manager
    }
}

/// Create server instance with all resources
async fn create_server(
    database: Database,
    auth_manager: AuthManager,
    jwt_secret: &str,
    config: &ServerConfig,
    cache: Cache,
) -> MultiTenantMcpServer {
    let rsa_key_size = get_rsa_key_size();
    info!("Using {}-bit RSA keys for JWT signing", rsa_key_size);

    let mut resources_instance = ServerResources::new(
        database,
        auth_manager,
        jwt_secret,
        Arc::new(config.clone()),
        cache,
        rsa_key_size,
        None, // Generate new JWKS manager for production
    )
    .await;

    // Initialize synthetic provider database pool for non-OAuth activity access
    #[cfg(feature = "provider-synthetic")]
    if let Some(pool) = resources_instance.database.sqlite_pool() {
        set_synthetic_database_pool(Arc::new(pool.clone()));
        info!("Synthetic provider database pool initialized");
    }

    // Wrap in Arc for plugin executor initialization
    let resources_arc = Arc::new(resources_instance.clone());

    // Initialize plugin system with resources
    let plugin_executor = PluginToolExecutor::new(resources_arc);
    info!(
        "Plugin system initialized: {} core tools, {} plugin tools",
        plugin_executor.get_statistics().core_tools,
        plugin_executor.get_statistics().plugin_tools
    );

    // Set plugin executor back on resources
    resources_instance.set_plugin_executor(Arc::new(plugin_executor));

    // Use the updated resources instance
    let resources = Arc::new(resources_instance);
    MultiTenantMcpServer::new(resources)
}

/// Get RSA key size from environment or use production default
fn get_rsa_key_size() -> usize {
    env::var("PIERRE_RSA_KEY_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4096)
}

/// Run the server after displaying endpoints
async fn run_server(
    server: MultiTenantMcpServer,
    config: &ServerConfig,
    stdio_only: bool,
) -> Result<()> {
    if stdio_only {
        run_stdio_only_mode(server).await
    } else {
        run_http_mode(server, config).await
    }
}

/// Run server in stdio-only mode (no HTTP/SSE transports)
async fn run_stdio_only_mode(server: MultiTenantMcpServer) -> Result<()> {
    info!("Starting in stdio-only mode (HTTP/SSE transports disabled)");
    info!("Listening on stdin/stdout for MCP JSON-RPC messages");

    let transport_manager = TransportManager::new(server.resources());
    transport_manager.start_stdio_only().await.map_err(|e| {
        error!("Stdio transport error: {}", e);
        e
    })?;

    Ok(())
}

/// Run server in HTTP mode with all transports
async fn run_http_mode(server: MultiTenantMcpServer, config: &ServerConfig) -> Result<()> {
    info!(
        "Server starting on port {} (unified MCP and HTTP)",
        config.http_port
    );
    display_available_endpoints(config);
    info!("Ready to serve fitness data!");

    server.run(config.http_port).await.map_err(|e| {
        error!("Server error: {}", e);
        e
    })?;

    Ok(())
}

/// Display all available API endpoints with their ports
fn display_available_endpoints(config: &ServerConfig) {
    // Default to 127.0.0.1 for local development - production uses reverse proxy
    let host = "127.0.0.1";

    info!("=== Available API Endpoints ===");
    display_mcp_endpoints(host, config.http_port);
    display_auth_endpoints(host, config.http_port);
    display_oauth2_endpoints(host, config.http_port);
    display_oauth_callback_urls(host, config);
    display_admin_endpoints(host, config.http_port);
    display_api_key_endpoints(host, config.http_port);
    display_tenant_endpoints(host, config.http_port);
    display_dashboard_endpoints(host, config.http_port);
    display_a2a_endpoints(host, config.http_port);
    display_config_endpoints(host, config.http_port);
    display_fitness_endpoints(host, config.http_port);
    display_notification_endpoints(host, config.http_port);
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
            ("OAuth Authorize:", "GET", "/api/oauth/authorize/{provider}"),
            ("OAuth Callback:", "GET", "/api/oauth/callback/{provider}"),
            ("OAuth Status:", "GET", "/api/oauth/status"),
            (
                "OAuth Disconnect:",
                "POST",
                "/api/oauth/disconnect/{provider}",
            ),
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
            ("Plugin Status:", "GET", "/health/plugins"),
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
