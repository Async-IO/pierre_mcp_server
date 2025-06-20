// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! MCP Analytics Integration Tests
//!
//! Tests the complete analytics workflow that we demonstrated:
//! 1. User registration and authentication
//! 2. MCP server initialization and tool discovery
//! 3. Analytics tools execution with real data
//! 4. Comprehensive fitness reporting

use anyhow::Result;
use pierre_mcp_server::auth::AuthManager;
use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::factory::Database;
use pierre_mcp_server::mcp::multitenant::MultiTenantMcpServer;
use pierre_mcp_server::routes::{AuthRoutes, LoginRequest, RegisterRequest};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use uuid::Uuid;

/// Create a test ServerConfig for analytics integration tests
fn create_test_server_config(
) -> std::sync::Arc<pierre_mcp_server::config::environment::ServerConfig> {
    std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
        mcp_port: 3000,
        http_port: 4000,
        log_level: pierre_mcp_server::config::environment::LogLevel::Info,
        database: pierre_mcp_server::config::environment::DatabaseConfig {
            url: pierre_mcp_server::config::environment::DatabaseUrl::Memory,
            encryption_key_path: std::path::PathBuf::from("test.key"),
            auto_migrate: true,
            backup: pierre_mcp_server::config::environment::BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: std::path::PathBuf::from("test_backups"),
            },
        },
        auth: pierre_mcp_server::config::environment::AuthConfig {
            jwt_secret_path: std::path::PathBuf::from("test.secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: pierre_mcp_server::config::environment::OAuthConfig {
            strava: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_client_id".to_string()),
                client_secret: Some("test_client_secret".to_string()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/strava".to_string()),
                scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: Some("test_fitbit_id".to_string()),
                client_secret: Some("test_fitbit_secret".to_string()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/fitbit".to_string()),
                scopes: vec!["activity".to_string(), "profile".to_string()],
                enabled: true,
            },
        },
        security: pierre_mcp_server::config::environment::SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: pierre_mcp_server::config::environment::RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: pierre_mcp_server::config::environment::TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: pierre_mcp_server::config::environment::SecurityHeadersConfig {
                environment: pierre_mcp_server::config::environment::Environment::Development,
            },
        },
        external_services: pierre_mcp_server::config::environment::ExternalServicesConfig {
            weather: pierre_mcp_server::config::environment::WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
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
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    })
}

/// MCP client for testing analytics functionality
struct McpTestClient {
    stream: TcpStream,
    jwt_token: Option<String>,
}

impl McpTestClient {
    async fn connect(port: u16) -> Result<Self> {
        // Retry connection with exponential backoff - increased attempts for high-concurrency environments
        let mut attempts = 0;
        let max_attempts = 8;
        let mut delay = tokio::time::Duration::from_millis(100);

        loop {
            match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                Ok(stream) => {
                    return Ok(Self {
                        stream,
                        jwt_token: None,
                    });
                }
                Err(e) if attempts < max_attempts => {
                    attempts += 1;
                    eprintln!(
                        "Connection attempt {} failed: {}. Retrying in {:?}...",
                        attempts, e, delay
                    );
                    tokio::time::sleep(delay).await;
                    delay *= 2; // Exponential backoff
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    async fn send_request(&mut self, request: Value) -> Result<Value> {
        let mut request_with_auth = request;

        // Add authentication if we have a token
        if let Some(token) = &self.jwt_token {
            request_with_auth["auth"] = json!(format!("Bearer {}", token));
        }

        let request_str = serde_json::to_string(&request_with_auth)? + "\n";
        self.stream.write_all(request_str.as_bytes()).await?;

        let mut reader = BufReader::new(&mut self.stream);
        let mut response_line = String::new();
        reader.read_line(&mut response_line).await?;

        let response: Value = serde_json::from_str(response_line.trim())?;
        Ok(response)
    }

    async fn initialize(&mut self) -> Result<Value> {
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "analytics-test-client",
                    "version": "1.0.0"
                }
            }
        });

        self.send_request(init_request).await
    }

    fn set_token(&mut self, token: String) {
        self.jwt_token = Some(token);
    }

    async fn call_tool(&mut self, tool_name: &str, arguments: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": rand::random::<u32>(),
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        self.send_request(request).await
    }
}

/// Setup test environment with database and authentication
async fn setup_test_environment() -> Result<(Database, AuthManager, u16)> {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key).await?;
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    // Use a random port for testing with wider range to avoid conflicts
    let test_port = 19000 + rand::random::<u16>() % 10000;

    Ok((database, auth_manager, test_port))
}

