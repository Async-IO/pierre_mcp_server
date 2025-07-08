use chrono::{Datelike, Duration, Timelike, Utc};
use pierre_mcp_server::api_keys::{ApiKey, ApiKeyManager, ApiKeyTier, CreateApiKeyRequest};
use uuid::Uuid;

#[test]
fn test_api_key_generation() {
    let manager = ApiKeyManager::new();

    // Test regular key generation
    let api_key_data = manager.generate_api_key(false);
    let (full_key, prefix, hash) = (
        api_key_data.full_key,
        api_key_data.key_prefix,
        api_key_data.key_hash,
    );
    assert!(full_key.starts_with("pk_live_"));
    assert_eq!(full_key.len(), 40);
    assert_eq!(prefix.len(), 12);
    assert_eq!(hash.len(), 64); // SHA-256 hex

    // Test trial key generation
    let trial_data = manager.generate_api_key(true);
    let (trial_key, trial_prefix, trial_hash) = (
        trial_data.full_key,
        trial_data.key_prefix,
        trial_data.key_hash,
    );
    assert!(trial_key.starts_with("pk_trial_"));
    assert_eq!(trial_key.len(), 41);
    assert_eq!(trial_prefix.len(), 12);
    assert_eq!(trial_hash.len(), 64);
}

#[test]
fn test_key_validation() {
    let manager = ApiKeyManager::new();

    // Test regular key validation
    assert!(manager
        .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz123456")
        .is_ok());

    // Test trial key validation
    assert!(manager
        .validate_key_format("pk_trial_abcdefghijklmnopqrstuvwxyz123456")
        .is_ok());

    // Test invalid keys
    assert!(manager.validate_key_format("invalid_key").is_err());
    assert!(manager.validate_key_format("pk_live_short").is_err());
    assert!(manager.validate_key_format("pk_trial_short").is_err());
}

#[test]
fn test_tier_limits() {
    assert_eq!(ApiKeyTier::Trial.monthly_limit(), Some(1_000));
    assert_eq!(ApiKeyTier::Starter.monthly_limit(), Some(10_000));
    assert_eq!(ApiKeyTier::Professional.monthly_limit(), Some(100_000));
    assert_eq!(ApiKeyTier::Enterprise.monthly_limit(), None);

    // Test trial defaults
    assert_eq!(ApiKeyTier::Trial.default_trial_days(), Some(14));
    assert_eq!(ApiKeyTier::Starter.default_trial_days(), None);
    assert!(ApiKeyTier::Trial.is_trial());
    assert!(!ApiKeyTier::Starter.is_trial());
}

