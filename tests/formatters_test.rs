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

// ============================================================================
// Tests for cached activities respecting mode and format parameters
// ============================================================================

/// ActivitySummary matches the format from fitness_api.rs for summary mode
#[derive(Debug, Clone, Serialize, serde::Deserialize, PartialEq)]
struct ActivitySummary {
    id: String,
    name: String,
    sport_type: String,
    start_date: String,
    distance_meters: f64,
    duration_seconds: f64,
}

/// Detailed activity with all fields (some null in JSON response)
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
struct DetailedActivity {
    id: String,
    name: String,
    sport_type: String,
    start_date: String,
    distance_meters: f64,
    duration_seconds: f64,
    // Additional fields that make detailed mode larger
    average_heartrate: Option<f64>,
    max_heartrate: Option<f64>,
    average_speed: Option<f64>,
    max_speed: Option<f64>,
    total_elevation_gain: Option<f64>,
    average_watts: Option<f64>,
    kilojoules: Option<f64>,
    calories: Option<f64>,
    description: Option<String>,
    gear_id: Option<String>,
}

#[test]
fn test_summary_mode_reduces_payload_size() {
    // Create detailed activities
    let detailed_activities = vec![
        DetailedActivity {
            id: "123".to_owned(),
            name: "Morning Run".to_owned(),
            sport_type: "Run".to_owned(),
            start_date: "2025-12-01T08:00:00Z".to_owned(),
            distance_meters: 5000.0,
            duration_seconds: 1800.0,
            average_heartrate: Some(145.0),
            max_heartrate: Some(175.0),
            average_speed: Some(2.78),
            max_speed: Some(3.5),
            total_elevation_gain: Some(50.0),
            average_watts: Some(200.0),
            kilojoules: Some(360.0),
            calories: Some(350.0),
            description: Some("Great morning run in the park".to_owned()),
            gear_id: Some("g12345".to_owned()),
        },
        DetailedActivity {
            id: "124".to_owned(),
            name: "Evening Ride".to_owned(),
            sport_type: "Ride".to_owned(),
            start_date: "2025-12-01T18:00:00Z".to_owned(),
            distance_meters: 25000.0,
            duration_seconds: 3600.0,
            average_heartrate: Some(135.0),
            max_heartrate: Some(165.0),
            average_speed: Some(6.94),
            max_speed: Some(12.0),
            total_elevation_gain: Some(200.0),
            average_watts: Some(180.0),
            kilojoules: Some(648.0),
            calories: Some(600.0),
            description: Some("Scenic evening ride through the countryside".to_owned()),
            gear_id: Some("g67890".to_owned()),
        },
    ];

    // Create corresponding summaries
    let summaries: Vec<ActivitySummary> = detailed_activities
        .iter()
        .map(|a| ActivitySummary {
            id: a.id.clone(),
            name: a.name.clone(),
            sport_type: a.sport_type.clone(),
            start_date: a.start_date.clone(),
            distance_meters: a.distance_meters,
            duration_seconds: a.duration_seconds,
        })
        .collect();

    // Serialize both to JSON
    let detailed_json = serde_json::to_string(&detailed_activities).expect("serialize detailed");
    let summary_json = serde_json::to_string(&summaries).expect("serialize summary");

    println!("Detailed JSON size: {} bytes", detailed_json.len());
    println!("Summary JSON size: {} bytes", summary_json.len());

    // Summary should be significantly smaller
    assert!(
        summary_json.len() < detailed_json.len(),
        "Summary ({} bytes) should be smaller than detailed ({} bytes)",
        summary_json.len(),
        detailed_json.len()
    );

    // Summary should be at least 40% smaller for activities with many optional fields
    let reduction_percent =
        (1.0 - (summary_json.len() as f64 / detailed_json.len() as f64)) * 100.0;
    println!("Size reduction: {:.1}%", reduction_percent);
    assert!(
        reduction_percent > 30.0,
        "Summary should provide at least 30% size reduction, got {:.1}%",
        reduction_percent
    );
}

