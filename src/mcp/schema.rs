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
    pub tools: Vec<ToolSchema>,
}

/// Complete MCP Initialize Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
    pub capabilities: ServerCapabilities,
}

impl InitializeResponse {
    /// Create a new initialize response with current server configuration
    pub fn new(protocol_version: String, server_name: String, server_version: String) -> Self {
        Self {
            protocol_version,
            server_info: ServerInfo {
                name: server_name,
                version: server_version,
            },
            capabilities: ServerCapabilities {
                tools: create_fitness_tools(),
            },
        }
    }
}

/// Create all fitness provider tool schemas
fn create_fitness_tools() -> Vec<ToolSchema> {
    vec![
        // Original tools
        create_get_activities_tool(),
        create_get_athlete_tool(), 
        create_get_stats_tool(),
        create_get_activity_intelligence_tool(),
        create_connect_strava_tool(),
        create_connect_fitbit_tool(),
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
    ]
}

/// Create the get_activities tool schema
fn create_get_activities_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".to_string()),
    });
    
    properties.insert("limit".to_string(), PropertySchema {
        property_type: "number".to_string(),
        description: Some("Maximum number of activities to return".to_string()),
    });
    
    properties.insert("offset".to_string(), PropertySchema {
        property_type: "number".to_string(),
        description: Some("Number of activities to skip (for pagination)".to_string()),
    });

    ToolSchema {
        name: "get_activities".to_string(),
        description: "Get fitness activities from a provider".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

/// Create the get_athlete tool schema
fn create_get_athlete_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".to_string()),
    });

    ToolSchema {
        name: "get_athlete".to_string(),
        description: "Get athlete profile from a provider".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

/// Create the get_stats tool schema
fn create_get_stats_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".to_string()),
    });

    ToolSchema {
        name: "get_stats".to_string(),
        description: "Get fitness statistics from a provider".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

/// Create the get_activity_intelligence tool schema
fn create_get_activity_intelligence_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".to_string()),
    });
    
    properties.insert("activity_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("ID of the specific activity to analyze".to_string()),
    });
    
    properties.insert("include_weather".to_string(), PropertySchema {
        property_type: "boolean".to_string(),
        description: Some("Whether to include weather analysis (default: true)".to_string()),
    });
    
    properties.insert("include_location".to_string(), PropertySchema {
        property_type: "boolean".to_string(),
        description: Some("Whether to include location intelligence (default: true)".to_string()),
    });

    ToolSchema {
        name: "get_activity_intelligence".to_string(),
        description: "Generate AI-powered insights and analysis for a specific activity".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "activity_id".to_string()]),
        },
    }
}

