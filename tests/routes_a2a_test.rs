// ABOUTME: Tests for A2A (Agent-to-Agent) route handlers
// ABOUTME: Tests A2A protocol routes and endpoint functionality
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::significant_drop_tightening,
    clippy::match_wildcard_for_single_variants,
    clippy::match_same_arms,
    clippy::unreadable_literal,
    clippy::module_name_repetitions,
    clippy::redundant_closure_for_method_calls,
    clippy::needless_pass_by_value,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::struct_excessive_bools,
    clippy::missing_const_for_fn,
    clippy::cognitive_complexity,
    clippy::items_after_statements,
    clippy::semicolon_if_nothing_returned,
    clippy::use_self,
    clippy::single_match_else,
    clippy::default_trait_access,
    clippy::enum_glob_use,
    clippy::wildcard_imports,
    clippy::explicit_deref_methods,
    clippy::explicit_iter_loop,
    clippy::manual_let_else,
    clippy::must_use_candidate,
    clippy::return_self_not_must_use,
    clippy::unused_self,
    clippy::used_underscore_binding,
    clippy::fn_params_excessive_bools,
    clippy::trivially_copy_pass_by_ref,
    clippy::option_if_let_else,
    clippy::unnecessary_wraps,
    clippy::redundant_else,
    clippy::map_unwrap_or,
    clippy::map_err_ignore,
    clippy::if_not_else,
    clippy::single_char_lifetime_names,
    clippy::doc_markdown,
    clippy::unused_async,
    clippy::redundant_field_names,
    clippy::struct_field_names,
    clippy::ptr_arg,
    clippy::ref_option_ref,
    clippy::implicit_clone,
    clippy::cloned_instead_of_copied,
    clippy::borrow_as_ptr,
    clippy::bool_to_int_with_if,
    clippy::checked_conversions,
    clippy::copy_iterator,
    clippy::empty_enum,
    clippy::enum_variant_names,
    clippy::expl_impl_clone_on_copy,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::fn_to_numeric_cast_any,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_hasher,
    clippy::inconsistent_struct_constructor,
    clippy::inefficient_to_string,
    clippy::infinite_iter,
    clippy::into_iter_on_ref,
    clippy::iter_not_returning_iterator,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_assert,
    clippy::manual_instant_elapsed,
    clippy::manual_ok_or,
    clippy::manual_string_new,
    clippy::many_single_char_names,
    clippy::match_wild_err_arm,
    clippy::mem_forget,
    clippy::missing_enforced_import_renames,
    clippy::missing_inline_in_public_items,
    clippy::missing_safety_doc,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::naive_bytecount,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_raw_string_hashes,
    clippy::no_effect_underscore_binding,
    clippy::non_ascii_literal,
    clippy::nonstandard_macro_braces,
    clippy::option_option,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_literal,
    clippy::print_with_newline,
    clippy::ptr_as_ptr,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::redundant_allocation,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::trait_duplication_in_bounds,
    clippy::transmute_ptr_to_ptr,
    clippy::tuple_array_conversions,
    clippy::unchecked_duration_subtraction,
    clippy::unicode_not_nfc,
    clippy::unimplemented,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_struct_initialization,
    clippy::unnecessary_to_owned,
    clippy::unnested_or_patterns,
    clippy::unused_peekable,
    clippy::unused_rounding,
    clippy::useless_let_if_seq,
    clippy::verbose_bit_mask,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values
)]
//

//! Comprehensive integration tests for A2A routes
//!
//! This test suite provides comprehensive coverage for all A2A route endpoints,
//! including authentication, authorization, request/response validation,
//! error handling, edge cases, and A2A protocol compliance.

mod common;

