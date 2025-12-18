// ABOUTME: Criterion benchmarks for intelligence module algorithms
// ABOUTME: Measures performance of training load, metrics, and data processing
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Criterion benchmarks for intelligence module algorithms.
//!
//! Measures performance of training load calculations, metrics computation,
//! and fitness data processing pipelines.

#![allow(clippy::missing_docs_in_private_items, missing_docs)]

mod common;

use chrono::{Duration, Utc};
use common::fixtures::{generate_activities, ActivityBatchSize};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pierre_mcp_server::intelligence::metrics::MetricsCalculator;
use pierre_mcp_server::intelligence::sleep_analysis::SleepData;
use pierre_mcp_server::intelligence::training_load::TrainingLoadCalculator;
use pierre_mcp_server::models::{Activity, ActivityBuilder, SportType};

/// Large dataset size for stress testing (500 activities)
const LARGE_DATASET_SIZE: usize = 500;

/// Generate a custom number of activities for large dataset benchmarks
/// Local implementation to avoid `dead_code` warnings in other benchmarks
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
fn generate_activities_custom(count: usize) -> Vec<Activity> {
    let base_date = Utc::now();
    (0..count)
        .map(|index| {
            let sport_type = match index % 4 {
                0 => SportType::Run,
                1 => SportType::Ride,
                2 => SportType::Swim,
                _ => SportType::Walk,
            };
            let duration_seconds = 1800_u64 + ((index * 137) % 3600) as u64;
            let distance_meters = 5000.0 + ((index * 251) % 10000) as f64;
            let avg_hr = 130_u32 + ((index * 17) % 40) as u32;
            let days_ago = (index * 2) as i64;
            let start_date = base_date - Duration::days(days_ago);
            let is_run = sport_type == SportType::Run;
            let is_walk = sport_type == SportType::Walk;
            let is_ride = sport_type == SportType::Ride;

            {
                let mut builder = ActivityBuilder::new(
                    format!("bench_activity_{index}"),
                    format!("Benchmark Activity {index}"),
                    sport_type,
                    start_date,
                    duration_seconds,
                    "benchmark",
                )
                .distance_meters(distance_meters)
                .elevation_gain(((index * 31) % 500) as f64)
                .average_heart_rate(avg_hr)
                .max_heart_rate(avg_hr + 25)
                .average_speed(distance_meters / duration_seconds as f64)
                .max_speed(distance_meters / duration_seconds as f64 * 1.5)
                .calories(((duration_seconds / 60) * 10) as u32)
                .average_cadence(80 + ((index * 7) % 30) as u32)
                .max_cadence(100 + ((index * 9) % 40) as u32)
                .hrv_score(50.0 + ((index * 19) % 50) as f64)
                .recovery_heart_rate(40 + ((index * 3) % 30) as u32)
                .temperature(15.0 + ((index * 5) % 20) as f32)
                .humidity(50.0 + ((index * 7) % 40) as f32)
                .average_altitude(100.0 + ((index * 23) % 500) as f32)
                .wind_speed(((index * 11) % 20) as f32)
                .breathing_rate(15 + ((index * 3) % 15) as u32)
                .spo2(95.0 + ((index * 2) % 5) as f32)
                .training_stress_score(50.0 + ((index * 17) % 150) as f32)
                .intensity_factor(0.7 + ((index * 3) % 30) as f32 / 100.0)
                .suffer_score(50 + ((index * 11) % 150) as u32)
                .start_latitude(45.5017 + ((index * 7) % 100) as f64 / 10000.0)
                .start_longitude(-73.5673 + ((index * 11) % 100) as f64 / 10000.0)
                .city("Montreal".to_owned())
                .region("Quebec".to_owned())
                .country("Canada".to_owned())
                .workout_type((index % 4) as u32);

                // Conditional fields based on sport type
                if is_run || is_walk {
                    builder = builder.steps((duration_seconds * 3) as u32);
                }
                if is_ride {
                    builder = builder
                        .average_power(200 + ((index * 13) % 100) as u32)
                        .max_power(350 + ((index * 17) % 150) as u32)
                        .normalized_power(210 + ((index * 11) % 90) as u32)
                        .ftp(250);
                }
                if is_run {
                    builder = builder
                        .ground_contact_time(250 + ((index * 13) % 50) as u32)
                        .vertical_oscillation(8.0 + ((index * 3) % 4) as f32)
                        .stride_length(1.0 + ((index * 5) % 50) as f32 / 100.0)
                        .running_power(200 + ((index * 7) % 100) as u32);
                }

                builder.build()
            }
        })
        .collect()
}

