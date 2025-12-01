// ABOUTME: Integration tests for Redis cache backend implementation
// ABOUTME: Tests all CacheProvider operations with a real Redis instance (CI-only)
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::cache::{factory::Cache, CacheConfig, CacheKey, CacheResource};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TestData {
    value: String,
    count: u32,
}

/// Helper: Create test cache key
fn test_cache_key(resource: CacheResource) -> CacheKey {
    CacheKey::new(
        Uuid::new_v4(),
        Uuid::new_v4(),
        "strava".to_owned(),
        resource,
    )
}

/// Helper: Create Redis cache from `REDIS_URL` environment variable
/// Returns None if `REDIS_URL` is not set (allows skipping tests in non-Redis environments)
async fn create_redis_cache() -> Result<Option<Cache>> {
    let Ok(redis_url) = std::env::var("REDIS_URL") else {
        println!("REDIS_URL not set, skipping Redis cache tests");
        return Ok(None);
    };

    let config = CacheConfig {
        max_entries: 1000, // Not used for Redis, but required by config
        redis_url: Some(redis_url),
        cleanup_interval: Duration::from_secs(300),
        enable_background_cleanup: false, // Disable in tests
    };

    let cache = Cache::new(config).await?;

    Ok(Some(cache))
}

/// Helper macro to skip test if Redis is not available
macro_rules! require_redis {
    ($cache:expr) => {
        match $cache {
            Some(cache) => cache,
            None => {
                println!("Skipping test: Redis not available");
                return Ok(());
            }
        }
    };
}

