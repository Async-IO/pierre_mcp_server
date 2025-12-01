// ABOUTME: Plugin registry for managing all available plugins
// ABOUTME: Provides thread-safe, efficient plugin management with Rust-idiomatic patterns
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

// NOTE: All `.clone()` calls in this file are Safe - they are necessary for:
// - Plugin info cloning for registry operations
// - Arc plugin clones for concurrent access

use super::core::{PluginInfo, PluginTool, PluginToolStatic};
use super::PluginEnvironment;
use crate::protocols::universal::UniversalRequest;
use crate::protocols::ProtocolError;
use std::collections::HashMap;
use std::sync::Arc;

/// Plugin registry that manages all available plugins
pub struct PluginRegistry {
    /// Map of plugin name to plugin instance
    plugins: HashMap<String, Arc<dyn PluginTool>>,
    /// Cached plugin information for quick lookup
    plugin_info: HashMap<String, PluginInfo>,
}

impl PluginRegistry {
    /// Create new plugin registry and register all compile-time plugins
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
            plugin_info: HashMap::new(),
        };

        registry.register_builtin_plugins();
        registry
    }

    /// Register all built-in plugins
    fn register_builtin_plugins(&mut self) {
        // Manually register all built-in plugins
        // Note: When adding new plugins, add them to this list
        self.register_plugin(Box::new(
            crate::plugins::community::BasicAnalysisPlugin::new(),
        ))
        .ok();
        self.register_plugin(Box::new(
            crate::plugins::community::WeatherIntegrationPlugin::new(),
        ))
        .ok();

        tracing::info!("Registered {} plugins", self.plugins.len());
    }

    /// Register a plugin dynamically (for testing or runtime registration)
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if plugin name conflicts or registration fails
    pub fn register_plugin(&mut self, plugin: Box<dyn PluginTool>) -> Result<(), ProtocolError> {
        let info = plugin.info().clone();

        // Check for duplicate names
        if self.plugins.contains_key(info.name) {
            return Err(ProtocolError::PluginError {
                plugin_id: info.name.to_owned(),
                details: format!("Plugin '{}' is already registered", info.name),
            });
        }

        // Call plugin lifecycle hook
        plugin
            .on_register()
            .map_err(|e| ProtocolError::PluginError {
                plugin_id: info.name.to_owned(),
                details: format!("Plugin registration failed: {e}"),
            })?;

        tracing::info!("Dynamically registering plugin: {}", info.name);

        let plugin_arc = Arc::from(plugin);
        self.plugins.insert(info.name.to_owned(), plugin_arc);
        self.plugin_info.insert(info.name.to_owned(), info);

        Ok(())
    }

    /// Unregister a plugin
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if plugin not found or unregistration fails
    pub fn unregister_plugin(&mut self, plugin_name: &str) -> Result<(), ProtocolError> {
        let plugin =
            self.plugins
                .remove(plugin_name)
                .ok_or_else(|| ProtocolError::PluginNotFound {
                    plugin_id: plugin_name.to_owned(),
                })?;

        self.plugin_info.remove(plugin_name);

        // Call plugin lifecycle hook
        plugin.on_unregister()?;

        tracing::info!("Unregistered plugin: {}", plugin_name);
        Ok(())
    }

    /// Get all registered plugin names
    #[must_use]
    pub fn list_plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    /// Get plugin information by name
    #[must_use]
    pub fn get_plugin_info(&self, plugin_name: &str) -> Option<&PluginInfo> {
        self.plugin_info.get(plugin_name)
    }

    /// Get all plugin information
    #[must_use]
    pub fn get_all_plugin_info(&self) -> Vec<&PluginInfo> {
        self.plugin_info.values().collect()
    }

    /// Check if plugin exists
    #[must_use]
    pub fn has_plugin(&self, plugin_name: &str) -> bool {
        self.plugins.contains_key(plugin_name)
    }

    /// Execute a plugin by name
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if plugin not found or execution fails
    pub async fn execute_plugin(
        &self,
        plugin_name: &str,
        request: UniversalRequest,
        env: PluginEnvironment<'_>,
    ) -> Result<super::PluginResult, ProtocolError> {
        let plugin =
            self.plugins
                .get(plugin_name)
                .ok_or_else(|| ProtocolError::PluginNotFound {
                    plugin_id: plugin_name.to_owned(),
                })?;

        tracing::debug!(
            "Executing plugin: {} for user: {}",
            plugin_name,
            request.user_id
        );

        plugin.execute(request, env).await
    }

    /// Get plugins by category
    #[must_use]
    pub fn get_plugins_by_category(
        &self,
        category: super::core::PluginCategory,
    ) -> Vec<&PluginInfo> {
        self.plugin_info
            .values()
            .filter(|info| info.category == category)
            .collect()
    }

    /// Get plugin statistics
    #[must_use]
    pub fn get_statistics(&self) -> PluginRegistryStatistics {
        let mut stats = PluginRegistryStatistics {
            total_plugins: self.plugins.len(),
            ..Default::default()
        };

        for info in self.plugin_info.values() {
            match info.category {
                super::core::PluginCategory::DataAccess => stats.data_access_plugins += 1,
                super::core::PluginCategory::Intelligence => stats.intelligence_plugins += 1,
                super::core::PluginCategory::Analytics => stats.analytics_plugins += 1,
                super::core::PluginCategory::Goals => stats.goals_plugins += 1,
                super::core::PluginCategory::Providers => stats.provider_plugins += 1,
                super::core::PluginCategory::Environmental => stats.environmental_plugins += 1,
                super::core::PluginCategory::Community => stats.custom_plugins += 1,
            }
        }

        stats
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Plugin registry statistics for monitoring and analytics
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct PluginRegistryStatistics {
    /// Total number of registered plugins
    pub total_plugins: usize,
    /// Number of data access plugins
    pub data_access_plugins: usize,
    /// Number of intelligence/AI plugins
    pub intelligence_plugins: usize,
    /// Number of analytics plugins
    pub analytics_plugins: usize,
    /// Number of goal tracking plugins
    pub goals_plugins: usize,
    /// Number of provider integration plugins
    pub provider_plugins: usize,
    /// Number of environmental data plugins
    pub environmental_plugins: usize,
    /// Number of custom plugins
    pub custom_plugins: usize,
}

/// Convenience function to create a global plugin registry
static GLOBAL_REGISTRY: std::sync::OnceLock<std::sync::Arc<std::sync::RwLock<PluginRegistry>>> =
    std::sync::OnceLock::new();

/// Get the global plugin registry instance
#[must_use]
pub fn global_registry() -> std::sync::Arc<std::sync::RwLock<PluginRegistry>> {
    GLOBAL_REGISTRY
        .get_or_init(|| std::sync::Arc::new(std::sync::RwLock::new(PluginRegistry::new())))
        .clone()
}
