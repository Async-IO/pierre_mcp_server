// ABOUTME: Criterion benchmarks for JSON serialization and deserialization
// ABOUTME: Measures serde_json performance for Activity, MCP messages, and API responses
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Criterion benchmarks for JSON serialization and deserialization.
//!
//! Measures `serde_json` performance for Activity models, MCP messages,
//! and API responses with various payload sizes.

#![allow(
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    missing_docs
)]

mod common;

use chrono::{Duration, Utc};
use common::fixtures::{generate_activities, ActivityBatchSize};
use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use pierre_mcp_server::intelligence::metrics::AdvancedMetrics;
use pierre_mcp_server::intelligence::training_load::{TrainingLoad, TssDataPoint};
use pierre_mcp_server::models::{Activity, SleepSession, SleepStage, SleepStageType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP-style tool result for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpToolResult {
    tool_name: String,
    success: bool,
    data: serde_json::Value,
    metadata: McpMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct McpMetadata {
    execution_time_ms: u64,
    cache_hit: bool,
    provider: String,
    tenant_id: String,
}

/// API response wrapper for benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: T,
    pagination: Option<PaginationInfo>,
    request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PaginationInfo {
    cursor: Option<String>,
    has_more: bool,
    total_count: Option<u64>,
}

/// Generate sleep sessions for serialization benchmarks
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
fn generate_sleep_sessions(count: usize) -> Vec<SleepSession> {
    let base_date = Utc::now();
    (0..count)
        .map(|index| {
            let days_ago = index as i64;
            let sleep_start = base_date - Duration::days(days_ago) - Duration::hours(8);
            let total_sleep_time = 360 + ((index * 17) % 180) as u32;
            let time_in_bed = total_sleep_time + 30;
            let stages = generate_sleep_stages(sleep_start, total_sleep_time, index);

            SleepSession {
                id: format!("bench_sleep_{index}"),
                start_time: sleep_start,
                end_time: sleep_start + Duration::minutes(i64::from(time_in_bed)),
                time_in_bed,
                total_sleep_time,
                sleep_efficiency: (total_sleep_time as f32 / time_in_bed as f32) * 100.0,
                sleep_score: Some(70.0 + ((index * 13) % 30) as f32),
                stages,
                hrv_during_sleep: Some(40.0 + ((index * 11) % 40) as f64),
                respiratory_rate: Some(14.0 + ((index * 3) % 6) as f32),
                temperature_variation: Some(0.5 + ((index * 7) % 10) as f32 / 10.0),
                wake_count: Some(2 + ((index * 3) % 5) as u32),
                sleep_onset_latency: Some(10 + ((index * 5) % 20) as u32),
                provider: "benchmark".to_owned(),
            }
        })
        .collect()
}

#[allow(clippy::cast_possible_truncation)]
fn generate_sleep_stages(
    start_time: chrono::DateTime<Utc>,
    total_minutes: u32,
    seed: usize,
) -> Vec<SleepStage> {
    let mut stages = Vec::new();
    let mut current_time = start_time;
    let mut remaining = total_minutes;
    let mut cycle = 0_usize;

    while remaining > 0 {
        let stage_types = [
            SleepStageType::Light,
            SleepStageType::Deep,
            SleepStageType::Light,
            SleepStageType::Rem,
            SleepStageType::Awake,
        ];

        for (i, stage_type) in stage_types.iter().enumerate() {
            if remaining == 0 {
                break;
            }

            let base_duration = calculate_stage_duration(*stage_type, seed, cycle, i);
            let duration = base_duration.min(remaining);

            stages.push(SleepStage {
                stage_type: *stage_type,
                start_time: current_time,
                duration_minutes: duration,
            });

            current_time += Duration::minutes(i64::from(duration));
            remaining = remaining.saturating_sub(duration);
        }
        cycle += 1;

        if cycle > 10 {
            break;
        }
    }

    stages
}

#[allow(clippy::cast_possible_truncation)]
const fn calculate_stage_duration(
    stage_type: SleepStageType,
    seed: usize,
    cycle: usize,
    i: usize,
) -> u32 {
    match stage_type {
        SleepStageType::Light => 20 + ((seed * 7 + cycle * 3) % 15) as u32,
        SleepStageType::Deep => {
            if cycle < 2 {
                25 + ((seed * 11 + cycle * 5) % 20) as u32
            } else {
                10 + ((seed * 11 + cycle * 5) % 10) as u32
            }
        }
        SleepStageType::Rem => 10 + (cycle * 5) as u32 + ((seed * 13) % 15) as u32,
        SleepStageType::Awake => 2 + ((seed * i + cycle) % 5) as u32,
    }
}

/// Benchmark Activity serialization
#[allow(clippy::cast_possible_truncation)]
fn bench_activity_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_activity");

    // Single activity
    let single = generate_activities(ActivityBatchSize::Small)
        .into_iter()
        .next()
        .unwrap();
    let serialized_single = serde_json::to_vec(&single).unwrap();

    group.throughput(Throughput::Bytes(serialized_single.len() as u64));
    group.bench_function("single", |b| {
        b.iter(|| serde_json::to_vec(black_box(&single)));
    });

    // Batch of 10
    let batch_10 = generate_activities(ActivityBatchSize::Small);
    let serialized_10 = serde_json::to_vec(&batch_10).unwrap();

    group.throughput(Throughput::Bytes(serialized_10.len() as u64));
    group.bench_function("batch_10", |b| {
        b.iter(|| serde_json::to_vec(black_box(&batch_10)));
    });

    // Batch of 100
    let batch_100 = generate_activities(ActivityBatchSize::Medium);
    let serialized_100 = serde_json::to_vec(&batch_100).unwrap();

    group.throughput(Throughput::Bytes(serialized_100.len() as u64));
    group.bench_function("batch_100", |b| {
        b.iter(|| serde_json::to_vec(black_box(&batch_100)));
    });

    // Pretty print (for debugging/logs)
    group.bench_function("single_pretty", |b| {
        b.iter(|| serde_json::to_string_pretty(black_box(&single)));
    });

    group.finish();
}

/// Benchmark Activity deserialization
#[allow(clippy::cast_possible_truncation)]
fn bench_activity_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("deserialize_activity");

    // Single activity
    let single = generate_activities(ActivityBatchSize::Small)
        .into_iter()
        .next()
        .unwrap();
    let serialized_single = serde_json::to_vec(&single).unwrap();

    group.throughput(Throughput::Bytes(serialized_single.len() as u64));
    group.bench_function("single", |b| {
        b.iter(|| serde_json::from_slice::<Activity>(black_box(&serialized_single)));
    });

    // Batch of 100
    let batch_100 = generate_activities(ActivityBatchSize::Medium);
    let serialized_100 = serde_json::to_vec(&batch_100).unwrap();

    group.throughput(Throughput::Bytes(serialized_100.len() as u64));
    group.bench_function("batch_100", |b| {
        b.iter(|| serde_json::from_slice::<Vec<Activity>>(black_box(&serialized_100)));
    });

    // From string (common API path)
    let serialized_string = serde_json::to_string(&batch_100).unwrap();
    group.bench_function("batch_100_from_str", |b| {
        b.iter(|| serde_json::from_str::<Vec<Activity>>(black_box(&serialized_string)));
    });

    group.finish();
}

