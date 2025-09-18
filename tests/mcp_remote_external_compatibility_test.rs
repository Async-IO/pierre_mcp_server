// ABOUTME: Integration test for mcp-remote OAuth 2.0 + MCP protocol with Pierre server
// ABOUTME: Tests programmatic MCP server with OAuth 2.0 discovery for mcp-remote compatibility

#![recursion_limit = "512"]

use anyhow::Result;
use pierre_mcp_server::{
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::factory::Database,
    mcp::{multitenant::MultiTenantMcpServer, resources::ServerResources},
};
use rand::Rng;
use serde_json::{json, Value};
use std::{net::TcpListener, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::time::sleep;

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_mcp_remote_tests";

/// Check if a port is available
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{port}")).is_ok()
}

/// Find an available port using randomized search like the working test
fn find_available_port() -> u16 {
    let mut rng = rand::thread_rng();
    let mut port = rng.gen_range(20000..30000);

    // Try to find an available port with retry logic
    for _ in 0..20 {
        if is_port_available(port) {
            return port;
        }
        port = rng.gen_range(20000..30000);
    }
    panic!("Could not find an available port in range 20000-30000 after 20 attempts");
}

/// Create test configuration for mcp-remote compatibility
fn create_test_config(
    jwt_secret_path: &std::path::Path,
    encryption_key_path: &std::path::Path,
    port: u16,
) -> Arc<pierre_mcp_server::config::environment::ServerConfig> {
    Arc::new(pierre_mcp_server::config::environment::ServerConfig {
        http_port: port,
        log_level: pierre_mcp_server::config::environment::LogLevel::Info,
        database: pierre_mcp_server::config::environment::DatabaseConfig {
            url: pierre_mcp_server::config::environment::DatabaseUrl::SQLite {
                path: ":memory:".into(),
            },
            encryption_key_path: encryption_key_path.to_path_buf(),
            auto_migrate: true,
            backup: pierre_mcp_server::config::environment::BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: "./backups".into(),
            },
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_secret_path: jwt_secret_path.to_path_buf(),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_client_id".into()),
                client_secret: Some("test_client_secret".into()),
                redirect_uri: Some(format!("http://localhost:{port}/oauth/callback/strava")),
                scopes: vec!["read".into(), "activity:read".into()],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".into()],
            rate_limit: pierre_mcp_server::config::environment::RateLimitConfig {
                enabled: true,
                requests_per_window: 1000,
                window_seconds: 60,
            },
            tls: pierre_mcp_server::config::environment::TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: pierre_mcp_server::config::environment::SecurityHeadersConfig {
                environment: pierre_mcp_server::config::environment::Environment::Testing,
            },
        },
        external_services: pierre_mcp_server::config::environment::ExternalServicesConfig {
            weather: pierre_mcp_server::config::environment::WeatherServiceConfig {
                api_key: Some("test_weather_key".into()),
                base_url: "https://api.openweathermap.org/data/2.5".into(),
                enabled: true,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".into(),
                enabled: true,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".into(),
                auth_url: "https://www.strava.com/oauth/authorize".into(),
                token_url: "https://www.strava.com/oauth/token".into(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".into(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".into(),
                token_url: "https://api.fitbit.com/oauth2/token".into(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    })
}

/// Test OAuth 2.0 discovery endpoints for mcp-remote compatibility
#[tokio::test]
async fn test_oauth_discovery_endpoints() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = find_available_port();

    // Setup test environment
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key.clone()).await?;
    let encryption_key_path = temp_dir.path().join("encryption.key");
    std::fs::write(&encryption_key_path, &encryption_key)?;

    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let stored_jwt_secret = TEST_JWT_SECRET.to_string();
    std::fs::write(&jwt_secret_path, stored_jwt_secret.as_bytes())?;

    let auth_manager = AuthManager::new(stored_jwt_secret.as_bytes().to_vec(), 24);

    // Create server configuration
    let config = create_test_config(&jwt_secret_path, &encryption_key_path, port);

    // Create server resources and start server
    let resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        config,
    ));

    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Wait for server to be ready
    sleep(Duration::from_secs(2)).await;

    // Test OAuth 2.0 Server Metadata Discovery (RFC 8414)
    let client = reqwest::Client::new();
    let discovery_url = format!("http://localhost:{port}/.well-known/oauth-authorization-server");
    let discovery_response = client.get(&discovery_url).send().await?;

    assert!(
        discovery_response.status().is_success(),
        "Discovery endpoint should work"
    );

    let metadata: Value = discovery_response.json().await?;
    assert!(
        metadata["authorization_endpoint"].is_string(),
        "Should have authorization_endpoint"
    );
    assert!(
        metadata["token_endpoint"].is_string(),
        "Should have token_endpoint"
    );
    assert!(
        metadata["registration_endpoint"].is_string(),
        "Should have registration_endpoint"
    );

    println!("✅ OAuth 2.0 discovery metadata validated for mcp-remote compatibility");

    // Clean up
    server_handle.abort();
    Ok(())
}

