// ABOUTME: Tests for provider helper functions
// ABOUTME: Verifies provider extraction and response creation
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(missing_docs)]

use pierre_mcp_server::protocols::universal::handlers::provider_helpers::{
    build_activities_success_response, create_auth_error_response, create_no_token_response,
    extract_provider,
};

#[test]
fn test_extract_provider_with_value() {
    let mut params = serde_json::Map::new();
    params.insert("provider".to_owned(), serde_json::json!("garmin"));
    assert_eq!(extract_provider(&params), "garmin");
}

#[test]
fn test_extract_provider_default() {
    let params = serde_json::Map::new();
    // Default is "synthetic" unless PIERRE_DEFAULT_PROVIDER is set
    let result = extract_provider(&params);
    assert!(!result.is_empty());
}

#[test]
fn test_no_token_response() {
    let response = create_no_token_response("strava");
    assert!(!response.success);
    assert!(response
        .error
        .as_ref()
        .is_some_and(|e| e.contains("strava")));
}

// ============================================================================
// NEW TESTS: Response Building Functions
// ============================================================================

#[test]
fn test_no_token_response_metadata() {
    let response = create_no_token_response("garmin");

    assert!(!response.success);
    assert!(response.error.is_some());
    assert!(response.result.is_none());

    // Check metadata
    assert!(response.metadata.is_some());
    if let Some(ref metadata) = response.metadata {
        assert_eq!(
            metadata.get("total_activities"),
            Some(&serde_json::Value::Number(0.into()))
        );
        assert_eq!(
            metadata.get("authentication_required"),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            metadata.get("provider"),
            Some(&serde_json::Value::String("garmin".to_owned()))
        );
    }
}

#[test]
fn test_auth_error_response() {
    let response = create_auth_error_response("strava", "Invalid token");

    assert!(response.success); // Success=true with error in result
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    if let Some(ref result) = response.result {
        assert!(result.get("error").is_some());
        if let Some(error) = result.get("error").and_then(serde_json::Value::as_str) {
            assert!(error.contains("Invalid token"));
        }
        assert_eq!(
            result.get("provider"),
            Some(&serde_json::Value::String("strava".to_owned()))
        );
    }
}

#[test]
fn test_auth_error_response_metadata() {
    let response = create_auth_error_response("garmin", "Token expired");

    assert!(response.metadata.is_some());
    if let Some(ref metadata) = response.metadata {
        assert_eq!(
            metadata.get("authentication_error"),
            Some(&serde_json::Value::Bool(true))
        );
        assert_eq!(
            metadata.get("provider"),
            Some(&serde_json::Value::String("garmin".to_owned()))
        );
    }
}

#[test]
fn test_build_activities_success_response() {
    use chrono::Utc;
    use pierre_mcp_server::models::{Activity, SportType};
    use uuid::Uuid;

    let activities = vec![
        Activity {
            id: "123".to_owned(),
            name: "Morning Run".to_owned(),
            sport_type: SportType::Run,
            start_date: Utc::now(),
            distance_meters: Some(5000.0),
            duration_seconds: 1800,
            ..Default::default()
        },
        Activity {
            id: "456".to_owned(),
            name: "Evening Ride".to_owned(),
            sport_type: SportType::Ride,
            start_date: Utc::now(),
            distance_meters: Some(25000.0),
            duration_seconds: 3600,
            ..Default::default()
        },
    ];

    let user_id = Uuid::new_v4();
    let tenant_id = Some("tenant-123".to_owned());

    let response = build_activities_success_response(&activities, "strava", user_id, tenant_id);

    assert!(response.success);
    assert!(response.error.is_none());
    assert!(response.result.is_some());

    if let Some(ref result) = response.result {
        assert_eq!(result.get("count"), Some(&serde_json::json!(2)));
        assert_eq!(
            result.get("provider"),
            Some(&serde_json::Value::String("strava".to_owned()))
        );
    }

    // Check metadata
    assert!(response.metadata.is_some());
    if let Some(ref metadata) = response.metadata {
        assert_eq!(
            metadata.get("total_activities"),
            Some(&serde_json::Value::Number(2.into()))
        );
        assert_eq!(
            metadata.get("user_id"),
            Some(&serde_json::Value::String(user_id.to_string()))
        );
        assert_eq!(
            metadata.get("tenant_id"),
            Some(&serde_json::Value::String("tenant-123".to_owned()))
        );
        assert_eq!(
            metadata.get("provider"),
            Some(&serde_json::Value::String("strava".to_owned()))
        );
        assert_eq!(
            metadata.get("cached"),
            Some(&serde_json::Value::Bool(false))
        );
    }
}

#[test]
fn test_build_activities_empty_list() {
    use uuid::Uuid;

    let activities = vec![];
    let user_id = Uuid::new_v4();

    let response = build_activities_success_response(&activities, "garmin", user_id, None);

    assert!(response.success);
    if let Some(ref result) = response.result {
        assert_eq!(result.get("count"), Some(&serde_json::json!(0)));
    }

    assert!(response.metadata.is_some());
    if let Some(ref metadata) = response.metadata {
        assert_eq!(
            metadata.get("total_activities"),
            Some(&serde_json::Value::Number(0.into()))
        );
        assert_eq!(metadata.get("tenant_id"), Some(&serde_json::Value::Null));
    }
}

#[test]
fn test_extract_provider_different_providers() {
    let providers = vec!["strava", "garmin", "fitbit", "synthetic"];

    for provider in providers {
        let mut params = serde_json::Map::new();
        params.insert("provider".to_owned(), serde_json::json!(provider));
        assert_eq!(extract_provider(&params), provider);
    }
}

#[test]
fn test_no_token_response_different_providers() {
    let providers = vec!["strava", "garmin", "fitbit"];

    for provider in providers {
        let response = create_no_token_response(provider);
        assert!(!response.success);
        if let Some(ref error) = response.error {
            assert!(error.contains(provider));
        }

        assert!(response.metadata.is_some());
        if let Some(ref metadata) = response.metadata {
            assert_eq!(
                metadata.get("provider"),
                Some(&serde_json::Value::String(provider.to_owned()))
            );
        }
    }
}
