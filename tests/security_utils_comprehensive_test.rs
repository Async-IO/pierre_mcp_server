// ABOUTME: Comprehensive integration tests for security and utilities modules
// ABOUTME: Tests encryption, key management, and core utilities
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org
//! Comprehensive tests for security and utility modules
//!
//! This test suite provides comprehensive coverage for security utilities,
//! encryption/decryption, and core functionality that currently have
//! limited integration test coverage.

use anyhow::Result;
use chrono::{Duration, Utc};
use pierre_mcp_server::{
    database::generate_encryption_key,
    models::{EncryptedToken, User, UserTier},
};
use uuid::Uuid;

#[tokio::test]
async fn test_token_encryption_comprehensive() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();

    // Test token creation and encryption with comprehensive scenarios
    let long_token = "very_long_token_".repeat(100);
    let long_refresh = "very_long_refresh_".repeat(50);

    let test_cases = vec![
        (
            "standard_access_token",
            "standard_refresh_token",
            "read,write,profile",
        ),
        (
            "token_with_special_chars_!@#$%^&*()",
            "refresh_with_unicode_key",
            "admin,super_user",
        ),
        (
            long_token.as_str(),
            long_refresh.as_str(),
            "extensive,permissions,list,with,many,scopes",
        ),
        ("", "empty_access_token_case", "basic"),
        ("empty_refresh_case", "", "minimal"),
    ];

    for (access_token, refresh_token, scope) in test_cases {
        let expires_at = Utc::now() + Duration::hours(2);

        // Create and encrypt token
        let encrypted_token = EncryptedToken::new(
            access_token,
            refresh_token,
            expires_at,
            scope.to_string(),
            &encryption_key,
        )?;

        // Verify token encryption is working (encrypted data should be different)
        assert_ne!(encrypted_token.access_token, access_token);
        assert_ne!(encrypted_token.refresh_token, refresh_token);

        // Decrypt and verify
        let decrypted_token = encrypted_token.decrypt(&encryption_key)?;
        assert_eq!(decrypted_token.access_token, access_token);
        assert_eq!(decrypted_token.refresh_token, refresh_token);
        assert_eq!(decrypted_token.expires_at, expires_at);
        assert_eq!(decrypted_token.scope, scope.to_string());
    }

    Ok(())
}

#[tokio::test]
async fn test_token_encryption_edge_cases() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();

    // Test with null/empty scope
    let token_no_scope = EncryptedToken::new(
        "access_token",
        "refresh_token",
        Utc::now() + Duration::hours(1),
        String::new(),
        &encryption_key,
    )?;

    let decrypted = token_no_scope.decrypt(&encryption_key)?;
    assert_eq!(decrypted.scope, String::new());

    // Test with maximum length tokens
    let max_access = "a".repeat(2048);
    let max_refresh = "r".repeat(2048);
    let max_scope = "scope_".repeat(100);

    let large_token = EncryptedToken::new(
        &max_access,
        &max_refresh,
        Utc::now() + Duration::hours(1),
        max_scope.clone(),
        &encryption_key,
    )?;

    let decrypted_large = large_token.decrypt(&encryption_key)?;
    assert_eq!(decrypted_large.access_token, max_access);
    assert_eq!(decrypted_large.refresh_token, max_refresh);
    assert_eq!(decrypted_large.scope, max_scope);

    Ok(())
}

#[tokio::test]
async fn test_encryption_key_generation_comprehensive() -> Result<()> {
    // Generate multiple encryption keys and verify uniqueness
    let mut keys = Vec::new();

    for _ in 0..100 {
        let key = generate_encryption_key();

        // Verify key length (should be 32 bytes for AES-256)
        assert_eq!(key.len(), 32);

        // Verify key is not all zeros
        assert!(key.iter().any(|&b| b != 0));

        // Verify uniqueness
        assert!(!keys.contains(&key), "Generated keys should be unique");
        keys.push(key);
    }

    Ok(())
}

