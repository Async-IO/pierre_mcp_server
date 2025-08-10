// ABOUTME: End-to-end integration test for complete multi-tenant onboarding workflow
// ABOUTME: Tests tenant creation, OAuth app registration, credential management, and tool execution

//! # End-to-End Tenant Onboarding Test
//!
//! This test demonstrates the complete multi-tenant onboarding workflow:
//! 1. Create a new tenant with admin user
//! 2. Register OAuth applications for fitness providers
//! 3. Configure tenant-specific OAuth credentials
//! 4. Execute tools using tenant-isolated credentials
//! 5. Verify proper isolation between tenants

use anyhow::Result;
use pierre_mcp_server::{
    config::environment::ServerConfig,
    database_plugins::{factory::Database, DatabaseProvider},
    intelligence::ActivityIntelligence,
    models::{OAuthApp, Tenant, User, UserTier},
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
    tenant::{StoreCredentialsRequest, TenantContext, TenantOAuthClient, TenantRole},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Test configuration for end-to-end tenant onboarding
#[allow(clippy::too_many_lines)] // Long function: Defines complete end-to-end tenant onboarding workflow
#[tokio::test]
async fn test_complete_tenant_onboarding_workflow() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Step 1: Create test database and base infrastructure
    let database = Arc::new(
        Database::new("sqlite::memory:", vec![0; 32])
            .await
            .expect("Failed to create test database"),
    );

    // Step 2: Create admin users first
    let acme_admin_id = Uuid::new_v4();
    let beta_admin_id = Uuid::new_v4();

    // Step 3: Create first tenant ("Acme Fitness Co.")
    let acme_tenant_id = Uuid::new_v4();
    let acme_tenant = Tenant {
        id: acme_tenant_id,
        name: "Acme Fitness Co.".to_string(),
        slug: "acme-fitness".to_string(),
        domain: None,
        plan: "enterprise".to_string(),
        owner_user_id: acme_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&acme_tenant).await?;

    // Step 4: Create second tenant ("Beta Health Inc.") for isolation testing
    let beta_tenant_id = Uuid::new_v4();
    let beta_tenant = Tenant {
        id: beta_tenant_id,
        name: "Beta Health Inc.".to_string(),
        slug: "beta-health".to_string(),
        domain: None,
        plan: "professional".to_string(),
        owner_user_id: beta_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&beta_tenant).await?;

    // Step 5: Create admin users for each tenant
    let acme_admin = User {
        id: acme_admin_id,
        email: "admin@acmefitness.com".to_string(),
        display_name: Some("Acme Admin".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: UserTier::Enterprise,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
    };

    let beta_admin = User {
        id: beta_admin_id,
        email: "admin@betahealth.com".to_string(),
        display_name: Some("Beta Admin".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
    };

    database.create_user(&acme_admin).await?;
    database.create_user(&beta_admin).await?;

    // Step 5: Register OAuth applications for each tenant
    let acme_strava_app = OAuthApp {
        id: Uuid::new_v4(),
        client_id: "acme_strava_client_123".to_string(),
        client_secret: "encrypted_acme_secret".to_string(),
        name: "Acme Fitness Strava Integration".to_string(),
        description: Some("Strava integration for Acme Fitness".to_string()),
        redirect_uris: vec!["https://acme-fitness.com/oauth/strava/callback".to_string()],
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        app_type: "server".to_string(),
        owner_user_id: acme_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let beta_strava_app = OAuthApp {
        id: Uuid::new_v4(),
        client_id: "beta_strava_client_456".to_string(),
        client_secret: "encrypted_beta_secret".to_string(),
        name: "Beta Health Strava Integration".to_string(),
        description: Some("Strava integration for Beta Health".to_string()),
        redirect_uris: vec!["https://beta-health.com/oauth/strava/callback".to_string()],
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        app_type: "server".to_string(),
        owner_user_id: beta_admin_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_oauth_app(&acme_strava_app).await?;
    database.create_oauth_app(&beta_strava_app).await?;

    // Step 6: Set up tenant OAuth client and configure credentials
    let tenant_oauth_client = Arc::new(TenantOAuthClient::new());

    // Configure Acme's Strava credentials
    let acme_credentials = StoreCredentialsRequest {
        client_id: "acme_strava_client_123".to_string(),
        client_secret: "acme_secret_key".to_string(),
        redirect_uri: "https://acme-fitness.com/oauth/strava/callback".to_string(),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        configured_by: acme_admin_id,
    };

    tenant_oauth_client
        .store_credentials(acme_tenant_id, "strava", acme_credentials)
        .await?;

    // Configure Beta's Strava credentials
    let beta_credentials = StoreCredentialsRequest {
        client_id: "beta_strava_client_456".to_string(),
        client_secret: "beta_secret_key".to_string(),
        redirect_uri: "https://beta-health.com/oauth/strava/callback".to_string(),
        scopes: vec!["read".to_string(), "activity:read_all".to_string()],
        configured_by: beta_admin_id,
    };

    tenant_oauth_client
        .store_credentials(beta_tenant_id, "strava", beta_credentials)
        .await?;

    // Step 7: Create Universal Tool Executor with tenant OAuth support
    let intelligence = Arc::new(ActivityIntelligence::new(
        "E2E Test Intelligence".to_string(),
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

    let executor = UniversalToolExecutor::new(
        database.clone(),
        intelligence,
        config,
        tenant_oauth_client.clone(),
    );

    // Step 8: Test tenant-aware tool execution for Acme
    let acme_context = TenantContext::new(
        acme_tenant_id,
        "Acme Fitness Co.".to_string(),
        acme_admin_id,
        TenantRole::Admin,
    );

    let acme_request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: acme_admin_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: Some(acme_tenant_id.to_string()),
    };

    let acme_response = executor.execute_tool(acme_request).await?;
    assert!(acme_response.success);
    println!("✅ Acme tenant tool execution successful");

    // Step 9: Test tenant-aware tool execution for Beta
    let beta_context = TenantContext::new(
        beta_tenant_id,
        "Beta Health Inc.".to_string(),
        beta_admin_id,
        TenantRole::Admin,
    );

    let beta_request = UniversalRequest {
        tool_name: "get_connection_status".to_string(),
        parameters: json!({}),
        user_id: beta_admin_id.to_string(),
        protocol: "test".to_string(),
        tenant_id: Some(beta_tenant_id.to_string()),
    };

    let beta_response = executor.execute_tool(beta_request).await?;
    assert!(beta_response.success);
    println!("✅ Beta tenant tool execution successful");

    // Step 10: Verify tenant isolation - check OAuth credentials
    let acme_oauth_creds = tenant_oauth_client
        .get_tenant_credentials(acme_tenant_id, "strava")
        .await?;
    let beta_oauth_creds = tenant_oauth_client
        .get_tenant_credentials(beta_tenant_id, "strava")
        .await?;

    assert!(acme_oauth_creds.is_some());
    assert!(beta_oauth_creds.is_some());

    let acme_creds = acme_oauth_creds.unwrap();
    let beta_creds = beta_oauth_creds.unwrap();

    // Verify credentials are isolated
    assert_eq!(acme_creds.client_id, "acme_strava_client_123");
    assert_eq!(beta_creds.client_id, "beta_strava_client_456");
    assert_ne!(acme_creds.client_secret, beta_creds.client_secret);

    println!("✅ Tenant OAuth credential isolation verified");

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

    println!("✅ Tenant rate limiting isolation verified");

    // Step 12: Test OAuth authorization URL generation for each tenant
    let acme_auth_url = tenant_oauth_client
        .get_authorization_url(&acme_context, "strava", "acme_state_123")
        .await?;

    let beta_auth_url = tenant_oauth_client
        .get_authorization_url(&beta_context, "strava", "beta_state_456")
        .await?;

    // Verify URLs contain tenant-specific client IDs
    assert!(acme_auth_url.contains("acme_strava_client_123"));
    assert!(beta_auth_url.contains("beta_strava_client_456"));

    println!("✅ Tenant-specific OAuth authorization URLs generated");

    // Step 13: Comprehensive workflow validation
    println!("\n🎉 END-TO-END TENANT ONBOARDING WORKFLOW COMPLETED SUCCESSFULLY!");
    println!("   ✅ Multi-tenant database setup");
    println!("   ✅ Tenant creation and user management");
    println!("   ✅ OAuth application registration per tenant");
    println!("   ✅ Tenant-specific credential configuration");
    println!("   ✅ Isolated tool execution per tenant");
    println!("   ✅ OAuth credential isolation verification");
    println!("   ✅ Rate limiting isolation");
    println!("   ✅ Tenant-specific OAuth URLs");

    Ok(())
}

/// Test tenant switching and context validation
#[tokio::test]
async fn test_tenant_context_switching() -> Result<()> {
    let database = Arc::new(
        Database::new("sqlite::memory:", vec![0; 32])
            .await
            .expect("Failed to create test database"),
    );

    // Create a user that belongs to multiple tenants
    let user_id = Uuid::new_v4();
    let user = User {
        id: user_id,
        email: "multi-tenant-user@example.com".to_string(),
        display_name: Some("Multi Tenant User".to_string()),
        password_hash: "hashed_password".to_string(),
        tier: UserTier::Professional,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
    };

    database.create_user(&user).await?;

    // Create two tenants
    let tenant1_id = Uuid::new_v4();
    let tenant1 = Tenant {
        id: tenant1_id,
        name: "Tenant One".to_string(),
        slug: "tenant-one".to_string(),
        domain: None,
        plan: "basic".to_string(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let tenant2_id = Uuid::new_v4();
    let tenant2 = Tenant {
        id: tenant2_id,
        name: "Tenant Two".to_string(),
        slug: "tenant-two".to_string(),
        domain: None,
        plan: "premium".to_string(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    database.create_tenant(&tenant1).await?;
    database.create_tenant(&tenant2).await?;

    // Set up different OAuth credentials for each tenant
    let tenant_oauth_client = Arc::new(TenantOAuthClient::new());

    let tenant1_creds = StoreCredentialsRequest {
        client_id: "tenant1_client".to_string(),
        client_secret: "tenant1_secret".to_string(),
        redirect_uri: "https://tenant1.com/callback".to_string(),
        scopes: vec!["read".to_string()],
        configured_by: user_id,
    };

    let tenant2_creds = StoreCredentialsRequest {
        client_id: "tenant2_client".to_string(),
        client_secret: "tenant2_secret".to_string(),
        redirect_uri: "https://tenant2.com/callback".to_string(),
        scopes: vec!["read".to_string(), "write".to_string()],
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
        "Tenant One".to_string(),
        user_id,
        TenantRole::Member,
    );

    let tenant2_context = TenantContext::new(
        tenant2_id,
        "Tenant Two".to_string(),
        user_id,
        TenantRole::Member,
    );

    let oauth1 = tenant_oauth_client
        .get_oauth_client(&tenant1_context, "strava")
        .await?;
    let oauth2 = tenant_oauth_client
        .get_oauth_client(&tenant2_context, "strava")
        .await?;

    // Verify different configurations are used
    assert_eq!(oauth1.config().client_id, "tenant1_client");
    assert_eq!(oauth2.config().client_id, "tenant2_client");
    assert_ne!(oauth1.config().client_secret, oauth2.config().client_secret);

    println!("✅ Tenant context switching validated");

    Ok(())
}

/// Helper function to create test server configuration
fn create_test_server_config() -> ServerConfig {
    use pierre_mcp_server::config::environment::*;
    use std::path::PathBuf;

    ServerConfig {
        mcp_port: 3000,
        http_port: 4000,
        log_level: LogLevel::Info,
        database: DatabaseConfig {
            url: DatabaseUrl::Memory,
            encryption_key_path: PathBuf::from("test.key"),
            auto_migrate: true,
            backup: BackupConfig {
                enabled: false,
                interval_seconds: 3600,
                retention_count: 7,
                directory: PathBuf::from("test_backups"),
            },
        },
        auth: AuthConfig {
            jwt_secret_path: PathBuf::from("test.secret"),
            jwt_expiry_hours: 24,
            enable_refresh_tokens: false,
        },
        oauth: OAuthConfig {
            strava: OAuthProviderConfig {
                client_id: Some("test_strava_client".to_string()),
                client_secret: Some("test_strava_secret".to_string()),
                redirect_uri: Some("http://localhost:3000/oauth/strava/callback".to_string()),
                scopes: vec!["read".to_string(), "activity:read_all".to_string()],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: None,
                client_secret: None,
                redirect_uri: None,
                scopes: Vec::new(),
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
                environment: Environment::Development,
            },
        },
        external_services: ExternalServicesConfig {
            weather: WeatherServiceConfig {
                api_key: None,
                base_url: "https://api.openweathermap.org/data/2.5".to_string(),
                enabled: false,
            },
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_string(),
                enabled: true,
            },
            strava_api: StravaApiConfig {
                base_url: "https://www.strava.com/api/v3".to_string(),
                auth_url: "https://www.strava.com/oauth/authorize".to_string(),
                token_url: "https://www.strava.com/oauth/token".to_string(),
            },
            fitbit_api: FitbitApiConfig {
                base_url: "https://api.fitbit.com".to_string(),
                auth_url: "https://www.fitbit.com/oauth2/authorize".to_string(),
                token_url: "https://api.fitbit.com/oauth2/token".to_string(),
            },
        },
        app_behavior: AppBehaviorConfig {
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_string(),
                server_name: "pierre-mcp-server-e2e-test".to_string(),
                server_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        },
    }
}
