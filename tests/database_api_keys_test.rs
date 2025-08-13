// ABOUTME: Integration tests for database API key operations
// ABOUTME: Tests creation, retrieval, usage tracking, and expiration cleanup

use chrono::{DateTime, Duration, Utc};
use pierre_mcp_server::api_keys::{
    ApiKey, ApiKeyManager, ApiKeyTier, ApiKeyUsage, CreateApiKeyRequest,
};
use pierre_mcp_server::database::Database;
use pierre_mcp_server::models::{User, UserTier};
use uuid::Uuid;

async fn create_test_user(db: &Database) -> User {
    let user = User {
        id: Uuid::new_v4(),
        email: {
            let uuid = Uuid::new_v4();
            format!("test_{uuid}@example.com")
        },
        display_name: Some("Test User".into()),
        password_hash: "hashed".into(),
        tier: UserTier::Professional,
        tenant_id: Some("test-tenant".to_string()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
    };

    db.create_user(&user).await.expect("Failed to create user");
    user
}

#[tokio::test]
async fn test_create_and_retrieve_api_key() {
    let db = pierre_mcp_server::database::tests::create_test_db()
        .await
        .expect("Failed to create test database");

    let user = create_test_user(&db).await;

    // Create API key
    let manager = ApiKeyManager::new();
    let request = CreateApiKeyRequest {
        name: "Test Key".into(),
        description: Some("Test API key".into()),
        tier: ApiKeyTier::Professional,
        rate_limit_requests: Some(1000),
        expires_in_days: Some(30),
    };

    let (api_key, _raw_key) = manager
        .create_api_key(user.id, request)
        .expect("Failed to create API key");

    // Store in database
    db.create_api_key(&api_key)
        .await
        .expect("Failed to store API key");

    // Retrieve by prefix
    let retrieved = db
        .get_api_key_by_prefix(&api_key.key_prefix, &api_key.key_hash)
        .await
        .expect("Failed to get API key")
        .expect("API key not found");

    assert_eq!(retrieved.id, api_key.id);
    assert_eq!(retrieved.name, api_key.name);
    assert_eq!(retrieved.tier, api_key.tier);
}

#[tokio::test]
async fn test_api_key_usage_tracking() {
    let db = pierre_mcp_server::database::tests::create_test_db()
        .await
        .expect("Failed to create test database");

    let user = create_test_user(&db).await;

    // Create API key
    let manager = ApiKeyManager::new();
    let request = CreateApiKeyRequest {
        name: "Usage Test Key".into(),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: Some(100),
        expires_in_days: None,
    };

    let (api_key, _) = manager
        .create_api_key(user.id, request)
        .expect("Failed to create API key");

    db.create_api_key(&api_key)
        .await
        .expect("Failed to store API key");

    // Record usage
    let usage = ApiKeyUsage {
        id: None,
        api_key_id: api_key.id.clone(),
        timestamp: Utc::now(),
        tool_name: "get_activities".into(),
        status_code: 200,
        response_time_ms: Some(50),
        request_size_bytes: Some(256),
        response_size_bytes: Some(1024),
        ip_address: Some(pierre_mcp_server::constants::demo_data::TEST_IP_ADDRESS.to_string()),
        user_agent: Some("TestAgent/1.0".into()),
        error_message: None,
    };

    db.record_api_key_usage(&usage)
        .await
        .expect("Failed to record usage");

    // Check current usage
    let current_usage = db
        .get_api_key_current_usage(&api_key.id)
        .await
        .expect("Failed to get current usage");
    assert_eq!(current_usage, 1);

    // Get usage stats
    let stats = db
        .get_api_key_usage_stats(
            &api_key.id,
            Utc::now() - Duration::hours(1),
            Utc::now() + Duration::hours(1),
        )
        .await
        .expect("Failed to get usage stats");

    assert_eq!(stats.total_requests, 1);
    assert_eq!(stats.successful_requests, 1);
    assert_eq!(stats.failed_requests, 0);
}

#[tokio::test]
async fn test_api_key_expiration() {
    let db = pierre_mcp_server::database::tests::create_test_db()
        .await
        .expect("Failed to create test database");

    let user = create_test_user(&db).await;

    // Create expired API key - use a clearly expired timestamp
    let unique_id = Uuid::new_v4();
    let api_key = ApiKey {
        id: format!("test_{unique_id}"),
        user_id: user.id,
        name: format!("Expired Key {unique_id}"),
        description: None,
        key_hash: format!("expired_hash_{unique_id}"),
        key_prefix: {
            let simple_id = unique_id.simple();
            format!("exp_{simple_id}_")
        },
        tier: ApiKeyTier::Trial,
        rate_limit_requests: 10,
        rate_limit_window_seconds: 3600,
        is_active: true,
        expires_at: Some(DateTime::from_timestamp(1_000_000_000, 0).unwrap()), // Year 2001 - clearly expired
        last_used_at: None,
        created_at: Utc::now() - Duration::days(1),
    };

    db.create_api_key(&api_key)
        .await
        .expect("Failed to store API key");

    // Get expired keys
    let expired = db
        .get_expired_api_keys()
        .await
        .expect("Failed to get expired keys");
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].id, api_key.id);

    // Cleanup expired keys
    let cleaned = db
        .cleanup_expired_api_keys()
        .await
        .expect("Failed to cleanup expired keys");
    assert_eq!(cleaned, 1);

    // Verify key is deactivated
    let updated = db
        .get_api_key_by_id(&api_key.id)
        .await
        .expect("Failed to get API key")
        .expect("API key not found");
    assert!(!updated.is_active);
}
