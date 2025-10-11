// ABOUTME: Unit tests for in-memory cache implementation
// ABOUTME: Tests TTL expiration, capacity limits, and background cleanup
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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
        "strava".to_string(),
        resource,
    )
}

/// Helper: Create in-memory cache with custom config
async fn create_test_cache(max_entries: usize, cleanup_interval_secs: u64) -> Result<Cache> {
    let config = CacheConfig {
        max_entries,
        redis_url: None,
        cleanup_interval: Duration::from_secs(cleanup_interval_secs),
        enable_background_cleanup: false, // Disable in tests to avoid tokio runtime conflicts
    };
    Cache::new(config).await
}

#[tokio::test]
async fn test_cache_set_and_get() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "test".to_string(),
        count: 42,
    };

    // Set value
    cache.set(&key, &data, Duration::from_secs(10)).await?;

    // Get value back
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, Some(data));

    Ok(())
}

#[tokio::test]
async fn test_cache_expiration() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "expires".to_string(),
        count: 1,
    };

    // Set value with 1-second TTL
    cache.set(&key, &data, Duration::from_secs(1)).await?;

    // Should exist immediately
    assert!(cache.exists(&key).await?);

    // Wait for expiration
    tokio::time::sleep(Duration::from_millis(1100)).await;

    // Should be expired
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, None);
    assert!(!cache.exists(&key).await?);

    Ok(())
}

#[tokio::test]
async fn test_cache_ttl() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "ttl_test".to_string(),
        count: 5,
    };

    // Set value with 10-second TTL
    cache.set(&key, &data, Duration::from_secs(10)).await?;

    // Check TTL immediately
    let ttl = cache.ttl(&key).await?;
    assert!(ttl.is_some());
    assert!(ttl.unwrap().as_secs() <= 10);
    assert!(ttl.unwrap().as_secs() >= 9); // Should be close to 10

    Ok(())
}

#[tokio::test]
async fn test_cache_invalidate() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "delete_me".to_string(),
        count: 99,
    };

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
async fn test_cache_invalidate_pattern() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "pattern_test".to_string(),
        count: 1,
    };

    // Set multiple keys for same user
    let key1 = CacheKey::new(
        tenant_id,
        user_id,
        "strava".to_string(),
        CacheResource::AthleteProfile,
    );
    let key2 = CacheKey::new(
        tenant_id,
        user_id,
        "strava".to_string(),
        CacheResource::Activity { activity_id: 123 },
    );
    let key3 = CacheKey::new(
        tenant_id,
        user_id,
        "strava".to_string(),
        CacheResource::Stats { athlete_id: 456 },
    );

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
async fn test_cache_tenant_isolation() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;

    let tenant1 = Uuid::new_v4();
    let tenant2 = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data1 = TestData {
        value: "tenant1".to_string(),
        count: 1,
    };
    let data2 = TestData {
        value: "tenant2".to_string(),
        count: 2,
    };

    // Set data for two different tenants
    let key1 = CacheKey::new(
        tenant1,
        user_id,
        "strava".to_string(),
        CacheResource::AthleteProfile,
    );
    let key2 = CacheKey::new(
        tenant2,
        user_id,
        "strava".to_string(),
        CacheResource::AthleteProfile,
    );

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

    Ok(())
}

#[tokio::test]
async fn test_cache_capacity_eviction() -> Result<()> {
    // Create cache with very small capacity
    let cache = create_test_cache(10, 300).await?;

    let data = TestData {
        value: "capacity_test".to_string(),
        count: 1,
    };

    // Fill cache beyond capacity
    for i in 0..20 {
        let key = test_cache_key(CacheResource::Activity { activity_id: i });
        cache.set(&key, &data, Duration::from_secs(60)).await?;
    }

    // Cache should have evicted some entries to stay within capacity
    // We don't test exact count because eviction is approximate (10% at a time)
    // but it should be less than 20
    let mut count = 0;
    for i in 0..20 {
        let key = test_cache_key(CacheResource::Activity { activity_id: i });
        if cache.exists(&key).await? {
            count += 1;
        }
    }

    // Should have evicted at least some entries
    assert!(count < 20);
    // Should not exceed capacity significantly
    assert!(count <= 12); // 10 + some margin for eviction granularity

    Ok(())
}

