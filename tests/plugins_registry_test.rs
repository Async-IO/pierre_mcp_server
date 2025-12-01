// ABOUTME: Integration tests for plugin registry functionality
// ABOUTME: Tests plugin registration, filtering, and management operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(missing_docs)]

use async_trait::async_trait;
use pierre_mcp_server::plugins::{
    core::{PluginCategory, PluginInfo, PluginTool},
    registry::PluginRegistry,
    PluginEnvironment, PluginResult,
};
use pierre_mcp_server::protocols::{ProtocolError, UniversalRequest};

struct TestPlugin;

#[async_trait]
impl PluginTool for TestPlugin {
    fn info(&self) -> &PluginInfo {
        &PluginInfo {
            name: "test_plugin",
            description: "Test plugin for registry",
            input_schema: r#"{"type": "object"}"#,
            version: "1.0.0",
            credit_cost: 1,
            author: "Test Author",
            category: PluginCategory::Community,
        }
    }

    async fn execute(
        &self,
        _request: UniversalRequest,
        _env: PluginEnvironment<'_>,
    ) -> Result<PluginResult, ProtocolError> {
        Ok(pierre_mcp_server::plugins::core::plugin_success(
            serde_json::json!({"test": true}),
            1,
            100,
        ))
    }
}

#[test]
fn test_plugin_registry_creation() {
    let registry = PluginRegistry::new();
    // Registry should be successfully created - no plugins initially
    assert!(!registry.has_plugin("nonexistent_plugin"));
}

#[test]
fn test_plugin_registration() {
    let mut registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin);

    registry.register_plugin(plugin).unwrap();
    assert!(registry.has_plugin("test_plugin"));
    assert!(registry.get_plugin_info("test_plugin").is_some());
}

#[test]
fn test_plugin_filtering() {
    let mut registry = PluginRegistry::new();
    let plugin = Box::new(TestPlugin);
    registry.register_plugin(plugin).unwrap();

    let community_plugins = registry.get_plugins_by_category(PluginCategory::Community);
    assert!(!community_plugins.is_empty());
}
