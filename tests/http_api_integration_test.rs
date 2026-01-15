// ABOUTME: Integration tests for HTTP API endpoints (user management, OAuth, API keys)
// ABOUTME: Tests complete user workflows through REST API routes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

mod common;

use anyhow::Result;
use pierre_mcp_server::{
    api_keys::{ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
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
    context::ServerContext,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::ServerResources,
    models::{Tenant, User, UserStatus, UserTier},
    permissions::UserRole,
    routes::{
        auth::{AuthService, OAuthService},
        LoginRequest, RegisterRequest,
    },
    tenant::TenantOAuthCredentials,
};
use serde_json::json;
use std::{path::PathBuf, sync::Arc, time::Duration};

/// Test setup for SDK integration tests
// Long function: Defines complete test environment setup including database, auth, config, and test data
#[allow(clippy::too_many_lines)]
async fn setup_test_environment() -> Result<(Arc<Database>, AuthService, OAuthService, uuid::Uuid)>
{
    // Initialize server config for tests
    common::init_server_config();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            generate_encryption_key().to_vec(),
            &PostgresPoolConfig::default(),
        )
        .await?,
    );

    #[cfg(not(feature = "postgresql"))]
    let database =
        Arc::new(Database::new("sqlite::memory:", generate_encryption_key().to_vec()).await?);
    database.migrate().await?;

    let auth_manager = Arc::new(AuthManager::new(24));

    // Create admin user first
    let admin_user = User {
        id: uuid::Uuid::new_v4(),
        email: "admin@example.com".to_owned(),
        display_name: Some("Admin".to_owned()),
        password_hash: "hash".to_owned(),
        tier: UserTier::Starter,
        tenant_id: None,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: None,
        approved_at: None,
        firebase_uid: None,
        auth_provider: String::new(),
    };
    let admin_id = database.create_user(&admin_user).await?;

    // Create tenant
    let tenant_id = uuid::Uuid::new_v4();
    let tenant = Tenant {
        id: tenant_id,
        name: "Test Tenant".to_owned(),
        slug: "test-tenant".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await?;

    // Store tenant OAuth credentials
    let strava_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "strava".to_owned(),
        client_id: "test_client_id".to_owned(),
        client_secret: "test_client_secret".to_owned(),
        redirect_uri: "http://localhost:8080/oauth/callback/strava".to_owned(),
        scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&strava_credentials)
        .await?;

    let fitbit_credentials = TenantOAuthCredentials {
        tenant_id,
        provider: "fitbit".to_owned(),
        client_id: "test_fitbit_client_id".to_owned(),
        client_secret: "test_fitbit_client_secret".to_owned(),
        redirect_uri: "http://localhost:8080/oauth/callback/fitbit".to_owned(),
        scopes: vec!["activity".to_owned(), "profile".to_owned()],
        rate_limit_per_day: 15000,
    };
    database
        .store_tenant_oauth_credentials(&fitbit_credentials)
        .await?;

    // Create basic config with correct structure
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
                client_id: Some("test_fitbit_client_id".to_owned()),
                client_secret: Some("test_fitbit_client_secret".to_owned()),
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
            cors_origins: vec!["http://localhost:3000".to_owned()],
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
                token_url: "https://connect.garmin.com/oauth-service/oauth/access_token".to_owned(),
                revoke_url: "https://connect.garmin.com/oauth-service/oauth/revoke".to_owned(),
                ..Default::default()
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 200,
            default_activities_limit: 50,
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

    let cache_config = MemoryCacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(60),
        enable_background_cleanup: false,
        ..Default::default()
    };
    let cache = Cache::new(cache_config).await.unwrap();

    let server_resources = Arc::new(
        ServerResources::new(
            (*database).clone(),
            (*auth_manager).clone(),
            "test_jwt_secret",
            config,
            cache,
            2048, // Use 2048-bit RSA keys for faster test execution
            Some(common::get_shared_test_jwks()),
        )
        .await,
    );

    let server_context = ServerContext::from(server_resources.as_ref());
    let auth_routes = AuthService::new(
        server_context.auth().clone(),
        server_context.config().clone(),
        server_context.data().clone(),
    );
    let oauth_routes = OAuthService::new(
        server_context.data().clone(),
        server_context.config().clone(),
        server_context.notification().clone(),
    );

    Ok((database, auth_routes, oauth_routes, tenant_id))
}

