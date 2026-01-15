// ABOUTME: Tool selection configuration from environment variables
// ABOUTME: Parses PIERRE_DISABLED_TOOLS for global tool filtering across all tenants
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

use std::collections::HashSet;
use std::env;

/// Configuration for global tool selection from environment variables
///
/// This configuration allows operators to globally disable specific MCP tools
/// via the `PIERRE_DISABLED_TOOLS` environment variable. Disabled tools will
/// not be exposed to any tenant regardless of their plan or overrides.
///
/// # Example
///
/// ```bash
/// export PIERRE_DISABLED_TOOLS="predict_performance,get_activity_intelligence"
/// ```
#[derive(Debug, Clone, Default)]
pub struct ToolSelectionConfig {
    /// Set of tool names disabled globally via `PIERRE_DISABLED_TOOLS` env var
    disabled_tools: HashSet<String>,
}

impl ToolSelectionConfig {
    /// Load tool selection configuration from environment variables
    ///
    /// Parses the `PIERRE_DISABLED_TOOLS` environment variable as a comma-separated
    /// list of tool names. Whitespace around tool names is trimmed.
    ///
    /// # Environment Variables
    ///
    /// - `PIERRE_DISABLED_TOOLS`: Comma-separated list of tool names to disable globally
    ///   Example: `"predict_performance,get_activity_intelligence,analyze_training_load"`
    #[must_use]
    pub fn from_env() -> Self {
        let disabled_tools = env::var("PIERRE_DISABLED_TOOLS")
            .ok()
            .map(|s| {
                s.split(',')
                    .map(|t| t.trim().to_owned())
                    .filter(|t| !t.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        Self { disabled_tools }
    }

    /// Create a new configuration with explicitly disabled tools
    ///
    /// This constructor is useful for testing or programmatic configuration.
    #[must_use]
    pub fn with_disabled_tools(tools: Vec<String>) -> Self {
        Self {
            disabled_tools: tools.into_iter().collect(),
        }
    }

    /// Check if a tool is globally disabled
    ///
    /// Returns `true` if the tool name appears in the `PIERRE_DISABLED_TOOLS`
    /// environment variable list.
    #[must_use]
    pub fn is_globally_disabled(&self, tool_name: &str) -> bool {
        self.disabled_tools.contains(tool_name)
    }

    /// Get the list of globally disabled tool names
    ///
    /// Returns a reference to the set of tool names that are globally disabled.
    #[must_use]
    pub const fn disabled_tools(&self) -> &HashSet<String> {
        &self.disabled_tools
    }

    /// Get the number of globally disabled tools
    #[must_use]
    pub fn disabled_count(&self) -> usize {
        self.disabled_tools.len()
    }

    /// Check if any tools are globally disabled
    #[must_use]
    pub fn has_disabled_tools(&self) -> bool {
        !self.disabled_tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
