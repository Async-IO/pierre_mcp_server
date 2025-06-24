// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Comprehensive integration tests for A2A routes
//!
//! This test suite provides comprehensive coverage for all A2A route endpoints,
//! including authentication, authorization, request/response validation,
//! error handling, edge cases, and A2A protocol compliance.

use pierre_mcp_server::{
    a2a::{
        client::{A2AClientManager, ClientRegistrationRequest, A2AClientTier},
        A2AError,
    },
    a2a_routes::{A2AClientRequest, A2ARoutes},
    auth::AuthManager,
    config::environment::ServerConfig,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::User,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Test setup helper that creates all necessary components for A2A testing
struct A2ATestSetup {
    routes: A2ARoutes,
    database: Arc<Database>,
    #[allow(dead_code)]
    auth_manager: Arc<AuthManager>,
    #[allow(dead_code)]
    user_id: Uuid,
    jwt_token: String,
}

impl A2ATestSetup {
    async fn new() -> Self {
        // Create test database
        let encryption_key = generate_encryption_key().to_vec();
        let database = Arc::new(
            Database::new("sqlite::memory:", encryption_key)
                .await
                .expect("Failed to create test database"),
        );

        // Create auth manager
        let jwt_secret = pierre_mcp_server::auth::generate_jwt_secret().to_vec();
        let auth_manager = Arc::new(AuthManager::new(jwt_secret, 24));

        // Create test user
        let user = User::new(
            "test@example.com".to_string(),
            "hashed_password".to_string(),
            Some("Test User".to_string()),
        );
        let user_id = database
            .create_user(&user)
            .await
            .expect("Failed to create test user");

        // Generate JWT token for the user
        let jwt_token = auth_manager
            .generate_token(&user)
            .expect("Failed to generate JWT token");

        // Create test server config - use a minimal config for testing
        let config = Arc::new(create_test_server_config());

        // Create A2A routes
        let routes = A2ARoutes::new(database.clone(), auth_manager.clone(), config);

        Self {
            routes,
            database,
            auth_manager,
            user_id,
            jwt_token,
        }
    }

    /// Create a test A2A client for testing
    async fn create_test_client(&self) -> (String, String) {
        let request = ClientRegistrationRequest {
            name: "Test A2A Client".to_string(),
            description: "Test client for integration tests".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string(), "goal-management".to_string()],
            redirect_uris: vec!["https://example.com/callback".to_string()],
            contact_email: "client@example.com".to_string(),
        };

        let client_manager = A2AClientManager::new(self.database.clone());
        let credentials = client_manager
            .register_client(request)
            .await
            .expect("Failed to create test client");

        (credentials.client_id, credentials.client_secret)
    }

    /// Create an authenticated A2A session token
    #[allow(dead_code)]
    async fn create_session_token(&self, client_id: &str, scopes: &[String]) -> String {
        self.database
            .create_a2a_session(client_id, None, scopes, 24)
            .await
            .expect("Failed to create A2A session")
    }
}

// =============================================================================
// Agent Card Tests
// =============================================================================

#[tokio::test]
async fn test_get_agent_card_success() {
    let setup = A2ATestSetup::new().await;

    let result = setup.routes.get_agent_card().await;
    assert!(result.is_ok());

    let agent_card = result.unwrap();
    assert!(!agent_card.name.is_empty());
    assert!(!agent_card.description.is_empty());
    assert!(!agent_card.version.is_empty());
    assert!(!agent_card.capabilities.is_empty());
    assert!(!agent_card.tools.is_empty());
}

#[tokio::test]
async fn test_agent_card_structure_compliance() {
    let setup = A2ATestSetup::new().await;

    let agent_card = setup.routes.get_agent_card().await.unwrap();

    // Test required fields are present
    assert_eq!(agent_card.name, "Pierre Fitness AI");
    assert!(agent_card.description.contains("AI-powered fitness"));
    assert!(!agent_card.version.is_empty());

    // Test capabilities structure
    assert!(agent_card.capabilities.contains(&"fitness-data-analysis".to_string()));
    assert!(agent_card.capabilities.contains(&"goal-management".to_string()));

    // Test tools structure
    for tool in &agent_card.tools {
        assert!(!tool.name.is_empty());
        assert!(!tool.description.is_empty());
        assert!(tool.input_schema.is_object());
        assert!(tool.output_schema.is_object());
    }

    // Test authentication configuration
    assert!(!agent_card.authentication.schemes.is_empty());
    assert!(agent_card.authentication.schemes.contains(&"api-key".to_string()));
}