/// Register a test user and get JWT token
async fn create_test_user(
    database: &Database,
    auth_manager: &AuthManager,
) -> Result<(Uuid, String)> {
    let auth_routes = AuthRoutes::new(database.clone(), auth_manager.clone());

    // Register user
    let register_request = RegisterRequest {
        email: "analytics-test@example.com".to_string(),
        password: "testpass123".to_string(),
        display_name: Some("Analytics Test User".to_string()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = Uuid::parse_str(&register_response.user_id)?;

    // Login to get JWT token
    let login_request = LoginRequest {
        email: "analytics-test@example.com".to_string(),
        password: "testpass123".to_string(),
    };

    let login_response = auth_routes.login(login_request).await?;

    Ok((user_id, login_response.jwt_token))
}

#[tokio::test]
async fn test_mcp_server_initialization() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;

    // Start MCP server in background
    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test MCP client connection
    let mut client = McpTestClient::connect(test_port).await?;
    let init_response = client.initialize().await?;

    // Verify initialization response
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert!(init_response["result"]["capabilities"]["tools"].is_array());

    // Check that we have all 21 expected tools
    let tools = init_response["result"]["capabilities"]["tools"]
        .as_array()
        .unwrap();
    assert_eq!(tools.len(), 21);

    // Verify key analytics tools are present
    let tool_names: Vec<&str> = tools
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();

    assert!(tool_names.contains(&"get_activities"));
    assert!(tool_names.contains(&"analyze_activity"));
    assert!(tool_names.contains(&"calculate_metrics"));
    assert!(tool_names.contains(&"calculate_fitness_score"));
    assert!(tool_names.contains(&"generate_recommendations"));
    assert!(tool_names.contains(&"set_goal"));
    assert!(tool_names.contains(&"track_progress"));

    // Clean up
    server_handle.abort();

    Ok(())
}

#[tokio::test]
async fn test_analytics_workflow_without_provider() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;
    let (_user_id, jwt_token) = create_test_user(&database, &auth_manager).await?;

    // Start MCP server
    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    // Give server more time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Connect and authenticate (with retry logic)
    let mut client = McpTestClient::connect(test_port).await?;
    client.initialize().await?;
    client.set_token(jwt_token);

    // Test connection status (should work without provider)
    let connection_status = client.call_tool("get_connection_status", json!({})).await?;
    assert!(connection_status["result"].is_array());

    // Test fitness score calculation (should work with default data)
    let fitness_score = client
        .call_tool(
            "calculate_fitness_score",
            json!({
                "timeframe": "month"
            }),
        )
        .await?;

    assert!(fitness_score["result"]["fitness_score"]["overall_score"].is_number());
    let score = fitness_score["result"]["fitness_score"]["overall_score"]
        .as_f64()
        .unwrap();
    assert!((0.0..=100.0).contains(&score));

    // Test training recommendations
    let recommendations = client
        .call_tool(
            "generate_recommendations",
            json!({
                "recommendation_type": "training"
            }),
        )
        .await?;

    assert!(recommendations["result"]["training_recommendations"].is_array());
    let recs = recommendations["result"]["training_recommendations"]
        .as_array()
        .unwrap();
    assert!(!recs.is_empty());

    // Test goal feasibility analysis
    let feasibility = client
        .call_tool(
            "analyze_goal_feasibility",
            json!({
                "goal_type": "distance",
                "target_value": 100.0,
                "timeframe": "month"
            }),
        )
        .await?;

    assert!(feasibility["result"]["feasibility_analysis"]["feasible"].is_boolean());
    assert!(feasibility["result"]["feasibility_analysis"]["confidence"].is_number());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_goal_management_workflow() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;
    let (_user_id, jwt_token) = create_test_user(&database, &auth_manager).await?;

    // Start MCP server
    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    // Wait longer for server to start and retry connection
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut client = match McpTestClient::connect(test_port).await {
        Ok(client) => client,
        Err(_) => {
            // Retry after additional wait
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            McpTestClient::connect(test_port).await?
        }
    };
    client.initialize().await?;
    client.set_token(jwt_token);

    // Test goal creation
    let goal_response = client
        .call_tool(
            "set_goal",
            json!({
                "title": "Monthly Distance Challenge",
                "description": "Run 100km this month",
                "goal_type": "distance",
                "target_value": 100.0,
                "target_date": "2025-07-11",
                "sport_type": "run"
            }),
        )
        .await?;

    // Should succeed and return goal ID
    assert!(goal_response["result"]["goal_created"]["goal_id"].is_string());
    let goal_id = goal_response["result"]["goal_created"]["goal_id"]
        .as_str()
        .unwrap();

    // Test goal progress tracking
    let progress_response = client
        .call_tool(
            "track_progress",
            json!({
                "goal_id": goal_id
            }),
        )
        .await?;

    assert!(progress_response["result"]["progress_report"]["goal_id"].is_string());
    assert!(progress_response["result"]["progress_report"]["progress_percentage"].is_number());

    // Test goal suggestions
    let suggestions = client.call_tool("suggest_goals", json!({})).await?;

    assert!(suggestions["result"]["goal_suggestions"].is_array());
    let goals = suggestions["result"]["goal_suggestions"]
        .as_array()
        .unwrap();
    assert!(!goals.is_empty());

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_analytics_tools_comprehensive() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;
    let (_user_id, jwt_token) = create_test_user(&database, &auth_manager).await?;

    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    // Wait longer for server to start in high-concurrency test environments
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Use the enhanced connection logic with retries
    let mut client = match McpTestClient::connect(test_port).await {
        Ok(client) => client,
        Err(_) => {
            // Extra retry with longer wait for high-concurrency scenarios
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
            McpTestClient::connect(test_port).await?
        }
    };
    client.initialize().await?;
    client.set_token(jwt_token);

    // Test all analytics tools that should work without provider data
    let analytics_tools = vec![
        ("calculate_fitness_score", json!({"timeframe": "month"})),
        (
            "generate_recommendations",
            json!({"recommendation_type": "training"}),
        ),
        ("suggest_goals", json!({})),
        (
            "analyze_goal_feasibility",
            json!({"goal_type": "distance", "target_value": 100.0}),
        ),
        (
            "predict_performance",
            json!({"provider": "strava", "target_sport": "run", "target_distance": 10000}),
        ),
        ("analyze_training_load", json!({"timeframe": "month"})),
        (
            "detect_patterns",
            json!({"pattern_type": "training_consistency"}),
        ),
        (
            "analyze_performance_trends",
            json!({"timeframe": "month", "metric": "pace"}),
        ),
    ];

    for (tool_name, args) in analytics_tools {
        let response = client.call_tool(tool_name, args).await?;

        // Each tool should return a successful response
        if response["error"].is_null() {
            assert!(
                response["result"].is_object(),
                "Tool {} should return result object",
                tool_name
            );
        } else {
            // If error, it should be about missing provider data, not a system error
            let error_code = response["error"]["code"].as_i64().unwrap();
            assert!(
                error_code == -32603,
                "Tool {} should have provider-related error, got code {}",
                tool_name,
                error_code
            );
        }
    }

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_authentication_required() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;

    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut client = McpTestClient::connect(test_port).await?;
    client.initialize().await?;
    // Don't set JWT token - should fail authentication

    // Test that tools require authentication
    let response = client.call_tool("get_connection_status", json!({})).await?;

    assert!(!response["error"].is_null());
    assert_eq!(response["error"]["code"], -32000); // ERROR_UNAUTHORIZED
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Authentication required"));

    server_handle.abort();
    Ok(())
}