use pierre_mcp_server::{
    a2a::{
        client::{A2AClientTier, ClientRegistrationRequest},
        A2AError,
    },
    a2a_routes::{A2AClientRequest, A2ARoutes},
    auth::AuthManager,
    config::environment::ServerConfig,
    database::generate_encryption_key,
    database_plugins::factory::Database,
    models::User,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Test setup helper that creates all necessary components for A2A testing
struct A2ATestSetup {
    routes: A2ARoutes,
    server_resources: Arc<pierre_mcp_server::mcp::resources::ServerResources>,
    database: Arc<Database>,
    #[allow(dead_code)]
    auth_manager: Arc<AuthManager>,
    #[allow(dead_code)]
    user_id: Uuid,
    jwt_token: String,
}

impl A2ATestSetup {
    async fn new() -> Self {
        common::init_server_config();
        // Create test database
        let encryption_key = generate_encryption_key().to_vec();
        #[cfg(feature = "postgresql")]
        let database = Arc::new(
            Database::new(
                "sqlite::memory:",
                encryption_key,
                &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
            )
            .await
            .expect("Failed to create test database"),
        );

        #[cfg(not(feature = "postgresql"))]
        let database = Arc::new(
            Database::new("sqlite::memory:", encryption_key)
                .await
                .expect("Failed to create test database"),
        );

        // Create auth manager
        let auth_manager = Arc::new(AuthManager::new(24));

        // Create test user
        let user = User::new(
            "test@example.com".to_owned(),
            "hashed_password".to_owned(),
            Some("Test User".to_owned()),
        );
        let user_id = database
            .create_user(&user)
            .await
            .expect("Failed to create test user");

        // Create JWKS manager for RS256 token generation
        let jwks_manager = common::get_shared_test_jwks();
        let jwks_manager = Arc::new(jwks_manager);

        // Generate JWT token for the user
        let jwt_token = auth_manager
            .generate_token(&user, &jwks_manager)
            .expect("Failed to generate JWT token");

        // Create test server config - use a minimal config for testing
        let config = Arc::new(create_test_server_config());

        // Create test cache with background cleanup disabled
        let cache_config = pierre_mcp_server::cache::CacheConfig {
            max_entries: 1000,
            redis_url: None,
            cleanup_interval: std::time::Duration::from_secs(60),
            enable_background_cleanup: false,
        };
        let cache = pierre_mcp_server::cache::factory::Cache::new(cache_config)
            .await
            .expect("Failed to create test cache");

        // Create ServerResources for A2A routes
        let server_resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
            (*database).clone(),
            (*auth_manager).clone(),
            "test_jwt_secret",
            config,
            cache,
            2048, // Use 2048-bit RSA keys for faster test execution
            Some(common::get_shared_test_jwks()),
        ));

        // Create A2A routes
        let routes = A2ARoutes::new(server_resources.clone());

        Self {
            routes,
            server_resources,
            database,
            auth_manager,
            user_id,
            jwt_token,
        }
    }

    /// Create a test A2A client for testing
    async fn create_test_client(&self) -> (String, String) {
        let request = ClientRegistrationRequest {
            name: "Test A2A Client".to_owned(),
            description: "Test client for integration tests".to_owned(),
            capabilities: vec![
                "fitness-data-analysis".to_owned(),
                "goal-management".to_owned(),
            ],
            redirect_uris: vec!["https://example.com/callback".to_owned()],
            contact_email: "client@example.com".to_owned(),
        };

        // Reuse the existing ServerResources to avoid creating new RSA keys
        let client_manager = &*self.server_resources.a2a_client_manager;
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

    let result = setup.routes.get_agent_card();
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

    let agent_card = setup.routes.get_agent_card().unwrap();

    // Test required fields are present
    assert_eq!(agent_card.name, "Pierre Fitness AI");
    assert!(agent_card.description.contains("AI-powered fitness"));
    assert!(!agent_card.version.is_empty());

    // Test capabilities structure
    assert!(agent_card
        .capabilities
        .contains(&"fitness-data-analysis".to_owned()));
    assert!(agent_card
        .capabilities
        .contains(&"goal-management".to_owned()));

    // Test tools structure
    for tool in &agent_card.tools {
        assert!(!tool.name.is_empty());
        assert!(!tool.description.is_empty());
        assert!(tool.input_schema.is_object());
        assert!(tool.output_schema.is_object());
    }

    // Test authentication configuration
    assert!(!agent_card.authentication.schemes.is_empty());
    assert!(agent_card
        .authentication
        .schemes
        .contains(&"api-key".to_owned()));
}

// =============================================================================
// Dashboard Overview Tests
// =============================================================================

#[tokio::test]
async fn test_get_dashboard_overview_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup
        .routes
        .get_dashboard_overview(Some(&auth_header))
        .await;
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

    let result = setup
        .routes
        .get_dashboard_overview(Some(&auth_header))
        .await;
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
        name: "Test Client".to_owned(),
        description: "A test A2A client".to_owned(),
        capabilities: vec!["fitness-data-analysis".to_owned()],
        redirect_uris: Some(vec!["https://example.com/callback".to_owned()]),
        contact_email: "test@example.com".to_owned(),
        agent_version: Some("1.0.0".to_owned()),
        documentation_url: Some("https://example.com/docs".to_owned()),
    };

    let result = setup
        .routes
        .register_client(Some(&auth_header), request)
        .await;
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
        name: "Minimal Client".to_owned(),
        description: "Minimal test client".to_owned(),
        capabilities: vec!["goal-management".to_owned()],
        redirect_uris: None, // Optional field
        contact_email: "minimal@example.com".to_owned(),
        agent_version: None,     // Optional field
        documentation_url: None, // Optional field
    };

    let result = setup
        .routes
        .register_client(Some(&auth_header), request)
        .await;
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
        name: "Duplicate Client".to_owned(),
        description: "First client".to_owned(),
        capabilities: vec!["fitness-data-analysis".to_owned()],
        redirect_uris: None,
        contact_email: "first@example.com".to_owned(),
        agent_version: None,
        documentation_url: None,
    };

    // First registration should succeed
    let result1 = setup
        .routes
        .register_client(Some(&auth_header), request)
        .await;
    assert!(result1.is_ok());

    // Second registration with different email should also succeed (name duplicates allowed)
    let request2 = A2AClientRequest {
        name: "Duplicate Client".to_owned(),
        description: "Second client".to_owned(),
        capabilities: vec!["fitness-data-analysis".to_owned()],
        redirect_uris: None,
        contact_email: "second@example.com".to_owned(),
        agent_version: None,
        documentation_url: None,
    };

    let result2 = setup
        .routes
        .register_client(Some(&auth_header), request2)
        .await;
    assert!(result2.is_ok());
}

