// ABOUTME: Shared test utilities and setup functions for integration tests
// ABOUTME: Provides common database, auth, and user creation helpers
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(
    dead_code,
    clippy::wildcard_in_or_patterns,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::uninlined_format_args,
    clippy::redundant_closure_for_method_calls
)]
//! Shared test utilities for `pierre_mcp_server`
//!
//! This module provides common test setup functions to reduce duplication
//! across integration tests.

use anyhow::Result;
#[cfg(feature = "postgresql")]
use pierre_mcp_server::config::environment::PostgresPoolConfig;
use pierre_mcp_server::{
    admin::jwks::JwksManager,
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::AuthManager,
    cache::{factory::Cache, CacheConfig},
    config::{
        self,
        environment::{RateLimitConfig, ServerConfig},
    },
    constants,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::{ServerResources, ServerResourcesOptions},
    middleware::McpAuthMiddleware,
    models::{Tenant, TenantId, User, UserStatus, UserTier},
    routes::mcp::McpRoutes,
    utils,
};
use rand::Rng;
#[cfg(feature = "postgresql")]
use std::thread;
use std::{
    env,
    net::TcpListener,
    path::Path,
    process::{ChildStderr, ChildStdin, ChildStdout},
    sync::{Arc, LazyLock, Once},
    time::Duration as StdDuration,
};
#[cfg(feature = "postgresql")]
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::{net::TcpListener as TokioTcpListener, task::JoinHandle, time::sleep as tokio_sleep};
use uuid::Uuid;

#[cfg(feature = "postgresql")]
use sqlx::postgres::PgPoolOptions;

static INIT_LOGGER: Once = Once::new();
static INIT_HTTP_CLIENTS: Once = Once::new();
static INIT_SERVER_CONFIG: Once = Once::new();

/// Initialize server configuration for tests (call once per test process)
pub fn init_server_config() {
    INIT_SERVER_CONFIG.call_once(|| {
        env::set_var("CI", "true");
        env::set_var("DATABASE_URL", "sqlite::memory:");

        // Set OAuth environment variables for testing
        env::set_var("PIERRE_STRAVA_CLIENT_ID", "test_strava_client_id");
        env::set_var("PIERRE_STRAVA_CLIENT_SECRET", "test_strava_client_secret");
        env::set_var("PIERRE_GARMIN_CLIENT_ID", "test_garmin_client_id");
        env::set_var("PIERRE_GARMIN_CLIENT_SECRET", "test_garmin_client_secret");
        env::set_var("PIERRE_FITBIT_CLIENT_ID", "test_fitbit_client_id");
        env::set_var("PIERRE_FITBIT_CLIENT_SECRET", "test_fitbit_client_secret");

        let _ = constants::init_server_config();
    });
}

/// Shared JWKS manager for all tests (generated once, reused everywhere)
/// This eliminates expensive RSA key generation (100ms+ per key) in every test
static SHARED_TEST_JWKS: LazyLock<Arc<JwksManager>> = LazyLock::new(|| {
    let mut jwks = JwksManager::new();
    jwks.generate_rsa_key_pair_with_size("shared_test_key", 2048)
        .expect("Failed to generate shared test JWKS key");
    Arc::new(jwks)
});

/// Initialize quiet logging for tests (call once per test process)
pub fn init_test_logging() {
    INIT_LOGGER.call_once(|| {
        // Check for TEST_LOG environment variable to control test logging level
        let log_level = match env::var("TEST_LOG").as_deref() {
            Ok("TRACE") => tracing::Level::TRACE,
            Ok("DEBUG") => tracing::Level::DEBUG,
            Ok("INFO") => tracing::Level::INFO,
            Ok("WARN" | "ERROR") | _ => tracing::Level::WARN, // Default to WARN for quiet tests
        };

        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_test_writer()
            .init();
    });
}

/// Initialize HTTP clients for tests (call once per test process)
///
/// This function ensures HTTP client configuration is initialized exactly once
/// across all tests in the process. It uses default configuration suitable for testing.
///
/// Call this function at the start of any test that uses HTTP clients, either directly
/// or indirectly through providers or other components that make HTTP requests.
///
/// Safe to call multiple times - initialization happens only once due to `Once` guard.
pub fn init_test_http_clients() {
    INIT_HTTP_CLIENTS.call_once(|| {
        utils::http_client::initialize_http_clients(
            config::environment::HttpClientConfig::default(),
        );
    });
}

/// Standard test database setup
pub async fn create_test_database() -> Result<Arc<Database>> {
    init_test_logging();
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(database_url, encryption_key, &PostgresPoolConfig::default()).await?,
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await?);

    Ok(database)
}

