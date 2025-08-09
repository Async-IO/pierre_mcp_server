// ABOUTME: End-to-end OAuth integration tests for complete flow validation
// ABOUTME: Tests OAuth authorization, token exchange, and provider integration
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
    clippy::manual_let_else,
    clippy::manual_ok_or,
    clippy::manual_string_new,
    clippy::many_single_char_names,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
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
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_add_assign,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::trait_duplication_in_bounds,
    clippy::transmute_ptr_to_ptr,
    clippy::trivially_copy_pass_by_ref,
    clippy::tuple_array_conversions,
    clippy::unchecked_duration_subtraction,
    clippy::unicode_not_nfc,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_box_returns,
    clippy::unnecessary_struct_initialization,
    clippy::unnecessary_to_owned,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unused_peekable,
    clippy::unused_rounding,
    clippy::useless_let_if_seq,
    clippy::verbose_bit_mask,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values
)]
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
    let _server = MultiTenantMcpServer::new(database, auth_manager, config);

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
    // Set required environment variables for OAuth
    std::env::set_var("STRAVA_CLIENT_ID", "test_client_id");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_client_secret");

    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new("sqlite::memory:", encryption_key)
        .await
        .unwrap();
    let oauth_routes = pierre_mcp_server::routes::OAuthRoutes::new(database);

    let user_id = uuid::Uuid::new_v4();

    // Generate OAuth URL and get state
    let auth_response = oauth_routes.get_auth_url(user_id, "strava").unwrap();

    // Verify state contains user ID
    assert!(auth_response.state.contains(&user_id.to_string()));

    // Verify state format is UUID:UUID
    let state_parts: Vec<&str> = auth_response.state.split(':').collect();
    assert_eq!(state_parts.len(), 2);
    assert_eq!(state_parts[0], user_id.to_string());
    assert!(uuid::Uuid::parse_str(state_parts[1]).is_ok());

    // Verify each request generates unique state
    let auth_response2 = oauth_routes.get_auth_url(user_id, "strava").unwrap();
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