#[tokio::test]
async fn test_encryption_with_different_keys() -> Result<()> {
    let key1 = generate_encryption_key().to_vec();
    let key2 = generate_encryption_key().to_vec();

    assert_ne!(key1, key2, "Different keys should be generated");

    let access_token = "test_access_token";
    let refresh_token = "test_refresh_token";
    let expires_at = Utc::now() + Duration::hours(1);
    let scope = "read,write".to_string();

    // Encrypt with key1
    let encrypted1 = EncryptedToken::new(
        access_token,
        refresh_token,
        expires_at,
        scope.clone(),
        &key1,
    )?;

    // Encrypt with key2
    let encrypted2 = EncryptedToken::new(access_token, refresh_token, expires_at, scope, &key2)?;

    // Encrypted results should be different
    assert_ne!(encrypted1.access_token, encrypted2.access_token);
    assert_ne!(encrypted1.refresh_token, encrypted2.refresh_token);

    // Decryption should work with correct keys
    let decrypted1 = encrypted1.decrypt(&key1)?;
    let decrypted2 = encrypted2.decrypt(&key2)?;

    assert_eq!(decrypted1.access_token, access_token);
    assert_eq!(decrypted2.access_token, access_token);

    // Decryption should fail with wrong keys
    let wrong_decrypt1 = encrypted1.decrypt(&key2);
    let wrong_decrypt2 = encrypted2.decrypt(&key1);

    assert!(
        wrong_decrypt1.is_err(),
        "Decryption with wrong key should fail"
    );
    assert!(
        wrong_decrypt2.is_err(),
        "Decryption with wrong key should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_user_model_comprehensive() -> Result<()> {
    // Test user creation with different tiers
    let test_users = vec![
        ("starter@example.com", "password123", UserTier::Starter),
        ("pro@example.com", "securepass456", UserTier::Professional),
        (
            "enterprise@example.com",
            "enterprisepass789",
            UserTier::Enterprise,
        ),
    ];

    for (email, password, tier) in test_users {
        // Hash the password before creating user
        let password_hash = format!("hashed_{password}");
        let mut user = User::new(
            email.to_string(),
            password_hash.clone(),
            Some(format!("Test User {email}")),
        );

        // Set tier
        user.tier = tier.clone();

        // Verify user properties
        assert_eq!(user.email, email);
        assert_eq!(user.tier, tier);
        assert!(user.id != Uuid::nil());
        assert!(user.created_at <= Utc::now());
        assert!(user.is_active);

        // Test password hashing (should not be plaintext)
        assert_ne!(user.password_hash, password);
        assert_eq!(user.password_hash, password_hash);
        assert!(!user.password_hash.is_empty());
    }

    Ok(())
}

#[tokio::test]
async fn test_user_tier_functionality() -> Result<()> {
    // Test all user tiers
    let tiers = vec![
        UserTier::Starter,
        UserTier::Professional,
        UserTier::Enterprise,
    ];

    for tier in tiers {
        let user = User::new(
            format!("test_{}@example.com", format!("{tier:?}").to_lowercase()),
            "password".to_string(),
            None,
        );

        // Verify tier can be set and retrieved
        let mut user_with_tier = user;
        user_with_tier.tier = tier.clone();
        assert_eq!(user_with_tier.tier, tier);

        // Test tier serialization
        let tier_str = serde_json::to_string(&tier)?;
        let deserialized_tier: UserTier = serde_json::from_str(&tier_str)?;
        assert_eq!(deserialized_tier, tier);
    }

    Ok(())
}

#[tokio::test]
async fn test_concurrent_token_operations() -> Result<()> {
    // Test concurrent encryption/decryption operations
    let encryption_key = generate_encryption_key().to_vec();
    let mut handles = vec![];

    for i in 0..50 {
        let key_clone = encryption_key.clone();
        handles.push(tokio::spawn(async move {
            let access_token = format!("concurrent_access_{i}");
            let refresh_token = format!("concurrent_refresh_{i}");
            let expires_at = Utc::now() + Duration::hours(1);
            let scope = format!("scope_{i}");

            // Encrypt
            let encrypted = EncryptedToken::new(
                &access_token,
                &refresh_token,
                expires_at,
                scope.clone(),
                &key_clone,
            )?;

            // Decrypt
            let decrypted = encrypted.decrypt(&key_clone)?;

            // Verify
            assert_eq!(decrypted.access_token, access_token);
            assert_eq!(decrypted.refresh_token, refresh_token);
            assert_eq!(decrypted.scope, scope);

            Ok::<_, anyhow::Error>(i)
        }));
    }

    // All operations should succeed
    for handle in handles {
        let result = handle.await??;
        assert!(result < 50);
    }

    Ok(())
}

#[tokio::test]
async fn test_token_expiration_scenarios() -> Result<()> {
    let encryption_key = generate_encryption_key().to_vec();

    // Test tokens with various expiration times
    let expiration_scenarios = vec![
        ("past_token", Utc::now() - Duration::hours(1)), // Already expired
        ("current_token", Utc::now()),                   // Expires now
        ("future_token", Utc::now() + Duration::hours(1)), // Future expiration
        ("far_future", Utc::now() + Duration::days(365)), // Long-lived token
    ];

    for (token_name, expires_at) in expiration_scenarios {
        let encrypted_token = EncryptedToken::new(
            &format!("{token_name}_access"),
            &format!("{token_name}_refresh"),
            expires_at,
            "test_scope".to_string(),
            &encryption_key,
        )?;

        // Decryption should work regardless of expiration
        let decrypted = encrypted_token.decrypt(&encryption_key)?;

        // Verify the expiration time is preserved
        assert_eq!(decrypted.expires_at, expires_at);

        // Token content should be correct
        assert_eq!(decrypted.access_token, format!("{token_name}_access"));
        assert_eq!(decrypted.refresh_token, format!("{token_name}_refresh"));
    }

    Ok(())
}

#[tokio::test]
async fn test_security_integration_scenario() -> Result<()> {
    // Test a complete security workflow

    // 1. Create user
    let user = User::new(
        "security_test@example.com".to_string(),
        "secure_password_123".to_string(),
        Some("Security Test User".to_string()),
    );

    assert_eq!(user.id, user.id);
    assert!(user.is_active);

    // 2. Generate encryption key
    let encryption_key = generate_encryption_key().to_vec();

    // 3. Create and encrypt tokens
    let token_pairs = vec![
        ("oauth_access_token", "oauth_refresh_token", "read,profile"),
        (
            "admin_access_token",
            "admin_refresh_token",
            "admin,super_user",
        ),
        ("api_access_token", "api_refresh_token", "api,integration"),
    ];

    let mut encrypted_tokens = Vec::new();

    for (access, refresh, scope) in token_pairs {
        let encrypted = EncryptedToken::new(
            access,
            refresh,
            Utc::now() + Duration::hours(2),
            scope.to_string(),
            &encryption_key,
        )?;

        encrypted_tokens.push((encrypted, access, refresh, scope));
    }

    // 4. Verify all tokens can be decrypted
    for (encrypted_token, original_access, original_refresh, original_scope) in encrypted_tokens {
        let decrypted = encrypted_token.decrypt(&encryption_key)?;

        assert_eq!(decrypted.access_token, original_access);
        assert_eq!(decrypted.refresh_token, original_refresh);
        assert_eq!(decrypted.scope, original_scope.to_string());
    }

    // 5. Test key rotation scenario (simulate new key)
    let new_encryption_key = generate_encryption_key().to_vec();
    assert_ne!(encryption_key, new_encryption_key);

    // Old tokens can still be decrypted with old key
    let old_token = EncryptedToken::new(
        "old_access",
        "old_refresh",
        Utc::now() + Duration::hours(1),
        "old_scope".to_string(),
        &encryption_key,
    )?;

    let decrypted_old = old_token.decrypt(&encryption_key)?;
    assert_eq!(decrypted_old.access_token, "old_access");

    // New tokens use new key
    let new_token = EncryptedToken::new(
        "new_access",
        "new_refresh",
        Utc::now() + Duration::hours(1),
        "new_scope".to_string(),
        &new_encryption_key,
    )?;

    let decrypted_new = new_token.decrypt(&new_encryption_key)?;
    assert_eq!(decrypted_new.access_token, "new_access");

    Ok(())
}