#[tokio::test]
async fn test_error_handling_invalid_tool() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;
    let (_user_id, jwt_token) = create_test_user(&database, &auth_manager).await?;

    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut client = McpTestClient::connect(test_port).await?;
    client.initialize().await?;
    client.set_token(jwt_token);

    // Test invalid tool name
    let response = client.call_tool("invalid_tool_name", json!({})).await?;

    assert!(!response["error"].is_null());
    assert_eq!(response["error"]["code"], -32601); // ERROR_METHOD_NOT_FOUND

    server_handle.abort();
    Ok(())
}

/// Integration test that mimics the exact workflow we demonstrated
#[tokio::test]
async fn test_fitness_report_generation_workflow() -> Result<()> {
    let (database, auth_manager, test_port) = setup_test_environment().await?;
    let (_user_id, jwt_token) = create_test_user(&database, &auth_manager).await?;

    let server = MultiTenantMcpServer::new(database, auth_manager, create_test_server_config());
    let server_handle = tokio::spawn(async move { server.run(test_port).await });

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut client = McpTestClient::connect(test_port).await?;
    client.initialize().await?;
    client.set_token(jwt_token);

    // Step 1: Get connection status
    let connection_status = client.call_tool("get_connection_status", json!({})).await?;
    assert!(connection_status["result"].is_array());

    // Step 2: Calculate fitness score (like we did in the report)
    let fitness_score = client
        .call_tool(
            "calculate_fitness_score",
            json!({
                "timeframe": "month"
            }),
        )
        .await?;

    let score = fitness_score["result"]["fitness_score"]["overall_score"]
        .as_f64()
        .unwrap();
    assert!((0.0..=100.0).contains(&score));

    // Step 3: Analyze training load
    let training_load = client
        .call_tool(
            "analyze_training_load",
            json!({
                "timeframe": "month"
            }),
        )
        .await?;

    assert!(training_load["result"]["training_load_analysis"]["load_level"].is_string());
    assert!(training_load["result"]["training_load_analysis"]["weekly_hours"].is_number());

    // Step 4: Generate recommendations
    let recommendations = client
        .call_tool(
            "generate_recommendations",
            json!({
                "recommendation_type": "training"
            }),
        )
        .await?;

    let recs = recommendations["result"]["training_recommendations"]
        .as_array()
        .unwrap();
    assert!(!recs.is_empty());
    assert!(recs[0]["type"].is_string());
    assert!(recs[0]["priority"].is_string());

    // Step 5: Detect patterns
    let patterns = client
        .call_tool(
            "detect_patterns",
            json!({
                "pattern_type": "training_consistency"
            }),
        )
        .await?;

    assert!(patterns["result"]["pattern_analysis"]["patterns_detected"].is_array());

    // Step 6: Performance trends
    let trends = client
        .call_tool(
            "analyze_performance_trends",
            json!({
                "timeframe": "month",
                "metric": "pace"
            }),
        )
        .await?;

    assert!(trends["result"]["trend_analysis"]["trend_direction"].is_string());

    // All steps completed successfully - this verifies the exact workflow we used
    // to generate the comprehensive fitness report

    server_handle.abort();
    Ok(())
}
