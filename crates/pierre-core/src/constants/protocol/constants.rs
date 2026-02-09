// ABOUTME: MCP protocol constants for version and server identification
// ABOUTME: Pure compile-time constants without runtime configuration dependencies
//
// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Pierre Fitness Intelligence

//! Protocol constants for MCP and JSON-RPC
//!
//! Runtime-configurable protocol values (server name, MCP version) are provided
//! by the main crate's constants module via `get_server_config()`.

/// JSON-RPC version (standard, not configurable)
pub const JSONRPC_VERSION: &str = "2.0";

/// Server version from Cargo.toml
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
