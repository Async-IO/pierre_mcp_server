// ABOUTME: Unit tests for training_load module
// ABOUTME: Tests training load calculations and TSB analysis with comprehensive coverage
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use chrono::{DateTime, Duration, Utc};
use pierre_mcp_server::intelligence::{
    RiskLevel, TrainingLoad, TrainingLoadCalculator, TrainingStatus,
};
use pierre_mcp_server::models::{Activity, SportType};

fn create_test_activity(
    date: DateTime<Utc>,
    duration_seconds: u32,
    avg_power: Option<u32>,
    avg_hr: Option<u32>,
) -> Activity {
    use pierre_mcp_server::models::ActivityBuilder;

    let mut builder = ActivityBuilder::new(
        format!("test_{}", date.timestamp()),
        "Test Activity",
        SportType::Run,
        date,
        u64::from(duration_seconds),
        "test",
    )
    .distance_meters(10000.0);

    if let Some(power) = avg_power {
        builder = builder.average_power(power);
    }
    if let Some(hr) = avg_hr {
        builder = builder.average_heart_rate(hr);
    }

    builder.build()
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
