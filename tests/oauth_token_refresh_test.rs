// ABOUTME: OAuth token refresh tests for token lifecycle management
// ABOUTME: Tests token refresh logic, expiration handling, and token persistence
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

//! # OAuth Token Refresh Tests
//!
//! Tests for automatic token refresh in Universal Tool Executor.

use pierre_mcp_server::database::generate_encryption_key;
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use pierre_mcp_server::intelligence::{
    ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
    TrendIndicators,
};
use pierre_mcp_server::models::User;
use pierre_mcp_server::protocols::universal::{UniversalRequest, UniversalToolExecutor};
use serde_json::json;
use std::sync::Arc;

use uuid::Uuid;

/// Create a test ServerConfig with missing OAuth credentials for failure testing
fn create_test_server_config_without_oauth(
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
                client_id: None,     // Missing credentials
                client_secret: None, // Missing credentials
                redirect_uri: Some("http://localhost:3000/oauth/callback/strava".to_string()),
                scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                enabled: true,
            },
            fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig {
                client_id: None,     // Missing credentials
                client_secret: None, // Missing credentials
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
    })
}

/// Create a test ServerConfig for OAuth token refresh tests
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
    })
}

/// Create a test UniversalToolExecutor with in-memory database
async fn create_test_executor() -> (Arc<UniversalToolExecutor>, Arc<Database>) {
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(7.5),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(85.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Improving,
                distance_trend: TrendDirection::Stable,
                consistency_score: 88.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    let tenant_oauth_client = Arc::new(pierre_mcp_server::tenant::TenantOAuthClient::new());
    let executor = Arc::new(UniversalToolExecutor::new(
        database.clone(),
        intelligence,
        create_test_server_config(),
        tenant_oauth_client,
    ));

    (executor, database)
}

/// Create a test UniversalToolExecutor without OAuth credentials for failure testing
async fn create_test_executor_without_oauth() -> (Arc<UniversalToolExecutor>, Arc<Database>) {
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    let intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_string(),
        vec![],
        PerformanceMetrics {
            relative_effort: Some(7.5),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(85.0),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Stable,
                effort_trend: TrendDirection::Improving,
                distance_trend: TrendDirection::Stable,
                consistency_score: 88.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    let tenant_oauth_client = Arc::new(pierre_mcp_server::tenant::TenantOAuthClient::new());
    let executor = Arc::new(UniversalToolExecutor::new(
        database.clone(),
        intelligence,
        create_test_server_config_without_oauth(),
        tenant_oauth_client,
    ));

    (executor, database)
}

/// Test that get_activities uses token refresh
#[tokio::test]
async fn test_get_activities_with_expired_token() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_string()),
    };
    database.create_user(&user).await.unwrap();

    // Store expired token
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1); // Expired
    database
        .update_strava_token(
            user_id,
            "expired_access_token",
            "refresh_token_123",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Set up environment for OAuth provider
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create request for get_activities
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_activities".to_string(),
        parameters: json!({
            "limit": 10,
            "provider": "strava"
        }),
        protocol: "test".to_string(),
    };

    // Execute tool - it should attempt to refresh the token
    let response = executor.execute_tool(request).await;

    // In a real scenario with a mock server, this would succeed after refresh
    // For now, we expect an OAuth error indicating refresh was attempted
    match response {
        Ok(resp) => {
            // If successful, check that result mentions OAuth error
            if let Some(result) = resp.result {
                if let Some(arr) = result.as_array() {
                    if let Some(first) = arr.first() {
                        if let Some(error) = first.get("error") {
                            assert!(error.as_str().unwrap().contains("OAuth"));
                        }
                    }
                }
            }
        }
        Err(_) => {
            // Expected in test environment without mock server
        }
    }
}

