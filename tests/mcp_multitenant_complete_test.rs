// ABOUTME: Complete multi-tenant MCP server test covering the entire workflow
// ABOUTME: Tests user registration, JWT auth, OAuth integration, and MCP protocol usage

#![allow(clippy::too_many_lines)]
#![recursion_limit = "256"]

use anyhow::Result;
use pierre_mcp_server::{
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::multitenant::MultiTenantMcpServer,
};
use rand::Rng;
use reqwest::Client;
use serde_json::{json, Value};
use std::{net::TcpListener, sync::Arc, time::Duration};
use tempfile::TempDir;
use tokio::time::{sleep, timeout};
use uuid::Uuid;

const TEST_JWT_SECRET: &str = "test_jwt_secret_for_complete_multitenant_tests";

/// Check if a port is available
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{port}")).is_ok()
}

/// Test configuration for multi-tenant MCP server
fn create_test_config(
    jwt_secret_path: &std::path::Path,
    encryption_key_path: &std::path::Path,
) -> Arc<pierre_mcp_server::config::environment::ServerConfig> {
    Arc::new(pierre_mcp_server::config::environment::ServerConfig {
        mcp_port: 8080,
        http_port: 8081,
        log_level: pierre_mcp_server::config::environment::LogLevel::Info,
        database: pierre_mcp_server::config::environment::DatabaseConfig {
            url: pierre_mcp_server::config::environment::DatabaseUrl::Memory,
            encryption_key_path: encryption_key_path.to_path_buf(),
            auto_migrate: true,
            backup: pierre_mcp_server::config::environment::BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: std::path::PathBuf::from("test_backups"),
            },
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_secret_path: jwt_secret_path.to_path_buf(),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_client_id".to_string()),
                client_secret: Some("test_client_secret".to_string()),
                redirect_uri: Some("http://localhost:8081/oauth/callback/strava".to_string()),
                scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: Some("http://localhost:8081/oauth/callback/fitbit".to_string()),
                scopes: vec!["activity".to_string()],
                enabled: false,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: pierre_mcp_server::config::environment::RateLimitConfig {
                enabled: true,
                requests_per_window: 100,
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
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: true,
            },
            strava_api: pierre_mcp_server::config::environment::StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: pierre_mcp_server::config::environment::FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: pierre_mcp_server::config::environment::AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: pierre_mcp_server::config::environment::ProtocolConfig {
                mcp_version: "2025-06-18".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    })
}

/// Multi-tenant MCP client for testing
struct MultiTenantMcpClient {
    http_client: Client,
    base_url: String,
    jwt_token: Option<String>,
}

impl MultiTenantMcpClient {
    fn new(port: u16) -> Self {
        Self {
            http_client: Client::builder()
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .unwrap(),
            base_url: format!("http://127.0.0.1:{port}"),
            jwt_token: None,
        }
    }

    /// Register a new user and auto-approve for testing
    async fn register_and_approve_user(
        &self,
        database: &Database,
        email: &str,
        password: &str,
        display_name: &str,
    ) -> Result<String> {
        // Store tenant OAuth credentials for testing
        let tenant_uuid = Uuid::new_v4();

        // Create the actual test user first (will be tenant owner)
        let user_id = uuid::Uuid::new_v4();
        let password_hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
        let test_user = pierre_mcp_server::models::User {
            id: user_id,
            email: email.to_string(),
            display_name: Some(display_name.to_string()),
            password_hash,
            tier: pierre_mcp_server::models::UserTier::Starter,
            tenant_id: Some(tenant_uuid.to_string()), // Associate with the tenant that has OAuth credentials
            strava_token: None,
            fitbit_token: None,
            is_active: true,
            user_status: pierre_mcp_server::models::UserStatus::Active, // Already active
            is_admin: false,
            approved_by: Some(user_id), // Self-approved for test
            approved_at: Some(chrono::Utc::now()),
            created_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
        };
        database.create_user(&test_user).await?;

        // Create a test tenant for OAuth credentials with test user as owner
        let test_tenant = pierre_mcp_server::models::Tenant {
            id: tenant_uuid,
            name: "Test Tenant".to_string(),
            slug: "test-tenant".to_string(),
            domain: None,
            plan: "starter".to_string(),
            owner_user_id: user_id, // Test user is the owner
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        database.create_tenant(&test_tenant).await?;

        let strava_credentials = pierre_mcp_server::tenant::TenantOAuthCredentials {
            tenant_id: tenant_uuid,
            provider: "strava".to_string(),
            client_id: "test_client_id".to_string(),
            client_secret: "test_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            scopes: vec!["read".to_string(), "activity:read_all".to_string()],
            rate_limit_per_day: 1000,
        };
        database
            .store_tenant_oauth_credentials(&strava_credentials)
            .await?;

        let fitbit_credentials = pierre_mcp_server::tenant::TenantOAuthCredentials {
            tenant_id: tenant_uuid,
            provider: "fitbit".to_string(),
            client_id: "test_fitbit_client_id".to_string(),
            client_secret: "test_fitbit_client_secret".to_string(),
            redirect_uri: "http://localhost:3000/auth/callback".to_string(),
            scopes: vec!["activity".to_string(), "profile".to_string()],
            rate_limit_per_day: 1000,
        };
        database
            .store_tenant_oauth_credentials(&fitbit_credentials)
            .await?;

        // User and tenant are already created above

        Ok(user_id.to_string())
    }

    /// Login and get JWT token
    async fn login(&mut self, email: &str, password: &str) -> Result<()> {
        let response = timeout(
            Duration::from_secs(10),
            self.http_client
                .post(format!("{}/api/auth/login", self.base_url))
                .json(&json!({
                    "email": email,
                    "password": password
                }))
                .send(),
        )
        .await??;

        if response.status().is_success() {
            let data: Value = response.json().await?;
            let token = data["jwt_token"].as_str().unwrap().to_string();
            eprintln!(
                "DEBUG: Received JWT token from login: {} (first 100 chars)",
                &token[..std::cmp::min(100, token.len())]
            );
            eprintln!("DEBUG: Token length: {} characters", token.len());
            self.jwt_token = Some(token);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Login failed: {}", response.status()))
        }
    }

    /// Get Strava OAuth URL
    async fn get_strava_oauth_url(&self, user_id: &str) -> Result<String> {
        let url = format!("{}/api/oauth/auth/strava/{user_id}", self.base_url);
        let response = self.http_client.get(url).send().await?;

        if response.status() == 302 {
            // Extract URL from Location header for redirect response
            if let Some(location) = response.headers().get("location") {
                let auth_url = location.to_str()?.to_string();
                Ok(auth_url)
            } else {
                Err(anyhow::anyhow!("OAuth redirect missing Location header"))
            }
        } else if response.status().is_success() {
            // Handle JSON response (if server returns JSON instead of redirect)
            let data: Value = response.json().await?;
            Ok(data["authorization_url"].as_str().unwrap().to_string())
        } else {
            Err(anyhow::anyhow!(
                "OAuth URL generation failed: {}",
                response.status()
            ))
        }
    }

    /// Send MCP request via HTTP transport (to MCP server on base port)
    async fn send_mcp_request(&self, request: Value) -> Result<Value> {
        let request_with_auth = request;

        // After consolidation, both OAuth and MCP are on the same HTTP server
        let mcp_url = format!("{}/mcp", self.base_url);

        let mut request_builder = self
            .http_client
            .post(mcp_url)
            .header("Content-Type", "application/json")
            .header("Origin", "http://localhost");

        // Add JWT authentication in Authorization header
        if let Some(token) = &self.jwt_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {token}"));
        }

        let response = timeout(
            Duration::from_secs(10),
            request_builder.json(&request_with_auth).send(),
        )
        .await??;

        if response.status() == 200 || response.status() == 202 {
            Ok(response.json().await?)
        } else {
            Err(anyhow::anyhow!("MCP request failed: {}", response.status()))
        }
    }

    /// Initialize MCP connection
    async fn initialize_mcp(&self) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "roots": {"listChanged": true},
                    "sampling": {}
                },
                "clientInfo": {
                    "name": "multitenant-test-client",
                    "version": "1.0.0"
                }
            }
        });

        self.send_mcp_request(request).await
    }

    /// List available MCP tools
    async fn list_tools(&self) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        self.send_mcp_request(request).await
    }

    /// Call an MCP tool
    async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        self.send_mcp_request(request).await
    }
}

