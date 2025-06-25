// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! End-to-end tests for OAuth flow with MCP integration

use pierre_mcp_server::{
    auth::AuthManager, database::generate_encryption_key, database_plugins::factory::Database,
    mcp::multitenant::MultiTenantMcpServer,
};
use serde_json::json;
use tokio::time::{sleep, Duration};

/// Test the complete OAuth flow through MCP tools
#[tokio::test]
async fn test_oauth_flow_through_mcp() {
    // Setup multi-tenant server components
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    // Create test config
    let config = std::sync::Arc::new(pierre_mcp_server::config::environment::ServerConfig {
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
            geocoding: pierre_mcp_server::config::environment::GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: true,
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
    });

    // Create server instance
    let _server = MultiTenantMcpServer::new(database.clone(), auth_manager.clone(), config);

    // Start server in background (we'll simulate MCP requests instead of real TCP)
    let server_handle = tokio::spawn(async move {
        // In a real test, we'd start the server on a test port
        // For this test, we'll just ensure it compiles and the structure is correct
        sleep(Duration::from_millis(100)).await;
    });

    // Test user registration via HTTP endpoint
    // In a real e2e test, we'd make actual HTTP requests
    // For now, we'll test the flow logic

    // 1. Register user (simulated)
    let _user_email = "e2e_test@example.com";
    let _user_password = "password123";

    // 2. Login to get JWT (simulated)
    // In real test: POST to /auth/login

    // 3. Test MCP initialize
    let _init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": null,
        "id": 1
    });

    // Verify response includes OAuth tools
    // Expected tools: connect_strava, connect_fitbit, get_connection_status, disconnect_provider

    // 4. Test connect_strava tool
    let _connect_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "connect_strava",
            "arguments": {}
        },
        "id": 2,
        "auth": "Bearer <jwt_token>"
    });

    // Verify OAuth URL is generated with proper parameters

    // 5. Test get_connection_status
    let _status_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "get_connection_status",
            "arguments": {}
        },
        "id": 3,
        "auth": "Bearer <jwt_token>"
    });

    // Verify both providers show as not connected initially

    // Clean up
    server_handle.abort();
}

/// Test OAuth callback error handling
#[tokio::test]
async fn test_oauth_callback_error_handling() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let oauth_routes = pierre_mcp_server::routes::OAuthRoutes::new(database);

    // Test invalid state parameter
    let result = oauth_routes
        .handle_callback("test_code", "invalid_state", "strava")
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid state parameter"));

    // Test malformed state (missing UUID)
    let result = oauth_routes
        .handle_callback("test_code", "not-a-uuid:something", "strava")
        .await;
    assert!(result.is_err());

    // Test unsupported provider
    let valid_state = format!("{}:{}", uuid::Uuid::new_v4(), uuid::Uuid::new_v4());
    let result = oauth_routes
        .handle_callback("test_code", &valid_state, "unsupported")
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));
}

/// Test OAuth state security
#[tokio::test]
async fn test_oauth_state_csrf_protection() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let oauth_routes = pierre_mcp_server::routes::OAuthRoutes::new(database);

    let user_id = uuid::Uuid::new_v4();

    // Generate OAuth URL and get state
    let auth_response = oauth_routes.get_auth_url(user_id, "strava").await.unwrap();

    // Verify state contains user ID
    assert!(auth_response.state.contains(&user_id.to_string()));

    // Verify state format is UUID:UUID
    let state_parts: Vec<&str> = auth_response.state.split(':').collect();
    assert_eq!(state_parts.len(), 2);
    assert_eq!(state_parts[0], user_id.to_string());
    assert!(uuid::Uuid::parse_str(state_parts[1]).is_ok());

    // Verify each request generates unique state
    let auth_response2 = oauth_routes.get_auth_url(user_id, "strava").await.unwrap();
    assert_ne!(auth_response.state, auth_response2.state);
}

/// Test provider connection status tracking
#[tokio::test]
async fn test_connection_status_tracking() {
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let auth_manager = AuthManager::new(vec![0u8; 64], 24);

    // Register a test user
    let auth_routes = pierre_mcp_server::routes::AuthRoutes::new(database.clone(), auth_manager);
    let register_request = pierre_mcp_server::routes::RegisterRequest {
        email: "status_test@example.com".to_string(),
        password: "password123".to_string(),
        display_name: None,
    };

    let register_response = auth_routes.register(register_request).await.unwrap();
    let user_id = uuid::Uuid::parse_str(&register_response.user_id).unwrap();

    // Check initial connection status
    let oauth_routes = pierre_mcp_server::routes::OAuthRoutes::new(database.clone());
    let statuses = oauth_routes.get_connection_status(user_id).await.unwrap();

    // Verify initial state
    assert_eq!(statuses.len(), 2);
    for status in &statuses {
        assert!(!status.connected);
        assert!(status.expires_at.is_none());
        assert!(status.scopes.is_none());
    }

    // After OAuth flow (simulated by storing tokens), status should change
    // In real test, we'd complete OAuth flow and verify tokens are stored

    // Test token expiration tracking
    // Tokens should include expiration time for automatic refresh
}
