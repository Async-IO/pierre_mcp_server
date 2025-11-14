// ABOUTME: Database performance benchmarks for refactoring baseline
// ABOUTME: Establishes performance baselines before code refactoring begins
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pierre_mcp_server::api_keys::ApiKey;
use pierre_mcp_server::database::Database;
use pierre_mcp_server::database_plugins::{
    postgres::PostgresDatabase, sqlite::SqliteDatabase, DatabaseProvider,
};
use pierre_mcp_server::models::{User, UserOAuthToken, UserStatus, UserTier};
use std::time::Duration;
use tokio::runtime::Runtime;
use uuid::Uuid;

/// Create test database with encryption key
async fn setup_sqlite_db() -> SqliteDatabase {
    let encryption_key = vec![0u8; 32]; // Test key
    SqliteDatabase::new(":memory:", encryption_key)
        .await
        .expect("Failed to create test database")
}

/// Create test user for benchmarks
fn create_test_user() -> User {
    User {
        id: Uuid::new_v4(),
        email: format!("test-{}@example.com", Uuid::new_v4()),
        display_name: Some("Test User".to_string()),
        password_hash: "$argon2id$v=19$m=19456,t=2,p=1$test".to_string(),
        tier: UserTier::Professional,
        tenant_id: Some("test-tenant".to_string()),
        strava_token: None,
        fitbit_token: None,
        is_active: true,
        user_status: UserStatus::Active,
        is_admin: false,
        approved_by: None,
        approved_at: None,
        created_at: Utc::now(),
        last_active: Utc::now(),
    }
}

/// Create test OAuth token
fn create_test_oauth_token(user_id: Uuid) -> UserOAuthToken {
    UserOAuthToken {
        id: Uuid::new_v4().to_string(),
        user_id,
        tenant_id: "test-tenant".to_string(),
        provider: "strava".to_string(),
        access_token: "test_access_token_".to_string() + &Uuid::new_v4().to_string(),
        refresh_token: Some("test_refresh_token".to_string()),
        token_type: "Bearer".to_string(),
        expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
        scope: Some("activity:read".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

/// Benchmark: create_user
fn bench_create_user(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(setup_sqlite_db());
    rt.block_on(db.migrate()).unwrap();

    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("create_user/sqlite", |b| {
        b.to_async(&rt).iter(|| async {
            let user = create_test_user();
            black_box(db.create_user(&user).await.unwrap())
        });
    });

    group.finish();
}

/// Benchmark: get_user (read operation)
fn bench_get_user(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(setup_sqlite_db());
    rt.block_on(db.migrate()).unwrap();

    // Pre-populate with test user
    let user = create_test_user();
    let user_id = rt.block_on(db.create_user(&user)).unwrap();

    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("get_user/sqlite", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(db.get_user(user_id).await.unwrap()) });
    });

    group.finish();
}

/// Benchmark: upsert_user_oauth_token (with encryption)
fn bench_upsert_oauth_token(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(setup_sqlite_db());
    rt.block_on(db.migrate()).unwrap();

    // Create user first
    let user = create_test_user();
    let user_id = rt.block_on(db.create_user(&user)).unwrap();

    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("upsert_user_oauth_token/sqlite", |b| {
        b.to_async(&rt).iter(|| async {
            let token = create_test_oauth_token(user_id);
            black_box(db.upsert_user_oauth_token(&token).await.unwrap())
        });
    });

    group.finish();
}

/// Benchmark: get_user_oauth_token (with decryption)
fn bench_get_oauth_token(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(setup_sqlite_db());
    rt.block_on(db.migrate()).unwrap();

    // Setup: Create user and token
    let user = create_test_user();
    let user_id = rt.block_on(db.create_user(&user)).unwrap();
    let token = create_test_oauth_token(user_id);
    rt.block_on(db.upsert_user_oauth_token(&token)).unwrap();

    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("get_user_oauth_token/sqlite", |b| {
        b.to_async(&rt).iter(|| async {
            black_box(
                db.get_user_oauth_token(user_id, "test-tenant", "strava")
                    .await
                    .unwrap(),
            )
        });
    });

    group.finish();
}

/// Benchmark: get_users_by_status (list operation with filtering)
fn bench_get_users_by_status(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(setup_sqlite_db());
    rt.block_on(db.migrate()).unwrap();

    // Pre-populate with 100 users
    for _ in 0..100 {
        let user = create_test_user();
        rt.block_on(db.create_user(&user)).unwrap();
    }

    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("get_users_by_status/sqlite", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(db.get_users_by_status("active").await.unwrap()) });
    });

    group.finish();
}

/// Benchmark: update_last_active (simple update)
fn bench_update_last_active(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let db = rt.block_on(setup_sqlite_db());
    rt.block_on(db.migrate()).unwrap();

    let user = create_test_user();
    let user_id = rt.block_on(db.create_user(&user)).unwrap();

    let mut group = c.benchmark_group("database_operations");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("update_last_active/sqlite", |b| {
        b.to_async(&rt)
            .iter(|| async { black_box(db.update_last_active(user_id).await.unwrap()) });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_create_user,
    bench_get_user,
    bench_upsert_oauth_token,
    bench_get_oauth_token,
    bench_get_users_by_status,
    bench_update_last_active,
);

criterion_main!(benches);
