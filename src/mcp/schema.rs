// ABOUTME: MCP protocol schema definitions and message structures
// ABOUTME: Defines JSON-RPC protocol schemas for Model Context Protocol communication
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! MCP Protocol Schema Definitions
//!
//! This module contains type-safe definitions for all MCP protocol messages,
//! capabilities, and tool schemas. This ensures protocol compliance and makes
//! it easy to modify the schema without hardcoding JSON.

use crate::constants::{
    json_fields::{ACTIVITY_ID, LIMIT, OFFSET, PROVIDER},
    tools::{
        ANALYZE_ACTIVITY, DISCONNECT_PROVIDER, GET_ACTIVITIES, GET_ACTIVITY_INTELLIGENCE,
        GET_ATHLETE, GET_CONNECTION_STATUS, GET_STATS,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP Protocol Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolInfo {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
}

/// Server Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// MCP Tool Schema Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: JsonSchema,
}

/// JSON Schema Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, PropertySchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Tool Call for executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: Option<serde_json::Value>,
}

/// Tool Response after execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResponse {
    pub content: Vec<Content>,
    #[serde(rename = "isError")]
    pub is_error: bool,
    #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<serde_json::Value>,
}

/// Content types for MCP messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
}

/// Tool definition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// JSON Schema Property Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    #[serde(rename = "type")]
    pub property_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// MCP Server Capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<LoggingCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

/// Tools capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Logging capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingCapability {}

/// Prompts capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Client capabilities (for processing client initialize requests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
}

/// Sampling capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability {}

/// Roots capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    #[serde(rename = "listChanged", skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Complete MCP Initialize Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    pub capabilities: ServerCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Initialize Request from client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
    pub capabilities: ClientCapabilities,
}

/// Client Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

impl InitializeResponse {
    /// Create a new initialize response with current server configuration
    #[must_use]
    pub fn new(protocol_version: String, server_name: String, server_version: String) -> Self {
        Self {
            protocol_version,
            server_info: ServerInfo {
                name: server_name,
                version: server_version,
            },
            capabilities: ServerCapabilities {
                experimental: None,
                logging: None,
                prompts: None,
                resources: None,
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
            },
            instructions: Some("This server provides fitness data tools for Strava and Fitbit integration. OAuth must be configured at tenant level via REST API. Use `get_activities`, `get_athlete`, and other analytics tools to access your fitness data.".into()),
        }
    }
}

/// Get all available tools (public interface for tests)
#[must_use]
pub fn get_tools() -> Vec<ToolSchema> {
    create_fitness_tools()
}

/// Create all fitness provider tool schemas
fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // Original tools
        create_get_activities_tool(),
        create_get_athlete_tool(),
        create_get_stats_tool(),
        create_get_activity_intelligence_tool(),
        create_get_connection_status_tool(),
        create_disconnect_provider_tool(),
        // Advanced Analytics Tools
        create_analyze_activity_tool(),
        create_calculate_metrics_tool(),
        create_analyze_performance_trends_tool(),
        create_compare_activities_tool(),
        create_detect_patterns_tool(),
        create_set_goal_tool(),
        create_track_progress_tool(),
        create_suggest_goals_tool(),
        create_analyze_goal_feasibility_tool(),
        create_generate_recommendations_tool(),
        create_calculate_fitness_score_tool(),
        create_predict_performance_tool(),
        create_analyze_training_load_tool(),
        // Configuration Management Tools
        create_get_configuration_catalog_tool(),
        create_get_configuration_profiles_tool(),
        create_get_user_configuration_tool(),
        create_update_user_configuration_tool(),
        create_calculate_personalized_zones_tool(),
        create_validate_configuration_tool(),
    ]
}

/// Create the `get_activities` tool schema
fn create_get_activities_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    properties.insert(
        LIMIT.to_string(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum number of activities to return".into()),
        },
    );

    properties.insert(
        OFFSET.to_string(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Number of activities to skip (for pagination)".into()),
        },
    );

    ToolSchema {
        name: GET_ACTIVITIES.to_string(),
        description: "Get fitness activities from a provider".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_string()]),
        },
    }
}