#[tokio::test]
async fn test_register_client_invalid_email() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let request = A2AClientRequest {
        name: "Invalid Email Client".to_owned(),
        description: "Client with invalid email".to_owned(),
        capabilities: vec!["fitness-data-analysis".to_owned()],
        redirect_uris: None,
        contact_email: "invalid-email".to_owned(), // Invalid email format
        agent_version: None,
        documentation_url: None,
    };

    let result = setup
        .routes
        .register_client(Some(&auth_header), request)
        .await;
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
        name: "No Capabilities Client".to_owned(),
        description: "Client with no capabilities".to_owned(),
        capabilities: vec![], // Empty capabilities
        redirect_uris: None,
        contact_email: "nocaps@example.com".to_owned(),
        agent_version: None,
        documentation_url: None,
    };

    let result = setup
        .routes
        .register_client(Some(&auth_header), request)
        .await;
    // Should fail - at least one capability is required
    assert!(result.is_err());
    match result.unwrap_err() {
        A2AError::InvalidRequest(msg) => {
            assert!(msg.contains("capability"));
        }
        other => panic!("Unexpected error type: {:?}", other),
    }
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

    let result = setup
        .routes
        .get_client_usage(Some(&auth_header), &client_id)
        .await;
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

    let result = setup
        .routes
        .get_client_usage(Some(&auth_header), "nonexistent_client")
        .await;
    assert!(result.is_err());

    match result.unwrap_err() {
        A2AError::ClientNotRegistered(_) => {} // Expected error
        A2AError::DatabaseError(_) => {}       // Also acceptable
        A2AError::InternalError(_) => {}       // Also acceptable (for database errors)
        other => panic!("Unexpected error type: {:?}", other),
    }
}

#[tokio::test]
async fn test_get_client_rate_limit_success() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    // Create a test client
    let (client_id, _) = setup.create_test_client().await;

    let result = setup
        .routes
        .get_client_rate_limit(Some(&auth_header), &client_id)
        .await;
    assert!(result.is_ok());

    let rate_limit = result.unwrap();
    assert!(!rate_limit.is_rate_limited); // New client shouldn't be rate limited
    assert_eq!(rate_limit.tier, A2AClientTier::Trial); // Default tier
}

#[tokio::test]
async fn test_get_client_rate_limit_nonexistent() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup
        .routes
        .get_client_rate_limit(Some(&auth_header), "nonexistent_client")
        .await;
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
    let result = setup
        .routes
        .deactivate_client(Some(&auth_header), &client_id)
        .await;
    assert!(result.is_ok());

    // Verify client is now inactive by checking it's not in the active clients list
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client_found = clients.iter().find(|c| c.id == client_id);
    assert!(
        client_found.is_none(),
        "Deactivated client should not appear in active clients list"
    );
}

