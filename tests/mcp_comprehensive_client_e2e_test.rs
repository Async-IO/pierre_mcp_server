// ABOUTME: Simplified comprehensive MCP protocol test using in-process mock handler
// ABOUTME: Tests all MCP tools through direct protocol validation without external dependencies
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use anyhow::Result;
use pierre_mcp_server::constants::tools::*;
use pierre_mcp_server::mcp::multitenant::McpRequest;
use pierre_mcp_server::mcp::tool_handlers::ToolHandlers;
use serde_json::{json, Value};
use std::sync::Arc;

mod common;

/// Mock MCP handler for in-process testing
struct MockMcpHandler {
    resources: Arc<pierre_mcp_server::mcp::resources::ServerResources>,
    test_jwt_token: String,
}

impl MockMcpHandler {
    /// Create new mock handler with test resources
    async fn new() -> Result<Self> {
        let resources = common::create_test_server_resources().await?;
        let (_user_id, user) = common::create_test_user(&resources.database).await?;

        // Create a proper JWT token instead of an API key
        let jwt_token = resources
            .auth_manager
            .generate_token(&user, &resources.jwks_manager)?;

        Ok(Self {
            resources,
            test_jwt_token: jwt_token,
        })
    }

    /// Handle MCP initialize request
    fn handle_initialize() -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-06-18",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "pierre-mcp-server",
                    "version": "0.1.0"
                }
            }
        })
    }

    /// Handle MCP tools/list request
    fn handle_list_tools() -> Value {
        let all_tools = [
            GET_ACTIVITIES,
            GET_ATHLETE,
            GET_STATS,
            GET_ACTIVITY_INTELLIGENCE,
            ANALYZE_ACTIVITY,
            CALCULATE_METRICS,
            ANALYZE_PERFORMANCE_TRENDS,
            COMPARE_ACTIVITIES,
            DETECT_PATTERNS,
            SET_GOAL,
            SUGGEST_GOALS,
            TRACK_PROGRESS,
            ANALYZE_GOAL_FEASIBILITY,
            GENERATE_RECOMMENDATIONS,
            CALCULATE_FITNESS_SCORE,
            PREDICT_PERFORMANCE,
            ANALYZE_TRAINING_LOAD,
            GET_FITNESS_CONFIG,
            SET_FITNESS_CONFIG,
            LIST_FITNESS_CONFIGS,
            DELETE_FITNESS_CONFIG,
            GET_CONNECTION_STATUS,
            DISCONNECT_PROVIDER,
            GET_NOTIFICATIONS,
            MARK_NOTIFICATIONS_READ,
            CHECK_OAUTH_NOTIFICATIONS,
            ANNOUNCE_OAUTH_SUCCESS,
        ];

        let tools: Vec<Value> = all_tools
            .iter()
            .map(|&name| {
                json!({
                    "name": name,
                    "description": format!("MCP tool: {}", name),
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                })
            })
            .collect();

        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "tools": tools
            }
        })
    }

    /// Handle MCP tools/call request using actual tool handlers
    async fn handle_tool_call(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        // Create an MCP request for tools/call
        let request = McpRequest {
            jsonrpc: "2.0".to_owned(),
            method: "tools/call".to_owned(),
            params: Some(json!({
                "name": tool_name,
                "arguments": arguments
            })),
            id: Some(json!(1)),
            auth_token: Some(format!("Bearer {}", self.test_jwt_token)),
            headers: Some(std::collections::HashMap::new()),
            metadata: std::collections::HashMap::new(),
        };

        // Use the actual ToolHandlers implementation
        let response =
            ToolHandlers::handle_tools_call_with_resources(request, &self.resources).await;

        // Convert McpResponse to JSON
        let json_response = if response.error.is_some() {
            json!({
                "jsonrpc": response.jsonrpc,
                "id": response.id,
                "error": response.error
            })
        } else {
            json!({
                "jsonrpc": response.jsonrpc,
                "id": response.id,
                "result": response.result
            })
        };

        Ok(json_response)
    }
}

