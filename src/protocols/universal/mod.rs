// ABOUTME: Universal protocol module with clean architecture
// ABOUTME: Modular components for tool execution, authentication, and routing
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

pub mod auth_service;
pub mod executor;
pub mod handlers;
pub mod tool_registry;
pub mod types;

// Re-export core types
pub use types::{UniversalRequest, UniversalResponse, UniversalTool, UniversalToolExecutor};

// Re-export new architecture components
pub use auth_service::AuthService;
pub use executor::UniversalExecutor;
pub use tool_registry::ToolRegistry;