/// Benchmark MCP tool result serialization
#[allow(clippy::cast_possible_truncation)]
fn bench_mcp_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_mcp");

    let activities = generate_activities(ActivityBatchSize::Small);
    let mcp_result = McpToolResult {
        tool_name: "get_activities".to_owned(),
        success: true,
        data: serde_json::to_value(&activities).unwrap(),
        metadata: McpMetadata {
            execution_time_ms: 150,
            cache_hit: true,
            provider: "strava".to_owned(),
            tenant_id: "tenant_123".to_owned(),
        },
    };

    let serialized = serde_json::to_vec(&mcp_result).unwrap();
    group.throughput(Throughput::Bytes(serialized.len() as u64));

    group.bench_function("tool_result_with_activities", |b| {
        b.iter(|| serde_json::to_vec(black_box(&mcp_result)));
    });

    // Large result (100 activities)
    let activities_large = generate_activities(ActivityBatchSize::Medium);
    let mcp_result_large = McpToolResult {
        tool_name: "get_activities".to_owned(),
        success: true,
        data: serde_json::to_value(&activities_large).unwrap(),
        metadata: McpMetadata {
            execution_time_ms: 250,
            cache_hit: false,
            provider: "strava".to_owned(),
            tenant_id: "tenant_123".to_owned(),
        },
    };

    let serialized_large = serde_json::to_vec(&mcp_result_large).unwrap();
    group.throughput(Throughput::Bytes(serialized_large.len() as u64));

    group.bench_function("tool_result_large", |b| {
        b.iter(|| serde_json::to_vec(black_box(&mcp_result_large)));
    });

    group.finish();
}

/// Benchmark API response serialization (paginated)
#[allow(clippy::cast_possible_truncation)]
fn bench_api_response_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_api_response");

    let activities = generate_activities(ActivityBatchSize::Medium);
    let response: ApiResponse<Vec<Activity>> = ApiResponse {
        success: true,
        data: activities,
        pagination: Some(PaginationInfo {
            cursor: Some("eyJpZCI6MTAwfQ==".to_owned()),
            has_more: true,
            total_count: Some(500),
        }),
        request_id: "req_abc123".to_owned(),
    };

    let serialized = serde_json::to_vec(&response).unwrap();
    group.throughput(Throughput::Bytes(serialized.len() as u64));

    group.bench_function("paginated_100_activities", |b| {
        b.iter(|| serde_json::to_vec(black_box(&response)));
    });

    group.finish();
}