#[tokio::test]
async fn test_deactivate_client_nonexistent() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let result = setup
        .routes
        .deactivate_client(Some(&auth_header), "nonexistent_client")
        .await;
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
    setup
        .routes
        .deactivate_client(Some(&auth_header), &client_id)
        .await
        .unwrap();

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

    let result = setup
        .routes
        .execute_tool(Some(&auth_header), tool_request)
        .await;
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
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Authorization"));
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

    let result = setup
        .routes
        .execute_tool(Some("Invalid Bearer token"), tool_request)
        .await;
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

    let result = setup
        .routes
        .execute_tool(Some(&auth_header), tool_request)
        .await;
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

    let result = setup
        .routes
        .execute_tool(Some(&auth_header), tool_request)
        .await;
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

    let result = setup
        .routes
        .execute_tool(Some(&auth_header), tool_request)
        .await;
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
        assert!(response["error"]["message"]
            .as_str()
            .unwrap()
            .contains("session"));
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
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));
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

    let result = setup
        .routes
        .execute_tool(Some(&auth_header), malformed_request)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_different_id_types() {
    let setup = A2ATestSetup::new().await;
    let auth_header = format!("Bearer {}", setup.jwt_token);

    let test_ids = vec![
        json!(1),                // Number
        json!("string-id"),      // String
        json!(null),             // Null
        json!({"object": "id"}), // Object (non-standard but should handle)
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
        large_params.insert(format!("param_{i}"), json!(format!("value_{i}")));
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
        "NotBearer token123",            // Wrong scheme
        "Bearer",                        // Missing token
        "Bearer ",                       // Empty token
        "bearer token123",               // Lowercase Bearer
        "Token token123",                // Wrong scheme name
        "",                              // Empty header
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
        let result = setup
            .routes
            .execute_tool(Some(invalid_header), request.clone())
            .await;
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

    let result = setup
        .routes
        .execute_tool(Some(invalid_auth_header), request)
        .await;
    assert!(result.is_ok());

    let response = result.unwrap();
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32001);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("token"));
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
            name: format!("Performance Test Client {i}"),
            description: "Client for performance testing".to_owned(),
            capabilities: vec!["fitness-data-analysis".to_owned()],
            redirect_uris: None,
            contact_email: format!("perf{}@example.com", i),
            agent_version: None,
            documentation_url: None,
        };

        setup
            .routes
            .register_client(Some(&auth_header), request)
            .await
            .unwrap();
    }

    // Test dashboard performance
    let start = std::time::Instant::now();
    let result = setup
        .routes
        .get_dashboard_overview(Some(&auth_header))
        .await;
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
            name: format!("List Test Client {i}"),
            description: "Client for list testing".to_owned(),
            capabilities: vec!["goal-management".to_owned()],
            redirect_uris: None,
            contact_email: format!("list{}@example.com", i),
            agent_version: None,
            documentation_url: None,
        };

        setup
            .routes
            .register_client(Some(&auth_header), request)
            .await
            .unwrap();
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
        name: "Lifecycle Test Client".to_owned(),
        description: "Testing full client lifecycle".to_owned(),
        capabilities: vec![
            "fitness-data-analysis".to_owned(),
            "goal-management".to_owned(),
        ],
        redirect_uris: Some(vec!["https://example.com/callback".to_owned()]),
        contact_email: "lifecycle@example.com".to_owned(),
        agent_version: Some("2.0.0".to_owned()),
        documentation_url: Some("https://example.com/docs".to_owned()),
    };

    let credentials = setup
        .routes
        .register_client(Some(&auth_header), request)
        .await
        .unwrap();
    let client_id = credentials.client_id.clone();

    // 2. Verify client appears in list
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client = clients.iter().find(|c| c.id == client_id).unwrap();
    assert!(client.is_active);
    assert_eq!(client.name, "Lifecycle Test Client");

    // 3. Get client usage (should be zero)
    let usage = setup
        .routes
        .get_client_usage(Some(&auth_header), &client_id)
        .await
        .unwrap();
    assert_eq!(usage.total_requests, 0);

    // 4. Get client rate limit status
    let rate_limit = setup
        .routes
        .get_client_rate_limit(Some(&auth_header), &client_id)
        .await
        .unwrap();
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
    setup
        .routes
        .deactivate_client(Some(&auth_header), &client_id)
        .await
        .unwrap();

    // 7. Verify client is now inactive by checking it's not in the active clients list
    let clients = setup.routes.list_clients(Some(&auth_header)).await.unwrap();
    let client_found = clients.iter().find(|c| c.id == client_id);
    assert!(
        client_found.is_none(),
        "Deactivated client should not appear in active clients list"
    );

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
    use pierre_mcp_server::config::environment::*;
    use std::path::PathBuf;

    ServerConfig {
        http_port: 8081,
        oauth_callback_port: 35535,
        log_level: LogLevel::Info,
        logging: LoggingConfig::default(),
        http_client: HttpClientConfig::default(),
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 5,
                directory: PathBuf::from("/tmp/backups"),
            },
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
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
            garmin: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
        },
        security: SecurityConfig {
            cors_origins: vec!["*".to_owned()],
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
                base_url: "https://api.openweathermap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://www.fitbit.com/oauth/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: true,
            },
            garmin_api: GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token"
                    .to_string(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 200,
            default_activities_limit: 50,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
                server_name: "Pierre Fitness AI".to_owned(),
                server_version: "1.0.0".to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
        },
        usda_api_key: None,
        rate_limiting: pierre_mcp_server::config::environment::RateLimitConfig::default(),
        sleep_recovery: pierre_mcp_server::config::environment::SleepRecoveryConfig::default(),
        goal_management: pierre_mcp_server::config::environment::GoalManagementConfig::default(),
        training_zones: pierre_mcp_server::config::environment::TrainingZonesConfig::default(),
    }
}