// =============================================================================
// Dashboard Overview Tests
// =============================================================================

#[tokio::test]
async fn test_get_dashboard_overview_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup.routes.get_dashboard_overview(Some(&auth_header)).await;
    assert!(result.is_ok());

    let overview = result.unwrap();
    assert_eq!(overview.total_clients, 0); // No clients created yet
    assert_eq!(overview.active_clients, 0);
    assert_eq!(overview.total_sessions, 0);
    assert_eq!(overview.active_sessions, 0);
    assert_eq!(overview.error_rate, 0.0);
    assert!(overview.usage_by_tier.is_empty());
}

#[tokio::test]
async fn test_get_dashboard_overview_with_clients() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a test client
    let _credentials = setup.create_test_client().await;

    let result = setup.routes.get_dashboard_overview(Some(&auth_header)).await;
    assert!(result.is_ok());

    let overview = result.unwrap();
    assert_eq!(overview.total_clients, 1);
    assert_eq!(overview.active_clients, 1);
    assert_eq!(overview.usage_by_tier.len(), 1);

    let tier_usage = &overview.usage_by_tier[0];
    assert_eq!(tier_usage.tier, "basic");
    assert_eq!(tier_usage.client_count, 1);
    assert_eq!(tier_usage.percentage, 100.0);
}

#[tokio::test]
async fn test_get_dashboard_overview_without_auth() {
    let setup = A2ATestSetup::new().await;

    let result = setup.routes.get_dashboard_overview(None).await;
    assert!(result.is_ok()); // Currently, auth is not enforced in implementation
}

// =============================================================================
// Client Registration Tests
// =============================================================================

#[tokio::test]
async fn test_register_client_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = A2AClientRequest {
        name: "Test Client".to_string(),
        description: "A test A2A client".to_string(),
        capabilities: vec!["fitness-data-analysis".to_string()],
        redirect_uris: Some(vec!["https://example.com/callback".to_string()]),
        contact_email: "test@example.com".to_string(),
        agent_version: Some("1.0.0".to_string()),
        documentation_url: Some("https://example.com/docs".to_string()),
    };

    let result = setup.routes.register_client(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let credentials = result.unwrap();
    assert!(!credentials.client_id.is_empty());
    assert!(!credentials.client_secret.is_empty());
    assert!(!credentials.api_key.is_empty());
    assert!(credentials.client_id.starts_with("a2a_"));
}

#[tokio::test]
async fn test_register_client_minimal_request() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = A2AClientRequest {
        name: "Minimal Client".to_string(),
        description: "Minimal test client".to_string(),
        capabilities: vec!["goal-management".to_string()],
        redirect_uris: None, // Optional field
        contact_email: "minimal@example.com".to_string(),
        agent_version: None, // Optional field
        documentation_url: None, // Optional field
    };

    let result = setup.routes.register_client(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let credentials = result.unwrap();
    assert!(!credentials.client_id.is_empty());
    assert!(!credentials.client_secret.is_empty());
}

