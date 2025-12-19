// ABOUTME: Unit tests for security module encryption functionality
// ABOUTME: Validates tenant encryption, key derivation, and security headers
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use pierre_mcp_server::security::{
    audit_security_headers, headers::SecurityConfig, EncryptedData, EncryptionMetadata,
    TenantEncryptionManager,
};
use std::collections::HashMap;
use uuid::Uuid;

// =============================================================================
// TenantEncryptionManager Tests
// =============================================================================

#[test]
fn test_encryption_manager_creation() {
    let master_key = [0u8; 32];
    let manager = TenantEncryptionManager::new(master_key);

    // Verify initial state
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.cached_tenant_keys, 0);
    assert_eq!(stats.master_key_algorithm, "AES-256-GCM");
    assert_eq!(stats.key_derivation_algorithm, "HKDF-SHA256");
}

#[test]
fn test_derive_tenant_key() {
    let master_key = [1u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    // Derive a key
    let key1 = manager.derive_tenant_key(tenant_id).unwrap();

    // Key should be 32 bytes
    assert_eq!(key1.len(), 32);

    // Deriving again should return cached key (same value)
    let key2 = manager.derive_tenant_key(tenant_id).unwrap();
    assert_eq!(key1, key2);

    // Stats should show 1 cached key
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.cached_tenant_keys, 1);
}

