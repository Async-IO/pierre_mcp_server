// ABOUTME: Unit tests for analysis config functionality
// ABOUTME: Validates analysis config behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Analysis configuration tests

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::intelligence::{AnalysisConfig, ConfidenceLevel};

#[test]
fn test_default_config_validation() {
    let config = AnalysisConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_invalid_weights_sum() {
    let mut config = AnalysisConfig::default();
    config.fitness_scoring.aerobic_weight = 0.8;
    config.fitness_scoring.strength_weight = 0.8;
    config.fitness_scoring.consistency_weight = 0.8;

    assert!(config.validate().is_err());
}

#[test]
fn test_confidence_level_calculation() {
    let config = AnalysisConfig::default();

    // High confidence
    assert_eq!(
        config.calculate_confidence_level(0.8, 25),
        ConfidenceLevel::High
    );

    // Medium confidence
    assert_eq!(
        config.calculate_confidence_level(0.5, 15),
        ConfidenceLevel::Medium
    );

    // Low confidence
    assert_eq!(
        config.calculate_confidence_level(0.2, 5),
        ConfidenceLevel::Low
    );
}

#[test]
fn test_environment_variable_override() {
    std::env::set_var("INTELLIGENCE_FITNESS_SCORE_WEEKS", "8");
    std::env::set_var("INTELLIGENCE_HIGH_R_SQUARED_THRESHOLD", "0.75");

    let config = AnalysisConfig::from_environment().unwrap();

    assert_eq!(config.timeframes.fitness_score_weeks, 8);
    assert!((config.confidence.high_r_squared - 0.75).abs() < 0.001);

    // Clean up
    std::env::remove_var("INTELLIGENCE_FITNESS_SCORE_WEEKS");
    std::env::remove_var("INTELLIGENCE_HIGH_R_SQUARED_THRESHOLD");
}
