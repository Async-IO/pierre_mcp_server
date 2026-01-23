// ABOUTME: Criterion benchmarks for database operations using SQLite backend
// ABOUTME: Measures query performance for user operations, pagination, and multi-tenant isolation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Criterion benchmarks for database operations.
//!
//! Measures query performance for user operations, pagination,
//! and multi-tenant isolation using the `SQLite` backend.

#![allow(
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    missing_docs
)]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pierre_mcp_server::database::Database;
use pierre_mcp_server::models::{User, UserStatus, UserTier};
use pierre_mcp_server::pagination::{PaginationDirection, PaginationParams};
use pierre_mcp_server::permissions::UserRole;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Counter for unique user generation across benchmark iterations
static USER_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Benchmark admin UUID used for user approval
const BENCH_ADMIN_UUID: &str = "00000000-0000-0000-0000-000000000001";

/// Generate a unique test user for benchmarking
fn generate_test_user() -> User {
    let counter = USER_COUNTER.fetch_add(1, Ordering::SeqCst);
    let admin_id = Uuid::parse_str(BENCH_ADMIN_UUID).unwrap();
    User {
        id: Uuid::new_v4(),
        email: format!("bench_user_{counter}@example.com"),
        password_hash: "benchmark_hash_value".to_owned(),
        display_name: Some(format!("Bench User {counter}")),
        tier: UserTier::Starter,
        strava_token: None,
        fitbit_token: None,
        created_at: chrono::Utc::now(),
        last_active: chrono::Utc::now(),
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        role: UserRole::User,
        approved_by: Some(admin_id),
        approved_at: Some(chrono::Utc::now()),
        firebase_uid: None,
        auth_provider: "email".to_owned(),
    }
}

/// Create in-memory test database with encryption key
async fn create_test_db() -> Database {
    let encryption_key = vec![0u8; 32]; // Test key
    Database::new(":memory:", encryption_key).await.unwrap()
}

/// Benchmark user creation
fn bench_user_create(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_user_create");

    let db = rt.block_on(create_test_db());
    rt.block_on(db.migrate()).unwrap();

    group.bench_function("single_user", |b| {
        b.iter(|| {
            let user = generate_test_user();
            rt.block_on(async { db.create_user(black_box(&user)).await })
        });
    });

    // Batch creation (simulating registration spike)
    group.throughput(Throughput::Elements(10));
    group.bench_function("batch_10_users", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..10 {
                    let user = generate_test_user();
                    let _ = db.create_user(&user).await;
                }
            });
        });
    });

    group.finish();
}

/// Benchmark user lookup operations
fn bench_user_lookup(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_user_lookup");

    let db = rt.block_on(create_test_db());
    rt.block_on(db.migrate()).unwrap();

    // Pre-populate with users for lookup tests
    let mut user_ids = Vec::new();
    let mut user_emails = Vec::new();
    rt.block_on(async {
        for _ in 0..100 {
            let user = generate_test_user();
            user_ids.push(user.id);
            user_emails.push(user.email.clone());
            let _ = db.create_user(&user).await;
        }
    });

    // Lookup by ID
    group.bench_function("by_id", |b| {
        let mut index = 0;
        b.iter(|| {
            let id = user_ids[index % user_ids.len()];
            index += 1;
            rt.block_on(async { db.get_user(black_box(id)).await })
        });
    });

    // Lookup by email
    group.bench_function("by_email", |b| {
        let mut index = 0;
        b.iter(|| {
            let email = &user_emails[index % user_emails.len()];
            index += 1;
            rt.block_on(async { db.get_user_by_email(black_box(email)).await })
        });
    });

    // Lookup non-existent (miss case)
    group.bench_function("by_email_miss", |b| {
        b.iter(|| {
            rt.block_on(async {
                db.get_user_by_email(black_box("nonexistent@example.com"))
                    .await
            })
        });
    });

    group.finish();
}

/// Benchmark cursor-based pagination
#[allow(clippy::cast_possible_truncation)]
fn bench_pagination(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_pagination");

    let db = rt.block_on(create_test_db());
    rt.block_on(db.migrate()).unwrap();

    // Pre-populate with users for pagination
    rt.block_on(async {
        for _ in 0..500 {
            let user = generate_test_user();
            let _ = db.create_user(&user).await;
        }
    });

    // Pagination with different page sizes
    for page_size in [10_usize, 50, 100] {
        group.throughput(Throughput::Elements(page_size as u64));
        group.bench_with_input(
            BenchmarkId::new("first_page", page_size),
            &page_size,
            |b, &page_size| {
                let params = PaginationParams {
                    cursor: None,
                    limit: page_size,
                    direction: PaginationDirection::Forward,
                };
                b.iter(|| {
                    rt.block_on(async {
                        db.get_users_by_status_cursor(black_box("active"), black_box(&params))
                            .await
                    })
                });
            },
        );
    }

    // Deep pagination (later pages)
    group.bench_function("deep_pagination_page_5", |b| {
        // First, get the cursor for page 5
        let cursor = rt.block_on(async {
            let mut cursor = None;
            for _ in 0..4 {
                let params = PaginationParams {
                    cursor: cursor.clone(),
                    limit: 50,
                    direction: PaginationDirection::Forward,
                };
                let page = db
                    .get_users_by_status_cursor("active", &params)
                    .await
                    .unwrap();
                cursor = page.next_cursor;
            }
            cursor
        });

        let params = PaginationParams {
            cursor,
            limit: 50,
            direction: PaginationDirection::Forward,
        };
        b.iter(|| {
            rt.block_on(async {
                db.get_users_by_status_cursor(black_box("active"), black_box(&params))
                    .await
            })
        });
    });

    group.finish();
}

