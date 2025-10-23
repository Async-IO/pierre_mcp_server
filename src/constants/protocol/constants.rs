// ABOUTME: MCP protocol constants including version, JSON-RPC, and server identification
// ABOUTME: Provides environment-configurable protocol values with sensible defaults
//
// Licensed under either of Apache License, Version 2.0 or MIT License at your option.
// Copyright ©2025 Async-IO.org

//! Protocol constants for MCP and JSON-RPC

/// Get MCP Protocol version from environment or default
#[must_use]
pub fn mcp_protocol_version() -> String {
    crate::constants::env_config::mcp_protocol_version()
}

/// JSON-RPC version (standard, not configurable)
pub const JSONRPC_VERSION: &str = "2.0";

/// Get server name from environment or default
#[must_use]
pub fn server_name() -> String {
    crate::constants::env_config::server_name()
}

/// Get server name variant with specific suffix
#[must_use]
pub fn server_name_multitenant() -> String {
    crate::constants::env_config::server_name()
}

/// Server version from Cargo.toml
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
