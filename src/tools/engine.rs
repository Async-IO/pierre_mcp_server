// ABOUTME: Core tool execution engine for processing fitness and analysis operations
// ABOUTME: Handles tool routing, execution, error handling, and response formatting for all protocols
//! Unified tool execution engine
//!
//! This engine provides a single implementation for tool execution that can be
//! used by multi-tenant MCP servers with comprehensive authentication and user isolation.

use crate::database_plugins::factory::Database;
use crate::errors::AppError;
use crate::intelligence::weather::WeatherService;
use crate::intelligence::ActivityAnalyzer;
use crate::protocols::universal::UniversalToolExecutor;
use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use uuid::Uuid;

/// User context for multi-tenant operations
#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: Uuid,
    pub email: String,
    pub tier: String,
}

/// Unified tool execution engine that provides multi-tenant server functionality
pub struct ToolEngine {
    universal_executor: Arc<UniversalToolExecutor>,
}

impl ToolEngine {
    /// Create a new tool engine instance
    #[must_use]
    pub fn new(
        _database: Arc<Database>,
        _intelligence: Arc<ActivityAnalyzer>,
        _weather: Arc<WeatherService>,
        universal_executor: Arc<UniversalToolExecutor>,
    ) -> Self {
        Self { universal_executor }
    }

