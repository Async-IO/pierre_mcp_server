// ABOUTME: Schema completeness validation - ensures all tools in schema are properly registered
// ABOUTME: Prevents regressions like "Unknown tool" errors by validating schema/registry consistency

#![allow(clippy::unwrap_used)]

use pierre_mcp_server::mcp::schema;
use pierre_mcp_server::protocols::universal::tool_registry::ToolId;
use std::collections::HashSet;

#[test]
fn test_all_schema_tools_are_registered() {
    // Get all tools from MCP schema (what Claude Desktop sees)
    let schema_tools = schema::get_tools();
    let schema_tool_names: HashSet<String> = schema_tools.iter().map(|t| t.name.clone()).collect();

    println!("Found {} tools in MCP schema", schema_tool_names.len());

    // Get all tools registered in ToolId enum (unified execution path)
    let registered_tools: HashSet<String> = schema_tool_names
        .iter()
        .filter(|name| ToolId::from_name(name).is_some())
        .cloned()
        .collect();

    println!(
        "Found {} tools registered in ToolId enum",
        registered_tools.len()
    );

    // Find tools in schema but NOT in ToolId (will cause "Unknown tool" errors)
    let missing_from_registry: Vec<_> = schema_tool_names
        .difference(&registered_tools)
        .cloned()
        .collect();

    // Report findings
    if !missing_from_registry.is_empty() {
        println!("\n❌ CRITICAL: Tools in schema but NOT in ToolId enum:");
        for tool in &missing_from_registry {
            println!("   - {tool}");
        }
        println!("\nThese tools will cause 'Unknown tool' errors in Claude Desktop!");
        println!("Add them to ToolId enum in src/protocols/universal/tool_registry.rs");
    }

    // FAIL if any mismatches found
    assert!(
        missing_from_registry.is_empty(),
        "Schema contains {} tools not in ToolId enum: {:?}\nAll tools MUST be registered in ToolId enum for unified execution",
        missing_from_registry.len(),
        missing_from_registry
    );

    println!(
        "\n✅ All {} tools are properly registered in ToolId enum (unified execution path)",
        schema_tool_names.len()
    );
}

#[test]
fn test_critical_tools_are_present() {
    // Tools that MUST exist (from the original bug reports)
    let critical_tools = vec![
        "get_activity_intelligence",
        "calculate_recovery_score",
        "get_activities",
        "get_athlete",
        "connect_provider",
        "get_connection_status",
    ];

    let schema_tools = schema::get_tools();
    let schema_tool_names: HashSet<String> = schema_tools.iter().map(|t| t.name.clone()).collect();

    for tool in critical_tools {
        assert!(
            schema_tool_names.contains(tool),
            "Critical tool '{tool}' is missing from schema! This will break Claude Desktop integration."
        );
    }

    println!("✅ All critical tools are present in schema");
}

#[test]
fn test_tool_schemas_have_valid_structure() {
    // Validate each tool schema has required fields
    let tools = schema::get_tools();

    for tool in &tools {
        // Tool must have a name
        assert!(!tool.name.is_empty(), "Tool has empty name: {tool:?}");

        // Tool must have a description
        assert!(
            !tool.description.is_empty(),
            "Tool '{}' has empty description",
            tool.name
        );

        // Tool must have schema type
        assert_eq!(
            tool.input_schema.schema_type, "object",
            "Tool '{}' schema type must be 'object', got '{}'",
            tool.name, tool.input_schema.schema_type
        );

        // If tool has required fields, they must exist in properties
        if let Some(ref required) = tool.input_schema.required {
            if let Some(ref properties) = tool.input_schema.properties {
                for req_field in required {
                    assert!(
                        properties.contains_key(req_field),
                        "Tool '{}' requires field '{}' but it's not in properties",
                        tool.name,
                        req_field
                    );
                }
            } else if !required.is_empty() {
                // Use assert! instead of panic! for test assertions
                assert!(
                    false,
                    "Tool '{}' has required fields {required:?} but no properties defined",
                    tool.name
                );
            }
        }
    }

    println!("✅ All {} tool schemas have valid structure", tools.len());
}

#[test]
fn test_provider_parameter_consistency() {
    // Tools that require 'provider' parameter (from original bug #1)
    let provider_tools = vec![
        "get_activities",
        "get_athlete",
        "get_stats",
        "get_activity_intelligence", // This was the bug - must have 'provider'
        "calculate_recovery_score",  // And this one
        "suggest_rest_day",
        "analyze_activity",
        "compare_activities",
    ];

    let tools = schema::get_tools();

    for tool_name in provider_tools {
        let tool = tools
            .iter()
            .find(|t| t.name == tool_name)
            .expect(&format!("Tool '{tool_name}' not found in schema"));

        // Check if 'provider' is in required fields
        let has_provider_required = tool
            .input_schema
            .required
            .as_ref()
            .is_some_and(|r| r.contains(&"provider".to_owned()));

        // Check if 'provider' is in properties
        let has_provider_property = tool
            .input_schema
            .properties
            .as_ref()
            .is_some_and(|p| p.contains_key("provider"));

        assert!(
            has_provider_required,
            "Tool '{tool_name}' must have 'provider' in required fields (this was bug #1)"
        );

        assert!(
            has_provider_property,
            "Tool '{tool_name}' must have 'provider' in properties"
        );

        println!("✅ Tool '{tool_name}' correctly requires 'provider' parameter");
    }
}
