// ABOUTME: Verification tests comparing VDOT predictions against Jack Daniels' VDOT tables
// ABOUTME: Tests accuracy of performance prediction implementation with known reference values
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use pierre_mcp_server::intelligence::PerformancePredictor;

/// Test VDOT 50 predictions against Jack Daniels' VDOT tables
///
/// Reference values from Jack Daniels' Running Formula (3rd Edition):
/// VDOT 50:
/// - 5K: 19:31 (1171 seconds)
/// - 10K: 40:31 (2431 seconds)
/// - Half Marathon: 1:30:00 (5400 seconds)
/// - Marathon: 3:08:00 (11280 seconds)
#[test]
fn test_vdot_50_against_daniels_tables() {
    let vdot = 50.0;

    // Get predictions from our implementation
    let time_5k = PerformancePredictor::predict_time_vdot(vdot, 5_000.0).unwrap();
    let time_10k = PerformancePredictor::predict_time_vdot(vdot, 10_000.0).unwrap();
    let time_half = PerformancePredictor::predict_time_vdot(vdot, 21_097.5).unwrap();
    let time_marathon = PerformancePredictor::predict_time_vdot(vdot, 42_195.0).unwrap();

    // Jack Daniels' reference times for VDOT 50
    let reference_5k = 1171.0;
    let reference_10k = 2431.0;
    let reference_half = 5400.0;
    let reference_marathon = 11_280.0;

    // Calculate percentage differences
    let diff_5k = ((time_5k - reference_5k).abs() / reference_5k) * 100.0;
    let diff_10k = ((time_10k - reference_10k).abs() / reference_10k) * 100.0;
    let diff_half = ((time_half - reference_half).abs() / reference_half) * 100.0;
    let diff_marathon = ((time_marathon - reference_marathon).abs() / reference_marathon) * 100.0;

    println!("\n=== VDOT 50 Verification ===");
    println!(
        "5K: {} (expected 19:31) - {:.1}% diff",
        PerformancePredictor::format_time(time_5k),
        diff_5k
    );
    println!(
        "10K: {} (expected 40:31) - {:.1}% diff",
        PerformancePredictor::format_time(time_10k),
        diff_10k
    );
    println!(
        "Half: {} (expected 1:30:00) - {:.1}% diff",
        PerformancePredictor::format_time(time_half),
        diff_half
    );
    println!(
        "Marathon: {} (expected 3:08:00) - {:.1}% diff",
        PerformancePredictor::format_time(time_marathon),
        diff_marathon
    );

    // All predictions should be within 6% of reference values
    // Note: Jack Daniels' tables use empirical adjustments, while our implementation
    // uses pure mathematical formulas. 6% tolerance is acceptable for race predictions.
    assert!(diff_5k < 6.0, "5K prediction off by {diff_5k:.1}%");
    assert!(diff_10k < 6.0, "10K prediction off by {diff_10k:.1}%");
    assert!(
        diff_half < 6.0,
        "Half marathon prediction off by {diff_half:.1}%"
    );
    assert!(
        diff_marathon < 6.0,
        "Marathon prediction off by {diff_marathon:.1}%"
    );
}

/// Test VDOT 60 predictions (faster runner)
///
/// Reference values for VDOT 60:
/// - 5K: 16:39 (999 seconds)
/// - 10K: 34:40 (2080 seconds)
/// - Marathon: 2:40:00 (9600 seconds)
#[test]
fn test_vdot_60_against_daniels_tables() {
    let vdot = 60.0;

    let time_5k = PerformancePredictor::predict_time_vdot(vdot, 5_000.0).unwrap();
    let time_10k = PerformancePredictor::predict_time_vdot(vdot, 10_000.0).unwrap();
    let time_marathon = PerformancePredictor::predict_time_vdot(vdot, 42_195.0).unwrap();

    let reference_5k = 999.0;
    let reference_10k = 2080.0;
    let reference_marathon = 9600.0;

    let diff_5k = ((time_5k - reference_5k).abs() / reference_5k) * 100.0;
    let diff_10k = ((time_10k - reference_10k).abs() / reference_10k) * 100.0;
    let diff_marathon = ((time_marathon - reference_marathon).abs() / reference_marathon) * 100.0;

    println!("\n=== VDOT 60 Verification ===");
    println!(
        "5K: {} (expected 16:39) - {:.1}% diff",
        PerformancePredictor::format_time(time_5k),
        diff_5k
    );
    println!(
        "10K: {} (expected 34:40) - {:.1}% diff",
        PerformancePredictor::format_time(time_10k),
        diff_10k
    );
    println!(
        "Marathon: {} (expected 2:40:00) - {:.1}% diff",
        PerformancePredictor::format_time(time_marathon),
        diff_marathon
    );

    assert!(diff_5k < 6.0, "5K prediction off by {diff_5k:.1}%");
    assert!(diff_10k < 6.0, "10K prediction off by {diff_10k:.1}%");
    assert!(
        diff_marathon < 6.0,
        "Marathon prediction off by {diff_marathon:.1}%"
    );
}

/// Test VDOT 40 predictions (recreational runner)
///
/// Reference values for VDOT 40:
/// - 5K: 24:44 (1484 seconds)
/// - 10K: 51:42 (3102 seconds)
/// - Marathon: 3:57:00 (14220 seconds)
#[test]
fn test_vdot_40_against_daniels_tables() {
    let vdot = 40.0;

    let time_5k = PerformancePredictor::predict_time_vdot(vdot, 5_000.0).unwrap();
    let time_10k = PerformancePredictor::predict_time_vdot(vdot, 10_000.0).unwrap();
    let time_marathon = PerformancePredictor::predict_time_vdot(vdot, 42_195.0).unwrap();

    let reference_5k = 1484.0;
    let reference_10k = 3102.0;
    let reference_marathon = 14_220.0;

    let diff_5k = ((time_5k - reference_5k).abs() / reference_5k) * 100.0;
    let diff_10k = ((time_10k - reference_10k).abs() / reference_10k) * 100.0;
    let diff_marathon = ((time_marathon - reference_marathon).abs() / reference_marathon) * 100.0;

    println!("\n=== VDOT 40 Verification ===");
    println!(
        "5K: {} (expected 24:44) - {:.1}% diff",
        PerformancePredictor::format_time(time_5k),
        diff_5k
    );
    println!(
        "10K: {} (expected 51:42) - {:.1}% diff",
        PerformancePredictor::format_time(time_10k),
        diff_10k
    );
    println!(
        "Marathon: {} (expected 3:57:00) - {:.1}% diff",
        PerformancePredictor::format_time(time_marathon),
        diff_marathon
    );

    assert!(diff_5k < 6.0, "5K prediction off by {diff_5k:.1}%");
    assert!(diff_10k < 6.0, "10K prediction off by {diff_10k:.1}%");
    assert!(
        diff_marathon < 6.0,
        "Marathon prediction off by {diff_marathon:.1}%"
    );
}