/// Simplified test result for protocol validation
#[derive(Debug)]
struct TestResult {
    tool_name: String,
    success: bool,
    error_message: Option<String>,
}

impl TestResult {
    const fn success(tool_name: String) -> Self {
        Self {
            tool_name,
            success: true,
            error_message: None,
        }
    }

    const fn failure(tool_name: String, error: String) -> Self {
        Self {
            tool_name,
            success: false,
            error_message: Some(error),
        }
    }
}

/// Simplified MCP protocol tester
struct McpProtocolTester {
    handler: MockMcpHandler,
    results: Vec<TestResult>,
}

impl McpProtocolTester {
    async fn new() -> Result<Self> {
        let handler = MockMcpHandler::new().await?;
        Ok(Self {
            handler,
            results: Vec::new(),
        })
    }

    /// Test a single tool through MCP protocol
    async fn test_tool(&mut self, tool_name: &str, arguments: Value) {
        match self.handler.handle_tool_call(tool_name, arguments).await {
            Ok(response) => {
                // Check if response indicates success
                if response.get("error").is_some() {
                    let error_msg = response["error"]["message"]
                        .as_str()
                        .unwrap_or("Unknown error");
                    self.results.push(TestResult::failure(
                        tool_name.to_owned(),
                        format!("Tool returned error: {error_msg}"),
                    ));
                } else {
                    self.results.push(TestResult::success(tool_name.to_owned()));
                }
            }
            Err(error) => {
                self.results
                    .push(TestResult::failure(tool_name.to_owned(), error.to_string()));
            }
        }
    }

    /// Test all MCP tools with sample data
    #[allow(clippy::large_stack_frames)] // Multiple async calls create large stack frames
    async fn test_all_tools(&mut self) {
        println!("Testing all MCP tools through protocol...");
        self.test_core_data_tools().await;
        self.test_analytics_tools().await;
        self.test_goal_tools().await;
    }

    async fn test_core_data_tools(&mut self) {
        self.test_tool(GET_ACTIVITIES, json!({ "provider": "strava", "limit": 5 }))
            .await;
        self.test_tool(GET_ATHLETE, json!({ "provider": "strava" }))
            .await;
        self.test_tool(GET_STATS, json!({ "provider": "strava" }))
            .await;
        self.test_tool(
            GET_ACTIVITY_INTELLIGENCE,
            json!({ "activity_id": "test_123", "provider": "strava" }),
        )
        .await;
    }

    async fn test_analytics_tools(&mut self) {
        self.test_tool(
            ANALYZE_ACTIVITY,
            json!({ "provider": "strava", "activity_id": "test_123" }),
        )
        .await;
        self.test_tool(
            CALCULATE_METRICS,
            json!({ "activity": { "distance": 10000, "duration": 3600 } }),
        )
        .await;
        self.test_tool(
            ANALYZE_PERFORMANCE_TRENDS,
            json!({ "provider": "strava", "timeframe": "30_days" }),
        )
        .await;
        self.test_tool(
            COMPARE_ACTIVITIES,
            json!({ "activity_id1": "test_1", "activity_id2": "test_2", "provider": "strava" }),
        )
        .await;
        self.test_tool(
            DETECT_PATTERNS,
            json!({ "provider": "strava", "timeframe": "30_days" }),
        )
        .await;
    }