/// Generate sleep data for intelligence benchmarks
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
fn generate_sleep_data(count: usize) -> Vec<SleepData> {
    let base_date = Utc::now();
    (0..count)
        .map(|index| {
            let days_ago = index as i64;
            let sleep_date = base_date - Duration::days(days_ago);
            let duration_hours = 6.0 + ((index * 17) % 180) as f64 / 60.0;

            SleepData {
                date: sleep_date,
                duration_hours,
                deep_sleep_hours: Some(duration_hours * 0.2 + ((index * 3) % 30) as f64 / 100.0),
                rem_sleep_hours: Some(duration_hours * 0.2 + ((index * 5) % 30) as f64 / 100.0),
                light_sleep_hours: Some(duration_hours * 0.5),
                awake_hours: Some(0.3 + ((index * 2) % 30) as f64 / 100.0),
                efficiency_percent: Some(85.0 + ((index * 7) % 15) as f64),
                hrv_rmssd_ms: Some(40.0 + ((index * 11) % 40) as f64),
                resting_hr_bpm: Some(55 + ((index * 3) % 15) as u32),
                provider_score: Some(70.0 + ((index * 13) % 30) as f64),
            }
        })
        .collect()
}

/// Benchmark training load calculations with varying dataset sizes
#[allow(clippy::cast_possible_truncation)]
fn bench_training_load_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("training_load");

    let datasets = [
        (10, generate_activities(ActivityBatchSize::Small)),
        (100, generate_activities(ActivityBatchSize::Medium)),
        (
            LARGE_DATASET_SIZE,
            generate_activities_custom(LARGE_DATASET_SIZE),
        ),
    ];

    for (count, activities) in datasets {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("calculate_training_load", count),
            &activities,
            |b, activities| {
                let calculator = TrainingLoadCalculator::new();
                b.iter(|| {
                    calculator.calculate_training_load(
                        black_box(activities),
                        black_box(Some(250.0)), // FTP
                        black_box(Some(165.0)), // LTHR
                        black_box(Some(185.0)), // Max HR
                        black_box(Some(50.0)),  // Resting HR
                        black_box(Some(70.0)),  // Weight
                    )
                });
            },
        );
    }

    group.finish();
}

/// Benchmark TSS calculation for individual activities
#[allow(clippy::cast_possible_truncation)]
fn bench_tss_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tss_calculation");

    let activities = generate_activities(ActivityBatchSize::Medium);

    group.bench_function("single_activity_tss", |b| {
        let calculator = TrainingLoadCalculator::new();
        let activity = &activities[0];
        b.iter(|| {
            calculator.calculate_tss(
                black_box(activity),
                black_box(Some(250.0)), // FTP
                black_box(Some(165.0)), // LTHR
                black_box(Some(185.0)), // Max HR
                black_box(Some(50.0)),  // Resting HR
                black_box(Some(70.0)),  // Weight
            )
        });
    });

    group.throughput(Throughput::Elements(activities.len() as u64));
    group.bench_function("batch_tss_100_activities", |b| {
        let calculator = TrainingLoadCalculator::new();
        b.iter(|| {
            for activity in black_box(&activities) {
                let _ = calculator.calculate_tss(
                    activity,
                    Some(250.0),
                    Some(165.0),
                    Some(185.0),
                    Some(50.0),
                    Some(70.0),
                );
            }
        });
    });

    group.finish();
}

