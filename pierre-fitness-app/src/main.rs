// ABOUTME: Pierre Fitness Server - Main binary entry point
// ABOUTME: Configures and runs the fitness intelligence server with all providers

#![recursion_limit = "256"]
#![deny(unsafe_code)]

//! # Pierre Fitness Server Binary
//!
//! This binary starts the complete fitness intelligence server with:
//! - Framework infrastructure (authentication, database, MCP protocol)
//! - Fitness-specific intelligence and handlers
//! - All fitness data providers (Strava, Garmin, Synthetic)

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
use tracing::{error, info, warn};

/// Command-line arguments for the Pierre Fitness Server
#[derive(Parser)]
#[command(name = "pierre-fitness-server")]
#[command(about = "Pierre Fitness Intelligence Server - Complete fitness platform with all providers")]
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
    bootstrap_fitness_server(config).await
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
    info!("Starting Pierre Fitness Intelligence Server");
    info!("{}", config.summary());

    validate_oauth_providers(&config);

    Ok(config)
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
        error!("Some OAuth providers have missing or invalid credentials");
        error!("Provider connections may not work until credentials are configured");
    }
}

/// Bootstrap and run the fitness server with all providers
async fn bootstrap_fitness_server(config: ServerConfig) -> Result<()> {
    // Initialize HTTP client configuration
    pierre_mcp_server::utils::http_client::initialize_http_clients(config.http_client.clone());
    info!("HTTP client configuration initialized");

    // Initialize route timeout configuration
    pierre_mcp_server::utils::route_timeout::initialize_route_timeouts(
        config.route_timeouts.clone(),
    );
    info!("Route timeout configuration initialized");

    // Initialize static server configuration
    pierre_mcp_server::constants::init_server_config()?;
    info!("Static server configuration initialized");

    let (database, auth_manager, jwt_secret) = initialize_core_systems(&config).await?;

    // Initialize cache from environment
    let cache = Cache::from_env().await?;
    info!("Cache initialized successfully");

    // Register fitness providers with the registry
    register_fitness_providers()?;

    let server = create_server(database, auth_manager, &jwt_secret, &config, cache);
    run_server(server, config).await
}

/// Register all fitness providers from pierre-fitness-providers
fn register_fitness_providers() -> Result<()> {
    info!("Initializing fitness data providers...");

    // NOTE: Provider descriptors are auto-registered via feature flags during ProviderRegistry::new()
    // The actual provider factory implementations are in pierre-fitness-providers
    // TODO: Register actual provider factories from pierre-fitness-providers when plugin system is ready

    let mut provider_count = 0;

    #[cfg(feature = "provider-strava")]
    {
        info!("  ✓ Strava provider available");
        provider_count += 1;
    }

    #[cfg(feature = "provider-garmin")]
    {
        info!("  ✓ Garmin provider available");
        provider_count += 1;
    }

    #[cfg(feature = "provider-synthetic")]
    {
        info!("  ✓ Synthetic provider available");
        provider_count += 1;
    }

    if provider_count == 0 {
        warn!("No fitness providers enabled - check feature flags");
        warn!("Enable providers with: --features provider-strava,provider-garmin,provider-synthetic");
    } else {
        info!(
            "Fitness providers initialized: {} provider(s) available",
            provider_count
        );
    }

    Ok(())
}

/// Initialize core systems (key management, database, auth)
async fn initialize_core_systems(
    config: &ServerConfig,
) -> Result<(Database, AuthManager, String)> {
    // Initialize two-tier key management system
    let (mut key_manager, database_encryption_key) =
        pierre_mcp_server::key_management::KeyManager::bootstrap()?;
    info!("Two-tier key management system bootstrapped");

    // Initialize database with DEK from key manager
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

    // Complete key manager initialization with database
    key_manager.complete_initialization(&database).await?;
    info!("Two-tier key management system fully initialized");

    // Get or create JWT secret from database (for server-first bootstrap)
    let jwt_secret_string = database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;
    info!("Admin JWT secret ready for secure token generation");

    // Initialize authentication manager with RS256 (no HS256 secret needed)
    let auth_manager = {
        // Safe: JWT expiry hours are small positive configuration values (1-168)
        #[allow(clippy::cast_possible_wrap)]
        {
            AuthManager::new(config.auth.jwt_expiry_hours as i64)
        }
    };
    info!("Authentication manager initialized with RS256");

    Ok((database, auth_manager, jwt_secret_string))
}

/// Create and configure the server with all resources
fn create_server(
    database: Database,
    auth_manager: AuthManager,
    jwt_secret: &str,
    config: &ServerConfig,
    cache: Cache,
) -> MultiTenantMcpServer {
    let server_resources = ServerResources::new(
        database,
        auth_manager,
        jwt_secret,
        Arc::new(config.clone()),
        cache,
        4096, // RSA key size for production
        None, // Let ServerResources create JWKS manager
    );

    MultiTenantMcpServer::new(Arc::new(server_resources))
}

/// Run the server and handle shutdown
async fn run_server(server: MultiTenantMcpServer, config: ServerConfig) -> Result<()> {
    info!(
        "Server starting on port {} (unified MCP and HTTP)",
        config.http_port
    );
    info!("Pierre Fitness Intelligence Server ready!");
    info!("  MCP Protocol: http://127.0.0.1:{}/mcp", config.http_port);
    info!("  HTTP API: http://127.0.0.1:{}", config.http_port);
    info!("Ready to serve fitness data with intelligence!");

    server.run(config.http_port).await.map_err(|e| {
        error!("Server error: {}", e);
        e
    })?;

    Ok(())
}
