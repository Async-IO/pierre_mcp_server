// ABOUTME: Unit tests for performance_prediction module
// ABOUTME: Tests VDOT calculations and race predictions with comprehensive coverage
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::intelligence::PerformancePredictor;

// Test constants (matching values from performance_prediction.rs)
const DISTANCE_5K: f64 = 5_000.0;
const DISTANCE_10K: f64 = 10_000.0;
const DISTANCE_MARATHON: f64 = 42_195.0;

#[test]
fn test_calculate_vdot_from_10k() {
    // 10K in 40 minutes = VDOT ~50
    let vdot = PerformancePredictor::calculate_vdot(10_000.0, 40.0 * 60.0).unwrap();
    assert!(
        (48.0..=52.0).contains(&vdot),
        "VDOT should be around 50, got {vdot}"
    );
}

#[test]
fn test_calculate_vdot_from_5k() {
    // 5K in 19:30 (19.5 minutes) = VDOT ~50
    let vdot = PerformancePredictor::calculate_vdot(5_000.0, 19.5 * 60.0).unwrap();
    assert!(
        (48.0..=52.0).contains(&vdot),
        "VDOT should be around 50, got {vdot}"
    );
}

#[test]
fn test_predict_time_vdot() {
    // VDOT 50 predicts 5K around 19:30-20:00
    let predicted = PerformancePredictor::predict_time_vdot(50.0, DISTANCE_5K).unwrap();
    let predicted_minutes = predicted / 60.0;
    assert!(
        (19.0..=20.5).contains(&predicted_minutes),
        "5K time should be 19:00-20:30 for VDOT 50, got {predicted_minutes} minutes"
    );
}

#[test]
fn test_riegel_formula() {
    // 10K in 40 minutes predicts marathon around 3:05-3:15
    let marathon_time =
        PerformancePredictor::predict_time_riegel(DISTANCE_10K, 40.0 * 60.0, DISTANCE_MARATHON)
            .unwrap();
    let marathon_hours = marathon_time / 3600.0;
    assert!(
        (3.0..=3.3).contains(&marathon_hours),
        "Marathon should be 3:00-3:20 from 40 min 10K, got {marathon_hours:.2} hours"
    );
}

#[test]
fn test_generate_race_predictions() {
    // 10K in 40 minutes
    let predictions =
        PerformancePredictor::generate_race_predictions(10_000.0, 40.0 * 60.0).unwrap();

    assert!(
        (48.0..=52.0).contains(&predictions.vdot),
        "VDOT should be around 50, got {}",
        predictions.vdot
    );
    assert!(predictions.predictions.contains_key("5K"));
    assert!(predictions.predictions.contains_key("Marathon"));

    // 5K should be faster than 10K
    let time_5k = predictions.predictions.get("5K").unwrap();
    assert!(*time_5k < 45.0 * 60.0);

    // Marathon should be slower
    let time_marathon = predictions.predictions.get("Marathon").unwrap();
    assert!(*time_marathon > 45.0 * 60.0);
}

#[test]
fn test_format_time() {
    assert_eq!(PerformancePredictor::format_time(125.0), "2:05");
    assert_eq!(PerformancePredictor::format_time(3665.0), "1:01:05");
    assert_eq!(PerformancePredictor::format_time(45.0 * 60.0), "45:00");
}

#[test]
fn test_format_pace() {
    // 5 m/s = 3:20 min/km
    let pace = PerformancePredictor::format_pace_per_km(5.0);
    assert_eq!(pace, "3:20");

    // 3.33 m/s = 5:00 min/km
    let pace = PerformancePredictor::format_pace_per_km(3.33);
    assert_eq!(pace, "5:00");
}

#[test]
fn test_invalid_inputs() {
    // Zero time should error
    assert!(PerformancePredictor::calculate_vdot(10_000.0, 0.0).is_err());

    // Negative distance should error
    assert!(PerformancePredictor::calculate_vdot(-100.0, 600.0).is_err());

    // Unrealistic pace (too fast) should error
    assert!(PerformancePredictor::calculate_vdot(10_000.0, 60.0).is_err());
}

#[test]
fn test_riegel_vs_vdot_consistency() {
    // Both methods should give reasonably similar predictions
    let distance_10k = 10_000.0;
    let time_10k = 45.0 * 60.0;

    let vdot = PerformancePredictor::calculate_vdot(distance_10k, time_10k).unwrap();
    let vdot_5k = PerformancePredictor::predict_time_vdot(vdot, DISTANCE_5K).unwrap();
    let riegel_5k =
        PerformancePredictor::predict_time_riegel(distance_10k, time_10k, DISTANCE_5K).unwrap();

    // Should be within 5% of each other
    let diff_percent = ((vdot_5k - riegel_5k).abs() / vdot_5k) * 100.0;
    assert!(
        diff_percent < 5.0,
        "VDOT and Riegel predictions should be similar (diff: {diff_percent:.1}%)"
    );
}
