// ABOUTME: Unit tests for metrics extractor functionality
// ABOUTME: Validates metrics extractor behavior, edge cases, and error handling
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Metrics extractor tests

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]
#![allow(clippy::float_cmp)] // Test values are exact

use chrono::Utc;
use pierre_mcp_server::intelligence::{MetricType, SafeMetricExtractor};
use pierre_mcp_server::models::{Activity, SportType};

fn create_test_activity() -> Activity {
    use pierre_mcp_server::models::ActivityBuilder;

    ActivityBuilder::new(
        "test_activity",
        "Test Run",
        SportType::Run,
        Utc::now(),
        3600, // 1 hour
        "test_provider",
    )
    .distance_meters(10000.0) // 10km
    .average_speed(2.78) // ~10 km/h
    .max_speed(3.33)
    .average_heart_rate(150)
    .max_heart_rate(180)
    .elevation_gain(100.0)
    .calories(500)
    .average_power(200)
    .max_power(300)
    .build()
}

fn create_test_activity_with_distance(distance: f64) -> Activity {
    use pierre_mcp_server::models::ActivityBuilder;

    ActivityBuilder::new(
        "test_activity",
        "Test Run",
        SportType::Run,
        Utc::now(),
        3600, // 1 hour
        "test_provider",
    )
    .distance_meters(distance)
    .average_speed(2.78)
    .max_speed(3.33)
    .average_heart_rate(150)
    .max_heart_rate(180)
    .elevation_gain(100.0)
    .calories(500)
    .average_power(200)
    .max_power(300)
    .build()
}

#[test]
fn test_metric_type_extraction() {
    let activity = create_test_activity();

    assert_eq!(MetricType::Distance.extract_value(&activity), Some(10000.0));
    assert_eq!(MetricType::Duration.extract_value(&activity), Some(3600.0));
    assert_eq!(MetricType::HeartRate.extract_value(&activity), Some(150.0));
    assert_eq!(MetricType::Speed.extract_value(&activity), Some(2.78));
    assert_eq!(MetricType::Elevation.extract_value(&activity), Some(100.0));
    assert_eq!(MetricType::Power.extract_value(&activity), Some(200.0));
}

#[test]
fn test_metric_type_properties() {
    assert!(MetricType::Pace.is_lower_better());
    assert!(!MetricType::Speed.is_lower_better());
    assert!(!MetricType::Distance.is_lower_better());

    assert_eq!(MetricType::Distance.unit(), "meters");
    assert_eq!(MetricType::HeartRate.unit(), "bpm");
    assert_eq!(MetricType::Speed.unit(), "m/s");

    assert_eq!(MetricType::Distance.display_name(), "Distance");
    assert_eq!(MetricType::HeartRate.display_name(), "Heart Rate");
}

#[test]
fn test_safe_metric_extractor_success() {
    let activities = vec![create_test_activity()];
    let result = SafeMetricExtractor::extract_metric_values(&activities, MetricType::Distance);

    assert!(result.is_ok());
    let values = result.unwrap();
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].1, 10000.0);
}

#[test]
fn test_safe_metric_extractor_empty_activities() {
    let activities = vec![];
    let result = SafeMetricExtractor::extract_metric_values(&activities, MetricType::Distance);

    assert!(result.is_err());
}

#[test]
fn test_metric_summary_calculation() {
    let activities = vec![
        create_test_activity_with_distance(9000.0),
        create_test_activity_with_distance(10000.0),
        create_test_activity_with_distance(11000.0),
    ];

    let summary =
        SafeMetricExtractor::calculate_metric_summary(&activities, MetricType::Distance).unwrap();

    assert_eq!(summary.count, 3);
    assert_eq!(summary.min, 9000.0);
    assert_eq!(summary.max, 11000.0);
    assert_eq!(summary.mean, 10000.0);
    assert_eq!(summary.median, 10000.0);

    assert!(!summary.is_highly_variable()); // CV should be reasonable for this data
}