/// Create the `get_athlete` tool schema
fn create_get_athlete_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    ToolSchema {
        name: GET_ATHLETE.to_string(),
        description: "Get athlete profile from a provider".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_string()]),
        },
    }
}

/// Create the `get_stats` tool schema
fn create_get_stats_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    ToolSchema {
        name: GET_STATS.to_string(),
        description: "Get fitness statistics from a provider".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_string()]),
        },
    }
}

/// Create the `get_activity_intelligence` tool schema
fn create_get_activity_intelligence_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    properties.insert(
        ACTIVITY_ID.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the specific activity to analyze".into()),
        },
    );

    properties.insert(
        "include_weather".into(),
        PropertySchema {
            property_type: "boolean".into(),
            description: Some("Whether to include weather analysis (default: true)".into()),
        },
    );

    properties.insert(
        "include_location".into(),
        PropertySchema {
            property_type: "boolean".into(),
            description: Some("Whether to include location intelligence (default: true)".into()),
        },
    );

    ToolSchema {
        name: GET_ACTIVITY_INTELLIGENCE.to_string(),
        description: "Generate AI-powered insights and analysis for a specific activity"
            .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_string(), ACTIVITY_ID.to_string()]),
        },
    }
}

/// Create the `get_connection_status` tool schema
fn create_get_connection_status_tool() -> ToolSchema {
    let properties = HashMap::new(); // No parameters needed - uses user's JWT context

    ToolSchema {
        name: GET_CONNECTION_STATUS.to_string(),
        description: "Check which fitness providers are currently connected and authorized for the user. Returns connection status for all supported providers.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the `disconnect_provider` tool schema
fn create_disconnect_provider_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider to disconnect (e.g., 'strava', 'fitbit')".into()),
        },
    );

    ToolSchema {
        name: DISCONNECT_PROVIDER.to_string(),
        description: "Disconnect and remove stored tokens for a specific fitness provider. This revokes access to the provider's data.".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_string()]),
        },
    }
}

// === ADVANCED ANALYTICS TOOLS ===

/// Create the `analyze_activity` tool schema
fn create_analyze_activity_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        PROVIDER.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".into()),
        },
    );

    properties.insert(
        ACTIVITY_ID.to_string(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the activity to analyze".into()),
        },
    );

    ToolSchema {
        name: ANALYZE_ACTIVITY.to_string(),
        description: "Perform deep analysis of an individual activity including insights, metrics, and anomaly detection".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![PROVIDER.to_string(), ACTIVITY_ID.to_string()]),
        },
    }
}

/// Create the `calculate_metrics` tool schema
fn create_calculate_metrics_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "activity_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the activity".into()),
        },
    );

    properties.insert(
        "metrics".into(),
        PropertySchema {
            property_type: "array".into(),
            description: Some(
                "Specific metrics to calculate (e.g., ['trimp', 'power_to_weight', 'efficiency'])"
                    .to_string(),
            ),
        },
    );

    ToolSchema {
        name: "calculate_metrics".into(),
        description: "Calculate advanced fitness metrics for an activity (TRIMP, power-to-weight ratio, efficiency scores, etc.)".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "activity_id".into()]),
        },
    }
}

/// Create the `analyze_performance_trends` tool schema
fn create_analyze_performance_trends_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Time period for analysis ('week', 'month', 'quarter', 'sixmonths', 'year')"
                    .to_string(),
            ),
        },
    );

    properties.insert("metric".into(), PropertySchema {
        property_type: "string".into(),
        description: Some("Metric to analyze trends for ('pace', 'heart_rate', 'power', 'distance', 'duration')".into()),
    });

    properties.insert(
        "sport_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Filter by sport type (optional)".into()),
        },
    );

    ToolSchema {
        name: "analyze_performance_trends".into(),
        description: "Analyze performance trends over time with statistical analysis and insights"
            .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "timeframe".into(), "metric".into()]),
        },
    }
}

