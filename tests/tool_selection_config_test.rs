// ABOUTME: Unit tests for tool selection configuration
// ABOUTME: Validates environment variable loading, TTL clamping, and default values
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use pierre_mcp_server::config::ToolSelectionConfig;
use pierre_mcp_server::constants::tool_selection;
use serial_test::serial;
use std::env;
use std::time::Duration;

#[test]
fn test_default_config_values() {
    let config = ToolSelectionConfig::default();

    assert!(config.enabled);
    assert_eq!(config.cache_size, tool_selection::DEFAULT_CACHE_SIZE);
    assert_eq!(
        config.cache_ttl_secs,
        tool_selection::DEFAULT_CACHE_TTL_SECS
    );
    assert_eq!(
        config.max_overrides_per_tenant,
        tool_selection::MAX_OVERRIDES_PER_TENANT
    );
}

#[test]
fn test_cache_ttl_method() {
    let config = ToolSelectionConfig::default();

    assert_eq!(
        config.cache_ttl(),
        Duration::from_secs(tool_selection::DEFAULT_CACHE_TTL_SECS)
    );
}

#[test]
#[serial]
fn test_from_env_enabled_true() {
    env::set_var("TOOL_SELECTION_ENABLED", "true");

    let config = ToolSelectionConfig::from_env();
    assert!(config.enabled);

    env::remove_var("TOOL_SELECTION_ENABLED");
}

#[test]
#[serial]
fn test_from_env_enabled_false() {
    env::set_var("TOOL_SELECTION_ENABLED", "false");

    let config = ToolSelectionConfig::from_env();
    assert!(!config.enabled);

    env::remove_var("TOOL_SELECTION_ENABLED");
}

#[test]
#[serial]
fn test_from_env_enabled_numeric() {
    env::set_var("TOOL_SELECTION_ENABLED", "1");

    let config = ToolSelectionConfig::from_env();
    assert!(config.enabled);

    env::remove_var("TOOL_SELECTION_ENABLED");
}

#[test]
#[serial]
fn test_from_env_cache_size() {
    env::set_var("TOOL_SELECTION_CACHE_SIZE", "500");

    let config = ToolSelectionConfig::from_env();
    assert_eq!(config.cache_size, 500);

    env::remove_var("TOOL_SELECTION_CACHE_SIZE");
}

#[test]
#[serial]
fn test_from_env_cache_ttl() {
    env::set_var("TOOL_SELECTION_CACHE_TTL_SECS", "600");

    let config = ToolSelectionConfig::from_env();
    assert_eq!(config.cache_ttl_secs, 600);

    env::remove_var("TOOL_SELECTION_CACHE_TTL_SECS");
}

#[test]
#[serial]
fn test_from_env_max_overrides() {
    env::set_var("TOOL_SELECTION_MAX_OVERRIDES", "50");

    let config = ToolSelectionConfig::from_env();
    assert_eq!(config.max_overrides_per_tenant, 50);

    env::remove_var("TOOL_SELECTION_MAX_OVERRIDES");
}

#[test]
#[serial]
fn test_ttl_clamp_minimum() {
    // TTL below minimum should be clamped to MIN_CACHE_TTL_SECS
    env::set_var("TOOL_SELECTION_CACHE_TTL_SECS", "1");

    let config = ToolSelectionConfig::from_env();
    assert_eq!(config.cache_ttl_secs, tool_selection::MIN_CACHE_TTL_SECS);

    env::remove_var("TOOL_SELECTION_CACHE_TTL_SECS");
}

#[test]
#[serial]
fn test_ttl_clamp_maximum() {
    // TTL above maximum should be clamped to MAX_CACHE_TTL_SECS
    env::set_var("TOOL_SELECTION_CACHE_TTL_SECS", "100000");

    let config = ToolSelectionConfig::from_env();
    assert_eq!(config.cache_ttl_secs, tool_selection::MAX_CACHE_TTL_SECS);

    env::remove_var("TOOL_SELECTION_CACHE_TTL_SECS");
}

#[test]
#[serial]
fn test_from_env_invalid_values_use_defaults() {
    // Invalid values should fall back to defaults
    env::set_var("TOOL_SELECTION_CACHE_SIZE", "not_a_number");
    env::set_var("TOOL_SELECTION_CACHE_TTL_SECS", "invalid");
    env::set_var("TOOL_SELECTION_MAX_OVERRIDES", "xyz");

    let config = ToolSelectionConfig::from_env();

    assert_eq!(config.cache_size, tool_selection::DEFAULT_CACHE_SIZE);
    assert_eq!(
        config.cache_ttl_secs,
        tool_selection::DEFAULT_CACHE_TTL_SECS
    );
    assert_eq!(
        config.max_overrides_per_tenant,
        tool_selection::MAX_OVERRIDES_PER_TENANT
    );

    env::remove_var("TOOL_SELECTION_CACHE_SIZE");
    env::remove_var("TOOL_SELECTION_CACHE_TTL_SECS");
    env::remove_var("TOOL_SELECTION_MAX_OVERRIDES");
}

#[test]
#[serial]
fn test_from_env_all_values() {
    env::set_var("TOOL_SELECTION_ENABLED", "false");
    env::set_var("TOOL_SELECTION_CACHE_SIZE", "2000");
    env::set_var("TOOL_SELECTION_CACHE_TTL_SECS", "1800");
    env::set_var("TOOL_SELECTION_MAX_OVERRIDES", "200");

    let config = ToolSelectionConfig::from_env();

    assert!(!config.enabled);
    assert_eq!(config.cache_size, 2000);
    assert_eq!(config.cache_ttl_secs, 1800);
    assert_eq!(config.max_overrides_per_tenant, 200);

    env::remove_var("TOOL_SELECTION_ENABLED");
    env::remove_var("TOOL_SELECTION_CACHE_SIZE");
    env::remove_var("TOOL_SELECTION_CACHE_TTL_SECS");
    env::remove_var("TOOL_SELECTION_MAX_OVERRIDES");
}

#[test]
fn test_config_serialization() {
    let config = ToolSelectionConfig::default();

    // Should be serializable to JSON
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"enabled\":true"));
    assert!(json.contains("\"cache_size\":1000"));

    // Should be deserializable from JSON
    let deserialized: ToolSelectionConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.enabled, config.enabled);
    assert_eq!(deserialized.cache_size, config.cache_size);
    assert_eq!(deserialized.cache_ttl_secs, config.cache_ttl_secs);
    assert_eq!(
        deserialized.max_overrides_per_tenant,
        config.max_overrides_per_tenant
    );
}

#[test]
fn test_ttl_boundary_values() {
    // Test exact boundary values
    let config_min = ToolSelectionConfig {
        cache_ttl_secs: tool_selection::MIN_CACHE_TTL_SECS,
        ..Default::default()
    };
    assert_eq!(
        config_min.cache_ttl(),
        Duration::from_secs(tool_selection::MIN_CACHE_TTL_SECS)
    );

    let config_max = ToolSelectionConfig {
        cache_ttl_secs: tool_selection::MAX_CACHE_TTL_SECS,
        ..Default::default()
    };
    assert_eq!(
        config_max.cache_ttl(),
        Duration::from_secs(tool_selection::MAX_CACHE_TTL_SECS)
    );
}
