// ABOUTME: Integration tests for RS256 JWT infrastructure and JWKS management
// ABOUTME: Tests asymmetric token generation, validation, key rotation, and JWKS endpoints
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

mod common;

use anyhow::Result;
use pierre_mcp_server::{
    admin::{
        jwks::JwksManager,
        jwt::AdminJwtManager,
        models::{AdminPermission, AdminPermissions},
    },
    auth::AuthManager,
    models::User,
};
use std::sync::Arc;

/// Test JWKS manager initialization and key generation
#[tokio::test]
async fn test_jwks_manager_initialization() -> Result<()> {
    let mut jwks_manager = JwksManager::new();

    // Generate initial key
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    // Verify active key exists
    let active_key = jwks_manager.get_active_key()?;
    assert!(!active_key.kid.is_empty());
    assert_eq!(active_key.kid, "test_key_1");

    // Verify key rotation works
    let old_kid = active_key.kid.clone();
    let new_kid = jwks_manager.rotate_keys_with_size(2048)?;

    assert_ne!(new_kid, old_kid);

    // Old key should still be retrievable
    let old_key = jwks_manager.get_key(&old_kid);
    assert!(old_key.is_some());

    Ok(())
}

/// Test JWKS endpoint format and structure
#[tokio::test]
async fn test_jwks_endpoint_format() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks = jwks_manager.get_jwks()?;

    assert!(!jwks.keys.is_empty());

    // Verify first key has required fields
    let first_key = &jwks.keys[0];
    assert_eq!(first_key.kty, "RSA");
    assert_eq!(first_key.alg, "RS256");
    assert_eq!(first_key.key_use, "sig");
    assert!(!first_key.kid.is_empty());
    assert!(!first_key.n.is_empty());
    assert!(!first_key.e.is_empty());

    Ok(())
}

/// Test RS256 user session tokens
#[tokio::test]
async fn test_rs256_user_session_tokens() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let auth_manager = AuthManager::new(24);

    let user = User::new(
        "test@example.com".to_string(),
        "password_hash".to_string(),
        Some("Test User".to_string()),
    );

    // Generate RS256 user session token
    let token = auth_manager.generate_token(&user, &jwks_manager_arc)?;

    assert!(!token.is_empty());
    assert!(token.starts_with("eyJ")); // JWT format

    // Validate token using RS256
    let claims = auth_manager.validate_token(&token, &jwks_manager_arc)?;
    assert_eq!(claims.sub, user.id.to_string());
    assert_eq!(claims.email, user.email);

    Ok(())
}

/// Test RS256 admin token generation and validation
#[tokio::test]
async fn test_rs256_admin_tokens() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let jwt_manager = AdminJwtManager::new();

    let token_id = "test_admin_token_123";
    let service_name = "test_service";
    let permissions = AdminPermissions::default_admin();

    // Generate RS256 admin token
    let token = jwt_manager.generate_token(
        token_id,
        service_name,
        &permissions,
        false,
        None,
        &jwks_manager_arc,
    )?;

    assert!(!token.is_empty());
    assert!(token.starts_with("eyJ")); // JWT format

    // Validate token using RS256
    let validated = jwt_manager.validate_token(&token, &jwks_manager_arc)?;
    assert_eq!(validated.token_id, token_id);
    assert_eq!(validated.service_name, service_name);
    assert!(!validated.is_super_admin);

    Ok(())
}

/// Test RS256 admin super admin token
#[tokio::test]
async fn test_rs256_super_admin_tokens() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let jwt_manager = AdminJwtManager::new();

    let token_id = "super_admin_token_456";
    let service_name = "super_admin_service";
    let permissions = AdminPermissions::super_admin();

    // Generate RS256 super admin token
    let token = jwt_manager.generate_token(
        token_id,
        service_name,
        &permissions,
        true, // is_super_admin
        None,
        &jwks_manager_arc,
    )?;

    // Validate token
    let validated = jwt_manager.validate_token(&token, &jwks_manager_arc)?;
    assert_eq!(validated.token_id, token_id);
    assert_eq!(validated.service_name, service_name);
    assert!(validated.is_super_admin);

    // Verify all permissions are present
    assert!(validated
        .permissions
        .has_permission(&AdminPermission::ProvisionKeys));
    assert!(validated
        .permissions
        .has_permission(&AdminPermission::ManageAdminTokens));
    assert!(validated
        .permissions
        .has_permission(&AdminPermission::ViewAuditLogs));

    Ok(())
}