#[tokio::test]
async fn test_redis_cache_health_check() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);

    // Redis PING should return PONG
    cache.health_check().await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_set_and_get() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "redis_test".to_owned(),
        count: 42,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Set value
    cache.set(&key, &data, Duration::from_secs(60)).await?;

    // Get value back
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, Some(data.clone()));

    // Clean up
    cache.invalidate(&key).await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_get_nonexistent() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);

    // Ensure key doesn't exist
    let _ = cache.invalidate(&key).await;

    // Get should return None for non-existent key
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, None);

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_expiration() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "expires".to_owned(),
        count: 1,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Set value with 1-second TTL
    cache.set(&key, &data, Duration::from_secs(1)).await?;

    // Should exist immediately
    assert!(cache.exists(&key).await?);

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // Should be expired
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, None);
    assert!(!cache.exists(&key).await?);

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_ttl() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "ttl_test".to_owned(),
        count: 5,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Set value with 60-second TTL
    cache.set(&key, &data, Duration::from_secs(60)).await?;

    // Check TTL immediately
    let ttl = cache.ttl(&key).await?;
    assert!(ttl.is_some());
    let ttl_secs = ttl.unwrap().as_secs();
    assert!(ttl_secs <= 60);
    assert!(ttl_secs >= 58); // Allow some time drift

    // Clean up
    cache.invalidate(&key).await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_ttl_nonexistent() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);

    // Ensure key doesn't exist
    let _ = cache.invalidate(&key).await;

    // TTL should return None for non-existent key
    let ttl = cache.ttl(&key).await?;
    assert!(ttl.is_none());

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_exists() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "exists_test".to_owned(),
        count: 1,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Should not exist initially
    assert!(!cache.exists(&key).await?);

    // Set value
    cache.set(&key, &data, Duration::from_secs(60)).await?;

    // Should exist now
    assert!(cache.exists(&key).await?);

    // Clean up
    cache.invalidate(&key).await?;

    // Should not exist after invalidation
    assert!(!cache.exists(&key).await?);

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_invalidate() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "delete_me".to_owned(),
        count: 99,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Set value
    cache.set(&key, &data, Duration::from_secs(60)).await?;
    assert!(cache.exists(&key).await?);

    // Invalidate
    cache.invalidate(&key).await?;

    // Should no longer exist
    assert!(!cache.exists(&key).await?);
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, None);

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_invalidate_pattern() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);

    // Use unique IDs for this test to avoid conflicts
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "pattern_test".to_owned(),
        count: 1,
    };

    // Create multiple keys for same user
    let key1 = CacheKey::new(
        tenant_id,
        user_id,
        "strava".to_owned(),
        CacheResource::AthleteProfile,
    );
    let key2 = CacheKey::new(
        tenant_id,
        user_id,
        "strava".to_owned(),
        CacheResource::Activity { activity_id: 123 },
    );
    let key3 = CacheKey::new(
        tenant_id,
        user_id,
        "strava".to_owned(),
        CacheResource::Stats { athlete_id: 456 },
    );

    // Clean up any existing keys first
    let _ = cache.invalidate(&key1).await;
    let _ = cache.invalidate(&key2).await;
    let _ = cache.invalidate(&key3).await;

    // Set all values
    cache.set(&key1, &data, Duration::from_secs(60)).await?;
    cache.set(&key2, &data, Duration::from_secs(60)).await?;
    cache.set(&key3, &data, Duration::from_secs(60)).await?;

    // All should exist
    assert!(cache.exists(&key1).await?);
    assert!(cache.exists(&key2).await?);
    assert!(cache.exists(&key3).await?);

    // Invalidate all entries for this user
    let pattern = CacheKey::user_pattern(tenant_id, user_id, "strava");
    let removed = cache.invalidate_pattern(&pattern).await?;
    assert_eq!(removed, 3);

    // All should be gone
    assert!(!cache.exists(&key1).await?);
    assert!(!cache.exists(&key2).await?);
    assert!(!cache.exists(&key3).await?);

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_tenant_isolation() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data1 = TestData {
        value: "tenant1".to_owned(),
        count: 1,
    };
    let data2 = TestData {
        value: "tenant2".to_owned(),
        count: 2,
    };

    // Create keys for two different tenants
    let key1 = CacheKey::new(
        tenant1,
        user_id,
        "strava".to_owned(),
        CacheResource::AthleteProfile,
    );
    let key2 = CacheKey::new(
        tenant2,
        user_id,
        "strava".to_owned(),
        CacheResource::AthleteProfile,
    );

    // Clean up any existing keys first
    let _ = cache.invalidate(&key1).await;
    let _ = cache.invalidate(&key2).await;

    // Set data for both tenants
    cache.set(&key1, &data1, Duration::from_secs(60)).await?;
    cache.set(&key2, &data2, Duration::from_secs(60)).await?;

    // Each tenant should only see their own data
    let retrieved1: Option<TestData> = cache.get(&key1).await?;
    let retrieved2: Option<TestData> = cache.get(&key2).await?;

    assert_eq!(retrieved1, Some(data1));
    assert_eq!(retrieved2, Some(data2));

    // Invalidating tenant1 should not affect tenant2
    cache.invalidate(&key1).await?;
    assert!(!cache.exists(&key1).await?);
    assert!(cache.exists(&key2).await?);

    // Clean up
    cache.invalidate(&key2).await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_clear_all() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);

    // Use unique IDs for this test
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "clear_test".to_owned(),
        count: 1,
    };

    // Create multiple keys
    let keys: Vec<_> = (0..10)
        .map(|i| {
            CacheKey::new(
                tenant_id,
                user_id,
                "strava".to_owned(),
                CacheResource::Activity { activity_id: i },
            )
        })
        .collect();

    // Clean up any existing keys first
    for key in &keys {
        let _ = cache.invalidate(key).await;
    }

    // Add entries
    for key in &keys {
        cache.set(key, &data, Duration::from_secs(60)).await?;
    }

    // All should exist
    for key in &keys {
        assert!(cache.exists(key).await?);
    }

    // Clear all (only within our namespace)
    cache.clear_all().await?;

    // All should be gone
    for key in &keys {
        assert!(!cache.exists(key).await?);
    }

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_different_resource_types() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "resource_type_test".to_owned(),
        count: 1,
    };

    // Test different resource types
    let resources = [
        CacheResource::AthleteProfile,
        CacheResource::ActivityList {
            page: 1,
            per_page: 30,
        },
        CacheResource::Activity { activity_id: 123 },
        CacheResource::Stats { athlete_id: 456 },
        CacheResource::DetailedActivity { activity_id: 789 },
    ];

    // Create keys and clean up first
    let keys: Vec<_> = resources
        .iter()
        .map(|r| CacheKey::new(tenant_id, user_id, "strava".to_owned(), r.clone()))
        .collect();

    for key in &keys {
        let _ = cache.invalidate(key).await;
    }

    // Set all values
    for key in &keys {
        cache.set(key, &data, Duration::from_secs(60)).await?;
    }

    // All should be retrievable
    for key in &keys {
        let retrieved: Option<TestData> = cache.get(key).await?;
        assert_eq!(retrieved, Some(data.clone()));
    }

    // Clean up
    for key in &keys {
        cache.invalidate(key).await?;
    }

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_overwrite() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);

    let data1 = TestData {
        value: "original".to_owned(),
        count: 1,
    };
    let data2 = TestData {
        value: "updated".to_owned(),
        count: 2,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Set initial value
    cache.set(&key, &data1, Duration::from_secs(60)).await?;

    // Verify initial value
    let retrieved1: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved1, Some(data1));

    // Overwrite with new value
    cache.set(&key, &data2, Duration::from_secs(60)).await?;

    // Should get updated value
    let retrieved2: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved2, Some(data2));

    // Clean up
    cache.invalidate(&key).await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_large_value() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let key = test_cache_key(CacheResource::AthleteProfile);

    // Create a larger data structure
    let large_value = "x".repeat(100_000); // 100KB string
    let data = TestData {
        value: large_value,
        count: 12345,
    };

    // Clean up any existing key first
    let _ = cache.invalidate(&key).await;

    // Set large value
    cache.set(&key, &data, Duration::from_secs(60)).await?;

    // Get value back
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, Some(data));

    // Clean up
    cache.invalidate(&key).await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_concurrent_operations() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    // Run concurrent set/get operations
    let mut handles = Vec::new();

    for i in 0u64..10 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move {
            let key = CacheKey::new(
                tenant_id,
                user_id,
                "strava".to_owned(),
                CacheResource::Activity { activity_id: i },
            );
            #[allow(clippy::cast_possible_truncation)] // i is in range 0..10, safe to truncate
            let data = TestData {
                value: format!("concurrent_{i}"),
                count: i as u32,
            };

            // Set value
            cache_clone
                .set(&key, &data, Duration::from_secs(60))
                .await?;

            // Get value back
            let retrieved: Option<TestData> = cache_clone.get(&key).await?;
            assert_eq!(retrieved, Some(data));

            Ok::<(), anyhow::Error>(())
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await??;
    }

    // Clean up
    let pattern = CacheKey::user_pattern(tenant_id, user_id, "strava");
    cache.invalidate_pattern(&pattern).await?;

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_invalidate_pattern_no_matches() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);

    // Try to invalidate pattern with no matching keys
    let removed = cache
        .invalidate_pattern("nonexistent_tenant:*:nonexistent_provider:*")
        .await?;

    // Should return 0 - no keys were removed
    assert_eq!(removed, 0);

    Ok(())
}

