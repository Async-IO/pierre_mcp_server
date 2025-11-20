// ABOUTME: Integration tests for type-safe parameter deserialization
// ABOUTME: Validates that typed parameter structs correctly handle edge cases and errors
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Integration tests for type-safe parameter handling
//!
//! This test suite validates:
//! - Correct deserialization of valid parameters
//! - Proper error messages for invalid JSON
//! - Backward compatibility with old JSON formats
//! - Edge cases (empty strings, null values, missing fields)

#![allow(clippy::unwrap_used)] // Test assertions - unwrap is idiomatic

use pierre_mcp_server::types::json_schemas;
use serde_json::json;

// ============================================================================
// ToolCallParams Tests
// ============================================================================

#[test]
fn test_tool_call_params_valid() {
    let json = json!({
        "name": "get_activities",
        "arguments": {"limit": 10}
    });

    let params: json_schemas::ToolCallParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.name, "get_activities");
    assert_eq!(params.arguments["limit"], 10);
}

#[test]
fn test_tool_call_params_empty_arguments() {
    let json = json!({
        "name": "get_athlete",
        "arguments": {}
    });

    let params: json_schemas::ToolCallParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.name, "get_athlete");
    assert!(params.arguments.is_object());
}

#[test]
fn test_tool_call_params_missing_name() {
    let json = json!({
        "arguments": {"limit": 10}
    });

    let result: Result<json_schemas::ToolCallParams, _> = serde_json::from_value(json);
    assert!(result.is_err(), "Should fail when 'name' field is missing");
}

#[test]
fn test_tool_call_params_invalid_name_type() {
    let json = json!({
        "name": 123,
        "arguments": {}
    });

    let result: Result<json_schemas::ToolCallParams, _> = serde_json::from_value(json);
    assert!(result.is_err(), "Should fail when 'name' is not a string");
}

#[test]
fn test_tool_call_params_invalid_arguments_type() {
    let json = json!({
        "name": "test_tool",
        "arguments": "not_an_object"
    });

    let result: json_schemas::ToolCallParams = serde_json::from_value(json).unwrap();
    // Should succeed - arguments is serde_json::Value and can be any type
    assert!(result.arguments.is_string());
}

// ============================================================================
// ResourceReadParams Tests
// ============================================================================

#[test]
fn test_resource_read_params_valid() {
    let json = json!({
        "uri": "strava://activities/123"
    });

    let params: json_schemas::ResourceReadParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.uri, "strava://activities/123");
}

#[test]
fn test_resource_read_params_empty_uri() {
    let json = json!({
        "uri": ""
    });

    let params: json_schemas::ResourceReadParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.uri, "");
}

#[test]
fn test_resource_read_params_missing_uri() {
    let json = json!({});

    let result: Result<json_schemas::ResourceReadParams, _> = serde_json::from_value(json);
    assert!(result.is_err(), "Should fail when 'uri' field is missing");
}

// ============================================================================
// ProviderParams Tests
// ============================================================================

#[test]
fn test_provider_params_valid_strava() {
    let json = json!({
        "provider": "strava"
    });

    let params: json_schemas::ProviderParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.provider, Some("strava".to_owned()));
}

#[test]
fn test_provider_params_valid_fitbit() {
    let json = json!({
        "provider": "fitbit"
    });

    let params: json_schemas::ProviderParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.provider, Some("fitbit".to_owned()));
}

#[test]
fn test_provider_params_case_sensitive() {
    let json = json!({
        "provider": "STRAVA"
    });

    let params: json_schemas::ProviderParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.provider, Some("STRAVA".to_owned()));
}

#[test]
fn test_provider_params_optional_provider() {
    let json = json!({});

    let params: json_schemas::ProviderParams = serde_json::from_value(json).unwrap();
    assert_eq!(
        params.provider, None,
        "Provider should be optional - configuration tools don't have provider parameter"
    );
}

// ============================================================================
// TrackProgressParams Tests
// ============================================================================

#[test]
fn test_track_progress_params_valid() {
    let json = json!({
        "goal_id": "goal_123"
    });

    let params: json_schemas::TrackProgressParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.goal_id, "goal_123");
}

#[test]
fn test_track_progress_params_uuid_format() {
    let json = json!({
        "goal_id": "550e8400-e29b-41d4-a716-446655440000"
    });

    let params: json_schemas::TrackProgressParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.goal_id, "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn test_track_progress_params_empty_goal_id() {
    let json = json!({
        "goal_id": ""
    });

    let params: json_schemas::TrackProgressParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.goal_id, "");
}

// ============================================================================
// Response Type Tests
// ============================================================================

