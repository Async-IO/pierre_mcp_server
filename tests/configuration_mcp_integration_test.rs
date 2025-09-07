// ABOUTME: Integration tests for configuration system MCP protocol exposure
// ABOUTME: Verifies configuration tools are properly exposed through MCP schema
//! Integration test for configuration system MCP protocol exposure
//!
//! Verifies that configuration tools are properly exposed through the MCP schema
//! and can be executed in both single-tenant and multi-tenant modes.

use pierre_mcp_server::mcp::schema::get_tools;

#[test]
fn test_configuration_tools_in_mcp_schema() {
    // Get all available MCP tools
    let tools = get_tools();

    // Define expected configuration tools
    let expected_config_tools = vec![
        "get_configuration_catalog",
        "get_configuration_profiles",
        "get_user_configuration",
        "update_user_configuration",
        "calculate_personalized_zones",
        "validate_configuration",
    ];

    // Check that all configuration tools are present in the schema
    for expected_tool in &expected_config_tools {
        let tool_found = tools.iter().any(|tool| tool.name == *expected_tool);
        assert!(
            tool_found,
            "Configuration tool '{}' not found in MCP schema. Available tools: {:?}",
            expected_tool,
            tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }

    println!(
        "All {} configuration tools found in MCP schema",
        expected_config_tools.len()
    );

    // Verify total tool count includes our configuration tools
    let total_tools = tools.len();
    assert!(
        total_tools >= 29, // 23 fitness tools + 6 configuration tools
        "Expected at least 29 tools (23 fitness + 6 configuration), but found {total_tools}"
    );

    println!("Total of {total_tools} tools available in MCP schema");
}

#[test]
fn test_configuration_tool_schemas() {
    let tools = get_tools();

    // Test get_configuration_catalog tool schema
    let catalog_tool = tools
        .iter()
        .find(|tool| tool.name == "get_configuration_catalog")
        .expect("get_configuration_catalog tool should exist");

    assert_eq!(
        catalog_tool.description,
        "Get the complete configuration catalog with all available parameters and their metadata"
    );
    assert_eq!(catalog_tool.input_schema.schema_type, "object");
    assert_eq!(catalog_tool.input_schema.required, Some(vec![]));

    // Test update_user_configuration tool schema
    let update_tool = tools
        .iter()
        .find(|tool| tool.name == "update_user_configuration")
        .expect("update_user_configuration tool should exist");

    assert!(update_tool
        .description
        .contains("Update user's configuration"));

    if let Some(properties) = &update_tool.input_schema.properties {
        assert!(properties.contains_key("profile"));
        assert!(properties.contains_key("parameters"));
    } else {
        panic!("update_user_configuration should have input properties");
    }

    // Test calculate_personalized_zones tool schema
    let zones_tool = tools
        .iter()
        .find(|tool| tool.name == "calculate_personalized_zones")
        .expect("calculate_personalized_zones tool should exist");

    assert!(zones_tool
        .description
        .contains("personalized training zones"));
    assert_eq!(
        zones_tool.input_schema.required,
        Some(vec!["vo2_max".to_string()])
    );

    if let Some(properties) = &zones_tool.input_schema.properties {
        assert!(properties.contains_key("vo2_max"));
        assert!(properties.contains_key("resting_hr"));
        assert!(properties.contains_key("max_hr"));
        assert!(properties.contains_key("lactate_threshold"));
        assert!(properties.contains_key("sport_efficiency"));
    } else {
        panic!("calculate_personalized_zones should have input properties");
    }

    // Test validate_configuration tool schema
    let validate_tool = tools
        .iter()
        .find(|tool| tool.name == "validate_configuration")
        .expect("validate_configuration tool should exist");

    assert!(validate_tool
        .description
        .contains("Validate configuration parameters"));
    assert_eq!(
        validate_tool.input_schema.required,
        Some(vec!["parameters".to_string()])
    );

    println!("All configuration tool schemas are properly defined");
}

#[test]
fn test_configuration_tools_have_proper_parameter_types() {
    let tools = get_tools();

    // Test calculate_personalized_zones parameter types
    let zones_tool = tools
        .iter()
        .find(|tool| tool.name == "calculate_personalized_zones")
        .expect("calculate_personalized_zones tool should exist");

    if let Some(properties) = &zones_tool.input_schema.properties {
        // vo2_max should be number type
        if let Some(vo2_max_prop) = properties.get("vo2_max") {
            assert_eq!(vo2_max_prop.property_type, "number");
            assert!(vo2_max_prop.description.is_some());
        } else {
            panic!("vo2_max property should exist");
        }

        // resting_hr should be number type
        if let Some(resting_hr_prop) = properties.get("resting_hr") {
            assert_eq!(resting_hr_prop.property_type, "number");
        } else {
            panic!("resting_hr property should exist");
        }
    }

    // Test update_user_configuration parameter types
    let update_tool = tools
        .iter()
        .find(|tool| tool.name == "update_user_configuration")
        .expect("update_user_configuration tool should exist");

    if let Some(properties) = &update_tool.input_schema.properties {
        // profile should be string type
        if let Some(profile_prop) = properties.get("profile") {
            assert_eq!(profile_prop.property_type, "string");
        }

        // parameters should be object type
        if let Some(params_prop) = properties.get("parameters") {
            assert_eq!(params_prop.property_type, "object");
        }
    }

    println!("All configuration tool parameters have correct types");
}

#[tokio::test]
async fn test_configuration_tools_count_in_total() {
    // Verify we have the expected total number of tools
    let tools = get_tools();

    // Count fitness tools (should be 23)
    let fitness_tools = tools
        .iter()
        .filter(|tool| {
            !tool.name.starts_with("get_configuration")
                && !tool.name.starts_with("get_user_configuration")
                && !tool.name.starts_with("update_user_configuration")
                && !tool.name.starts_with("calculate_personalized")
                && !tool.name.starts_with("validate_configuration")
        })
        .count();

    // Count configuration tools (should be 6)
    let config_tools: Vec<_> = tools
        .iter()
        .filter(|tool| {
            tool.name.starts_with("get_configuration")
                || tool.name.starts_with("get_user_configuration")
                || tool.name.starts_with("update_user_configuration")
                || tool.name.starts_with("calculate_personalized")
                || tool.name.starts_with("validate_configuration")
        })
        .collect();

    println!(
        "Fitness tools: {}, Configuration tools: {}, Total: {}",
        fitness_tools,
        config_tools.len(),
        tools.len()
    );

    println!("Found configuration tools:");
    for tool in &config_tools {
        println!("  - {}", tool.name);
    }

    assert_eq!(
        config_tools.len(),
        6,
        "Expected exactly 6 configuration tools"
    );
    assert_eq!(fitness_tools, 27, "Expected exactly 27 fitness tools");
    assert_eq!(tools.len(), 33, "Expected total of 33 tools");
}
