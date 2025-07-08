// ABOUTME: Multi-tenant server implementation for serving multiple users with isolated data access
// ABOUTME: Production-ready server with authentication, user isolation, and tenant management capabilities
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
    constants::{env_config, network_config::HTTP_PORT_OFFSET},
    database::generate_encryption_key,
    database_plugins::factory::Database,
    health::HealthChecker,
    logging,
    mcp::multitenant::MultiTenantMcpServer,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{error, info};
use warp::Filter;

#[derive(Parser)]
#[command(name = "pierre-mcp-server")]
#[command(about = "Pierre Fitness API - Multi-protocol fitness data API for LLMs")]
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
            eprintln!("Argument parsing failed: {e}");
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

        info!("Starting Pierre Fitness API - Single-Tenant Mode");

        // In single-tenant mode, use the original server with OAuth support
        let config = pierre_mcp_server::config::fitness_config::FitnessConfig::load(args.config)?;
        let server = pierre_mcp_server::mcp::McpServer::new(config);

        let mcp_port = args.mcp_port.unwrap_or_else(env_config::mcp_port);
        let http_port = args
            .http_port
            .unwrap_or_else(|| env_config::mcp_port() + HTTP_PORT_OFFSET);

        info!(
            "🚀 Single-tenant MCP server starting on port {} (MCP) and {} (HTTP)",
            mcp_port, http_port
        );
        info!("📊 Ready to serve fitness data with OAuth support!");

        // Run both MCP server and OAuth HTTP server concurrently
        if let Err(e) = run_single_tenant_server(server, mcp_port, http_port).await {
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

        info!("🚀 Starting Pierre Fitness API - Production Mode");
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
            #[allow(clippy::cast_possible_wrap)]
            {
                AuthManager::new(jwt_secret.to_vec(), config.auth.jwt_expiry_hours as i64)
            }
        };
        info!("Authentication manager initialized");

        // Initialize health checker
        let health_checker = HealthChecker::new(database.clone());
        info!("Health checker initialized");

        // Create and run multi-tenant server with health checks
        let server = MultiTenantMcpServer::new(database, auth_manager, Arc::new(config.clone()));

        info!(
            "🚀 Multi-tenant MCP server starting on ports {} (MCP) and {} (HTTP)",
            config.mcp_port, config.http_port
        );
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
    // Load admin JWT secret for admin API authentication
    let admin_jwt_secret = load_or_generate_admin_jwt_secret(&config.auth.jwt_secret_path)?;
    let admin_jwt_secret_str = String::from_utf8(admin_jwt_secret.to_vec())
        .unwrap_or_else(|_| "fallback_admin_secret".into());

    // Setup HTTP routes with health checks and admin API
    let health_routes = pierre_mcp_server::health::middleware::routes(health_checker);

    // Setup admin API routes
    let admin_context = pierre_mcp_server::admin_routes::AdminApiContext::new(
        server.database().clone(),
        &admin_jwt_secret_str,
        server.auth_manager().clone(),
    );
    let admin_routes = pierre_mcp_server::admin_routes::admin_routes(admin_context);

    // Combine all routes
    let routes = health_routes.or(admin_routes);

    info!("🔧 Admin API enabled at /admin/* endpoints");
    info!("📋 Available admin endpoints:");
    info!("  POST /admin/provision-api-key - Provision API keys for users");
    info!("  POST /admin/revoke-api-key - Revoke existing API keys");
    info!("  GET  /admin/list-api-keys - List API keys (with filters)");
    info!("  GET  /admin/token-info - Get admin token information");
    info!("  GET  /admin/health - Admin API health check");
    info!("  GET  /admin/setup-status - Check if admin user exists");
    info!("  GET  /admin/tokens - List admin tokens");
    info!("  POST /admin/tokens - Create admin token");
    info!("  GET  /admin/tokens/{{id}} - Get admin token details");
    info!("  POST /admin/tokens/{{id}}/revoke - Revoke admin token");
    info!("  POST /admin/tokens/{{id}}/rotate - Rotate admin token");

    // Run HTTP server and MCP server concurrently
    let http_server = warp::serve(routes).run(([0, 0, 0, 0], config.http_port));

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

