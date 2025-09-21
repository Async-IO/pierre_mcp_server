// ABOUTME: Core plugin trait definitions and metadata structures
// ABOUTME: Provides the foundational abstractions for all Pierre MCP Server plugins

use super::{PluginEnvironment, PluginResult};
use crate::protocols::universal::{UniversalRequest, UniversalResponse};
use crate::protocols::ProtocolError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Plugin metadata for discovery and registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Tool name (must be unique)
    pub name: &'static str,
    /// Human-readable description
    pub description: &'static str,
    /// JSON schema for input validation
    pub input_schema: &'static str,
    /// Plugin version for compatibility
    pub version: &'static str,
    /// Credit cost for usage metering
    pub credit_cost: u32,
    /// Plugin author/maintainer
    pub author: &'static str,
    /// Plugin category for organization
    pub category: PluginCategory,
}

/// Plugin categories for organization and discovery
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PluginCategory {
    /// Data access tools (`get_activities`, `get_athlete`)
    DataAccess,
    /// AI/ML analysis tools
    Intelligence,
    /// Performance analytics
    Analytics,
    /// Goal and training management
    Goals,
    /// Provider connectivity
    Providers,
    /// Weather and environmental data
    Environmental,
    /// Custom/community tools
    Community,
}

impl PluginCategory {
    /// Get all available categories
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::DataAccess,
            Self::Intelligence,
            Self::Analytics,
            Self::Goals,
            Self::Providers,
            Self::Environmental,
            Self::Community,
        ]
    }

    /// Get category display name
    #[must_use]
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::DataAccess => "Data Access",
            Self::Intelligence => "AI Intelligence",
            Self::Analytics => "Analytics",
            Self::Goals => "Goals & Training",
            Self::Providers => "Provider Integration",
            Self::Environmental => "Environmental Data",
            Self::Community => "Community Tools",
        }
    }
}

/// Core plugin trait for runtime execution
#[async_trait]
pub trait PluginTool: Send + Sync {
    /// Get plugin metadata
    fn info(&self) -> &PluginInfo;

    /// Execute the plugin with full context
    async fn execute(
        &self,
        request: UniversalRequest,
        env: PluginEnvironment<'_>,
    ) -> Result<PluginResult, ProtocolError>;

    /// Validate plugin input parameters
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if validation fails or required parameters are missing
    fn validate_input(&self, params: &Value) -> Result<(), ProtocolError> {
        // Default implementation - plugins can override for custom validation
        let schema: Value = serde_json::from_str(self.info().input_schema)
            .map_err(|e| ProtocolError::InvalidSchema(format!("Invalid input schema: {e}")))?;

        // Basic validation - in production would use jsonschema crate
        if schema.get("required").is_some() {
            if let Some(required_fields) = schema["required"].as_array() {
                for field in required_fields {
                    if let Some(field_name) = field.as_str() {
                        if params.get(field_name).is_none() {
                            return Err(ProtocolError::InvalidParameters(format!(
                                "Missing required parameter: {field_name}"
                            )));
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Plugin lifecycle hook - called when plugin is registered
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if plugin registration fails
    fn on_register(&self) -> Result<(), ProtocolError> {
        tracing::info!("Registered plugin: {}", self.info().name);
        Ok(())
    }

    /// Plugin lifecycle hook - called when plugin is unregistered
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` if plugin unregistration fails
    fn on_unregister(&self) -> Result<(), ProtocolError> {
        tracing::info!("Unregistered plugin: {}", self.info().name);
        Ok(())
    }
}

/// Static plugin trait for compile-time registration
/// This enables zero-cost plugin creation via const functions
pub trait PluginToolStatic: PluginTool {
    /// Create plugin instance (must be const-constructible)
    fn new() -> Self
    where
        Self: Sized;

    /// Get plugin info at compile time
    const INFO: PluginInfo;
}

// Plugins implement PluginTool directly for type-safe registration

/// Helper macro to implement `PluginTool` for static plugins
#[macro_export]
macro_rules! impl_static_plugin {
    ($plugin_type:ty) => {
        #[async_trait::async_trait]
        impl $crate::plugins::core::PluginTool for $plugin_type {
            fn info(&self) -> &$crate::plugins::core::PluginInfo {
                &Self::INFO
            }

            async fn execute(
                &self,
                request: $crate::protocols::universal::UniversalRequest,
                env: $crate::plugins::PluginEnvironment<'_>,
            ) -> Result<$crate::plugins::PluginResult, $crate::protocols::ProtocolError> {
                // Validate input parameters
                self.validate_input(&request.parameters)?;

                // Track execution time
                let start_time = std::time::Instant::now();

                // Execute plugin-specific logic
                let response = self.execute_impl(request, env).await;

                // Safe: execution times are bounded by system limits, won't exceed u64
                #[allow(clippy::cast_possible_truncation)]
                let execution_time_ms = start_time.elapsed().as_millis() as u64;

                Ok($crate::plugins::PluginResult {
                    response,
                    credits_consumed: self.info().credit_cost,
                    execution_time_ms,
                })
            }
        }
    };
}

/// Implementation trait for static plugins - plugins implement this instead of PluginTool directly
#[async_trait]
pub trait PluginImplementation {
    /// Plugin-specific implementation
    async fn execute_impl(
        &self,
        request: UniversalRequest,
        env: PluginEnvironment<'_>,
    ) -> Result<UniversalResponse, ProtocolError>;
}

/// Convenience macro for creating plugin metadata
#[macro_export]
macro_rules! plugin_info {
    (
        name: $name:literal,
        description: $desc:literal,
        category: $category:expr,
        input_schema: $schema:literal,
        credit_cost: $cost:literal,
        author: $author:literal,
        version: $version:literal $(,)?
    ) => {
        $crate::plugins::core::PluginInfo {
            name: $name,
            description: $desc,
            input_schema: $schema,
            version: $version,
            credit_cost: $cost,
            author: $author,
            category: $category,
        }
    };
}

/// Helper function to create a successful plugin result
#[must_use]
pub fn plugin_success(result: Value, credits: u32, execution_time: u64) -> PluginResult {
    PluginResult {
        response: Ok(UniversalResponse {
            success: true,
            result: Some(result),
            error: None,
            metadata: Some(std::collections::HashMap::from([
                ("credits_consumed".into(), Value::Number(credits.into())),
                (
                    "execution_time_ms".into(),
                    Value::Number(execution_time.into()),
                ),
            ])),
        }),
        credits_consumed: credits,
        execution_time_ms: execution_time,
    }
}

/// Helper function to create a plugin error result
#[must_use]
pub fn plugin_error(error: String, credits: u32, execution_time: u64) -> PluginResult {
    PluginResult {
        response: Ok(UniversalResponse {
            success: false,
            result: None,
            error: Some(error),
            metadata: Some(std::collections::HashMap::from([
                ("credits_consumed".into(), Value::Number(credits.into())),
                (
                    "execution_time_ms".into(),
                    Value::Number(execution_time.into()),
                ),
            ])),
        }),
        credits_consumed: credits,
        execution_time_ms: execution_time,
    }
}