    /// Execute a tool with unified error handling and context
    ///
    /// This method provides a single point for tool execution for multi-tenant implementations.
    /// # Errors
    ///
    /// Returns an error if:
    /// - User permissions validation fails
    /// - Tool execution fails
    /// - Universal executor returns an error
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        params: Value,
        user_context: Option<&UserContext>,
    ) -> Result<Value, AppError> {
        // Validate permissions for authenticated users
        if let Some(ctx) = user_context {
            self.validate_user_permissions(ctx, tool_name)?;
        }

        // Convert to the universal request format that the existing infrastructure expects
        let universal_request = crate::protocols::universal::UniversalRequest {
            tool_name: tool_name.to_string(),
            parameters: params,
            protocol: "mcp".into(),
            user_id: user_context
                .map_or_else(|| Uuid::new_v4().to_string(), |ctx| ctx.user_id.to_string()),
            tenant_id: None, // UserContext doesn't have tenant_id - needs tenant system integration
        };

        // Execute using the existing universal executor
        match self
            .universal_executor
            .execute_tool(universal_request)
            .await
        {
            Ok(response) => Ok(response.result.unwrap_or(serde_json::Value::Null)),
            Err(e) => {
                // Convert protocol errors to app errors for consistent handling
                Err(AppError::internal(format!(
                    "Tool '{tool_name}' execution failed: {e}"
                )))
            }
        }
    }

    /// Execute a tool for multi-tenant mode with user context
    /// # Errors
    ///
    /// Returns an error if tool execution fails
    pub async fn execute_tool_multi_tenant(
        &self,
        tool_name: &str,
        params: Value,
        user_context: &UserContext,
    ) -> Result<Value, AppError> {
        self.execute_tool(tool_name, params, Some(user_context))
            .await
    }

    /// Get list of available tools
    #[must_use]
    pub const fn list_available_tools(&self) -> &'static [&'static str] {
        &[
            // Data Access Tools
            "get_activities",
            "get_athlete",
            "get_stats",
            // Intelligence Tools
            "get_activity_intelligence",
            "analyze_activity",
            "calculate_metrics",
            // Analytics Tools
            "analyze_performance_trends",
            "compare_activities",
            "detect_patterns",
            // Goal Tools
            "create_goal",
            "get_goals",
            "suggest_goals",
            // Weather Tools
            "get_weather_for_activity",
            // Provider Tools
            "connect_provider",
            "disconnect_provider",
            "get_connection_status",
            // Prediction Tools
            "predict_performance",
            "generate_recommendations",
        ]
    }

    /// Get tool description for MCP schema
    #[must_use]
    pub fn get_tool_description(tool_name: &str) -> Option<&'static str> {
        match tool_name {
            "get_activities" => Some("Fetch fitness activities with pagination support"),
            "get_athlete" => Some("Get complete athlete profile information"),
            "get_stats" => Some("Get aggregated fitness statistics and lifetime metrics"),
            "get_activity_intelligence" => Some("AI-powered activity analysis with full context"),
            "analyze_activity" => Some("Deep dive analysis of individual activities"),
            "calculate_metrics" => {
                Some("Advanced fitness calculations (TRIMP, power ratios, efficiency)")
            }
            "analyze_performance_trends" => Some("Analyze performance trends over time"),
            "compare_activities" => Some("Compare multiple activities for insights"),
            "detect_patterns" => Some("Detect patterns in training data"),
            "create_goal" => Some("Create a new fitness goal"),
            "get_goals" => Some("Get all user goals"),
            "suggest_goals" => Some("AI-suggested goals based on activity history"),
            "get_weather_for_activity" => Some("Get weather conditions for a specific activity"),
            "connect_provider" => Some("Connect to a fitness data provider (Strava, Fitbit)"),
            "disconnect_provider" => Some("Disconnect from a fitness data provider"),
            "get_connection_status" => Some("Check connection status for all providers"),
            "predict_performance" => Some("Predict future performance based on training data"),
            "generate_recommendations" => Some("Generate personalized training recommendations"),
            _ => None,
        }
    }

    /// Get MCP tool schema for a specific tool
    #[must_use]
    pub fn get_tool_schema(&self, tool_name: &str) -> Option<serde_json::Value> {
        match tool_name {
            "get_activities" => Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of activities to return",
                        "minimum": 1,
                        "maximum": 50,
                        "default": 10
                    },
                    "provider": {
                        "type": "string",
                        "description": "Fitness provider to query",
                        "enum": ["strava", "fitbit"],
                        "default": "strava"
                    }
                }
            })),
            "get_activity_intelligence" => Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "activity_id": {
                        "type": "string",
                        "description": "ID of the activity to analyze"
                    },
                    "analysis_type": {
                        "type": "string",
                        "description": "Type of analysis to perform",
                        "enum": ["performance", "health", "training", "comprehensive"],
                        "default": "comprehensive"
                    }
                },
                "required": ["activity_id"]
            })),
            "get_weather_for_activity" => Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "activity_id": {
                        "type": "string",
                        "description": "Activity ID to get weather for"
                    },
                    "units": {
                        "type": "string",
                        "description": "Temperature units",
                        "enum": ["metric", "imperial", "kelvin"],
                        "default": "metric"
                    }
                },
                "required": ["activity_id"]
            })),
            _ => None,
        }
    }

    /// Get all available tools with their schemas (for MCP capabilities)
    #[must_use]
    pub fn get_all_tool_schemas(&self) -> Vec<crate::mcp::schema::ToolSchema> {
        self.list_available_tools()
            .iter()
            .filter_map(|&tool_name| {
                let description = Self::get_tool_description(tool_name)?.to_string();
                let input_schema = self
                    .get_tool_schema(tool_name)
                    .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));

                Some(crate::mcp::schema::ToolSchema {
                    name: tool_name.to_string(),
                    description,
                    input_schema: crate::mcp::schema::JsonSchema {
                        schema_type: "object".into(),
                        properties: input_schema
                            .get("properties")
                            .cloned()
                            .map(|props| serde_json::from_value(props).unwrap_or_default()),
                        required: input_schema
                            .get("required")
                            .and_then(|req| serde_json::from_value(req.clone()).ok()), // Safe: JSON value ownership for schema parsing
                    },
                })
            })
            .collect()
    }

    /// Validate user permissions for tool execution (for multi-tenant)
    /// # Errors
    ///
    /// Returns an error if user tier is invalid
    pub fn validate_user_permissions(
        &self,
        user_context: &UserContext,
        _tool_name: &str,
    ) -> Result<bool, AppError> {
        // All authenticated users can use all tools
        // This can be extended with more granular permissions based on tiers
        match user_context.tier.as_str() {
            "trial" | "starter" | "professional" | "enterprise" => Ok(true),
            _ => Err(AppError::auth_invalid(format!(
                "Invalid user tier: {}",
                user_context.tier
            ))),
        }
    }
}