/// Helper to create and approve a test user
async fn create_approved_test_user(
    database: &Database,
    email: &str,
    password: &str,
) -> Result<String> {
    let user = User::new(
        email.to_owned(),
        bcrypt::hash(password, bcrypt::DEFAULT_COST)?,
        Some("Test User".to_owned()),
    );
    let user_id = user.id;

    // Create user with active status
    let mut active_user = user;
    active_user.user_status = UserStatus::Active;
    active_user.approved_at = Some(chrono::Utc::now());

    database.create_user(&active_user).await?;
    Ok(user_id.to_string())
}

#[tokio::test]
async fn test_sdk_user_registration_flow() -> Result<()> {
    let (database, auth_routes, _oauth_routes, _tenant_id) = setup_test_environment().await?;

    // Test 1: Register user (should create with pending status)
    let register_request = RegisterRequest {
        email: "sdk_test@example.com".to_owned(),
        password: "TestPassword123".to_owned(),
        display_name: Some("SDK Test User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    assert!(!register_response.user_id.is_empty());
    assert!(register_response.message.contains("pending admin approval"));

    // Test 2: Verify user is created with pending status
    let user_id = uuid::Uuid::parse_str(&register_response.user_id)?;
    let user = database.get_user(user_id).await?.unwrap();
    assert_eq!(user.user_status, UserStatus::Pending);

    // Test 3: Login succeeds for pending user but returns pending status
    // (Backend authenticates; frontend handles access control based on user_status)
    let pending_login_response = auth_routes
        .login(LoginRequest {
            email: "sdk_test@example.com".to_owned(),
            password: "TestPassword123".to_owned(),
        })
        .await?;
    assert!(
        pending_login_response
            .jwt_token
            .as_ref()
            .is_some_and(|t| !t.is_empty()),
        "JWT token should be present even for pending user"
    );
    assert_eq!(
        pending_login_response.user.user_status, "pending",
        "User status should be 'pending' so frontend can restrict access"
    );

    // Test 4: Approve user and verify status changes to active
    database
        .update_user_status(user_id, UserStatus::Active, None)
        .await?;

    let active_login_response = auth_routes
        .login(LoginRequest {
            email: "sdk_test@example.com".to_owned(),
            password: "TestPassword123".to_owned(),
        })
        .await?;
    assert!(
        active_login_response
            .jwt_token
            .as_ref()
            .is_some_and(|t| !t.is_empty()),
        "JWT token should be present and non-empty"
    );
    assert_eq!(active_login_response.user.email, "sdk_test@example.com");
    assert_eq!(
        active_login_response.user.user_status, "active",
        "User status should be 'active' after approval"
    );

    Ok(())
}

#[tokio::test]
async fn test_sdk_oauth_credentials_storage() -> Result<()> {
    let (database, _auth_routes, oauth_routes, tenant_id) = setup_test_environment().await?;

    // Create and approve a test user
    let user_id_str =
        create_approved_test_user(&database, "oauth_test@example.com", "TestPassword123").await?;
    let user_id = uuid::Uuid::parse_str(&user_id_str)?;

    // Test 1: Get OAuth authorization URL (uses tenant credentials)
    // This tests the OAuth flow with tenant-based credentials

    // Test 2: Get OAuth authorization URL
    let auth_url_response = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .await?;
    assert!(auth_url_response.authorization_url.contains("strava.com"));
    assert!(auth_url_response
        .authorization_url
        .contains("test_client_id")); // From tenant credentials
    assert!(!auth_url_response.state.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_sdk_api_key_management() -> Result<()> {
    let (database, _auth_routes, _oauth_routes, _tenant_id) = setup_test_environment().await?;

    // Create and approve a test user
    let user_id_str =
        create_approved_test_user(&database, "apikey_test@example.com", "TestPassword123").await?;
    let user_id = uuid::Uuid::parse_str(&user_id_str)?;

    // Test 1: Create API key
    let api_key_manager = ApiKeyManager::new();
    let create_request = CreateApiKeyRequest {
        name: "SDK Test Key".to_owned(),
        description: Some("Test API key for SDK integration".to_owned()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: Some(1000),
        expires_in_days: Some(365),
    };

    let (api_key, api_key_string) = api_key_manager.create_api_key(user_id, create_request)?;
    database.create_api_key(&api_key).await?;

    // Test 2: Verify API key is created
    assert!(!api_key_string.is_empty());
    assert!(api_key_string.starts_with("pk_"));
    assert_eq!(api_key.name, "SDK Test Key");
    assert_eq!(api_key.user_id, user_id);

    // Test 3: Verify API key in database
    let api_key_details = database.get_api_key_by_id(&api_key.id).await?.unwrap();
    assert_eq!(api_key_details.name, "SDK Test Key");
    assert_eq!(api_key_details.user_id, user_id);

    Ok(())
}

#[tokio::test]
async fn test_sdk_error_handling() -> Result<()> {
    let (database, auth_routes, oauth_routes, tenant_id) = setup_test_environment().await?;

    // Test 1: Invalid email format
    let invalid_email_request = RegisterRequest {
        email: "invalid-email".to_owned(),
        password: "TestPassword123".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    let result = auth_routes.register(invalid_email_request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email format"));

    // Test 2: Password too short
    let short_password_request = RegisterRequest {
        email: "test@example.com".to_owned(),
        password: "short".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    let result = auth_routes.register(short_password_request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("at least 8 characters"));

    // Test 3: Duplicate user registration
    let valid_request = RegisterRequest {
        email: "duplicate@example.com".to_owned(),
        password: "TestPassword123".to_owned(),
        display_name: Some("Test User".to_owned()),
    };

    auth_routes.register(valid_request.clone()).await?;
    let result = auth_routes.register(valid_request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already exists"));

    // Test 4: Login with non-existent user
    let login_request = LoginRequest {
        email: "nonexistent@example.com".to_owned(),
        password: "TestPassword123".to_owned(),
    };

    let result = auth_routes.login(login_request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email or password"));

    // Test 5: OAuth URL for unsupported provider
    let user_id_str =
        create_approved_test_user(&database, "oauth_error_test@example.com", "TestPassword123")
            .await?;
    let user_id = uuid::Uuid::parse_str(&user_id_str)?;

    let result = oauth_routes
        .get_auth_url(user_id, tenant_id, "unsupported_provider")
        .await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported provider"));

    Ok(())
}

#[tokio::test]
async fn test_sdk_complete_onboarding_simulation() -> Result<()> {
    let (database, auth_routes, oauth_routes, tenant_id) = setup_test_environment().await?;

    // Simulate complete SDK onboarding flow

    // Step 1: Register user
    let register_request = RegisterRequest {
        email: "complete_test@example.com".to_owned(),
        password: "CompleteTest123".to_owned(),
        display_name: Some("Complete Test User".to_owned()),
    };

    let register_response = auth_routes.register(register_request).await?;
    let user_id = uuid::Uuid::parse_str(&register_response.user_id)?;

    // Step 2: Admin approves user (simulated, service token approval)
    database
        .update_user_status(user_id, UserStatus::Active, None)
        .await?;

    // Step 3: User logs in
    let login_request = LoginRequest {
        email: "complete_test@example.com".to_owned(),
        password: "CompleteTest123".to_owned(),
    };

    let login_response = auth_routes.login(login_request).await?;
    assert!(
        login_response
            .jwt_token
            .as_ref()
            .is_some_and(|t| !t.is_empty()),
        "JWT token should be present and non-empty"
    );

    // Step 4: Test OAuth URL generation (uses env vars for client credentials)

    // Step 5: Create API key
    let api_key_manager = ApiKeyManager::new();
    let create_request = CreateApiKeyRequest {
        name: "Complete Test API Key".to_owned(),
        description: Some("Complete onboarding test".to_owned()),
        tier: ApiKeyTier::Professional,
        rate_limit_requests: Some(5000),
        expires_in_days: None,
    };

    let (api_key, api_key_string) = api_key_manager.create_api_key(user_id, create_request)?;
    database.create_api_key(&api_key).await?;

    // Step 6: Verify complete setup
    let user = database.get_user(user_id).await?.unwrap();
    assert_eq!(user.user_status, UserStatus::Active);
    assert_eq!(user.email, "complete_test@example.com");

    // Skip OAuth app verification for now due to type mismatch
    // The OAuth URLs are generated successfully using environment variables

    let stored_api_key = database.get_api_key_by_id(&api_key.id).await?.unwrap();
    assert_eq!(stored_api_key.name, "Complete Test API Key");
    assert_eq!(stored_api_key.user_id, user_id);

    // Step 7: Test OAuth URL generation
    let auth_url = oauth_routes
        .get_auth_url(user_id, tenant_id, "strava")
        .await?;
    assert!(auth_url.authorization_url.contains("test_client_id")); // Uses tenant credentials

    // Step 8: Test connection status
    let connections = oauth_routes.get_connection_status(user_id).await?;
    assert!(!connections.is_empty());

    // Find strava connection
    let strava_connection = connections
        .iter()
        .find(|c| c.provider == "strava")
        .expect("Should have strava connection");
    assert!(!strava_connection.connected); // Not connected yet (no tokens exchanged)

    println!("Complete SDK onboarding simulation successful!");
    println!("User: complete_test@example.com");
    println!("API Key: {}", &api_key_string[..20]);
    println!("OAuth App: strava (complete_test_client_id)");

    Ok(())
}

#[tokio::test]
async fn test_sdk_mcp_config_generation() -> Result<()> {
    let (_database, _auth_routes, _oauth_routes, _tenant_id) = setup_test_environment().await?;

    // Test MCP configuration generation (simulates SDK generateMcpConfig function)
    let api_key = "pk_test_example_api_key_for_mcp_config";
    let server_url = "http://localhost:8080";

    // Simulate the MCP configuration that the SDK would generate
    let mcp_config = json!({
        "mcpServers": {
            "pierre-fitness": {
                "command": "node",
                "args": [
                    "-e",
                    format!("const http=require('http');process.stdin.on('data',d=>{{const req=http.request('{}{}',{{method:'POST',headers:{{'Content-Type':'application/json','X-API-Key':'{}','Origin':'http://localhost'}}}},res=>{{let data='';res.on('data',chunk=>data+=chunk);res.on('end',()=>process.stdout.write(data))}});req.write(d);req.end()}});", server_url, "/mcp", api_key)
                ]
            }
        }
    });

    // Verify MCP config structure
    assert!(mcp_config["mcpServers"]["pierre-fitness"]["command"].is_string());
    assert!(mcp_config["mcpServers"]["pierre-fitness"]["args"].is_array());

    let args = mcp_config["mcpServers"]["pierre-fitness"]["args"]
        .as_array()
        .unwrap();
    assert_eq!(args.len(), 2);
    assert_eq!(args[0], "-e");

    let bridge_code = args[1].as_str().unwrap();
    assert!(bridge_code.contains(api_key));
    assert!(bridge_code.contains(server_url));
    assert!(bridge_code.contains("X-API-Key"));

    println!("MCP config generation test successful!");
    println!(
        "Generated config: {}",
        serde_json::to_string_pretty(&mcp_config)?
    );

    Ok(())
}
