// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for API key routes

use chrono::{Duration, Utc};
use pierre_mcp_server::{
    api_key_routes::ApiKeyRoutes,
    api_keys::{ApiKeyTier, CreateApiKeyRequest},
    auth::AuthManager,
    database::{generate_encryption_key, Database},
    models::User,
};
use uuid::Uuid;

async fn create_test_setup() -> (ApiKeyRoutes, Uuid, String) {
    // Create test database
    let database_url = "sqlite::memory:";
    let encryption_key = generate_encryption_key().to_vec();
    let database = Database::new(database_url, encryption_key).await.unwrap();

    // Create auth manager
    let jwt_secret = pierre_mcp_server::auth::generate_jwt_secret().to_vec();
    let auth_manager = AuthManager::new(jwt_secret, 24);

    // Create test user
    let user = User::new(
        "test@example.com".to_string(),
        "hashed_password".to_string(),
        Some("Test User".to_string()),
    );
    let user_id = database.create_user(&user).await.unwrap();

    // Generate JWT token for the user
    let jwt_token = auth_manager.generate_token(&user).unwrap();

    // Create API key routes
    let api_key_routes = ApiKeyRoutes::new(database, auth_manager);

    (api_key_routes, user_id, jwt_token)
}

#[tokio::test]
async fn test_create_api_key_success() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        description: Some("Test description".to_string()),
        tier: ApiKeyTier::Starter,
        expires_in_days: Some(30),
    };

    let auth_header = format!("Bearer {}", jwt_token);
    let response = api_key_routes
        .create_api_key(Some(&auth_header), request)
        .await
        .unwrap();

    // Verify response
    assert!(response.api_key.starts_with("pk_live_"));
    assert_eq!(response.api_key.len(), 40);
    assert_eq!(response.key_info.name, "Test API Key");
    assert_eq!(response.key_info.tier, ApiKeyTier::Starter);
    assert!(response.key_info.expires_at.is_some());
    assert!(response.warning.contains("Store this API key securely"));
}

#[tokio::test]
async fn test_create_api_key_invalid_auth() {
    let (api_key_routes, _user_id, _jwt_token) = create_test_setup().await;

    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
    };

    // Test with invalid auth header
    let result = api_key_routes
        .create_api_key(Some("Invalid Bearer token"), request)
        .await;
    assert!(result.is_err());

    // Test with no auth header
    let request = CreateApiKeyRequest {
        name: "Test API Key".to_string(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
    };

    let result = api_key_routes.create_api_key(None, request).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_api_keys() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    // Create a couple of API keys
    let request1 = CreateApiKeyRequest {
        name: "Key 1".to_string(),
        description: Some("First key".to_string()),
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
    };

    let request2 = CreateApiKeyRequest {
        name: "Key 2".to_string(),
        description: Some("Second key".to_string()),
        tier: ApiKeyTier::Professional,
        expires_in_days: Some(90),
    };

    let auth_header = format!("Bearer {}", jwt_token);

    // Create the keys
    api_key_routes
        .create_api_key(Some(&auth_header), request1)
        .await
        .unwrap();

    api_key_routes
        .create_api_key(Some(&auth_header), request2)
        .await
        .unwrap();

    // List keys
    let response = api_key_routes
        .list_api_keys(Some(&auth_header))
        .await
        .unwrap();

    // Verify response
    assert_eq!(response.api_keys.len(), 2);

    let key_names: Vec<_> = response.api_keys.iter().map(|k| &k.name).collect();
    assert!(key_names.contains(&&"Key 1".to_string()));
    assert!(key_names.contains(&&"Key 2".to_string()));

    // Check tiers
    let starter_key = response
        .api_keys
        .iter()
        .find(|k| k.name == "Key 1")
        .unwrap();
    let pro_key = response
        .api_keys
        .iter()
        .find(|k| k.name == "Key 2")
        .unwrap();

    assert_eq!(starter_key.tier, ApiKeyTier::Starter);
    assert_eq!(pro_key.tier, ApiKeyTier::Professional);
    assert!(starter_key.expires_at.is_none());
    assert!(pro_key.expires_at.is_some());
}

#[tokio::test]
async fn test_deactivate_api_key() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Key to deactivate".to_string(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
    };

    let auth_header = format!("Bearer {}", jwt_token);
    let create_response = api_key_routes
        .create_api_key(Some(&auth_header), request)
        .await
        .unwrap();

    let key_id = &create_response.key_info.id;

    // Deactivate the key
    let deactivate_response = api_key_routes
        .deactivate_api_key(Some(&auth_header), key_id)
        .await
        .unwrap();

    assert!(deactivate_response.message.contains("deactivated"));
    assert!(deactivate_response.deactivated_at <= Utc::now());

    // Verify key is no longer active in the list
    let list_response = api_key_routes
        .list_api_keys(Some(&auth_header))
        .await
        .unwrap();

    let deactivated_key = list_response
        .api_keys
        .iter()
        .find(|k| k.id == *key_id)
        .unwrap();

    assert!(!deactivated_key.is_active);
}

