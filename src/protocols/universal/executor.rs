// ABOUTME: Clean universal executor that coordinates authentication, routing, and execution
// ABOUTME: Replaces monolithic universal.rs with composable services and type-safe routing
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

use super::auth_service::AuthService;
use super::handlers::{
    handle_calculate_personalized_zones, handle_connect_provider,
    handle_disconnect_provider, handle_get_configuration_catalog,
    handle_get_configuration_profiles, handle_get_connection_status,
    handle_get_user_configuration, handle_update_user_configuration,
    handle_validate_configuration,
};
// NOTE: Fitness-specific handlers removed - now in pierre-fitness-app
// TODO: Implement plugin system for fitness-app to register tools dynamically
use super::tool_registry::{ToolId, ToolInfo, ToolRegistry};
use crate::mcp::resources::ServerResources;
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use std::sync::Arc;

// NOTE: IntelligenceService moved to pierre-fitness-app
// This was fitness-specific and used crate::models::Activity and crate::intelligence
// TODO: Make this extensible so fitness-app can provide intelligence service implementation

/// Clean universal executor with separated concerns
/// No clippy suppressions needed - this is well-designed code
pub struct UniversalExecutor {
    /// Authentication service for handling OAuth and token validation
    pub auth_service: AuthService,
    /// Shared server resources (database, weather service, etc.)
    pub resources: Arc<ServerResources>,
    /// Tool registry mapping tool IDs to handlers
    registry: ToolRegistry,
}

impl UniversalExecutor {
    /// Create new executor with all services
    #[must_use]
    pub fn new(resources: Arc<ServerResources>) -> Self {
        let auth_service = AuthService::new(resources.clone()); // Safe: Arc clone for service creation
        let mut registry = ToolRegistry::new();

        // Register all tools with their handlers
        Self::register_all_tools(&mut registry);

        Self {
            auth_service,
            resources,
            registry,
        }
    }

    // NOTE: Fitness tool registration functions commented out - moved to pierre-fitness-app
    // TODO: Implement plugin system so fitness-app can register these dynamically
    //
    // /// Register all tools with type-safe handlers
    // fn register_strava_tools(registry: &mut ToolRegistry) {
    //     registry.register(ToolInfo::async_tool(
    //         ToolId::GetActivities,
    //         |executor, request| Box::pin(handle_get_activities(executor, request)),
    //     ));
    //     registry.register(ToolInfo::async_tool(
    //         ToolId::GetAthlete,
    //         |executor, request| Box::pin(handle_get_athlete(executor, request)),
    //     ));
    //     registry.register(ToolInfo::async_tool(
    //         ToolId::GetStats,
    //         |executor, request| Box::pin(handle_get_stats(executor, request)),
    //     ));
    //     registry.register(ToolInfo::async_tool(
    //         ToolId::AnalyzeActivity,
    //         |executor, request| Box::pin(handle_analyze_activity(executor, request)),
    //     ));
    // }

    fn register_connection_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::async_tool(
            ToolId::GetConnectionStatus,
            |executor, request| Box::pin(handle_get_connection_status(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::ConnectProvider,
            |executor, request| Box::pin(handle_connect_provider(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::DisconnectProvider,
            |executor, request| Box::pin(handle_disconnect_provider(executor, request)),
        ));
    }

    fn register_configuration_tools(registry: &mut ToolRegistry) {
        registry.register(ToolInfo::sync_tool(
            ToolId::GetConfigurationCatalog,
            handle_get_configuration_catalog,
        ));
        registry.register(ToolInfo::sync_tool(
            ToolId::GetConfigurationProfiles,
            handle_get_configuration_profiles,
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::GetUserConfiguration,
            |executor, request| Box::pin(handle_get_user_configuration(executor, request)),
        ));
        registry.register(ToolInfo::async_tool(
            ToolId::UpdateUserConfiguration,
            |executor, request| Box::pin(handle_update_user_configuration(executor, request)),
        ));
        registry.register(ToolInfo::sync_tool(
            ToolId::CalculatePersonalizedZones,
            handle_calculate_personalized_zones,
        ));
        registry.register(ToolInfo::sync_tool(
            ToolId::ValidateConfiguration,
            handle_validate_configuration,
        ));
    }

    // Fitness tool registration functions commented out - moved to pierre-fitness-app
    // TODO: Implement plugin system for dynamic tool registration
    //
    // fn register_intelligence_tools(registry: &mut ToolRegistry) { ... }
    // fn register_goal_tools(registry: &mut ToolRegistry) { ... }
    // fn register_sleep_recovery_tools(registry: &mut ToolRegistry) { ... }
    // fn register_nutrition_tools(registry: &mut ToolRegistry) { ... }

    fn register_all_tools(registry: &mut ToolRegistry) {
        // Only register generic framework tools
        // Fitness tools will be registered by pierre-fitness-app via plugin system (TBD)
        Self::register_connection_tools(registry);
        Self::register_configuration_tools(registry);
    }

    /// Execute a tool with type-safe routing (no string matching!)
    ///
    /// # Errors
    /// Returns `ProtocolError` if tool is not found or execution fails
    pub async fn execute_tool(
        &self,
        request: UniversalRequest,
    ) -> Result<UniversalResponse, ProtocolError> {
        // Convert string tool name to type-safe ID
        let tool_id = self
            .registry
            .resolve_tool_name(&request.tool_name)
            .ok_or_else(|| ProtocolError::ToolNotFound {
                tool_id: request.tool_name.clone(),
                available_count: self.registry.list_tools().len(),
            })?; // Safe: String ownership needed for error message

        // Get registered tool info
        let tool_info = self.registry.get_tool(tool_id).ok_or_else(|| {
            ProtocolError::InternalError(format!("Tool {tool_id:?} not registered"))
        })?;

        // Convert to legacy UniversalToolExecutor for handler compatibility
        let legacy_executor = Self::new(self.resources.clone()); // Safe: Arc clone for legacy executor creation

        // Execute based on tool type
        match (tool_info.async_handler, tool_info.sync_handler) {
            (Some(async_handler), None) => {
                // Execute async handler
                async_handler(&legacy_executor, request).await
            }
            (None, Some(sync_handler)) => {
                // Execute sync handler
                sync_handler(&legacy_executor, &request)
            }
            _ => Err(ProtocolError::InternalError(format!(
                "Tool {tool_id:?} has invalid handler configuration"
            ))),
        }
    }

    /// List all available tools for MCP schema generation
    #[must_use]
    pub fn list_tools(&self) -> Vec<ToolId> {
        self.registry.list_tools()
    }

    /// Get tool metadata for documentation
    #[must_use]
    pub fn get_tool_info(&self, tool_id: ToolId) -> Option<(String, String, bool, bool)> {
        if self.registry.has_tool(tool_id) {
            Some((
                tool_id.name().to_owned(),
                tool_id.description().to_owned(),
                tool_id.requires_auth(),
                tool_id.is_async(),
            ))
        } else {
            None
        }
    }

    /// Check if executor has a specific tool
    #[must_use]
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.registry.resolve_tool_name(tool_name).is_some()
    }
}
