//! Shared test utilities for pierre_mcp_server
//!
//! This module provides common test setup functions to reduce duplication
//! across integration tests.

#![allow(dead_code)]

use anyhow::Result;
use pierre_mcp_server::{
    api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest},
    auth::{AuthManager, McpAuthMiddleware},
    database::generate_encryption_key,
    database_plugins::{factory::Database, DatabaseProvider},
    models::{User, UserTier},
};
use std::sync::{Arc, Once};
use uuid::Uuid;

static INIT_LOGGER: Once = Once::new();

/// Initialize quiet logging for tests (call once per test process)
pub fn init_test_logging() {
    INIT_LOGGER.call_once(|| {
        // Check for TEST_LOG environment variable to control test logging level
        let log_level = match std::env::var("TEST_LOG").as_deref() {
            Ok("TRACE") => tracing::Level::TRACE,
            Ok("DEBUG") => tracing::Level::DEBUG,
            Ok("INFO") => tracing::Level::INFO,
            Ok("WARN") => tracing::Level::WARN,
            Ok("ERROR") => tracing::Level::ERROR,
            _ => tracing::Level::WARN, // Default to WARN for quiet tests
        };

        tracing_subscriber::fmt()
            .with_max_level(log_level)
            .with_test_writer()
            .init();
    });
}

/// Standard test database setup
pub async fn create_test_database() -> Result<Arc<Database>> {
    init_test_logging();
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Arc::new(Database::new(database_url, encryption_key).await?);
    Ok(database)
}

/// Standard test database setup with custom encryption key
pub async fn create_test_database_with_key(encryption_key: Vec<u8>) -> Result<Arc<Database>> {
    init_test_logging();
    let database_url = "sqlite::memory:";
    let database = Arc::new(Database::new(database_url, encryption_key).await?);
    Ok(database)
}

/// Create test authentication manager
pub fn create_test_auth_manager() -> Arc<AuthManager> {
    let jwt_secret = pierre_mcp_server::auth::generate_jwt_secret().to_vec();
    Arc::new(AuthManager::new(jwt_secret, 24))
}

/// Create test authentication middleware
pub fn create_test_auth_middleware(
    auth_manager: Arc<AuthManager>,
    database: Arc<Database>,
) -> Arc<McpAuthMiddleware> {
    Arc::new(McpAuthMiddleware::new((*auth_manager).clone(), database))
}

/// Create a standard test user
pub async fn create_test_user(database: &Database) -> Result<(Uuid, User)> {
    let user = User::new(
        "test@example.com".to_string(),
        "test_hash".to_string(),
        Some("Test User".to_string()),
    );
    let user_id = user.id;

    database.create_user(&user).await?;
    Ok((user_id, user))
}

/// Create a test user with custom email
pub async fn create_test_user_with_email(database: &Database, email: &str) -> Result<(Uuid, User)> {
    let user = User::new(
        email.to_string(),
        "test_hash".to_string(),
        Some("Test User".to_string()),
    );
    let user_id = user.id;

    database.create_user(&user).await?;
    Ok((user_id, user))
}

/// Create a test API key for a user (returns API key string)
pub async fn create_test_api_key(
    _database: &Database,
    user_id: Uuid,
    name: &str,
) -> Result<String> {
    let request = CreateApiKeyRequest {
        name: name.to_string(),
        description: Some("Test API key".to_string()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: Some(1000),
        expires_in_days: None,
    };

    let manager = ApiKeyManager::new();
    let (_, api_key_string) = manager.create_api_key(user_id, request).await?;
    Ok(api_key_string)
}

/// Create a test API key and store it in the database (returns ApiKey object)
pub async fn create_and_store_test_api_key(
    database: &Database,
    user_id: Uuid,
    name: &str,
) -> Result<ApiKey> {
    let request = CreateApiKeyRequest {
        name: name.to_string(),
        description: Some("Test API key".to_string()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: Some(1000),
        expires_in_days: None,
    };

    let manager = ApiKeyManager::new();
    let (api_key, _) = manager.create_api_key(user_id, request).await?;
    database.create_api_key(&api_key).await?;
    Ok(api_key)
}

/// Complete test environment setup
/// Returns (database, auth_manager, auth_middleware, user_id, api_key)
pub async fn setup_test_environment() -> Result<(
    Arc<Database>,
    Arc<AuthManager>,
    Arc<McpAuthMiddleware>,
    Uuid,
    String,
)> {
    let database = create_test_database().await?;
    let auth_manager = create_test_auth_manager();
    let auth_middleware = create_test_auth_middleware(auth_manager.clone(), database.clone());

    let (user_id, _user) = create_test_user(&database).await?;
    let api_key = create_test_api_key(&database, user_id, "test-key").await?;

    Ok((database, auth_manager, auth_middleware, user_id, api_key))
}

/// Lightweight test environment for simple tests
/// Returns (database, user_id)
pub async fn setup_simple_test_environment() -> Result<(Arc<Database>, Uuid)> {
    let database = create_test_database().await?;
    let (user_id, _user) = create_test_user(&database).await?;
    Ok((database, user_id))
}

/// Test environment with custom user tier
pub async fn setup_test_environment_with_tier(tier: UserTier) -> Result<(Arc<Database>, Uuid)> {
    let database = create_test_database().await?;
    let mut user = User::new(
        "test@example.com".to_string(),
        "test_hash".to_string(),
        Some("Test User".to_string()),
    );
    user.tier = tier;
    let user_id = user.id;

    database.create_user(&user).await?;
    Ok((database, user_id))
}