/// Standard test database setup with custom encryption key
pub async fn create_test_database_with_key(encryption_key: Vec<u8>) -> Result<Arc<Database>> {
    init_test_logging();
    let database_url = "sqlite::memory:";

    #[cfg(feature = "postgresql")]
    let database = Arc::new(
        Database::new(database_url, encryption_key, &PostgresPoolConfig::default()).await?,
    );

    #[cfg(not(feature = "postgresql"))]
    let database = Arc::new(Database::new(database_url, encryption_key).await?);

    Ok(database)
}

/// Get shared test JWKS manager (reused across all tests for performance)
pub fn get_shared_test_jwks() -> Arc<JwksManager> {
    SHARED_TEST_JWKS.clone()
}

/// Create test authentication manager
pub fn create_test_auth_manager() -> Arc<AuthManager> {
    Arc::new(AuthManager::new(24))
}

/// Create test authentication middleware
pub fn create_test_auth_middleware(
    auth_manager: &Arc<AuthManager>,
    database: Arc<Database>,
) -> Arc<McpAuthMiddleware> {
    // Use shared JWKS manager instead of generating new keys
    let jwks_manager = get_shared_test_jwks();
    Arc::new(McpAuthMiddleware::new(
        (**auth_manager).clone(),
        database,
        jwks_manager,
        RateLimitConfig::default(),
    ))
}

/// Create test cache with background cleanup disabled
pub async fn create_test_cache() -> Result<Cache> {
    let cache_config = CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: StdDuration::from_secs(60),
        enable_background_cleanup: false, // Disable background cleanup for tests
        ..Default::default()
    };
    Cache::new(cache_config)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to create test cache: {e}"))
}

/// Create a standard test user
pub async fn create_test_user(database: &Database) -> Result<(Uuid, User)> {
    // Create a proper bcrypt hash for the default test password "password123"
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)?;

    let mut user = User::new(
        "test@example.com".to_owned(),
        password_hash,
        Some("Test User".to_owned()),
    );

    // Activate the user for testing (bypass admin approval)
    user.user_status = UserStatus::Active;
    user.approved_by = Some(user.id); // Self-approved for testing
    user.approved_at = Some(chrono::Utc::now());

    let user_id = user.id;
    database.create_user(&user).await?;

    // Create the tenant with this user as owner
    // The create_tenant function automatically adds the owner to tenant_users
    let tenant_id = TenantId::new();
    let tenant = Tenant {
        id: tenant_id,
        name: "Test Tenant".to_owned(),
        slug: format!("test-tenant-{}", tenant_id),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await?;

    // Also update legacy tenant_id column for backward compatibility
    database
        .update_user_tenant_id(user_id, &tenant_id.to_string())
        .await?;

    Ok((user_id, user))
}

/// Create a test user with custom email
pub async fn create_test_user_with_email(database: &Database, email: &str) -> Result<(Uuid, User)> {
    // Create a proper bcrypt hash for the default test password "password123"
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST)?;

    let mut user = User::new(
        email.to_owned(),
        password_hash,
        Some("Test User".to_owned()),
    );

    // Activate the user for testing (bypass admin approval)
    user.user_status = UserStatus::Active;
    user.approved_by = Some(user.id); // Self-approved for testing
    user.approved_at = Some(chrono::Utc::now());

    let user_id = user.id;
    database.create_user(&user).await?;

    // Create the tenant with this user as owner
    // The create_tenant function automatically adds the owner to tenant_users
    let tenant_id = TenantId::new();
    let tenant = Tenant {
        id: tenant_id,
        name: format!("Test Tenant for {}", email),
        slug: format!("test-tenant-{}", tenant_id),
        domain: None,
        plan: "starter".to_owned(),
        owner_user_id: user_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    database.create_tenant(&tenant).await?;

    // Also update legacy tenant_id column for backward compatibility
    database
        .update_user_tenant_id(user_id, &tenant_id.to_string())
        .await?;

    Ok((user_id, user))
}

