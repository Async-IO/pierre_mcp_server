// ABOUTME: Integration tests for multi-tenant architecture and functionality
// ABOUTME: Tests tenant isolation, data separation, and multi-tenant workflows
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
    clippy::unchecked_time_subtraction,
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

mod common;

use anyhow::Result;
use pierre_mcp_server::{
    auth::AuthManager,
    cache::{factory::Cache, CacheConfig as MemoryCacheConfig},
    config::environment::{
        AppBehaviorConfig, AuthConfig, BackupConfig, CacheConfig, CorsConfig, DatabaseConfig,
        DatabaseUrl, Environment, ExternalServicesConfig, FirebaseConfig, FitbitApiConfig,
        GarminApiConfig, GeocodingServiceConfig, GoalManagementConfig, HttpClientConfig, LogLevel,
        LoggingConfig, McpConfig, MonitoringConfig, OAuth2ServerConfig, OAuthConfig,
        OAuthProviderConfig, PostgresPoolConfig, ProtocolConfig, RateLimitConfig,
        RouteTimeoutConfig, SecurityConfig, SecurityHeadersConfig, ServerConfig,
        SleepToolParamsConfig, SqlxConfig, SseConfig, StravaApiConfig, TlsConfig,
        TokioRuntimeConfig, TrainingZonesConfig, WeatherServiceConfig,
    },
    constants::oauth_providers,
    context::ServerContext,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::{ServerResources, ServerResourcesOptions},
    models::{TenantId, User, UserOAuthToken, UserStatus, UserTier},
    permissions::UserRole,
    routes::{auth::AuthService, LoginRequest, RegisterRequest},
};
use std::{sync::Arc, time::Duration};
use tempfile::TempDir;
use uuid::Uuid;