#[test]
fn test_disconnect_provider_response_serialization() {
    let response = json_schemas::DisconnectProviderResponse {
        success: true,
        message: "Successfully disconnected strava".to_owned(),
        provider: "strava".to_owned(),
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["success"], true);
    assert_eq!(json["message"], "Successfully disconnected strava");
    assert_eq!(json["provider"], "strava");
}

#[test]
fn test_goal_created_response_serialization() {
    let response = json_schemas::GoalCreatedResponse {
        goal_created: json_schemas::GoalCreatedDetails {
            goal_id: "goal_789".to_owned(),
            status: "active".to_owned(),
            message: "Goal created successfully".to_owned(),
        },
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["goal_created"]["goal_id"], "goal_789");
    assert_eq!(json["goal_created"]["status"], "active");
}

#[test]
fn test_progress_report_response_serialization() {
    let response = json_schemas::ProgressReportResponse {
        progress_report: json_schemas::ProgressReportDetails {
            goal_id: "goal_456".to_owned(),
            goal: json!({"name": "Run 100km"}),
            progress_percentage: 75.5,
            on_track: true,
            insights: vec!["Great progress!".to_owned()],
        },
    };

    let json = serde_json::to_value(&response).unwrap();
    assert_eq!(json["progress_report"]["goal_id"], "goal_456");
    assert_eq!(json["progress_report"]["progress_percentage"], 75.5);
    assert_eq!(json["progress_report"]["on_track"], true);
    assert_eq!(json["progress_report"]["insights"][0], "Great progress!");
}

#[test]
fn test_notification_item_round_trip() {
    use chrono::Utc;

    let notification = json_schemas::NotificationItem {
        id: "notif_123".to_owned(),
        provider: "strava".to_owned(),
        success: true,
        message: "Activity synced".to_owned(),
        created_at: Utc::now(),
    };

    // Serialize to JSON
    let json = serde_json::to_value(&notification).unwrap();

    // Deserialize back
    let deserialized: json_schemas::NotificationItem = serde_json::from_value(json).unwrap();

    assert_eq!(deserialized.id, notification.id);
    assert_eq!(deserialized.provider, notification.provider);
    assert_eq!(deserialized.success, notification.success);
    assert_eq!(deserialized.message, notification.message);
}

#[test]
fn test_connection_help_serialization() {
    let help = json_schemas::ConnectionHelp {
        message: "Connect your provider".to_owned(),
        supported_providers: vec!["strava".to_owned(), "fitbit".to_owned()],
        note: "Authorization required".to_owned(),
    };

    let json = serde_json::to_value(&help).unwrap();
    assert_eq!(json["message"], "Connect your provider");
    assert_eq!(json["supported_providers"][0], "strava");
    assert_eq!(json["supported_providers"][1], "fitbit");
    assert_eq!(json["note"], "Authorization required");
}

// ============================================================================
// Error Message Quality Tests
// ============================================================================

#[test]
fn test_error_message_for_missing_field() {
    let json = json!({
        "arguments": {}
    });

    let result: Result<json_schemas::ToolCallParams, _> = serde_json::from_value(json);
    let err = result.unwrap_err();
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("missing field"),
        "Error message should mention missing field: {err_msg}"
    );
}

#[test]
fn test_error_message_for_type_mismatch() {
    let json = json!({
        "provider": 123
    });

    let result: Result<json_schemas::ProviderParams, _> = serde_json::from_value(json);
    let err = result.unwrap_err();
    let err_msg = err.to_string();

    assert!(
        err_msg.contains("invalid type") || err_msg.contains("expected"),
        "Error message should mention type mismatch: {err_msg}"
    );
}

// ============================================================================
// Backward Compatibility Tests
// ============================================================================

#[test]
fn test_extra_fields_ignored() {
    let json = json!({
        "provider": "strava",
        "extra_field": "ignored",
        "another_field": 123
    });

    let params: json_schemas::ProviderParams = serde_json::from_value(json).unwrap();
    assert_eq!(params.provider, Some("strava".to_owned()));
    // Extra fields should be silently ignored
}

#[test]
fn test_notification_item_with_extra_fields() {
    use chrono::Utc;

    let json = json!({
        "id": "notif_456",
        "provider": "strava",
        "success": true,
        "message": "Test",
        "created_at": Utc::now().to_rfc3339(),
        "legacy_field": "old_data",
        "deprecated_flag": true
    });

    let notification: json_schemas::NotificationItem = serde_json::from_value(json).unwrap();
    assert_eq!(notification.provider, "strava");
    // Backward compatible - extra fields don't break parsing
}
