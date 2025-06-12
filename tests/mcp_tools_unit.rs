// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Unit tests for individual MCP analytics tools

use pierre_mcp_server::mcp::schema::*;

#[test]
fn test_mcp_tool_schemas() {
    // Test that all analytics tools are properly defined
    let tools = get_tools();
    
    // Should have all 21 tools
    assert_eq!(tools.len(), 21);
    
    // Check key analytics tools are present
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    
    // Core functionality
    assert!(tool_names.contains(&"get_activities"));
    assert!(tool_names.contains(&"get_athlete"));
    assert!(tool_names.contains(&"get_stats"));
    assert!(tool_names.contains(&"get_activity_intelligence"));
    
    // Connection management
    assert!(tool_names.contains(&"connect_strava"));
    assert!(tool_names.contains(&"connect_fitbit"));
    assert!(tool_names.contains(&"get_connection_status"));
    assert!(tool_names.contains(&"disconnect_provider"));
    
    // Analytics tools
    assert!(tool_names.contains(&"analyze_activity"));
    assert!(tool_names.contains(&"calculate_metrics"));
    assert!(tool_names.contains(&"analyze_performance_trends"));
    assert!(tool_names.contains(&"compare_activities"));
    assert!(tool_names.contains(&"detect_patterns"));
    
    // Goal management
    assert!(tool_names.contains(&"set_goal"));
    assert!(tool_names.contains(&"track_progress"));
    assert!(tool_names.contains(&"suggest_goals"));
    assert!(tool_names.contains(&"analyze_goal_feasibility"));
    
    // Advanced analytics
    assert!(tool_names.contains(&"generate_recommendations"));
    assert!(tool_names.contains(&"calculate_fitness_score"));
    assert!(tool_names.contains(&"predict_performance"));
    assert!(tool_names.contains(&"analyze_training_load"));
}

#[test]
fn test_analytics_tool_schemas() {
    let tools = get_tools();
    
    // Test analyze_activity tool schema
    let analyze_activity = tools.iter()
        .find(|t| t.name == "analyze_activity")
        .expect("analyze_activity tool should exist");
    
    assert_eq!(analyze_activity.description, "Perform deep analysis of an individual activity including insights, metrics, and anomaly detection");
    
    // Check required parameters
    let schema = &analyze_activity.input_schema;
    if let Some(required) = &schema.required {
        assert!(required.contains(&"provider".to_string()));
        assert!(required.contains(&"activity_id".to_string()));
    } else {
        panic!("analyze_activity should have required parameters");
    }
    
    // Test calculate_fitness_score tool
    let fitness_score = tools.iter()
        .find(|t| t.name == "calculate_fitness_score")
        .expect("calculate_fitness_score tool should exist");
    
    assert!(fitness_score.description.contains("comprehensive fitness score"));
    
    // Test goal management tools
    let set_goal = tools.iter()
        .find(|t| t.name == "set_goal")
        .expect("set_goal tool should exist");
    
    if let Some(required) = &set_goal.input_schema.required {
        assert!(required.contains(&"title".to_string()));
        assert!(required.contains(&"goal_type".to_string()));
        assert!(required.contains(&"target_value".to_string()));
        assert!(required.contains(&"target_date".to_string()));
    } else {
        panic!("set_goal should have required parameters");
    }
}

#[test]
fn test_tool_parameter_validation() {
    let tools = get_tools();
    
    for tool in &tools {
        // Each tool should have proper schema structure
        assert_eq!(tool.input_schema.schema_type, "object");
        assert!(tool.input_schema.properties.is_some());
        
        // Required parameters should be valid
        if let Some(required) = &tool.input_schema.required {
            let properties = tool.input_schema.properties.as_ref().unwrap();
            
            for param_name in required {
                assert!(properties.contains_key(param_name), 
                    "Tool {} requires parameter '{}' but it's not in properties", 
                    tool.name, param_name);
            }
        }
    }
}

#[test]
fn test_initialize_response() {
    let response = InitializeResponse::new(
        "2024-11-05".to_string(),
        "pierre-mcp-server-multitenant".to_string(),
        "0.1.0".to_string(),
    );
    
    assert_eq!(response.protocol_version, "2024-11-05");
    assert_eq!(response.server_info.name, "pierre-mcp-server-multitenant");
    assert_eq!(response.server_info.version, "0.1.0");
    assert_eq!(response.capabilities.tools.len(), 21);
}