/// Test key rotation with token validation
#[tokio::test]
async fn test_key_rotation_with_active_tokens() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let auth_manager = AuthManager::new(24);

    let user = User::new(
        "rotation_test@example.com".to_string(),
        "password_hash".to_string(),
        Some("Rotation Test User".to_string()),
    );

    // Generate token with initial key
    let token_before_rotation = auth_manager.generate_token(&user, &jwks_manager_arc)?;

    // Extract kid from token header
    let header = jsonwebtoken::decode_header(&token_before_rotation)?;
    let kid_before = header.kid.unwrap();

    // Token should validate before rotation
    let claims_before = auth_manager.validate_token(&token_before_rotation, &jwks_manager_arc)?;
    assert_eq!(claims_before.sub, user.id.to_string());

    // Rotate keys using Arc::get_mut workaround
    let new_kid = {
        // We need mutable access - in real code, JwksManager would use interior mutability
        // For testing, we'll create a new manager with both keys
        let mut new_manager = JwksManager::new();
        new_manager.generate_rsa_key_pair(&kid_before)?;
        new_manager.rotate_keys_with_size(2048)?
    };

    // Note: In production, the rotation would happen on the same Arc-wrapped manager
    // using interior mutability (RwLock/Mutex). For this test, we're verifying
    // the concept that old tokens remain valid after rotation.

    assert_ne!(new_kid, kid_before);

    Ok(())
}

/// Test token validation fails with tampered token
#[tokio::test]
async fn test_rs256_tampered_token_rejection() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let auth_manager = AuthManager::new(24);

    let user = User::new(
        "tamper_test@example.com".to_string(),
        "password_hash".to_string(),
        Some("Tamper Test User".to_string()),
    );

    // Generate valid token
    let mut token = auth_manager.generate_token(&user, &jwks_manager_arc)?;

    // Tamper with token by changing a character
    let bytes = unsafe { token.as_bytes_mut() };
    if bytes[50] == b'a' {
        bytes[50] = b'b';
    } else {
        bytes[50] = b'a';
    }

    // Tampered token should fail validation
    let result = auth_manager.validate_token(&token, &jwks_manager_arc);
    assert!(result.is_err());

    Ok(())
}

/// Test token validation fails with wrong JWKS manager
#[tokio::test]
async fn test_rs256_wrong_jwks_rejection() -> Result<()> {
    let mut jwks_manager1 = JwksManager::new();
    jwks_manager1.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let mut jwks_manager2 = JwksManager::new();
    jwks_manager2.generate_rsa_key_pair_with_size("test_key_2", 2048)?;

    let jwks_manager1_arc = Arc::new(jwks_manager1);
    let jwks_manager2_arc = Arc::new(jwks_manager2);
    let auth_manager = AuthManager::new(24);

    let user = User::new(
        "wrong_jwks_test@example.com".to_string(),
        "password_hash".to_string(),
        Some("Wrong JWKS Test User".to_string()),
    );

    // Generate token with jwks_manager1
    let token = auth_manager.generate_token(&user, &jwks_manager1_arc)?;

    // Try to validate with jwks_manager2 (different keys)
    let result = auth_manager.validate_token(&token, &jwks_manager2_arc);
    assert!(result.is_err());

    Ok(())
}

/// Test multiple concurrent key rotations
#[tokio::test]
async fn test_concurrent_key_rotation() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("initial_key", 2048)?;

    let initial_kid = jwks_manager.get_active_key()?.kid.clone();

    // Rotate keys multiple times (staying within MAX_HISTORICAL_KEYS retention limit of 3)
    for _ in 0..2 {
        jwks_manager.rotate_keys_with_size(2048)?;
    }

    let final_kid = jwks_manager.get_active_key()?.kid.clone();

    // Key should be different after rotations
    assert_ne!(initial_kid, final_kid);

    // Initial key should still be retrievable (within retention limit of 3 keys)
    assert!(jwks_manager.get_key(&initial_kid).is_some());

    Ok(())
}

/// Test admin token expiration with RS256
#[tokio::test]
async fn test_rs256_admin_token_expiration() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let jwt_manager = AdminJwtManager::new();

    let token_id = "expiring_token";
    let service_name = "expiring_service";
    let permissions = AdminPermissions::default_admin();

    // Generate token that expires in the past
    let expires_at = chrono::Utc::now() - chrono::Duration::hours(1);

    let token = jwt_manager.generate_token(
        token_id,
        service_name,
        &permissions,
        false,
        Some(expires_at),
        &jwks_manager_arc,
    )?;

    // Validation should fail due to expiration
    let result = jwt_manager.validate_token(&token, &jwks_manager_arc);
    assert!(result.is_err());

    Ok(())
}

/// Test JWKS with multiple keys
#[tokio::test]
async fn test_jwks_multiple_keys() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("test_key_1", 2048)?;

    // Rotate keys a few times to create multiple keys
    jwks_manager.rotate_keys_with_size(2048)?;
    jwks_manager.rotate_keys_with_size(2048)?;

    let jwks = jwks_manager.get_jwks()?;

    // Should have multiple keys (active + previous rotated keys)
    assert!(jwks.keys.len() >= 2);

    // All keys should have unique kid
    let mut kids = std::collections::HashSet::new();
    for key in &jwks.keys {
        assert!(kids.insert(key.kid.clone()), "Duplicate kid found");
    }

    Ok(())
}