/// Create the `compare_activities` tool schema
fn create_compare_activities_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "activity_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Primary activity to compare".into()),
        },
    );

    properties.insert(
        "comparison_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Type of comparison ('similar_activities', 'personal_best', 'average', 'recent')"
                    .to_string(),
            ),
        },
    );

    ToolSchema {
        name: "compare_activities".into(),
        description:
            "Compare an activity against similar activities, personal bests, or historical averages"
                .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "provider".into(),
                "activity_id".into(),
                "comparison_type".into(),
            ]),
        },
    }
}

/// Create the `detect_patterns` tool schema
fn create_detect_patterns_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert("pattern_type".into(), PropertySchema {
        property_type: "string".into(),
        description: Some("Type of pattern to detect ('training_consistency', 'seasonal_trends', 'performance_plateaus', 'injury_risk')".into()),
    });

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Time period for pattern analysis".into()),
        },
    );

    ToolSchema {
        name: "detect_patterns".into(),
        description: "Detect patterns in training data such as consistency trends, seasonal variations, or performance plateaus".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "pattern_type".into()]),
        },
    }
}

/// Create the `set_goal` tool schema
fn create_set_goal_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "title".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Goal title".into()),
        },
    );

    properties.insert(
        "description".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Goal description".into()),
        },
    );

    properties.insert(
        "goal_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Type of goal ('distance', 'time', 'frequency', 'performance', 'custom')"
                    .to_string(),
            ),
        },
    );

    properties.insert(
        "target_value".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Target value to achieve".into()),
        },
    );

    properties.insert(
        "target_date".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Target completion date (ISO format)".into()),
        },
    );

    properties.insert(
        "sport_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Sport type for the goal".into()),
        },
    );

    ToolSchema {
        name: "set_goal".into(),
        description: "Create and manage fitness goals with tracking and progress monitoring"
            .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![
                "title".into(),
                "goal_type".into(),
                "target_value".into(),
                "target_date".into(),
            ]),
        },
    }
}

/// Create the `track_progress` tool schema
fn create_track_progress_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "goal_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the goal to track".into()),
        },
    );

    ToolSchema {
        name: "track_progress".into(),
        description: "Track progress toward a specific goal with milestone achievements and completion estimates".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["goal_id".into()]),
        },
    }
}

/// Create the `suggest_goals` tool schema
fn create_suggest_goals_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "goal_category".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Category of goals to suggest ('distance', 'performance', 'consistency', 'all')"
                    .to_string(),
            ),
        },
    );

    ToolSchema {
        name: "suggest_goals".into(),
        description: "Generate AI-powered goal suggestions based on user's activity history and fitness level".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `analyze_goal_feasibility` tool schema
fn create_analyze_goal_feasibility_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "goal_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("ID of the goal to analyze".into()),
        },
    );

    ToolSchema {
        name: "analyze_goal_feasibility".into(),
        description: "Assess whether a goal is realistic and achievable based on current performance and timeline".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["goal_id".into()]),
        },
    }
}

/// Create the `generate_recommendations` tool schema
fn create_generate_recommendations_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "recommendation_type".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Type of recommendations ('training', 'recovery', 'nutrition', 'equipment', 'all')"
                    .to_string(),
            ),
        },
    );

    properties.insert(
        "activity_id".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Specific activity to base recommendations on (optional)".into()),
        },
    );

    ToolSchema {
        name: "generate_recommendations".into(),
        description:
            "Generate personalized training recommendations based on activity data and user profile"
                .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `calculate_fitness_score` tool schema
fn create_calculate_fitness_score_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some(
                "Time period for fitness assessment ('month', 'quarter', 'sixmonths')".into(),
            ),
        },
    );

    ToolSchema {
        name: "calculate_fitness_score".into(),
        description: "Calculate comprehensive fitness score based on recent training load, consistency, and performance trends".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `predict_performance` tool schema
fn create_predict_performance_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "target_sport".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Sport type for prediction".into()),
        },
    );

    properties.insert(
        "target_distance".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Target distance for performance prediction".into()),
        },
    );

    properties.insert(
        "target_date".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Target date for prediction (ISO format)".into()),
        },
    );

    ToolSchema {
        name: "predict_performance".into(),
        description: "Predict future performance capabilities based on current fitness trends and training history".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into(), "target_sport".into(), "target_distance".into()]),
        },
    }
}