/// Create the connect_strava tool schema
fn create_connect_strava_tool() -> ToolSchema {
    let properties = HashMap::new(); // No parameters needed - uses user's JWT context

    ToolSchema {
        name: "connect_strava".to_string(),
        description: "Generate authorization URL to connect user's Strava account. Returns a URL for the user to visit to authorize access to their Strava data.".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the connect_fitbit tool schema
fn create_connect_fitbit_tool() -> ToolSchema {
    let properties = HashMap::new(); // No parameters needed - uses user's JWT context

    ToolSchema {
        name: "connect_fitbit".to_string(),
        description: "Generate authorization URL to connect user's Fitbit account. Returns a URL for the user to visit to authorize access to their Fitbit data.".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the get_connection_status tool schema
fn create_get_connection_status_tool() -> ToolSchema {
    let properties = HashMap::new(); // No parameters needed - uses user's JWT context

    ToolSchema {
        name: "get_connection_status".to_string(),
        description: "Check which fitness providers are currently connected and authorized for the user. Returns connection status for all supported providers.".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec![]),
        },
    }
}

/// Create the disconnect_provider tool schema
fn create_disconnect_provider_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider to disconnect (e.g., 'strava', 'fitbit')".to_string()),
    });

    ToolSchema {
        name: "disconnect_provider".to_string(),
        description: "Disconnect and remove stored tokens for a specific fitness provider. This revokes access to the provider's data.".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_initialize_response_serialization() {
        let response = InitializeResponse::new(
            "2024-11-05".to_string(),
            "test-server".to_string(),
            "1.0.0".to_string(),
        );
        
        let json = serde_json::to_value(&response).expect("Should serialize");
        
        assert_eq!(json["protocolVersion"], "2024-11-05");
        assert_eq!(json["serverInfo"]["name"], "test-server");
        assert_eq!(json["serverInfo"]["version"], "1.0.0");
        assert!(json["capabilities"]["tools"].is_array());
        
        let tools = json["capabilities"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 8);
        
        let tool_names: Vec<&str> = tools.iter()
            .filter_map(|t| t["name"].as_str())
            .collect();
        
        assert!(tool_names.contains(&"get_activities"));
        assert!(tool_names.contains(&"get_athlete"));
        assert!(tool_names.contains(&"get_stats"));
        assert!(tool_names.contains(&"get_activity_intelligence"));
        assert!(tool_names.contains(&"connect_strava"));
        assert!(tool_names.contains(&"connect_fitbit"));
        assert!(tool_names.contains(&"get_connection_status"));
        assert!(tool_names.contains(&"disconnect_provider"));
    }

    #[test]
    fn test_tool_schema_structure() {
        let tool = create_get_activities_tool();
        
        assert_eq!(tool.name, "get_activities");
        assert!(!tool.description.is_empty());
        assert_eq!(tool.input_schema.schema_type, "object");
        assert!(tool.input_schema.properties.is_some());
        assert!(tool.input_schema.required.is_some());
        
        let properties = tool.input_schema.properties.unwrap();
        assert!(properties.contains_key("provider"));
        assert!(properties.contains_key("limit"));
        assert!(properties.contains_key("offset"));
        
        let required = tool.input_schema.required.unwrap();
        assert!(required.contains(&"provider".to_string()));
    }

    #[test]
    fn test_round_trip_serialization() {
        let original = InitializeResponse::new(
            "2024-11-05".to_string(),
            "pierre-mcp-server".to_string(),
            "0.1.0".to_string(),
        );
        
        let json_str = serde_json::to_string(&original).expect("Should serialize");
        let deserialized: InitializeResponse = serde_json::from_str(&json_str)
            .expect("Should deserialize");
        
        assert_eq!(original.protocol_version, deserialized.protocol_version);
        assert_eq!(original.server_info.name, deserialized.server_info.name);
        assert_eq!(original.server_info.version, deserialized.server_info.version);
        assert_eq!(original.capabilities.tools.len(), deserialized.capabilities.tools.len());
    }
}

// === ADVANCED ANALYTICS TOOLS ===

/// Create the analyze_activity tool schema
fn create_analyze_activity_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name (e.g., 'strava', 'fitbit')".to_string()),
    });
    
    properties.insert("activity_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("ID of the activity to analyze".to_string()),
    });

    ToolSchema {
        name: "analyze_activity".to_string(),
        description: "Perform deep analysis of an individual activity including insights, metrics, and anomaly detection".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "activity_id".to_string()]),
        },
    }
}

/// Create the calculate_metrics tool schema
fn create_calculate_metrics_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("activity_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("ID of the activity".to_string()),
    });
    
    properties.insert("metrics".to_string(), PropertySchema {
        property_type: "array".to_string(),
        description: Some("Specific metrics to calculate (e.g., ['trimp', 'power_to_weight', 'efficiency'])".to_string()),
    });

    ToolSchema {
        name: "calculate_metrics".to_string(),
        description: "Calculate advanced fitness metrics for an activity (TRIMP, power-to-weight ratio, efficiency scores, etc.)".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "activity_id".to_string()]),
        },
    }
}

/// Create the analyze_performance_trends tool schema
fn create_analyze_performance_trends_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("timeframe".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Time period for analysis ('week', 'month', 'quarter', 'sixmonths', 'year')".to_string()),
    });
    
    properties.insert("metric".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Metric to analyze trends for ('pace', 'heart_rate', 'power', 'distance', 'duration')".to_string()),
    });
    
    properties.insert("sport_type".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Filter by sport type (optional)".to_string()),
    });

    ToolSchema {
        name: "analyze_performance_trends".to_string(),
        description: "Analyze performance trends over time with statistical analysis and insights".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "timeframe".to_string(), "metric".to_string()]),
        },
    }
}

/// Create the compare_activities tool schema
fn create_compare_activities_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("activity_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Primary activity to compare".to_string()),
    });
    
    properties.insert("comparison_type".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Type of comparison ('similar_activities', 'personal_best', 'average', 'recent')".to_string()),
    });

    ToolSchema {
        name: "compare_activities".to_string(),
        description: "Compare an activity against similar activities, personal bests, or historical averages".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "activity_id".to_string(), "comparison_type".to_string()]),
        },
    }
}