/// Setup test environment
async fn setup_test_environment() -> Result<(Database, AuthManager, u16, TempDir, String)> {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key.clone()).await?;

    // Initialize the system secret in the database to match what the server expects
    // First get_or_create to ensure the entry exists, then update with our test value
    let _ = database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;
    database
        .update_system_secret("admin_jwt_secret", TEST_JWT_SECRET)
        .await?;

    // CRITICAL: Create AuthManager with the exact same secret we just stored in the database
    // This ensures the test AuthManager and any server-created AuthManager use identical secrets
    let auth_manager = AuthManager::new(TEST_JWT_SECRET.as_bytes().to_vec(), 24);

    // Verify the database contains the expected secret
    let verified_secret = database
        .get_or_create_system_secret("admin_jwt_secret")
        .await?;
    assert_eq!(
        verified_secret, TEST_JWT_SECRET,
        "Database JWT secret mismatch!"
    );

    // Create temporary files for JWT secret and encryption key
    let temp_dir = TempDir::new()?;
    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let encryption_key_path = temp_dir.path().join("encryption.key");

    // Write JWT secret (using the actual value stored in database)
    std::fs::write(&jwt_secret_path, &verified_secret)?;

    // Write encryption key
    std::fs::write(&encryption_key_path, &encryption_key)?;

    // Use a random port to avoid conflicts
    let mut rng = rand::thread_rng();
    let mut port = rng.gen_range(20000..30000);

    // Try to find an available port
    for _ in 0..10 {
        if is_port_available(port) && is_port_available(port + 1) {
            break;
        }
        port = rng.gen_range(20000..30000);
    }

    Ok((database, auth_manager, port, temp_dir, verified_secret))
}