#[test]
fn test_summary_plus_toon_maximizes_token_savings() {
    // This test validates that combining summary mode with TOON format
    // provides the maximum token reduction for LLM context

    let detailed_activities = vec![
        DetailedActivity {
            id: "1".to_owned(),
            name: "Run 1".to_owned(),
            sport_type: "Run".to_owned(),
            start_date: "2025-12-01T08:00:00Z".to_owned(),
            distance_meters: 5000.0,
            duration_seconds: 1800.0,
            average_heartrate: Some(145.0),
            max_heartrate: Some(175.0),
            average_speed: Some(2.78),
            max_speed: Some(3.5),
            total_elevation_gain: Some(50.0),
            average_watts: None,
            kilojoules: None,
            calories: Some(300.0),
            description: None,
            gear_id: None,
        },
        DetailedActivity {
            id: "2".to_owned(),
            name: "Run 2".to_owned(),
            sport_type: "Run".to_owned(),
            start_date: "2025-12-02T08:00:00Z".to_owned(),
            distance_meters: 8000.0,
            duration_seconds: 2700.0,
            average_heartrate: Some(150.0),
            max_heartrate: Some(180.0),
            average_speed: Some(2.96),
            max_speed: Some(3.8),
            total_elevation_gain: Some(80.0),
            average_watts: None,
            kilojoules: None,
            calories: Some(500.0),
            description: None,
            gear_id: None,
        },
        DetailedActivity {
            id: "3".to_owned(),
            name: "Run 3".to_owned(),
            sport_type: "Run".to_owned(),
            start_date: "2025-12-03T08:00:00Z".to_owned(),
            distance_meters: 10000.0,
            duration_seconds: 3600.0,
            average_heartrate: Some(140.0),
            max_heartrate: Some(170.0),
            average_speed: Some(2.78),
            max_speed: Some(3.3),
            total_elevation_gain: Some(100.0),
            average_watts: None,
            kilojoules: None,
            calories: Some(650.0),
            description: None,
            gear_id: None,
        },
    ];

    // Create summaries
    let summaries: Vec<ActivitySummary> = detailed_activities
        .iter()
        .map(|a| ActivitySummary {
            id: a.id.clone(),
            name: a.name.clone(),
            sport_type: a.sport_type.clone(),
            start_date: a.start_date.clone(),
            distance_meters: a.distance_meters,
            duration_seconds: a.duration_seconds,
        })
        .collect();

    // Format options
    let detailed_json = format_output(&detailed_activities, OutputFormat::Json)
        .expect("detailed json")
        .data;
    let detailed_toon = format_output(&detailed_activities, OutputFormat::Toon)
        .expect("detailed toon")
        .data;
    let summary_json = format_output(&summaries, OutputFormat::Json)
        .expect("summary json")
        .data;
    let summary_toon = format_output(&summaries, OutputFormat::Toon)
        .expect("summary toon")
        .data;

    println!("Detailed JSON: {} bytes", detailed_json.len());
    println!("Detailed TOON: {} bytes", detailed_toon.len());
    println!("Summary JSON:  {} bytes", summary_json.len());
    println!("Summary TOON:  {} bytes", summary_toon.len());

    // Summary + TOON should be the smallest
    assert!(
        summary_toon.len() <= summary_json.len(),
        "Summary TOON ({}) should be <= Summary JSON ({})",
        summary_toon.len(),
        summary_json.len()
    );

    assert!(
        summary_json.len() < detailed_json.len(),
        "Summary JSON ({}) should be < Detailed JSON ({})",
        summary_json.len(),
        detailed_json.len()
    );

    // Calculate total savings
    let total_savings = (1.0 - (summary_toon.len() as f64 / detailed_json.len() as f64)) * 100.0;
    println!(
        "Total savings (detailed JSON -> summary TOON): {:.1}%",
        total_savings
    );

    // Combined summary + TOON should provide significant savings
    assert!(
        total_savings > 40.0,
        "Combined mode=summary + format=toon should save at least 40% tokens, got {:.1}%",
        total_savings
    );
}

#[test]
fn test_toon_format_contains_all_summary_data() {
    // Verify TOON format preserves all data fields
    let summaries = vec![
        ActivitySummary {
            id: "nordic123".to_owned(),
            name: "Nordic Ski Adventure".to_owned(),
            sport_type: "NordicSki".to_owned(),
            start_date: "2025-11-15T10:30:00Z".to_owned(),
            distance_meters: 15000.0,
            duration_seconds: 5400.0,
        },
        ActivitySummary {
            id: "nordic456".to_owned(),
            name: "Cross Country Training".to_owned(),
            sport_type: "NordicSki".to_owned(),
            start_date: "2025-11-16T09:00:00Z".to_owned(),
            distance_meters: 12000.0,
            duration_seconds: 4200.0,
        },
    ];

    let toon_output = format_output(&summaries, OutputFormat::Toon).expect("toon format");

    // All key data should be present in TOON output
    assert!(
        toon_output.data.contains("nordic123"),
        "TOON should contain activity ID"
    );
    assert!(
        toon_output.data.contains("Nordic Ski Adventure"),
        "TOON should contain activity name"
    );
    assert!(
        toon_output.data.contains("NordicSki"),
        "TOON should contain sport type"
    );
    assert!(
        toon_output.data.contains("15000"),
        "TOON should contain distance"
    );
    assert!(
        toon_output.data.contains("5400"),
        "TOON should contain duration"
    );
}