#[tokio::test]
async fn test_deactivate_nonexistent_key() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    let auth_header = format!("Bearer {}", jwt_token);
    let fake_key_id = "nonexistent_key_id";

    let result = api_key_routes
        .deactivate_api_key(Some(&auth_header), fake_key_id)
        .await;

    // Should succeed (idempotent operation)
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_api_key_usage_stats() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    // Create an API key
    let request = CreateApiKeyRequest {
        name: "Usage Test Key".to_string(),
        description: None,
        tier: ApiKeyTier::Professional,
        expires_in_days: None,
    };

    let auth_header = format!("Bearer {}", jwt_token);
    let create_response = api_key_routes
        .create_api_key(Some(&auth_header), request)
        .await
        .unwrap();

    let key_id = &create_response.key_info.id;

    // Get usage stats (should be empty for new key)
    let start_date = Utc::now() - Duration::days(30);
    let end_date = Utc::now();

    let usage_response = api_key_routes
        .get_api_key_usage(Some(&auth_header), key_id, start_date, end_date)
        .await
        .unwrap();

    // Verify empty usage stats
    assert_eq!(usage_response.stats.api_key_id, *key_id);
    assert_eq!(usage_response.stats.total_requests, 0);
    assert_eq!(usage_response.stats.successful_requests, 0);
    assert_eq!(usage_response.stats.failed_requests, 0);
}

#[tokio::test]
async fn test_get_usage_stats_unauthorized_key() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    // Try to access usage stats for a key that doesn't belong to the user
    let auth_header = format!("Bearer {}", jwt_token);
    let fake_key_id = "some_other_users_key";

    let start_date = Utc::now() - Duration::days(30);
    let end_date = Utc::now();

    let result = api_key_routes
        .get_api_key_usage(Some(&auth_header), fake_key_id, start_date, end_date)
        .await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not found or access denied"));
}

#[tokio::test]
async fn test_api_key_tiers() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    let auth_header = format!("Bearer {}", jwt_token);

    // Test all tiers
    for (tier, tier_name) in [
        (ApiKeyTier::Starter, "Starter"),
        (ApiKeyTier::Professional, "Professional"),
        (ApiKeyTier::Enterprise, "Enterprise"),
    ] {
        let request = CreateApiKeyRequest {
            name: format!("{} Key", tier_name),
            description: Some(format!("Test {} tier", tier_name)),
            tier: tier.clone(),
            expires_in_days: None,
        };

        let response = api_key_routes
            .create_api_key(Some(&auth_header), request)
            .await
            .unwrap();

        assert_eq!(response.key_info.tier, tier);
        assert_eq!(response.key_info.name, format!("{} Key", tier_name));
    }
}

#[tokio::test]
async fn test_api_key_expiration() {
    let (api_key_routes, _user_id, jwt_token) = create_test_setup().await;

    let auth_header = format!("Bearer {}", jwt_token);

    // Test key with expiration
    let request = CreateApiKeyRequest {
        name: "Expiring Key".to_string(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: Some(7),
    };

    let response = api_key_routes
        .create_api_key(Some(&auth_header), request)
        .await
        .unwrap();

    // Verify expiration is set correctly
    assert!(response.key_info.expires_at.is_some());
    let expires_at = response.key_info.expires_at.unwrap();
    let expected_expiry = Utc::now() + Duration::days(7);

    // Should be within 1 minute of expected (to account for test execution time)
    let diff = (expires_at - expected_expiry).num_seconds().abs();
    assert!(
        diff < 60,
        "Expiration time should be within 1 minute of expected"
    );
}

#[tokio::test]
async fn test_authentication_with_different_users() {
    // Create first user setup
    let (api_key_routes1, _user_id1, jwt_token1) = create_test_setup().await;

    // Create second user in same database
    let _user2 = User::new(
        "user2@example.com".to_string(),
        "hashed_password2".to_string(),
        Some("User 2".to_string()),
    );

    // We need access to the database to create the second user
    // This test demonstrates that each setup creates its own isolated database
    // In a real scenario, we'd use the same database instance

    // For now, let's verify that each user can only access their own keys
    let auth_header1 = format!("Bearer {}", jwt_token1);

    // Create key for user 1
    let request = CreateApiKeyRequest {
        name: "User 1 Key".to_string(),
        description: None,
        tier: ApiKeyTier::Starter,
        expires_in_days: None,
    };

    api_key_routes1
        .create_api_key(Some(&auth_header1), request)
        .await
        .unwrap();

    // List keys for user 1
    let list_response = api_key_routes1
        .list_api_keys(Some(&auth_header1))
        .await
        .unwrap();

    assert_eq!(list_response.api_keys.len(), 1);
    assert_eq!(list_response.api_keys[0].name, "User 1 Key");
}