/// Benchmark user status updates
fn bench_user_update(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_user_update");

    let db = rt.block_on(create_test_db());
    rt.block_on(db.migrate()).unwrap();

    // Pre-populate
    let mut user_ids = Vec::new();
    rt.block_on(async {
        for _ in 0..100 {
            let user = generate_test_user();
            user_ids.push(user.id);
            let _ = db.create_user(&user).await;
        }
    });

    // Update last active (frequent operation)
    group.bench_function("update_last_active", |b| {
        let mut index = 0;
        b.iter(|| {
            let id = user_ids[index % user_ids.len()];
            index += 1;
            rt.block_on(async { db.update_last_active(black_box(id)).await })
        });
    });

    // Update status (admin operation)
    group.bench_function("update_status", |b| {
        let mut index = 0;
        b.iter(|| {
            let id = user_ids[index % user_ids.len()];
            index += 1;
            let status = if index % 2 == 0 {
                UserStatus::Active
            } else {
                UserStatus::Suspended
            };
            rt.block_on(async {
                db.update_user_status(black_box(id), black_box(status), black_box(None))
                    .await
            })
        });
    });

    group.finish();
}

/// Benchmark user count query (aggregation)
fn bench_aggregation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_aggregation");

    let db = rt.block_on(create_test_db());
    rt.block_on(db.migrate()).unwrap();

    // Test with different database sizes
    for user_count in [100, 500, 1000] {
        // Reset and populate
        let db = rt.block_on(create_test_db());
        rt.block_on(db.migrate()).unwrap();
        rt.block_on(async {
            for _ in 0..user_count {
                let user = generate_test_user();
                let _ = db.create_user(&user).await;
            }
        });

        group.bench_with_input(BenchmarkId::new("user_count", user_count), &db, |b, db| {
            b.iter(|| rt.block_on(async { db.get_user_count().await }));
        });
    }

    group.finish();
}

/// Benchmark concurrent database operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("database_concurrent");
    group.sample_size(30); // Reduce samples for concurrent tests

    let db = rt.block_on(create_test_db());
    rt.block_on(db.migrate()).unwrap();

    // Pre-populate
    let mut user_ids = Vec::new();
    rt.block_on(async {
        for _ in 0..100 {
            let user = generate_test_user();
            user_ids.push(user.id);
            let _ = db.create_user(&user).await;
        }
    });

    // Concurrent reads
    group.throughput(Throughput::Elements(10));
    group.bench_function("10_parallel_reads", |b| {
        b.iter(|| {
            rt.block_on(async {
                let handles: Vec<_> = (0..10)
                    .map(|i| {
                        let db = db.clone();
                        let id = user_ids[i % user_ids.len()];
                        tokio::spawn(async move { db.get_user(id).await })
                    })
                    .collect();

                for handle in handles {
                    let _ = handle.await;
                }
            });
        });
    });

    // Mixed read/write
    group.throughput(Throughput::Elements(20));
    group.bench_function("mixed_10_reads_10_writes", |b| {
        b.iter(|| {
            rt.block_on(async {
                let mut read_handles = Vec::with_capacity(10);
                let mut write_handles = Vec::with_capacity(10);

                // 10 reads
                for i in 0..10 {
                    let db = db.clone();
                    let id = user_ids[i % user_ids.len()];
                    read_handles.push(tokio::spawn(async move { db.get_user(id).await }));
                }

                // 10 writes (update last active)
                for i in 0..10 {
                    let db = db.clone();
                    let id = user_ids[i % user_ids.len()];
                    write_handles
                        .push(tokio::spawn(async move { db.update_last_active(id).await }));
                }

                for handle in read_handles {
                    let _ = handle.await;
                }
                for handle in write_handles {
                    let _ = handle.await;
                }
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_user_create,
    bench_user_lookup,
    bench_pagination,
    bench_user_update,
    bench_aggregation,
    bench_concurrent_operations,
);
criterion_main!(benches);
