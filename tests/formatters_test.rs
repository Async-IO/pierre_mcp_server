// ABOUTME: Integration tests for the formatters module
// ABOUTME: Tests JSON and TOON output format serialization
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use pierre_mcp_server::formatters::{format_output, OutputFormat};
use serde::Serialize;

#[derive(Serialize)]
struct TestActivity {
    id: String,
    name: String,
    distance_meters: f64,
}

#[test]
fn test_output_format_from_str() {
    assert_eq!(OutputFormat::from_str_param("json"), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str_param("JSON"), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str_param("toon"), OutputFormat::Toon);
    assert_eq!(OutputFormat::from_str_param("TOON"), OutputFormat::Toon);
    assert_eq!(OutputFormat::from_str_param("Toon"), OutputFormat::Toon);
    // Unknown defaults to JSON
    assert_eq!(OutputFormat::from_str_param("xml"), OutputFormat::Json);
    assert_eq!(OutputFormat::from_str_param(""), OutputFormat::Json);
}

#[test]
fn test_output_format_content_type() {
    assert_eq!(OutputFormat::Json.content_type(), "application/json");
    assert_eq!(OutputFormat::Toon.content_type(), "application/vnd.toon");
}

#[test]
fn test_format_json() {
    let activity = TestActivity {
        id: "123".to_owned(),
        name: "Morning Run".to_owned(),
        distance_meters: 5000.0,
    };

    let result = format_output(&activity, OutputFormat::Json);
    assert!(result.is_ok());
    let output = result.expect("JSON format should succeed");
    assert_eq!(output.format, OutputFormat::Json);
    assert!(output.data.contains("Morning Run"));
    assert!(output.data.contains("5000"));
}

#[test]
fn test_format_toon() {
    let activity = TestActivity {
        id: "123".to_owned(),
        name: "Morning Run".to_owned(),
        distance_meters: 5000.0,
    };

    let result = format_output(&activity, OutputFormat::Toon);
    assert!(result.is_ok());
    let output = result.expect("TOON format should succeed");
    assert_eq!(output.format, OutputFormat::Toon);
    // TOON output should contain the data
    assert!(output.data.contains("Morning Run"));
}

#[test]
fn test_format_activity_list_toon() {
    // Test with a list of activities - this is the key use case for TOON
    // TOON excels at uniform arrays, collapsing them into CSV-like rows
    let activities = vec![
        TestActivity {
            id: "1".to_owned(),
            name: "Run 1".to_owned(),
            distance_meters: 5000.0,
        },
        TestActivity {
            id: "2".to_owned(),
            name: "Run 2".to_owned(),
            distance_meters: 10000.0,
        },
        TestActivity {
            id: "3".to_owned(),
            name: "Run 3".to_owned(),
            distance_meters: 7500.0,
        },
    ];

    let json_result = format_output(&activities, OutputFormat::Json);
    let toon_result = format_output(&activities, OutputFormat::Toon);

    assert!(json_result.is_ok());
    assert!(toon_result.is_ok());

    let json_output = json_result.expect("JSON format should succeed");
    let toon_output = toon_result.expect("TOON format should succeed");

    // TOON should be more compact for uniform arrays (~40% reduction)
    println!("JSON length: {}", json_output.data.len());
    println!("TOON length: {}", toon_output.data.len());
    assert!(
        toon_output.data.len() < json_output.data.len(),
        "TOON ({} bytes) should be smaller than JSON ({} bytes) for uniform arrays",
        toon_output.data.len(),
        json_output.data.len()
    );

    // Both should contain all the data
    assert!(toon_output.data.contains("Run 1"));
    assert!(toon_output.data.contains("Run 2"));
    assert!(toon_output.data.contains("Run 3"));
}

#[test]
fn test_default_format() {
    assert_eq!(OutputFormat::default(), OutputFormat::Json);
}
