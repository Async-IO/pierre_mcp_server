// ABOUTME: Universal protocol module with clean architecture
// ABOUTME: Modular components for tool execution, authentication, and routing
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

/// Authentication service for protocol requests
pub mod auth_service;
/// Universal tool executor
pub mod executor;
/// Tool handler implementations
pub mod handlers;
/// Tool registry for discovering available tools
pub mod tool_registry;
/// Universal protocol types and interfaces
pub mod types;

// Re-export core types
pub use types::{UniversalRequest, UniversalResponse, UniversalTool, UniversalToolExecutor};

// Re-export new architecture components
/// Authentication service for universal protocol
pub use auth_service::AuthService;
/// Main executor for universal protocol tools
pub use executor::UniversalExecutor;
/// Registry for managing universal protocol tools
pub use tool_registry::ToolRegistry;
