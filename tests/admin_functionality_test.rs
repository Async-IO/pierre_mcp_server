//! Comprehensive tests for admin functionality
//!
//! This module tests the complete admin system including:
//! - Admin token creation and management
//! - JWT authentication and authorization
//! - Admin API endpoints
//! - Database operations
//! - Security features

use anyhow::Result;
use chrono::Utc;
use pierre_mcp_server::admin::{
    auth::AdminAuthService,
    jwt::AdminJwtManager,
    models::{AdminAction, AdminPermission, AdminPermissions, CreateAdminTokenRequest},
};
use pierre_mcp_server::api_keys::{ApiKey, ApiKeyTier};
use pierre_mcp_server::database_plugins::{factory::Database, DatabaseProvider};
use pierre_mcp_server::models::User;
use serial_test::serial;
use uuid::Uuid;

/// Test database setup helper
async fn setup_test_database() -> Result<Database> {
    let encryption_key = vec![0u8; 32]; // Test key
    let db = Database::new("sqlite::memory:", encryption_key).await?;
    Ok(db)
}

/// Test user creation helper
async fn create_test_user(db: &Database) -> Result<(Uuid, User)> {
    let user = User {
        id: Uuid::new_v4(),
        email: "test@example.com".to_string(),
        display_name: Some("Test User".to_string()),
        password_hash: "test_hash".to_string(),
        strava_token: None,
        fitbit_token: None,
        created_at: Utc::now(),
        last_active: Utc::now(),
        is_active: true,
    };

    let user_id = db.create_user(&user).await?;
    Ok((user_id, user))
}

/// Test API key creation helper
async fn create_test_api_key(db: &Database, user_id: Uuid) -> Result<ApiKey> {
    let api_key = ApiKey {
        id: format!("test_key_{}", Uuid::new_v4().simple()),
        user_id,
        name: "Test Key".to_string(),
        key_prefix: "pk_test".to_string(),
        key_hash: "test_hash".to_string(),
        description: Some("Test API key".to_string()),
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 100,
        rate_limit_window: 3600,
        is_active: true,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        last_used_at: None,
        expires_at: None,
    };

    db.create_api_key(&api_key).await?;
    Ok(api_key)
}