/// Test complete multi-tenant MCP server workflow
#[tokio::test]
async fn test_complete_multitenant_workflow() -> Result<()> {
    // Set required environment variables for OAuth
    std::env::set_var("STRAVA_CLIENT_ID", "test_client_id");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_client_secret");
    std::env::set_var("FITBIT_CLIENT_ID", "test_fitbit_client_id");
    std::env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_client_secret");

    let (database, auth_manager, server_port, temp_dir, stored_jwt_secret) =
        setup_test_environment().await?;

    // Clone database for user approval operations
    let database_for_approval = database.clone();

    // Start the server
    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let encryption_key_path = temp_dir.path().join("encryption.key");
    let resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        create_test_config(&jwt_secret_path, &encryption_key_path),
    ));
    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(server_port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(1000)).await;

    // Wait for server to be ready
    let http_port = server_port + 1;
    for _attempt in 0..10 {
        if !is_port_available(http_port) {
            break; // Port is in use, server is likely ready
        }
        sleep(Duration::from_millis(200)).await;
    }

    let mut client = MultiTenantMcpClient::new(http_port);

    // Test 1: User Registration
    let email = "test@example.com";
    let password = "testpassword123";
    let display_name = "Test User";

    let user_id = client
        .register_and_approve_user(&database_for_approval, email, password, display_name)
        .await?;
    assert!(!user_id.is_empty());
    assert!(Uuid::parse_str(&user_id).is_ok());

    // Test 2: User Login
    client.login(email, password).await?;
    assert!(client.jwt_token.is_some());

    // Test 3: OAuth URL Generation
    let oauth_url = client.get_strava_oauth_url(&user_id).await?;
    assert!(oauth_url.contains("strava.com/oauth/authorize"));
    assert!(oauth_url.contains("client_id=test_client_id")); // Verify tenant-specific credentials are used
    assert!(oauth_url.contains("redirect_uri="));
    assert!(oauth_url.contains("response_type=code"));
    assert!(oauth_url.contains("scope="));
    assert!(oauth_url.contains(&format!("state={user_id}%3A"))); // URL encoded ":"

    // Test 4: MCP Protocol - Initialize
    let init_response = client.initialize_mcp().await?;
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(!init_response["result"]["serverInfo"]["name"].is_null());
    assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");

    // Test 5: MCP Protocol - List Tools
    let tools_response = client.list_tools().await?;
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);

    let tools = tools_response["result"]["tools"].as_array().unwrap();
    assert!(!tools.is_empty());

    // Verify essential tools are available
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();

    assert!(tool_names.contains(&"get_connection_status"));
    assert!(tool_names.contains(&"get_activities"));
    assert!(tool_names.contains(&"get_athlete"));
    assert!(tool_names.contains(&"get_stats"));

    // Test 6: MCP Protocol - Call Tool (Connection Status)
    let connection_response = client.call_tool("get_connection_status", json!({})).await?;

    assert_eq!(connection_response["jsonrpc"], "2.0");
    assert_eq!(connection_response["id"], 3);

    // Check if there's an error in the response
    if connection_response.get("error").is_some() {
        eprintln!(
            "DEBUG: MCP error response: {:?}",
            connection_response["error"]
        );
    }

    // Should return providers status (not connected yet)
    let result = &connection_response["result"];

    // Debug: print the actual result to see what we're getting
    if !result.is_object() {
        eprintln!("DEBUG: Expected result to be object, got: {result:?}");
        eprintln!("DEBUG: Full response: {connection_response:?}");
    }

    // The result has structuredContent with providers array and tenant_info
    assert!(result.is_object());
    let structured_content = &result["structuredContent"];
    assert!(structured_content.is_object());
    assert!(structured_content["providers"].is_array());
    assert_eq!(structured_content["providers"].as_array().unwrap().len(), 2); // Strava and Fitbit
    assert!(structured_content["tenant_info"].is_object());
    assert_eq!(
        structured_content["tenant_info"]["tenant_name"],
        "Test Tenant"
    );

    // Test 7: MCP Protocol - Call Tool (Get Activities - should work without provider)
    let activities_response = client
        .call_tool("get_activities", json!({"provider": "strava", "limit": 5}))
        .await?;

    assert_eq!(activities_response["jsonrpc"], "2.0");
    // Should return empty array or specific message about no provider connection

    // Test 8: MCP Protocol - Call Tool (Get Athlete - should work without provider)
    let athlete_response = client
        .call_tool("get_athlete", json!({"provider": "strava"}))
        .await?;

    assert_eq!(athlete_response["jsonrpc"], "2.0");
    // Should return appropriate response for unauthenticated provider

    // Test 9: MCP Protocol - Call Tool (Get Stats)
    let stats_response = client
        .call_tool("get_stats", json!({"provider": "strava"}))
        .await?;

    assert_eq!(stats_response["jsonrpc"], "2.0");

    // Test 10: MCP Protocol - Error Handling (Invalid Tool)
    let invalid_tool_response = client.call_tool("invalid_tool", json!({})).await?;

    assert_eq!(invalid_tool_response["jsonrpc"], "2.0");
    assert!(!invalid_tool_response["error"].is_null());
    assert_eq!(invalid_tool_response["error"]["code"], -32601); // Method not found

    println!("All multi-tenant MCP server tests passed!");

    // Clean up server
    server_handle.abort();

    Ok(())
}