/// Create a test API key for a user (returns API key string)
pub fn create_test_api_key(_database: &Database, user_id: Uuid, name: &str) -> Result<String> {
    let request = CreateApiKeyRequest {
        name: name.to_owned(),
        description: Some("Test API key".to_owned()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: Some(1000),
        expires_in_days: None,
    };

    let manager = ApiKeyManager::new();
    let (_, api_key_string) = manager.create_api_key(user_id, request)?;
    Ok(api_key_string)
}

/// Create a test API key and store it in the database (returns `ApiKey` object)
pub async fn create_and_store_test_api_key(
    database: &Database,
    user_id: Uuid,
    name: &str,
) -> Result<ApiKey> {
    let request = CreateApiKeyRequest {
        name: name.to_owned(),
        description: Some("Test API key".to_owned()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: Some(1000),
        expires_in_days: None,
    };

    let manager = ApiKeyManager::new();
    let (api_key, _) = manager.create_api_key(user_id, request)?;
    database.create_api_key(&api_key).await?;
    Ok(api_key)
}

/// Complete test environment setup
/// Returns (database, `auth_manager`, `auth_middleware`, `user_id`, `api_key`)
pub async fn setup_test_environment() -> Result<(
    Arc<Database>,
    Arc<AuthManager>,
    Arc<McpAuthMiddleware>,
    Uuid,
    String,
)> {
    let database = create_test_database().await?;
    let auth_manager = create_test_auth_manager();
    let auth_middleware = create_test_auth_middleware(&auth_manager, database.clone());

    let (user_id, _user) = create_test_user(&database).await?;
    let api_key = create_test_api_key(&database, user_id, "test-key")?;

    Ok((database, auth_manager, auth_middleware, user_id, api_key))
}

/// Lightweight test environment for simple tests
/// Returns (database, `user_id`)
pub async fn setup_simple_test_environment() -> Result<(Arc<Database>, Uuid)> {
    let database = create_test_database().await?;
    let (user_id, _user) = create_test_user(&database).await?;
    Ok((database, user_id))
}

/// Test environment with custom user tier
pub async fn setup_test_environment_with_tier(tier: UserTier) -> Result<(Arc<Database>, Uuid)> {
    let database = create_test_database().await?;
    let mut user = User::new(
        "test@example.com".to_owned(),
        "test_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    user.tier = tier;
    let user_id = user.id;

    database.create_user(&user).await?;
    Ok((database, user_id))
}

/// Create test `ServerResources` with all components properly initialized
/// This replaces individual resource creation for proper architectural patterns
pub async fn create_test_server_resources() -> Result<Arc<ServerResources>> {
    init_test_logging();
    init_test_http_clients();
    init_server_config();
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();

    #[cfg(feature = "postgresql")]
    let database =
        Database::new(database_url, encryption_key, &PostgresPoolConfig::default()).await?;

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(database_url, encryption_key).await?;

    let auth_manager = AuthManager::new(24);

    let admin_jwt_secret = "test_admin_secret";
    let config = Arc::new(ServerConfig {
        usda_api_key: env::var("USDA_API_KEY").ok(),
        ..ServerConfig::default()
    });

    // Create test cache with background cleanup disabled for tests
    let cache_config = CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: StdDuration::from_secs(60),
        enable_background_cleanup: false, // Disable background cleanup for tests
        ..Default::default()
    };
    let cache = Cache::new(cache_config).await?;

    // Use shared JWKS manager to eliminate expensive RSA key generation (250-350ms per test)
    let jwks_manager = get_shared_test_jwks();

    Ok(Arc::new(
        ServerResources::new(
            database,
            auth_manager,
            admin_jwt_secret,
            config,
            cache,
            ServerResourcesOptions {
                rsa_key_size_bits: Some(2048),
                jwks_manager: Some(jwks_manager),
                llm_provider: None, // Use ChatProvider::from_env() by default, override for tests needing mock LLM
            },
        )
        .await,
    ))
}

/// Complete test environment setup using `ServerResources` pattern
/// Returns (`server_resources`, `user_id`, `api_key`)
pub async fn setup_server_resources_test_environment(
) -> Result<(Arc<ServerResources>, Uuid, String)> {
    let resources = create_test_server_resources().await?;
    let (user_id, _user) = create_test_user(&resources.database).await?;
    let api_key = create_test_api_key(&resources.database, user_id, "test-key")?;

    Ok((resources, user_id, api_key))
}

// ✅ IMPORTANT: Test Database Cleanup Best Practices
//
// The accumulated test database files (459 files, 188MB) have been cleaned up.
// Moving forward, follow these patterns:
//
// 1. **CI Environment**: Tests should use `sqlite::memory:` (no files created)
// 2. **Local Environment**: Use unique test database names with cleanup
// 3. **Automatic Cleanup**: Run `./scripts/testing/clean-test-databases.sh` before/after tests
//
// Example of GOOD test database pattern:
// ```rust
// let database_url = if std::env::var("CI").is_ok() {
//     "sqlite::memory:".to_owned()  // ✅ No files in CI
// } else {
//     let test_id = Uuid::new_v4();
//     let db_path = format!("./test_data/my_test_{}.db", test_id);
//     let _ = std::fs::remove_file(&db_path);  // ✅ Cleanup before
//     format!("sqlite:{}", db_path)
// };
// ```
//
// The lint-and-test.sh script now includes automatic database cleanup.

// ============================================================================
// USDA Mock Client for Testing (No Real API Calls)
// ============================================================================

use async_trait::async_trait;
use futures_util::stream;
use pierre_mcp_server::errors::AppError;
use pierre_mcp_server::external::{FoodDetails, FoodNutrient, FoodSearchResult};
use pierre_mcp_server::llm::{
    ChatRequest, ChatResponse, ChatStream, LlmCapabilities, LlmProvider, StreamChunk,
};
use std::collections::HashMap;

/// Mock USDA client for testing (no API calls)
pub struct MockUsdaClient {
    mock_foods: HashMap<u64, FoodDetails>,
}

impl MockUsdaClient {
    /// Create a new mock client with predefined test data
    #[must_use]
    pub fn new() -> Self {
        let mut mock_foods = HashMap::new();

        // Mock food: Chicken breast (FDC ID: 171_477)
        mock_foods.insert(
            171_477,
            FoodDetails {
                fdc_id: 171_477,
                description: "Chicken, breast, meat only, cooked, roasted".to_owned(),
                data_type: "SR Legacy".to_owned(),
                food_nutrients: vec![
                    FoodNutrient {
                        nutrient_id: 1003,
                        nutrient_name: "Protein".to_owned(),
                        unit_name: "g".to_owned(),
                        amount: 31.02,
                    },
                    FoodNutrient {
                        nutrient_id: 1004,
                        nutrient_name: "Total lipid (fat)".to_owned(),
                        unit_name: "g".to_owned(),
                        amount: 3.57,
                    },
                    FoodNutrient {
                        nutrient_id: 1005,
                        nutrient_name: "Carbohydrate, by difference".to_owned(),
                        unit_name: "g".to_owned(),
                        amount: 0.0,
                    },
                    FoodNutrient {
                        nutrient_id: 1008,
                        nutrient_name: "Energy".to_owned(),
                        unit_name: "kcal".to_owned(),
                        amount: 165.0,
                    },
                ],
                serving_size: Some(100.0),
                serving_size_unit: Some("g".to_owned()),
            },
        );

        // Mock food: Apple (FDC ID: 171_688)
        mock_foods.insert(
            171_688,
            FoodDetails {
                fdc_id: 171_688,
                description: "Apples, raw, with skin".to_owned(),
                data_type: "SR Legacy".to_owned(),
                food_nutrients: vec![
                    FoodNutrient {
                        nutrient_id: 1003,
                        nutrient_name: "Protein".to_owned(),
                        unit_name: "g".to_owned(),
                        amount: 0.26,
                    },
                    FoodNutrient {
                        nutrient_id: 1004,
                        nutrient_name: "Total lipid (fat)".to_owned(),
                        unit_name: "g".to_owned(),
                        amount: 0.17,
                    },
                    FoodNutrient {
                        nutrient_id: 1005,
                        nutrient_name: "Carbohydrate, by difference".to_owned(),
                        unit_name: "g".to_owned(),
                        amount: 13.81,
                    },
                    FoodNutrient {
                        nutrient_id: 1008,
                        nutrient_name: "Energy".to_owned(),
                        unit_name: "kcal".to_owned(),
                        amount: 52.0,
                    },
                ],
                serving_size: Some(182.0),
                serving_size_unit: Some("g".to_owned()),
            },
        );

        Self { mock_foods }
    }

    /// Mock search implementation
    ///
    /// # Errors
    /// Returns `AppError::InvalidInput` if query is empty
    pub fn search_foods(
        &self,
        query: &str,
        _page_size: u32,
    ) -> Result<Vec<FoodSearchResult>, AppError> {
        if query.is_empty() {
            return Err(AppError::invalid_input("Search query cannot be empty"));
        }

        let query_lower = query.to_lowercase();
        let results: Vec<FoodSearchResult> = self
            .mock_foods
            .values()
            .filter(|food| food.description.to_lowercase().contains(&query_lower))
            .map(|food| FoodSearchResult {
                fdc_id: food.fdc_id,
                description: food.description.clone(),
                data_type: food.data_type.clone(),
                publication_date: None,
                brand_owner: None,
            })
            .collect();

        Ok(results)
    }

    /// Mock details implementation
    ///
    /// # Errors
    /// Returns `AppError::NotFound` if food with given FDC ID doesn't exist
    pub fn get_food_details(&self, fdc_id: u64) -> Result<FoodDetails, AppError> {
        self.mock_foods
            .get(&fdc_id)
            .cloned()
            .ok_or_else(|| AppError::not_found(format!("Food with FDC ID {fdc_id}")))
    }
}

impl Default for MockUsdaClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// SDK Bridge Helpers for Multi-Tenant E2E Testing
// ============================================================================

use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};

/// Handle for SDK bridge process that cleans up automatically on drop
/// Ensures subprocess is terminated when test completes
pub struct SdkBridgeHandle {
    process: Child,
    port: u16,
}

impl SdkBridgeHandle {
    /// Get the server port this bridge is connected to
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Get mutable reference to stdin for sending requests
    #[allow(clippy::missing_const_for_fn)] // Cannot be const - returns &mut
    pub fn stdin(&mut self) -> Option<&mut ChildStdin> {
        self.process.stdin.as_mut()
    }

    /// Get mutable reference to stdout for reading responses
    #[allow(clippy::missing_const_for_fn)] // Cannot be const - returns &mut
    pub fn stdout(&mut self) -> Option<&mut ChildStdout> {
        self.process.stdout.as_mut()
    }

    /// Get mutable reference to stderr for reading errors
    #[allow(clippy::missing_const_for_fn)] // Cannot be const - returns &mut
    pub fn stderr(&mut self) -> Option<&mut ChildStderr> {
        self.process.stderr.as_mut()
    }
}

impl Drop for SdkBridgeHandle {
    fn drop(&mut self) {
        // Kill the SDK bridge process when handle is dropped
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

/// Spawn SDK bridge process for testing
/// Returns RAII handle that automatically cleans up subprocess on drop
///
/// # Arguments
/// * `jwt_token` - Valid JWT token for authentication
/// * `server_port` - Port where Pierre server is running
///
/// # Errors
/// Returns error if SDK bridge binary not found or process fails to start
pub async fn spawn_sdk_bridge(jwt_token: &str, server_port: u16) -> Result<SdkBridgeHandle> {
    // Find SDK CLI entry point (dist/cli.js - built from TypeScript)
    let sdk_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("sdk")
        .join("dist")
        .join("cli.js");

    if !sdk_path.exists() {
        return Err(anyhow::Error::msg(format!(
            "SDK entry point not found at: {}",
            sdk_path.display()
        )));
    }

    // Spawn Node.js process running SDK bridge in stdio mode
    // Increase connection timeouts for CI runners where startup can be slow
    let mut process = Command::new("node")
        .arg(sdk_path)
        .env(
            "PIERRE_SERVER_URL",
            format!("http://localhost:{}", server_port),
        )
        .env("PIERRE_JWT_TOKEN", jwt_token)
        .env("MCP_TRANSPORT", "stdio")
        .env("PIERRE_PROACTIVE_CONNECTION_TIMEOUT_MS", "10000")
        .env("PIERRE_PROACTIVE_TOOLS_LIST_TIMEOUT_MS", "10000")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Wait for process to initialize (longer on CI where startup can be slow)
    sleep(Duration::from_millis(2000)).await;

    // Check if process is still alive
    if let Ok(Some(status)) = process.try_wait() {
        // Capture stderr to understand why the process failed
        use std::io::Read;
        let stderr_output = process.stderr.take().map_or_else(
            || String::from("(stderr not available)"),
            |mut stderr| {
                let mut output = String::new();
                stderr.read_to_string(&mut output).unwrap_or_default();
                output
            },
        );
        return Err(anyhow::Error::msg(format!(
            "SDK bridge process exited immediately with status: {}\nStderr: {}",
            status, stderr_output
        )));
    }

    Ok(SdkBridgeHandle {
        process,
        port: server_port,
    })
}

/// Send MCP request via SDK stdio bridge
/// Writes JSON-RPC request to stdin and reads response from stdout
///
/// # Arguments
/// * `sdk_bridge` - Mutable reference to SDK bridge handle
/// * `method` - MCP method name (e.g., "tools/list", "tools/call")
/// * `params` - JSON parameters for the method
///
/// # Errors
/// Returns error if stdio communication fails or server returns error
pub fn send_sdk_stdio_request(
    sdk_bridge: &mut SdkBridgeHandle,
    method: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value> {
    use std::io::{BufRead, BufReader, Write};

    // Build JSON-RPC 2.0 request
    let request_id = uuid::Uuid::new_v4().to_string();
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "method": method,
        "params": params
    });

    // Get stdin handle
    let stdin = sdk_bridge
        .stdin()
        .ok_or_else(|| anyhow::Error::msg("SDK bridge stdin not available"))?;

    // Write request to stdin (MCP protocol expects newline-delimited JSON)
    let request_str = serde_json::to_string(&request)?;
    writeln!(stdin, "{}", request_str)?;
    stdin.flush()?;

    // Get stdout handle
    let stdout = sdk_bridge
        .stdout()
        .ok_or_else(|| anyhow::Error::msg("SDK bridge stdout not available"))?;

    // Read response from stdout
    let mut reader = BufReader::new(stdout);
    let mut response_line = String::new();
    reader.read_line(&mut response_line)?;

    // Parse JSON-RPC response
    let response: serde_json::Value = serde_json::from_str(response_line.trim())?;

    // Validate JSON-RPC 2.0 response structure
    if response.get("jsonrpc") != Some(&serde_json::json!("2.0")) {
        return Err(anyhow::Error::msg(
            "Invalid JSON-RPC response (missing jsonrpc field)",
        ));
    }

    if response.get("id").and_then(|v| v.as_str()) != Some(&request_id) {
        return Err(anyhow::Error::msg("Response ID mismatch"));
    }

    // Return the result or error
    if let Some(error) = response.get("error") {
        return Err(anyhow::Error::msg(format!("SDK returned error: {error}")));
    }

    response
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow::Error::msg("Response missing result field"))
}

/// Send HTTP MCP request directly to server
/// Bypasses SDK to test HTTP transport directly
///
/// # Arguments
/// * `url` - Full URL to MCP endpoint (e.g., `http://localhost:8081/mcp`)
/// * `method` - MCP method name (e.g., "tools/list", "tools/call")
/// * `params` - JSON parameters for the method
/// * `jwt_token` - Valid JWT token for authentication
///
/// # Errors
/// Returns error if HTTP request fails or server returns error
pub async fn send_http_mcp_request(
    url: &str,
    method: &str,
    params: serde_json::Value,
    jwt_token: &str,
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", jwt_token))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::Error::msg(format!(
            "HTTP request failed with status: {}",
            response.status()
        )));
    }

    let response_json: serde_json::Value = response.json().await?;

    // Check for JSON-RPC error
    if let Some(error) = response_json.get("error") {
        return Err(anyhow::Error::msg(format!("MCP error: {}", error)));
    }

    // Extract result field
    response_json
        .get("result")
        .cloned()
        .ok_or_else(|| anyhow::Error::msg("Response missing 'result' field".to_owned()))
}