/// Create the `analyze_training_load` tool schema
fn create_analyze_training_load_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "provider".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Fitness provider name".into()),
        },
    );

    properties.insert(
        "timeframe".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Time period for load analysis ('week', 'month', 'quarter')".into()),
        },
    );

    ToolSchema {
        name: "analyze_training_load".into(),
        description:
            "Analyze training load balance, recovery needs, and load distribution over time"
                .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["provider".into()]),
        },
    }
}

/// Create the `get_configuration_catalog` tool schema
fn create_get_configuration_catalog_tool() -> ToolSchema {
    ToolSchema {
        name: "get_configuration_catalog".into(),
        description: "Get the complete configuration catalog with all available parameters and their metadata".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `get_configuration_profiles` tool schema
fn create_get_configuration_profiles_tool() -> ToolSchema {
    ToolSchema {
        name: "get_configuration_profiles".into(),
        description: "Get available configuration profiles (Research, Elite, Recreational, Beginner, Medical, etc.)".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `get_user_configuration` tool schema
fn create_get_user_configuration_tool() -> ToolSchema {
    ToolSchema {
        name: "get_user_configuration".into(),
        description:
            "Get current user's configuration including active profile and parameter overrides"
                .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(HashMap::new()),
            required: Some(vec![]),
        },
    }
}

/// Create the `update_user_configuration` tool schema
fn create_update_user_configuration_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "profile".into(),
        PropertySchema {
            property_type: "string".into(),
            description: Some("Configuration profile to apply (optional)".into()),
        },
    );

    properties.insert(
        "parameters".into(),
        PropertySchema {
            property_type: "object".into(),
            description: Some("Parameter overrides to apply (optional)".into()),
        },
    );

    ToolSchema {
        name: "update_user_configuration".into(),
        description: "Update user's configuration by applying a profile and/or parameter overrides"
            .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the `calculate_personalized_zones` tool schema
fn create_calculate_personalized_zones_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "vo2_max".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("VO2 max in ml/kg/min".into()),
        },
    );

    properties.insert(
        "resting_hr".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Resting heart rate in bpm (optional, defaults to 60)".into()),
        },
    );

    properties.insert(
        "max_hr".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Maximum heart rate in bpm (optional, defaults to 190)".into()),
        },
    );

    properties.insert(
        "lactate_threshold".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some(
                "Lactate threshold as percentage of VO2 max (optional, defaults to 0.85)"
                    .to_string(),
            ),
        },
    );

    properties.insert(
        "sport_efficiency".into(),
        PropertySchema {
            property_type: "number".into(),
            description: Some("Sport efficiency factor (optional, defaults to 1.0)".into()),
        },
    );

    ToolSchema {
        name: "calculate_personalized_zones".into(),
        description: "Calculate personalized training zones (heart rate, pace, power) based on VO2 max and physiological parameters".into(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["vo2_max".into()]),
        },
    }
}

/// Create the `validate_configuration` tool schema
fn create_validate_configuration_tool() -> ToolSchema {
    let mut properties = HashMap::new();

    properties.insert(
        "parameters".into(),
        PropertySchema {
            property_type: "object".into(),
            description: Some("Configuration parameters to validate".into()),
        },
    );

    ToolSchema {
        name: "validate_configuration".into(),
        description:
            "Validate configuration parameters for physiological limits and scientific bounds"
                .to_string(),
        input_schema: JsonSchema {
            schema_type: "object".into(),
            properties: Some(properties),
            required: Some(vec!["parameters".into()]),
        },
    }
}