#[test]
fn test_derive_different_tenant_keys() {
    let master_key = [2u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();

    let key1 = manager.derive_tenant_key(tenant1).unwrap();
    let key2 = manager.derive_tenant_key(tenant2).unwrap();

    // Different tenants should have different derived keys
    assert_ne!(key1, key2);

    // Stats should show 2 cached keys
    let stats = manager.get_stats().unwrap();
    assert_eq!(stats.cached_tenant_keys, 2);
}

#[test]
fn test_encrypt_decrypt_tenant_data() {
    let master_key = [3u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();
    let plaintext = "This is sensitive OAuth token data";

    // Encrypt
    let encrypted = manager.encrypt_tenant_data(tenant_id, plaintext).unwrap();

    // Verify metadata
    assert!(encrypted.metadata.tenant_id.is_some());
    assert_eq!(encrypted.metadata.tenant_id.unwrap(), tenant_id);
    assert_eq!(encrypted.metadata.algorithm, "AES-256-GCM");
    assert_eq!(encrypted.metadata.key_version, 1);

    // Encrypted data should be different from plaintext
    assert_ne!(encrypted.data, plaintext);

    // Decrypt
    let decrypted = manager.decrypt_tenant_data(tenant_id, &encrypted).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_decrypt_global_data() {
    let master_key = [4u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let plaintext = "Global configuration secret";

    // Encrypt with global key
    let encrypted = manager.encrypt_global_data(plaintext).unwrap();

    // Verify metadata has no tenant ID
    assert!(encrypted.metadata.tenant_id.is_none());
    assert_eq!(encrypted.metadata.algorithm, "AES-256-GCM");

    // Decrypt
    let decrypted = manager.decrypt_global_data(&encrypted).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_tenant_data_isolation() {
    let master_key = [5u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();
    let plaintext = "Tenant-specific secret";

    // Encrypt with tenant1
    let encrypted = manager.encrypt_tenant_data(tenant1, plaintext).unwrap();

    // Attempt to decrypt with tenant2 should fail
    let result = manager.decrypt_tenant_data(tenant2, &encrypted);
    assert!(result.is_err());
}

#[test]
fn test_global_vs_tenant_data_mismatch() {
    let master_key = [6u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    // Encrypt as tenant data
    let tenant_encrypted = manager
        .encrypt_tenant_data(tenant_id, "tenant secret")
        .unwrap();

    // Try to decrypt as global data should fail
    let result = manager.decrypt_global_data(&tenant_encrypted);
    assert!(result.is_err());

    // Encrypt as global data
    let global_encrypted = manager.encrypt_global_data("global secret").unwrap();

    // Try to decrypt as tenant data should fail
    let result = manager.decrypt_tenant_data(tenant_id, &global_encrypted);
    assert!(result.is_err());
}

#[test]
fn test_encryption_produces_different_ciphertext() {
    let master_key = [7u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();
    let plaintext = "Same plaintext each time";

    // Encrypt the same plaintext multiple times
    let encrypted1 = manager.encrypt_tenant_data(tenant_id, plaintext).unwrap();
    let encrypted2 = manager.encrypt_tenant_data(tenant_id, plaintext).unwrap();

    // Due to random nonce, ciphertext should be different
    assert_ne!(encrypted1.data, encrypted2.data);

    // But both should decrypt to the same plaintext
    let decrypted1 = manager.decrypt_tenant_data(tenant_id, &encrypted1).unwrap();
    let decrypted2 = manager.decrypt_tenant_data(tenant_id, &encrypted2).unwrap();
    assert_eq!(decrypted1, plaintext);
    assert_eq!(decrypted2, plaintext);
}

#[test]
fn test_clear_key_cache() {
    let master_key = [8u8; 32];
    let manager = TenantEncryptionManager::new(master_key);

    // Derive keys for multiple tenants
    for _ in 0..5 {
        let tenant_id = Uuid::new_v4();
        manager.derive_tenant_key(tenant_id).unwrap();
    }

    let stats_before = manager.get_stats().unwrap();
    assert_eq!(stats_before.cached_tenant_keys, 5);

    // Clear cache
    manager.clear_key_cache().unwrap();

    let stats_after = manager.get_stats().unwrap();
    assert_eq!(stats_after.cached_tenant_keys, 0);
}

#[test]
fn test_key_version_management() {
    let master_key = [9u8; 32];
    let manager = TenantEncryptionManager::new(master_key);

    // Initial version should be 1
    assert_eq!(manager.get_current_version().unwrap(), 1);

    // Set new version
    manager.set_current_version(2).unwrap();
    assert_eq!(manager.get_current_version().unwrap(), 2);

    // Set higher version
    manager.set_current_version(10).unwrap();
    assert_eq!(manager.get_current_version().unwrap(), 10);
}

#[test]
fn test_encrypt_empty_string() {
    let master_key = [10u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    // Encrypt empty string
    let encrypted = manager.encrypt_tenant_data(tenant_id, "").unwrap();

    // Decrypt should return empty string
    let decrypted = manager.decrypt_tenant_data(tenant_id, &encrypted).unwrap();
    assert_eq!(decrypted, "");
}

#[test]
fn test_encrypt_large_data() {
    let master_key = [11u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    // Create large plaintext (1MB)
    let plaintext = "A".repeat(1_000_000);

    let encrypted = manager.encrypt_tenant_data(tenant_id, &plaintext).unwrap();
    let decrypted = manager.decrypt_tenant_data(tenant_id, &encrypted).unwrap();

    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_encrypt_unicode_data() {
    let master_key = [12u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    let plaintext = "Unicode test: 日本語 한국어 中文 émojis: 🏃‍♂️🚴‍♀️🏊‍♂️";

    let encrypted = manager.encrypt_tenant_data(tenant_id, plaintext).unwrap();
    let decrypted = manager.decrypt_tenant_data(tenant_id, &encrypted).unwrap();

    assert_eq!(decrypted, plaintext);
}

// =============================================================================
// SecurityConfig Tests
// =============================================================================

#[test]
fn test_security_config_development() {
    let config = SecurityConfig::development();

    assert_eq!(config.environment, "development");

    let headers = config.to_headers();
    assert!(headers.contains_key("Content-Security-Policy"));
    assert!(headers.contains_key("X-Frame-Options"));
    assert!(headers.contains_key("X-Content-Type-Options"));
    assert!(headers.contains_key("Referrer-Policy"));
    assert!(headers.contains_key("Permissions-Policy"));

    // Development CSP should allow unsafe-inline for easier debugging
    let csp = headers.get("Content-Security-Policy").unwrap();
    assert!(csp.contains("unsafe-inline"));
}

#[test]
fn test_security_config_production() {
    let config = SecurityConfig::production();

    assert_eq!(config.environment, "production");

    let headers = config.to_headers();
    assert!(headers.contains_key("Content-Security-Policy"));
    assert!(headers.contains_key("X-Frame-Options"));
    assert!(headers.contains_key("X-Content-Type-Options"));
    assert!(headers.contains_key("Strict-Transport-Security"));

    // Production should have HSTS
    let hsts = headers.get("Strict-Transport-Security").unwrap();
    assert!(hsts.contains("max-age="));
    assert!(hsts.contains("includeSubDomains"));

    // Production CSP should NOT allow unsafe-inline
    let csp = headers.get("Content-Security-Policy").unwrap();
    assert!(!csp.contains("unsafe-inline"));
}

#[test]
fn test_security_config_from_environment() {
    let prod_config = SecurityConfig::from_environment("production");
    assert_eq!(prod_config.environment, "production");

    let prod_config2 = SecurityConfig::from_environment("prod");
    assert_eq!(prod_config2.environment, "production");

    let dev_config = SecurityConfig::from_environment("development");
    assert_eq!(dev_config.environment, "development");

    let other_config = SecurityConfig::from_environment("staging");
    assert_eq!(other_config.environment, "development"); // Falls back to dev
}

// =============================================================================
// audit_security_headers Tests
// =============================================================================

#[test]
fn test_audit_security_headers_all_present() {
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Security-Policy".to_owned(),
        "default-src 'self'".to_owned(),
    );
    headers.insert("X-Frame-Options".to_owned(), "DENY".to_owned());
    headers.insert("X-Content-Type-Options".to_owned(), "nosniff".to_owned());

    assert!(audit_security_headers(&headers));
}

#[test]
fn test_audit_security_headers_missing_csp() {
    let mut headers = HashMap::new();
    headers.insert("X-Frame-Options".to_owned(), "DENY".to_owned());
    headers.insert("X-Content-Type-Options".to_owned(), "nosniff".to_owned());

    assert!(!audit_security_headers(&headers));
}

#[test]
fn test_audit_security_headers_missing_frame_options() {
    let mut headers = HashMap::new();
    headers.insert(
        "Content-Security-Policy".to_owned(),
        "default-src 'self'".to_owned(),
    );
    headers.insert("X-Content-Type-Options".to_owned(), "nosniff".to_owned());

    assert!(!audit_security_headers(&headers));
}

#[test]
fn test_audit_security_headers_empty() {
    let headers: HashMap<String, String> = HashMap::new();
    assert!(!audit_security_headers(&headers));
}

// =============================================================================
// EncryptionMetadata Tests
// =============================================================================

#[test]
fn test_encryption_metadata_serialization() {
    let metadata = EncryptionMetadata {
        key_version: 3,
        tenant_id: Some(Uuid::new_v4()),
        algorithm: "AES-256-GCM".to_owned(),
        encrypted_at: chrono::Utc::now(),
    };

    let json = serde_json::to_string(&metadata).unwrap();
    assert!(json.contains("key_version"));
    assert!(json.contains("AES-256-GCM"));

    let deserialized: EncryptionMetadata = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.key_version, metadata.key_version);
    assert_eq!(deserialized.algorithm, metadata.algorithm);
}

#[test]
fn test_encrypted_data_serialization() {
    let encrypted = EncryptedData {
        data: "base64encodeddata==".to_owned(),
        metadata: EncryptionMetadata {
            key_version: 1,
            tenant_id: None,
            algorithm: "AES-256-GCM".to_owned(),
            encrypted_at: chrono::Utc::now(),
        },
    };

    let json = serde_json::to_string(&encrypted).unwrap();
    let deserialized: EncryptedData = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.data, encrypted.data);
    assert!(deserialized.metadata.tenant_id.is_none());
}

// =============================================================================
// Edge Cases and Error Handling
// =============================================================================

#[test]
fn test_decrypt_invalid_base64_data() {
    let master_key = [13u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    let invalid_encrypted = EncryptedData {
        data: "not-valid-base64!!!".to_owned(),
        metadata: EncryptionMetadata {
            key_version: 1,
            tenant_id: Some(tenant_id),
            algorithm: "AES-256-GCM".to_owned(),
            encrypted_at: chrono::Utc::now(),
        },
    };

    let result = manager.decrypt_tenant_data(tenant_id, &invalid_encrypted);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_truncated_data() {
    let master_key = [14u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    // Valid base64 but too short (less than 12 bytes for nonce)
    let truncated_encrypted = EncryptedData {
        data: "AQIDBA==".to_owned(), // Only 4 bytes
        metadata: EncryptionMetadata {
            key_version: 1,
            tenant_id: Some(tenant_id),
            algorithm: "AES-256-GCM".to_owned(),
            encrypted_at: chrono::Utc::now(),
        },
    };

    let result = manager.decrypt_tenant_data(tenant_id, &truncated_encrypted);
    assert!(result.is_err());
}

#[test]
fn test_decrypt_tampered_data() {
    let master_key = [15u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    // First encrypt valid data
    let encrypted = manager
        .encrypt_tenant_data(tenant_id, "original data")
        .unwrap();

    // Tamper with the encrypted data (flip a bit)
    let mut tampered_bytes = STANDARD.decode(&encrypted.data).unwrap();
    if !tampered_bytes.is_empty() {
        let last_idx = tampered_bytes.len() - 1;
        tampered_bytes[last_idx] ^= 0xFF;
    }
    let tampered_data = STANDARD.encode(&tampered_bytes);

    let tampered_encrypted = EncryptedData {
        data: tampered_data,
        metadata: encrypted.metadata,
    };

    // Decryption should fail due to authentication tag mismatch
    let result = manager.decrypt_tenant_data(tenant_id, &tampered_encrypted);
    assert!(result.is_err());
}

#[test]
fn test_different_master_keys_produce_different_derived_keys() {
    let tenant_id = Uuid::new_v4();

    let manager1 = TenantEncryptionManager::new([1u8; 32]);
    let manager2 = TenantEncryptionManager::new([2u8; 32]);

    let key1 = manager1.derive_tenant_key(tenant_id).unwrap();
    let key2 = manager2.derive_tenant_key(tenant_id).unwrap();

    assert_ne!(key1, key2);
}

#[test]
fn test_encryption_with_special_characters() {
    let master_key = [16u8; 32];
    let manager = TenantEncryptionManager::new(master_key);
    let tenant_id = Uuid::new_v4();

    let special_data = r#"{"token": "abc123", "special": "\n\t\r", "unicode": "日本語"}"#;

    let encrypted = manager
        .encrypt_tenant_data(tenant_id, special_data)
        .unwrap();
    let decrypted = manager.decrypt_tenant_data(tenant_id, &encrypted).unwrap();

    assert_eq!(decrypted, special_data);
}
