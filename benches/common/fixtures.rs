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
    let is_run = sport_type == SportType::Run;
    let is_walk = sport_type == SportType::Walk;
    let is_ride = sport_type == SportType::Ride;

    Activity {
        id: format!("bench_activity_{index}"),
        name: format!("Benchmark Activity {index}"),
        sport_type,
        start_date,
        duration_seconds,
        distance_meters: Some(distance_meters),
        elevation_gain: Some(((index * 31) % 500) as f64),
        average_heart_rate: Some(avg_hr),
        max_heart_rate: Some(avg_hr + 25),
        average_speed: Some(distance_meters / duration_seconds as f64),
        max_speed: Some(distance_meters / duration_seconds as f64 * 1.5),
        calories: Some(((duration_seconds / 60) * 10) as u32),
        steps: if is_run || is_walk {
            Some((duration_seconds * 3) as u32)
        } else {
            None
        },
        heart_rate_zones: None,
        average_power: if is_ride {
            Some(200 + ((index * 13) % 100) as u32)
        } else {
            None
        },
        max_power: if is_ride {
            Some(350 + ((index * 17) % 150) as u32)
        } else {
            None
        },
        normalized_power: if is_ride {
            Some(210 + ((index * 11) % 90) as u32)
        } else {
            None
        },
        power_zones: None,
        ftp: if is_ride { Some(250) } else { None },
        average_cadence: Some(80 + ((index * 7) % 30) as u32),
        max_cadence: Some(100 + ((index * 9) % 40) as u32),
        hrv_score: Some(50.0 + ((index * 19) % 50) as f64),
        recovery_heart_rate: Some(40 + ((index * 3) % 30) as u32),
        temperature: Some(15.0 + ((index * 5) % 20) as f32),
        humidity: Some(50.0 + ((index * 7) % 40) as f32),
        average_altitude: Some(100.0 + ((index * 23) % 500) as f32),
        wind_speed: Some(((index * 11) % 20) as f32),
        ground_contact_time: if is_run {
            Some(250 + ((index * 13) % 50) as u32)
        } else {
            None
        },
        vertical_oscillation: if is_run {
            Some(8.0 + ((index * 3) % 4) as f32)
        } else {
            None
        },
        stride_length: if is_run {
            Some(1.0 + ((index * 5) % 50) as f32 / 100.0)
        } else {
            None
        },
        running_power: if is_run {
            Some(200 + ((index * 7) % 100) as u32)
        } else {
            None
        },
        breathing_rate: Some(15 + ((index * 3) % 15) as u32),
        spo2: Some(95.0 + ((index * 2) % 5) as f32),
        training_stress_score: Some(50.0 + ((index * 17) % 150) as f32),
        intensity_factor: Some(0.7 + ((index * 3) % 30) as f32 / 100.0),
        suffer_score: Some(50 + ((index * 11) % 150) as u32),
        time_series_data: None,
        start_latitude: Some(45.5017 + ((index * 7) % 100) as f64 / 10000.0),
        start_longitude: Some(-73.5673 + ((index * 11) % 100) as f64 / 10000.0),
        city: Some("Montreal".to_owned()),
        region: Some("Quebec".to_owned()),
        country: Some("Canada".to_owned()),
        trail_name: None,
        workout_type: Some((index % 4) as u32),
        sport_type_detail: None,
        segment_efforts: None,
        provider: "benchmark".to_owned(),
    }
}

/// Generate a batch of activities for benchmarking
#[must_use]
pub fn generate_activities(size: ActivityBatchSize) -> Vec<Activity> {
    let config = BenchmarkConfig::default();
    (0..size.count())
        .map(|i| generate_activity(i, &config))
        .collect()
}
