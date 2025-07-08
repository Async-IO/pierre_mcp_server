// ABOUTME: A2A agent capability discovery and advertisement system
// ABOUTME: Provides agent card with capabilities, endpoints, and protocol information for A2A discovery
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Agent Card Implementation
//!
//! Implements the A2A Agent Card specification for Pierre,
//! enabling agent discovery and capability negotiation.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A2A Agent Card for Pierre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCard {
    pub name: String,
    pub description: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub authentication: AuthenticationInfo,
    pub tools: Vec<ToolDefinition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,
}

/// Authentication information for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationInfo {
    pub schemes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2: Option<OAuth2Info>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<ApiKeyInfo>,
}

/// `OAuth2` authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2Info {
    pub authorization_url: String,
    pub token_url: String,
    pub scopes: Vec<String>,
}

/// API Key authentication information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    pub header_name: String,
    pub prefix: Option<String>,
    pub registration_url: String,
}

/// Tool definition in the agent card
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    pub output_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<ToolExample>>,
}

/// Example usage of a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: Value,
    pub output: Value,
}

impl AgentCard {
    /// Create a new Agent Card for Pierre
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "Pierre Fitness AI".into(),
            description: "AI-powered fitness data analysis and insights platform providing comprehensive activity analysis, performance tracking, and intelligent recommendations for athletes and fitness enthusiasts.".into(),
            version: "1.0.0".into(),
            capabilities: vec![
                "fitness-data-analysis".into(),
                "activity-intelligence".into(),
                "goal-management".into(),
                "performance-prediction".into(),
                "training-analytics".into(),
                "provider-integration".into(),
            ],
            authentication: AuthenticationInfo {
                schemes: vec!["api-key".into(), "oauth2".into()],
                oauth2: Some(OAuth2Info {
                    authorization_url: "https://pierre.ai/oauth/authorize".into(),
                    token_url: "https://pierre.ai/oauth/token".into(),
                    scopes: vec![
                        "fitness:read".into(),
                        "analytics:read".into(),
                        "goals:read".into(),
                        "goals:write".into(),
                    ],
                }),
                api_key: Some(ApiKeyInfo {
                    header_name: "Authorization".into(),
                    prefix: Some("Bearer".into()),
                    registration_url: "https://pierre.ai/api/keys/request".into(),
                }),
            },
            tools: Self::create_tool_definitions(),
            metadata: Some(Self::create_metadata()),
        }
    }

    /// Create tool definitions for the agent card
    #[allow(clippy::too_many_lines)]
    fn create_tool_definitions() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "get_activities".into(),
                description: "Retrieve user fitness activities from connected providers"
                    .to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "limit": {
                            "type": "number",
                            "description": "Number of activities to retrieve (max 100)",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 10
                        },
                        "before": {
                            "type": "string",
                            "format": "date-time",
                            "description": "ISO 8601 date to get activities before"
                        },
                        "provider": {
                            "type": "string",
                            "enum": ["strava", "fitbit"],
                            "description": "Specific provider to query (optional)"
                        }
                    }
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "activities": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": {"type": "string"},
                                    "name": {"type": "string"},
                                    "sport_type": {"type": "string"},
                                    "start_date": {"type": "string", "format": "date-time"},
                                    "duration_seconds": {"type": "number"},
                                    "distance_meters": {"type": "number"},
                                    "elevation_gain": {"type": "number"}
                                }
                            }
                        },
                        "total_count": {"type": "number"}
                    }
                }),
                examples: Some(vec![ToolExample {
                    description: "Get recent activities".into(),
                    input: serde_json::json!({"limit": 5}),
                    output: serde_json::json!({
                        "activities": [
                            {
                                "id": "123456",
                                "name": "Morning Run",
                                "sport_type": "Run",
                                "start_date": "2024-01-15T07:00:00Z",
                                "duration_seconds": 1800,
                                "distance_meters": 5000,
                                "elevation_gain": 50
                            }
                        ],
                        "total_count": 1
                    }),
                }]),
            },
            ToolDefinition {
                name: "analyze_activity".into(),
                description: "AI-powered analysis of a specific fitness activity".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "activity_id": {
                            "type": "string",
                            "description": "Unique identifier of the activity to analyze"
                        }
                    },
                    "required": ["activity_id"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "activity": {"type": "object"},
                        "insights": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": {"type": "string"},
                                    "title": {"type": "string"},
                                    "description": {"type": "string"},
                                    "confidence": {"type": "number"}
                                }
                            }
                        },
                        "metrics": {"type": "object"},
                        "recommendations": {"type": "array"}
                    }
                }),
                examples: None,
            },
            ToolDefinition {
                name: "get_athlete".into(),
                description: "Retrieve athlete profile information".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": {"type": "string"},
                        "username": {"type": "string"},
                        "firstname": {"type": "string"},
                        "lastname": {"type": "string"},
                        "city": {"type": "string"},
                        "country": {"type": "string"}
                    }
                }),
                examples: None,
            },
            ToolDefinition {
                name: "set_goal".into(),
                description: "Set a fitness goal for the user".into(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "goal_type": {
                            "type": "string",
                            "enum": ["distance", "time", "frequency", "speed"],
                            "description": "Type of goal to set"
                        },
                        "target_value": {
                            "type": "number",
                            "description": "Target value for the goal"
                        },
                        "timeframe": {
                            "type": "string",
                            "enum": ["weekly", "monthly", "yearly"],
                            "description": "Timeframe for achieving the goal"
                        },
                        "sport_type": {
                            "type": "string",
                            "description": "Sport type for the goal (optional)"
                        }
                    },
                    "required": ["goal_type", "target_value", "timeframe"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "goal_id": {"type": "string"},
                        "status": {"type": "string"},
                        "progress": {"type": "number"},
                        "estimated_completion": {"type": "string", "format": "date-time"}
                    }
                }),
                examples: None,
            },
        ]
    }

    /// Create metadata for the agent card
    fn create_metadata() -> HashMap<String, Value> {
        let mut metadata = HashMap::new();

        metadata.insert(
            "supported_providers".into(),
            serde_json::json!(["strava", "fitbit"]),
        );

        metadata.insert("data_retention_days".into(), serde_json::json!(365));

        metadata.insert(
            "rate_limits".into(),
            serde_json::json!({
                "trial": {"requests_per_month": 1000},
                "starter": {"requests_per_month": 10000},
                "professional": {"requests_per_month": 100_000},
                "enterprise": {"requests_per_month": -1}
            }),
        );

        metadata.insert(
            "contact".into(),
            serde_json::json!({
                "email": "support@pierre.ai",
                "documentation": "https://docs.pierre.ai/a2a",
                "status_page": "https://status.pierre.ai"
            }),
        );

        metadata
    }

    /// Serialize the agent card to JSON
    ///
    /// # Errors
    ///
    /// Returns an error if serialization fails
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Create agent card from JSON
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - JSON parsing fails
    /// - JSON structure is invalid
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for AgentCard {
    fn default() -> Self {
        Self::new()
    }
}
