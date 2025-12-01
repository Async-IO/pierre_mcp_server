// ABOUTME: Integration tests for VO2 max configuration and calculations
// ABOUTME: Tests extracted from embedded test module in src/configuration/vo2_max.rs
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::configuration::vo2_max::VO2MaxCalculator;

#[test]
fn test_vo2_max_calculator_creation() {
    let calc = VO2MaxCalculator::new(50.0, 50, 180, 0.85, 1.0);
    assert!((calc.vo2_max - 50.0).abs() < f64::EPSILON);
    assert!((calc.lactate_threshold - 0.85).abs() < f64::EPSILON);
}

#[test]
fn test_hr_zones_calculation() {
    let calc = VO2MaxCalculator::new(45.0, 50, 180, 0.85, 1.0);
    let zones = calc.calculate_hr_zones();

    // Verify zones are in ascending order
    assert!(zones.zone1_lower < zones.zone1_upper);
    assert!(zones.zone1_upper <= zones.zone2_lower);
    assert!(zones.zone2_upper <= zones.zone3_lower);
    assert!(zones.zone3_upper <= zones.zone4_lower);
    assert!(zones.zone4_upper <= zones.zone5_lower);

    // Verify zone 1 starts above resting HR
    assert!(zones.zone1_lower > calc.resting_hr);

    // Verify zone 5 goes to max HR
    assert_eq!(zones.zone5_upper, calc.max_hr);
}

#[test]
fn test_pace_zones_calculation() {
    let calc = VO2MaxCalculator::new(50.0, 50, 180, 0.85, 1.0);
    let paces = calc.calculate_pace_zones();

    // Verify pace ranges make sense (faster pace = lower seconds/km)
    assert!(paces.easy_pace_range.0 > paces.easy_pace_range.1);
    assert!(paces.threshold_pace_range.0 < paces.easy_pace_range.1);
    assert!(paces.vo2max_pace_range.0 < paces.threshold_pace_range.1);
}

#[test]
fn test_elite_vs_beginner_zones() {
    let elite = VO2MaxCalculator::new(65.0, 40, 180, 0.85, 1.0);
    let beginner = VO2MaxCalculator::new(35.0, 70, 180, 0.75, 1.0);

    let elite_zones = elite.calculate_hr_zones();
    let beginner_zones = beginner.calculate_hr_zones();

    // Elite should have zone 6
    assert!(elite_zones.zone6_lower.is_some());

    // Beginner should not have zone 6
    assert!(beginner_zones.zone6_lower.is_none());
}

#[test]
fn test_trimp_calculation() {
    let calc = VO2MaxCalculator::new(50.0, 50, 180, 0.85, 1.0);

    let trimp_male = calc.calculate_trimp(140, 30.0, "M");
    let trimp_female = calc.calculate_trimp(140, 30.0, "F");

    // TRIMP should be positive
    assert!(trimp_male > 0.0);
    assert!(trimp_female > 0.0);

    // Female TRIMP is typically lower due to gender factor
    assert!(trimp_female < trimp_male);
}
