// ABOUTME: Data access tools implementing the McpTool trait as wrappers.
// ABOUTME: Delegates to existing handlers for get_activities, get_athlete, get_stats.
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Data Access Tools
//!
//! This module contains tools for accessing fitness data:
//! - `GetActivitiesTool` - Retrieve user activities with filtering and pagination
//! - `GetAthleteTool` - Get athlete profile information
//! - `GetStatsTool` - Get aggregated activity statistics
//!
//! These tools wrap the existing universal protocol handlers to maintain
//! backward compatibility while providing the new `McpTool` interface.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::{json, Value};

use crate::errors::AppResult;
use crate::mcp::schema::{JsonSchema, PropertySchema};
use crate::protocols::universal::executor::UniversalExecutor;
use crate::protocols::universal::handlers::fitness_api::{
    handle_get_activities, handle_get_athlete, handle_get_stats,
};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::tools::context::ToolExecutionContext;
use crate::tools::result::ToolResult;
use crate::tools::traits::{McpTool, ToolCapabilities};

// ============================================================================
// Helper functions for converting between request/response types
// ============================================================================

/// Build a `UniversalRequest` from tool context and args
fn build_universal_request(
    tool_name: &str,
    args: &Value,
    context: &ToolExecutionContext,
) -> UniversalRequest {
    // Convert parameters from object to Value, or use empty object
    let parameters = if args.is_object() {
        args.clone()
    } else {
        json!({})
    };

    UniversalRequest {
        tool_name: tool_name.to_owned(),
        parameters,
        user_id: context.user_id.to_string(),
        protocol: "mcp".to_owned(),
        tenant_id: context.tenant_id.map(|id| id.to_string()),
        progress_token: None,
        cancellation_token: None,
        progress_reporter: None,
    }
}

/// Convert `UniversalResponse` to `ToolResult`
fn convert_to_tool_result(response: UniversalResponse) -> ToolResult {
    if response.success {
        // Build result with metadata if present
        let mut result = response.result.unwrap_or_else(|| json!({}));

        // Merge metadata into result if both are objects
        if let (Some(result_obj), Some(metadata)) = (result.as_object_mut(), response.metadata) {
            for (key, value) in metadata {
                // Only add metadata fields not already in result
                if !result_obj.contains_key(&key) {
                    result_obj.insert(key, value);
                }
            }
        }

        ToolResult::ok(result)
    } else {
        let error_message = response.error.unwrap_or_else(|| "Unknown error".to_owned());
        ToolResult::error(json!({ "error": error_message }))
    }
}

// ============================================================================
// GetActivitiesTool - Retrieve user activities
// ============================================================================

/// Tool for retrieving user activities from fitness providers.
///
/// Supports filtering by sport type, date ranges, pagination, and
/// different output modes (summary/detailed) and formats (json/toon).
pub struct GetActivitiesTool;

#[async_trait]
impl McpTool for GetActivitiesTool {
    fn name(&self) -> &'static str {
        "get_activities"
    }

    fn description(&self) -> &'static str {
        "Retrieve user's fitness activities from connected providers with optional filtering by sport type, date range, and pagination support"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();

        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query (e.g., 'strava', 'fitbit'). Defaults to configured default provider.".to_owned(),
                ),
            },
        );

        properties.insert(
            "limit".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some(
                    "Maximum number of activities to return. Defaults to format-aware limit to prevent context overflow.".to_owned(),
                ),
            },
        );

        properties.insert(
            "offset".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Number of activities to skip for pagination.".to_owned()),
            },
        );

        properties.insert(
            "sport_type".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Filter by sport type (e.g., 'run', 'ride', 'swim'). Case-insensitive."
                        .to_owned(),
                ),
            },
        );

        properties.insert(
            "before".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some(
                    "Unix timestamp - return activities before this time.".to_owned(),
                ),
            },
        );

        properties.insert(
            "after".to_owned(),
            PropertySchema {
                property_type: "integer".to_owned(),
                description: Some("Unix timestamp - return activities after this time.".to_owned()),
            },
        );

        properties.insert(
            "mode".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Output mode: 'summary' (default, minimal fields) or 'detailed' (full activity data).".to_owned(),
                ),
            },
        );

        properties.insert(
            "format".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Output format: 'json' (default) or 'toon' (token-efficient for LLMs)."
                        .to_owned(),
                ),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None, // All parameters are optional
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let executor = UniversalExecutor::new(context.resources.clone());
        let request = build_universal_request("get_activities", &args, context);

        match handle_get_activities(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to get activities: {}", e),
                "error_type": "protocol_error"
            }))),
        }
    }
}

// ============================================================================
// GetAthleteTool - Get athlete profile
// ============================================================================

/// Tool for retrieving the user's athlete profile from a fitness provider.
pub struct GetAthleteTool;

#[async_trait]
impl McpTool for GetAthleteTool {
    fn name(&self) -> &'static str {
        "get_athlete"
    }

    fn description(&self) -> &'static str {
        "Retrieve the user's athlete profile from connected fitness providers including personal details and preferences"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();

        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query (e.g., 'strava', 'fitbit'). Defaults to configured default provider.".to_owned(),
                ),
            },
        );

        properties.insert(
            "format".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Output format: 'json' (default) or 'toon' (token-efficient for LLMs)."
                        .to_owned(),
                ),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let executor = UniversalExecutor::new(context.resources.clone());
        let request = build_universal_request("get_athlete", &args, context);

        match handle_get_athlete(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to get athlete profile: {}", e),
                "error_type": "protocol_error"
            }))),
        }
    }
}

// ============================================================================
// GetStatsTool - Get activity statistics
// ============================================================================

/// Tool for retrieving aggregated activity statistics from a fitness provider.
pub struct GetStatsTool;

#[async_trait]
impl McpTool for GetStatsTool {
    fn name(&self) -> &'static str {
        "get_stats"
    }

    fn description(&self) -> &'static str {
        "Retrieve aggregated activity statistics from connected fitness providers including totals, records, and year-to-date metrics"
    }

    fn input_schema(&self) -> JsonSchema {
        let mut properties = HashMap::new();

        properties.insert(
            "provider".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Fitness provider to query (e.g., 'strava', 'fitbit'). Defaults to configured default provider.".to_owned(),
                ),
            },
        );

        properties.insert(
            "format".to_owned(),
            PropertySchema {
                property_type: "string".to_owned(),
                description: Some(
                    "Output format: 'json' (default) or 'toon' (token-efficient for LLMs)."
                        .to_owned(),
                ),
            },
        );

        JsonSchema {
            schema_type: "object".to_owned(),
            properties: Some(properties),
            required: None,
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::REQUIRES_AUTH | ToolCapabilities::READS_DATA
    }

    async fn execute(&self, args: Value, context: &ToolExecutionContext) -> AppResult<ToolResult> {
        let executor = UniversalExecutor::new(context.resources.clone());
        let request = build_universal_request("get_stats", &args, context);

        match handle_get_stats(&executor, request).await {
            Ok(response) => Ok(convert_to_tool_result(response)),
            Err(e) => Ok(ToolResult::error(json!({
                "error": format!("Failed to get stats: {}", e),
                "error_type": "protocol_error"
            }))),
        }
    }
}

// ============================================================================
// Module exports
// ============================================================================

/// Create all data access tools for registration
#[must_use]
pub fn create_data_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        Box::new(GetActivitiesTool),
        Box::new(GetAthleteTool),
        Box::new(GetStatsTool),
    ]
}