/// Benchmark intelligence result serialization
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn bench_intelligence_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_intelligence");

    // Training load result
    let training_load = TrainingLoad {
        ctl: 75.5,
        atl: 82.3,
        tsb: -6.8,
        tss_history: (0..42)
            .map(|i| TssDataPoint {
                date: chrono::Utc::now() - chrono::Duration::days(i),
                tss: (i as f64).mul_add(1.5, 50.0),
            })
            .collect(),
    };

    let serialized = serde_json::to_vec(&training_load).unwrap();
    group.throughput(Throughput::Bytes(serialized.len() as u64));

    group.bench_function("training_load", |b| {
        b.iter(|| serde_json::to_vec(black_box(&training_load)));
    });

    // Advanced metrics
    let metrics = AdvancedMetrics {
        trimp: Some(125.5),
        aerobic_efficiency: Some(1.85),
        power_to_weight_ratio: Some(3.5),
        training_stress_score: Some(85.0),
        intensity_factor: Some(0.85),
        variability_index: Some(1.05),
        efficiency_factor: Some(1.42),
        decoupling_percentage: Some(3.2),
        normalized_power: Some(265.0),
        work: Some(850.0),
        avg_power_to_weight: Some(3.4),
        running_effectiveness: None,
        stride_efficiency: None,
        ground_contact_balance: None,
        estimated_recovery_time: Some(24.0),
        training_load: Some(75.0),
        aerobic_contribution: Some(85.0),
        temperature_stress: Some(1.05),
        altitude_adjustment: Some(0.98),
        custom_metrics: HashMap::new(),
    };

    let serialized_metrics = serde_json::to_vec(&metrics).unwrap();
    group.throughput(Throughput::Bytes(serialized_metrics.len() as u64));

    group.bench_function("advanced_metrics", |b| {
        b.iter(|| serde_json::to_vec(black_box(&metrics)));
    });

    group.finish();
}

/// Benchmark sleep session serialization
#[allow(clippy::cast_possible_truncation)]
fn bench_sleep_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialize_sleep");

    let sessions = generate_sleep_sessions(7);
    let serialized = serde_json::to_vec(&sessions).unwrap();

    group.throughput(Throughput::Bytes(serialized.len() as u64));

    group.bench_function("7_days_sleep_data", |b| {
        b.iter(|| serde_json::to_vec(black_box(&sessions)));
    });

    // 30 days
    let sessions_30 = generate_sleep_sessions(30);
    let serialized_30 = serde_json::to_vec(&sessions_30).unwrap();

    group.throughput(Throughput::Bytes(serialized_30.len() as u64));

    group.bench_function("30_days_sleep_data", |b| {
        b.iter(|| serde_json::to_vec(black_box(&sessions_30)));
    });

    group.finish();
}

/// Benchmark JSON value operations (dynamic typing)
fn bench_json_value_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_value");

    let activities = generate_activities(ActivityBatchSize::Medium);

    // Convert to Value (common for MCP)
    group.bench_function("to_value_100_activities", |b| {
        b.iter(|| serde_json::to_value(black_box(&activities)));
    });

    // From Value back to typed
    let value = serde_json::to_value(&activities).unwrap();
    group.bench_function("from_value_100_activities", |b| {
        b.iter(|| serde_json::from_value::<Vec<Activity>>(black_box(value.clone())));
    });

    // Value indexing (common in MCP handlers)
    let json_str = serde_json::to_string(&activities).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    group.bench_function("value_array_indexing", |b| {
        b.iter(|| {
            for i in 0..100 {
                let _ = black_box(&parsed[i]["name"]);
            }
        });
    });

    group.finish();
}

/// Benchmark roundtrip (serialize + deserialize)
#[allow(clippy::cast_possible_truncation)]
fn bench_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    let activities = generate_activities(ActivityBatchSize::Medium);
    let serialized = serde_json::to_vec(&activities).unwrap();

    group.throughput(Throughput::Bytes(serialized.len() as u64));

    group.bench_function("100_activities", |b| {
        b.iter(|| {
            let bytes = serde_json::to_vec(black_box(&activities)).unwrap();
            serde_json::from_slice::<Vec<Activity>>(&bytes).unwrap()
        });
    });

    // String roundtrip (more common in HTTP)
    group.bench_function("100_activities_string", |b| {
        b.iter(|| {
            let s = serde_json::to_string(black_box(&activities)).unwrap();
            serde_json::from_str::<Vec<Activity>>(&s).unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_activity_serialization,
    bench_activity_deserialization,
    bench_mcp_serialization,
    bench_api_response_serialization,
    bench_intelligence_serialization,
    bench_sleep_serialization,
    bench_json_value_operations,
    bench_roundtrip,
);
criterion_main!(benches);