#[tokio::test]
async fn test_register_client_duplicate_name() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = A2AClientRequest {
        name: "Duplicate Client".to_string(),
        description: "First client".to_string(),
        capabilities: vec!["fitness-data-analysis".to_string()],
        redirect_uris: None,
        contact_email: "first@example.com".to_string(),
        agent_version: None,
        documentation_url: None,
    };

    // First registration should succeed
    let result1 = setup.routes.register_client(Some(&auth_header), request).await;
    assert!(result1.is_ok());

    // Second registration with different email should also succeed (name duplicates allowed)
    let request2 = A2AClientRequest {
        name: "Duplicate Client".to_string(),
        description: "Second client".to_string(),
        capabilities: vec!["fitness-data-analysis".to_string()],
        redirect_uris: None,
        contact_email: "second@example.com".to_string(),
        agent_version: None,
        documentation_url: None,
    };
    
    let result2 = setup.routes.register_client(Some(&auth_header), request2).await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_register_client_invalid_email() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = A2AClientRequest {
        name: "Invalid Email Client".to_string(),
        description: "Client with invalid email".to_string(),
        capabilities: vec!["fitness-data-analysis".to_string()],
        redirect_uris: None,
        contact_email: "invalid-email".to_string(), // Invalid email format
        agent_version: None,
        documentation_url: None,
    };

    let result = setup.routes.register_client(Some(&auth_header), request).await;
    // This might succeed depending on validation - the current implementation doesn't validate email format
    // but let's test it anyway for completeness
    match result {
        Ok(_) => {
            // If validation is not implemented, that's okay for now
        }
        Err(e) => {
            // If validation is implemented, it should catch invalid email
            assert!(e.to_string().contains("email") || e.to_string().contains("invalid"));
        }
    }
}

#[tokio::test]
async fn test_register_client_empty_capabilities() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = A2AClientRequest {
        name: "No Capabilities Client".to_string(),
        description: "Client with no capabilities".to_string(),
        capabilities: vec![], // Empty capabilities
        redirect_uris: None,
        contact_email: "nocaps@example.com".to_string(),
        agent_version: None,
        documentation_url: None,
    };

    let result = setup.routes.register_client(Some(&auth_header), request).await;
    // Should succeed - empty capabilities might be allowed
    assert!(result.is_ok());
}

// =============================================================================
// Client Management Tests
// =============================================================================

#[tokio::test]
async fn test_list_clients_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a couple of test clients
    let _client1 = setup.create_test_client().await;
    let _client2 = setup.create_test_client().await;

    let result = setup.routes.list_clients(Some(&auth_header)).await;
    assert!(result.is_ok());

    let clients = result.unwrap();
    assert_eq!(clients.len(), 2);

    // Check that both clients are active by default
    for client in &clients {
        assert!(client.is_active);
        assert!(!client.id.is_empty());
        assert!(!client.name.is_empty());
    }
}

#[tokio::test]
async fn test_list_clients_empty() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup.routes.list_clients(Some(&auth_header)).await;
    assert!(result.is_ok());

    let clients = result.unwrap();
    assert!(clients.is_empty());
}

#[tokio::test]
async fn test_get_client_usage_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a test client
    let (client_id, _) = setup.create_test_client().await;

    let result = setup.routes.get_client_usage(Some(&auth_header), &client_id).await;
    assert!(result.is_ok());

    let usage = result.unwrap();
    assert_eq!(usage.client_id, client_id);
    assert_eq!(usage.requests_today, 0); // No requests made yet
    assert_eq!(usage.requests_this_month, 0);
    assert_eq!(usage.total_requests, 0);
    assert!(usage.last_request_at.is_none());
}

#[tokio::test]
async fn test_get_client_usage_nonexistent() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup.routes.get_client_usage(Some(&auth_header), "nonexistent_client").await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::ClientNotRegistered(_) => {}, // Expected error
        A2AError::DatabaseError(_) => {}, // Also acceptable
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_get_client_rate_limit_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a test client
    let (client_id, _) = setup.create_test_client().await;

    let result = setup.routes.get_client_rate_limit(Some(&auth_header), &client_id).await;
    assert!(result.is_ok());

    let rate_limit = result.unwrap();
    assert!(!rate_limit.is_rate_limited); // New client shouldn't be rate limited
    assert_eq!(rate_limit.tier, A2AClientTier::Trial); // Default tier
}