/// Test MCP server without authentication (should fail)
#[tokio::test]
async fn test_mcp_authentication_required() -> Result<()> {
    let (database, auth_manager, server_port, temp_dir, stored_jwt_secret) =
        setup_test_environment().await?;

    // Start the server
    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let encryption_key_path = temp_dir.path().join("encryption.key");
    let resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        create_test_config(&jwt_secret_path, &encryption_key_path),
    ));
    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(server_port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(1000)).await;

    // Wait for server to be ready
    let http_port = server_port + 1;
    for _attempt in 0..10 {
        if !is_port_available(http_port) {
            break; // Port is in use, server is likely ready
        }
        sleep(Duration::from_millis(200)).await;
    }

    let client = MultiTenantMcpClient::new(http_port);
    // Note: No login, so no JWT token

    // Try to list tools without authentication (this should work)
    let tools_response = client.list_tools().await?;

    // Tools list should work without authentication
    assert_eq!(tools_response["jsonrpc"], "2.0");
    assert_eq!(tools_response["id"], 2);
    assert!(tools_response["result"]["tools"].is_array());
    assert!(!tools_response["result"]["tools"]
        .as_array()
        .unwrap()
        .is_empty());

    // Try to call a tool without authentication (this should fail)
    let tool_call_response = client.call_tool("get_connection_status", json!({})).await?;

    // Should return an authentication error for tool call
    assert_eq!(tool_call_response["jsonrpc"], "2.0");
    assert!(!tool_call_response["error"].is_null());
    assert_eq!(tool_call_response["error"]["code"], -32603); // Authentication error (Internal error)

    println!("Authentication requirement test passed!");

    // Clean up server
    server_handle.abort();

    Ok(())
}

