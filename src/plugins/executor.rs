// ABOUTME: Plugin-enabled tool executor with builder pattern for flexible tool registration
// ABOUTME: Bridges the plugin system with the existing UniversalToolExecutor architecture

use super::registry::PluginRegistry;
use super::{PluginContext, PluginEnvironment};
use crate::mcp::resources::ServerResources;
use crate::protocols::universal::{UniversalRequest, UniversalResponse, UniversalToolExecutor};
use crate::protocols::ProtocolError;
use std::sync::Arc;

/// Plugin-enabled tool executor that extends `UniversalToolExecutor` with plugin support
pub struct PluginToolExecutor {
    /// Core universal tool executor for existing hardcoded tools
    core_executor: UniversalToolExecutor,
    /// Plugin registry for dynamic tools
    plugin_registry: PluginRegistry,
    /// Server resources for plugin environment
    resources: Arc<ServerResources>,
}

impl PluginToolExecutor {
    /// Create new plugin-enabled tool executor
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        Self {
            core_executor: UniversalToolExecutor::new(resources.clone()),
            plugin_registry: PluginRegistry::new(),
            resources,
        }
    }

    /// Get list of core tools
    const fn get_core_tools() -> &'static [&'static str] {
        &[
            // Data Access Tools
            "get_activities",
            "get_athlete",
            "get_stats",
            // Intelligence Tools
            "get_activity_intelligence",
            "analyze_activity",
            "calculate_metrics",
            // Analytics Tools
            "analyze_performance_trends",
            "compare_activities",
            "detect_patterns",
            // Goal Tools
            "create_goal",
            "get_goals",
            "suggest_goals",
            // Weather Tools
            "get_weather_for_activity",
            // Provider Tools
            "connect_provider",
            "disconnect_provider",
            "get_connection_status",
            // Prediction Tools
            "predict_performance",
            "generate_recommendations",
        ]
    }

    /// Execute a tool (plugin or core) based on the tool name
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if tool execution fails
    pub async fn execute_tool(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, ProtocolError> {
        // Create plugin context
        let context = PluginContext {
            user_id: uuid::Uuid::parse_str(&request.user_id)
                .map_err(|e| ProtocolError::InvalidParameters(format!("Invalid user_id: {e}")))?,
            tenant_id: request
                .tenant_id
                .as_ref()
                .map(|id| uuid::Uuid::parse_str(id))
                .transpose()
                .map_err(|e| ProtocolError::InvalidParameters(format!("Invalid tenant_id: {e}")))?,
        };

        // Check if it's a plugin tool first
        if self.plugin_registry.has_plugin(&request.tool_name) {
            tracing::debug!("Executing plugin tool: {}", request.tool_name);

            let env = PluginEnvironment::new(
                &self.resources.database,
                &self.resources.provider_registry,
                &context,
            );

            let plugin_result = self
                .plugin_registry
                .execute_plugin(&request.tool_name.clone(), request, env)
                .await?;

            return plugin_result.response;
        }

        // Fall back to core executor for hardcoded tools
        tracing::debug!("Executing core tool: {}", request.tool_name);
        self.core_executor.execute_tool(request).await
    }

    /// List all available tools (core + plugins)
    #[must_use]
    pub fn list_all_tools(&self) -> Vec<String> {
        let mut tools = Vec::new();

        // Add core tools
        let core_tools = Self::get_core_tools();
        for &tool_name in core_tools {
            tools.push(tool_name.to_string());
        }

        // Add plugin tools
        tools.extend(self.plugin_registry.list_plugin_names());

        tools.sort();
        tools
    }

    /// Get plugin registry for management operations
    #[must_use]
    pub const fn plugin_registry(&self) -> &PluginRegistry {
        &self.plugin_registry
    }

    /// Get mutable plugin registry for dynamic registration
    pub const fn plugin_registry_mut(&mut self) -> &mut PluginRegistry {
        &mut self.plugin_registry
    }

    /// Get core executor for direct access to hardcoded tools
    #[must_use]
    pub const fn core_executor(&self) -> &UniversalToolExecutor {
        &self.core_executor
    }

    /// Check if a tool exists (core or plugin)
    #[must_use]
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.plugin_registry.has_plugin(tool_name) || Self::get_core_tools().contains(&tool_name)
    }

    /// Get tool information (tries plugin first, then core)
    #[must_use]
    pub fn get_tool_info(&self, tool_name: &str) -> Option<ToolInfo> {
        // Check plugin registry first
        if let Some(plugin_info) = self.plugin_registry.get_plugin_info(tool_name) {
            return Some(ToolInfo::Plugin(plugin_info.clone()));
        }

        // Check core tools
        if Self::get_core_tools().contains(&tool_name) {
            let description = match tool_name {
                "get_activities" => "Fetch fitness activities with pagination support",
                "get_athlete" => "Get complete athlete profile information",
                "get_stats" => "Get aggregated fitness statistics and lifetime metrics",
                _ => "Core fitness tool",
            };
            return Some(ToolInfo::Core {
                name: tool_name.to_string(),
                description: description.to_string(),
            });
        }

        None
    }

    /// Get execution statistics
    #[must_use]
    pub fn get_statistics(&self) -> ExecutorStatistics {
        ExecutorStatistics {
            total_tools: self.list_all_tools().len(),
            core_tools: Self::get_core_tools().len(),
            plugin_tools: self.plugin_registry.list_plugin_names().len(),
            plugin_stats: self.plugin_registry.get_statistics(),
        }
    }
}

