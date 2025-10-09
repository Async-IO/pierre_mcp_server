// ABOUTME: Unit tests for metrics extractor functionality
// ABOUTME: Validates metrics extractor behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Metrics extractor tests

#![allow(clippy::float_cmp)] // Test values are exact

use chrono::Utc;
use pierre_mcp_server::intelligence::{MetricType, SafeMetricExtractor};
use pierre_mcp_server::models::{Activity, SportType};

fn create_test_activity() -> Activity {
    Activity {
        id: "test_activity".to_string(),
        name: "Test Run".to_string(),
        sport_type: SportType::Run,
        start_date: Utc::now(),
        duration_seconds: 3600,         // 1 hour
        distance_meters: Some(10000.0), // 10km
        average_speed: Some(2.78),      // ~10 km/h
        max_speed: Some(3.33),
        average_heart_rate: Some(150),
        max_heart_rate: Some(180),
        elevation_gain: Some(100.0),
        calories: Some(500),
        average_power: Some(200),
        max_power: Some(300),
        temperature: None,
        provider: "test_provider".to_string(),
        steps: None,
        heart_rate_zones: None,
        normalized_power: None,
        power_zones: None,
        ftp: None,
        average_cadence: None,
        max_cadence: None,
        hrv_score: None,
        recovery_heart_rate: None,
        humidity: None,
        average_altitude: None,
        wind_speed: None,
        ground_contact_time: None,
        vertical_oscillation: None,
        stride_length: None,
        running_power: None,
        breathing_rate: None,
        spo2: None,
        training_stress_score: None,
        intensity_factor: None,
        suffer_score: None,
        time_series_data: None,
        start_latitude: None,
        start_longitude: None,
        city: None,
        region: None,
        country: None,
        trail_name: None,
    }
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
        {
            let mut activity = create_test_activity();
            activity.distance_meters = Some(9000.0);
            activity
        },
        {
            let mut activity = create_test_activity();
            activity.distance_meters = Some(10000.0);
            activity
        },
        {
            let mut activity = create_test_activity();
            activity.distance_meters = Some(11000.0);
            activity
        },
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