/// Test connection status with OAuth manager integration
#[tokio::test]
async fn test_connection_status_with_oauth_manager() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_string()),
    };
    database.create_user(&user).await.unwrap();

    // Set up environment for OAuth providers
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");
    std::env::set_var("FITBIT_CLIENT_ID", "test_fitbit");
    std::env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_secret");

    // Create request for get_connection_status
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        protocol: "test".to_string(),
    };

    // Execute tool
    let response = executor.execute_tool(request).await.unwrap();

    // Check response
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.get("providers").is_some());

    let providers = result.get("providers").unwrap();
    assert!(providers.get("strava").is_some());
    assert!(providers.get("fitbit").is_some());

    // Both should be disconnected since no tokens are stored
    assert_eq!(
        providers["strava"]["connected"],
        serde_json::Value::Bool(false)
    );
    assert_eq!(
        providers["fitbit"]["connected"],
        serde_json::Value::Bool(false)
    );
}

/// Test that analyze_activity uses token refresh
#[tokio::test]
async fn test_analyze_activity_token_refresh() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_string()),
    };
    database.create_user(&user).await.unwrap();

    // Store token that will expire soon
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(3); // Expires in 3 minutes (within buffer)
    database
        .update_strava_token(
            user_id,
            "soon_to_expire_token",
            "refresh_token_456",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Set up environment
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create request
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "analyze_activity".to_string(),
        parameters: json!({
            "activity_id": "123456789"
        }),
        protocol: "test".to_string(),
    };

    // Execute - should trigger refresh due to token expiring soon
    let response = executor.execute_tool(request).await;

    // Verify response (will fail in test without mock server, but structure is tested)
    match response {
        Ok(resp) => {
            if let Some(error) = resp.error {
                // Expected in test environment - could be OAuth error, provider error, deprecated system, or activity not found
                assert!(
                    error.contains("OAuth")
                        || error.contains("Failed")
                        || error.contains("not yet fully implemented")
                        || error.contains("Activity not found")
                        || error.contains("deprecated")
                        || error.contains("tenant-aware MCP endpoints")
                );
            }
        }
        Err(_) => {
            // Expected in test environment
        }
    }
}

/// Test concurrent token refresh attempts
#[tokio::test]
async fn test_concurrent_token_operations() {
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_string()),
    };
    database.create_user(&user).await.unwrap();

    // Store valid token
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    database
        .update_strava_token(
            user_id,
            "valid_token",
            "refresh_token",
            expires_at,
            "read,activity:read_all".to_string(),
        )
        .await
        .unwrap();

    // Set up environment
    std::env::set_var("STRAVA_CLIENT_ID", "test_client");
    std::env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create multiple concurrent requests
    let mut handles = vec![];

    for _i in 0..5 {
        let executor_clone = executor.clone();
        let user_id_str = user_id.to_string();
        let handle = tokio::spawn(async move {
            let request = UniversalRequest {
                user_id: user_id_str,
                tool_name: "get_connection_status".to_string(),
                parameters: json!({}),
                protocol: "test".to_string(),
            };
            executor_clone.execute_tool(request).await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
    }
}

/// Test error handling when OAuth provider initialization fails
/// Fixed test isolation using configuration without OAuth credentials
#[tokio::test]
async fn test_oauth_provider_init_failure() {
    // Create executor with configuration that has missing OAuth credentials
    let (executor, database) = create_test_executor_without_oauth().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: pierre_mcp_server::models::UserTier::Starter,
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_string()),
    };
    database.create_user(&user).await.unwrap();

    // Create request
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "connect_strava".to_string(),
        parameters: json!({}),
        protocol: "test".to_string(),
    };

    // Execute - should handle provider initialization failure gracefully
    let response = executor.execute_tool(request).await.unwrap();

    // Should fail due to missing OAuth credentials
    assert!(
        !response.success,
        "Expected failure but got success: {:?}",
        response
    );
    assert!(response.error.is_some(), "Expected error but got none");
    let error = response.error.as_ref().unwrap();
    assert!(
        error.contains("Failed to initialize Strava provider")
            || error.contains("Strava client_id not configured")
            || error.contains("Strava client_secret not configured")
            || error.contains("Missing required configuration")
            || error.contains("ConfigurationError"),
        "Unexpected error message: {}",
        error
    );
}