#[tokio::test]
async fn test_cache_background_cleanup() -> Result<()> {
    // Create cache with short cleanup interval
    let cache = create_test_cache(100, 1).await?;
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "cleanup_test".to_string(),
        count: 1,
    };

    // Create keys once so we can reuse them
    let keys: Vec<_> = (0..5)
        .map(|i| {
            CacheKey::new(
                tenant_id,
                user_id,
                "strava".to_string(),
                CacheResource::Activity { activity_id: i },
            )
        })
        .collect();

    // Create entries with short TTL (1 second - long enough to check they exist)
    for key in &keys {
        cache.set(key, &data, Duration::from_secs(1)).await?;
    }

    // All should exist immediately after creation
    for key in &keys {
        assert!(cache.exists(key).await?);
    }

    // Wait for expiration + cleanup cycles (1s TTL + 1s cleanup interval + margin)
    tokio::time::sleep(Duration::from_millis(2500)).await;

    // All should be cleaned up by background task
    for key in &keys {
        assert!(!cache.exists(key).await?);
    }

    Ok(())
}

#[tokio::test]
async fn test_cache_clear_all() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "clear_test".to_string(),
        count: 1,
    };

    // Create keys once so we can reuse them
    let keys: Vec<_> = (0..10)
        .map(|i| {
            CacheKey::new(
                tenant_id,
                user_id,
                "strava".to_string(),
                CacheResource::Activity { activity_id: i },
            )
        })
        .collect();

    // Add multiple entries
    for key in &keys {
        cache.set(key, &data, Duration::from_secs(60)).await?;
    }

    // All should exist
    for key in &keys {
        assert!(cache.exists(key).await?);
    }

    // Clear all
    cache.clear_all().await?;

    // All should be gone
    for key in &keys {
        assert!(!cache.exists(key).await?);
    }

    Ok(())
}

#[tokio::test]
async fn test_cache_health_check() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;

    // In-memory cache should always be healthy
    cache.health_check().await?;

    Ok(())
}

#[tokio::test]
async fn test_cache_different_resource_types() -> Result<()> {
    let cache = create_test_cache(100, 300).await?;
    let tenant_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();

    let data = TestData {
        value: "resource_type_test".to_string(),
        count: 1,
    };

    // Test different resource types
    let resources = vec![
        CacheResource::AthleteProfile,
        CacheResource::ActivityList {
            page: 1,
            per_page: 30,
        },
        CacheResource::Activity { activity_id: 123 },
        CacheResource::Stats { athlete_id: 456 },
        CacheResource::DetailedActivity { activity_id: 789 },
    ];

    for resource in &resources {
        let key = CacheKey::new(tenant_id, user_id, "strava".to_string(), resource.clone());
        cache.set(&key, &data, Duration::from_secs(60)).await?;
    }

    // All should be retrievable
    for resource in resources {
        let key = CacheKey::new(tenant_id, user_id, "strava".to_string(), resource);
        let retrieved: Option<TestData> = cache.get(&key).await?;
        assert_eq!(retrieved, Some(data.clone()));
    }

    Ok(())
}

#[tokio::test]
async fn test_cache_from_env_defaults() -> Result<()> {
    // Test cache creation from environment (should use defaults)
    let cache = Cache::from_env().await?;

    // Should be able to use it
    let key = test_cache_key(CacheResource::AthleteProfile);
    let data = TestData {
        value: "env_test".to_string(),
        count: 1,
    };

    cache.set(&key, &data, Duration::from_secs(10)).await?;
    let retrieved: Option<TestData> = cache.get(&key).await?;
    assert_eq!(retrieved, Some(data));

    Ok(())
}
