// ABOUTME: Tests for TOON format token efficiency telemetry
// ABOUTME: Validates token estimation, efficiency metrics, and format comparison
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, missing_docs)]

use pierre_mcp_server::formatters::{
    format_output, format_output_with_telemetry, OutputFormat, TokenEfficiencyMetrics,
};
use serde::Serialize;

#[derive(Serialize, Clone)]
struct TestActivity {
    id: String,
    name: String,
    distance_meters: f64,
    duration_seconds: u32,
    sport_type: String,
}

fn create_test_activities(count: usize) -> Vec<TestActivity> {
    (0..count)
        .map(|i| TestActivity {
            id: format!("activity_{i}"),
            name: format!("Morning Run {i}"),
            distance_meters: (i as f64).mul_add(100.0, 5000.0),
            duration_seconds: 1800 + (i as u32 * 60),
            sport_type: "running".to_owned(),
        })
        .collect()
}

mod token_estimation {
    use super::*;

    #[test]
    fn test_estimate_tokens_empty_string() {
        let tokens = TokenEfficiencyMetrics::estimate_tokens("");
        // Empty string should estimate as ~0 tokens (but we add 3 and divide by 4)
        assert_eq!(tokens, 0);
    }

    #[test]
    fn test_estimate_tokens_short_string() {
        // 4 characters should be ~1 token
        let tokens = TokenEfficiencyMetrics::estimate_tokens("test");
        assert_eq!(tokens, 1);
    }

    #[test]
    fn test_estimate_tokens_longer_string() {
        // 16 characters should be ~4 tokens
        let tokens = TokenEfficiencyMetrics::estimate_tokens("1234567890123456");
        assert_eq!(tokens, 4);
    }

    #[test]
    fn test_estimate_tokens_rounds_up() {
        // 5 characters should round up to 2 tokens
        let tokens = TokenEfficiencyMetrics::estimate_tokens("12345");
        assert_eq!(tokens, 2);
    }
}

mod efficiency_metrics {
    use super::*;

    #[test]
    fn test_metrics_for_json_format() {
        let activities = create_test_activities(5);
        let output = format_output(&activities, OutputFormat::Json).unwrap();
        let metrics = output.calculate_efficiency(&activities);

        assert_eq!(metrics.format_used, "json");
        assert!(metrics.byte_size > 0);
        assert!(metrics.estimated_tokens > 0);
        // JSON vs JSON should have no savings
        assert!(metrics.token_savings_percent < 1.0);
        assert!((metrics.compression_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_metrics_for_toon_format() {
        let activities = create_test_activities(5);
        let output = format_output(&activities, OutputFormat::Toon).unwrap();
        let metrics = output.calculate_efficiency(&activities);

        assert_eq!(metrics.format_used, "toon");
        assert!(metrics.byte_size > 0);
        assert!(metrics.estimated_tokens > 0);
        // TOON should show savings compared to JSON
        // Note: actual savings depend on data structure
        assert!(metrics.json_equivalent_size >= metrics.byte_size);
    }

    #[test]
    fn test_metrics_compression_ratio() {
        let activities = create_test_activities(10);
        let toon_output = format_output(&activities, OutputFormat::Toon).unwrap();
        let metrics = toon_output.calculate_efficiency(&activities);

        // TOON should have compression ratio >= 1.0 (same or smaller than JSON)
        assert!(
            metrics.compression_ratio >= 0.9,
            "Expected compression ratio >= 0.9, got {}",
            metrics.compression_ratio
        );
    }

    #[test]
    fn test_metrics_with_large_dataset() {
        let activities = create_test_activities(100);

        let json_output = format_output(&activities, OutputFormat::Json).unwrap();
        let toon_output = format_output(&activities, OutputFormat::Toon).unwrap();

        let json_metrics = json_output.calculate_efficiency(&activities);
        let toon_metrics = toon_output.calculate_efficiency(&activities);

        // Larger datasets should show meaningful token reduction with TOON
        assert!(toon_metrics.byte_size <= json_metrics.byte_size);
        assert!(toon_metrics.estimated_tokens <= json_metrics.estimated_tokens);
    }
}

mod format_output_with_telemetry_tests {
    use super::*;

    #[test]
    fn test_telemetry_function_returns_metrics() {
        let activities = create_test_activities(3);
        let result =
            format_output_with_telemetry(&activities, OutputFormat::Toon, "test_operation");

        assert!(result.is_ok());
        let (output, metrics) = result.unwrap();

        assert_eq!(output.format, OutputFormat::Toon);
        assert_eq!(metrics.format_used, "toon");
        assert!(metrics.estimated_tokens > 0);
    }

    #[test]
    fn test_telemetry_for_json_format() {
        let activities = create_test_activities(3);
        let result = format_output_with_telemetry(&activities, OutputFormat::Json, "json_test");

        assert!(result.is_ok());
        let (output, metrics) = result.unwrap();

        assert_eq!(output.format, OutputFormat::Json);
        assert_eq!(metrics.format_used, "json");
    }

    #[test]
    fn test_telemetry_metrics_consistency() {
        let activities = create_test_activities(5);

        // Compare direct calculation vs telemetry function
        let direct_output = format_output(&activities, OutputFormat::Toon).unwrap();
        let direct_metrics = direct_output.calculate_efficiency(&activities);

        let (telemetry_output, telemetry_metrics) =
            format_output_with_telemetry(&activities, OutputFormat::Toon, "consistency_test")
                .unwrap();

        assert_eq!(direct_output.data, telemetry_output.data);
        assert_eq!(direct_metrics.byte_size, telemetry_metrics.byte_size);
        assert_eq!(
            direct_metrics.estimated_tokens,
            telemetry_metrics.estimated_tokens
        );
    }
}

mod formatted_output_methods {
    use super::*;

    #[test]
    fn test_estimated_tokens_method() {
        let activities = create_test_activities(3);
        let output = format_output(&activities, OutputFormat::Json).unwrap();

        let tokens = output.estimated_tokens();
        let expected = TokenEfficiencyMetrics::estimate_tokens(&output.data);

        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_calculate_efficiency_method() {
        let activities = create_test_activities(3);
        let output = format_output(&activities, OutputFormat::Toon).unwrap();

        let metrics = output.calculate_efficiency(&activities);

        assert_eq!(metrics.format_used, "toon");
        assert_eq!(metrics.byte_size, output.data.len());
    }
}