/// Run the single-tenant server with OAuth callback support
async fn run_single_tenant_server(
    server: pierre_mcp_server::mcp::McpServer,
    mcp_port: u16,
    http_port: u16,
) -> Result<()> {
    // Setup OAuth callback routes for single-tenant mode
    let oauth_routes = setup_single_tenant_oauth_routes();

    // Run HTTP server for OAuth callbacks
    let http_server = warp::serve(oauth_routes).run(([0, 0, 0, 0], http_port));

    // Run MCP server
    let mcp_server = server.run(mcp_port);

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

/// Setup OAuth callback routes for single-tenant mode
fn setup_single_tenant_oauth_routes(
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    // OAuth callback route for Strava
    let strava_callback = warp::path!("oauth" / "callback" / "strava")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(handle_strava_oauth_callback);

    // OAuth callback route for Fitbit
    let fitbit_callback = warp::path!("oauth" / "callback" / "fitbit")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and_then(handle_fitbit_oauth_callback);

    // Health check route
    let health = warp::path!("health").and(warp::get()).map(|| {
        warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "service": "pierre-mcp-server-single-tenant",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
    });

    strava_callback.or(fitbit_callback).or(health)
}

/// Handle Strava OAuth callback in single-tenant mode
async fn handle_strava_oauth_callback(
    query: std::collections::HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    handle_oauth_callback("strava", query).await
}

/// Handle Fitbit OAuth callback in single-tenant mode
async fn handle_fitbit_oauth_callback(
    query: std::collections::HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    handle_oauth_callback("fitbit", query).await
}

/// Generic OAuth callback handler for single-tenant mode
#[allow(clippy::too_many_lines)]
async fn handle_oauth_callback(
    provider: &str,
    query: std::collections::HashMap<String, String>,
) -> Result<impl warp::Reply, warp::Rejection> {
    use pierre_mcp_server::oauth::manager::OAuthManager;

    // Extract code and state from query parameters
    let code = query
        .get("code")
        .ok_or_else(|| warp::reject::custom(OAuthCallbackError::MissingParameter("code".into())))?;

    let state = query.get("state").ok_or_else(|| {
        warp::reject::custom(OAuthCallbackError::MissingParameter("state".into()))
    })?;

    // Initialize database with default configuration for single-tenant
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "data/pierre.db".into());

    let encryption_key = if let Ok(key_path) = std::env::var("ENCRYPTION_KEY_PATH") {
        std::fs::read(&key_path).map_err(|e| {
            tracing::error!("Failed to read encryption key from {}: {}", key_path, e);
            warp::reject::custom(OAuthCallbackError::ServerError(format!(
                "Key read error: {e}"
            )))
        })?
    } else {
        let key_path = "data/encryption.key";
        if std::path::Path::new(key_path).exists() {
            std::fs::read(key_path).map_err(|e| {
                tracing::error!("Failed to read encryption key from {}: {}", key_path, e);
                warp::reject::custom(OAuthCallbackError::ServerError(format!(
                    "Key read error: {e}"
                )))
            })?
        } else {
            let key = pierre_mcp_server::database::generate_encryption_key();
            if let Some(parent) = std::path::Path::new(key_path).parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    warp::reject::custom(OAuthCallbackError::ServerError(format!(
                        "Directory creation error: {e}"
                    )))
                })?;
            }
            std::fs::write(key_path, key).map_err(|e| {
                warp::reject::custom(OAuthCallbackError::ServerError(format!(
                    "Key write error: {e}"
                )))
            })?;
            key.to_vec()
        }
    };

    let database = std::sync::Arc::new(
        Database::new(&database_url, encryption_key)
            .await
            .map_err(|e| {
                tracing::error!("Failed to create database: {}", e);
                warp::reject::custom(OAuthCallbackError::ServerError(format!(
                    "Database error: {e}"
                )))
            })?,
    );

    tracing::info!(
        "OAuth callback handler initialized with: {}",
        database.backend_info()
    );

    // Create OAuth manager and register providers
    let mut oauth_manager = OAuthManager::new(database);

    match provider {
        "strava" => {
            // For single-tenant mode, read from environment variables
            let client_id = std::env::var("STRAVA_CLIENT_ID").map_err(|_| {
                warp::reject::custom(OAuthCallbackError::ServerError(
                    "STRAVA_CLIENT_ID not set".into(),
                ))
            })?;
            let client_secret = std::env::var("STRAVA_CLIENT_SECRET").map_err(|_| {
                warp::reject::custom(OAuthCallbackError::ServerError(
                    "STRAVA_CLIENT_SECRET not set".into(),
                ))
            })?;
            let redirect_uri = std::env::var("STRAVA_REDIRECT_URI").ok();

            let config = pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some(client_id),
                client_secret: Some(client_secret),
                redirect_uri,
                scopes: vec!["read".into(), "activity:read_all".into()],
                enabled: true,
            };

            let strava_provider =
                pierre_mcp_server::oauth::providers::StravaOAuthProvider::from_config(&config)
                    .map_err(|e| {
                        tracing::error!("Failed to create Strava provider: {}", e);
                        warp::reject::custom(OAuthCallbackError::ServerError(format!(
                            "Provider error: {e}"
                        )))
                    })?;
            oauth_manager.register_provider(Box::new(strava_provider));
        }
        "fitbit" => {
            // For single-tenant mode, read from environment variables
            let client_id = std::env::var("FITBIT_CLIENT_ID").map_err(|_| {
                warp::reject::custom(OAuthCallbackError::ServerError(
                    "FITBIT_CLIENT_ID not set".into(),
                ))
            })?;
            let client_secret = std::env::var("FITBIT_CLIENT_SECRET").map_err(|_| {
                warp::reject::custom(OAuthCallbackError::ServerError(
                    "FITBIT_CLIENT_SECRET not set".into(),
                ))
            })?;
            let redirect_uri = std::env::var("FITBIT_REDIRECT_URI").ok();

            let config = pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some(client_id),
                client_secret: Some(client_secret),
                redirect_uri,
                scopes: vec![
                    "activity".into(),
                    "heartrate".into(),
                    "location".into(),
                    "nutrition".into(),
                    "profile".into(),
                    "settings".into(),
                    "sleep".into(),
                    "social".into(),
                    "weight".into(),
                ],
                enabled: true,
            };

            let fitbit_provider =
                pierre_mcp_server::oauth::providers::FitbitOAuthProvider::from_config(&config)
                    .map_err(|e| {
                        tracing::error!("Failed to create Fitbit provider: {}", e);
                        warp::reject::custom(OAuthCallbackError::ServerError(format!(
                            "Provider error: {e}"
                        )))
                    })?;
            oauth_manager.register_provider(Box::new(fitbit_provider));
        }
        _ => {
            return Err(warp::reject::custom(
                OAuthCallbackError::UnsupportedProvider(provider.to_string()),
            ));
        }
    }

    // Handle the OAuth callback
    match oauth_manager.handle_callback(code, state, provider).await {
        Ok(callback_response) => {
            tracing::info!(
                "OAuth callback successful for provider {}: user {}",
                provider,
                callback_response.user_id
            );

            // Return success page
            let success_html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>OAuth Success - Pierre Fitness API</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; text-align: center; }}
        .success {{ color: #4CAF50; }}
        .info {{ color: #2196F3; margin: 20px 0; }}
        .details {{ background: #f5f5f5; padding: 20px; border-radius: 8px; margin: 20px 0; }}
    </style>
</head>
<body>
    <h1 class="success">✅ OAuth Authorization Successful!</h1>
    <div class="info">
        <p>Your {} account has been successfully connected to Pierre Fitness API.</p>
        <p>You can now close this browser window and return to your MCP client.</p>
    </div>
    <div class="details">
        <h3>Connection Details:</h3>
        <p><strong>Provider:</strong> {}</p>
        <p><strong>User ID:</strong> {}</p>
        <p><strong>Scopes:</strong> {}</p>
        <p><strong>Expires:</strong> {}</p>
    </div>
</body>
</html>"#,
                provider
                    .chars()
                    .next()
                    .unwrap()
                    .to_uppercase()
                    .collect::<String>()
                    + &provider[1..],
                callback_response.provider,
                callback_response.user_id,
                callback_response.scopes,
                callback_response.expires_at
            );

            Ok(warp::reply::html(success_html))
        }
        Err(e) => {
            tracing::error!("OAuth callback failed for provider {}: {}", provider, e);

            // Return error page
            let error_html = format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>OAuth Error - Pierre Fitness API</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; text-align: center; }}
        .error {{ color: #f44336; }}
        .details {{ background: #ffebee; padding: 20px; border-radius: 8px; margin: 20px 0; border-left: 4px solid #f44336; }}
    </style>
</head>
<body>
    <h1 class="error">❌ OAuth Authorization Failed</h1>
    <div class="details">
        <h3>Error Details:</h3>
        <p><strong>Provider:</strong> {provider}</p>
        <p><strong>Error:</strong> {e}</p>
        <p>Please try the authorization process again.</p>
    </div>
</body>
</html>"#
            );

            Ok(warp::reply::html(error_html))
        }
    }
}

/// Custom error type for OAuth callbacks
#[derive(Debug)]
enum OAuthCallbackError {
    MissingParameter(String),
    UnsupportedProvider(String),
    ServerError(String),
}

impl std::fmt::Display for OAuthCallbackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingParameter(param) => {
                write!(f, "Missing required parameter: {param}")
            }
            Self::UnsupportedProvider(provider) => {
                write!(f, "Unsupported OAuth provider: {provider}")
            }
            Self::ServerError(error) => {
                write!(f, "OAuth server error: {error}")
            }
        }
    }
}

impl warp::reject::Reject for OAuthCallbackError {}

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

/// Load admin JWT secret from file or generate a new one (reuses user JWT secret)
fn load_or_generate_admin_jwt_secret(secret_file: &PathBuf) -> Result<[u8; 64]> {
    // For now, we reuse the same JWT secret for admin tokens
    // In production, you might want separate secrets
    load_or_generate_jwt_secret(secret_file)
}
