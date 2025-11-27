// ABOUTME: Unit tests for training_load module
// ABOUTME: Tests training load calculations and TSB analysis with comprehensive coverage
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::{DateTime, Duration, Utc};
use pierre_mcp_server::intelligence::{
    RiskLevel, TrainingLoad, TrainingLoadCalculator, TrainingStatus,
};
use pierre_mcp_server::models::Activity;

fn create_test_activity(
    date: DateTime<Utc>,
    duration_seconds: u32,
    avg_power: Option<u32>,
    avg_hr: Option<u32>,
) -> Activity {
    Activity {
        id: format!("test_{}", date.timestamp()),
        name: "Test Activity".to_owned(),
        sport_type: pierre_mcp_server::models::SportType::Run,
        start_date: date,
        duration_seconds: u64::from(duration_seconds),
        distance_meters: Some(10000.0),
        average_power: avg_power,
        average_heart_rate: avg_hr,
        elevation_gain: None,
        max_heart_rate: None,
        average_speed: None,
        max_speed: None,
        calories: None,
        steps: None,
        heart_rate_zones: None,
        max_power: None,
        normalized_power: None,
        power_zones: None,
        ftp: None,
        average_cadence: None,
        max_cadence: None,
        hrv_score: None,
        recovery_heart_rate: None,
        temperature: None,
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
        workout_type: None,
        sport_type_detail: None,
        segment_efforts: None,
        provider: "test".to_owned(),
    }
}

#[test]
fn test_calculate_tsb() {
    let ctl = 100.0;
    let atl = 80.0;
    let tsb = TrainingLoadCalculator::calculate_tsb(ctl, atl);
    assert!((tsb - 20.0).abs() < f64::EPSILON, "TSB should be 20.0");
}

#[test]
fn test_interpret_tsb() {
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-15.0),
        TrainingStatus::Overreaching
    );
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(-5.0),
        TrainingStatus::Productive
    );
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(5.0),
        TrainingStatus::Fresh
    );
    assert_eq!(
        TrainingLoadCalculator::interpret_tsb(15.0),
        TrainingStatus::Detraining
    );
}

#[test]
fn test_recommend_recovery_days() {
    assert_eq!(TrainingLoadCalculator::recommend_recovery_days(-25.0), 5);
    assert_eq!(TrainingLoadCalculator::recommend_recovery_days(-18.0), 3);
    assert_eq!(TrainingLoadCalculator::recommend_recovery_days(-12.0), 2);
    assert_eq!(TrainingLoadCalculator::recommend_recovery_days(-5.0), 1);
    assert_eq!(TrainingLoadCalculator::recommend_recovery_days(5.0), 0);
}

#[test]
fn test_empty_activities() {
    let calculator = TrainingLoadCalculator::new();
    let result = calculator
        .calculate_training_load(&[], Some(250.0), None, Some(180.0), Some(60.0), Some(70.0))
        .unwrap();

    assert!(result.ctl.abs() < f64::EPSILON, "CTL should be 0.0");
    assert!(result.atl.abs() < f64::EPSILON, "ATL should be 0.0");
    assert!(result.tsb.abs() < f64::EPSILON, "TSB should be 0.0");
}

#[test]
fn test_training_load_with_power() {
    let calculator = TrainingLoadCalculator::new();
    let now = Utc::now();

    let activities = vec![
        create_test_activity(now - Duration::days(2), 3600, Some(200), None),
        create_test_activity(now - Duration::days(1), 3600, Some(220), None),
        create_test_activity(now, 3600, Some(210), None),
    ];

    let result = calculator
        .calculate_training_load(
            &activities,
            Some(250.0), // FTP
            None,
            None,
            None,
            Some(70.0),
        )
        .unwrap();

    // Should have calculated CTL and ATL
    assert!(result.ctl > 0.0);
    assert!(result.atl > 0.0);
    assert_eq!(result.tss_history.len(), 3);
}

#[test]
fn test_overtraining_risk_detection() {
    let high_risk = TrainingLoad {
        ctl: 80.0,
        atl: 150.0, // Very high ATL
        tsb: -70.0, // Deep fatigue
        tss_history: Vec::new(),
    };

    let risk = TrainingLoadCalculator::check_overtraining_risk(&high_risk);
    assert_eq!(risk.risk_level, RiskLevel::High);
    assert!(risk.risk_factors.len() >= 2);

    let low_risk = TrainingLoad {
        ctl: 90.0,
        atl: 80.0,
        tsb: 10.0,
        tss_history: Vec::new(),
    };

    let risk = TrainingLoadCalculator::check_overtraining_risk(&low_risk);
    assert_eq!(risk.risk_level, RiskLevel::Low);
}

