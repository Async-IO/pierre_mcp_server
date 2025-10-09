// ABOUTME: Unit tests for statistical analysis functionality
// ABOUTME: Validates statistical analysis behavior, edge cases, and error handling
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

// Statistical analysis module tests

#![allow(clippy::cast_possible_wrap)] // Test helper uses small indices

use chrono::Utc;
use pierre_mcp_server::intelligence::{StatisticalAnalyzer, TrendDataPoint};

fn create_test_data_points(values: Vec<f64>) -> Vec<TrendDataPoint> {
    values
        .into_iter()
        .enumerate()
        .map(|(i, value)| TrendDataPoint {
            date: Utc::now() + chrono::Duration::days(i as i64),
            value,
            smoothed_value: None,
        })
        .collect()
}

#[test]
fn test_linear_regression_perfect_positive_correlation() {
    let data_points = create_test_data_points(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let result = StatisticalAnalyzer::linear_regression(&data_points).unwrap();

    assert!((result.slope - 1.0).abs() < 0.001);
    assert!((result.correlation - 1.0).abs() < 0.001);
    assert!((result.r_squared - 1.0).abs() < 0.001);
}

#[test]
fn test_linear_regression_perfect_negative_correlation() {
    let data_points = create_test_data_points(vec![5.0, 4.0, 3.0, 2.0, 1.0]);
    let result = StatisticalAnalyzer::linear_regression(&data_points).unwrap();

    assert!((result.slope - (-1.0)).abs() < 0.001);
    assert!((result.correlation - (-1.0)).abs() < 0.001);
    assert!((result.r_squared - 1.0).abs() < 0.001);
}

#[test]
fn test_trend_strength_calculation() {
    let data_points = create_test_data_points(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
    let trend_strength = StatisticalAnalyzer::calculate_trend_strength(&data_points).unwrap();

    assert!((trend_strength - 1.0).abs() < 0.001);
}

#[test]
fn test_insufficient_data_points() {
    let data_points = create_test_data_points(vec![1.0]);
    let result = StatisticalAnalyzer::linear_regression(&data_points);

    assert!(result.is_err());
}
