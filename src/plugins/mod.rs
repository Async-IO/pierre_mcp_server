// ABOUTME: Compile-time plugin system for extensible tool architecture
// ABOUTME: Type-safe plugin registration and execution with plugin lifecycle management
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Plugin System
//!
//! Compile-time plugin registration system that provides:
//! - Zero-cost plugin abstractions via distributed slices
//! - Type-safe plugin registration and execution
//! - Plugin lifecycle management
//! - Categorized plugin organization
//!
//! ## Usage
//!
//! ```rust,no_run
//! use pierre_mcp_server::plugins::core::{PluginToolStatic, PluginImplementation};
//! use pierre_mcp_server::plugins::PluginEnvironment;
//! use pierre_mcp_server::protocols::{UniversalRequest, UniversalResponse, ProtocolError};
//!
//! pub struct CustomAnalysisTool;
//!
//! # #[async_trait::async_trait]
//! impl PluginImplementation for CustomAnalysisTool {
//!     async fn execute_impl(&self, request: UniversalRequest, env: PluginEnvironment<'_>) -> Result<UniversalResponse, ProtocolError> {
//!         // Plugin implementation
//! #       Ok(UniversalResponse {
//! #           success: true,
//! #           result: None,
//! #           error: None,
//! #           metadata: None,
//! #       })
//!     }
//! }
//! ```

/// Core plugin types and traits
pub mod core;
/// Plugin executor for running tools
pub mod executor;
/// Plugin registry for tool management
pub mod registry;

/// Community-contributed plugins
pub mod community;

// Re-export key types

/// Plugin information metadata
pub use core::PluginInfo;
/// Plugin tool trait
pub use core::PluginTool;
/// Static plugin tool registry entry
pub use core::PluginToolStatic;
/// Plugin tool executor
pub use executor::PluginToolExecutor;
/// Plugin tool executor builder
pub use executor::PluginToolExecutorBuilder;
/// Tool information struct
pub use executor::ToolInfo;
/// Plugin registry
pub use registry::PluginRegistry;

use crate::database_plugins::factory::Database;
use crate::protocols::universal::UniversalResponse;
use crate::protocols::ProtocolError;
use crate::providers::registry::ProviderRegistry;

/// Plugin execution context with user information
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// User ID for this plugin execution
    pub user_id: uuid::Uuid,
    /// Optional tenant ID for multi-tenant isolation
    pub tenant_id: Option<uuid::Uuid>,
}

/// Plugin execution result with usage tracking
#[derive(Debug)]
pub struct PluginResult {
    /// Plugin execution response or error
    pub response: Result<UniversalResponse, ProtocolError>,
    /// Credits consumed by this plugin execution
    pub credits_consumed: u32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

/// Plugin execution environment for safe resource access
pub struct PluginEnvironment<'a> {
    /// Database connection for data persistence
    pub database: &'a Database,
    /// Provider registry for fitness data access
    pub provider_registry: &'a ProviderRegistry,
    /// Execution context with user information
    pub context: &'a PluginContext,
}

impl<'a> PluginEnvironment<'a> {
    /// Create new plugin environment with resource access
    #[must_use]
    pub const fn new(
        database: &'a Database,
        provider_registry: &'a ProviderRegistry,
        context: &'a PluginContext,
    ) -> Self {
        Self {
            database,
            provider_registry,
            context,
        }
    }
}