#[test]
fn test_tool_descriptions_quality() {
    let tools = get_tools();
    
    for tool in &tools {
        // Each tool should have a meaningful description
        assert!(!tool.description.is_empty(), "Tool {} has empty description", tool.name);
        assert!(tool.description.len() > 20, "Tool {} description too short: '{}'", tool.name, tool.description);
        
        // Analytics tools should mention their purpose
        if tool.name.contains("analyze") || tool.name.contains("calculate") {
            assert!(tool.description.to_lowercase().contains("analy") || 
                   tool.description.to_lowercase().contains("calculat") ||
                   tool.description.to_lowercase().contains("assess") ||
                   tool.description.to_lowercase().contains("generat"),
                   "Analytics tool {} should have analysis-related description: '{}'", 
                   tool.name, tool.description);
        }
    }
}

#[test]
fn test_provider_parameter_consistency() {
    let tools = get_tools();
    
    // Tools that should require provider parameter
    let provider_tools = [
        "get_activities", "get_athlete", "get_stats", "get_activity_intelligence",
        "analyze_activity", "calculate_metrics", "analyze_performance_trends",
        "compare_activities", "detect_patterns", "suggest_goals",
        "generate_recommendations", "calculate_fitness_score", 
        "predict_performance", "analyze_training_load"
    ];
    
    for tool_name in &provider_tools {
        let tool = tools.iter()
            .find(|t| t.name == *tool_name)
            .expect(&format!("Tool {} should exist", tool_name));
        
        if let Some(required) = &tool.input_schema.required {
            assert!(required.contains(&"provider".to_string()), 
                "Tool {} should require provider parameter", tool_name);
        } else {
            panic!("Tool {} should have required parameters", tool_name);
        }
        
        if let Some(properties) = &tool.input_schema.properties {
            let provider_prop = &properties["provider"];
            assert_eq!(provider_prop.property_type, "string");
            assert!(provider_prop.description.as_ref().unwrap().contains("provider"));
        } else {
            panic!("Tool {} should have properties", tool_name);
        }
    }
}

#[test]
fn test_goal_tools_consistency() {
    let tools = get_tools();
    
    // Goal-related tools should have consistent parameter naming
    let goal_tools = ["set_goal", "track_progress", "analyze_goal_feasibility"];
    
    for tool_name in &goal_tools {
        let tool = tools.iter()
            .find(|t| t.name == *tool_name)
            .expect(&format!("Tool {} should exist", tool_name));
        
        // Description should mention goals
        assert!(tool.description.to_lowercase().contains("goal"),
            "Goal tool {} should mention 'goal' in description", tool_name);
    }
    
    // set_goal should have comprehensive parameters
    let set_goal = tools.iter().find(|t| t.name == "set_goal").unwrap();
    if let Some(required) = &set_goal.input_schema.required {
        assert!(required.contains(&"title".to_string()));
        assert!(required.contains(&"goal_type".to_string()));
        assert!(required.contains(&"target_value".to_string()));
        assert!(required.contains(&"target_date".to_string()));
    } else {
        panic!("set_goal should have required parameters");
    }
}

/// Test that validates the exact tools we used in our fitness report demo
#[test]
fn test_fitness_report_tools_available() {
    let tools = get_tools();
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    
    // These are the exact tools we used to generate the fitness report
    let required_tools = [
        "get_activities",           // Get 50 activities 
        "calculate_fitness_score",  // 85/100 score
        "analyze_training_load",    // High load analysis
        "generate_recommendations", // Training advice
        "detect_patterns",          // Consistency patterns
        "analyze_performance_trends", // Stable trends
        "get_connection_status",    // Provider status
    ];
    
    for tool_name in &required_tools {
        assert!(tool_names.contains(tool_name), 
            "Required tool '{}' for fitness reporting is missing", tool_name);
    }
    
    // Verify these tools have the parameters we used
    let get_activities = tools.iter().find(|t| t.name == "get_activities").unwrap();
    if let Some(properties) = &get_activities.input_schema.properties {
        assert!(properties.contains_key("limit"));
        assert!(properties.contains_key("provider"));
    } else {
        panic!("get_activities should have properties");
    }
    
    let fitness_score = tools.iter().find(|t| t.name == "calculate_fitness_score").unwrap();
    if let Some(properties) = &fitness_score.input_schema.properties {
        assert!(properties.contains_key("provider"));
        assert!(properties.contains_key("timeframe"));
    } else {
        panic!("calculate_fitness_score should have properties");
    }
}