/// Test MCP protocol endpoints for mcp-remote compatibility
#[tokio::test]
async fn test_mcp_protocol_endpoints() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = find_available_port();

    // Setup test environment
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key.clone()).await?;
    let encryption_key_path = temp_dir.path().join("encryption.key");
    std::fs::write(&encryption_key_path, &encryption_key)?;

    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let stored_jwt_secret = TEST_JWT_SECRET.to_string();
    std::fs::write(&jwt_secret_path, stored_jwt_secret.as_bytes())?;

    let auth_manager = AuthManager::new(stored_jwt_secret.as_bytes().to_vec(), 24);

    // Create server configuration
    let config = create_test_config(&jwt_secret_path, &encryption_key_path, port);

    // Create server resources and start server
    let resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        config,
    ));

    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Wait for server to be ready
    sleep(Duration::from_secs(2)).await;

    // Test MCP HTTP endpoint availability
    let client = reqwest::Client::new();
    let mcp_url = format!("http://localhost:{port}/mcp");

    // Test with a simple MCP initialize message
    let init_message = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "mcp-remote-test",
                "version": "1.0.0"
            }
        }
    });

    let response = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
        .json(&init_message)
        .send()
        .await?;

    assert!(
        response.status().is_success(),
        "MCP endpoint should be available"
    );

    let mcp_response: Value = response.json().await?;
    assert!(
        mcp_response.get("result").is_some(),
        "Initialize should succeed"
    );
    assert!(
        mcp_response["result"]["capabilities"].is_object(),
        "Should return server capabilities"
    );

    println!("✅ MCP HTTP protocol endpoint working for mcp-remote compatibility");

    // Test tools/list endpoint (should work without auth)
    let tools_message = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let tools_response = client
        .post(&mcp_url)
        .header("Content-Type", "application/json")
        .json(&tools_message)
        .send()
        .await?;

    assert!(
        tools_response.status().is_success(),
        "Tools list endpoint should work"
    );

    let tools_result: Value = tools_response.json().await?;
    assert!(
        tools_result.get("result").is_some(),
        "Tools list should succeed"
    );

    println!("✅ MCP tools/list endpoint working for mcp-remote compatibility");

    // Clean up
    server_handle.abort();
    Ok(())
}

/// Test OAuth 2.0 client registration endpoint for mcp-remote compatibility
#[tokio::test]
async fn test_oauth_client_registration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let port = find_available_port();

    // Setup test environment
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key.clone()).await?;
    let encryption_key_path = temp_dir.path().join("encryption.key");
    std::fs::write(&encryption_key_path, &encryption_key)?;

    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let stored_jwt_secret = TEST_JWT_SECRET.to_string();
    std::fs::write(&jwt_secret_path, stored_jwt_secret.as_bytes())?;

    let auth_manager = AuthManager::new(stored_jwt_secret.as_bytes().to_vec(), 24);

    // Create server configuration
    let config = create_test_config(&jwt_secret_path, &encryption_key_path, port);

    // Create server resources and start server
    let resources = Arc::new(ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        config,
    ));

    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Wait for server to be ready
    sleep(Duration::from_secs(2)).await;

    // Test OAuth 2.0 Dynamic Client Registration (RFC 7591)
    let client = reqwest::Client::new();
    let registration_url = format!("http://localhost:{port}/oauth2/register");

    let registration_request = json!({
        "client_name": "mcp-remote-test-client",
        "client_uri": "https://github.com/anthropics/mcp-remote",
        "redirect_uris": [
            format!("http://localhost:{}/oauth/callback", port)
        ],
        "grant_types": ["authorization_code"],
        "response_types": ["code"],
        "scope": "read activity:read"
    });

    let registration_response = client
        .post(&registration_url)
        .header("Content-Type", "application/json")
        .json(&registration_request)
        .send()
        .await?;

    if !registration_response.status().is_success() {
        let status = registration_response.status();
        let error_body = registration_response
            .text()
            .await
            .unwrap_or_else(|_| "Could not read response body".to_string());
        panic!("Client registration failed with status {status}: {error_body}");
    }

    let registration_result: Value = registration_response.json().await?;
    assert!(
        registration_result["client_id"].is_string(),
        "Should return client_id"
    );
    assert!(
        registration_result["client_secret"].is_string(),
        "Should return client_secret"
    );

    println!("✅ OAuth 2.0 client registration working for mcp-remote compatibility");

    // Clean up
    server_handle.abort();
    Ok(())
}