/// Create the detect_patterns tool schema
fn create_detect_patterns_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("pattern_type".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Type of pattern to detect ('training_consistency', 'seasonal_trends', 'performance_plateaus', 'injury_risk')".to_string()),
    });
    
    properties.insert("timeframe".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Time period for pattern analysis".to_string()),
    });

    ToolSchema {
        name: "detect_patterns".to_string(),
        description: "Detect patterns in training data such as consistency trends, seasonal variations, or performance plateaus".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "pattern_type".to_string()]),
        },
    }
}

/// Create the set_goal tool schema
fn create_set_goal_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("title".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Goal title".to_string()),
    });
    
    properties.insert("description".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Goal description".to_string()),
    });
    
    properties.insert("goal_type".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Type of goal ('distance', 'time', 'frequency', 'performance', 'custom')".to_string()),
    });
    
    properties.insert("target_value".to_string(), PropertySchema {
        property_type: "number".to_string(),
        description: Some("Target value to achieve".to_string()),
    });
    
    properties.insert("target_date".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Target completion date (ISO format)".to_string()),
    });
    
    properties.insert("sport_type".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Sport type for the goal".to_string()),
    });

    ToolSchema {
        name: "set_goal".to_string(),
        description: "Create and manage fitness goals with tracking and progress monitoring".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["title".to_string(), "goal_type".to_string(), "target_value".to_string(), "target_date".to_string()]),
        },
    }
}

/// Create the track_progress tool schema
fn create_track_progress_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("goal_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("ID of the goal to track".to_string()),
    });

    ToolSchema {
        name: "track_progress".to_string(),
        description: "Track progress toward a specific goal with milestone achievements and completion estimates".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["goal_id".to_string()]),
        },
    }
}

/// Create the suggest_goals tool schema
fn create_suggest_goals_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("goal_category".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Category of goals to suggest ('distance', 'performance', 'consistency', 'all')".to_string()),
    });

    ToolSchema {
        name: "suggest_goals".to_string(),
        description: "Generate AI-powered goal suggestions based on user's activity history and fitness level".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

/// Create the analyze_goal_feasibility tool schema
fn create_analyze_goal_feasibility_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("goal_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("ID of the goal to analyze".to_string()),
    });

    ToolSchema {
        name: "analyze_goal_feasibility".to_string(),
        description: "Assess whether a goal is realistic and achievable based on current performance and timeline".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["goal_id".to_string()]),
        },
    }
}

/// Create the generate_recommendations tool schema
fn create_generate_recommendations_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("recommendation_type".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Type of recommendations ('training', 'recovery', 'nutrition', 'equipment', 'all')".to_string()),
    });
    
    properties.insert("activity_id".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Specific activity to base recommendations on (optional)".to_string()),
    });

    ToolSchema {
        name: "generate_recommendations".to_string(),
        description: "Generate personalized training recommendations based on activity data and user profile".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

/// Create the calculate_fitness_score tool schema
fn create_calculate_fitness_score_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("timeframe".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Time period for fitness assessment ('month', 'quarter', 'sixmonths')".to_string()),
    });

    ToolSchema {
        name: "calculate_fitness_score".to_string(),
        description: "Calculate comprehensive fitness score based on recent training load, consistency, and performance trends".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}

/// Create the predict_performance tool schema
fn create_predict_performance_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("target_sport".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Sport type for prediction".to_string()),
    });
    
    properties.insert("target_distance".to_string(), PropertySchema {
        property_type: "number".to_string(),
        description: Some("Target distance for performance prediction".to_string()),
    });
    
    properties.insert("target_date".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Target date for prediction (ISO format)".to_string()),
    });

    ToolSchema {
        name: "predict_performance".to_string(),
        description: "Predict future performance capabilities based on current fitness trends and training history".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string(), "target_sport".to_string(), "target_distance".to_string()]),
        },
    }
}

/// Create the analyze_training_load tool schema
fn create_analyze_training_load_tool() -> ToolSchema {
    let mut properties = HashMap::new();
    
    properties.insert("provider".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Fitness provider name".to_string()),
    });
    
    properties.insert("timeframe".to_string(), PropertySchema {
        property_type: "string".to_string(),
        description: Some("Time period for load analysis ('week', 'month', 'quarter')".to_string()),
    });

    ToolSchema {
        name: "analyze_training_load".to_string(),
        description: "Analyze training load balance, recovery needs, and load distribution over time".to_string(),
        input_schema: JsonSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["provider".to_string()]),
        },
    }
}