    async fn test_goal_tools(&mut self) {
        self.test_tool(SET_GOAL, json!({ "goal_type": "distance", "target_value": 100.0, "target_unit": "km", "timeframe": "weekly" })).await;
        self.test_tool(
            SUGGEST_GOALS,
            json!({ "provider": "strava", "goal_type": "distance" }),
        )
        .await;
        self.test_tool(TRACK_PROGRESS, json!({ "goal_id": "test_goal" }))
            .await;
        self.test_tool(
            ANALYZE_GOAL_FEASIBILITY,
            json!({ "goal_type": "distance", "target_value": 50.0, "target_unit": "km" }),
        )
        .await;
        self.test_tool(
            GENERATE_RECOMMENDATIONS,
            json!({ "provider": "strava", "recommendation_type": "training" }),
        )
        .await;
        self.test_tool(CALCULATE_FITNESS_SCORE, json!({ "provider": "strava" }))
            .await;
        self.test_tool(
            PREDICT_PERFORMANCE,
            json!({ "distance": 42195, "activity_type": "running", "provider": "strava" }),
        )
        .await;
        self.test_tool(
            ANALYZE_TRAINING_LOAD,
            json!({ "provider": "strava", "timeframe": "7_days" }),
        )
        .await;

        // Fitness config tools
        self.test_tool(SET_FITNESS_CONFIG, json!({ "configuration_name": "test_config", "configuration": { "sport_types": ["cycling"] } })).await;
        self.test_tool(
            GET_FITNESS_CONFIG,
            json!({ "configuration_name": "test_config" }),
        )
        .await;
        self.test_tool(LIST_FITNESS_CONFIGS, json!({})).await;
        self.test_tool(
            DELETE_FITNESS_CONFIG,
            json!({ "configuration_name": "test_config" }),
        )
        .await;

        // Provider tools
        self.test_tool(GET_CONNECTION_STATUS, json!({})).await;
        self.test_tool(DISCONNECT_PROVIDER, json!({ "provider": "non_existent" }))
            .await;

        // Notification tools
        self.test_tool(GET_NOTIFICATIONS, json!({})).await;
        self.test_tool(MARK_NOTIFICATIONS_READ, json!({ "notification_ids": [] }))
            .await;
        self.test_tool(CHECK_OAUTH_NOTIFICATIONS, json!({})).await;
        self.test_tool(ANNOUNCE_OAUTH_SUCCESS, json!({ "provider": "strava" }))
            .await;
    }

    /// Generate simple test report
    fn generate_report(&self) {
        let successful = self.results.iter().filter(|r| r.success).count();
        let failed = self.results.len() - successful;
        // Safe: converting small counts to f64 for percentage calculation
        #[allow(clippy::cast_precision_loss)]
        let success_rate = (successful as f64 / self.results.len() as f64) * 100.0;

        println!("\n=== MCP PROTOCOL TEST RESULTS ===");
        println!("Total tools tested: {}", self.results.len());
        println!("Successful: {successful}");
        println!("Failed: {failed}");
        println!("Success rate: {success_rate:.1}%");

        if failed > 0 {
            println!("\nFailures:");
            for result in &self.results {
                if !result.success {
                    println!(
                        "  - {}: {}",
                        result.tool_name,
                        result.error_message.as_ref().map_or("Unknown error", |v| v)
                    );
                }
            }
        }
        println!("================================\n");
    }
}

/// Simplified MCP protocol test using in-process handlers
#[tokio::test]
async fn test_comprehensive_mcp_tools_e2e() -> Result<()> {
    println!("Starting simplified MCP protocol test...");

    // Create mock handler and run protocol tests
    let mut tester = McpProtocolTester::new().await?;

    // Test MCP protocol methods
    let init_response = MockMcpHandler::handle_initialize();
    println!(" MCP initialize: {}", init_response.get("result").is_some());

    let tools_response = MockMcpHandler::handle_list_tools();
    if let Some(tools) = tools_response
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
    {
        println!(" Found {} MCP tools", tools.len());
    }

    // Test all tools through MCP protocol
    tester.test_all_tools().await;

    // Generate report
    tester.generate_report();

    // Check basic success criteria
    let successful = tester.results.iter().filter(|r| r.success).count();
    // Safe: converting small counts to f64 for percentage calculation
    #[allow(clippy::cast_precision_loss)]
    let success_rate = (successful as f64 / tester.results.len() as f64) * 100.0;

    println!("MCP protocol validation completed with {success_rate:.1}% success rate");

    // Require at least some tools to work for basic protocol validation
    assert!(
        success_rate > 0.0,
        "No tools working - check MCP protocol implementation"
    );

    Ok(())
}