/// Create test tenant with user and JWT token
/// Combines user creation and token generation for multi-tenant tests
///
/// # Arguments
/// * `resources` - Server resources containing database and auth
/// * `email` - Email address for the test user
///
/// # Errors
/// Returns error if user creation or token generation fails
pub async fn create_test_tenant(
    resources: &ServerResources,
    email: &str,
) -> Result<(User, String)> {
    // Create test user with specified email
    let (_user_id, user) = create_test_user_with_email(&resources.database, email).await?;

    // Generate JWT token for this user
    let token = resources
        .auth_manager
        .generate_token(&user, &resources.jwks_manager)
        .map_err(|e| anyhow::Error::msg(format!("Failed to generate JWT: {}", e)))?;

    Ok((user, token))
}

/// Handle for HTTP MCP server that cleans up automatically on drop
pub struct HttpServerHandle {
    task_handle: JoinHandle<()>,
    port: u16,
}

impl HttpServerHandle {
    /// Get the port the server is listening on
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Get the base URL for making HTTP requests to this server
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

impl Drop for HttpServerHandle {
    fn drop(&mut self) {
        // Abort the server task when handle is dropped (RAII cleanup)
        self.task_handle.abort();
    }
}

/// Check if a TCP port is available for binding
fn is_port_available(port: u16) -> bool {
    TcpListener::bind(format!("127.0.0.1:{port}")).is_ok()
}

/// Find an available port for testing
fn find_available_port() -> u16 {
    let mut rng = rand::thread_rng();
    for _ in 0..100 {
        let port = rng.gen_range(10000..60000);
        if is_port_available(port) {
            return port;
        }
    }
    panic!("Could not find an available port after 100 attempts");
}

/// Spawn HTTP MCP server for E2E testing
///
/// Creates an Axum server with MCP routes listening on a random available port.
/// The server runs in the background and is automatically cleaned up when the
/// returned handle is dropped (RAII pattern).
///
/// # Arguments
/// * `resources` - Arc-wrapped server resources with database, auth, and configuration
///
/// # Returns
/// Handle to the running server with port and base URL
///
/// # Errors
/// Returns error if server cannot be started
pub async fn spawn_http_mcp_server(resources: &Arc<ServerResources>) -> Result<HttpServerHandle> {
    let port = find_available_port();

    // Clone Arc for moving into spawned task (Arc enables sharing across tasks)
    let resources_for_task = Arc::clone(resources);

    // Spawn server task
    let task_handle = tokio::spawn(async move {
        let app = McpRoutes::routes(resources_for_task);

        let listener = TokioTcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind to port");

        axum::serve(listener, app)
            .await
            .expect("Server failed to run");
    });

    // Wait for server to be ready
    tokio_sleep(StdDuration::from_millis(500)).await;

    Ok(HttpServerHandle { task_handle, port })
}

// ============================================================================
// PostgreSQL Test Database Isolation
// ============================================================================
// Each PostgreSQL test gets its own unique database to prevent concurrent
// schema creation conflicts when running with --test-threads=4.
// See issue #36 for context on the race condition this solves.

/// Handle for an isolated `PostgreSQL` test database with RAII cleanup
/// Automatically drops the database when the handle goes out of scope
#[cfg(feature = "postgresql")]
pub struct IsolatedPostgresDb {
    /// Connection URL for this isolated test database
    pub url: String,
    /// Database name (used for cleanup)
    pub db_name: String,
    /// Base URL for connecting to postgres admin database
    admin_url: String,
}

#[cfg(feature = "postgresql")]
impl IsolatedPostgresDb {
    /// Create a new isolated `PostgreSQL` database for testing
    ///
    /// Creates a unique database with UUID suffix to prevent conflicts
    /// between concurrent tests.
    ///
    /// # Errors
    /// Returns error if database creation fails
    pub async fn new() -> Result<Self> {
        let base_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgresql://pierre:ci_test_password@localhost:5432/pierre_mcp_server".to_owned()
        });

