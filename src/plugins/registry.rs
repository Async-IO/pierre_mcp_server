// ABOUTME: Compile-time plugin registry using distributed slices for zero-cost plugin discovery
// ABOUTME: Provides thread-safe, efficient plugin management with Rust-idiomatic patterns

use super::core::{PluginInfo, PluginTool};
use super::PluginEnvironment;
use crate::protocols::universal::UniversalRequest;
use crate::protocols::ProtocolError;
use linkme::distributed_slice;
use std::collections::HashMap;
use std::sync::Arc;

/// Distributed slice for compile-time plugin registration
/// Each plugin adds itself to this slice via the linkme crate
#[distributed_slice]
pub static PIERRE_PLUGINS: [fn() -> Box<dyn PluginTool>] = [..];

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

    /// Register all plugins from the distributed slice
    fn register_builtin_plugins(&mut self) {
        for plugin_factory in PIERRE_PLUGINS {
            let plugin = plugin_factory();
            let info = plugin.info().clone();

            tracing::info!(
                "Registering plugin: {} (category: {:?})",
                info.name,
                info.category
            );

            // Call plugin lifecycle hook
            if let Err(e) = plugin.on_register() {
                tracing::error!("Failed to register plugin {}: {}", info.name, e);
                continue;
            }

            let plugin_arc = Arc::from(plugin);
            self.plugins.insert(info.name.to_string(), plugin_arc);
            self.plugin_info.insert(info.name.to_string(), info);
        }

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
            return Err(ProtocolError::PluginError(format!(
                "Plugin '{}' is already registered",
                info.name
            )));
        }

        // Call plugin lifecycle hook
        plugin.on_register()?;

        tracing::info!("Dynamically registering plugin: {}", info.name);

        let plugin_arc = Arc::from(plugin);
        self.plugins.insert(info.name.to_string(), plugin_arc);
        self.plugin_info.insert(info.name.to_string(), info);

        Ok(())
    }

    /// Unregister a plugin
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if plugin not found or unregistration fails
    pub fn unregister_plugin(&mut self, plugin_name: &str) -> Result<(), ProtocolError> {
        let plugin = self
            .plugins
            .remove(plugin_name)
            .ok_or_else(|| ProtocolError::PluginNotFound(plugin_name.to_string()))?;

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
        let plugin = self
            .plugins
            .get(plugin_name)
            .ok_or_else(|| ProtocolError::PluginNotFound(plugin_name.to_string()))?;

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
    pub total_plugins: usize,
    pub data_access_plugins: usize,
    pub intelligence_plugins: usize,
    pub analytics_plugins: usize,
    pub goals_plugins: usize,
    pub provider_plugins: usize,
    pub environmental_plugins: usize,
    pub custom_plugins: usize,
}

/// Macro for registering a plugin at compile time
#[macro_export]
macro_rules! register_plugin {
    ($plugin_type:ty) => {
        #[linkme::distributed_slice($crate::plugins::registry::PIERRE_PLUGINS)]
        #[linkme(crate = linkme)]
        static PLUGIN_FACTORY: fn() -> Box<dyn $crate::plugins::core::PluginTool> =
            || Box::new(<$plugin_type>::new());
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::core::{PluginCategory, PluginInfo};
    use async_trait::async_trait;

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
        ) -> Result<super::super::PluginResult, ProtocolError> {
            Ok(super::super::core::plugin_success(
                serde_json::json!({"test": true}),
                1,
                100,
            ))
        }
    }

    #[test]
    fn test_plugin_registry_creation() {
        let registry = PluginRegistry::new();
        // Registry may have compile-time registered plugins (count could be 0 or more)
        let _ = registry.plugins.len(); // Just verify registry is accessible
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
}