/// Test full multi-tenant authentication flow
#[tokio::test]
async fn test_multitenant_auth_flow() -> Result<()> {
    common::init_server_config();
    // Setup
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;
    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&database_url, encryption_key).await?;

    let auth_manager = AuthManager::new(24);

    // Create minimal config for ServerResources
    let config = Arc::new(ServerConfig {
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
                directory: temp_dir.path().to_path_buf(),
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
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
                ..Default::default()
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
                ..Default::default()
            },
            garmin_api: GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token"
                    .to_string(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ..Default::default()
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            auto_approve_users_from_env: false,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: SseConfig::default(),
        oauth2_server: OAuth2ServerConfig::default(),
        route_timeouts: RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_tool_params: SleepToolParamsConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });

    // Create test cache with background cleanup disabled
    let cache_config = MemoryCacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(60),
        enable_background_cleanup: false,
        ..Default::default()
    };
    let cache = Cache::new(cache_config)
        .await
        .expect("Failed to create test cache");

    let server_resources = Arc::new(
        ServerResources::new(
            database.clone(),
            auth_manager.clone(),
            "test_jwt_secret",
            config,
            cache,
            ServerResourcesOptions {
                rsa_key_size_bits: Some(2048),
                jwks_manager: Some(common::get_shared_test_jwks()),
                llm_provider: None,
            },
        )
        .await,
    );

    let server_context = ServerContext::from(server_resources.as_ref());
    let auth_routes = AuthService::new(
        server_context.auth().clone(),
        server_context.config().clone(),
        server_context.data().clone(),
    );

    // Test user registration
    let register_request = RegisterRequest {
        email: "test@multitenant.com".to_owned(),
        password: "securepassword123".to_owned(),
        display_name: Some("Multi-Tenant User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    assert!(!register_response.user_id.is_empty());
    assert_eq!(
        register_response.message,
        "User registered successfully. Your account is pending admin approval."
    );

    // Parse user ID
    let user_id = Uuid::parse_str(&register_response.user_id)?;

    // Verify user exists in database
    let user = database.get_user(user_id).await?.unwrap();
    assert_eq!(user.email, "test@multitenant.com");
    assert_eq!(user.display_name, Some("Multi-Tenant User".to_owned()));
    assert!(user.is_active);
    assert_eq!(user.user_status, UserStatus::Pending);

    // Create admin user and approve the user for testing
    let admin_id = uuid::Uuid::new_v4();
    let admin_user = User {
        id: admin_id,
        email: "admin@test.com".to_owned(),
        display_name: Some("Test Admin".to_owned()),
        password_hash: "$2b$10$hashedpassword".to_owned(),
        tier: UserTier::Enterprise,
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        firebase_uid: None,
        auth_provider: String::new(),
    };
    database.create_user(&admin_user).await?;

    // Approve the user with admin's UUID
    database
        .update_user_status(user_id, UserStatus::Active, Some(admin_id))
        .await?;

    // Test user login
    let login_request = LoginRequest {
        email: "test@multitenant.com".to_owned(),
        password: "securepassword123".to_owned(),
    };

    let login_response = auth_routes.login(login_request).await?;
    assert!(
        login_response
            .jwt_token
            .as_ref()
            .is_some_and(|t| !t.is_empty()),
        "JWT token should be present and non-empty"
    );
    assert_eq!(login_response.user.email, "test@multitenant.com");
    assert_eq!(login_response.user.user_id, register_response.user_id);

    // Test JWT token validation using the same JWKS manager that generated the token
    let jwt_token = login_response
        .jwt_token
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("JWT token not found in response"))?;
    let claims = auth_manager.validate_token(jwt_token, &server_resources.jwks_manager)?;
    assert_eq!(claims.email, "test@multitenant.com");
    assert_eq!(claims.sub, register_response.user_id);

    // Test duplicate registration fails
    let duplicate_request = RegisterRequest {
        email: "test@multitenant.com".to_owned(),
        password: "differentpassword".to_owned(),
        display_name: None,
    };

    let duplicate_result = auth_routes.register(duplicate_request).await;
    assert!(duplicate_result.is_err());
    assert!(duplicate_result
        .unwrap_err()
        .to_string()
        .contains("already exists"));

    // Test login with wrong password fails
    let wrong_password_request = LoginRequest {
        email: "test@multitenant.com".to_owned(),
        password: "wrongpassword".to_owned(),
    };

    let wrong_password_result = auth_routes.login(wrong_password_request).await;
    assert!(wrong_password_result.is_err());
    assert!(wrong_password_result
        .unwrap_err()
        .to_string()
        .contains("Invalid credentials provided"));

    Ok(())
}