        // Generate unique database name using UUID
        let db_name = format!("pierre_test_{}", Uuid::new_v4().as_simple());

        // Connect to postgres admin database to create test database
        let admin_url = base_url
            .replace("/pierre_mcp_server", "/postgres")
            .replace("/pierre_test_", "/postgres"); // Handle nested test URLs

        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(StdDuration::from_secs(10))
            .connect(&admin_url)
            .await?;

        // Create the isolated test database
        sqlx::query(&format!("CREATE DATABASE {db_name}"))
            .execute(&pool)
            .await?;

        // Build test database URL
        let test_url = base_url.replace("/pierre_mcp_server", &format!("/{db_name}"));

        Ok(Self {
            url: test_url,
            db_name,
            admin_url,
        })
    }

    /// Get a Database instance connected to this isolated database
    ///
    /// # Errors
    /// Returns error if connection or migration fails
    pub async fn get_database(&self) -> Result<Database> {
        let encryption_key = generate_encryption_key().to_vec();
        let pool_config = PostgresPoolConfig {
            max_connections: 5,
            min_connections: 1,
            acquire_timeout_secs: 30,
            ..Default::default()
        };

        let db = Database::new(&self.url, encryption_key, &pool_config).await?;
        db.migrate().await?;

        Ok(db)
    }

    /// Cleanup the isolated database
    /// Called automatically on Drop, but can be called explicitly
    async fn cleanup(&self) -> Result<()> {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(StdDuration::from_secs(5))
            .connect(&self.admin_url)
            .await?;

        // Terminate all connections to the test database first
        let terminate_query = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
            self.db_name
        );
        let _ = sqlx::query(&terminate_query).execute(&pool).await;

        // Drop the database
        let drop_query = format!("DROP DATABASE IF EXISTS {}", self.db_name);
        let _ = sqlx::query(&drop_query).execute(&pool).await;

        Ok(())
    }
}

