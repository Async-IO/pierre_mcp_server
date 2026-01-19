// ABOUTME: Unified tool execution engine providing fitness analysis and data processing tools
// ABOUTME: Central tool registry for MCP protocol tools, A2A tools, and fitness intelligence operations
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! # Unified Tool Execution Engine
//!
//! This module provides a shared tool execution engine that can be used
//! by both single-tenant and multi-tenant MCP implementations, eliminating
//! code duplication and providing a single source of truth for tool logic.
//!
//! ## Architecture
//!
//! The tools module is organized as follows:
//!
//! - **Core Types** (`traits`, `context`, `result`, `errors`)
//!   - `McpTool` trait - the main interface for all tools
//!   - `ToolCapabilities` - bitflags for capability-based filtering
//!   - `ToolExecutionContext` - context for tool execution
//!   - `ToolResult` - structured result with notifications
//!
//! - **Registry** (`registry`)
//!   - Central tool registration and lookup
//!   - Capability-based filtering (admin vs user)
//!   - Feature-flag-based conditional compilation
//!
//! - **Decorators** (`decorators`)
//!   - `AuditedTool` - audit logging for admin operations and security tracking
//!
//! - **Implementations** (`implementations`)
//!   - Tool implementations organized by category
//!   - Each category behind a feature flag
//!
//! ## Feature Flags
//!
//! Tool categories can be enabled/disabled via Cargo features:
//!
//! - `tools-connection` - Provider connection management
//! - `tools-data` - Data access tools
//! - `tools-analytics` - Analytics and metrics tools
//! - `tools-goals` - Goal management tools
//! - `tools-config` - Configuration tools
//! - `tools-nutrition` - Nutrition tools
//! - `tools-sleep` - Sleep/recovery tools
//! - `tools-recipes` - Recipe management tools
//! - `tools-coaches` - AI coach tools
//! - `tools-admin` - Admin-only tools
//!
//! ## Example
//!
//! ```
//! use pierre_mcp_server::tools::ToolRegistry;
//!
//! // Create and populate registry
//! let mut registry = ToolRegistry::new();
//! registry.register_builtin_tools();
//!
//! // List user-visible tools
//! let is_admin = false;
//! let schemas = registry.list_schemas_for_role(is_admin);
//! ```
//!
//! For async tool execution examples, see [`ToolRegistry::execute`].

// ============================================================================
// Core Types - Pluggable MCP Tools Architecture
// ============================================================================

/// MCP tool trait and capability flags
pub mod traits;

/// Tool execution context with resources and user info
pub mod context;

/// Tool result types with notification support
pub mod result;

/// Tool-specific error types
pub mod errors;

/// Central tool registry with capability-based filtering
pub mod registry;

/// Tool decorators (caching, auditing)
pub mod decorators;

/// Tool implementations organized by category
pub mod implementations;

// ============================================================================
// Legacy Modules - To be migrated in later phases
// ============================================================================

/// Tool execution engine core (legacy)
pub mod engine;

/// Provider-specific tool implementations (legacy)
pub mod providers;

/// Tool response formatting utilities (legacy)
pub mod responses;

// ============================================================================
// Re-exports for convenient access
// ============================================================================

pub use context::{AuthMethod, ToolExecutionContext};
pub use decorators::AuditedTool;
pub use errors::ToolError;
pub use registry::{register_external_tool, ToolRegistry};
pub use result::{NotificationType, ToolNotification, ToolResult};
pub use traits::{McpTool, ToolBundle, ToolCapabilities, ToolDescriptor, ToolFactory};
