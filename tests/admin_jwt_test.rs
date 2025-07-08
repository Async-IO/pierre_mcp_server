// ABOUTME: Integration tests for admin JWT token generation, validation, and management
// ABOUTME: Tests JWT token lifecycle including generation, validation, expiration, and storage operations

use chrono::{Duration, Utc};
use pierre_mcp_server::admin::{
    jwt::AdminJwtManager,
    models::{AdminPermission, AdminPermissions},
};

#[test]
fn test_jwt_generation_and_validation() {
    let jwt_manager = AdminJwtManager::new();
    let permissions = AdminPermissions::default_admin();

    let token = jwt_manager
        .generate_token(
            "test_token_123",
            "test_service",
            &permissions,
            false,
            Some(Utc::now() + Duration::hours(1)),
        )
        .unwrap();

    let validated = jwt_manager.validate_token(&token).unwrap();

    assert_eq!(validated.token_id, "test_token_123");
    assert_eq!(validated.service_name, "test_service");
    assert!(!validated.is_super_admin);
    assert!(validated
        .permissions
        .has_permission(&AdminPermission::ProvisionKeys));
}

#[test]
fn test_expired_token_rejection() {
    let jwt_manager = AdminJwtManager::new();
    let permissions = AdminPermissions::default_admin();

    // Create token that expires immediately
    let token = jwt_manager
        .generate_token(
            "expired_token",
            "test_service",
            &permissions,
            false,
            Some(Utc::now() - Duration::hours(1)), // Expired 1 hour ago
        )
        .unwrap();

    let result = jwt_manager.validate_token(&token);
    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();
    assert!(error_msg.contains("ExpiredSignature") || error_msg.contains("expired"));
}

#[test]
fn test_super_admin_token() {
    let jwt_manager = AdminJwtManager::new();
    let permissions = AdminPermissions::super_admin();

    let token = jwt_manager
        .generate_token(
            "super_admin_token",
            "admin_service",
            &permissions,
            true,
            None, // Never expires
        )
        .unwrap();

    let validated = jwt_manager.validate_token(&token).unwrap();

    assert!(validated.is_super_admin);
    assert!(validated
        .permissions
        .has_permission(&AdminPermission::ManageAdminTokens));
}

#[test]
fn test_token_prefix_generation() {
    let token = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9...";
    let prefix = AdminJwtManager::generate_token_prefix(token);
    assert_eq!(prefix, "admin_jwt_eyJ0eXAi");
}

#[test]
fn test_token_hashing() {
    let token = "test_token_123";
    let hash = AdminJwtManager::hash_token_for_storage(token).unwrap();
    assert!(AdminJwtManager::verify_token_hash(token, &hash).unwrap());
    assert!(!AdminJwtManager::verify_token_hash("wrong_token", &hash).unwrap());
}

#[test]
fn test_secret_generation() {
    let secret1 = AdminJwtManager::generate_jwt_secret();
    let secret2 = AdminJwtManager::generate_jwt_secret();

    assert_eq!(secret1.len(), 64);
    assert_eq!(secret2.len(), 64);
    assert_ne!(secret1, secret2); // Should be different
}

#[test]
fn test_secret_hashing() {
    let secret = "test_secret_123";
    let hash1 = AdminJwtManager::hash_secret(secret);
    let hash2 = AdminJwtManager::hash_secret(secret);

    assert_eq!(hash1, hash2); // Should be deterministic
    assert_eq!(hash1.len(), 64); // SHA-256 hex is 64 chars
}