#[cfg(feature = "postgresql")]
impl Drop for IsolatedPostgresDb {
    fn drop(&mut self) {
        // Spawn a blocking task to cleanup the database
        // We can't use async in Drop, so we spawn a new runtime
        let admin_url = self.admin_url.clone();
        let db_name = self.db_name.clone();

        thread::spawn(move || {
            let rt = RuntimeBuilder::new_current_thread()
                .enable_all()
                .build()
                .expect("Failed to create runtime for cleanup");

            rt.block_on(async {
                if let Ok(pool) = PgPoolOptions::new()
                    .max_connections(1)
                    .acquire_timeout(StdDuration::from_secs(5))
                    .connect(&admin_url)
                    .await
                {
                    // Terminate connections
                    let terminate_query = format!(
                        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{}'",
                        db_name
                    );
                    let _ = sqlx::query(&terminate_query).execute(&pool).await;

                    // Drop database
                    let drop_query = format!("DROP DATABASE IF EXISTS {}", db_name);
                    let _ = sqlx::query(&drop_query).execute(&pool).await;
                }
            });
        });
    }
}

/// Create an isolated `PostgreSQL` database for a single test
/// Returns the database URL and database name for cleanup
///
/// # Errors
/// Returns error if database creation fails
#[cfg(feature = "postgresql")]
pub async fn create_isolated_postgres_db() -> Result<(String, String)> {
    let isolated_db = IsolatedPostgresDb::new().await?;
    Ok((isolated_db.url.clone(), isolated_db.db_name.clone()))
}

