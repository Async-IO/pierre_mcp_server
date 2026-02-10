// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence
//
// ABOUTME: Tests for the TimeSeriesVisitor pattern
// ABOUTME: Validates single-pass time series data processing

#![allow(clippy::expect_used)]
#![allow(missing_docs)]

use pierre_mcp_server::intelligence::visitor::{
    DecouplingDetector, NormalizedPowerCalculator, StatsCollector, TimeSeriesExt, ZoneBoundaries,
    ZoneTimeCalculator,
};
use pierre_mcp_server::models::TimeSeriesData;

fn create_test_time_series() -> TimeSeriesData {
    TimeSeriesData {
        timestamps: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        heart_rate: Some(vec![120, 125, 130, 135, 140, 145, 150, 155, 160, 165]),
        power: Some(vec![200, 210, 220, 230, 240, 250, 260, 270, 280, 290]),
        cadence: Some(vec![80, 82, 84, 86, 88, 90, 92, 94, 96, 98]),
        speed: Some(vec![3.0, 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 3.9]),
        altitude: Some(vec![
            100.0, 102.0, 105.0, 103.0, 101.0, 100.0, 98.0, 97.0, 99.0, 100.0,
        ]),
        temperature: Some(vec![
            20.0, 20.5, 21.0, 21.5, 22.0, 22.5, 23.0, 22.5, 22.0, 21.5,
        ]),
        gps_coordinates: Some(vec![
            (40.0, -74.0),
            (40.001, -74.001),
            (40.002, -74.002),
            (40.003, -74.003),
            (40.004, -74.004),
            (40.005, -74.005),
            (40.006, -74.006),
            (40.007, -74.007),
            (40.008, -74.008),
            (40.009, -74.009),
        ]),
    }
}

#[test]
fn test_stats_collector() {
    let time_series = create_test_time_series();
    let mut stats = StatsCollector::default();

    time_series.accept(&mut stats);

    // Heart rate: 120-165, avg = 142.5
    assert_eq!(stats.heart_rate.min, Some(120.0));
    assert_eq!(stats.heart_rate.max, Some(165.0));
    assert_eq!(stats.heart_rate.count, 10);
    let hr_avg = stats.heart_rate.average().expect("Should have HR average");
    assert!((hr_avg - 142.5).abs() < 0.01);

    // Power: 200-290, avg = 245
    assert_eq!(stats.power.min, Some(200.0));
    assert_eq!(stats.power.max, Some(290.0));
    let power_avg = stats.power.average().expect("Should have power average");
    assert!((power_avg - 245.0).abs() < 0.01);
}

#[test]
fn test_zone_time_calculator() {
    let time_series = TimeSeriesData {
        timestamps: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        heart_rate: Some(vec![100, 110, 120, 130, 140, 150, 160, 170, 180, 190]),
        power: None,
        cadence: None,
        speed: None,
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    let mut zone_calc = ZoneTimeCalculator::new(200, ZoneBoundaries::default());
    time_series.accept(&mut zone_calc);

    let distribution = zone_calc.zone_distribution();

    // With max HR 200:
    // Zone 1: <= 120 (60%) -> 100, 110, 120 = 3 points
    // Zone 2: <= 140 (70%) -> 130, 140 = 2 points
    // Zone 3: <= 160 (80%) -> 150, 160 = 2 points
    // Zone 4: <= 180 (90%) -> 170, 180 = 2 points
    // Zone 5: > 180 -> 190 = 1 point
    assert!(distribution.zone1_pct > 0.0);
    assert!(distribution.zone5_pct > 0.0);
    assert_eq!(distribution.total_seconds, 10);
}

#[test]
fn test_accept_all_multiple_visitors() {
    let time_series = create_test_time_series();
    let mut stats = StatsCollector::default();
    let mut zone_calc = ZoneTimeCalculator::new(200, ZoneBoundaries::default());

    time_series.accept_all(&mut [&mut stats, &mut zone_calc]);

    // Both visitors should have processed the data
    assert_eq!(stats.heart_rate.count, 10);
    assert!(zone_calc.zone_distribution().total_seconds > 0);
}

#[test]
fn test_empty_time_series() {
    let time_series = TimeSeriesData {
        timestamps: vec![],
        heart_rate: None,
        power: None,
        cadence: None,
        speed: None,
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    let mut stats = StatsCollector::default();
    time_series.accept(&mut stats);

    assert_eq!(stats.heart_rate.count, 0);
    assert!(stats.heart_rate.average().is_none());
}

#[test]
fn test_partial_data() {
    let time_series = TimeSeriesData {
        timestamps: vec![0, 1, 2, 3, 4],
        heart_rate: Some(vec![120, 130, 140]),
        power: None,
        cadence: None,
        speed: None,
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    let mut stats = StatsCollector::default();
    time_series.accept(&mut stats);

    // Only 3 HR values even though 5 timestamps
    assert_eq!(stats.heart_rate.count, 3);
    assert_eq!(stats.power.count, 0);
}

#[test]
fn test_decoupling_detector() {
    // Create data with increasing HR/speed ratio (decoupling)
    let time_series = TimeSeriesData {
        timestamps: (0..40).collect(),
        heart_rate: Some((120..160).collect()),
        power: None,
        cadence: None,
        speed: Some(vec![
            // First half: steady speed
            3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5,
            3.5, 3.5, 3.5, // Second half: same speed but HR increased
            3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5, 3.5,
            3.5, 3.5, 3.5,
        ]),
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    let mut detector = DecouplingDetector::default();
    time_series.accept(&mut detector);

    let decoupling = detector.decoupling_percentage();
    assert!(decoupling.is_some());
    // HR increased while speed stayed same, so positive decoupling
    let pct = decoupling.expect("Should have decoupling percentage");
    assert!(pct > 0.0);
}

#[test]
fn test_normalized_power_insufficient_data() {
    let time_series = TimeSeriesData {
        timestamps: (0..10).collect(),
        heart_rate: None,
        power: Some(vec![200, 210, 220, 230, 240, 250, 260, 270, 280, 290]),
        cadence: None,
        speed: None,
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    let mut np_calc = NormalizedPowerCalculator::default();
    time_series.accept(&mut np_calc);

    // Need 30+ data points for NP calculation
    assert!(np_calc.normalized_power().is_none());
}

#[test]
fn test_normalized_power_calculation() {
    // Create 60 seconds of power data
    let time_series = TimeSeriesData {
        timestamps: (0..60).collect(),
        heart_rate: None,
        power: Some(vec![250; 60]),
        cadence: None,
        speed: None,
        altitude: None,
        temperature: None,
        gps_coordinates: None,
    };

    let mut np_calc = NormalizedPowerCalculator::default();
    time_series.accept(&mut np_calc);

    let np = np_calc.normalized_power();
    assert!(np.is_some());
    // With constant power, NP should equal average power
    let np_val = np.expect("Should have normalized power");
    assert!((np_val - 250.0).abs() < 1.0);
}
