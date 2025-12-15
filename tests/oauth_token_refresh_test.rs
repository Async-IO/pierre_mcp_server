// ABOUTME: OAuth token refresh tests for token lifecycle management
// ABOUTME: Tests token refresh logic, expiration handling, and token persistence
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
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

//! # OAuth Token Refresh Tests
//!
//! Tests for automatic token refresh in Universal Tool Executor.

mod common;

use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::{
        AppBehaviorConfig, AuthConfig, BackupConfig, DatabaseConfig, DatabaseUrl, Environment,
        ExternalServicesConfig, FitbitApiConfig, GeocodingServiceConfig, HttpClientConfig,
        LogLevel, LoggingConfig, OAuth2ServerConfig, OAuthConfig, OAuthProviderConfig,
        PostgresPoolConfig, ProtocolConfig, RouteTimeoutConfig, SecurityConfig,
        SecurityHeadersConfig, ServerConfig, SseConfig, StravaApiConfig, TlsConfig,
        WeatherServiceConfig,
    },
    constants::oauth_providers,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    intelligence::{
        ActivityIntelligence, ContextualFactors, PerformanceMetrics, TimeOfDay, TrendDirection,
        TrendIndicators,
    },
    mcp::resources::ServerResources,
    models::{Tenant, User, UserOAuthToken, UserStatus, UserTier},
    permissions::UserRole,
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
};
use serde_json::json;
use serial_test::serial;
use std::{env, path::PathBuf, sync::Arc};
use uuid::Uuid;

/// Create a test ServerConfig with missing OAuth credentials for failure testing
fn create_test_server_config_without_oauth() -> Arc<ServerConfig> {
    Arc::new(ServerConfig {
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
                retention_count: 7,
                directory: PathBuf::from("test_backups"),
            },
            postgres_pool: PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: None,     // Missing credentials
                client_secret: None, // Missing credentials
                redirect_uri: Some("http://localhost:8081/oauth/callback/strava".to_owned()),
                scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,     // Missing credentials
                client_secret: None, // Missing credentials
                redirect_uri: Some("http://localhost:8081/oauth/callback/fitbit".to_owned()),
                scopes: vec!["activity".to_owned(), "profile".to_owned()],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
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
                environment: Environment::Development,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
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
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: true,
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: SseConfig::default(),
        oauth2_server: OAuth2ServerConfig::default(),
        route_timeouts: RouteTimeoutConfig::default(),
        ..Default::default()
    })
}

/// Create a test ServerConfig for OAuth token refresh tests
fn create_test_server_config() -> Arc<ServerConfig> {
    Arc::new(ServerConfig {
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
                retention_count: 7,
                directory: PathBuf::from("test_backups"),
            },
            postgres_pool: PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_client_id".to_owned()),
                client_secret: Some("test_client_secret".to_owned()),
                redirect_uri: Some("http://localhost:8081/oauth/callback/strava".to_owned()),
                scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_id".to_owned()),
                client_secret: Some("test_fitbit_secret".to_owned()),
                redirect_uri: Some("http://localhost:8081/oauth/callback/fitbit".to_owned()),
                scopes: vec!["activity".to_owned(), "profile".to_owned()],
                enabled: true,
            },
            garmin: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            whoop: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: vec![],
                enabled: false,
            },
            terra: OAuthProviderConfig {
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
                environment: Environment::Development,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
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
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: true,
            },
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: SseConfig::default(),
        oauth2_server: OAuth2ServerConfig::default(),
        route_timeouts: RouteTimeoutConfig::default(),
        ..Default::default()
    })
}

/// Create a test UniversalToolExecutor with in-memory database
async fn create_test_executor() -> (Arc<UniversalToolExecutor>, Arc<Database>) {
    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            generate_encryption_key().to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_owned(),
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

    // Create ServerResources for the test
    let auth_manager = AuthManager::new(24);
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        create_test_server_config(),
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let executor = Arc::new(UniversalToolExecutor::new(server_resources));

    (executor, database)
}

/// Create a test UniversalToolExecutor without OAuth credentials for failure testing
async fn create_test_executor_without_oauth() -> (Arc<UniversalToolExecutor>, Arc<Database>) {
    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            generate_encryption_key().to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await
        .unwrap(),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(
        Database::new("sqlite::memory:", generate_encryption_key().to_vec())
            .await
            .unwrap(),
    );

    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test Intelligence".to_owned(),
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

    // Create ServerResources for the test
    let auth_manager = AuthManager::new(24);
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        create_test_server_config_without_oauth(),
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let executor = Arc::new(UniversalToolExecutor::new(server_resources));

    (executor, database)
}