/// Benchmark advanced metrics calculation
#[allow(clippy::cast_possible_truncation)]
fn bench_metrics_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("metrics");

    let activities = generate_activities(ActivityBatchSize::Medium);

    group.bench_function("single_activity_metrics", |b| {
        let calculator = MetricsCalculator {
            ftp: Some(250.0),
            lthr: Some(165.0),
            max_hr: Some(185.0),
            resting_hr: Some(50.0),
            weight_kg: Some(70.0),
        };
        let activity = &activities[0];
        b.iter(|| calculator.calculate_metrics(black_box(activity)));
    });

    group.throughput(Throughput::Elements(activities.len() as u64));
    group.bench_function("batch_metrics_100_activities", |b| {
        let calculator = MetricsCalculator {
            ftp: Some(250.0),
            lthr: Some(165.0),
            max_hr: Some(185.0),
            resting_hr: Some(50.0),
            weight_kg: Some(70.0),
        };
        b.iter(|| {
            for activity in black_box(&activities) {
                let _ = calculator.calculate_metrics(activity);
            }
        });
    });

    group.finish();
}

/// Benchmark sleep data processing (generation and iteration)
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn bench_sleep_data_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sleep_data");

    let sleep_data = generate_sleep_data(30);

    group.throughput(Throughput::Elements(sleep_data.len() as u64));
    group.bench_function("iterate_30_nights", |b| {
        b.iter(|| {
            let mut total_hours = 0.0;
            for sleep in black_box(&sleep_data) {
                total_hours += sleep.duration_hours;
            }
            total_hours
        });
    });

    // Benchmark HRV data extraction
    group.bench_function("extract_hrv_values", |b| {
        b.iter(|| {
            let hrv_values: Vec<f64> = black_box(&sleep_data)
                .iter()
                .filter_map(|s| s.hrv_rmssd_ms)
                .collect();
            hrv_values
        });
    });

    // Benchmark sleep efficiency calculation
    group.bench_function("calculate_avg_efficiency", |b| {
        b.iter(|| {
            let efficiencies: Vec<f64> = black_box(&sleep_data)
                .iter()
                .filter_map(|s| s.efficiency_percent)
                .collect();
            if efficiencies.is_empty() {
                0.0
            } else {
                efficiencies.iter().sum::<f64>() / efficiencies.len() as f64
            }
        });
    });

    group.finish();
}

/// Benchmark combined training metrics pipeline
fn bench_training_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("training_pipeline");
    group.sample_size(50);

    let activities = generate_activities(ActivityBatchSize::Medium);

    group.bench_function("full_training_analysis", |b| {
        b.iter(|| {
            // 1. Calculate training load
            let tl_calc = TrainingLoadCalculator::new();
            let _training_load = tl_calc.calculate_training_load(
                black_box(&activities),
                Some(250.0),
                Some(165.0),
                Some(185.0),
                Some(50.0),
                Some(70.0),
            );

            // 2. Calculate metrics for each activity
            let metrics_calc = MetricsCalculator {
                ftp: Some(250.0),
                lthr: Some(165.0),
                max_hr: Some(185.0),
                resting_hr: Some(50.0),
                weight_kg: Some(70.0),
            };
            let _metrics: Vec<_> = activities
                .iter()
                .map(|a| metrics_calc.calculate_metrics(a))
                .collect();
        });
    });

    group.finish();
}

/// Benchmark activity data aggregation
#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn bench_activity_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("activity_aggregation");

    let datasets = [
        (10, generate_activities(ActivityBatchSize::Small)),
        (100, generate_activities(ActivityBatchSize::Medium)),
        (
            LARGE_DATASET_SIZE,
            generate_activities_custom(LARGE_DATASET_SIZE),
        ),
    ];

    for (count, activities) in datasets {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("aggregate_stats", count),
            &activities,
            |b, activities| {
                b.iter(|| {
                    let total_distance: f64 = black_box(activities)
                        .iter()
                        .filter_map(Activity::distance_meters)
                        .sum();
                    let total_duration: u64 =
                        activities.iter().map(Activity::duration_seconds).sum();
                    let avg_hr: f64 = {
                        let hrs: Vec<u32> = activities
                            .iter()
                            .filter_map(Activity::average_heart_rate)
                            .collect();
                        if hrs.is_empty() {
                            0.0
                        } else {
                            hrs.iter().map(|&h| f64::from(h)).sum::<f64>() / hrs.len() as f64
                        }
                    };
                    (total_distance, total_duration, avg_hr)
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_training_load_calculation,
    bench_tss_calculation,
    bench_metrics_calculation,
    bench_sleep_data_processing,
    bench_training_pipeline,
    bench_activity_aggregation,
);
criterion_main!(benches);
