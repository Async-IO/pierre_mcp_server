// ABOUTME: Benchmark test fixtures for generating realistic fitness data
// ABOUTME: Provides deterministic data generation for reproducible performance measurements
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Benchmark test fixtures for generating realistic fitness data.
//!
//! Provides deterministic data generation for reproducible performance measurements.

use chrono::{DateTime, Duration, Utc};
use pierre_mcp_server::models::{Activity, SportType};

/// Predefined batch sizes for benchmark scenarios
#[derive(Debug, Clone, Copy)]
pub enum ActivityBatchSize {
    /// Small dataset (10 activities) - quick benchmarks
    Small,
    /// Medium dataset (100 activities) - typical user
    Medium,
}

impl ActivityBatchSize {
    #[must_use]
    pub const fn count(self) -> usize {
        match self {
            Self::Small => 10,
            Self::Medium => 100,
        }
    }
}

/// Configuration for benchmark test fixtures (internal use only)
#[derive(Debug, Clone)]
struct BenchmarkConfig {
    /// Base date for activity generation (activities go backwards from here)
    base_date: DateTime<Utc>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            base_date: Utc::now(),
        }
    }
}

/// Generate a single activity for benchmarking (internal use only)
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap
)]
fn generate_activity(index: usize, config: &BenchmarkConfig) -> Activity {
    let sport_type = determine_sport_type(index);
    let duration_seconds = calculate_duration(index);
    let distance_meters = calculate_distance(index);
    let avg_hr = calculate_heart_rate(index);
    let days_ago = (index * 2) as i64;
    let start_date = config.base_date - Duration::days(days_ago);

    build_activity(
        index,
        sport_type,
        start_date,
        duration_seconds,
        distance_meters,
        avg_hr,
    )
}

const fn determine_sport_type(index: usize) -> SportType {
    match index % 4 {
        0 => SportType::Run,
        1 => SportType::Ride,
        2 => SportType::Swim,
        _ => SportType::Walk,
    }
}

#[allow(clippy::cast_possible_truncation)]
const fn calculate_duration(index: usize) -> u64 {
    let base_duration = 1800_u64; // 30 minutes base
    let duration_variation = ((index * 137) % 3600) as u64;
    base_duration + duration_variation
}

#[allow(clippy::cast_precision_loss)]
const fn calculate_distance(index: usize) -> f64 {
    let base_distance = 5000.0; // 5km base
    let distance_variation = ((index * 251) % 10000) as f64;
    base_distance + distance_variation
}

#[allow(clippy::cast_possible_truncation)]
const fn calculate_heart_rate(index: usize) -> u32 {
    let base_hr = 130_u32;
    let hr_variation = ((index * 17) % 40) as u32;
    base_hr + hr_variation
}

#[allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
fn build_activity(
    index: usize,
    sport_type: SportType,
    start_date: DateTime<Utc>,
    duration_seconds: u64,
    distance_meters: f64,
    avg_hr: u32,
) -> Activity {
    use pierre_mcp_server::models::ActivityBuilder;

    let is_run = sport_type == SportType::Run;
    let is_walk = sport_type == SportType::Walk;
    let is_ride = sport_type == SportType::Ride;

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

/// Generate a batch of activities for benchmarking
#[must_use]
pub fn generate_activities(size: ActivityBatchSize) -> Vec<Activity> {
    let config = BenchmarkConfig::default();
    (0..size.count())
        .map(|i| generate_activity(i, &config))
        .collect()
}