/// Clean up an isolated `PostgreSQL` test database
///
/// # Errors
/// Returns error if cleanup fails
#[cfg(feature = "postgresql")]
pub async fn cleanup_postgres_db(db_name: &str) -> Result<()> {
    let base_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgresql://pierre:ci_test_password@localhost:5432/pierre_mcp_server".to_owned()
    });

    let admin_url = base_url.replace("/pierre_mcp_server", "/postgres");

    let pool = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(StdDuration::from_secs(5))
        .connect(&admin_url)
        .await?;

    // Terminate all connections to the test database
    let terminate_query = format!(
        "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '{db_name}'"
    );
    let _ = sqlx::query(&terminate_query).execute(&pool).await;

    // Drop the database
    let drop_query = format!("DROP DATABASE IF EXISTS {db_name}");
    sqlx::query(&drop_query).execute(&pool).await?;

    Ok(())
}

// ============================================================================
// Test LLM Provider for Mocking
// ============================================================================

/// Mock LLM provider for testing insight validation logic
///
/// Allows tests to control LLM responses without making actual API calls.
/// Configured with predetermined responses for validation testing.
pub struct TestLlmProvider {
    /// Pre-configured JSON response that will be returned by `complete()`
    response: String,
    /// Model name to return (stored to satisfy lifetime requirements)
    model_name: String,
}