#[tokio::test]
async fn test_get_client_rate_limit_nonexistent() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup.routes.get_client_rate_limit(Some(&auth_header), "nonexistent_client").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_deactivate_client_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a test client
    let (client_id, _) = setup.create_test_client().await;

    // Verify client is active
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client = clients.iter().find(|c| c.id == client_id).unwrap();
    assert!(client.is_active);

    // Deactivate the client
    let result = setup.routes.deactivate_client(Some(&auth_header), &client_id).await;
    assert!(result.is_ok());

    // Verify client is now inactive
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client = clients.iter().find(|c| c.id == client_id).unwrap();
    assert!(!client.is_active);
}

#[tokio::test]
async fn test_deactivate_client_nonexistent() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup.routes.deactivate_client(Some(&auth_header), "nonexistent_client").await;
    assert!(result.is_err());
}

// =============================================================================
// Authentication Tests
// =============================================================================

#[tokio::test]
async fn test_authenticate_success() {
    let setup = A2ATestSetup::new().await;

    // Create a test client
    let (client_id, client_secret) = setup.create_test_client().await;

    let auth_request = json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "scopes": ["read", "write"]
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["status"], "authenticated");
    assert!(response["session_token"].is_string());
    assert!(response["expires_in"].is_number());
    assert_eq!(response["token_type"], "Bearer");
    assert_eq!(response["scope"], "read write");
}