/// Test database encryption functionality
#[tokio::test]
async fn test_database_encryption() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("encryption_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;
    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&database_url, encryption_key).await?;

    // Create user
    let user = User::new(
        "encryption@test.com".to_owned(),
        "bcrypt_hashed_password".to_owned(),
        Some("Encryption Test".to_owned()),
    );
    let user_id = database.create_user(&user).await?;

    // Store encrypted Strava token
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);
    let oauth_token = UserOAuthToken::new(
        user_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "secret_access_token_123".to_owned(),
        Some("secret_refresh_token_456".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database.upsert_user_oauth_token(&oauth_token).await?;

    // Retrieve and decrypt token
    let decrypted_token = database
        .get_user_oauth_token(
            user_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();
    assert_eq!(decrypted_token.access_token, "secret_access_token_123");
    assert_eq!(
        decrypted_token.refresh_token,
        Some("secret_refresh_token_456".to_owned())
    );
    assert_eq!(
        decrypted_token.scope,
        Some("read,activity:read_all".to_owned())
    );

    Ok(())
}

/// Test JWT authentication edge cases
#[tokio::test]
async fn test_jwt_edge_cases() -> Result<()> {
    let auth_manager = AuthManager::new(1); // 1 hour expiry

    let user = User::new(
        "jwt@test.com".to_owned(),
        "hashed_password".to_owned(),
        Some("JWT Test".to_owned()),
    );

    // Test token generation and validation
    let jwks_manager = common::get_shared_test_jwks();
    let token = auth_manager.generate_token(&user, &jwks_manager)?;
    let claims = auth_manager.validate_token(&token, &jwks_manager)?;
    assert_eq!(claims.email, "jwt@test.com");
    assert_eq!(claims.sub, user.id.to_string());

    // Test token refresh
    let refreshed_token = auth_manager.refresh_token(&token, &user, &jwks_manager)?;
    let refreshed_claims = auth_manager.validate_token(&refreshed_token, &jwks_manager)?;
    assert_eq!(refreshed_claims.email, claims.email);
    assert_eq!(refreshed_claims.sub, claims.sub);

    // Test invalid token
    let invalid_token = "invalid.token.here";
    let invalid_result = auth_manager.validate_token(invalid_token, &jwks_manager);
    assert!(invalid_result.is_err());

    // Test malformed token
    let malformed_token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.malformed.signature";
    let malformed_result = auth_manager.validate_token(malformed_token, &jwks_manager);
    assert!(malformed_result.is_err());

    Ok(())
}

/// Test user isolation in multi-tenant database
#[tokio::test]
async fn test_user_isolation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("isolation_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;
    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&database_url, encryption_key).await?;

    // Create two users
    let user1 = User::new(
        "user1@isolation.test".to_owned(),
        "password1".to_owned(),
        Some("User One".to_owned()),
    );
    let user1_id = database.create_user(&user1).await?;

    let user2 = User::new(
        "user2@isolation.test".to_owned(),
        "password2".to_owned(),
        Some("User Two".to_owned()),
    );
    let user2_id = database.create_user(&user2).await?;

    // Store tokens for each user
    let expires_at = chrono::Utc::now() + chrono::Duration::hours(6);

    let oauth_token1 = UserOAuthToken::new(
        user1_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "user1_access_token".to_owned(),
        Some("user1_refresh_token".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database.upsert_user_oauth_token(&oauth_token1).await?;

    let oauth_token2 = UserOAuthToken::new(
        user2_id,
        "00000000-0000-0000-0000-000000000000".to_owned(),
        oauth_providers::STRAVA.to_owned(),
        "user2_access_token".to_owned(),
        Some("user2_refresh_token".to_owned()),
        Some(expires_at),
        Some("read,activity:read_all".to_owned()),
    );
    database.upsert_user_oauth_token(&oauth_token2).await?;

    // Verify user isolation - each user can only access their own tokens
    let user1_token = database
        .get_user_oauth_token(
            user1_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();
    assert_eq!(user1_token.access_token, "user1_access_token");

    let user2_token = database
        .get_user_oauth_token(
            user2_id,
            TenantId::from_uuid(Uuid::nil()),
            oauth_providers::STRAVA,
        )
        .await?
        .unwrap();
    assert_eq!(user2_token.access_token, "user2_access_token");

    // Verify users cannot access each other's data
    assert_ne!(user1_token.access_token, user2_token.access_token);
    assert_ne!(user1_token.refresh_token, user2_token.refresh_token);

    Ok(())
}

/// Test input validation
#[tokio::test]
async fn test_input_validation() -> Result<()> {
    common::init_server_config();
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("validation_test.db");
    let database_url = format!("sqlite:{}", db_path.display());
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Database::new(
        &database_url,
        encryption_key,
        &PostgresPoolConfig::default(),
    )
    .await?;
    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(&database_url, encryption_key).await?;

    let auth_manager = AuthManager::new(24);

    // Create minimal config for ServerResources
    let config = Arc::new(ServerConfig {
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
                directory: temp_dir.path().to_path_buf(),
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
                environment: Environment::Testing,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_owned(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: false,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_owned(),
                auth_url: "https://www.strava.com/oauth/authorize".to_owned(),
                token_url: "https://www.strava.com/oauth/token".to_owned(),
                deauthorize_url: "https://www.strava.com/oauth/deauthorize".to_owned(),
                ..Default::default()
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_owned(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_owned(),
                token_url: "https://api.fitbit.com/oauth2/token".to_owned(),
                revoke_url: "https://api.fitbit.com/oauth2/revoke".to_owned(),
                ..Default::default()
            },
            garmin_api: GarminApiConfig {
                base_url: "https://apis.garmin.com".to_owned(),
                auth_url: "https://connect.garmin.com/oauthConfirm".to_owned(),
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token"
                    .to_string(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ..Default::default()
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            auto_approve_users_from_env: false,
            protocol: ProtocolConfig {
                mcp_version: "2025-06-18".to_owned(),
                server_name: "pierre-mcp-server-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: SseConfig::default(),
        oauth2_server: OAuth2ServerConfig::default(),
        route_timeouts: RouteTimeoutConfig::default(),
        host: "localhost".to_owned(),
        base_url: "http://localhost:8081".to_owned(),
        mcp: McpConfig {
            protocol_version: "2025-06-18".to_owned(),
            server_name: "pierre-mcp-server-test".to_owned(),
            session_cache_size: 1000,
            ..Default::default()
        },
        cors: CorsConfig {
            allowed_origins: "*".to_owned(),
            allow_localhost_dev: true,
        },
        cache: CacheConfig {
            redis_url: None,
            max_entries: 10000,
            cleanup_interval_secs: 300,
            ..Default::default()
        },
        usda_api_key: None,
        rate_limiting: RateLimitConfig::default(),
        sleep_tool_params: SleepToolParamsConfig::default(),
        goal_management: GoalManagementConfig::default(),
        training_zones: TrainingZonesConfig::default(),
        firebase: FirebaseConfig::default(),
        tokio_runtime: TokioRuntimeConfig::default(),
        sqlx: SqlxConfig::default(),
        monitoring: MonitoringConfig::default(),
        frontend_url: None,
    });

    // Create test cache with background cleanup disabled
    let cache_config = MemoryCacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(60),
        enable_background_cleanup: false,
        ..Default::default()
    };
    let cache = Cache::new(cache_config)
        .await
        .expect("Failed to create test cache");

    let server_resources = Arc::new(
        ServerResources::new(
            database.clone(),
            auth_manager.clone(),
            "test_jwt_secret",
            config,
            cache,
            ServerResourcesOptions {
                rsa_key_size_bits: Some(2048),
                jwks_manager: Some(common::get_shared_test_jwks()),
                llm_provider: None,
            },
        )
        .await,
    );

    let server_context = ServerContext::from(server_resources.as_ref());
    let auth_routes = AuthService::new(
        server_context.auth().clone(),
        server_context.config().clone(),
        server_context.data().clone(),
    );

    // Test invalid email formats
    let invalid_emails = vec!["not-an-email", "@domain.com", "user@", "user", "a@b", ""];

    for invalid_email in invalid_emails {
        let request = RegisterRequest {
            email: invalid_email.to_owned(),
            password: "validpassword123".to_owned(),
            display_name: None,
        };

        let result = auth_routes.register(request).await;
        assert!(
            result.is_err(),
            "Should reject invalid email: {}",
            invalid_email
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid email format"));
    }

    // Test short passwords
    let short_passwords = vec!["1234567", "short", "", "a"];

    for short_password in short_passwords {
        let request = RegisterRequest {
            email: "test@valid.com".to_owned(),
            password: short_password.to_string(),
            display_name: None,
        };

        let result = auth_routes.register(request).await;
        assert!(
            result.is_err(),
            "Should reject short password: {}",
            short_password
        );
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("at least 8 characters"));
    }

    // Test valid inputs
    let valid_request = RegisterRequest {
        email: "valid@email.com".to_owned(),
        password: "validpassword123".to_owned(),
        display_name: Some("Valid User".to_owned()),
    };

    let result = auth_routes.register(valid_request).await;
    assert!(result.is_ok(), "Should accept valid inputs");

    Ok(())
}