#[test]
fn test_rate_limit_calculation() {
    let manager = ApiKeyManager::new();

    let api_key = ApiKey {
        id: "test".into(),
        user_id: Uuid::new_v4(),
        name: "Test Key".into(),
        key_prefix: "pk_live_test".into(),
        key_hash: "hash".into(),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 10_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    let status = manager.rate_limit_status(&api_key, 5000);
    assert!(!status.is_rate_limited);
    assert_eq!(status.remaining, Some(5000));

    let status = manager.rate_limit_status(&api_key, 10_000);
    assert!(status.is_rate_limited);
    assert_eq!(status.remaining, Some(0));
}

#[test]
fn test_rate_limit_enterprise_unlimited() {
    let manager = ApiKeyManager::new();

    let enterprise_key = ApiKey {
        id: "enterprise".into(),
        user_id: Uuid::new_v4(),
        name: "Enterprise Key".into(),
        key_prefix: "pk_live_ent".into(),
        key_hash: "hash".into(),
        description: None,
        tier: ApiKeyTier::Enterprise,
        rate_limit_requests: 1_000_000_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    // Enterprise tier should never be rate limited
    let status = manager.rate_limit_status(&enterprise_key, 0);
    assert!(!status.is_rate_limited);
    assert_eq!(status.limit, None);
    assert_eq!(status.remaining, None);
    assert_eq!(status.reset_at, None);

    let status = manager.rate_limit_status(&enterprise_key, 1_000_000);
    assert!(!status.is_rate_limited);
    assert_eq!(status.limit, None);
    assert_eq!(status.remaining, None);
    assert_eq!(status.reset_at, None);
}

#[test]
fn test_rate_limit_professional_tier() {
    let manager = ApiKeyManager::new();

    let professional_key = ApiKey {
        id: "professional".into(),
        user_id: Uuid::new_v4(),
        name: "Professional Key".into(),
        key_prefix: "pk_live_pro".into(),
        key_hash: "hash".into(),
        description: None,
        tier: ApiKeyTier::Professional,
        rate_limit_requests: 100_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    // Under limit
    let status = manager.rate_limit_status(&professional_key, 50_000);
    assert!(!status.is_rate_limited);
    assert_eq!(status.limit, Some(100_000));
    assert_eq!(status.remaining, Some(50_000));
    assert!(status.reset_at.is_some());

    // At limit
    let status = manager.rate_limit_status(&professional_key, 100_000);
    assert!(status.is_rate_limited);
    assert_eq!(status.limit, Some(100_000));
    assert_eq!(status.remaining, Some(0));

    // Over limit
    let status = manager.rate_limit_status(&professional_key, 150_000);
    assert!(status.is_rate_limited);
    assert_eq!(status.limit, Some(100_000));
    assert_eq!(status.remaining, Some(0)); // Should be 0, not negative
}

#[test]
fn test_rate_limit_reset_time_calculation() {
    let manager = ApiKeyManager::new();

    let api_key = ApiKey {
        id: "reset_test".into(),
        user_id: Uuid::new_v4(),
        name: "Reset Test Key".into(),
        key_prefix: "pk_live_reset".into(),
        key_hash: "hash".into(),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 10_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    let status = manager.rate_limit_status(&api_key, 5000);

    // Reset time should be beginning of next month
    if let Some(reset_at) = status.reset_at {
        let now = Utc::now();

        // Reset should be in the future
        assert!(reset_at > now);

        // Reset should be at beginning of day (hour 0, minute 0, second 0)
        assert_eq!(reset_at.hour(), 0);
        assert_eq!(reset_at.minute(), 0);
        assert_eq!(reset_at.second(), 0);

        // Reset should be on the 1st day of some month
        assert_eq!(reset_at.day(), 1);
    } else {
        panic!("Reset time should be set for non-enterprise tiers");
    }
}

#[test]
fn test_api_key_validation_edge_cases() {
    let manager = ApiKeyManager::new();

    // Test various invalid key formats
    assert!(manager.validate_key_format("").is_err());
    assert!(manager.validate_key_format("pk_live_").is_err()); // Too short
    assert!(manager
        .validate_key_format("sk_live_abcdefghijklmnopqrstuvwxyz123456")
        .is_err()); // Wrong prefix
    assert!(manager
        .validate_key_format("pk_test_abcdefghijklmnopqrstuvwxyz123456")
        .is_err()); // Wrong prefix
    assert!(manager
        .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz12345")
        .is_err()); // Too short
    assert!(manager
        .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz1234567")
        .is_err()); // Too long

    // Test valid format
    assert!(manager
        .validate_key_format("pk_live_abcdefghijklmnopqrstuvwxyz123456")
        .is_ok());
}

#[test]
fn test_key_prefix_extraction() {
    let manager = ApiKeyManager::new();

    let full_key = "pk_live_abcdefghijklmnopqrstuvwxyz123456";
    let prefix = manager.extract_key_prefix(full_key);

    assert_eq!(prefix, "pk_live_abcd");
    assert_eq!(prefix.len(), 12);
}

#[test]
fn test_key_hashing_consistency() {
    let manager = ApiKeyManager::new();

    let key = "pk_live_test_key_for_hashing_12345678";
    let hash1 = manager.hash_key(key);
    let hash2 = manager.hash_key(key);

    // Same key should always produce same hash
    assert_eq!(hash1, hash2);

    // Hash should be SHA-256 hex (64 characters)
    assert_eq!(hash1.len(), 64);

    // Different keys should produce different hashes
    let different_key = "pk_live_different_key_12345678901234";
    let hash3 = manager.hash_key(different_key);
    assert_ne!(hash1, hash3);
}

#[tokio::test]
async fn test_create_api_key_with_expiration() {
    let manager = ApiKeyManager::new();
    let user_id = Uuid::new_v4();

    let request = CreateApiKeyRequest {
        name: "Expiring Key".into(),
        description: Some("Test key with expiration".into()),
        tier: ApiKeyTier::Professional,
        rate_limit_requests: None,
        expires_in_days: Some(30),
    };

    let (api_key, full_key) = manager.create_api_key(user_id, request).unwrap();

    // Check expiration is set correctly
    assert!(api_key.expires_at.is_some());
    let expires_at = api_key.expires_at.unwrap();
    let expected_expiry = Utc::now() + Duration::days(30);

    // Should be within 1 minute of expected (to account for test execution time)
    let diff = (expires_at - expected_expiry).num_seconds().abs();
    assert!(
        diff < 60,
        "Expiration time should be within 1 minute of expected"
    );

    // Check other properties
    assert_eq!(api_key.user_id, user_id);
    assert_eq!(api_key.name, "Expiring Key");
    assert_eq!(api_key.tier, ApiKeyTier::Professional);
    assert_eq!(api_key.rate_limit_requests, 100_000);
    assert!(api_key.is_active);

    // Full key should be valid format
    assert!(manager.validate_key_format(&full_key).is_ok());
    assert!(full_key.starts_with("pk_live_"));
    assert_eq!(full_key.len(), 40);
}

#[tokio::test]
async fn test_create_api_key_without_expiration() {
    let manager = ApiKeyManager::new();
    let user_id = Uuid::new_v4();

    let request = CreateApiKeyRequest {
        name: "Permanent Key".into(),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: None,
        expires_in_days: None,
    };

    let (api_key, _full_key) = manager.create_api_key(user_id, request).unwrap();

    // Should not have expiration
    assert!(api_key.expires_at.is_none());
    assert_eq!(api_key.tier, ApiKeyTier::Starter);
    assert_eq!(api_key.rate_limit_requests, 10_000);
}

#[test]
fn test_api_key_validation_scenarios() {
    let manager = ApiKeyManager::new();

    // Test active key
    let active_key = ApiKey {
        id: "active".into(),
        user_id: Uuid::new_v4(),
        name: "Active Key".into(),
        key_prefix: "pk_live_active".into(),
        key_hash: "hash".into(),
        description: None,
        tier: ApiKeyTier::Starter,
        rate_limit_requests: 10_000,
        rate_limit_window_seconds: 30 * 24 * 60 * 60,
        is_active: true,
        last_used_at: None,
        expires_at: None,
        created_at: Utc::now(),
    };

    assert!(manager.is_key_valid(&active_key).is_ok());

    // Test inactive key
    let mut inactive_key = active_key.clone();
    inactive_key.is_active = false;

    assert!(manager.is_key_valid(&inactive_key).is_err());

    // Test expired key
    let mut expired_key = active_key.clone();
    expired_key.expires_at = Some(Utc::now() - Duration::days(1));

    assert!(manager.is_key_valid(&expired_key).is_err());

    // Test key expiring in future (should be valid)
    let mut future_expiry_key = active_key;
    future_expiry_key.expires_at = Some(Utc::now() + Duration::days(1));

    assert!(manager.is_key_valid(&future_expiry_key).is_ok());
}

#[test]
fn test_tier_specific_properties() {
    // Test that each tier has correct limits
    assert_eq!(ApiKeyTier::Starter.monthly_limit(), Some(10_000));
    assert_eq!(ApiKeyTier::Professional.monthly_limit(), Some(100_000));
    assert_eq!(ApiKeyTier::Enterprise.monthly_limit(), None);

    // Test rate limit windows are consistent
    assert_eq!(ApiKeyTier::Starter.rate_limit_window(), 30 * 24 * 60 * 60);
    assert_eq!(
        ApiKeyTier::Professional.rate_limit_window(),
        30 * 24 * 60 * 60
    );
    assert_eq!(
        ApiKeyTier::Enterprise.rate_limit_window(),
        30 * 24 * 60 * 60
    );
}

#[test]
fn test_generate_multiple_unique_keys() {
    let manager = ApiKeyManager::new();

    // Generate multiple keys and ensure they're all different
    let mut keys = Vec::new();
    for i in 0..10 {
        let is_trial = i % 2 == 0; // Alternate between trial and regular keys
        let api_key_data = manager.generate_api_key(is_trial);
        let (full_key, prefix, hash) = (
            api_key_data.full_key,
            api_key_data.key_prefix,
            api_key_data.key_hash,
        );
        keys.push((full_key, prefix, hash));
    }

    // Check all keys are unique
    for i in 0..keys.len() {
        for j in (i + 1)..keys.len() {
            assert_ne!(keys[i].0, keys[j].0, "Full keys should be unique");
            assert_ne!(keys[i].1, keys[j].1, "Prefixes should be unique");
            assert_ne!(keys[i].2, keys[j].2, "Hashes should be unique");
        }
    }

    // Check all keys have correct format
    for (i, (full_key, prefix, hash)) in keys.iter().enumerate() {
        assert!(manager.validate_key_format(full_key).is_ok());
        assert_eq!(prefix.len(), 12);
        assert_eq!(hash.len(), 64);
        if i % 2 == 0 {
            assert!(full_key.starts_with("pk_trial_"));
            assert!(manager.is_trial_key(full_key));
        } else {
            assert!(full_key.starts_with("pk_live_"));
            assert!(!manager.is_trial_key(full_key));
        }
    }
}

#[tokio::test]
async fn test_create_trial_key() {
    let manager = ApiKeyManager::new();
    let user_id = Uuid::new_v4();

    // Create a trial key using the convenience method
    let (api_key, full_key) = manager
        .create_trial_key(
            user_id,
            "Test Trial Key".into(),
            Some("Testing trial functionality".into()),
        )
        .unwrap();

    // Verify trial key properties
    assert_eq!(api_key.tier, ApiKeyTier::Trial);
    assert_eq!(api_key.rate_limit_requests, 1_000);
    assert!(api_key.expires_at.is_some());

    // Verify key format
    assert!(full_key.starts_with("pk_trial_"));
    assert!(manager.is_trial_key(&full_key));
    assert!(manager.validate_key_format(&full_key).is_ok());

    // Verify expiration is set to 14 days
    let expires_at = api_key.expires_at.unwrap();
    let expected_expiry = Utc::now() + Duration::days(14);
    let diff = (expires_at - expected_expiry).num_seconds().abs();
    assert!(diff < 60, "Trial key should expire in 14 days");
}

#[tokio::test]
async fn test_create_trial_key_with_custom_expiration() {
    let manager = ApiKeyManager::new();
    let user_id = Uuid::new_v4();

    let request = CreateApiKeyRequest {
        name: "Custom Trial".into(),
        description: None,
        tier: ApiKeyTier::Trial,
        rate_limit_requests: None,
        expires_in_days: Some(7), // Custom 7 day trial
    };

    let (api_key, full_key) = manager.create_api_key(user_id, request).unwrap();

    // Verify custom expiration is respected
    assert!(api_key.expires_at.is_some());
    let expires_at = api_key.expires_at.unwrap();
    let expected_expiry = Utc::now() + Duration::days(7);
    let diff = (expires_at - expected_expiry).num_seconds().abs();
    assert!(diff < 60, "Trial key should expire in 7 days");

    assert!(full_key.starts_with("pk_trial_"));
}