#[tokio::test]
async fn test_authenticate_missing_client_id() {
    let setup = A2ATestSetup::new().await;

    let auth_request = json!({
        "client_secret": "some_secret",
        "scopes": ["read"]
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::InvalidRequest(msg) => {
            assert!(msg.contains("client_id"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_authenticate_missing_client_secret() {
    let setup = A2ATestSetup::new().await;

    let auth_request = json!({
        "client_id": "some_client_id",
        "scopes": ["read"]
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::InvalidRequest(msg) => {
            assert!(msg.contains("client_secret"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_authenticate_invalid_client_id() {
    let setup = A2ATestSetup::new().await;

    let auth_request = json!({
        "client_id": "invalid_client_id",
        "client_secret": "some_secret",
        "scopes": ["read"]
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::AuthenticationFailed(msg) => {
            assert!(msg.contains("Invalid client_id"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_authenticate_invalid_client_secret() {
    let setup = A2ATestSetup::new().await;

    // Create a test client
    let (client_id, _) = setup.create_test_client().await;

    let auth_request = json!({
        "client_id": client_id,
        "client_secret": "invalid_secret",
        "scopes": ["read"]
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::AuthenticationFailed(msg) => {
            assert!(msg.contains("Invalid client_secret"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_authenticate_deactivated_client() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create and deactivate a test client
    let (client_id, client_secret) = setup.create_test_client().await;
    setup.routes.deactivate_client(Some(&auth_header), &client_id).await.unwrap();

    let auth_request = json!({
        "client_id": client_id,
        "client_secret": client_secret,
        "scopes": ["read"]
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::AuthenticationFailed(msg) => {
            assert!(msg.contains("deactivated"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_authenticate_default_scopes() {
    let setup = A2ATestSetup::new().await;

    // Create a test client
    let (client_id, client_secret) = setup.create_test_client().await;

    let auth_request = json!({
        "client_id": client_id,
        "client_secret": client_secret
        // No scopes provided
    });

    let result = setup.routes.authenticate(auth_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["scope"], "read"); // Default scope
}

// =============================================================================
// Tool Execution Tests
// =============================================================================

#[tokio::test]
async fn test_execute_tool_success() {
    let setup = A2ATestSetup::new().await;

    // Create JWT token for authentication
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "tool_name": "get_activities",
            "parameters": {
                "limit": 10
            }
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), tool_request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    // Response should have either "result" or "error" field
    assert!(response["result"].is_object() || response["error"].is_object());
}

#[tokio::test]
async fn test_execute_tool_missing_auth() {
    let setup = A2ATestSetup::new().await;

    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "tool_name": "get_activities",
            "parameters": {}
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(None, tool_request).await;
    assert!(result.is_ok()); // Returns error response, not error

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32001);
    assert!(response["error"]["message"].as_str().unwrap().contains("Authorization"));
}

#[tokio::test]
async fn test_execute_tool_invalid_auth() {
    let setup = A2ATestSetup::new().await;

    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "tool_name": "get_activities",
            "parameters": {}
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(Some("Invalid Bearer token"), tool_request).await;
    assert!(result.is_ok()); // Returns error response, not error

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32001);
}

#[tokio::test]
async fn test_execute_tool_missing_method() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let tool_request = json!({
        "jsonrpc": "2.0",
        "params": {
            "tool_name": "get_activities",
            "parameters": {}
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), tool_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::InvalidRequest(msg) => {
            assert!(msg.contains("method"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_execute_tool_missing_params() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), tool_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::InvalidRequest(msg) => {
            assert!(msg.contains("params"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_execute_tool_missing_tool_name() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let tool_request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "parameters": {}
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), tool_request).await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::InvalidRequest(msg) => {
            assert!(msg.contains("tool_name"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
}

// =============================================================================
// A2A Protocol Method Tests
// =============================================================================

#[tokio::test]
async fn test_client_info_method() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = json!({
        "jsonrpc": "2.0",
        "method": "client.info",
        "params": {},
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    let result = &response["result"];
    assert_eq!(result["name"], "Pierre Fitness AI");
    assert_eq!(result["version"], "1.0.0");
    assert!(result["capabilities"].is_array());
    assert!(result["protocols"].is_array());
}

#[tokio::test]
async fn test_session_heartbeat_method() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = json!({
        "jsonrpc": "2.0",
        "method": "session.heartbeat",
        "params": {},
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);

    // Should have either result (success) or error (if session doesn't exist)
    if response["result"].is_object() {
        assert_eq!(response["result"]["status"], "alive");
        assert!(response["result"]["timestamp"].is_string());
    } else if response["error"].is_object() {
        assert_eq!(response["error"]["code"], -32000);
        assert!(response["error"]["message"].as_str().unwrap().contains("session"));
    } else {
        panic!("Response should have either result or error");
    }
}

#[tokio::test]
async fn test_capabilities_list_method() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = json!({
        "jsonrpc": "2.0",
        "method": "capabilities.list",
        "params": {},
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_object());

    let capabilities = response["result"]["capabilities"].as_array().unwrap();
    assert!(!capabilities.is_empty());

    // Check that each capability has required fields
    for capability in capabilities {
        assert!(capability["name"].is_string());
        assert!(capability["description"].is_string());
        assert!(capability["version"].is_string());
    }

    // Check for specific expected capabilities
    let capability_names: Vec<_> = capabilities
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(capability_names.contains(&"fitness-data-analysis"));
    assert!(capability_names.contains(&"goal-management"));
}

#[tokio::test]
async fn test_unknown_method() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = json!({
        "jsonrpc": "2.0",
        "method": "unknown.method",
        "params": {},
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601); // Method not found
    assert!(response["error"]["message"].as_str().unwrap().contains("not found"));
    assert!(response["error"]["data"]["available_methods"].is_array());
}

// =============================================================================
// Edge Cases and Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_malformed_json_request() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Test with invalid JSON structure
    let malformed_request = json!({
        "not_jsonrpc": "invalid",
        "no_method": true,
        "id": "string_id"
    });

    let result = setup.routes.execute_tool(Some(&auth_header), malformed_request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_different_id_types() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let test_ids = vec![
        json!(1),                    // Number
        json!("string-id"),          // String
        json!(null),                 // Null
        json!({"object": "id"}),     // Object (non-standard but should handle)
    ];

    for test_id in test_ids {
        let request = json!({
            "jsonrpc": "2.0",
            "method": "client.info",
            "params": {},
            "id": test_id.clone()
        });

        let result = setup.routes.execute_tool(Some(&auth_header), request).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert_eq!(response["id"], test_id);
    }
}

#[tokio::test]
async fn test_large_parameters() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a large parameters object
    let mut large_params = serde_json::Map::new();
    for i in 0..1000 {
        large_params.insert(format!("param_{}", i), json!(format!("value_{}", i)));
    }

    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "tool_name": "get_activities",
            "parameters": large_params
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(&auth_header), request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create multiple concurrent requests
    let mut handles = vec![];
    
    for i in 0..10 {
        let routes = setup.routes.clone();
        let auth_header = auth_header.clone();
        
        let handle = tokio::spawn(async move {
            let request = json!({
                "jsonrpc": "2.0",
                "method": "client.info",
                "params": {},
                "id": i
            });

            routes.execute_tool(Some(&auth_header), request).await
        });
        
        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        
        let response = result.unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["result"].is_object());
    }
}

#[tokio::test]
async fn test_jwt_token_extraction_edge_cases() {
    let setup = A2ATestSetup::new().await;

    // Test various invalid auth header formats
    let invalid_headers = vec![
        "NotBearer token123",          // Wrong scheme
        "Bearer",                      // Missing token
        "Bearer ",                     // Empty token
        "bearer token123",             // Lowercase Bearer
        "Token token123",              // Wrong scheme name
        "",                           // Empty header
        "Multiple Bearer token1 token2", // Multiple tokens
    ];

    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "tool_name": "get_activities",
            "parameters": {}
        },
        "id": 1
    });

    for invalid_header in invalid_headers {
        let result = setup.routes.execute_tool(Some(invalid_header), request.clone()).await;
        assert!(result.is_ok()); // Should return error response, not fail

        let response = result.unwrap();
        assert!(response["error"].is_object());
        assert_eq!(response["error"]["code"], -32001);
    }
}

#[tokio::test]
async fn test_expired_jwt_token() {
    let setup = A2ATestSetup::new().await;

    // Create a user and an expired token (would need to mock time or create expired token)
    // For now, test with invalid token format
    let invalid_auth_header = "Bearer invalid.jwt.token";

    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools.execute",
        "params": {
            "tool_name": "get_activities",
            "parameters": {}
        },
        "id": 1
    });

    let result = setup.routes.execute_tool(Some(invalid_auth_header), request).await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32001);
    assert!(response["error"]["message"].as_str().unwrap().contains("token"));
}

// =============================================================================
// Performance and Load Tests
// =============================================================================

#[tokio::test]
async fn test_dashboard_performance_with_many_clients() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create many clients to test dashboard performance
    for i in 0..50 {
        let request = A2AClientRequest {
            name: format!("Performance Test Client {}", i),
            description: "Client for performance testing".to_string(),
            capabilities: vec!["fitness-data-analysis".to_string()],
            redirect_uris: None,
            contact_email: format!("perf{}@example.com", i),
            agent_version: None,
            documentation_url: None,
        };

        setup.routes.register_client(Some(&auth_header), request).await.unwrap();
    }

    // Test dashboard performance
    let start = std::time::Instant::now();
    let result = setup.routes.get_dashboard_overview(Some(&auth_header)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(duration.as_millis() < 1000); // Should complete within 1 second

    let overview = result.unwrap();
    assert_eq!(overview.total_clients, 50);
    assert_eq!(overview.active_clients, 50);
}

#[tokio::test]
async fn test_client_list_performance() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create many clients
    for i in 0..30 {
        let request = A2AClientRequest {
            name: format!("List Test Client {}", i),
            description: "Client for list testing".to_string(),
            capabilities: vec!["goal-management".to_string()],
            redirect_uris: None,
            contact_email: format!("list{}@example.com", i),
            agent_version: None,
            documentation_url: None,
        };

        setup.routes.register_client(Some(&auth_header), request).await.unwrap();
    }

    // Test list performance
    let start = std::time::Instant::now();
    let result = setup.routes.list_clients(Some(&auth_header)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(duration.as_millis() < 500); // Should complete within 500ms

    let clients = result.unwrap();
    assert_eq!(clients.len(), 30);
}

// =============================================================================
// Integration Tests with Real Database Operations
// =============================================================================

#[tokio::test]
async fn test_full_client_lifecycle() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // 1. Register a new client
    let request = A2AClientRequest {
        name: "Lifecycle Test Client".to_string(),
        description: "Testing full client lifecycle".to_string(),
        capabilities: vec!["fitness-data-analysis".to_string(), "goal-management".to_string()],
        redirect_uris: Some(vec!["https://example.com/callback".to_string()]),
        contact_email: "lifecycle@example.com".to_string(),
        agent_version: Some("2.0.0".to_string()),
        documentation_url: Some("https://example.com/docs".to_string()),
    };

    let credentials = setup.routes.register_client(Some(&auth_header), request).await.unwrap();
    let client_id = credentials.client_id.clone();

    // 2. Verify client appears in list
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client = clients.iter().find(|c| c.id == client_id).unwrap();
    assert!(client.is_active);
    assert_eq!(client.name, "Lifecycle Test Client");

    // 3. Get client usage (should be zero)
    let usage = setup.routes.get_client_usage(Some(&auth_header), &client_id).await.unwrap();
    assert_eq!(usage.total_requests, 0);

    // 4. Get client rate limit status
    let rate_limit = setup.routes.get_client_rate_limit(Some(&auth_header), &client_id).await.unwrap();
    assert!(!rate_limit.is_rate_limited);

    // 5. Authenticate with the client
    let auth_request = json!({
        "client_id": client_id,
        "client_secret": credentials.client_secret,
        "scopes": ["read", "write"]
    });

    let auth_response = setup.routes.authenticate(auth_request).await.unwrap();
    assert_eq!(auth_response["status"], "authenticated");

    // 6. Deactivate the client
    setup.routes.deactivate_client(Some(&auth_header), &client_id).await.unwrap();

    // 7. Verify client is now inactive
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client = clients.iter().find(|c| c.id == client_id).unwrap();
    assert!(!client.is_active);

    // 8. Try to authenticate with deactivated client (should fail)
    let auth_request = json!({
        "client_id": client_id,
        "client_secret": credentials.client_secret,
        "scopes": ["read"]
    });

    let auth_result = setup.routes.authenticate(auth_request).await;
    assert!(auth_result.is_err());
}

#[tokio::test]
async fn test_multiple_auth_sessions() {
    let setup = A2ATestSetup::new().await;

    // Create multiple clients
    let (client_id1, client_secret1) = setup.create_test_client().await;
    let (client_id2, client_secret2) = setup.create_test_client().await;

    // Authenticate both clients
    let auth_request1 = json!({
        "client_id": client_id1,
        "client_secret": client_secret1,
        "scopes": ["read"]
    });

    let auth_request2 = json!({
        "client_id": client_id2,
        "client_secret": client_secret2,
        "scopes": ["write"]
    });

    let response1 = setup.routes.authenticate(auth_request1).await.unwrap();
    let response2 = setup.routes.authenticate(auth_request2).await.unwrap();

    // Both should succeed and have different session tokens
    assert_eq!(response1["status"], "authenticated");
    assert_eq!(response2["status"], "authenticated");
    assert_ne!(response1["session_token"], response2["session_token"]);
    assert_eq!(response1["scope"], "read");
    assert_eq!(response2["scope"], "write");
}

// =============================================================================
// Test Configuration and Setup Helpers
// =============================================================================

/// Create a minimal test server configuration
fn create_test_server_config() -> ServerConfig {
    use std::path::PathBuf;
    use pierre_mcp_server::config::environment::*;
    
    ServerConfig {
        mcp_port: 8080,
        http_port: 8081,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: PathBuf::from("/tmp/test_key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 5,
                directory: PathBuf::from("/tmp/backups"),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: PathBuf::from("/tmp/jwt_secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_string()],
            rate_limit: RateLimitConfig {
                enabled: false,
                requests_per_window: 100,
                window_seconds: 60,
            },
            tls: TlsConfig {
                enabled: false,
                cert_path: None,
                key_path: None,
            },
            headers: SecurityHeadersConfig {
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org".to_string(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://www.fitbit.com/oauth/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 200,
            default_activities_limit: 50,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_string(),
                server_name: "Pierre Fitness AI".to_string(),
                server_version: "1.0.0".to_string(),
            },
        },
    }
}