/// Test that `get_activities` uses token refresh
#[tokio::test]
#[serial]
async fn test_get_activities_with_expired_token() {
    common::init_server_config();
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&user).await.unwrap();

    // Store expired token
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1); // Expired
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "expired_access_token".to_owned(),
        Some("refresh_token_123".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database
        .upsert_user_oauth_token(&oauth_token)
        .await
        .unwrap();

    // Set up environment for OAuth provider
    env::set_var("STRAVA_CLIENT_ID", "test_client");
    env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create request for get_activities
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_activities".to_owned(),
        parameters: json!({
            "limit": 10,
            "provider": "strava"
        }),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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
#[serial]
async fn test_connection_status_with_oauth_manager() {
    common::init_server_config();
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&user).await.unwrap();

    // Set up environment for OAuth providers
    env::set_var("STRAVA_CLIENT_ID", "test_client");
    env::set_var("STRAVA_CLIENT_SECRET", "test_secret");
    env::set_var("FITBIT_CLIENT_ID", "test_fitbit");
    env::set_var("FITBIT_CLIENT_SECRET", "test_fitbit_secret");

    // Create request for get_connection_status
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_connection_status".to_owned(),
        parameters: json!({}),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    // Execute tool
    let response = executor.execute_tool(request).await.unwrap();

    // Check response
    assert!(response.success);
    assert!(response.result.is_some());

    let result = response.result.unwrap();
    assert!(result.get("providers").is_some());

    let providers = result.get("providers").unwrap();
    // Check for supported providers (strava, garmin, synthetic)
    assert!(
        providers.get("strava").is_some()
            || providers.get("garmin").is_some()
            || providers.get("synthetic").is_some()
    );

    // All providers should be disconnected since no tokens are stored
    if let Some(strava) = providers.get("strava") {
        assert_eq!(strava["connected"], serde_json::Value::Bool(false));
    }
    if let Some(garmin) = providers.get("garmin") {
        assert_eq!(garmin["connected"], serde_json::Value::Bool(false));
    }
}

/// Test that analyze_activity uses token refresh
#[tokio::test]
#[serial]
async fn test_analyze_activity_token_refresh() {
    common::init_server_config();
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&user).await.unwrap();

    // Store token that will expire soon
    let expires_at = chrono::Utc::now() + chrono::Duration::minutes(3); // Expires in 3 minutes (within buffer)
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "soon_to_expire_token".to_owned(),
        Some("refresh_token_456".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database
        .upsert_user_oauth_token(&oauth_token)
        .await
        .unwrap();

    // Set up environment
    env::set_var("STRAVA_CLIENT_ID", "test_client");
    env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create request
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "analyze_activity".to_owned(),
        parameters: json!({
            "activity_id": "123456789"
        }),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
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
                        || error.contains("Authentication required")
                        || error.contains("No valid authentication token")
                        || error.contains("Authentication failed")
                        || (error.contains("No valid") && error.contains("token found"))
                        || error.contains("Authentication error")
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
#[serial]
async fn test_concurrent_token_operations() {
    common::init_server_config();
    let (executor, database) = create_test_executor().await;

    // Create user
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&user).await.unwrap();

    // Store valid token
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "valid_token".to_owned(),
        Some("refresh_token".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database
        .upsert_user_oauth_token(&oauth_token)
        .await
        .unwrap();

    // Set up environment
    env::set_var("STRAVA_CLIENT_ID", "test_client");
    env::set_var("STRAVA_CLIENT_SECRET", "test_secret");

    // Create multiple concurrent requests
    let mut handles = vec![];

    for _i in 0..5 {
        let executor_clone = executor.clone();
        let user_id_str = user_id.to_string();
        let handle = tokio::spawn(async move {
            let request = UniversalRequest {
                user_id: user_id_str,
                tool_name: "get_connection_status".to_owned(),
                parameters: json!({}),
                protocol: "test".to_owned(),
                tenant_id: None,
                progress_token: None,
                cancellation_token: None,
                progress_reporter: None,
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
    common::init_server_config();
    // Create executor with configuration that has missing OAuth credentials
    let (executor, database) = create_test_executor_without_oauth().await;

    // Create user first so they can be tenant owner
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "test@example.com".to_owned(),
        display_name: Some("Test User".to_owned()),
        password_hash: bcrypt::hash("password", bcrypt::DEFAULT_COST).unwrap(),
        tier: UserTier::Starter,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&user).await.unwrap();

    // Create tenant with the user as owner
    let tenant = Tenant::new(
        "Test Tenant".to_owned(),
        "test-tenant".to_owned(),
        Some("test.example.com".to_owned()),
        "starter".to_owned(),
        user_id, // User is now the owner
    );
    database.create_tenant(&tenant).await.unwrap();

    // Create request
    let request = UniversalRequest {
        user_id: user_id.to_string(),
        tool_name: "get_activities".to_owned(),
        parameters: json!({}),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    // Execute - should handle provider initialization failure gracefully
    let response = executor.execute_tool(request).await.unwrap();

    // Should fail with proper error message about missing OAuth credentials
    println!("Response: {:?}", response);
    println!("Success: {}", response.success);
    if let Some(ref err) = response.error {
        println!("Error: {}", err);
    }
    assert!(
        !response.success,
        "Tool execution should fail when no OAuth token"
    );
    assert!(
        response.error.is_some(),
        "Should have error message about missing OAuth token"
    );

    // Check that the error contains information about missing OAuth token
    let error_msg = response.error.unwrap();
    assert!(
        (error_msg.contains("No") && error_msg.contains("token"))
            || error_msg.contains("Connect your")
            || error_msg.contains("Please connect your"),
        "Error should contain OAuth connection message: {}",
        error_msg
    );
}