impl TestLlmProvider {
    /// Create a provider that returns a "valid" verdict
    #[must_use]
    pub fn valid() -> Self {
        Self {
            response: r#"{"verdict": "valid", "reason": "Content meets quality standards"}"#
                .to_owned(),
            model_name: "test-model-v1".to_owned(),
        }
    }

    /// Create a provider that returns a "rejected" verdict with custom reason
    #[must_use]
    pub fn rejected(reason: &str) -> Self {
        Self {
            response: format!(r#"{{"verdict": "rejected", "reason": "{reason}"}}"#),
            model_name: "test-model-v1".to_owned(),
        }
    }

    /// Create a provider that returns an "improved" verdict with enhanced content
    #[must_use]
    pub fn improved(improved_content: &str, reason: &str) -> Self {
        Self {
            response: format!(
                r#"{{"verdict": "improved", "reason": "{reason}", "improved_content": "{improved_content}"}}"#
            ),
            model_name: "test-model-v1".to_owned(),
        }
    }

    /// Create a provider with a custom JSON response
    #[must_use]
    pub fn with_response(response: String) -> Self {
        Self {
            response,
            model_name: "test-model-v1".to_owned(),
        }
    }
}

#[async_trait]
impl LlmProvider for TestLlmProvider {
    fn name(&self) -> &'static str {
        "test"
    }

    fn display_name(&self) -> &'static str {
        "Test Provider"
    }

    fn capabilities(&self) -> LlmCapabilities {
        LlmCapabilities::SYSTEM_MESSAGES | LlmCapabilities::JSON_MODE
    }

    fn default_model(&self) -> &str {
        &self.model_name
    }

    fn available_models(&self) -> &'static [&'static str] {
        &["test-model-v1"]
    }

    async fn complete(&self, _request: &ChatRequest) -> Result<ChatResponse, AppError> {
        Ok(ChatResponse {
            content: self.response.clone(),
            model: self.model_name.clone(),
            usage: None,
            finish_reason: Some("stop".to_owned()),
        })
    }

    async fn complete_stream(&self, request: &ChatRequest) -> Result<ChatStream, AppError> {
        // For tests, just delegate to non-streaming complete
        // Tests typically don't need streaming functionality
        let response = self.complete(request).await?;

        // Create a single-item stream
        let chunk = StreamChunk {
            delta: response.content,
            is_final: true,
            finish_reason: response.finish_reason,
        };

        let result_stream = stream::once(async move { Ok(chunk) });
        Ok(Box::pin(result_stream))
    }

    async fn health_check(&self) -> Result<bool, AppError> {
        Ok(true)
    }
}