/// Test MCP server initialization without authentication (should work)
#[tokio::test]
async fn test_mcp_initialization_no_auth() -> Result<()> {
    let (database, auth_manager, server_port, temp_dir, stored_jwt_secret) =
        setup_test_environment().await?;

    // Start the server
    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let encryption_key_path = temp_dir.path().join("encryption.key");
    let resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        create_test_config(&jwt_secret_path, &encryption_key_path),
    ));
    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(server_port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(1000)).await;

    // Wait for server to be ready
    let http_port = server_port + 1;
    for _attempt in 0..10 {
        if !is_port_available(http_port) {
            break; // Port is in use, server is likely ready
        }
        sleep(Duration::from_millis(200)).await;
    }

    let client = MultiTenantMcpClient::new(http_port);

    // Initialize should work without authentication
    let init_response = client.initialize_mcp().await?;

    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(!init_response["result"]["serverInfo"]["name"].is_null());
    assert_eq!(init_response["result"]["protocolVersion"], "2025-06-18");

    println!("MCP initialization without auth test passed!");

    // Clean up server
    server_handle.abort();

    Ok(())
}

/// Test rate limiting and concurrent requests
#[tokio::test]
async fn test_mcp_concurrent_requests() -> Result<()> {
    let (database, auth_manager, server_port, temp_dir, stored_jwt_secret) =
        setup_test_environment().await?;

    // Clone database for user approval operations
    let database_for_approval = database.clone();

    // Start the server
    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let encryption_key_path = temp_dir.path().join("encryption.key");
    let resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        create_test_config(&jwt_secret_path, &encryption_key_path),
    ));
    let server = MultiTenantMcpServer::new(resources);
    let server_handle = tokio::spawn(async move {
        tokio::select! {
            result = server.run_http_only(server_port) => {
                if let Err(e) = result {
                    eprintln!("Server failed to start: {e}");
                }
            }
            () = tokio::time::sleep(Duration::from_secs(30)) => {
                eprintln!("Server startup timed out after 30 seconds");
            }
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(1000)).await;

    // Wait for server to be ready
    let http_port = server_port + 1;
    for _attempt in 0..10 {
        if !is_port_available(http_port) {
            break; // Port is in use, server is likely ready
        }
        sleep(Duration::from_millis(200)).await;
    }

    let mut client = MultiTenantMcpClient::new(http_port);

    // Register and login
    let _user_id = client
        .register_and_approve_user(
            &database_for_approval,
            "concurrent@example.com",
            "password123",
            "Concurrent User",
        )
        .await?;
    client
        .login("concurrent@example.com", "password123")
        .await?;

    // Send multiple concurrent requests
    let mut handles = Vec::new();

    for _i in 0..5 {
        let client_clone = MultiTenantMcpClient {
            http_client: client.http_client.clone(),
            base_url: client.base_url.clone(),
            jwt_token: client.jwt_token.clone(),
        };

        let handle = tokio::spawn(async move {
            client_clone
                .call_tool("get_connection_status", json!({}))
                .await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await.unwrap();
        if result.is_ok() {
            success_count += 1;
        }
    }

    // All requests should succeed
    assert_eq!(success_count, 5);

    println!("Concurrent requests test passed!");

    // Clean up server
    server_handle.abort();

    Ok(())
}

/// Test multi-tenant server configuration creation
#[tokio::test]
async fn test_multitenant_server_config() -> Result<()> {
    let (database, auth_manager, _port, temp_dir, stored_jwt_secret) =
        setup_test_environment().await?;

    let jwt_secret_path = temp_dir.path().join("jwt.secret");
    let encryption_key_path = temp_dir.path().join("encryption.key");
    let config = create_test_config(&jwt_secret_path, &encryption_key_path);

    // Test server creation
    let resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        database,
        auth_manager,
        &stored_jwt_secret,
        config.clone(),
    ));
    let _server = MultiTenantMcpServer::new(resources);

    // Verify configuration
    assert_eq!(config.mcp_port, 8080);
    assert_eq!(config.http_port, 8081);
    assert!(config.oauth.strava.enabled);
    assert!(!config.oauth.fitbit.enabled);
    assert_eq!(config.app_behavior.protocol.mcp_version, "2025-06-18");
    assert_eq!(
        config.app_behavior.protocol.server_name,
        "pierre-mcp-server-test"
    );

    println!("Multi-tenant server configuration test passed!");
    Ok(())
}

/// Test MCP client configuration
#[test]
fn test_mcp_client_creation() {
    let client = MultiTenantMcpClient::new(8081);

    assert_eq!(client.base_url, "http://127.0.0.1:8081");
    assert!(client.jwt_token.is_none());

    println!("MCP client creation test passed!");
}