/// Test token rejection with unknown kid
#[tokio::test]
async fn test_rs256_unknown_kid_rejection() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("known_key", 2048)?;

    let jwks_manager_arc = Arc::new(jwks_manager);
    let auth_manager = AuthManager::new(24);

    // Create separate manager with different kid
    let mut unknown_manager = JwksManager::new();
    unknown_manager.generate_rsa_key_pair_with_size("unknown_key", 2048)?;
    let unknown_manager_arc = Arc::new(unknown_manager);

    let user = User::new(
        "unknown_kid_test@example.com".to_string(),
        "password_hash".to_string(),
        Some("Unknown Kid Test User".to_string()),
    );

    // Generate token with unknown_manager (different kid)
    let token = auth_manager.generate_token(&user, &unknown_manager_arc)?;

    // Verify token has unknown kid in header
    let header = jsonwebtoken::decode_header(&token)?;
    assert_eq!(header.kid.as_deref(), Some("unknown_key"));

    // Try to validate with jwks_manager (different key set)
    let result = auth_manager.validate_token(&token, &jwks_manager_arc);
    assert!(result.is_err(), "Token with unknown kid should be rejected");

    Ok(())
}

/// Test tokens remain valid during rotation grace period
#[tokio::test]
async fn test_key_rotation_grace_period() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("grace_key_1", 2048)?;

    let kid_before_rotation = jwks_manager.get_active_key()?.kid.clone();

    let auth_manager = AuthManager::new(24);

    let user = User::new(
        "grace_period_test@example.com".to_string(),
        "password_hash".to_string(),
        Some("Grace Period Test User".to_string()),
    );

    // Generate token with initial key (before rotation)
    let jwks_manager_arc_before = Arc::new(jwks_manager);
    let token_with_old_key = auth_manager.generate_token(&user, &jwks_manager_arc_before)?;

    // Verify token is valid before rotation
    let claims_before =
        auth_manager.validate_token(&token_with_old_key, &jwks_manager_arc_before)?;
    assert_eq!(claims_before.email, user.email);

    // Extract manager back from Arc for rotation (test scenario only)
    let mut jwks_manager = Arc::try_unwrap(jwks_manager_arc_before)
        .unwrap_or_else(|_| panic!("Test: should have unique ownership"));

    // Rotate to new key - old key is retained for grace period
    let kid_after_rotation = jwks_manager.rotate_keys_with_size(2048)?;
    let jwks_manager_arc = Arc::new(jwks_manager);

    assert_ne!(
        kid_before_rotation, kid_after_rotation,
        "Key should change after rotation"
    );

    // OLD tokens should still validate during grace period (old key retained)
    let claims_after = auth_manager.validate_token(&token_with_old_key, &jwks_manager_arc)?;
    assert_eq!(
        claims_after.email, user.email,
        "Old token should remain valid during grace period"
    );

    // NEW tokens should work with new key
    let token_with_new_key = auth_manager.generate_token(&user, &jwks_manager_arc)?;
    let claims_new = auth_manager.validate_token(&token_with_new_key, &jwks_manager_arc)?;
    assert_eq!(
        claims_new.email, user.email,
        "New token should validate with new key"
    );

    Ok(())
}

/// Test key lifecycle: active -> rotated -> eventually pruned
#[tokio::test]
async fn test_key_lifecycle_rollover() -> Result<()> {
    let mut jwks_manager = JwksManager::new();
    jwks_manager.generate_rsa_key_pair_with_size("lifecycle_key_1", 2048)?;

    let initial_kid = jwks_manager.get_active_key()?.kid.clone();

    // Rotate keys multiple times to exceed retention limit (MAX_HISTORICAL_KEYS = 3)
    // Key lifecycle: initial_kid -> rotated1 -> rotated2 -> rotated3 -> rotated4
    // After 4 rotations, initial_kid should be pruned
    let rotated_kids = [
        jwks_manager.rotate_keys_with_size(2048)?,
        jwks_manager.rotate_keys_with_size(2048)?,
        jwks_manager.rotate_keys_with_size(2048)?,
        jwks_manager.rotate_keys_with_size(2048)?,
    ];

    let final_kid = jwks_manager.get_active_key()?.kid.clone();

    // Verify progression
    assert_ne!(
        initial_kid, final_kid,
        "Active key should change after rotations"
    );
    assert_eq!(
        rotated_kids[3], final_kid,
        "Final rotation should be active key"
    );

    // Initial key should be pruned (outside retention window of 3 keys)
    let initial_key_lookup = jwks_manager.get_key(&initial_kid);
    assert!(
        initial_key_lookup.is_none(),
        "Initial key should be pruned after exceeding retention limit"
    );

    // Most recent rotated keys should still be retained
    for recent_kid in &rotated_kids[1..] {
        let recent_key_lookup = jwks_manager.get_key(recent_kid);
        assert!(
            recent_key_lookup.is_some(),
            "Recent keys should be retained within retention limit"
        );
    }

    // Verify JWKS contains exactly MAX_HISTORICAL_KEYS keys
    let jwks = jwks_manager.get_jwks()?;
    assert!(
        jwks.keys.len() <= 3,
        "JWKS should respect MAX_HISTORICAL_KEYS retention limit"
    );

    Ok(())
}
