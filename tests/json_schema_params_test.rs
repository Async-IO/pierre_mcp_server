// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// Test parameter deserialization for Phase 3A typed parameters
#![allow(missing_docs, clippy::unwrap_used, clippy::float_cmp)]

use serde_json::json;

#[test]
fn test_intelligence_params_deserialization() {
    // Test GetActivityIntelligenceParams
    let params_json = json!({"activity_id": "123456"});
    let params: pierre_mcp_server::types::json_schemas::GetActivityIntelligenceParams =
        serde_json::from_value(params_json).unwrap();
    assert_eq!(params.activity_id, "123456");

    // Test AnalyzePerformanceTrendsParams with defaults
    let params_json = json!({});
    let params: pierre_mcp_server::types::json_schemas::AnalyzePerformanceTrendsParams =
        serde_json::from_value(params_json).unwrap();
    assert_eq!(params.metric, "pace");
    assert_eq!(params.timeframe, "month");

    // Test CompareActivitiesParams
    let params_json = json!({"activity_id": "789"});
    let params: pierre_mcp_server::types::json_schemas::CompareActivitiesParams =
        serde_json::from_value(params_json).unwrap();
    assert_eq!(params.activity_id, "789");
    assert_eq!(params.comparison_type, "similar_activities");
    assert_eq!(params.compare_activity_id, None);
}

#[test]
fn test_goals_params_deserialization() {
    // Test AnalyzeGoalFeasibilityParams
    let params_json = json!({"goal_type": "distance", "target_value": 42.195});
    let params: pierre_mcp_server::types::json_schemas::AnalyzeGoalFeasibilityParams =
        serde_json::from_value(params_json).unwrap();
    assert_eq!(params.goal_type, "distance");
    assert_eq!(params.target_value, 42.195);

    // Test SetGoalParams
    let params_json = json!({
        "goal_type": "duration",
        "target_value": 60.0,
        "timeframe": "week"
    });
    let params: pierre_mcp_server::types::json_schemas::SetGoalParams =
        serde_json::from_value(params_json).unwrap();
    assert_eq!(params.goal_type, "duration");
    assert_eq!(params.target_value, 60.0);
    assert_eq!(params.timeframe, "week");
    assert_eq!(params.title, "Fitness Goal"); // default

    // Test TrackProgressParams
    let params_json = json!({"goal_id": "goal-123"});
    let params: pierre_mcp_server::types::json_schemas::TrackProgressParams =
        serde_json::from_value(params_json).unwrap();
    assert_eq!(params.goal_id, "goal-123");
}
