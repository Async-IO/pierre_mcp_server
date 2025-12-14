// ABOUTME: Criterion benchmarks for cache operations comparing in-memory backend
// ABOUTME: Measures set/get/invalidate latency and throughput for various payload sizes
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Criterion benchmarks for cache operations.
//!
//! Measures set/get/invalidate latency and throughput for various payload sizes
//! using the in-memory cache backend.

#![allow(
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    missing_docs
)]

mod common;

use common::fixtures::{generate_activities, ActivityBatchSize};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pierre_mcp_server::cache::memory::InMemoryCache;
use pierre_mcp_server::cache::{CacheConfig, CacheKey, CacheProvider, CacheResource};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Test payload sizes for benchmarking
#[derive(Debug, Clone, Copy)]
enum PayloadSize {
    Small,
    Medium,
    Large,
    ExtraLarge,
}

impl PayloadSize {
    const fn bytes(self) -> usize {
        match self {
            Self::Small => 100,
            Self::Medium => 1_000,
            Self::Large => 10_000,
            Self::ExtraLarge => 100_000,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Small => "100B",
            Self::Medium => "1KB",
            Self::Large => "10KB",
            Self::ExtraLarge => "100KB",
        }
    }
}

/// Generate test payload of specified size
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestPayload {
    data: String,
    count: usize,
}

fn generate_payload(size: PayloadSize) -> TestPayload {
    let target_size = size.bytes();
    let data_size = target_size.saturating_sub(50);
    TestPayload {
        data: "x".repeat(data_size),
        count: target_size,
    }
}

/// Create a cache key for benchmarking
#[allow(clippy::cast_possible_truncation)]
fn make_cache_key(index: usize) -> CacheKey {
    CacheKey::new(
        Uuid::from_u128(1000),
        Uuid::from_u128(index as u128),
        "benchmark".to_owned(),
        CacheResource::ActivityList {
            page: 1,
            per_page: 20,
            before: None,
            after: None,
        },
    )
}

/// Create test cache configuration (no background cleanup for benchmarks)
fn test_cache_config() -> CacheConfig {
    CacheConfig {
        max_entries: 10_000,
        redis_url: None,
        cleanup_interval: Duration::from_secs(3600),
        enable_background_cleanup: false,
        ..Default::default()
    }
}

