// ABOUTME: End-to-end integration test for complete multi-tenant onboarding workflow
// ABOUTME: Tests tenant creation, OAuth app registration, credential management, and tool execution
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # End-to-End Tenant Onboarding Test
//!
//! This test demonstrates the complete multi-tenant onboarding workflow:
//! 1. Create a new tenant with admin user
//! 2. Register OAuth applications for fitness providers
//! 3. Configure tenant-specific OAuth credentials
//! 4. Execute tools using tenant-isolated credentials
//! 5. Verify proper isolation between tenants

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::{
    config::environment::ServerConfig,
    database_plugins::{factory::Database, DatabaseProvider},
    intelligence::ActivityIntelligence,
    models::{OAuthApp, Tenant, User, UserTier},
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
    tenant::{
        oauth_manager::TenantOAuthManager, StoreCredentialsRequest, TenantContext,
        TenantOAuthClient, TenantRole,
    },
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

mod common;

/// Test configuration for end-to-end tenant onboarding
#[allow(clippy::too_many_lines)] // Long function: Defines complete end-to-end tenant onboarding workflow
#[tokio::test]
async fn test_complete_tenant_onboarding_workflow() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Initialize HTTP clients (only once across all tests)
    common::init_test_http_clients();
    common::init_server_config();

    // Step 1: Create test database and base infrastructure
    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            vec![0; 32],
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await
        .expect("Failed to create test database"),
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(
        Database::new("sqlite::memory:", vec![0; 32])
            .await
            .expect("Failed to create test database"),
    );

    // Step 2: Create admin users first (required for tenant foreign key constraints)
    let acme_admin_id = Uuid::new_v4();
    let beta_admin_id = Uuid::new_v4();
    let acme_tenant_id = Uuid::new_v4();
    let beta_tenant_id = Uuid::new_v4();

    let acme_admin = User {
        id: acme_admin_id,
        email: "admin@acmefitness.com".to_owned(),
        display_name: Some("Acme Admin".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Enterprise,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: true,
        role: pierre_mcp_server::permissions::UserRole::Admin,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
    };

    let beta_admin = User {
        id: beta_admin_id,
        email: "admin@betahealth.com".to_owned(),
        display_name: Some("Beta Admin".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: true,
        role: pierre_mcp_server::permissions::UserRole::Admin,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
    };

    database.create_user(&acme_admin).await?;
    database.create_user(&beta_admin).await?;

    // Step 3: Create first tenant ("Acme Fitness Co.")
    let acme_tenant = Tenant {
        id: acme_tenant_id,
        name: "Acme Fitness Co.".to_owned(),
        slug: "acme-fitness".to_owned(),
        domain: None,
        plan: "enterprise".to_owned(),
        owner_user_id: acme_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&acme_tenant).await?;

    // Step 4: Create second tenant ("Beta Health Inc.") for isolation testing
    let beta_tenant = Tenant {
        id: beta_tenant_id,
        name: "Beta Health Inc.".to_owned(),
        slug: "beta-health".to_owned(),
        domain: None,
        plan: "professional".to_owned(),
        owner_user_id: beta_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&beta_tenant).await?;

    // Step 5: Register OAuth applications for each tenant
    let acme_strava_app = OAuthApp {
        id: Uuid::new_v4(),
        client_id: "acme_strava_client_123".to_owned(),
        client_secret: "encrypted_acme_secret".to_owned(),
        name: "Acme Fitness Strava Integration".to_owned(),
        description: Some("Strava integration for Acme Fitness".to_owned()),
        redirect_uris: vec!["https://acme-fitness.com/oauth/strava/callback".to_owned()],
        scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
        app_type: "confidential".to_owned(),
        owner_user_id: acme_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let beta_strava_app = OAuthApp {
        id: Uuid::new_v4(),
        client_id: "beta_strava_client_456".to_owned(),
        client_secret: "encrypted_beta_secret".to_owned(),
        name: "Beta Health Strava Integration".to_owned(),
        description: Some("Strava integration for Beta Health".to_owned()),
        redirect_uris: vec!["https://beta-health.com/oauth/strava/callback".to_owned()],
        scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
        app_type: "confidential".to_owned(),
        owner_user_id: beta_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_oauth_app(&acme_strava_app).await?;
    database.create_oauth_app(&beta_strava_app).await?;

    // Step 6: Set up tenant OAuth client and configure credentials
    let oauth_config = Arc::new(pierre_mcp_server::config::environment::OAuthConfig {
        strava: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        garmin: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        whoop: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        terra: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
    });
    let tenant_oauth_client = Arc::new(TenantOAuthClient::new(TenantOAuthManager::new(
        oauth_config,
    )));

    // Configure Acme's Strava credentials
    let acme_credentials = StoreCredentialsRequest {
        client_id: "acme_strava_client_123".to_owned(),
        client_secret: "acme_secret_key".to_owned(),
        redirect_uri: "https://acme-fitness.com/oauth/strava/callback".to_owned(),
        scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
        configured_by: acme_admin_id,
    };

    tenant_oauth_client
        .store_credentials(acme_tenant_id, "strava", acme_credentials)
        .await?;

    // Configure Beta's Strava credentials
    let beta_credentials = StoreCredentialsRequest {
        client_id: "beta_strava_client_456".to_owned(),
        client_secret: "beta_secret_key".to_owned(),
        redirect_uri: "https://beta-health.com/oauth/strava/callback".to_owned(),
        scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
        configured_by: beta_admin_id,
    };

    tenant_oauth_client
        .store_credentials(beta_tenant_id, "strava", beta_credentials)
        .await?;

    // Step 7: Create Universal Tool Executor with tenant OAuth support
    let _intelligence = Arc::new(ActivityIntelligence::new(
        "E2E Test Intelligence".to_owned(),
        vec![], // No initial insights
        pierre_mcp_server::intelligence::PerformanceMetrics {
            relative_effort: Some(85.0),
            zone_distribution: None,
            personal_records: Vec::new(),
            efficiency_score: Some(90.0),
            trend_indicators: pierre_mcp_server::intelligence::TrendIndicators {
                pace_trend: pierre_mcp_server::intelligence::TrendDirection::Improving,
                effort_trend: pierre_mcp_server::intelligence::TrendDirection::Stable,
                distance_trend: pierre_mcp_server::intelligence::TrendDirection::Improving,
                consistency_score: 85.0,
            },
        },
        pierre_mcp_server::intelligence::ContextualFactors {
            weather: None,
            location: None,
            time_of_day: pierre_mcp_server::intelligence::TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: None,
        },
    ));

    let config = Arc::new(create_test_server_config());

    // Create ServerResources for the test
    let auth_manager = pierre_mcp_server::auth::AuthManager::new(24);
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(pierre_mcp_server::mcp::resources::ServerResources::new(
        (*database).clone(),
        auth_manager,
        "test_secret",
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution
        Some(common::get_shared_test_jwks()),
    ));
    let executor = UniversalToolExecutor::new(server_resources);

    // Step 8: Test tenant-aware tool execution for Acme
    let acme_context = TenantContext::new(
        acme_tenant_id,
        "Acme Fitness Co.".to_owned(),
        acme_admin_id,
        TenantRole::Admin,
    );

    let acme_request = UniversalRequest {
        tool_name: "get_connection_status".to_owned(),
        parameters: json!({}),
        user_id: acme_admin_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: Some("acme".to_owned()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let acme_response = executor.execute_tool(acme_request).await?;
    assert!(acme_response.success);
    println!("Acme tenant tool execution successful");

    // Step 9: Test tenant-aware tool execution for Beta
    let beta_context = TenantContext::new(
        beta_tenant_id,
        "Beta Health Inc.".to_owned(),
        beta_admin_id,
        TenantRole::Admin,
    );

    let beta_request = UniversalRequest {
        tool_name: "get_connection_status".to_owned(),
        parameters: json!({}),
        user_id: beta_admin_id.to_string(),
        protocol: "test".to_owned(),
        tenant_id: Some("beta".to_owned()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    };

    let beta_response = executor.execute_tool(beta_request).await?;
    assert!(beta_response.success);
    println!("Beta tenant tool execution successful");

    // Step 10: Verify tenant isolation - check OAuth credentials
    let acme_oauth_creds = tenant_oauth_client
        .get_tenant_credentials(acme_tenant_id, "strava", &database)
        .await?;
    let beta_oauth_creds = tenant_oauth_client
        .get_tenant_credentials(beta_tenant_id, "strava", &database)
        .await?;

    assert!(acme_oauth_creds.is_some());
    assert!(beta_oauth_creds.is_some());

    let acme_creds = acme_oauth_creds.unwrap();
    let beta_creds = beta_oauth_creds.unwrap();

    // Verify credentials are isolated
    assert_eq!(acme_creds.client_id, "acme_strava_client_123");
    assert_eq!(beta_creds.client_id, "beta_strava_client_456");
    assert_ne!(acme_creds.client_secret, beta_creds.client_secret);

    println!("Tenant OAuth credential isolation verified");

    // Step 11: Test rate limiting isolation
    let (acme_usage, acme_limit) = tenant_oauth_client
        .check_rate_limit(acme_tenant_id, "strava")
        .await?;
    let (beta_usage, beta_limit) = tenant_oauth_client
        .check_rate_limit(beta_tenant_id, "strava")
        .await?;

    // Both should start at 0 usage
    assert_eq!(acme_usage, 0);
    assert_eq!(beta_usage, 0);
    assert_eq!(acme_limit, 15000); // Default Strava limit
    assert_eq!(beta_limit, 15000);

    println!("Tenant rate limiting isolation verified");

    // Step 12: Test OAuth authorization URL generation for each tenant
    let acme_auth_url = tenant_oauth_client
        .get_authorization_url(&acme_context, "strava", "acme_state_123", &database)
        .await?;

    let beta_auth_url = tenant_oauth_client
        .get_authorization_url(&beta_context, "strava", "beta_state_456", &database)
        .await?;

    // Verify URLs contain tenant-specific client IDs
    assert!(acme_auth_url.contains("acme_strava_client_123"));
    assert!(beta_auth_url.contains("beta_strava_client_456"));

    println!("Tenant-specific OAuth authorization URLs generated");

    // Step 13: Comprehensive workflow validation
    println!("\nEND-TO-END TENANT ONBOARDING WORKFLOW COMPLETED SUCCESSFULLY!");
    println!("   Multi-tenant database setup");
    println!("   Tenant creation and user management");
    println!("   OAuth application registration per tenant");
    println!("   Tenant-specific credential configuration");
    println!("   Isolated tool execution per tenant");
    println!("   OAuth credential isolation verification");
    println!("   Rate limiting isolation");
    println!("   Tenant-specific OAuth URLs");

    Ok(())
}

/// Helper function to create test database for tenant context tests
async fn create_tenant_test_database() -> Result<Arc<Database>> {
    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            "sqlite::memory:",
            vec![0; 32],
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await?,
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new("sqlite::memory:", vec![0; 32]).await?);

    Ok(database)
}

/// Helper function to setup multi-tenant test scenario
async fn setup_multitenant_scenario(database: &Arc<Database>) -> Result<(Uuid, Uuid, Uuid)> {
    let tenant1_id = Uuid::new_v4();
    let tenant2_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let user = User {
        id: user_id,
        email: "multi-tenant-user@example.com".to_owned(),
        display_name: Some("Multi Tenant User".to_owned()),
        password_hash: "hashed_password".to_owned(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        tenant_id: Some("test-tenant".to_owned()),
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: pierre_mcp_server::models::UserStatus::Active,
        is_admin: false,
        role: pierre_mcp_server::permissions::UserRole::User,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
    };

    database.create_user(&user).await?;

    let tenant1 = Tenant {
        id: tenant1_id,
        name: "Tenant One".to_owned(),
        slug: "tenant-one".to_owned(),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let tenant2 = Tenant {
        id: tenant2_id,
        name: "Tenant Two".to_owned(),
        slug: "tenant-two".to_owned(),
        domain: None,
        plan: "professional".to_owned(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&tenant1).await?;
    database.create_tenant(&tenant2).await?;

    Ok((tenant1_id, tenant2_id, user_id))
}

/// Test tenant switching and context validation
#[tokio::test]
async fn test_tenant_context_switching() -> Result<()> {
    common::init_test_http_clients();
    let database = create_tenant_test_database().await?;
    let (tenant1_id, tenant2_id, user_id) = setup_multitenant_scenario(&database).await?;

    // Set up different OAuth credentials for each tenant
    let oauth_config = Arc::new(pierre_mcp_server::config::environment::OAuthConfig {
        strava: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        fitbit: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        garmin: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        whoop: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
        terra: pierre_mcp_server::config::environment::OAuthProviderConfig::default(),
    });
    let tenant_oauth_client = Arc::new(TenantOAuthClient::new(TenantOAuthManager::new(
        oauth_config,
    )));

    let tenant1_creds = StoreCredentialsRequest {
        client_id: "tenant1_client".to_owned(),
        client_secret: "tenant1_secret".to_owned(),
        redirect_uri: "https://tenant1.com/callback".to_owned(),
        scopes: vec!["read".to_owned()],
        configured_by: user_id,
    };

    let tenant2_creds = StoreCredentialsRequest {
        client_id: "tenant2_client".to_owned(),
        client_secret: "tenant2_secret".to_owned(),
        redirect_uri: "https://tenant2.com/callback".to_owned(),
        scopes: vec!["read".to_owned(), "write".to_owned()],
        configured_by: user_id,
    };

    tenant_oauth_client
        .store_credentials(tenant1_id, "strava", tenant1_creds)
        .await?;
    tenant_oauth_client
        .store_credentials(tenant2_id, "strava", tenant2_creds)
        .await?;

    // Test that the same user gets different OAuth clients for different tenants
    let tenant1_context = TenantContext::new(
        tenant1_id,
        "Tenant One".to_owned(),
        user_id,
        TenantRole::Member,
    );

    let tenant2_context = TenantContext::new(
        tenant2_id,
        "Tenant Two".to_owned(),
        user_id,
        TenantRole::Member,
    );

    let oauth1 = tenant_oauth_client
        .get_oauth_client(&tenant1_context, "strava", &database)
        .await?;
    let oauth2 = tenant_oauth_client
        .get_oauth_client(&tenant2_context, "strava", &database)
        .await?;

    // Verify different configurations are used
    assert_eq!(oauth1.config().client_id, "tenant1_client");
    assert_eq!(oauth2.config().client_id, "tenant2_client");
    assert_ne!(oauth1.config().client_secret, oauth2.config().client_secret);

    println!("Tenant context switching validated");

    Ok(())
}

/// Helper function to create test server configuration
fn create_test_server_config() -> ServerConfig {
    use pierre_mcp_server::config::environment::*;
    use std::path::PathBuf;

    ServerConfig {
        http_port: 4000,
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
            postgres_pool: pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        },
        auth: AuthConfig {
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
            ..pierre_mcp_server::config::environment::AuthConfig::default()
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client".to_owned()),
                client_secret: Some("test_strava_secret".to_owned()),
                redirect_uri: Some("http://localhost:3000/oauth/strava/callback".to_owned()),
                scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: Vec::new(),
                enabled: false,
            },
            ..Default::default()
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
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: true,
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
            ..Default::default()
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
                server_name: "pierre-mcp-server-e2e-test".to_owned(),
                server_version: env!("CARGO_PKG_VERSION").to_owned(),
            },
        },
        sse: pierre_mcp_server::config::environment::SseConfig::default(),
        oauth2_server: pierre_mcp_server::config::environment::OAuth2ServerConfig::default(),
        route_timeouts: pierre_mcp_server::config::environment::RouteTimeoutConfig::default(),
        ..Default::default()
    }
}
