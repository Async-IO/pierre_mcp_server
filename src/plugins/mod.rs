// ABOUTME: Compile-time plugin system for extensible tool architecture
// ABOUTME: Type-safe plugin registration and execution with plugin lifecycle management
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

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

pub mod core;
pub mod executor;
pub mod registry;

/// Community-contributed plugins
pub mod community;

// Re-export key types
pub use core::{PluginInfo, PluginTool, PluginToolStatic};
pub use executor::{PluginToolExecutor, PluginToolExecutorBuilder, ToolInfo};
pub use registry::PluginRegistry;

use crate::protocols::universal::UniversalResponse;
use crate::protocols::ProtocolError;

/// Plugin execution context with user information
#[derive(Debug, Clone)]
pub struct PluginContext {
    pub user_id: uuid::Uuid,
    pub tenant_id: Option<uuid::Uuid>,
}

/// Plugin execution result with usage tracking
#[derive(Debug)]
pub struct PluginResult {
    pub response: Result<UniversalResponse, ProtocolError>,
    pub credits_consumed: u32,
    pub execution_time_ms: u64,
}

/// Plugin execution environment for safe resource access
pub struct PluginEnvironment<'a> {
    pub database: &'a crate::database_plugins::factory::Database,
    pub provider_registry: &'a crate::providers::registry::ProviderRegistry,
    pub context: &'a PluginContext,
}

impl<'a> PluginEnvironment<'a> {
    /// Create new plugin environment with resource access
    #[must_use]
    pub const fn new(
        database: &'a crate::database_plugins::factory::Database,
        provider_registry: &'a crate::providers::registry::ProviderRegistry,
        context: &'a PluginContext,
    ) -> Self {
        Self {
            database,
            provider_registry,
            context,
        }
    }
}
