// ABOUTME: Shared test utilities and setup functions for integration tests
// ABOUTME: Provides common database, auth, and user creation helpers
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
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
use pierre_mcp_server::{
    admin::jwks::JwksManager,
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::AuthManager,
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    mcp::resources::ServerResources,
    middleware::McpAuthMiddleware,
    models::{User, UserTier},
};
use std::sync::{Arc, LazyLock, Once};
use uuid::Uuid;

static INIT_LOGGER: Once = Once::new();
static INIT_HTTP_CLIENTS: Once = Once::new();
static INIT_SERVER_CONFIG: Once = Once::new();

/// Initialize server configuration for tests (call once per test process)
pub fn init_server_config() {
    INIT_SERVER_CONFIG.call_once(|| {
        std::env::set_var("CI", "true");
        std::env::set_var("DATABASE_URL", "sqlite::memory:");
        let _ = pierre_mcp_server::constants::init_server_config();
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
        let log_level = match std::env::var("TEST_LOG").as_deref() {
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
        pierre_mcp_server::utils::http_client::initialize_http_clients(
            pierre_mcp_server::config::environment::HttpClientConfig::default(),
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
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await?,
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
        Database::new(
            database_url,
            encryption_key,
            &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
        )
        .await?,
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
        pierre_mcp_server::config::environment::RateLimitConfig::default(),
    ))
}

/// Create test cache with background cleanup disabled
pub async fn create_test_cache() -> Result<pierre_mcp_server::cache::factory::Cache> {
    let cache_config = pierre_mcp_server::cache::CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: std::time::Duration::from_secs(60),
        enable_background_cleanup: false, // Disable background cleanup for tests
    };
    pierre_mcp_server::cache::factory::Cache::new(cache_config).await
}

/// Create a standard test user
pub async fn create_test_user(database: &Database) -> Result<(Uuid, User)> {
    let user = User::new(
        "test@example.com".to_owned(),
        "test_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    let user_id = user.id;

    database.create_user(&user).await?;
    Ok((user_id, user))
}

/// Create a test user with custom email
pub async fn create_test_user_with_email(database: &Database, email: &str) -> Result<(Uuid, User)> {
    let user = User::new(
        email.to_owned(),
        "test_hash".to_owned(),
        Some("Test User".to_owned()),
    );
    let user_id = user.id;

    database.create_user(&user).await?;
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
    let database = Database::new(
        database_url,
        encryption_key,
        &pierre_mcp_server::config::environment::PostgresPoolConfig::default(),
    )
    .await?;

    #[cfg(not(feature = "postgresql"))]
    let database = Database::new(database_url, encryption_key).await?;

    let auth_manager = AuthManager::new(24);

    let admin_jwt_secret = "test_admin_secret";
    let config = Arc::new(pierre_mcp_server::config::environment::ServerConfig::default());

    // Create test cache with background cleanup disabled for tests
    let cache_config = pierre_mcp_server::cache::CacheConfig {
        max_entries: 1000,
        redis_url: None,
        cleanup_interval: std::time::Duration::from_secs(60),
        enable_background_cleanup: false, // Disable background cleanup for tests
    };
    let cache = pierre_mcp_server::cache::factory::Cache::new(cache_config).await?;

    // Use shared JWKS manager to eliminate expensive RSA key generation (250-350ms per test)
    let jwks_manager = get_shared_test_jwks();

    Ok(Arc::new(ServerResources::new(
        database,
        auth_manager,
        admin_jwt_secret,
        config,
        cache,
        2048, // Use 2048-bit RSA keys for faster test execution (if new keys needed)
        Some(jwks_manager),
    )))
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
// 3. **Automatic Cleanup**: Run `./scripts/clean-test-databases.sh` before/after tests
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

use pierre_mcp_server::errors::AppError;
use pierre_mcp_server::external::{FoodDetails, FoodNutrient, FoodSearchResult};
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