/// Test that EMA calculation works correctly regardless of input order.
/// This verifies the fix for the bug where newest-first data (as returned by APIs)
/// would cause CTL/ATL/TSB to incorrectly return 0.
#[test]
fn test_ema_calculation_order_independent() {
    let calculator = TrainingLoadCalculator::new();
    let now = Utc::now();

    // Create activities spanning 14 days with varying intensity
    let activities_oldest_first = vec![
        create_test_activity(now - Duration::days(13), 3600, Some(180), Some(140)),
        create_test_activity(now - Duration::days(12), 4200, Some(200), Some(145)),
        create_test_activity(now - Duration::days(10), 3000, Some(190), Some(142)),
        create_test_activity(now - Duration::days(8), 5400, Some(210), Some(150)),
        create_test_activity(now - Duration::days(6), 3600, Some(195), Some(143)),
        create_test_activity(now - Duration::days(4), 4800, Some(205), Some(148)),
        create_test_activity(now - Duration::days(2), 3600, Some(185), Some(138)),
        create_test_activity(now, 4200, Some(200), Some(145)),
    ];

    // Same activities but in newest-first order (as APIs typically return)
    let activities_newest_first = vec![
        create_test_activity(now, 4200, Some(200), Some(145)),
        create_test_activity(now - Duration::days(2), 3600, Some(185), Some(138)),
        create_test_activity(now - Duration::days(4), 4800, Some(205), Some(148)),
        create_test_activity(now - Duration::days(6), 3600, Some(195), Some(143)),
        create_test_activity(now - Duration::days(8), 5400, Some(210), Some(150)),
        create_test_activity(now - Duration::days(10), 3000, Some(190), Some(142)),
        create_test_activity(now - Duration::days(12), 4200, Some(200), Some(145)),
        create_test_activity(now - Duration::days(13), 3600, Some(180), Some(140)),
    ];

    let result_oldest_first = calculator
        .calculate_training_load(
            &activities_oldest_first,
            Some(250.0),
            Some(165.0),
            Some(190.0),
            Some(55.0),
            Some(75.0),
        )
        .unwrap();

    let result_newest_first = calculator
        .calculate_training_load(
            &activities_newest_first,
            Some(250.0),
            Some(165.0),
            Some(190.0),
            Some(55.0),
            Some(75.0),
        )
        .unwrap();

    // Results should be identical regardless of input order
    assert!(
        (result_oldest_first.ctl - result_newest_first.ctl).abs() < 0.001,
        "CTL should be the same regardless of input order: oldest_first={}, newest_first={}",
        result_oldest_first.ctl,
        result_newest_first.ctl
    );

    assert!(
        (result_oldest_first.atl - result_newest_first.atl).abs() < 0.001,
        "ATL should be the same regardless of input order: oldest_first={}, newest_first={}",
        result_oldest_first.atl,
        result_newest_first.atl
    );

    assert!(
        (result_oldest_first.tsb - result_newest_first.tsb).abs() < 0.001,
        "TSB should be the same regardless of input order: oldest_first={}, newest_first={}",
        result_oldest_first.tsb,
        result_newest_first.tsb
    );

    // Both should have non-zero values (the bug would cause zeros)
    assert!(
        result_newest_first.ctl > 0.0,
        "CTL should not be zero for newest-first input: {}",
        result_newest_first.ctl
    );
    assert!(
        result_newest_first.atl > 0.0,
        "ATL should not be zero for newest-first input: {}",
        result_newest_first.atl
    );
}

/// Test that EMA calculation produces non-zero results for typical API response order.
/// APIs like Strava return activities newest-first. This test ensures CTL/ATL are calculated.
#[test]
fn test_ema_with_api_typical_order() {
    let calculator = TrainingLoadCalculator::new();
    let now = Utc::now();

    // Simulate typical API response: newest activity first
    let activities_newest_first = vec![
        create_test_activity(now, 3600, Some(200), Some(145)),
        create_test_activity(now - Duration::days(1), 3600, Some(195), Some(142)),
        create_test_activity(now - Duration::days(2), 4200, Some(210), Some(150)),
        create_test_activity(now - Duration::days(4), 3600, Some(190), Some(140)),
        create_test_activity(now - Duration::days(5), 5400, Some(220), Some(155)),
    ];

    let result = calculator
        .calculate_training_load(
            &activities_newest_first,
            Some(250.0),
            Some(165.0),
            Some(190.0),
            Some(55.0),
            Some(75.0),
        )
        .unwrap();

    // CTL and ATL must be positive for a training athlete
    assert!(
        result.ctl > 0.0,
        "CTL must be positive for newest-first data: {}",
        result.ctl
    );
    assert!(
        result.atl > 0.0,
        "ATL must be positive for newest-first data: {}",
        result.atl
    );

    // TSS history should contain all activities
    assert_eq!(
        result.tss_history.len(),
        5,
        "Should have TSS for all 5 activities"
    );
}