/// Benchmark cache set operations with different payload sizes
#[allow(clippy::cast_possible_truncation)]
fn bench_cache_set(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_set");

    for size in [
        PayloadSize::Small,
        PayloadSize::Medium,
        PayloadSize::Large,
        PayloadSize::ExtraLarge,
    ] {
        let payload = generate_payload(size);
        let config = test_cache_config();
        let cache = rt
            .block_on(async { InMemoryCache::new(config).await })
            .unwrap();

        group.throughput(Throughput::Bytes(size.bytes() as u64));
        group.bench_with_input(
            BenchmarkId::new("memory", size.name()),
            &payload,
            |b, payload| {
                let mut key_index = 0_usize;
                b.iter(|| {
                    let key = make_cache_key(key_index);
                    key_index = key_index.wrapping_add(1);
                    rt.block_on(async {
                        cache
                            .set(
                                black_box(&key),
                                black_box(payload),
                                Duration::from_secs(3600),
                            )
                            .await
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark cache get operations (hits vs misses)
fn bench_cache_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_get");

    let config = test_cache_config();
    let cache = rt
        .block_on(async { InMemoryCache::new(config).await })
        .unwrap();

    // Pre-populate cache with 1000 entries
    let payload = generate_payload(PayloadSize::Medium);
    rt.block_on(async {
        for i in 0..1000 {
            let key = make_cache_key(i);
            let _ = cache.set(&key, &payload, Duration::from_secs(3600)).await;
        }
    });

    // Benchmark cache hits
    group.bench_function("memory_hit", |b| {
        let mut key_index = 0_usize;
        b.iter(|| {
            let key = make_cache_key(key_index % 1000);
            key_index = key_index.wrapping_add(1);
            rt.block_on(async {
                let _: Option<TestPayload> = cache.get(black_box(&key)).await.unwrap();
            });
        });
    });

    // Benchmark cache misses
    group.bench_function("memory_miss", |b| {
        let mut key_index = 10_000_usize;
        b.iter(|| {
            let key = make_cache_key(key_index);
            key_index = key_index.wrapping_add(1);
            rt.block_on(async {
                let _: Option<TestPayload> = cache.get(black_box(&key)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark cache invalidation
fn bench_cache_invalidate(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_invalidate");

    // Single key invalidation
    group.bench_function("memory_single_key", |b| {
        let config = test_cache_config();
        let cache = rt
            .block_on(async { InMemoryCache::new(config).await })
            .unwrap();
        let payload = generate_payload(PayloadSize::Small);

        b.iter(|| {
            let key = make_cache_key(0);
            rt.block_on(async {
                let _ = cache.set(&key, &payload, Duration::from_secs(3600)).await;
                let _ = cache.invalidate(black_box(&key)).await;
            });
        });
    });

    // Pattern invalidation (tenant-wide)
    group.bench_function("memory_pattern_100_entries", |b| {
        b.iter_custom(|iters| {
            let config = test_cache_config();
            let cache = rt
                .block_on(async { InMemoryCache::new(config).await })
                .unwrap();
            let payload = generate_payload(PayloadSize::Small);
            let tenant_id = Uuid::from_u128(1000);

            // Pre-populate with 100 entries per iteration
            rt.block_on(async {
                for i in 0..100 {
                    let key = make_cache_key(i);
                    let _ = cache.set(&key, &payload, Duration::from_secs(3600)).await;
                }
            });

            let start = std::time::Instant::now();
            for _ in 0..iters {
                let pattern = CacheKey::tenant_pattern(tenant_id, "benchmark");
                rt.block_on(async {
                    let _ = cache.invalidate_pattern(black_box(&pattern)).await;
                });
            }
            start.elapsed()
        });
    });

    group.finish();
}

/// Benchmark cache with realistic Activity data
#[allow(clippy::cast_possible_truncation)]
fn bench_cache_activities(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_activities");

    let activities_small = generate_activities(ActivityBatchSize::Small);
    let activities_medium = generate_activities(ActivityBatchSize::Medium);

    let config = test_cache_config();
    let cache = rt
        .block_on(async { InMemoryCache::new(config).await })
        .unwrap();

    // Cache 10 activities
    let serialized_small = serde_json::to_vec(&activities_small).unwrap();
    group.throughput(Throughput::Bytes(serialized_small.len() as u64));
    group.bench_function("set_10_activities", |b| {
        let mut key_index = 0_usize;
        b.iter(|| {
            let key = make_cache_key(key_index);
            key_index = key_index.wrapping_add(1);
            rt.block_on(async {
                cache
                    .set(
                        black_box(&key),
                        black_box(&activities_small),
                        Duration::from_secs(3600),
                    )
                    .await
            })
        });
    });

    // Cache 100 activities
    let serialized_medium = serde_json::to_vec(&activities_medium).unwrap();
    group.throughput(Throughput::Bytes(serialized_medium.len() as u64));
    group.bench_function("set_100_activities", |b| {
        let mut key_index = 0_usize;
        b.iter(|| {
            let key = make_cache_key(key_index);
            key_index = key_index.wrapping_add(1);
            rt.block_on(async {
                cache
                    .set(
                        black_box(&key),
                        black_box(&activities_medium),
                        Duration::from_secs(3600),
                    )
                    .await
            })
        });
    });

    // Pre-populate and benchmark retrieval
    let retrieval_key = make_cache_key(99999);
    rt.block_on(async {
        let _ = cache
            .set(
                &retrieval_key,
                &activities_medium,
                Duration::from_secs(3600),
            )
            .await;
    });

    group.bench_function("get_100_activities", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _: Option<Vec<pierre_mcp_server::models::Activity>> =
                    cache.get(black_box(&retrieval_key)).await.unwrap();
            });
        });
    });

    group.finish();
}

/// Benchmark concurrent cache operations
fn bench_cache_concurrent(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("cache_concurrent");
    group.sample_size(50);

    let config = test_cache_config();
    let cache = rt
        .block_on(async { InMemoryCache::new(config).await })
        .unwrap();
    let payload = generate_payload(PayloadSize::Medium);

    // Pre-populate cache
    rt.block_on(async {
        for i in 0..1000 {
            let key = make_cache_key(i);
            let _ = cache.set(&key, &payload, Duration::from_secs(3600)).await;
        }
    });

    // Concurrent reads (10 parallel)
    group.throughput(Throughput::Elements(10));
    group.bench_function("10_parallel_reads", |b| {
        b.iter(|| {
            rt.block_on(async {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        let cache = cache.clone();
                        let key = make_cache_key(i % 1000);
                        tokio::spawn(async move {
                            let _: Option<TestPayload> = cache.get(&key).await.unwrap();
                        })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            });
        });
    });

    // Mixed read/write workload
    group.throughput(Throughput::Elements(20));
    group.bench_function("mixed_10_reads_10_writes", |b| {
        let mut write_index = 2000_usize;
        b.iter(|| {
            rt.block_on(async {
                let mut handles = Vec::with_capacity(20);

                // 10 reads
                for i in 0..10 {
                    let cache = cache.clone();
                    let key = make_cache_key(i % 1000);
                    handles.push(tokio::spawn(async move {
                        let _: Option<TestPayload> = cache.get(&key).await.unwrap();
                    }));
                }

                // 10 writes
                for i in 0..10 {
                    let cache = cache.clone();
                    let key = make_cache_key(write_index + i);
                    let payload = payload.clone();
                    handles.push(tokio::spawn(async move {
                        let _ = cache.set(&key, &payload, Duration::from_secs(3600)).await;
                    }));
                }

                for handle in handles {
                    let _ = handle.await;
                }
            });
            write_index = write_index.wrapping_add(10);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_cache_set,
    bench_cache_get,
    bench_cache_invalidate,
    bench_cache_activities,
    bench_cache_concurrent,
);
criterion_main!(benches);