/// Tool information wrapper for both core and plugin tools
#[derive(Debug, Clone)]
pub enum ToolInfo {
    Plugin(crate::plugins::core::PluginInfo),
    Core { name: String, description: String },
}

impl ToolInfo {
    /// Get tool name
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Plugin(info) => info.name,
            Self::Core { name, .. } => name,
        }
    }

    /// Get tool description
    #[must_use]
    pub fn description(&self) -> &str {
        match self {
            Self::Plugin(info) => info.description,
            Self::Core { description, .. } => description,
        }
    }

    /// Check if tool is a plugin
    #[must_use]
    pub const fn is_plugin(&self) -> bool {
        matches!(self, Self::Plugin(_))
    }
}

/// Executor statistics for monitoring
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutorStatistics {
    pub total_tools: usize,
    pub core_tools: usize,
    pub plugin_tools: usize,
    pub plugin_stats: crate::plugins::registry::PluginRegistryStatistics,
}

/// Builder pattern for creating plugin-enabled tool executors
pub struct PluginToolExecutorBuilder {
    resources: Option<Arc<ServerResources>>,
    plugins: Vec<Box<dyn crate::plugins::core::PluginTool>>,
}

impl PluginToolExecutorBuilder {
    /// Create new builder
    #[must_use]
    pub const fn new() -> Self {
        Self {
            resources: None,
            plugins: Vec::new(),
        }
    }

    /// Set server resources
    #[must_use]
    pub fn with_resources(mut self, resources: Arc<ServerResources>) -> Self {
        self.resources = Some(resources);
        self
    }

    /// Add a custom plugin
    #[must_use]
    pub fn with_plugin(mut self, plugin: Box<dyn crate::plugins::core::PluginTool>) -> Self {
        self.plugins.push(plugin);
        self
    }

    /// Enable community plugins
    #[must_use]
    pub const fn with_community_plugins(self) -> Self {
        // Community plugins are automatically registered via compile-time registration
        // This method exists for API consistency
        self
    }

    /// Build the plugin-enabled executor
    ///
    /// # Panics
    ///
    /// Panics if resources were not provided
    #[must_use]
    pub fn build(self) -> PluginToolExecutor {
        let resources = self.resources.expect("ServerResources required");
        let mut executor = PluginToolExecutor::new(resources);

        // Register additional dynamic plugins
        for plugin in self.plugins {
            if let Err(e) = executor.plugin_registry_mut().register_plugin(plugin) {
                tracing::error!("Failed to register dynamic plugin: {}", e);
            }
        }

        executor
    }
}

impl Default for PluginToolExecutorBuilder {
    fn default() -> Self {
        Self::new()
    }
}
