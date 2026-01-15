// ABOUTME: Tests for ToolSelectionConfig - global tool disabling via environment variable
// ABOUTME: Validates PIERRE_DISABLED_TOOLS parsing and tool lookup functionality
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Tests for [`ToolSelectionConfig`] global tool disabling functionality.

use pierre_mcp_server::config::ToolSelectionConfig;

#[test]
fn test_with_disabled_tools() {
    let config = ToolSelectionConfig::with_disabled_tools(vec![
        "predict_performance".to_owned(),
        "get_activity_intelligence".to_owned(),
    ]);

    assert!(config.is_globally_disabled("predict_performance"));
    assert!(config.is_globally_disabled("get_activity_intelligence"));
    assert!(!config.is_globally_disabled("get_activities"));
    assert_eq!(config.disabled_count(), 2);
}

#[test]
fn test_default_has_no_disabled_tools() {
    let config = ToolSelectionConfig::default();
    assert!(!config.has_disabled_tools());
    assert_eq!(config.disabled_count(), 0);
}

#[test]
fn test_disabled_tools_returns_set() {
    let config =
        ToolSelectionConfig::with_disabled_tools(vec!["tool_a".to_owned(), "tool_b".to_owned()]);

    let disabled = config.disabled_tools();
    assert!(disabled.contains("tool_a"));
    assert!(disabled.contains("tool_b"));
    assert!(!disabled.contains("tool_c"));
}

#[test]
fn test_has_disabled_tools_true_when_tools_present() {
    let config = ToolSelectionConfig::with_disabled_tools(vec!["some_tool".to_owned()]);
    assert!(config.has_disabled_tools());
}

#[test]
fn test_empty_vector_creates_empty_config() {
    let config = ToolSelectionConfig::with_disabled_tools(vec![]);
    assert!(!config.has_disabled_tools());
    assert_eq!(config.disabled_count(), 0);
}
