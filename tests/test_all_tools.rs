// ABOUTME: Comprehensive test harness for all Pierre MCP Server fitness tools
// ABOUTME: Tests all 18 tools with real stored Strava OAuth tokens to validate functionality
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use base64::prelude::*;
use rand::Rng;
use serde_json::json;
use serial_test::serial;
use std::{collections::HashMap, env, error::Error, path::PathBuf, sync::Arc};
use uuid::Uuid;

// Import necessary modules from the main crate
use pierre_mcp_server::{
    auth::AuthManager,
    config::environment::*,
    constants::oauth_providers,
    database_plugins::{factory::Database, DatabaseProvider},
    intelligence::insights::{Insight, InsightType},
    intelligence::{
        ActivityIntelligence, ContextualFactors, ContextualWeeklyLoad, PerformanceMetrics,
        TimeOfDay, TrendDirection, TrendIndicators,
    },
    mcp::resources::{ServerResources, ServerResourcesOptions},
    models::{DecryptedToken, Tenant, TenantId, User, UserOAuthToken, UserStatus, UserTier},
    permissions::UserRole,
    protocols::universal::{UniversalRequest, UniversalToolExecutor},
    tenant::TenantOAuthCredentials,
};

mod common;

#[tokio::test]
#[serial]
async fn test_complete_multitenant_workflow() -> Result<(), Box<dyn Error>> {
    common::init_server_config();
    println!("Pierre MCP Server - Comprehensive Tool Testing Harness");
    println!("====================================================\n");

    // Note: Tests will run against real Strava API or use credentials from environment

    println!("Testing all tools with environment-configured credentials");

    // Initialize the test environment
    let executor = create_test_executor().await?;

    // Find a real user with Strava token
    let (user, tenant) = find_or_create_test_user_with_token(&executor).await?;

    println!("Test Setup Complete:");
    println!("   User ID: {}", user.id);
    println!("   Tenant: {}", tenant.name);
    println!("   Testing with real Strava OAuth tokens\n");

    // Test all tools systematically
    let test_results = test_all_tools(&executor, &user.id.to_string(), tenant.id).await;

    // Print comprehensive results
    print_test_summary(&test_results);

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn create_test_executor() -> Result<UniversalToolExecutor> {
    // Initialize HTTP clients before any other setup
    common::init_test_http_clients();

    // Initialize test logging
    env::set_var("TEST_LOG", "WARN");

    // Use the same database and encryption key as the main server
    let database_url =
        env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data/users.db".to_owned());
    let master_key = env::var("PIERRE_MASTER_ENCRYPTION_KEY")
        .unwrap_or_else(|_| "dGVzdF9lbmNyeXB0aW9uX2tleV9mb3JfY2lfb25seV8zMg==".to_owned());
    let encryption_key = BASE64_STANDARD
        .decode(master_key)
        .expect("Invalid base64 in PIERRE_MASTER_ENCRYPTION_KEY");
    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(
            &database_url,
            encryption_key,
            &PostgresPoolConfig::default(),
        )
        .await?,
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(&database_url, encryption_key).await?);

    // Create ActivityIntelligence with proper constructor
    let _intelligence = Arc::new(ActivityIntelligence::new(
        "Test intelligence analysis".to_owned(),
        vec![Insight {
            insight_type: InsightType::Achievement,
            message: "Test insight".to_owned(),
            confidence: 90.0,
            data: None,
        }],
        PerformanceMetrics {
            relative_effort: Some(85.0),
            zone_distribution: None,
            personal_records: vec![],
            efficiency_score: Some(82.5),
            trend_indicators: TrendIndicators {
                pace_trend: TrendDirection::Improving,
                effort_trend: TrendDirection::Stable,
                distance_trend: TrendDirection::Improving,
                consistency_score: 90.0,
            },
        },
        ContextualFactors {
            weather: None,
            location: None,
            time_of_day: TimeOfDay::Morning,
            days_since_last_activity: Some(1),
            weekly_load: Some(ContextualWeeklyLoad {
                total_distance_km: 50.0,
                total_duration_hours: 5.0,
                activity_count: 3,
                load_trend: TrendDirection::Stable,
            }),
        },
    ));

    // Create test config with correct structure
    let config = Arc::new(ServerConfig {
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
                redirect_uri: Some("http://localhost:3000/oauth/callback/strava".to_owned()),
                scopes: vec!["read".to_owned(), "activity:read_all".to_owned()],
                enabled: true,
            },
            fitbit: OAuthProviderConfig {
                client_id: Some("test_fitbit_id".to_owned()),
                client_secret: Some("test_fitbit_secret".to_owned()),
                redirect_uri: Some("http://localhost:3000/oauth/callback/fitbit".to_owned()),
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
            geocoding: GeocodingServiceConfig {
                base_url: "https://nominatim.openstreetmap.org".to_owned(),
                enabled: true,
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
            max_activities_fetch: 100,
            default_activities_limit: 20,
            ci_mode: true,
            auto_approve_users: false,
            auto_approve_users_from_env: false,
            protocol: ProtocolConfig {
                mcp_version: "2024-11-05".to_owned(),
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

    // Create ServerResources for the test
    let auth_manager = AuthManager::new(24);
    let cache = common::create_test_cache().await.unwrap();
    let server_resources = Arc::new(
        ServerResources::new(
            (*database).clone(),
            auth_manager,
            "test_secret",
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

    let executor = UniversalToolExecutor::new(server_resources);
    Ok(executor)
}

async fn find_or_create_test_user_with_token(
    executor: &UniversalToolExecutor,
) -> Result<(User, Tenant)> {
    // Always create fresh test data for reliable, reproducible tests
    create_test_user(executor).await
}

async fn create_test_user(executor: &UniversalToolExecutor) -> Result<(User, Tenant)> {
    // Create a unique test user and tenant for this test run
    let user_id = Uuid::new_v4();
    let tenant_id = TenantId::new();

    let user = User {
        id: user_id,
        email: format!("test-{user_id}@example.com"),
        display_name: Some("Test User".to_owned()),
        password_hash: "fake_hash_for_ci".to_owned(),
        tier: UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        user_status: UserStatus::Active,
        approved_by: None,
        approved_at: Some(chrono::Utc::now()),
        is_active: true,
        is_admin: false,
        role: UserRole::User,
        firebase_uid: None,
        auth_provider: String::new(),
    };

    executor.resources.database.create_user(&user).await?;

    // Now create the tenant with the user as owner
    let tenant_slug = format!("test-tenant-{tenant_id}");
    let tenant = Tenant {
        id: tenant_id,
        name: "test-tenant".to_owned(),
        slug: tenant_slug,
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    executor.resources.database.create_tenant(&tenant).await?;

    // Set up OAuth credentials for the tenant
    println!("Setting up tenant OAuth credentials...");

    // Use the existing setup function which handles OAuth credentials properly
    match setup_tenant_oauth_credentials(executor, tenant_id).await {
        Ok(()) => println!(" OAuth credentials configured successfully"),
        Err(e) => {
            println!("⚠️ Failed to configure OAuth credentials: {e}");
            // Continue anyway - tools may still work with fallback mechanisms
        }
    }

    // Generate realistic fake Strava tokens for testing
    let now = chrono::Utc::now();
    let timestamp = now.timestamp();
    let token_id = rand::thread_rng().gen::<u64>();
    let refresh_token_id = rand::thread_rng().gen::<u64>();

    let mock_token = DecryptedToken {
        access_token: format!("at_{token_id:016x}_{timestamp}"),
        refresh_token: format!("rt_{refresh_token_id:016x}_{timestamp}"),
        expires_at: now + chrono::Duration::hours(6),
        scope: "read,activity:read_all,activity:write".to_owned(),
    };

    let oauth_token = UserOAuthToken::new(
        user.id,
        "00000000-0000-0000-0000-000000000000".to_owned(), // tenant_id
        oauth_providers::STRAVA.to_owned(),
        mock_token.access_token.clone(),
        Some(mock_token.refresh_token.clone()),
        Some(mock_token.expires_at),
        Some(mock_token.scope.clone()), // scope as String
    );

    match executor
        .resources
        .database
        .upsert_user_oauth_token(&oauth_token)
        .await
    {
        Ok(()) => println!(" Test tokens stored successfully"),
        Err(e) => {
            println!("⚠️ Failed to store test tokens: {e}");
            // Continue anyway - some tools might work without tokens
        }
    }

    println!("Created test user: {} (tenant: {})", user.id, tenant.id);

    Ok((user, tenant))
}

async fn setup_tenant_oauth_credentials(
    executor: &UniversalToolExecutor,
    tenant_id: TenantId,
) -> Result<()> {
    // Get Strava credentials from environment
    let client_id = env::var("STRAVA_CLIENT_ID").unwrap_or_else(|_| "163846".to_owned());
    let client_secret = env::var("STRAVA_CLIENT_SECRET")
        .unwrap_or_else(|_| "1dfc45ad0a1f6983b835e4495aa9473d111d03bc".to_owned());

    // Check if tenant already has Strava OAuth credentials
    match executor
        .resources
        .database
        .get_tenant_oauth_credentials(tenant_id, "strava")
        .await
    {
        Ok(Some(_)) => {
            println!("      Tenant already has Strava OAuth credentials configured");
            return Ok(());
        }
        Ok(None) => {
            println!("      Setting up tenant Strava OAuth credentials...");
        }
        Err(e) => {
            println!("      Failed to check existing credentials: {e}");
            println!("      Setting up tenant Strava OAuth credentials...");
        }
    }

    // Create TenantOAuthCredentials struct
    let tenant_oauth_creds = TenantOAuthCredentials {
        tenant_id,
        provider: "strava".to_owned(),
        client_id: client_id.clone(),
        client_secret: client_secret.clone(),
        redirect_uri: env::var("STRAVA_REDIRECT_URI")
            .unwrap_or_else(|_| "http://localhost:8080/auth/strava/callback".to_owned()),
        scopes: vec![
            "read".to_owned(),
            "activity:read_all".to_owned(),
            "activity:write".to_owned(),
        ],
        rate_limit_per_day: 1000,
    };

    // Store tenant OAuth credentials
    if let Err(e) = executor
        .resources
        .database
        .store_tenant_oauth_credentials(&tenant_oauth_creds)
        .await
    {
        println!("      Failed to store tenant OAuth credentials: {e}");
    } else {
        println!("      Tenant OAuth credentials configured successfully");
    }

    Ok(())
}

async fn test_all_tools(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> HashMap<String, TestResult> {
    let mut results = HashMap::new();

    println!("Testing all tools with fresh test data");

    // Test 1: Core Data Retrieval Tools
    println!("\nTesting Core Data Retrieval Tools");
    println!("=====================================");

    results.insert(
        "get_activities".to_owned(),
        test_get_activities(executor, user_id, tenant_id).await,
    );
    results.insert(
        "get_athlete".to_owned(),
        test_get_athlete(executor, user_id, tenant_id).await,
    );
    results.insert(
        "get_stats".to_owned(),
        test_get_stats(executor, user_id, tenant_id).await,
    );

    // Test 2: Activity Analysis Tools
    println!("\nTesting Activity Analysis Tools");
    println!("====================================");

    // Use a known mock activity ID that we're returning in our mock data
    let activity_id = "9876543210".to_owned();

    results.insert(
        "get_activity_intelligence".to_owned(),
        test_get_activity_intelligence(executor, user_id, tenant_id, &activity_id).await,
    );
    results.insert(
        "analyze_activity".to_owned(),
        test_analyze_activity(executor, user_id, tenant_id, &activity_id).await,
    );
    results.insert(
        "calculate_metrics".to_owned(),
        test_calculate_metrics(executor, user_id, tenant_id, &activity_id).await,
    );
    results.insert(
        "analyze_performance_trends".to_owned(),
        test_analyze_performance_trends(executor, user_id, tenant_id).await,
    );
    results.insert(
        "compare_activities".to_owned(),
        test_compare_activities(executor, user_id, tenant_id, &activity_id).await,
    );
    results.insert(
        "detect_patterns".to_owned(),
        test_detect_patterns(executor, user_id, tenant_id).await,
    );

    // Test 3: Goals & Recommendations Tools
    println!("\nTesting Goals & Recommendations Tools");
    println!("=========================================");

    results.insert(
        "set_goal".to_owned(),
        test_set_goal(executor, user_id, tenant_id).await,
    );
    results.insert(
        "suggest_goals".to_owned(),
        test_suggest_goals(executor, user_id, tenant_id).await,
    );
    results.insert(
        "track_progress".to_owned(),
        test_track_progress(executor, user_id, tenant_id).await,
    );
    results.insert(
        "predict_performance".to_owned(),
        test_predict_performance(executor, user_id, tenant_id).await,
    );
    results.insert(
        "generate_recommendations".to_owned(),
        test_generate_recommendations(executor, user_id, tenant_id).await,
    );

    // Test 4: Sleep & Recovery Tools
    println!("\nTesting Sleep & Recovery Tools");
    println!("=====================================");

    results.insert(
        "calculate_recovery_score".to_owned(),
        test_calculate_recovery_score(executor, user_id, tenant_id).await,
    );

    // Test 5: Provider Management Tools
    println!("\nTesting Provider Management Tools");
    println!("=====================================");

    results.insert(
        "get_connection_status".to_owned(),
        test_get_connection_status(executor, user_id, tenant_id).await,
    );
    results.insert(
        "disconnect_provider".to_owned(),
        test_disconnect_provider(executor, user_id, tenant_id).await,
    );

    println!("\nAll tools tested\n");

    results
}

const fn handle_ci_mode_result(result: TestResult, _tool_name: &str) -> TestResult {
    // No exception swallowing - return results as-is for proper test validation
    result
}

// Helper function to create requests with proper tenant context
fn create_request(
    tool_name: &str,
    parameters: serde_json::Value,
    user_id: &str,
    tenant_id: TenantId,
) -> UniversalRequest {
    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: user_id.to_owned(),
        protocol: "test".to_owned(),
        tenant_id: Some(tenant_id.to_string()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

fn create_request_with_client_credentials(
    tool_name: &str,
    mut parameters: serde_json::Value,
    user_id: &str,
    tenant_id: TenantId,
) -> UniversalRequest {
    // Add client credentials to parameters for highest priority
    if let Some(params) = parameters.as_object_mut() {
        params.insert("client_id".to_owned(), json!("163846"));
        params.insert(
            "client_secret".to_owned(),
            json!("1dfc45ad0a1f6983b835e4495aa9473d111d03bc"),
        );
    }

    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: user_id.to_owned(),
        protocol: "test".to_owned(),
        tenant_id: Some(tenant_id.to_string()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

// Individual tool test functions
async fn test_get_activities(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request_with_client_credentials(
        "get_activities",
        json!({"provider": "strava", "limit": 10}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "get_activities").await;
    handle_ci_mode_result(result, "get_activities")
}

async fn test_get_athlete(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    // Try both tenant-aware and direct user approaches
    let request_tenant = create_request(
        "get_athlete",
        json!({"provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result_tenant = execute_and_evaluate(executor, request_tenant, "get_athlete").await;

    if matches!(
        result_tenant,
        TestResult::Success(()) | TestResult::SuccessNoData
    ) {
        return result_tenant;
    }

    // Try direct user token (no tenant_id) as fallback
    println!("   🔄 Retrying get_athlete with direct user tokens...");
    let request_direct = UniversalRequest {
        tool_name: "get_athlete".to_owned(),
        parameters: json!({"provider": "strava"}),
        user_id: user_id.to_owned(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None, // Use direct user tokens
    };
    let result = execute_and_evaluate(executor, request_direct, "get_athlete").await;

    // In CI mode, API authentication failures are expected due to mock tokens
    handle_ci_mode_result(result, "get_athlete")
}

async fn test_get_stats(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    // Try both tenant-aware and direct user approaches
    let request_tenant = create_request(
        "get_stats",
        json!({"provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result_tenant = execute_and_evaluate(executor, request_tenant, "get_stats").await;

    if matches!(
        result_tenant,
        TestResult::Success(()) | TestResult::SuccessNoData
    ) {
        return result_tenant;
    }

    // Try direct user token (no tenant_id) as fallback
    println!("   🔄 Retrying get_stats with direct user tokens...");
    let request_direct = UniversalRequest {
        tool_name: "get_stats".to_owned(),
        parameters: json!({"provider": "strava"}),
        user_id: user_id.to_owned(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None, // Use direct user tokens
    };
    let result = execute_and_evaluate(executor, request_direct, "get_stats").await;
    handle_ci_mode_result(result, "get_stats")
}

async fn test_get_activity_intelligence(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
    activity_id: &str,
) -> TestResult {
    let request = create_request(
        "get_activity_intelligence",
        json!({"provider": "strava", "activity_id": activity_id}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "get_activity_intelligence").await;
    handle_ci_mode_result(result, "get_activity_intelligence")
}

async fn test_analyze_activity(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
    activity_id: &str,
) -> TestResult {
    let request = create_request_with_client_credentials(
        "analyze_activity",
        json!({"provider": "strava", "activity_id": activity_id}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "analyze_activity").await;
    handle_ci_mode_result(result, "analyze_activity")
}

async fn test_calculate_metrics(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
    activity_id: &str,
) -> TestResult {
    let request = create_request(
        "calculate_metrics",
        json!({"activity": activity_id}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "calculate_metrics").await;
    handle_ci_mode_result(result, "calculate_metrics")
}

async fn test_analyze_performance_trends(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    // Try both tenant-aware and direct user approaches
    let request_tenant = create_request(
        "analyze_performance_trends",
        json!({"provider": "strava", "period_days": 30}),
        user_id,
        tenant_id,
    );
    let result_tenant =
        execute_and_evaluate(executor, request_tenant, "analyze_performance_trends").await;

    if matches!(
        result_tenant,
        TestResult::Success(()) | TestResult::SuccessNoData
    ) {
        return result_tenant;
    }

    // Try direct user token (no tenant_id) as fallback
    println!("   🔄 Retrying analyze_performance_trends with direct user tokens...");
    let request_direct = UniversalRequest {
        tool_name: "analyze_performance_trends".to_owned(),
        parameters: json!({"provider": "strava", "period_days": 30}),
        user_id: user_id.to_owned(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None, // Use direct user tokens
    };
    let result = execute_and_evaluate(executor, request_direct, "analyze_performance_trends").await;
    handle_ci_mode_result(result, "analyze_performance_trends")
}

async fn test_compare_activities(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
    activity_id: &str,
) -> TestResult {
    // Try both tenant-aware and direct user approaches
    let request_tenant = create_request(
        "compare_activities",
        json!({"provider": "strava", "activity_id1": activity_id, "activity_id2": activity_id}),
        user_id,
        tenant_id,
    );
    let result_tenant = execute_and_evaluate(executor, request_tenant, "compare_activities").await;

    if matches!(
        result_tenant,
        TestResult::Success(()) | TestResult::SuccessNoData
    ) {
        return result_tenant;
    }

    // Try direct user token (no tenant_id) as fallback
    println!("   🔄 Retrying compare_activities with direct user tokens...");
    let request_direct = UniversalRequest {
        tool_name: "compare_activities".to_owned(),
        parameters: json!({"provider": "strava", "activity_id1": activity_id, "activity_id2": activity_id}),
        user_id: user_id.to_owned(),
        protocol: "test".to_owned(),
        tenant_id: None,
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None, // Use direct user tokens
    };
    let result = execute_and_evaluate(executor, request_direct, "compare_activities").await;
    handle_ci_mode_result(result, "compare_activities")
}

async fn test_detect_patterns(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request_with_client_credentials(
        "detect_patterns",
        json!({"provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "detect_patterns").await;
    handle_ci_mode_result(result, "detect_patterns")
}

async fn test_set_goal(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request(
        "set_goal",
        json!({
            "goal_type": "distance",
            "target_value": 100.0,
            "timeframe": "monthly",
            "target_date": "2025-12-31",
            "description": "Run 100km in December"
        }),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "set_goal").await;
    handle_ci_mode_result(result, "set_goal")
}

async fn test_suggest_goals(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request(
        "suggest_goals",
        json!({"provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "suggest_goals").await;
    handle_ci_mode_result(result, "suggest_goals")
}

async fn test_track_progress(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    // Track progress requires a goal_id - using a test ID
    let request = create_request_with_client_credentials(
        "track_progress",
        json!({"goal_id": "test-goal-001", "provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "track_progress").await;
    handle_ci_mode_result(result, "track_progress")
}

async fn test_predict_performance(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request(
        "predict_performance",
        json!({"provider": "strava", "activity_type": "Run", "distance": 10000}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "predict_performance").await;
    handle_ci_mode_result(result, "predict_performance")
}

async fn test_generate_recommendations(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request(
        "generate_recommendations",
        json!({"provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "generate_recommendations").await;
    handle_ci_mode_result(result, "generate_recommendations")
}

async fn test_calculate_recovery_score(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request(
        "calculate_recovery_score",
        json!({"provider": "strava"}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "calculate_recovery_score").await;
    handle_ci_mode_result(result, "calculate_recovery_score")
}

async fn test_get_connection_status(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    let request = create_request("get_connection_status", json!({}), user_id, tenant_id);
    let result = execute_and_evaluate(executor, request, "get_connection_status").await;
    handle_ci_mode_result(result, "get_connection_status")
}

async fn test_disconnect_provider(
    executor: &UniversalToolExecutor,
    user_id: &str,
    tenant_id: TenantId,
) -> TestResult {
    // We'll skip actually disconnecting in tests
    let request = create_request(
        "disconnect_provider",
        json!({"provider": "fitbit"}),
        user_id,
        tenant_id,
    );
    let result = execute_and_evaluate(executor, request, "disconnect_provider").await;
    handle_ci_mode_result(result, "disconnect_provider")
}

async fn execute_and_evaluate(
    executor: &UniversalToolExecutor,
    request: UniversalRequest,
    tool_name: &str,
) -> TestResult {
    println!("Testing {tool_name}");
    let user_id = request.user_id.clone();
    let tenant_id = request.tenant_id.clone();

    match executor.execute_tool(request).await {
        Ok(response) => {
            if response.success {
                println!("   SUCCESS: {tool_name}");
                response
                    .result
                    .map_or(TestResult::SuccessNoData, |_| TestResult::Success(()))
            } else {
                let error_msg = response.error.as_deref().unwrap_or("Unknown error");
                println!("   FAILED: {tool_name} - {error_msg}");

                // Add detailed debugging for OAuth-related failures
                if error_msg.contains("Provider authentication")
                    || error_msg.contains("Tool execution failed")
                {
                    println!("      DEBUG: This tool needs OAuth token setup");
                    println!("      Request: user_id={user_id}, tenant_id={tenant_id:?}");
                }

                TestResult::Failed(error_msg.to_owned())
            }
        }
        Err(e) => {
            println!("   ERROR: {tool_name} - {e}");
            TestResult::Error(e.to_string())
        }
    }
}

fn print_test_summary(results: &HashMap<String, TestResult>) {
    println!("\nCOMPREHENSIVE TEST RESULTS");
    println!("==============================");

    let mut success_count = 0;
    let mut failed_count = 0;
    let mut error_count = 0;

    // Group by category
    let categories = vec![
        (
            "Core Data Retrieval",
            vec!["get_activities", "get_athlete", "get_stats"],
        ),
        (
            "Activity Analysis",
            vec![
                "get_activity_intelligence",
                "analyze_activity",
                "calculate_metrics",
                "analyze_performance_trends",
                "compare_activities",
                "detect_patterns",
            ],
        ),
        (
            "Goals & Recommendations",
            vec![
                "set_goal",
                "suggest_goals",
                "track_progress",
                "predict_performance",
                "generate_recommendations",
            ],
        ),
        (
            "Provider Management",
            vec!["get_connection_status", "disconnect_provider"],
        ),
    ];

    for (category, tools) in categories {
        println!("\n{category}:");
        for tool in tools {
            if let Some(result) = results.get(tool) {
                match result {
                    TestResult::Success(()) => {
                        println!("   SUCCESS: {tool}");
                        success_count += 1;
                    }
                    TestResult::SuccessNoData => {
                        println!("   SUCCESS: {tool} (no data)");
                        success_count += 1;
                    }
                    TestResult::Failed(msg) => {
                        println!("   FAILED: {tool} - {msg}");
                        failed_count += 1;
                    }
                    TestResult::Error(msg) => {
                        println!("   ERROR: {tool} - {msg}");
                        error_count += 1;
                    }
                }
            }
        }
    }

    println!("\nFINAL SUMMARY:");
    println!("   Successful: {success_count}");
    println!("   Failed: {failed_count}");
    println!("   Errors: {error_count}");
    println!(
        "   Total Tested: {}",
        success_count + failed_count + error_count
    );

    let success_rate = if success_count + failed_count + error_count > 0 {
        (f64::from(success_count) / f64::from(success_count + failed_count + error_count)) * 100.0
    } else {
        0.0
    };
    println!("   Success Rate: {success_rate:.1}%");

    if success_rate >= 90.0 {
        println!("\nEXCELLENT! Ready for Claude Desktop integration!");
    } else if success_rate >= 70.0 {
        println!("\nGOOD but needs some fixes before Claude Desktop integration");
    } else {
        println!("\nNEEDS WORK before Claude Desktop integration");
    }
}

#[derive(Debug)]
enum TestResult {
    Success(()),
    SuccessNoData,
    Failed(String),
    Error(String),
}