#[tokio::test]
async fn test_redis_cache_tenant_pattern_invalidation() -> Result<()> {
    let cache = require_redis!(create_redis_cache().await?);

    let tenant_id = Uuid::new_v4();
    let user1_id = Uuid::new_v4();
    let user2_id = Uuid::new_v4();

    let data = TestData {
        value: "tenant_pattern_test".to_owned(),
        count: 1,
    };

    // Create keys for two different users in same tenant
    let key1 = CacheKey::new(
        tenant_id,
        user1_id,
        "strava".to_owned(),
        CacheResource::AthleteProfile,
    );
    let key2 = CacheKey::new(
        tenant_id,
        user2_id,
        "strava".to_owned(),
        CacheResource::AthleteProfile,
    );

    // Clean up first
    let _ = cache.invalidate(&key1).await;
    let _ = cache.invalidate(&key2).await;

    // Set both values
    cache.set(&key1, &data, Duration::from_secs(60)).await?;
    cache.set(&key2, &data, Duration::from_secs(60)).await?;

    // Both should exist
    assert!(cache.exists(&key1).await?);
    assert!(cache.exists(&key2).await?);

    // Invalidate all entries for this tenant and provider
    let pattern = CacheKey::tenant_pattern(tenant_id, "strava");
    let removed = cache.invalidate_pattern(&pattern).await?;
    assert_eq!(removed, 2);

    // Both should be gone
    assert!(!cache.exists(&key1).await?);
    assert!(!cache.exists(&key2).await?);

    Ok(())
}