#[tokio::test]
#[serial]
async fn test_admin_jwt_manager_basic_operations() -> Result<()> {
    let jwt_manager = AdminJwtManager::new();
    let token_id = "test_token_123";
    let service_name = "test_service";
    let permissions = AdminPermissions::default_admin();

    // Test token generation
    let token = jwt_manager.generate_token(token_id, service_name, &permissions, false, None)?;

    assert!(!token.is_empty());
    assert!(token.starts_with("eyJ")); // JWT format

    // Test token validation
    let claims = jwt_manager.validate_token(&token)?;
    assert_eq!(claims.token_id, token_id);
    assert_eq!(claims.service_name, service_name);
    assert_eq!(claims.permissions, permissions);
    assert!(!claims.is_super_admin);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_jwt_with_expiration() -> Result<()> {
    let jwt_manager = AdminJwtManager::new();
    let token_id = "test_token_exp";
    let service_name = "test_service";
    let permissions = AdminPermissions::super_admin();
    let expires_at = Utc::now() + chrono::Duration::hours(1);

    // Generate token with expiration
    let token =
        jwt_manager.generate_token(token_id, service_name, &permissions, true, Some(expires_at))?;

    // Validate token
    let claims = jwt_manager.validate_token(&token)?;
    assert_eq!(claims.token_id, token_id);
    assert!(claims.is_super_admin);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_permissions_system() -> Result<()> {
    // Test default admin permissions
    let default_perms = AdminPermissions::default_admin();
    assert!(default_perms.has_permission(&AdminPermission::ProvisionKeys));
    assert!(default_perms.has_permission(&AdminPermission::RevokeKeys));
    assert!(!default_perms.has_permission(&AdminPermission::ManageAdminTokens)); // Default admin can't manage other admins

    // Test super admin permissions
    let super_perms = AdminPermissions::super_admin();
    assert!(super_perms.has_permission(&AdminPermission::ProvisionKeys));
    assert!(super_perms.has_permission(&AdminPermission::RevokeKeys));
    assert!(super_perms.has_permission(&AdminPermission::ManageAdminTokens));
    assert!(super_perms.has_permission(&AdminPermission::ViewAuditLogs));

    // Test custom permissions
    let custom_permissions = vec![AdminPermission::ProvisionKeys, AdminPermission::ListKeys];
    let custom_perms = AdminPermissions::new(custom_permissions);
    assert!(custom_perms.has_permission(&AdminPermission::ProvisionKeys));
    assert!(custom_perms.has_permission(&AdminPermission::ListKeys));
    assert!(!custom_perms.has_permission(&AdminPermission::RevokeKeys));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_token_database_operations() -> Result<()> {
    let db = setup_test_database().await?;

    // Create admin token request
    let request = CreateAdminTokenRequest {
        service_name: "test_service".to_string(),
        service_description: Some("Test admin service".to_string()),
        permissions: Some(vec![
            AdminPermission::ProvisionKeys,
            AdminPermission::ListKeys,
        ]),
        is_super_admin: false,
        expires_in_days: Some(30),
    };

    // Create admin token
    let generated_token = db.create_admin_token(&request).await?;
    assert_eq!(generated_token.service_name, "test_service");
    assert!(!generated_token.is_super_admin);
    assert!(generated_token.expires_at.is_some());
    assert!(!generated_token.jwt_token.is_empty());

    // Retrieve admin token by ID
    let retrieved_token = db.get_admin_token_by_id(&generated_token.token_id).await?;
    assert!(retrieved_token.is_some());
    let token = retrieved_token.unwrap();
    assert_eq!(token.service_name, "test_service");
    assert_eq!(token.id, generated_token.token_id);

    // Retrieve by prefix
    let prefix_token = db
        .get_admin_token_by_prefix(&generated_token.token_prefix)
        .await?;
    assert!(prefix_token.is_some());
    assert_eq!(prefix_token.unwrap().id, generated_token.token_id);

    // List tokens
    let tokens = db.list_admin_tokens(true).await?;
    assert!(!tokens.is_empty());
    assert!(tokens.iter().any(|t| t.id == generated_token.token_id));

    // Deactivate token
    db.deactivate_admin_token(&generated_token.token_id).await?;
    let deactivated_token = db.get_admin_token_by_id(&generated_token.token_id).await?;
    assert!(deactivated_token.is_some());
    assert!(!deactivated_token.unwrap().is_active);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_token_usage_tracking() -> Result<()> {
    let db = setup_test_database().await?;

    // Create admin token
    let request = CreateAdminTokenRequest {
        service_name: "usage_test_service".to_string(),
        service_description: Some("Usage tracking test".to_string()),
        permissions: None, // Default permissions
        is_super_admin: false,
        expires_in_days: None,
    };

    let generated_token = db.create_admin_token(&request).await?;

    // Update last used
    let ip_address = "192.168.1.100";
    db.update_admin_token_last_used(&generated_token.token_id, Some(ip_address))
        .await?;

    // Verify last used was updated
    let updated_token = db.get_admin_token_by_id(&generated_token.token_id).await?;
    assert!(updated_token.is_some());
    let token = updated_token.unwrap();
    assert!(token.last_used_at.is_some());
    assert_eq!(token.last_used_ip, Some(ip_address.to_string()));
    assert!(token.usage_count > 0);

    // Record usage
    let usage = pierre_mcp_server::admin::models::AdminTokenUsage {
        id: None,
        admin_token_id: generated_token.token_id.clone(),
        timestamp: Utc::now(),
        action: AdminAction::ProvisionKey,
        target_resource: Some("test_api_key_123".to_string()),
        ip_address: Some(ip_address.to_string()),
        user_agent: Some("Test Agent".to_string()),
        request_size_bytes: Some(1024),
        success: true,
        error_message: None,
        response_time_ms: Some(150),
    };

    db.record_admin_token_usage(&usage).await?;

    // Get usage history
    let start_date = Utc::now() - chrono::Duration::hours(1);
    let end_date = Utc::now() + chrono::Duration::hours(1);
    let usage_history = db
        .get_admin_token_usage_history(&generated_token.token_id, start_date, end_date)
        .await?;

    assert!(!usage_history.is_empty());
    let recorded_usage = &usage_history[0];
    assert_eq!(recorded_usage.admin_token_id, generated_token.token_id);
    assert_eq!(recorded_usage.action, AdminAction::ProvisionKey);
    assert_eq!(
        recorded_usage.target_resource,
        Some("test_api_key_123".to_string())
    );
    assert!(recorded_usage.success);

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_provisioned_keys_tracking() -> Result<()> {
    let db = setup_test_database().await?;

    // Create admin token
    let request = CreateAdminTokenRequest {
        service_name: "provisioning_service".to_string(),
        service_description: Some("Key provisioning test".to_string()),
        permissions: Some(vec![AdminPermission::ProvisionKeys]),
        is_super_admin: false,
        expires_in_days: None,
    };

    let admin_token = db.create_admin_token(&request).await?;

    // Create user and API key
    let (user_id, user) = create_test_user(&db).await?;
    let api_key = create_test_api_key(&db, user_id).await?;

    // Record provisioned key
    db.record_admin_provisioned_key(
        &admin_token.token_id,
        &api_key.id,
        &user.email,
        "starter",
        100,
        "day",
    )
    .await?;

    // Get provisioned keys history
    let start_date = Utc::now() - chrono::Duration::hours(1);
    let end_date = Utc::now() + chrono::Duration::hours(1);

    // Test with specific admin token filter
    let filtered_keys = db
        .get_admin_provisioned_keys(Some(&admin_token.token_id), start_date, end_date)
        .await?;

    assert!(!filtered_keys.is_empty());
    let provisioned_key = &filtered_keys[0];
    assert_eq!(provisioned_key["admin_token_id"], admin_token.token_id);
    assert_eq!(provisioned_key["api_key_id"], api_key.id);
    assert_eq!(provisioned_key["user_email"], user.email);
    assert_eq!(provisioned_key["requested_tier"], "starter");

    // Test without admin token filter (all keys)
    let all_keys = db
        .get_admin_provisioned_keys(None, start_date, end_date)
        .await?;

    assert!(!all_keys.is_empty());
    assert!(all_keys.iter().any(|k| k["api_key_id"] == api_key.id));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_auth_service_construction() -> Result<()> {
    let db = setup_test_database().await?;
    let jwt_secret = "test_secret_for_admin_auth_service_testing_purposes";

    // Test that AdminAuthService can be constructed successfully
    let auth_service = AdminAuthService::new(db.clone(), jwt_secret);

    // Test basic functionality - invalid token should fail
    let invalid_result = auth_service
        .authenticate_and_authorize(
            "invalid_token",
            AdminPermission::ProvisionKeys,
            Some("192.168.1.100"),
        )
        .await;

    assert!(invalid_result.is_err());

    // Test with malformed token should fail
    let malformed_result = auth_service
        .authenticate_and_authorize(
            "not.a.jwt.token",
            AdminPermission::ProvisionKeys,
            Some("192.168.1.100"),
        )
        .await;

    assert!(malformed_result.is_err());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_token_security_features() -> Result<()> {
    let jwt_manager = AdminJwtManager::new();

    // Test token prefix generation
    let token = jwt_manager.generate_token(
        "test_security",
        "security_service",
        &AdminPermissions::default_admin(),
        false,
        None,
    )?;

    let prefix = AdminJwtManager::generate_token_prefix(&token);
    assert!(!prefix.is_empty());
    assert!(prefix.len() >= 8); // Should be at least 8 characters

    // Test token hashing for storage
    let hash = AdminJwtManager::hash_token_for_storage(&token)?;
    assert!(!hash.is_empty());
    assert_ne!(hash, token); // Hash should be different from token

    // Test secret hashing
    let secret = AdminJwtManager::generate_jwt_secret();
    assert_eq!(secret.len(), 64); // 512 bits = 64 bytes

    let secret_hash = AdminJwtManager::hash_secret(&secret);
    assert!(!secret_hash.is_empty());
    assert_ne!(secret_hash.len(), 0); // Hash should have content

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_permissions_serialization() -> Result<()> {
    // Test permissions serialization/deserialization
    let permissions = AdminPermissions::new(vec![
        AdminPermission::ProvisionKeys,
        AdminPermission::RevokeKeys,
        AdminPermission::ListKeys,
    ]);

    // Serialize to JSON
    let json_str = permissions.to_json()?;
    assert!(!json_str.is_empty());

    // Deserialize from JSON
    let deserialized = AdminPermissions::from_json(&json_str)?;
    assert_eq!(permissions, deserialized);

    // Test specific permissions
    assert!(deserialized.has_permission(&AdminPermission::ProvisionKeys));
    assert!(deserialized.has_permission(&AdminPermission::RevokeKeys));
    assert!(deserialized.has_permission(&AdminPermission::ListKeys));
    assert!(!deserialized.has_permission(&AdminPermission::ManageAdminTokens));

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_database_error_handling() -> Result<()> {
    let db = setup_test_database().await?;

    // Test getting non-existent admin token
    let non_existent = db.get_admin_token_by_id("non_existent_token").await?;
    assert!(non_existent.is_none());

    // Test getting by invalid prefix
    let invalid_prefix = db.get_admin_token_by_prefix("invalid_prefix").await?;
    assert!(invalid_prefix.is_none());

    // Test deactivating non-existent token (should not error)
    let deactivate_result = db.deactivate_admin_token("non_existent").await;
    assert!(deactivate_result.is_ok());

    // Test usage tracking for non-existent token (should not error)
    let usage_result = db
        .update_admin_token_last_used("non_existent", Some("127.0.0.1"))
        .await;
    assert!(usage_result.is_ok());

    Ok(())
}

#[tokio::test]
#[serial]
async fn test_admin_super_admin_privileges() -> Result<()> {
    let db = setup_test_database().await?;

    // Create super admin token
    let request = CreateAdminTokenRequest {
        service_name: "super_admin_service".to_string(),
        service_description: Some("Super admin test".to_string()),
        permissions: None, // Will get super admin permissions
        is_super_admin: true,
        expires_in_days: None,
    };

    let super_admin_token = db.create_admin_token(&request).await?;
    assert!(super_admin_token.is_super_admin);

    // Verify super admin has all permissions
    let retrieved = db
        .get_admin_token_by_id(&super_admin_token.token_id)
        .await?;
    assert!(retrieved.is_some());
    let token = retrieved.unwrap();

    // Super admin should have all permissions
    assert!(token
        .permissions
        .has_permission(&AdminPermission::ProvisionKeys));
    assert!(token
        .permissions
        .has_permission(&AdminPermission::RevokeKeys));
    assert!(token.permissions.has_permission(&AdminPermission::ListKeys));
    assert!(token
        .permissions
        .has_permission(&AdminPermission::ManageAdminTokens));
    assert!(token
        .permissions
        .has_permission(&AdminPermission::ViewAuditLogs));
    assert!(token
        .permissions
        .has_permission(&AdminPermission::ManageUsers));

    Ok(())
}
