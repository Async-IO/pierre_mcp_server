// ABOUTME: Integration tests for JSON schema type safety and deserialization
// ABOUTME: Validates that typed parameter structs correctly parse JSON input
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Integration tests for JSON schema types.
//!
//! This module validates that typed parameter structs correctly deserialize
//! JSON input with proper type safety and validation.

use pierre_mcp_server::types::json_schemas::{
    A2ATaskCreateParams, ConfigValueInput, UpdateConfigurationRequest,
};

#[test]
fn test_config_value_input_deserialization() -> Result<(), Box<dyn std::error::Error>> {
    // Test float (use a value that's not a mathematical constant)
    let json = serde_json::json!(5.25);
    let value: ConfigValueInput = serde_json::from_value(json)?;
    assert!(
        matches!(value, ConfigValueInput::Float(f) if (f - 5.25).abs() < f64::EPSILON),
        "Expected Float(5.25), got {value:?}"
    );

    // Test integer - Note: JSON numbers without decimals might be parsed as Float or Integer
    // depending on serde_json's behavior. We accept either as long as conversion works.
    let json = serde_json::json!(42);
    let value: ConfigValueInput = serde_json::from_value(json)?;
    // Accept either Float(42.0) or Integer(42) - both convert correctly
    match value {
        ConfigValueInput::Float(f) => {
            assert!((f - 42.0).abs() < f64::EPSILON, "Expected 42.0, got {f}");
        }
        ConfigValueInput::Integer(i) => {
            assert_eq!(i, 42, "Expected 42, got {i}");
        }
        ConfigValueInput::Boolean(_) | ConfigValueInput::String(_) => {
            return Err(format!("Expected numeric value, got {value:?}").into());
        }
    }

    // Test boolean
    let json = serde_json::json!(true);
    let value: ConfigValueInput = serde_json::from_value(json)?;
    assert!(
        matches!(value, ConfigValueInput::Boolean(true)),
        "Expected Boolean(true), got {value:?}"
    );

    // Test string
    let json = serde_json::json!("test");
    let value: ConfigValueInput = serde_json::from_value(json)?;
    assert!(
        matches!(value, ConfigValueInput::String(ref s) if s == "test"),
        "Expected String(\"test\"), got {value:?}"
    );

    Ok(())
}

#[test]
fn test_update_configuration_request_parsing() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::json!({
        "profile": "endurance",
        "parameter_overrides": {
            "threshold_heart_rate": 165,
            "enable_auto_detection": true,
            "vo2_max": 55.5
        }
    });

    let request: UpdateConfigurationRequest = serde_json::from_value(json)?;

    assert_eq!(
        request.profile,
        Some("endurance".to_owned()),
        "Profile should be 'endurance'"
    );
    assert_eq!(
        request.parameter_overrides.len(),
        3,
        "Should have 3 parameter overrides"
    );

    Ok(())
}

#[test]
fn test_a2a_task_create_params_with_alias() -> Result<(), Box<dyn std::error::Error>> {
    // Test with 'task_type' field
    let json1 = serde_json::json!({
        "client_id": "test-client",
        "task_type": "analysis"
    });

    let params1: A2ATaskCreateParams = serde_json::from_value(json1)?;
    assert_eq!(
        params1.task_type, "analysis",
        "task_type should be 'analysis'"
    );

    // Test with 'type' alias
    let json2 = serde_json::json!({
        "client_id": "test-client",
        "type": "analysis"
    });

    let params2: A2ATaskCreateParams = serde_json::from_value(json2)?;
    assert_eq!(
        params2.task_type, "analysis",
        "task_type should be 'analysis' when using 'type' alias"
    );

    Ok(())
}

#[test]
fn test_deny_unknown_fields() {
    let json = serde_json::json!({
        "profile": "endurance",
        "unknown_field": "should_fail"
    });

    let result: Result<UpdateConfigurationRequest, _> = serde_json::from_value(json);

    assert!(
        result.is_err(),
        "Should fail to deserialize with unknown field"
    );
}

#[test]
fn test_config_value_conversion_to_internal() {
    use pierre_mcp_server::config::runtime::ConfigValue;

    // Test Float conversion
    let input = ConfigValueInput::Float(5.25);
    let config_val = input.to_config_value();
    assert!(
        matches!(config_val, ConfigValue::Float(f) if (f - 5.25).abs() < f64::EPSILON),
        "Float conversion failed"
    );

    // Test Integer conversion
    let input = ConfigValueInput::Integer(42);
    let config_val = input.to_config_value();
    assert!(
        matches!(config_val, ConfigValue::Integer(42)),
        "Integer conversion failed"
    );

    // Test Boolean conversion
    let input = ConfigValueInput::Boolean(true);
    let config_val = input.to_config_value();
    assert!(
        matches!(config_val, ConfigValue::Boolean(true)),
        "Boolean conversion failed"
    );

    // Test String conversion
    let input = ConfigValueInput::String("test".to_owned());
    let config_val = input.to_config_value();
    assert!(
        matches!(config_val, ConfigValue::String(ref s) if s == "test"),
        "String conversion failed"
    );
}
